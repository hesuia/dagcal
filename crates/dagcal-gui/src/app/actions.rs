use super::effects::{ENTRIES_SCROLLABLE_ID, UiEffect};
use super::{GuiApp, LoadResult, Message, SaveResult};
use crate::formatting::{entry_expression_source, entry_reference_token, state_summary};
use dagcal_core::{Engine, EngineSnapshot, EntryView, ExpressionId};
use iced::Task;
use iced::keyboard::{self, Key, key};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

impl GuiApp {
    pub(super) fn input_changed(&mut self, value: String) -> UiEffect {
        self.input.set(value);
        if self.ensure_empty_draft_entry() {
            UiEffect::ScrollToSelection
        } else {
            UiEffect::None
        }
    }

    pub(super) fn submit_input(&mut self) -> UiEffect {
        let source = self.input.source().trim().to_string();

        if let Some(id) = self.editing {
            let result = self.engine.set_statement_by_id(id, source);
            self.refresh_affected(&result.execution.affected_ids);
            let id = result.execution.id;
            self.status = self.entry_status(id);
            self.load_edit_entry(id, SelectionStatus::Keep);
            UiEffect::None
        } else if let Some(id) = self.draft_entry.take() {
            self.save_new_entry_draft(id, source);
            UiEffect::ScrollToSelection
        } else {
            let execution = self.engine.execute(&source);
            self.refresh_affected(&execution.affected_ids);
            self.selected = Some(execution.id);
            self.status = format!("{} = {}", execution.id, state_summary(&execution.state));
            self.editing = None;
            self.input.clear();
            UiEffect::ScrollToSelection
        }
    }

    pub(super) fn start_edit(&mut self, id: ExpressionId) -> UiEffect {
        self.load_edit_entry(id, SelectionStatus::Set(format!("Editing {id}")));
        UiEffect::None
    }

    pub(super) fn start_new_entry(&mut self) -> UiEffect {
        self.editing = None;
        self.draft_entry = None;
        self.input.clear();
        if self.ensure_empty_draft_entry() {
            UiEffect::ScrollToSelection
        } else {
            UiEffect::None
        }
    }

    pub(super) fn cancel_edit(&mut self) -> UiEffect {
        self.editing = None;
        self.input.clear();
        self.status = "Edit cancelled".to_string();
        UiEffect::None
    }

