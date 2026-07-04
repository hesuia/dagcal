use super::effects::{ENTRIES_SCROLLABLE_ID, UiEffect};
use super::{Confirmation, GuiApp, LoadResult, Message, SaveResult};
use dagcal_app::{AppSession, CompletionDirection, EngineSnapshot, SelectionDirection};
use iced::keyboard::{self, Key, key};
use iced::{Size, Task, window};
use std::path::{Path, PathBuf};

impl GuiApp {
    pub(super) fn open_entry_search(&mut self) -> UiEffect {
        self.session.open_entry_search().into()
    }

    pub(super) fn close_entry_search(&mut self) -> UiEffect {
        self.session.close_entry_search().into()
    }

    pub(super) fn entry_search_changed(&mut self, value: String) -> UiEffect {
        self.session.entry_search_changed(value).into()
    }

    pub(super) fn entry_state_filter_changed(
        &mut self,
        filter: dagcal_app::EntryStateFilter,
    ) -> UiEffect {
        self.session.entry_state_filter_changed(filter).into()
    }

    pub(super) fn clear_entry_search(&mut self) -> UiEffect {
        self.session.clear_entry_search().into()
    }

    pub(super) fn input_changed(&mut self, value: String) -> UiEffect {
        self.session.input_changed(value).into()
    }

    pub(super) fn submit_input(&mut self) -> UiEffect {
        self.session.submit_input().into()
    }

    pub(super) fn start_edit(&mut self, id: dagcal_app::ExpressionId) -> UiEffect {
        self.session.start_edit(id).into()
    }

    pub(super) fn start_new_entry(&mut self) -> UiEffect {
        self.session.start_new_entry().into()
    }

    pub(super) fn cancel_edit(&mut self) -> UiEffect {
        self.session.cancel_edit().into()
    }

    pub(super) fn delete_entry(&mut self, id: dagcal_app::ExpressionId) -> UiEffect {
        self.session.delete_entry(id).into()
    }

    pub(super) fn recalculate_entry(&mut self, id: dagcal_app::ExpressionId) -> UiEffect {
        self.session.recalculate_entry(id).into()
    }

    pub(super) fn recalculate_all(&mut self) -> UiEffect {
        self.session.recalculate_all().into()
    }

    pub(super) fn insert_reference(&mut self, id: dagcal_app::ExpressionId) -> UiEffect {
        self.session.insert_reference(id).into()
    }

    pub(super) fn insert_constant(&mut self, name: String) -> UiEffect {
        self.session.insert_constant(name).into()
    }

    pub(super) fn insert_function(&mut self, name: String) -> UiEffect {
        self.session.insert_function(name).into()
    }

    pub(super) fn select_entry(&mut self, id: dagcal_app::ExpressionId) -> UiEffect {
        self.session.select_entry(id).into()
    }

    pub(super) fn set_hovered_entry(&mut self, id: dagcal_app::ExpressionId) -> UiEffect {
        self.session.set_hovered_entry(id).into()
    }

    pub(super) fn clear_hovered_entry(&mut self, id: dagcal_app::ExpressionId) -> UiEffect {
        self.session.clear_hovered_entry(id).into()
    }

    pub(super) fn select_hovered_entry(&mut self) -> UiEffect {
        self.session.select_hovered_entry().into()
    }

