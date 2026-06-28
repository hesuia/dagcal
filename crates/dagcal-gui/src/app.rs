use crate::formatting::{
    entry_expression_source, entry_reference_token, needs_space_before_reference, state_summary,
};
use dagcal_core::{Engine, EntryView, ExpressionId};
use iced::keyboard::{self, Key, key};
use iced::{Subscription, Task};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    Submit,
    NewEntry,
    Edit(ExpressionId),
    CancelEdit,
    Delete(ExpressionId),
    InsertReference(ExpressionId),
    Select(ExpressionId),
    Keyboard(keyboard::Event),
    Clear,
}

pub struct GuiApp {
    pub(crate) engine: Engine,
    pub(crate) entries: Vec<EntryView>,
    pub(crate) input: Draft,
    pub(crate) editing: Option<ExpressionId>,
    pub(crate) draft_entry: Option<ExpressionId>,
    pub(crate) selected: Option<ExpressionId>,
    pub(crate) status: String,
}

impl GuiApp {
    pub(crate) fn new() -> (Self, Task<Message>) {
        (
            Self {
                engine: Engine::new(),
                entries: Vec::new(),
                input: Draft::default(),
                editing: None,
                draft_entry: None,
                selected: None,
                status: "Ready".to_string(),
            },
            Task::none(),
        )
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input.set(value);
                self.ensure_empty_draft_entry();
            }
            Message::Submit => self.submit_input(),
            Message::NewEntry => self.start_new_entry(),
            Message::Edit(id) => self.start_edit(id),
            Message::CancelEdit => self.cancel_edit(),
            Message::Delete(id) => self.delete_entry(id),
            Message::InsertReference(id) => self.insert_reference(id),
            Message::Select(id) => self.select_entry(id),
            Message::Keyboard(event) => self.handle_keyboard_event(event),
            Message::Clear => self.clear(),
        }

        Task::none()
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        if self.selection_navigation_enabled() {
            keyboard::listen().map(Message::Keyboard)
        } else {
            Subscription::none()
        }
    }

    fn submit_input(&mut self) {
        let source = self.input.source().trim().to_string();

        if let Some(id) = self.editing {
            let result = self.engine.set_statement_by_id(id, source);
            self.refresh_affected(&result.execution.affected_ids);
            let id = result.execution.id;
            self.status = match self.engine.entry_by_id(id) {
                Some(entry) => format!("{id} = {}", state_summary(&entry.state)),
                None => format!("{id} updated"),
            };
            self.load_selected_entry(id, SelectionStatus::Keep);
        } else if let Some(id) = self.draft_entry.take() {
            self.save_new_entry_draft(id, source);
        } else {
            let execution = self.engine.execute(&source);
            self.refresh_affected(&execution.affected_ids);
            self.selected = Some(execution.id);
            self.status = format!("{} = {}", execution.id, state_summary(&execution.state));
            self.editing = None;
            self.input.clear();
        }
    }

    fn start_edit(&mut self, id: ExpressionId) {
        self.load_selected_entry(id, SelectionStatus::Set(format!("Editing {id}")));
    }

    fn start_new_entry(&mut self) {
        self.editing = None;
        self.draft_entry = None;
        self.input.clear();
        self.ensure_empty_draft_entry();
    }

    fn load_selected_entry(&mut self, id: ExpressionId, status: SelectionStatus) -> bool {
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
                    self.status = format!("{id} is not available");
                }
                false
            }
        }
    }

    fn cancel_edit(&mut self) {
        self.editing = None;
        self.input.clear();
        self.status = "Edit cancelled".to_string();
    }

    fn delete_entry(&mut self, id: ExpressionId) {
        if let Some(removal) = self.engine.remove_entry_by_id(id) {
            self.entries
                .retain(|entry| entry.id != removal.removed_entry.id);
            let was_editing = self.editing == Some(id);
            if self.draft_entry == Some(id) {
                self.draft_entry = None;
            }
            self.refresh_affected(&removal.affected_ids);
            if self.selected == Some(id) {
                self.selected = self.entries.last().map(|entry| entry.id);
                if let Some(id) = self.selected {
                    self.load_selected_entry(id, SelectionStatus::Keep);
                } else {
                    self.editing = None;
                    self.input.clear();
                }
            }
            if was_editing && self.selected.is_none() {
                self.editing = None;
                self.input.clear();
            }
            self.status = format!("Removed {id}");
        } else {
            self.status = format!("{id} is not available");
        }
    }

    fn insert_reference(&mut self, id: ExpressionId) {
        let token = self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .map(entry_reference_token)
            .unwrap_or_else(|| id.to_string());

        self.input.insert_token(&token);
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {token}");
    }

    fn select_entry(&mut self, id: ExpressionId) {
        if !self.load_selected_entry(id, SelectionStatus::Keep) {
            self.status = format!("{id} is not available");
        }
    }

    fn handle_keyboard_event(&mut self, event: keyboard::Event) {
        match event {
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowUp),
                ..
            } => self.move_selection(SelectionDirection::Previous),
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowDown),
                ..
            } => self.move_selection(SelectionDirection::Next),
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Delete),
                ..
            } => self.delete_selected_entry(),
            _ => {}
        }
    }

    fn delete_selected_entry(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        self.delete_entry(id);
    }

    fn move_selection(&mut self, direction: SelectionDirection) {
        if !self.selection_navigation_enabled() || self.entries.is_empty() {
            return;
        }

        let next_index = match self
            .selected
            .and_then(|id| self.entries.iter().position(|entry| entry.id == id))
        {
            Some(index) => match direction {
                SelectionDirection::Previous => index.saturating_sub(1),
                SelectionDirection::Next => (index + 1).min(self.entries.len() - 1),
            },
            None => match direction {
                SelectionDirection::Previous => self.entries.len() - 1,
                SelectionDirection::Next => 0,
            },
        };

        self.load_selected_entry(self.entries[next_index].id, SelectionStatus::Keep);
    }

    fn selection_navigation_enabled(&self) -> bool {
        (self.editing.is_none() && self.input.source().is_empty())
            || (self.editing.is_some() && self.input.is_loaded_selection())
    }

    fn clear(&mut self) {
        self.engine = Engine::new();
        self.entries.clear();
        self.input.clear();
        self.editing = None;
        self.draft_entry = None;
        self.selected = None;
        self.status = "Cleared".to_string();
    }

    fn ensure_empty_draft_entry(&mut self) {
        if self.editing.is_some() {
            return;
        }

        if self
            .draft_entry
            .is_some_and(|id| self.engine.entry_by_id(id).is_some())
        {
            return;
        }

        let id = if let Some(id) = self.find_empty_entry_id() {
            id
        } else {
            let execution = self.engine.execute("");
            self.refresh_affected(&execution.affected_ids);
            execution.id
        };

        self.draft_entry = Some(id);
        self.selected = Some(id);
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
        self.status = format!("{id} = {}", state_summary(&result.execution.state));
        self.editing = None;
        self.input.clear();
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
}

