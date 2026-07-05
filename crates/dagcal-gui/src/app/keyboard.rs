use super::GuiApp;
use super::effects::UiEffect;
use dagcal_app::{CompletionDirection, SelectionDirection};
use iced::keyboard::{self, Key, key};

impl GuiApp {
    pub(super) fn handle_keyboard_event(&mut self, event: keyboard::Event) -> UiEffect {
        match event {
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowUp),
                ..
            } if self.session.completion_is_open() => {
                return self.move_completion_selection(CompletionDirection::Previous);
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowDown),
                ..
            } if self.session.completion_is_open() => {
                return self.move_completion_selection(CompletionDirection::Next);
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Tab),
                ..
            } if self.session.completion_is_open() => {
                self.session.accept_selected_completion();
                return UiEffect::FocusInput;
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Escape),
                ..
            } if self.session.completion_is_open() => self.session.close_completions(),
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Escape),
                ..
            } if self.session.entry_search_open => {
                self.close_entry_search();
            }
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if modifiers.control() && is_character_key(&key, "f") =>
            {
                return self.open_entry_search();
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowUp),
                ..
            } if self.session.selection_navigation_enabled() => {
                return self.move_entry_selection(SelectionDirection::Previous);
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowDown),
                ..
            } if self.session.selection_navigation_enabled() => {
                return self.move_entry_selection(SelectionDirection::Next);
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Delete),
                ..
            } if self.session.selection_navigation_enabled() => {
                self.session.delete_selected_entry();
            }
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if self.session.selection_navigation_enabled()
                    && modifiers.control()
                    && is_character_key(&key, "z") =>
            {
                self.undo();
            }
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if self.session.selection_navigation_enabled()
                    && modifiers.control()
                    && is_character_key(&key, "y") =>
            {
                self.redo();
            }
            _ => {}
        }

        UiEffect::None
    }
}

fn is_character_key(key: &Key, expected: &str) -> bool {
    matches!(key, Key::Character(value) if value.eq_ignore_ascii_case(expected))
}

impl GuiApp {
    fn move_entry_selection(&mut self, direction: SelectionDirection) -> UiEffect {
        let previous = self.session.selected;
        self.session.move_selection(direction);

        if self.session.selected != previous {
            UiEffect::ScrollToSelectionEdge(direction)
        } else {
            UiEffect::None
        }
    }

    fn move_completion_selection(&mut self, direction: CompletionDirection) -> UiEffect {
        let previous = self.session.selected_completion_index();
        self.session.move_completion_selection(direction);

        if self.session.selected_completion_index() != previous {
            UiEffect::ScrollToCompletionSelectionEdge(direction)
        } else {
            UiEffect::None
        }
    }
}
