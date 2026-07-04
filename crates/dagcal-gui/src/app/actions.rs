use super::effects::{ENTRIES_SCROLLABLE_ID, UiEffect};
use super::{Confirmation, GuiApp, Message};
use iced::Task;

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

    pub(super) fn clear(&mut self) -> Task<Message> {
        if self.is_dirty() {
            return self.request_confirmation(Confirmation::Clear);
        }

        self.perform_clear().into_task(self)
    }

    pub(super) fn perform_clear(&mut self) -> UiEffect {
        self.session.clear().into()
    }

    pub(super) fn undo(&mut self) -> UiEffect {
        self.session.undo().into()
    }

    pub(super) fn redo(&mut self) -> UiEffect {
        self.session.redo().into()
    }

    pub(super) fn scroll_entries_to_selection(&self) -> Task<Message> {
        let Some(selected) = self.session.selected else {
            return Task::none();
        };

        let visible_entries = self.session.filtered_entries();
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