    pub(super) fn delete_entry(&mut self, id: ExpressionId) -> UiEffect {
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
                }
            }
            if was_editing {
                self.editing = None;
                self.input.clear();
            }
            self.status = format!("Removed {id}");
        } else {
            self.status = unavailable_status(id);
        }

        UiEffect::None
    }

    pub(super) fn recalculate_entry(&mut self, id: ExpressionId) -> UiEffect {
        if let Some(affected) = self.engine.recompute_entry_by_id(id) {
            self.refresh_affected(&affected);
            self.status = format!("Recalculated {id}");
        } else {
            self.status = unavailable_status(id);
        }

        UiEffect::None
    }

    pub(super) fn recalculate_all(&mut self) -> UiEffect {
        self.engine.recompute_all();
        self.entries = self.engine.entries();
        self.status = "Recalculated all entries".to_string();
        UiEffect::None
    }

    pub(super) fn insert_reference(&mut self, id: ExpressionId) -> UiEffect {
        let token = self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .map(entry_reference_token)
            .unwrap_or_else(|| id.to_string());

        self.input.insert_token(&token);
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {token}");
        UiEffect::FocusInput
    }

    pub(super) fn insert_constant(&mut self, name: String) -> UiEffect {
        self.input.insert_token(&name);
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {name}");
        UiEffect::FocusInput
    }

    pub(super) fn insert_function(&mut self, name: String) -> UiEffect {
        let token = format!("{name}()");
        self.input.insert_token(&token);
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {token}");
        UiEffect::FocusInput
    }

    pub(super) fn select_entry(&mut self, id: ExpressionId) -> UiEffect {
        if self.engine.entry_by_id(id).is_some() {
            self.set_selected_entry(id);
        } else {
            self.status = unavailable_status(id);
        }

        UiEffect::None
    }

    pub(super) fn set_hovered_entry(&mut self, id: ExpressionId) -> UiEffect {
        self.hovered_entry = Some(id);
        UiEffect::None
    }

    pub(super) fn clear_hovered_entry(&mut self, id: ExpressionId) -> UiEffect {
        if self.hovered_entry == Some(id) {
            self.hovered_entry = None;
        }

        UiEffect::None
    }

    pub(super) fn select_hovered_entry(&mut self) -> UiEffect {
        if let Some(id) = self.hovered_entry {
            self.select_entry(id)
        } else {
            UiEffect::None
        }
    }

    pub(super) fn handle_keyboard_event(&mut self, event: keyboard::Event) -> UiEffect {
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
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if modifiers.control() && is_character_key(&key, "z") =>
            {
                self.undo();
            }
            keyboard::Event::KeyPressed { key, modifiers, .. }
                if modifiers.control() && is_character_key(&key, "y") =>
            {
                self.redo();
            }
            _ => {}
        }

        UiEffect::None
    }

    pub(super) fn clear(&mut self) -> UiEffect {
        self.engine.clear();
        self.entries = self.engine.entries();
        self.input.clear();
        self.editing = None;
        self.draft_entry = None;
        self.selected = None;
        self.hovered_entry = None;
        self.status = "Cleared".to_string();
        UiEffect::None
    }

    pub(super) fn save(&mut self) -> Task<Message> {
        let snapshot = self.engine.snapshot();
        self.status = "Saving...".to_string();

        Task::perform(save_snapshot(snapshot), Message::SaveFinished)
    }

    pub(super) fn load(&mut self) -> Task<Message> {
        self.status = "Loading...".to_string();

        Task::perform(load_snapshot(), Message::LoadFinished)
    }

    pub(super) fn finish_save(&mut self, result: SaveResult) -> UiEffect {
        self.status = match result {
            SaveResult::Cancelled => "Save cancelled".to_string(),
            SaveResult::Saved(path) => format!("Saved {}", display_path(&path)),
            SaveResult::Failed(error) => format!("Save failed: {error}"),
        };

        UiEffect::None
    }

    pub(super) fn finish_load(&mut self, result: LoadResult) -> UiEffect {
        match result {
            LoadResult::Cancelled => {
                self.status = "Load cancelled".to_string();
            }
            LoadResult::Loaded(path, snapshot) => match Engine::from_snapshot(snapshot) {
                Ok(engine) => {
                    self.engine = engine;
                    self.reset_after_load(&format!("Loaded {}", display_path(&path)));
                }
                Err(error) => {
                    self.status = format!("Load failed: could not restore snapshot ({error})");
                }
            },
            LoadResult::Failed(error) => {
                self.status = format!("Load failed: {error}");
            }
        }

        UiEffect::None
    }

    pub(super) fn undo(&mut self) -> UiEffect {
        if self.engine.undo() {
            self.reset_after_history_restore("Undone");
        } else {
            self.status = "Nothing to undo".to_string();
        }

        UiEffect::None
    }

    pub(super) fn redo(&mut self) -> UiEffect {
        if self.engine.redo() {
            self.reset_after_history_restore("Redone");
        } else {
            self.status = "Nothing to redo".to_string();
        }

        UiEffect::None
    }

    pub(super) fn selection_navigation_enabled(&self) -> bool {
        (self.editing.is_none() && self.input.source().is_empty())
            || (self.editing.is_some() && self.input.is_loaded_selection())
    }

    pub(super) fn scroll_entries_to_selection(&self) -> Task<Message> {
        let Some(selected) = self.selected else {
            return Task::none();
        };

        let Some(index) = self.entries.iter().position(|entry| entry.id == selected) else {
            return Task::none();
        };

        let y = if self.entries.len() <= 1 {
            0.0
        } else {
            index as f32 / (self.entries.len() - 1) as f32
        };

        iced::widget::operation::snap_to(
            ENTRIES_SCROLLABLE_ID,
            iced::widget::operation::RelativeOffset { x: 0.0, y },
        )
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

    fn delete_selected_entry(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        self.delete_entry(id);
    }

    pub(super) fn move_selection(&mut self, direction: SelectionDirection) {
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

        self.set_selected_entry(self.entries[next_index].id);
    }

    fn set_selected_entry(&mut self, id: ExpressionId) {
        let changed = self.selected != Some(id);
        self.selected = Some(id);

        if changed && self.editing.is_some() {
            self.editing = None;
            self.input.clear();
            self.status = "Edit cancelled".to_string();
        }
    }

    fn ensure_empty_draft_entry(&mut self) -> bool {
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

    fn entry_status(&self, id: ExpressionId) -> String {
        match self.engine.entry_by_id(id) {
            Some(entry) => format!("{id} = {}", state_summary(&entry.state)),
            None => format!("{id} updated"),
        }
    }

    fn reset_after_history_restore(&mut self, status: &str) {
        self.entries = self.engine.entries();
        self.input.clear();
        self.editing = None;
        self.draft_entry = None;
        self.hovered_entry = None;
        self.selected = self
            .selected
            .filter(|id| self.engine.entry_by_id(*id).is_some())
            .or_else(|| self.entries.last().map(|entry| entry.id));
        self.status = status.to_string();
    }

    fn reset_after_load(&mut self, status: &str) {
        self.entries = self.engine.entries();
        self.input.clear();
        self.editing = None;
        self.draft_entry = None;
        self.hovered_entry = None;
        self.selected = self.entries.last().map(|entry| entry.id);
        self.status = status.to_string();
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum SelectionDirection {
    Previous,
    Next,
}

enum SelectionStatus {
    Keep,
    Set(String),
}

fn unavailable_status(id: ExpressionId) -> String {
    format!("{id} is not available")
}

fn is_character_key(key: &Key, expected: &str) -> bool {
    matches!(key, Key::Character(value) if value.eq_ignore_ascii_case(expected))
}

async fn save_snapshot(snapshot: EngineSnapshot) -> SaveResult {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Dagcal session", &["json"])
        .set_file_name("dagcal-session.json")
        .save_file()
    else {
        return SaveResult::Cancelled;
    };

    let json = match serde_json::to_string_pretty(&snapshot) {
        Ok(json) => json,
        Err(error) => return SaveResult::Failed(format!("could not encode JSON ({error})")),
    };

    match std::fs::write(&path, json) {
        Ok(()) => SaveResult::Saved(path),
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
