use super::effects::{
    COMPLETION_ROW_ID_PREFIX, COMPLETIONS_SCROLLABLE_ID, ENTRIES_SCROLLABLE_ID,
    ENTRY_ROW_ID_PREFIX, UiEffect,
};
use super::{Confirmation, GuiApp, Message};
use dagcal_app::{CompletionDirection, SelectionDirection};
use iced::advanced::widget::{
    Id, Operation, operate,
    operation::{Outcome, Scrollable},
};
use iced::widget::operation::AbsoluteOffset;
use iced::{Rectangle, Task, Vector};

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

    pub(super) fn scroll_entries_to_selection_edge(
        &self,
        direction: SelectionDirection,
    ) -> Task<Message> {
        let Some(selected) = self.session.selected else {
            return Task::none();
        };

        if !self
            .session
            .filtered_entries()
            .iter()
            .any(|entry| entry.id == selected)
        {
            return Task::none();
        }

        operate(MeasureSelectionBounds::new(
            ENTRIES_SCROLLABLE_ID,
            entry_row_id(selected),
        ))
        .map(move |bounds| Message::SelectionBoundsMeasured(bounds, direction))
    }

    pub(super) fn scroll_entries_by_selection_bounds(
        &self,
        bounds: Option<SelectionScrollBounds>,
        direction: SelectionDirection,
    ) -> Task<Message> {
        let Some(bounds) = bounds else {
            return Task::none();
        };

        let Some(delta_y) = selection_scroll_delta(bounds, ScrollDirection::from(direction)) else {
            return Task::none();
        };

        iced::widget::operation::scroll_by(
            ENTRIES_SCROLLABLE_ID,
            AbsoluteOffset { x: 0.0, y: delta_y },
        )
    }

    pub(super) fn scroll_completion_to_selection_edge(
        &self,
        direction: CompletionDirection,
    ) -> Task<Message> {
        let Some(selected) = self.session.selected_completion_index() else {
            return Task::none();
        };

        if selected >= self.session.completion_candidates().len() {
            return Task::none();
        }

        operate(MeasureSelectionBounds::new(
            COMPLETIONS_SCROLLABLE_ID,
            completion_row_id(selected),
        ))
        .map(move |bounds| Message::CompletionBoundsMeasured(bounds, direction))
    }

    pub(super) fn scroll_completion_by_selection_bounds(
        &self,
        bounds: Option<SelectionScrollBounds>,
        direction: CompletionDirection,
    ) -> Task<Message> {
        let Some(bounds) = bounds else {
            return Task::none();
        };

        let Some(delta_y) = selection_scroll_delta(bounds, ScrollDirection::from(direction)) else {
            return Task::none();
        };

        iced::widget::operation::scroll_by(
            COMPLETIONS_SCROLLABLE_ID,
            AbsoluteOffset { x: 0.0, y: delta_y },
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionScrollBounds {
    viewport: Rectangle,
    selected_row: Rectangle,
    translation: Vector,
}

struct MeasureSelectionBounds {
    scrollable_id: Id,
    selected_row_id: Id,
    viewport: Option<Rectangle>,
    selected_row: Option<Rectangle>,
    translation: Vector,
}

impl MeasureSelectionBounds {
    fn new(scrollable_id: &'static str, selected_row_id: Id) -> Self {
        Self {
            scrollable_id: Id::new(scrollable_id),
            selected_row_id,
            viewport: None,
            selected_row: None,
            translation: Vector::default(),
        }
    }
}

impl Operation<Option<SelectionScrollBounds>> for MeasureSelectionBounds {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn Operation<Option<SelectionScrollBounds>>),
    ) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&self.scrollable_id) {
            self.viewport = Some(bounds);
            self.translation = translation;
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.selected_row_id) {
            self.selected_row = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<Option<SelectionScrollBounds>> {
        Outcome::Some(
            self.viewport
                .zip(self.selected_row)
                .map(|(viewport, selected_row)| SelectionScrollBounds {
                    viewport,
                    selected_row,
                    translation: self.translation,
                }),
        )
    }
}

pub(super) fn entry_row_id(id: dagcal_app::ExpressionId) -> Id {
    Id::from(format!("{ENTRY_ROW_ID_PREFIX}{id}"))
}

pub(super) fn completion_row_id(index: usize) -> Id {
    Id::from(format!("{COMPLETION_ROW_ID_PREFIX}{index}"))
}

#[derive(Debug, Clone, Copy)]
enum ScrollDirection {
    Previous,
    Next,
}

impl From<SelectionDirection> for ScrollDirection {
    fn from(direction: SelectionDirection) -> Self {
        match direction {
            SelectionDirection::Previous => Self::Previous,
            SelectionDirection::Next => Self::Next,
        }
    }
}

impl From<CompletionDirection> for ScrollDirection {
    fn from(direction: CompletionDirection) -> Self {
        match direction {
            CompletionDirection::Previous => Self::Previous,
            CompletionDirection::Next => Self::Next,
        }
    }
}

fn selection_scroll_delta(
    bounds: SelectionScrollBounds,
    direction: ScrollDirection,
) -> Option<f32> {
    let row_top = bounds.selected_row.y - bounds.translation.y;
    let row_bottom = row_top + bounds.selected_row.height;
    let viewport_top = bounds.viewport.y;
    let viewport_bottom = bounds.viewport.y + bounds.viewport.height;
    let outside_viewport = row_top < viewport_top || row_bottom > viewport_bottom;

    if !outside_viewport {
        return None;
    }

    Some(match direction {
        ScrollDirection::Previous => row_top - viewport_top,
        ScrollDirection::Next => row_bottom - viewport_bottom,
    })
}
