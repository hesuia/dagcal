use dagcal_app::{AppSession, EntryState, EntryView, ExpressionId, SelectionDirection, formatting};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Edit,
}

pub struct App {
    session: AppSession,
    mode: Mode,
    should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut session = AppSession::new();
        session.status =
            "i: insert  e: edit  d: delete  u: undo  r: redo  c: clear  q: quit".to_string();
        Self {
            session,
            mode: Mode::Normal,
            should_quit: false,
        }
    }

    pub fn entries(&self) -> Vec<EntryView> {
        self.session.entries.clone()
    }

    pub fn selected(&self) -> usize {
        self.session
            .selected
            .and_then(|id| self.session.entries.iter().position(|entry| entry.id == id))
            .unwrap_or(0)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn input(&self) -> &str {
        self.session.input.source()
    }

    pub fn status(&self) -> &str {
        &self.session.status
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn move_next(&mut self) {
        self.session.move_selection(SelectionDirection::Next);
    }

    pub fn move_previous(&mut self) {
        self.session.move_selection(SelectionDirection::Previous);
    }

    pub fn start_insert(&mut self) {
        self.mode = Mode::Insert;
        self.session.input.clear();
        self.session.editing = None;
        self.session.status = "enter expression".to_string();
    }

    pub fn start_edit(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.mode = Mode::Edit;
            self.session.editing = Some(entry.id);
            self.session.input.load_selection(entry.source);
            self.session.status = format!("editing {}", entry.id);
        } else {
            self.session.status = "no entry selected".to_string();
        }
    }

    pub fn cancel_input(&mut self) {
        self.mode = Mode::Normal;
        self.session.input.clear();
        self.session.editing = None;
        self.session.status = "cancelled".to_string();
    }

    pub fn push_input(&mut self, ch: char) {
        if self.mode != Mode::Normal {
            let mut source = self.session.input.source().to_string();
            source.push(ch);
            self.session.input.set(source);
        }
    }

    pub fn backspace_input(&mut self) {
        if self.mode != Mode::Normal {
            let mut source = self.session.input.source().to_string();
            source.pop();
            self.session.input.set(source);
        }
    }

    pub fn submit_input(&mut self) {
        match self.mode {
            Mode::Insert => self.submit_insert(),
            Mode::Edit => self.submit_edit(),
            Mode::Normal => {}
        }
    }

    pub fn delete_selected(&mut self) {
        let Some(id) = self.selected_id() else {
            self.session.status = "no entry selected".to_string();
            return;
        };

        self.session.delete_entry(id);
    }

    pub fn clear(&mut self) {
        self.session.clear();
        self.mode = Mode::Normal;
        self.session.status = "cleared".to_string();
    }

    pub fn undo(&mut self) {
        self.session.undo();
        self.mode = Mode::Normal;
        if self.session.status == "Undone" {
            self.session.status = "undone".to_string();
        } else if self.session.status == "Nothing to undo" {
            self.session.status = "nothing to undo".to_string();
        }
    }

    pub fn redo(&mut self) {
        self.session.redo();
        self.mode = Mode::Normal;
        if self.session.status == "Redone" {
            self.session.status = "redone".to_string();
        } else if self.session.status == "Nothing to redo" {
            self.session.status = "nothing to redo".to_string();
        }
    }

    fn submit_insert(&mut self) {
        if self.session.input.source().trim().is_empty() {
            self.session.status = "empty expression".to_string();
            return;
        }

        self.session.submit_input();
        self.finish_input();
    }

    fn submit_edit(&mut self) {
        if self.session.editing.is_none() {
            self.session.status = "no entry selected".to_string();
            self.finish_input();
            return;
        }

        if self.session.input.source().trim().is_empty() {
            self.session.status = "empty expression".to_string();
            return;
        }

        self.session.submit_input();
        self.finish_input();
    }

    fn finish_input(&mut self) {
        self.mode = Mode::Normal;
        self.session.input.clear();
        self.session.editing = None;
    }

    fn selected_entry(&self) -> Option<EntryView> {
        self.session.entries.get(self.selected()).cloned()
    }

    fn selected_id(&self) -> Option<ExpressionId> {
        self.selected_entry().map(|entry| entry.id)
    }
}