    pub(super) fn handle_keyboard_event(&mut self, event: keyboard::Event) -> UiEffect {
        match event {
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowUp),
                ..
            } if self.completion_is_open() => {
                self.move_completion_selection(CompletionDirection::Previous)
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowDown),
                ..
            } if self.completion_is_open() => {
                self.move_completion_selection(CompletionDirection::Next)
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Tab),
                ..
            } if self.completion_is_open() => {
                self.accept_selected_completion();
                return UiEffect::FocusInput;
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Escape),
                ..
            } if self.completion_is_open() => self.close_completions(),
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Escape),
                ..
            } if self.entry_search_open => {
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
            } if self.selection_navigation_enabled() => {
                self.move_selection(SelectionDirection::Previous);
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::ArrowDown),
                ..
            } if self.selection_navigation_enabled() => {
                self.move_selection(SelectionDirection::Next);
            }
            keyboard::Event::KeyPressed {
                key: Key::Named(key::Named::Delete),
                ..
            } if self.selection_navigation_enabled() => {
                self.delete_selected_entry();
            }
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if self.selection_navigation_enabled()
                    && modifiers.control()
                    && is_character_key(&key, "z") =>
            {
                self.undo();
            }
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if self.selection_navigation_enabled()
                    && modifiers.control()
                    && is_character_key(&key, "y") =>
            {
                self.redo();
            }
            _ => {}
        }

        UiEffect::None
    }

    pub(super) fn clear(&mut self) -> Task<Message> {
        if self.is_dirty() {
            return self.request_confirmation(Confirmation::Clear);
        }

        self.perform_clear().into_task(self)
    }

    fn perform_clear(&mut self) -> UiEffect {
        self.session.clear().into()
    }

    pub(super) fn save(&mut self) -> Task<Message> {
        if let Some(path) = self.current_path.clone() {
            self.save_to_path(path)
        } else {
            self.save_as()
        }
    }

    pub(super) fn save_as(&mut self) -> Task<Message> {
        let snapshot = self.engine.snapshot();
        self.status = "Saving...".to_string();

        Task::perform(
            save_snapshot_with_dialog(snapshot, self.current_path.clone()),
            Message::SaveFinished,
        )
    }

    fn save_to_path(&mut self, path: PathBuf) -> Task<Message> {
        let snapshot = self.engine.snapshot();
        self.status = format!("Saving {}...", display_path(&path));

        Task::perform(save_snapshot_to_path(path, snapshot), Message::SaveFinished)
    }

    pub(super) fn load(&mut self) -> Task<Message> {
        if self.is_dirty() {
            return self.request_confirmation(Confirmation::Load);
        }

        self.start_load()
    }

    fn start_load(&mut self) -> Task<Message> {
        self.status = "Loading...".to_string();

        Task::perform(load_snapshot(), Message::LoadFinished)
    }

    pub(super) fn finish_save(&mut self, result: SaveResult) -> UiEffect {
        self.status = match result {
            SaveResult::Cancelled => "Save cancelled".to_string(),
            SaveResult::Saved(path, snapshot) => {
                let status = format!("Saved {}", display_path(&path));
                self.current_path = Some(path);
                self.saved_snapshot = snapshot;
                status
            }
            SaveResult::Failed(error) => format!("Save failed: {error}"),
        };

        UiEffect::None
    }

    pub(super) fn finish_load(&mut self, result: LoadResult) -> UiEffect {
        match result {
            LoadResult::Cancelled => {
                self.status = "Load cancelled".to_string();
            }
            LoadResult::Loaded(path, snapshot) => {
                let status = format!("Loaded {}", display_path(&path));
                match AppSession::from_snapshot(snapshot) {
                    Ok(mut session) => {
                        session.status = status;
                        self.current_path = Some(path);
                        self.saved_snapshot = session.snapshot();
                        self.session = session;
                    }
                    Err(error) => {
                        self.status = format!("Load failed: could not restore snapshot ({error})");
                    }
                }
            }
            LoadResult::Failed(error) => {
                self.status = format!("Load failed: {error}");
            }
        }

        UiEffect::None
    }

    pub(super) fn confirm_pending(&mut self) -> Task<Message> {
        let Some(confirmation) = self.pending_confirmation.take() else {
            return Task::none();
        };
        let close_confirmation = self.close_confirmation_window();

        let action = match confirmation {
            Confirmation::Clear => {
                self.perform_clear();
                Task::none()
            }
            Confirmation::Load => self.start_load(),
            Confirmation::Quit => iced::exit(),
            Confirmation::CloseMain(_) => {
                self.main_window = None;
                iced::exit()
            }
        };

        Task::batch([close_confirmation, action])
    }

    pub(super) fn cancel_confirmation(&mut self) -> Task<Message> {
        self.pending_confirmation = None;
        self.status = "Action cancelled".to_string();
        self.close_confirmation_window()
    }

    pub(super) fn quit(&mut self) -> Task<Message> {
        if self.is_dirty() {
            self.request_confirmation(Confirmation::Quit)
        } else {
            iced::exit()
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.session.is_dirty_against(&self.saved_snapshot)
    }

    pub(super) fn request_confirmation(&mut self, confirmation: Confirmation) -> Task<Message> {
        self.pending_confirmation = Some(confirmation);
        self.status = match confirmation {
            Confirmation::Clear => "Confirm clear".to_string(),
            Confirmation::Load => "Confirm load".to_string(),
            Confirmation::Quit | Confirmation::CloseMain(_) => "Confirm quit".to_string(),
        };

        if self.confirmation_window.is_some() {
            return Task::none();
        }

        let (id, open_window) = window::open(window::Settings {
            size: Size::new(380.0, 180.0),
            min_size: Some(Size::new(340.0, 160.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        });
        self.confirmation_window = Some(id);

        open_window.discard()
    }

    fn close_confirmation_window(&mut self) -> Task<Message> {
        let Some(id) = self.confirmation_window.take() else {
            return Task::none();
        };

        window::close(id)
    }

    pub(super) fn undo(&mut self) -> UiEffect {
        self.session.undo().into()
    }

    pub(super) fn redo(&mut self) -> UiEffect {
        self.session.redo().into()
    }

    pub(super) fn scroll_entries_to_selection(&self) -> Task<Message> {
        let Some(selected) = self.selected else {
            return Task::none();
        };

        let visible_entries = self.filtered_entries();
        let Some(index) = visible_entries
            .iter()
            .position(|entry| entry.id == selected)
        else {
            return Task::none();
        };

        let y = if visible_entries.len() <= 1 {
            0.0
        } else {
            index as f32 / (visible_entries.len() - 1) as f32
        };

        iced::widget::operation::snap_to(
            ENTRIES_SCROLLABLE_ID,
            iced::widget::operation::RelativeOffset { x: 0.0, y },
        )
    }
}

fn is_character_key(key: &Key, expected: &str) -> bool {
    matches!(key, Key::Character(value) if value.eq_ignore_ascii_case(expected))
}

async fn save_snapshot_with_dialog(
    snapshot: EngineSnapshot,
    current_path: Option<PathBuf>,
) -> SaveResult {
    let file_name = current_path
        .as_deref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("dagcal-session.json");

    let Some(path) = rfd::FileDialog::new()
        .add_filter("Dagcal session", &["json"])
        .set_file_name(file_name)
        .save_file()
    else {
        return SaveResult::Cancelled;
    };

    save_snapshot_to_path(path, snapshot).await
}

async fn save_snapshot_to_path(path: PathBuf, snapshot: EngineSnapshot) -> SaveResult {
    let json = match serde_json::to_string_pretty(&snapshot) {
        Ok(json) => json,
        Err(error) => return SaveResult::Failed(format!("could not encode JSON ({error})")),
    };

    match std::fs::write(&path, json) {
        Ok(()) => SaveResult::Saved(path, snapshot),
        Err(error) => SaveResult::Failed(format!("could not write file ({error})")),
    }
}

async fn load_snapshot() -> LoadResult {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Dagcal session", &["json"])
        .pick_file()
    else {
        return LoadResult::Cancelled;
    };

    load_snapshot_from_path(path)
}

fn load_snapshot_from_path(path: PathBuf) -> LoadResult {
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) => return LoadResult::Failed(format!("could not read file ({error})")),
    };

    let snapshot = match serde_json::from_str(&json) {
        Ok(snapshot) => snapshot,
        Err(error) => return LoadResult::Failed(format!("could not parse JSON ({error})")),
    };

    LoadResult::Loaded(path, snapshot)
}

fn display_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}
