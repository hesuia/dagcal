use crate::completion::{accept_selected_completion, refresh_completion_state};
use crate::formatting::{
    entry_expression_source, entry_reference_token, selected_compact_text, selected_error_text,
    selected_summary_text, state_summary, status_state_summary,
};
use crate::{
    AppAction, AppEffect, CompletionCandidate, CompletionDirection, CompletionState, Draft, Engine,
    EngineSnapshot, EntryState, EntryStateFilter, EntryView, ExpressionId, SelectionDirection,
    SessionChange,
};
use std::collections::BTreeSet;

pub struct AppSession {
    pub engine: Engine,
    pub entries: Vec<EntryView>,
    pub entry_search_open: bool,
    pub entry_search_query: String,
    pub entry_state_filter: EntryStateFilter,
    pub input: Draft,
    pub editing: Option<ExpressionId>,
    pub draft_entry: Option<ExpressionId>,
    pub selected: Option<ExpressionId>,
    pub hovered_entry: Option<ExpressionId>,
    pub status: String,
    pub completion: CompletionState,
}

impl Default for AppSession {
    fn default() -> Self {
        Self::new()
    }
}

impl AppSession {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            entries: Vec::new(),
            entry_search_open: false,
            entry_search_query: String::new(),
            entry_state_filter: EntryStateFilter::All,
            input: Draft::default(),
            editing: None,
            draft_entry: None,
            selected: None,
            hovered_entry: None,
            status: "Ready".to_string(),
            completion: CompletionState::default(),
        }
    }

    /// Applies one frontend-independent action and returns requested UI effects.
    pub fn dispatch(&mut self, action: AppAction) -> Vec<AppEffect> {
        let effect = match action {
            AppAction::InputChanged(value) => self.input_changed(value),
            AppAction::OpenEntrySearch => self.open_entry_search(),
            AppAction::CloseEntrySearch => self.close_entry_search(),
            AppAction::EntrySearchChanged(value) => self.entry_search_changed(value),
            AppAction::EntryStateFilterChanged(filter) => self.entry_state_filter_changed(filter),
            AppAction::ClearEntrySearch => self.clear_entry_search(),
            AppAction::SubmitInput => self.submit_input(),
            AppAction::StartEdit(id) => self.start_edit(id),
            AppAction::StartNewEntry => self.start_new_entry(),
            AppAction::CancelEdit => self.cancel_edit(),
            AppAction::DeleteEntry(id) => self.delete_entry(id),
            AppAction::RecalculateEntry(id) => self.recalculate_entry(id),
            AppAction::RecalculateAll => self.recalculate_all(),
            AppAction::InsertReference(id) => self.insert_reference(id),
            AppAction::InsertConstant(name) => self.insert_constant(name),
            AppAction::InsertFunction(name) => self.insert_function(name),
            AppAction::SelectEntry(id) => self.select_entry(id),
            AppAction::SetHoveredEntry(id) => self.set_hovered_entry(id),
            AppAction::ClearHoveredEntry(id) => self.clear_hovered_entry(id),
            AppAction::SelectHoveredEntry => self.select_hovered_entry(),
            AppAction::Clear => self.clear(),
            AppAction::Undo => self.undo(),
            AppAction::Redo => self.redo(),
            AppAction::MoveSelection(direction) => self.move_selection(direction),
            AppAction::MoveCompletion(direction) => {
                self.move_completion_selection(direction);
                AppEffect::None
            }
            AppAction::AcceptCompletion(index) => {
                return self
                    .accept_completion(index)
                    .then_some(AppEffect::FocusInput)
                    .into_iter()
                    .collect();
            }
            AppAction::CloseCompletions => {
                self.close_completions();
                AppEffect::None
            }
        };

        (!matches!(effect, AppEffect::None))
            .then_some(effect)
            .into_iter()
            .collect()
    }

    pub fn from_engine(engine: Engine) -> Self {
        let mut session = Self::new();
        session.engine = engine;
        session.reset_after_load("Ready");
        session
    }

    pub fn from_snapshot(snapshot: EngineSnapshot) -> Result<Self, crate::DagcalError> {
        Ok(Self::from_engine(Engine::from_snapshot(snapshot)?))
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        self.engine.snapshot()
    }

    /// Returns the calculation engine used by this session.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Returns cached entries in stable-ID order.
    pub fn entries(&self) -> &[EntryView] {
        &self.entries
    }

    /// Returns the currently selected stable ID.
    pub fn selected_id(&self) -> Option<ExpressionId> {
        self.selected
    }

    /// Returns the current editor source.
    pub fn input_source(&self) -> &str {
        self.input.source()
    }

    /// Returns the latest user-facing status.
    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn restore_snapshot(
        &mut self,
        snapshot: EngineSnapshot,
        status: &str,
    ) -> Result<(), crate::DagcalError> {
        self.engine = Engine::from_snapshot(snapshot)?;
        self.reset_after_load(status);
        Ok(())
    }

    pub fn is_dirty_against(&self, snapshot: &EngineSnapshot) -> bool {
        self.engine.snapshot() != *snapshot
    }

    pub fn completion_candidates(&self) -> &[CompletionCandidate] {
        self.completion.items()
    }

    pub fn selected_completion_index(&self) -> Option<usize> {
        self.completion.selected_index()
    }

    pub fn completion_is_open(&self) -> bool {
        self.completion.is_open()
    }

    pub fn refresh_completions(&mut self) {
        let current_entry_id = self.current_input_entry_id();
        refresh_completion_state(
            &mut self.completion,
            &self.input,
            &self.engine,
            current_entry_id,
        );
    }

    pub fn close_completions(&mut self) {
        self.completion.clear();
    }

    pub fn move_completion_selection(&mut self, direction: CompletionDirection) {
        self.completion.move_selection(direction);
    }

    pub fn accept_selected_completion(&mut self) -> bool {
        let Some(change) =
            accept_selected_completion(&mut self.completion, &mut self.input, &mut self.status)
        else {
            return false;
        };
        if change == SessionChange::FocusInput {
            self.ensure_empty_draft_entry();
        }
        true
    }

    pub fn accept_completion(&mut self, index: usize) -> bool {
        if self.completion.select(index) {
            return self.accept_selected_completion();
        }
        false
    }

    pub fn open_entry_search(&mut self) -> SessionChange {
        self.close_completions();
        self.entry_search_open = true;
        SessionChange::FocusEntrySearch
    }

    pub fn close_entry_search(&mut self) -> SessionChange {
        self.reset_entry_filters();
        SessionChange::None
    }

    pub fn entry_search_changed(&mut self, value: String) -> SessionChange {
        self.entry_search_open = true;
        self.entry_search_query = value;
        SessionChange::None
    }

    pub fn entry_state_filter_changed(&mut self, filter: EntryStateFilter) -> SessionChange {
        self.entry_search_open = true;
        self.entry_state_filter = filter;
        SessionChange::None
    }

    pub fn clear_entry_search(&mut self) -> SessionChange {
        self.reset_entry_filters();
        SessionChange::None
    }

    pub fn input_changed(&mut self, value: String) -> SessionChange {
        self.input.set(value);
        let should_scroll = self.ensure_empty_draft_entry();
        self.refresh_completions();
        if should_scroll {
            SessionChange::ScrollToSelection
        } else {
            SessionChange::None
        }
    }

    pub fn submit_input(&mut self) -> SessionChange {
        if self.accept_selected_completion() {
            return SessionChange::FocusInput;
        }

        let source = self.input.source().trim().to_string();
        self.close_completions();

        if let Some(id) = self.editing {
            let result = self.engine.set_statement_by_id(id, source);
            self.refresh_affected(&result.execution.affected_ids);
            let id = result.execution.id;
            self.status = self.entry_status(id);
            self.load_edit_entry(id, SelectionStatus::Keep);
            SessionChange::None
        } else if let Some(id) = self.draft_entry.take() {
            self.save_new_entry_draft(id, source);
            SessionChange::ScrollToSelection
        } else {
            let execution = self.engine.execute(&source);
            self.refresh_affected(&execution.affected_ids);
            self.selected = Some(execution.id);
            self.status = format!(
                "{} = {}",
                execution.id,
                status_state_summary(&execution.state)
            );
            self.editing = None;
            self.input.clear();
            SessionChange::ScrollToSelection
        }
    }

    pub fn start_edit(&mut self, id: ExpressionId) -> SessionChange {
        self.close_completions();
        self.load_edit_entry(id, SelectionStatus::Set(format!("Editing {id}")));
        SessionChange::None
    }

    pub fn start_new_entry(&mut self) -> SessionChange {
        self.editing = None;
        self.draft_entry = None;
        self.input.clear();
        self.close_completions();
        if self.ensure_empty_draft_entry() {
            SessionChange::ScrollToSelection
        } else {
            SessionChange::None
        }
    }

    pub fn cancel_edit(&mut self) -> SessionChange {
        self.editing = None;
        self.input.clear();
        self.close_completions();
        self.status = "Edit cancelled".to_string();
        SessionChange::None
    }

    pub fn delete_entry(&mut self, id: ExpressionId) -> SessionChange {
        if self.engine.entry_by_id(id).is_none() {
            self.status = unavailable_status(id);
            return SessionChange::None;
        }

        self.perform_delete_entry(id)
    }

    pub fn recalculate_entry(&mut self, id: ExpressionId) -> SessionChange {
        if let Some(affected) = self.engine.recompute_entry_by_id(id) {
            self.refresh_affected(&affected);
            self.status = format!("Recalculated {id}");
        } else {
            self.status = unavailable_status(id);
        }

        SessionChange::None
    }

    pub fn recalculate_all(&mut self) -> SessionChange {
        self.engine.recompute_all();
        self.entries = self.engine.entries();
        self.status = "Recalculated all entries".to_string();
        SessionChange::None
    }

    pub fn insert_reference(&mut self, id: ExpressionId) -> SessionChange {
        let token = self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .map(entry_reference_token)
            .unwrap_or_else(|| id.to_string());

        self.input.insert_token(&token);
        self.refresh_completions();
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {token}");
        SessionChange::FocusInput
    }

    pub fn insert_constant(&mut self, name: String) -> SessionChange {
        self.input.insert_token(&name);
        self.refresh_completions();
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {name}");
        SessionChange::FocusInput
    }

    pub fn insert_function(&mut self, name: String) -> SessionChange {
        let token = format!("{name}()");
        self.input.insert_token(&token);
        self.refresh_completions();
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {token}");
        SessionChange::FocusInput
    }

    pub fn select_entry(&mut self, id: ExpressionId) -> SessionChange {
        if self.engine.entry_by_id(id).is_some() {
            self.set_selected_entry(id);
        } else {
            self.status = unavailable_status(id);
        }

        SessionChange::None
    }

    pub fn set_hovered_entry(&mut self, id: ExpressionId) -> SessionChange {
        self.hovered_entry = Some(id);
        SessionChange::None
    }

    pub fn clear_hovered_entry(&mut self, id: ExpressionId) -> SessionChange {
        if self.hovered_entry == Some(id) {
            self.hovered_entry = None;
        }

        SessionChange::None
    }

    pub fn select_hovered_entry(&mut self) -> SessionChange {
        if let Some(id) = self.hovered_entry {
            self.select_entry(id)
        } else {
            SessionChange::None
        }
    }

    pub fn clear(&mut self) -> SessionChange {
        self.engine.clear();
        self.entries = self.engine.entries();
        self.input.clear();
        self.close_completions();
        self.editing = None;
        self.draft_entry = None;
        self.selected = None;
        self.hovered_entry = None;
        self.reset_entry_filters();
        self.status = "Cleared".to_string();
        SessionChange::None
    }

    pub fn undo(&mut self) -> SessionChange {
        if self.engine.undo() {
            self.reset_after_history_restore("Undone");
        } else {
            self.status = "Nothing to undo".to_string();
        }

        SessionChange::None
    }

    pub fn redo(&mut self) -> SessionChange {
        if self.engine.redo() {
            self.reset_after_history_restore("Redone");
        } else {
            self.status = "Nothing to redo".to_string();
        }

        SessionChange::None
    }

    pub fn selection_navigation_enabled(&self) -> bool {
        (self.editing.is_none() && self.input.source().is_empty())
            || (self.editing.is_some() && self.input.is_loaded_selection())
    }

    pub fn delete_selected_entry(&mut self) -> SessionChange {
        let Some(id) = self.selected else {
            return SessionChange::None;
        };
        self.delete_entry(id)
    }

    pub fn move_selection(&mut self, direction: SelectionDirection) -> SessionChange {
        if !self.selection_navigation_enabled() {
            return SessionChange::None;
        }

        let visible_entries = self.filtered_entries();
        if visible_entries.is_empty() {
            return SessionChange::None;
        }

        let next_index = match self
            .selected
            .and_then(|id| visible_entries.iter().position(|entry| entry.id == id))
        {
            Some(index) => match direction {
                SelectionDirection::Previous => index.saturating_sub(1),
                SelectionDirection::Next => (index + 1).min(visible_entries.len() - 1),
            },
            None => match direction {
                SelectionDirection::Previous => visible_entries.len() - 1,
                SelectionDirection::Next => 0,
            },
        };

        self.set_selected_entry(visible_entries[next_index].id);
        SessionChange::None
    }

    pub fn ensure_empty_draft_entry(&mut self) -> bool {
        if self.editing.is_some() {
            return false;
        }

        if self
            .draft_entry
            .is_some_and(|id| self.engine.entry_by_id(id).is_some())
        {
            return false;
        }

        let (id, should_scroll) = if let Some(id) = self.find_empty_entry_id() {
            (id, true)
        } else {
            let execution = self.engine.execute("");
            self.refresh_affected(&execution.affected_ids);
            (execution.id, true)
        };

        self.draft_entry = Some(id);
        self.selected = Some(id);
        should_scroll
    }

    pub fn filtered_entries(&self) -> Vec<&EntryView> {
        self.filtered_entries_iter().collect()
    }

    /// Iterates over entries accepted by the active search and state filters.
    pub fn filtered_entries_iter(&self) -> impl DoubleEndedIterator<Item = &EntryView> {
        self.entries
            .iter()
            .filter(|entry| self.entry_matches_filters(entry))
    }

    pub fn filters_are_active(&self) -> bool {
        !self.entry_search_query.trim().is_empty()
            || self.entry_state_filter != EntryStateFilter::All
    }

    pub fn entry_count_status_text(&self) -> String {
        let visible_count = self.filtered_entries_iter().count();
        if self.filters_are_active() {
            format!("Entries: {visible_count} / {}", self.entries.len())
        } else {
            format!("Entries: {}", self.entries.len())
        }
    }

    pub fn selected_summary_text(&self, id: ExpressionId, entry: &EntryView) -> String {
        selected_summary_text(&self.engine, &self.entries, self.draft_entry, id, entry)
    }

    pub fn selected_error_text(&self, entry: &EntryView) -> String {
        selected_error_text(self.draft_entry, entry)
    }

    pub fn selected_compact_text(&self, id: ExpressionId, entry: &EntryView) -> String {
        selected_compact_text(&self.engine, &self.entries, self.draft_entry, id, entry)
    }

    pub fn reset_after_load(&mut self, status: &str) {
        self.entries = self.engine.entries();
        self.input.clear();
        self.close_completions();
        self.editing = None;
        self.draft_entry = None;
        self.hovered_entry = None;
        self.reset_entry_filters();
        self.selected = self.entries.last().map(|entry| entry.id);
        self.status = status.to_string();
    }

    fn perform_delete_entry(&mut self, id: ExpressionId) -> SessionChange {
        if let Some(removal) = self.engine.remove_entry_by_id(id) {
            self.entries
                .retain(|entry| entry.id != removal.removed_entry.id);
            let was_editing = self.editing == Some(id);
            if self.hovered_entry == Some(id) {
                self.hovered_entry = None;
            }
            if self.draft_entry == Some(id) {
                self.draft_entry = None;
            }
            self.refresh_affected(&removal.affected_ids);
            if self.selected == Some(id) {
                self.selected = self.entries.last().map(|entry| entry.id);
                if self.selected.is_none() {
                    self.editing = None;
                    self.input.clear();
                    self.close_completions();
                }
            }
            if was_editing {
                self.editing = None;
                self.input.clear();
                self.close_completions();
            }
            self.status = format!("Removed {id}");
        } else {
            self.status = unavailable_status(id);
        }

        SessionChange::None
    }

    fn load_edit_entry(&mut self, id: ExpressionId, status: SelectionStatus) -> bool {
        match self.engine.entry_by_id(id) {
            Some(entry) => {
                self.draft_entry = None;
                self.input.load_selection(entry_expression_source(&entry));
                self.editing = Some(id);
                self.selected = Some(id);
                if let SelectionStatus::Set(status) = status {
                    self.status = status;
                }
                true
            }
            None => {
                if !matches!(status, SelectionStatus::Keep) {
                    self.status = unavailable_status(id);
                }
                false
            }
        }
    }

    fn current_input_entry_id(&self) -> Option<ExpressionId> {
        self.editing.or(self.draft_entry)
    }

    fn set_selected_entry(&mut self, id: ExpressionId) {
        let changed = self.selected != Some(id);
        self.selected = Some(id);

        if changed && self.editing.is_some() {
            self.editing = None;
            self.input.clear();
            self.close_completions();
            self.status = "Edit cancelled".to_string();
        }
    }

    fn find_empty_entry_id(&self) -> Option<ExpressionId> {
        self.entries
            .iter()
            .find(|entry| entry.source.trim().is_empty())
            .map(|entry| entry.id)
    }

    fn save_new_entry_draft(&mut self, id: ExpressionId, source: String) {
        let result = self.engine.set_statement_by_id(id, source);
        self.refresh_affected(&result.execution.affected_ids);
        self.remove_redirected_draft(id, result.execution.id);
        let id = result.execution.id;
        self.selected = Some(id);
        self.status = format!("{id} = {}", status_state_summary(&result.execution.state));
        self.editing = None;
        self.input.clear();
        self.close_completions();
    }

    fn remove_redirected_draft(&mut self, requested_id: ExpressionId, saved_id: ExpressionId) {
        if requested_id == saved_id {
            return;
        }

        if let Some(removal) = self.engine.remove_entry_by_id(requested_id) {
            self.entries
                .retain(|entry| entry.id != removal.removed_entry.id);
            self.refresh_affected(&removal.affected_ids);
        }

        if self.draft_entry == Some(requested_id) {
            self.draft_entry = Some(saved_id);
        }
    }

    fn refresh_affected(&mut self, ids: &BTreeSet<ExpressionId>) {
        for id in ids {
            match self.engine.entry_by_id(*id) {
                Some(entry) => self.upsert_cached_entry(entry),
                None => self.entries.retain(|entry| entry.id != *id),
            }
        }
    }

    fn upsert_cached_entry(&mut self, entry: EntryView) {
        match self
            .entries
            .binary_search_by_key(&entry.id, |entry| entry.id)
        {
            Ok(index) => self.entries[index] = entry,
            Err(index) => self.entries.insert(index, entry),
        }
    }

    fn entry_status(&self, id: ExpressionId) -> String {
        match self.engine.entry_by_id(id) {
            Some(entry) => format!("{id} = {}", status_state_summary(&entry.state)),
            None => format!("{id} updated"),
        }
    }

    fn reset_after_history_restore(&mut self, status: &str) {
        self.entries = self.engine.entries();
        self.input.clear();
        self.close_completions();
        self.editing = None;
        self.draft_entry = None;
        self.hovered_entry = None;
        self.selected = self
            .selected
            .filter(|id| self.engine.entry_by_id(*id).is_some())
            .or_else(|| self.entries.last().map(|entry| entry.id));
        self.status = status.to_string();
    }

    fn entry_matches_filters(&self, entry: &EntryView) -> bool {
        if !self.entry_matches_state_filter(entry) {
            return false;
        }

        let query = self.entry_search_query.trim();
        if query.is_empty() {
            return true;
        }

        entry_search_text(entry).contains(&query.to_lowercase())
    }

    fn entry_matches_state_filter(&self, entry: &EntryView) -> bool {
        match self.entry_state_filter {
            EntryStateFilter::All => true,
            EntryStateFilter::Values => matches!(entry.state, EntryState::Value(_)),
            EntryStateFilter::Errors => matches!(entry.state, EntryState::Error(_)),
        }
    }

    fn reset_entry_filters(&mut self) {
        self.entry_search_open = false;
        self.entry_search_query.clear();
        self.entry_state_filter = EntryStateFilter::All;
    }
}