pub fn state_summary(state: &EntryState) -> String {
    formatting::state_summary(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_app::Number;

    #[test]
    fn insert_adds_entries_and_selects_the_latest_result() {
        let mut app = App::new();

        app.start_insert();
        for ch in "1 + 2".chars() {
            app.push_input(ch);
        }
        app.submit_input();

        app.start_insert();
        for ch in "$1 * 4".chars() {
            app.push_input(ch);
        }
        app.submit_input();

        let entries = app.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id.to_string(), "$1");
        assert_eq!(entries[0].state, EntryState::Value(Number::from(3)));
        assert_eq!(entries[1].id.to_string(), "$2");
        assert_eq!(entries[1].state, EntryState::Value(Number::from(12)));
        assert_eq!(app.selected(), 1);
    }

    #[test]
    fn edit_updates_selected_source_and_recomputes_dependents() {
        let mut app = App::new();
        app.session.engine.execute("subtotal = 100");
        app.session.engine.execute("subtotal * 2");
        app.session.entries = app.session.engine.entries();
        app.session.selected = Some(ExpressionId::new(1));

        app.start_edit();
        assert_eq!(app.input(), "100");
        app.session.input.clear();
        for ch in "120".chars() {
            app.push_input(ch);
        }
        app.submit_input();

        let entries = app.entries();
        assert_eq!(entries[0].state, EntryState::Value(Number::from(120)));
        assert_eq!(entries[1].state, EntryState::Value(Number::from(240)));
    }

    #[test]
    fn edit_keeps_invalid_source_as_error_entry() {
        let mut app = App::new();
        app.session.engine.execute("10");
        app.session.entries = app.session.engine.entries();
        app.session.selected = Some(ExpressionId::new(1));

        app.start_edit();
        app.session.input.clear();
        for ch in "1 +".chars() {
            app.push_input(ch);
        }
        app.submit_input();

        let entries = app.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "1 +");
        assert!(matches!(entries[0].state, EntryState::Error(_)));
    }

    #[test]
    fn insert_keeps_invalid_source_as_error_entry() {
        let mut app = App::new();

        app.start_insert();
        for ch in "1 +".chars() {
            app.push_input(ch);
        }
        app.submit_input();

        let entries = app.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id.to_string(), "$1");
        assert_eq!(entries[0].source, "1 +");
        assert!(matches!(entries[0].state, EntryState::Error(_)));
        assert_eq!(app.selected(), 0);
    }

    #[test]
    fn delete_preserves_later_ids() {
        let mut app = App::new();
        app.session.engine.execute("10");
        app.session.engine.execute("20");
        app.session.entries = app.session.engine.entries();
        app.session.selected = Some(ExpressionId::new(1));
        app.delete_selected();

        let entries = app.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id.to_string(), "$2");
    }

    #[test]
    fn undo_and_redo_refresh_cached_entries() {
        let mut app = App::new();

        app.start_insert();
        for ch in "10".chars() {
            app.push_input(ch);
        }
        app.submit_input();

        app.undo();
        assert!(app.entries().is_empty());
        assert_eq!(app.status(), "undone");

        app.redo();
        let entries = app.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, ExpressionId::new(1));
        assert_eq!(entries[0].state, EntryState::Value(Number::from(10)));
        assert_eq!(app.status(), "redone");
    }

    #[test]
    fn empty_list_actions_do_not_panic() {
        let mut app = App::new();

        app.move_next();
        app.move_previous();
        app.start_edit();
        app.delete_selected();

        assert!(app.entries().is_empty());
    }
}