#[derive(Debug, Clone, Copy)]
enum SelectionDirection {
    Previous,
    Next,
}

enum SelectionStatus {
    Keep,
    Set(String),
}

#[derive(Debug, Default)]
pub(crate) struct Draft {
    source: String,
    loaded_selection: Option<String>,
}

impl Draft {
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    fn set(&mut self, source: String) {
        self.source = source;
        self.loaded_selection = None;
    }

    fn load_selection(&mut self, source: String) {
        self.loaded_selection = Some(source.clone());
        self.source = source;
    }

    fn clear(&mut self) {
        self.source.clear();
        self.loaded_selection = None;
    }

    fn is_loaded_selection(&self) -> bool {
        self.loaded_selection
            .as_deref()
            .is_some_and(|source| source == self.source)
    }

    fn insert_token(&mut self, token: &str) {
        if needs_space_before_reference(&self.source) {
            self.source.push(' ');
        }
        self.source.push_str(token);
        self.source.push(' ');
        self.loaded_selection = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_core::{EntryState, Number};

    #[test]
    fn draft_inserts_result_reference_without_replacing_saved_source() {
        let mut draft = Draft::default();

        draft.set("1 +".to_string());
        draft.insert_token("$1");

        assert_eq!(draft.source(), "1 +$1 ");
    }

    #[test]
    fn use_inserts_name_for_named_entry() {
        let (mut app, _) = GuiApp::new();
        app.input.set("x = 10".to_string());
        app.submit_input();

        app.insert_reference(ExpressionId::new(1));

        assert_eq!(app.input.source(), "x ");
        assert_eq!(app.status, "Inserted x");
    }

    #[test]
    fn use_inserts_result_id_for_unnamed_entry() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();

        app.insert_reference(ExpressionId::new(1));

        assert_eq!(app.input.source(), "$1 ");
        assert_eq!(app.status, "Inserted $1");
    }