enum SelectionStatus {
    Keep,
    Set(String),
}

fn unavailable_status(id: ExpressionId) -> String {
    format!("{id} is not available")
}

fn entry_search_text(entry: &EntryView) -> String {
    let expression = entry_expression_source(entry);
    let state = state_summary(&entry.state);
    let name = entry.name.as_deref().unwrap_or_default();

    format!("{} {name} {expression} {} {state}", entry.id, entry.source).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompletionKind, Number};

    #[test]
    fn submit_input_adds_entry_and_selects_it() {
        let mut session = AppSession::new();

        session.input.set("10".to_string());
        let change = session.submit_input();

        assert_eq!(change, SessionChange::ScrollToSelection);
        assert_eq!(session.entries.len(), 1);
        assert_eq!(session.selected, Some(ExpressionId::new(1)));
        assert_eq!(session.status, "$1 = 10");
    }

    #[test]
    fn submit_input_keeps_syntax_error_status_single_line() {
        let mut session = AppSession::new();

        session.input.set("1 +".to_string());
        session.submit_input();

        assert_eq!(session.status, "$1 = error: syntax error");
        assert!(!session.status.contains('\n'));
    }

    #[test]
    fn submit_input_keeps_eval_error_status_compact() {
        let mut session = AppSession::new();

        session.input.set("a + 1".to_string());
        session.submit_input();

        assert_eq!(session.status, "$1 = error: unknown reference `a`");
    }

    #[test]
    fn edit_updates_existing_entry() {
        let mut session = AppSession::new();
        session.input.set("10".to_string());
        session.submit_input();

        session.start_edit(ExpressionId::new(1));
        session.input.set("20".to_string());
        session.submit_input();

        assert_eq!(
            session.entries[0].state,
            EntryState::Value(Number::from(20))
        );
        assert_eq!(session.editing, Some(ExpressionId::new(1)));
        assert_eq!(session.input.source(), "20");
    }

    #[test]
    fn delete_selected_removes_entry_and_updates_status() {
        let mut session = AppSession::new();
        session.input.set("10".to_string());
        session.submit_input();

        session.delete_entry(ExpressionId::new(1));

        assert!(session.entries.is_empty());
        assert_eq!(session.selected, None);
        assert_eq!(session.status, "Removed $1");
    }

    #[test]
    fn undo_and_redo_restore_entry_cache() {
        let mut session = AppSession::new();
        session.input.set("10".to_string());
        session.submit_input();

        session.undo();
        assert!(session.entries.is_empty());
        assert_eq!(session.status, "Undone");

        session.redo();
        assert_eq!(session.entries.len(), 1);
        assert_eq!(session.status, "Redone");
    }

    #[test]
    fn input_change_creates_empty_draft_entry() {
        let mut session = AppSession::new();

        session.input_changed("1 +".to_string());

        assert_eq!(session.entries.len(), 1);
        assert_eq!(session.entries[0].source, "");
        assert_eq!(session.draft_entry, Some(ExpressionId::new(1)));
        assert_eq!(session.selected, Some(ExpressionId::new(1)));
    }

    #[test]
    fn entry_search_and_state_filter_combine() {
        let mut session = AppSession::new();
        session.input.set("subtotal = 10".to_string());
        session.submit_input();
        session.input.set("subtotal / 0".to_string());
        session.submit_input();

        session.entry_search_changed("subtotal".to_string());
        session.entry_state_filter_changed(EntryStateFilter::Errors);

        let ids = session
            .filtered_entries()
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![ExpressionId::new(2)]);
    }

    #[test]
    fn completion_accepts_named_entry() {
        let mut session = AppSession::new();
        session.input.set("subtotal = 10".to_string());
        session.submit_input();

        session.input_changed("sub".to_string());
        assert!(session.completion_candidates().iter().any(|candidate| {
            candidate.label == "subtotal" && candidate.kind == CompletionKind::Entry
        }));

        session.submit_input();

        assert_eq!(session.input.source(), "subtotal");
        assert_eq!(session.status, "Inserted subtotal");
    }

    #[test]
    fn editing_entry_excludes_itself_from_named_completions() {
        let mut session = AppSession::new();
        session.input.set("subtotal = 10".to_string());
        session.submit_input();

        session.start_edit(ExpressionId::new(1));
        session.input_changed("sub".to_string());

        assert!(session.completion_candidates().iter().all(|candidate| {
            candidate.label != "subtotal" || candidate.kind != CompletionKind::Entry
        }));
    }

    #[test]
    fn editing_entry_excludes_its_own_result_completion() {
        let mut session = AppSession::new();
        session.input.set("subtotal = 10".to_string());
        session.submit_input();
        session.input.set("20".to_string());
        session.submit_input();

        session.start_edit(ExpressionId::new(1));
        session.input_changed("$".to_string());

        assert!(
            session
                .completion_candidates()
                .iter()
                .all(|candidate| candidate.label != "$1")
        );
        assert!(
            session
                .completion_candidates()
                .iter()
                .any(|candidate| candidate.label == "$2")
        );
    }

    #[test]
    fn new_draft_entry_excludes_its_own_result_completion() {
        let mut session = AppSession::new();
        session.input.set("10".to_string());
        session.submit_input();

        session.input_changed("$".to_string());

        assert_eq!(session.draft_entry, Some(ExpressionId::new(2)));
        assert!(
            session
                .completion_candidates()
                .iter()
                .any(|candidate| candidate.label == "$1")
        );
        assert!(
            session
                .completion_candidates()
                .iter()
                .all(|candidate| candidate.label != "$2")
        );
    }
}
