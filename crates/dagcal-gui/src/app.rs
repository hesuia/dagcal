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
            }
            Message::Submit => self.submit_input(),
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
        if source.is_empty() {
            self.status = "Input is empty".to_string();
            return;
        }

        if let Some(id) = self.editing {
            let source = self.edit_source_for_save(id, source);
            let result = self.engine.set_entry_by_id(id, source);
            self.refresh_affected(&result.execution.affected_ids);
            self.selected = Some(id);
            self.status = match self.engine.entry_by_id(id) {
                Some(entry) => format!("{id} = {}", state_summary(&entry.state)),
                None => format!("{id} updated"),
            };
        } else {
            let execution = self.engine.execute(&source);
            self.refresh_affected(&execution.affected_ids);
            self.selected = Some(execution.id);
            self.status = format!("{} = {}", execution.id, state_summary(&execution.state));
        }

        self.editing = None;
        self.input.clear();
    }

    fn start_edit(&mut self, id: ExpressionId) {
        match self.engine.entry_by_id(id) {
            Some(entry) => {
                self.input.set(entry_expression_source(&entry));
                self.editing = Some(id);
                self.selected = Some(id);
                self.status = format!("Editing {id}");
            }
            None => {
                self.status = format!("{id} is not available");
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
            self.refresh_affected(&removal.affected_ids);
            if self.selected == Some(id) {
                self.selected = self.entries.last().map(|entry| entry.id);
            }
            if self.editing == Some(id) {
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
        self.selected = Some(id);
        self.status = format!("Inserted {token}");
    }

    fn select_entry(&mut self, id: ExpressionId) {
        if self.entries.iter().any(|entry| entry.id == id) {
            self.selected = Some(id);
        } else {
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
            _ => {}
        }
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

        self.selected = Some(self.entries[next_index].id);
    }

    fn selection_navigation_enabled(&self) -> bool {
        self.editing.is_none() && self.input.source().is_empty()
    }

    fn edit_source_for_save(&self, id: ExpressionId, source: String) -> String {
        let Some(entry) = self.entries.iter().find(|entry| entry.id == id) else {
            return source;
        };

        let Some(name) = entry.name.as_deref() else {
            return source;
        };

        match source.split_once('=') {
            Some((left, right)) if left.trim() == name => right.trim().to_string(),
            _ => source,
        }
    }

    fn clear(&mut self) {
        self.engine = Engine::new();
        self.entries.clear();
        self.input.clear();
        self.editing = None;
        self.selected = None;
        self.status = "Cleared".to_string();
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

#[derive(Debug, Default)]
pub(crate) struct Draft {
    source: String,
}

impl Draft {
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    fn set(&mut self, source: String) {
        self.source = source;
    }

    fn clear(&mut self) {
        self.source.clear();
    }

    fn insert_token(&mut self, token: &str) {
        if needs_space_before_reference(&self.source) {
            self.source.push(' ');
        }
        self.source.push_str(token);
        self.source.push(' ');
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
    fn selecting_entry_does_not_replace_status_with_selected_message() {
        let (mut app, _) = GuiApp::new();
        app.input.set("21".to_string());
        app.submit_input();
        app.status = "Ready".to_string();

        app.select_entry(ExpressionId::new(1));

        assert_eq!(app.selected, Some(ExpressionId::new(1)));
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

        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));

        app.selected = None;
        app.move_selection(SelectionDirection::Previous);
        assert_eq!(app.selected, Some(ExpressionId::new(2)));
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

        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(2)));

        app.move_selection(SelectionDirection::Previous);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));

        app.move_selection(SelectionDirection::Previous);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
    }

    #[test]
    fn arrow_navigation_is_disabled_while_input_has_text_or_entry_is_editing() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("20".to_string());
        app.submit_input();

        app.selected = Some(ExpressionId::new(1));
        app.input.set("draft".to_string());
        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));

        app.input.clear();
        app.editing = Some(ExpressionId::new(1));
        app.move_selection(SelectionDirection::Next);
        assert_eq!(app.selected, Some(ExpressionId::new(1)));
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
}
