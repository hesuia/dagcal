use super::{App, Mode};
use dagcal_app::{
    AppAction, CompletionDirection, EntryStateFilter, EntryView, ExpressionId, SelectionDirection,
};

impl App {
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn move_next(&mut self) {
        self.session
            .dispatch(AppAction::MoveSelection(SelectionDirection::Next));
    }

    pub fn move_previous(&mut self) {
        self.session
            .dispatch(AppAction::MoveSelection(SelectionDirection::Previous));
    }

    pub fn start_insert(&mut self) {
        self.mode = Mode::Insert;
        self.session.editing = None;
        self.session.input.clear();
        self.session.close_completions();
        self.session.status = "enter expression".to_string();
    }

    pub fn start_edit(&mut self) {
        let Some(id) = self.selected_id() else {
            self.session.status = "no entry selected".to_string();
            return;
        };

        self.session.dispatch(AppAction::StartEdit(id));
        self.mode = Mode::Edit;
    }

    pub fn open_search(&mut self) {
        self.mode = Mode::Search;
        self.session.dispatch(AppAction::OpenEntrySearch);
        self.session.status = "search entries".to_string();
    }

    pub fn close_search(&mut self) {
        self.session.dispatch(AppAction::CloseEntrySearch);
        self.mode = Mode::Normal;
    }

    pub fn push_search(&mut self, ch: char) {
        if self.mode != Mode::Search {
            return;
        }

        let mut query = self.session.entry_search_query.clone();
        query.push(ch);
        self.session.dispatch(AppAction::EntrySearchChanged(query));
    }

    pub fn backspace_search(&mut self) {
        if self.mode != Mode::Search {
            return;
        }

        let mut query = self.session.entry_search_query.clone();
        query.pop();
        self.session.dispatch(AppAction::EntrySearchChanged(query));
    }

    pub fn cycle_entry_state_filter(&mut self) {
        let next = match self.session.entry_state_filter {
            EntryStateFilter::All => EntryStateFilter::Values,
            EntryStateFilter::Values => EntryStateFilter::Errors,
            EntryStateFilter::Errors => EntryStateFilter::All,
        };
        self.session
            .dispatch(AppAction::EntryStateFilterChanged(next));
    }

    pub fn cancel_input(&mut self) {
        if self.session.completion_is_open() {
            self.session.close_completions();
            return;
        }

        self.mode = Mode::Normal;
        self.session.cancel_edit();
        self.remove_empty_draft();
    }

    pub fn push_input(&mut self, ch: char) {
        if !self.input_mode_active() {
            return;
        }

        let mut source = self.session.input.source().to_string();
        source.push(ch);
        self.session.input.set(source);
        self.session.refresh_completions();
    }

    pub fn backspace_input(&mut self) {
        if !self.input_mode_active() {
            return;
        }

        let mut source = self.session.input.source().to_string();
        source.pop();
        self.session.input.set(source);
        self.session.refresh_completions();
    }

    pub fn move_completion_next(&mut self) {
        self.session
            .move_completion_selection(CompletionDirection::Next);
    }

    pub fn move_completion_previous(&mut self) {
        self.session
            .move_completion_selection(CompletionDirection::Previous);
    }

    pub fn accept_completion(&mut self) {
        self.session.accept_selected_completion();
        self.remove_empty_draft();
    }

    pub fn submit_input(&mut self) {
        if !self.input_mode_active() {
            return;
        }

        if self.session.completion_is_open() {
            self.accept_completion();
            return;
        }

        if self.session.input.source().trim().is_empty() {
            self.session.status = "empty expression".to_string();
            return;
        }

        self.session.submit_input();
        self.mode = Mode::Normal;
    }

    pub fn delete_selected(&mut self) {
        if self.selected_id().is_none() {
            self.session.status = "no entry selected".to_string();
            return;
        }

        if let Some(id) = self.selected_id() {
            self.session.dispatch(AppAction::DeleteEntry(id));
        }
    }

    pub fn insert_selected_reference(&mut self) {
        let Some(id) = self.selected_id() else {
            self.session.status = "no entry selected".to_string();
            return;
        };

        self.session.dispatch(AppAction::InsertReference(id));
        self.mode = Mode::Insert;
    }

    pub fn recalculate_selected(&mut self) {
        let Some(id) = self.selected_id() else {
            self.session.status = "no entry selected".to_string();
            return;
        };

        self.session.dispatch(AppAction::RecalculateEntry(id));
    }

    pub fn recalculate_all(&mut self) {
        self.session.dispatch(AppAction::RecalculateAll);
        self.mode = Mode::Normal;
    }

    pub fn clear(&mut self) {
        self.session.dispatch(AppAction::Clear);
        self.mode = Mode::Normal;
        self.session.status = "cleared".to_string();
    }

    pub fn undo(&mut self) {
        self.session.dispatch(AppAction::Undo);
        self.mode = Mode::Normal;
        if self.session.status == "Undone" {
            self.session.status = "undone".to_string();
        } else if self.session.status == "Nothing to undo" {
            self.session.status = "nothing to undo".to_string();
        }
    }

    pub fn redo(&mut self) {
        self.session.dispatch(AppAction::Redo);
        self.mode = Mode::Normal;
        if self.session.status == "Redone" {
            self.session.status = "redone".to_string();
        } else if self.session.status == "Nothing to redo" {
            self.session.status = "nothing to redo".to_string();
        }
    }

    fn input_mode_active(&self) -> bool {
        matches!(self.mode, Mode::Insert | Mode::Edit)
    }

    fn selected_entry(&self) -> Option<EntryView> {
        let selected = self.session.selected?;
        self.session
            .entries
            .iter()
            .find(|entry| entry.id == selected)
            .cloned()
    }

    fn selected_id(&self) -> Option<ExpressionId> {
        self.selected_entry().map(|entry| entry.id)
    }

    fn remove_empty_draft(&mut self) {
        let Some(id) = self.session.draft_entry else {
            return;
        };

        if self
            .session
            .entries
            .iter()
            .any(|entry| entry.id == id && entry.source.trim().is_empty())
        {
            let status = self.session.status.clone();
            self.session.delete_entry(id);
            self.session.status = status;
        }
        self.session.draft_entry = None;
    }
}
