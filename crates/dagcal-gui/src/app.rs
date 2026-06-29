mod actions;
mod draft;
mod effects;

#[cfg(test)]
mod tests;

pub(crate) use draft::Draft;
pub(crate) use effects::{ENTRIES_SCROLLABLE_ID, EXPRESSION_INPUT_ID};

use dagcal_core::{Engine, EntryView, ExpressionId};
use iced::{Subscription, Task, keyboard};

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    Submit,
    NewEntry,
    Edit(ExpressionId),
    CancelEdit,
    Delete(ExpressionId),
    InsertReference(ExpressionId),
    Select(ExpressionId),
    EntryHovered(ExpressionId),
    EntryUnhovered(ExpressionId),
    RightClick,
    Keyboard(keyboard::Event),
    Clear,
    Undo,
    Redo,
    InsertConstant(String),
    InsertFunction(String),
}

pub struct GuiApp {
    pub(crate) engine: Engine,
    pub(crate) entries: Vec<EntryView>,
    pub(crate) input: Draft,
    pub(crate) editing: Option<ExpressionId>,
    pub(crate) draft_entry: Option<ExpressionId>,
    pub(crate) selected: Option<ExpressionId>,
    pub(crate) hovered_entry: Option<ExpressionId>,
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
                draft_entry: None,
                selected: None,
                hovered_entry: None,
                status: "Ready".to_string(),
            },
            Task::none(),
        )
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => self.input_changed(value),
            Message::Submit => self.submit_input(),
            Message::NewEntry => self.start_new_entry(),
            Message::Edit(id) => self.start_edit(id),
            Message::CancelEdit => self.cancel_edit(),
            Message::Delete(id) => self.delete_entry(id),
            Message::InsertReference(id) => self.insert_reference(id),
            Message::Select(id) => self.select_entry(id),
            Message::EntryHovered(id) => self.set_hovered_entry(id),
            Message::EntryUnhovered(id) => self.clear_hovered_entry(id),
            Message::RightClick => self.select_hovered_entry(),
            Message::Keyboard(event) => self.handle_keyboard_event(event),
            Message::Clear => self.clear(),
            Message::Undo => self.undo(),
            Message::Redo => self.redo(),
            Message::InsertConstant(name) => self.insert_constant(name),
            Message::InsertFunction(name) => self.insert_function(name),
        }
        .into_task(self)
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        effects::subscription(self)
    }
}
