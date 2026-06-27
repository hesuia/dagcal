use dagcal_core::{Engine, EntryState, EntryView, ExpressionId};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Edit,
}

pub struct App {
    engine: Engine,
    entries: Vec<EntryView>,
    selected: usize,
    mode: Mode,
    input: String,
    status: String,
    should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
            entries: Vec::new(),
            selected: 0,
            mode: Mode::Normal,
            input: String::new(),
            status: "i: insert  e: edit  d: delete  c: clear  q: quit".to_string(),
            should_quit: false,
        }
    }

    pub fn entries(&self) -> Vec<EntryView> {
        self.entries.clone()
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn move_next(&mut self) {
        let len = self.entries.len();
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    pub fn move_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn start_insert(&mut self) {
        self.mode = Mode::Insert;
        self.input.clear();
        self.status = "enter expression".to_string();
    }

    pub fn start_edit(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.mode = Mode::Edit;
            self.input = entry.source;
            self.status = format!("editing {}", entry.id);
        } else {
            self.status = "no entry selected".to_string();
        }
    }

    pub fn cancel_input(&mut self) {
        self.mode = Mode::Normal;
        self.input.clear();
        self.status = "cancelled".to_string();
    }

    pub fn push_input(&mut self, ch: char) {
        if self.mode != Mode::Normal {
            self.input.push(ch);
        }
    }

    pub fn backspace_input(&mut self) {
        if self.mode != Mode::Normal {
            self.input.pop();
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
            self.status = "no entry selected".to_string();
            return;
        };

        if let Some(removal) = self.engine.remove_entry_by_id(id) {
            self.entries
                .retain(|entry| entry.id != removal.removed_entry.id);
            self.refresh_affected(&removal.affected_ids);
        }
        self.clamp_selection();
        self.status = format!("removed {id}");
    }

    pub fn clear(&mut self) {
        self.engine = Engine::new();
        self.entries.clear();
        self.selected = 0;
        self.mode = Mode::Normal;
        self.input.clear();
        self.status = "cleared".to_string();
    }

    fn submit_insert(&mut self) {
        let source = self.input.trim().to_string();
        if source.is_empty() {
            self.status = "empty expression".to_string();
            return;
        }

        let execution = self.engine.execute(&source);
        self.refresh_affected(&execution.affected_ids);
        self.select_id(execution.id);
        self.status = format!("{} = {}", execution.id, state_summary(&execution.state));
        self.finish_input();
    }

    fn submit_edit(&mut self) {
        let Some(id) = self.selected_id() else {
            self.status = "no entry selected".to_string();
            self.finish_input();
            return;
        };

        let source = self.input.trim().to_string();
        if source.is_empty() {
            self.status = "empty expression".to_string();
            return;
        }

        let affected_ids = self.engine.affected_by(id);
        match self.engine.set_entry_by_id(id, source) {
            Ok(execution) => self.refresh_affected(&execution.affected_ids),
            Err(_) => self.refresh_affected(&affected_ids),
        }
        self.select_id(id);
        if let Some(entry) = self.engine.entry_by_id(id) {
            self.status = format!("{id} = {}", state_summary(&entry.state));
        }
        self.finish_input();
    }

    fn finish_input(&mut self) {
        self.mode = Mode::Normal;
        self.input.clear();
    }

    fn selected_entry(&self) -> Option<EntryView> {
        self.entries.get(self.selected).cloned()
    }

    fn selected_id(&self) -> Option<ExpressionId> {
        self.selected_entry().map(|entry| entry.id)
    }

    fn select_id(&mut self, id: ExpressionId) {
        if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
            self.selected = index;
        }
    }

    fn clamp_selection(&mut self) {
        let len = self.entries.len();
        if len == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(len - 1);
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

    #[cfg(test)]
    fn refresh_cache(&mut self) {
        self.entries = self.engine.entries();
        self.clamp_selection();
    }
}

pub fn state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(err) => format!("error: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_core::Number;

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
        app.engine.execute("subtotal = 100");
        app.engine.execute("subtotal * 2");
        app.refresh_cache();

        app.move_previous();
        app.start_edit();
        assert_eq!(app.input(), "100");
        app.input.clear();
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
        app.engine.execute("10");
        app.refresh_cache();

        app.start_edit();
        app.input.clear();
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
        app.engine.execute("10");
        app.engine.execute("20");
        app.refresh_cache();
        app.delete_selected();

        let entries = app.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id.to_string(), "$2");
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
