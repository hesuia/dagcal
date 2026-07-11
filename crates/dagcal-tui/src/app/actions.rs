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
        self.session.dispatch(AppAction::ResetInput);
        self.set_status("enter expression");
    }

    pub fn start_edit(&mut self) {
        let Some(id) = self.selected_id() else {
            self.set_status("no entry selected");
            return;
        };

        self.session.dispatch(AppAction::StartEdit(id));
        self.mode = Mode::Edit;
    }

    pub fn open_search(&mut self) {
        self.mode = Mode::Search;
        self.session.dispatch(AppAction::OpenEntrySearch);
        self.set_status("search entries");
    }

    pub fn close_search(&mut self) {
        self.session.dispatch(AppAction::CloseEntrySearch);
        self.mode = Mode::Normal;
    }

    pub fn push_search(&mut self, ch: char) {
        if self.mode != Mode::Search {
            return;
        }

        let mut query = self.session.entry_search_query().to_string();
        query.push(ch);
        self.session.dispatch(AppAction::EntrySearchChanged(query));
    }

    pub fn backspace_search(&mut self) {
        if self.mode != Mode::Search {
            return;
        }

        let mut query = self.session.entry_search_query().to_string();
        query.pop();
        self.session.dispatch(AppAction::EntrySearchChanged(query));
    }

    pub fn cycle_entry_state_filter(&mut self) {
        let next = match self.session.entry_state_filter() {
            EntryStateFilter::All => EntryStateFilter::Values,
            EntryStateFilter::Values => EntryStateFilter::Errors,
            EntryStateFilter::Errors => EntryStateFilter::All,
        };
        self.session
            .dispatch(AppAction::EntryStateFilterChanged(next));
    }

    pub fn cancel_input(&mut self) {
        if self.session.completion_is_open() {
            self.session.dispatch(AppAction::CloseCompletions);
            return;
        }

        self.mode = Mode::Normal;
        self.session.dispatch(AppAction::CancelEdit);
        self.remove_empty_draft();
    }

    pub fn push_input(&mut self, ch: char) {
        if !self.input_mode_active() {
            return;
        }

        let mut source = self.session.input_source().to_string();
        source.push(ch);
        self.session.dispatch(AppAction::InputEdited(source));
    }

    pub fn backspace_input(&mut self) {
        if !self.input_mode_active() {
            return;
        }

        let mut source = self.session.input_source().to_string();
        source.pop();
        self.session.dispatch(AppAction::InputEdited(source));
    }

    pub fn move_completion_next(&mut self) {
        self.session
            .dispatch(AppAction::MoveCompletion(CompletionDirection::Next));
    }

    pub fn move_completion_previous(&mut self) {
        self.session
            .dispatch(AppAction::MoveCompletion(CompletionDirection::Previous));
    }

    pub fn accept_completion(&mut self) {
        self.session.dispatch(AppAction::SubmitInput);
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

        if self.session.input_source().trim().is_empty() {
            self.set_status("empty expression");
            return;
        }

        self.session.dispatch(AppAction::SubmitInput);
        self.mode = Mode::Normal;
    }

    pub fn delete_selected(&mut self) {
        if self.selected_id().is_none() {
            self.set_status("no entry selected");
            return;
        }

        if let Some(id) = self.selected_id() {
            self.session.dispatch(AppAction::DeleteEntry(id));
        }
    }

    pub fn insert_selected_reference(&mut self) {
        let Some(id) = self.selected_id() else {
            self.set_status("no entry selected");
            return;
        };

        self.session.dispatch(AppAction::InsertReference(id));
        self.mode = Mode::Insert;
    }

    pub fn recalculate_selected(&mut self) {
        let Some(id) = self.selected_id() else {
            self.set_status("no entry selected");
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
        self.set_status("cleared");
    }

    pub fn undo(&mut self) {
        self.session.dispatch(AppAction::Undo);
        self.mode = Mode::Normal;
        if self.session.status() == "Undone" {
            self.set_status("undone");
        } else if self.session.status() == "Nothing to undo" {
            self.set_status("nothing to undo");
        }
    }

    pub fn redo(&mut self) {
        self.session.dispatch(AppAction::Redo);
        self.mode = Mode::Normal;
        if self.session.status() == "Redone" {
            self.set_status("redone");
        } else if self.session.status() == "Nothing to redo" {
            self.set_status("nothing to redo");
        }
    }

    fn input_mode_active(&self) -> bool {
        matches!(self.mode, Mode::Insert | Mode::Edit)
    }

    fn selected_entry(&self) -> Option<EntryView> {
        let selected = self.session.selected_id()?;
        self.session.entry(selected).cloned()
    }

    fn selected_id(&self) -> Option<ExpressionId> {
        self.selected_entry().map(|entry| entry.id)
    }

    fn remove_empty_draft(&mut self) {
        self.session.dispatch(AppAction::DiscardEmptyDraft);
    }

    fn set_status(&mut self, status: impl Into<String>) {
        self.session.dispatch(AppAction::SetStatus(status.into()));
    }
}