    #[test]
    fn selecting_entry_loads_source_without_replacing_status() {
        let (mut app, _) = GuiApp::new();
        app.input.set("21".to_string());
        app.submit_input();
        app.status = "Ready".to_string();

        app.select_entry(ExpressionId::new(1));

        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.editing, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "21");
        assert_eq!(app.status, "Ready");
    }

    #[test]
    fn arrow_navigation_selects_first_or_last_entry_when_none_selected() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("20".to_string());
        app.submit_input();
        app.selected = None;
        app.editing = None;
        app.input.clear();

        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "10");

        app.selected = None;
        app.editing = None;
        app.input.clear();
        app.move_selection(SelectionDirection::Previous);
        assert_eq!(app.selected, Some(ExpressionId::new(2)));
        assert_eq!(app.input.source(), "20");
    }

    #[test]
    fn arrow_navigation_moves_selection_and_stops_at_edges() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("20".to_string());
        app.submit_input();

        app.selected = Some(ExpressionId::new(1));
        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(2)));
        assert_eq!(app.input.source(), "20");

        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(2)));
        assert_eq!(app.input.source(), "20");

        app.move_selection(SelectionDirection::Previous);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "10");

        app.move_selection(SelectionDirection::Previous);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "10");
    }

    #[test]
    fn arrow_navigation_is_disabled_while_selected_input_is_modified() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("20".to_string());
        app.submit_input();

        app.select_entry(ExpressionId::new(1));
        app.input.set("draft".to_string());
        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "draft");
    }

    #[test]
    fn submit_edit_recomputes_dependents() {
        let (mut app, _) = GuiApp::new();
        app.input.set("base = 10".to_string());
        app.submit_input();
        app.input.set("$1 * 2".to_string());
        app.submit_input();

        app.start_edit(ExpressionId::new(1));
        assert_eq!(app.input.source(), "base = 10");

        app.input.set("base = 20".to_string());
        app.submit_input();

        assert_eq!(app.entries[0].state, EntryState::Value(Number::from(20)));
        assert_eq!(app.entries[0].source, "20");
        assert_eq!(app.entries[1].state, EntryState::Value(Number::from(40)));
    }

    #[test]
    fn submit_after_select_updates_existing_entry() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();

        app.select_entry(ExpressionId::new(1));
        app.input.set("30".to_string());
        app.submit_input();

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].state, EntryState::Value(Number::from(30)));
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.editing, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "30");
    }

    #[test]
    fn edit_input_does_not_update_result_column_before_submit() {
        let (mut app, _) = GuiApp::new();
        app.input.set("base = 10".to_string());
        app.submit_input();
        app.input.set("$1 * 2".to_string());
        app.submit_input();

        app.start_edit(ExpressionId::new(1));
        let _ = app.update(Message::InputChanged("base = 20".to_string()));

        assert_eq!(app.entries[0].state, EntryState::Value(Number::from(10)));
        assert_eq!(app.entries[0].source, "10");
        assert_eq!(app.entries[1].state, EntryState::Value(Number::from(20)));
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.editing, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "base = 20");
    }

    #[test]
    fn edit_input_does_not_show_parse_errors_before_submit() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();

        app.start_edit(ExpressionId::new(1));
        let _ = app.update(Message::InputChanged("1 +".to_string()));

        assert_eq!(app.entries[0].state, EntryState::Value(Number::from(10)));
        assert_eq!(app.entries[0].source, "10");
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.editing, Some(ExpressionId::new(1)));
    }

    #[test]
    fn delete_keeps_later_ids_available() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("20".to_string());
        app.submit_input();

        app.delete_entry(ExpressionId::new(1));

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(2));
    }

    #[test]
    fn delete_key_removes_selected_entry() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("20".to_string());
        app.submit_input();
        app.editing = None;
        app.input.clear();
        app.selected = Some(ExpressionId::new(1));

        app.handle_keyboard_event(keyboard::Event::KeyPressed {
            key: Key::Named(key::Named::Delete),
            modified_key: Key::Named(key::Named::Delete),
            physical_key: key::Physical::Code(key::Code::Delete),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(2));
        assert_eq!(app.selected, Some(ExpressionId::new(2)));
        assert_eq!(app.status, "Removed $1");
    }

    #[test]
    fn submitting_empty_new_expression_creates_history_entry() {
        let (mut app, _) = GuiApp::new();

        app.submit_input();

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.entries[0].source, "");
        assert!(matches!(app.entries[0].state, EntryState::Error(_)));
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "");
    }

    #[test]
    fn new_expression_input_keeps_expression_in_input_and_empty_draft_in_history() {
        let (mut app, _) = GuiApp::new();

        let _ = app.update(Message::InputChanged("1".to_string()));
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.entries[0].source, "");
        assert!(matches!(app.entries[0].state, EntryState::Error(_)));
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "1");

        let _ = app.update(Message::InputChanged("1 +".to_string()));
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.entries[0].source, "");
        assert!(matches!(app.entries[0].state, EntryState::Error(_)));
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "1 +");
    }

    #[test]
    fn new_expression_input_saves_named_definition_on_submit() {
        let (mut app, _) = GuiApp::new();

        let _ = app.update(Message::InputChanged("x=2".to_string()));
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].source, "");
        assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));

        let _ = app.update(Message::Submit);

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.entries[0].name.as_deref(), Some("x"));
        assert_eq!(app.entries[0].source, "2");
        assert_eq!(app.entries[0].state, EntryState::Value(Number::from(2)));
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.draft_entry, None);
    }

    #[test]
    fn selected_entry_can_be_saved_as_named_definition() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();

        app.select_entry(ExpressionId::new(1));
        app.input.set("x=2".to_string());
        app.submit_input();

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.entries[0].name.as_deref(), Some("x"));
        assert_eq!(app.entries[0].source, "2");
        assert_eq!(app.entries[0].state, EntryState::Value(Number::from(2)));
        assert_eq!(app.input.source(), "x = 2");
    }

    #[test]
    fn new_entry_clears_input_and_creates_empty_draft_entry() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("draft".to_string());

        let _ = app.update(Message::NewEntry);

        assert_eq!(app.entries.len(), 2);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.entries[0].source, "10");
        assert_eq!(app.entries[1].id, ExpressionId::new(2));
        assert_eq!(app.entries[1].source, "");
        assert!(matches!(app.entries[1].state, EntryState::Error(_)));
        assert_eq!(app.selected, Some(ExpressionId::new(2)));
        assert_eq!(app.draft_entry, Some(ExpressionId::new(2)));
        assert_eq!(app.editing, None);
        assert_eq!(app.input.source(), "");
    }

    #[test]
    fn new_entry_from_editing_does_not_overwrite_selected_entry() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.select_entry(ExpressionId::new(1));

        let _ = app.update(Message::NewEntry);
        let _ = app.update(Message::InputChanged("20".to_string()));

        assert_eq!(app.entries.len(), 2);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.entries[0].source, "10");
        assert_eq!(app.entries[1].id, ExpressionId::new(2));
        assert_eq!(app.entries[1].source, "");
        assert!(matches!(app.entries[1].state, EntryState::Error(_)));
        assert_eq!(app.selected, Some(ExpressionId::new(2)));
        assert_eq!(app.draft_entry, Some(ExpressionId::new(2)));
        assert_eq!(app.editing, None);
        assert_eq!(app.input.source(), "20");
    }

    #[test]
    fn new_entry_after_empty_submit_reuses_existing_empty_entry() {
        let (mut app, _) = GuiApp::new();

        app.submit_input();
        let _ = app.update(Message::NewEntry);

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(1));
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
        assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
        assert_eq!(app.input.source(), "");
        assert_eq!(app.editing, None);
    }
}
