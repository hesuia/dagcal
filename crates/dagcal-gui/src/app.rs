mod actions;
mod document;
mod effects;
mod file_io;
mod keyboard;
mod windows;

#[cfg(test)]
mod tests;

pub(crate) use dagcal_app::EntryStateFilter;
pub(crate) use effects::{ENTRIES_SCROLLABLE_ID, ENTRY_SEARCH_INPUT_ID, EXPRESSION_INPUT_ID};
pub use file_io::{LoadResult, SaveResult};
pub(crate) use windows::{Confirmation, HelpTopic};

use dagcal_app::{AppSession, EngineSnapshot, ExpressionId};
use iced::{Size, Subscription, Task, keyboard as iced_keyboard, window};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    OpenEntrySearch,
    EntrySearchChanged(String),
    EntryStateFilterChanged(EntryStateFilter),
    ClearEntrySearch,
    Submit,
    NewEntry,
    Edit(ExpressionId),
    CancelEdit,
    Delete(ExpressionId),
    Recalculate(ExpressionId),
    RecalculateAll,
    InsertReference(ExpressionId),
    Select(ExpressionId),
    EntryHovered(ExpressionId),
    EntryUnhovered(ExpressionId),
    RightClick(window::Id),
    Keyboard(window::Id, iced_keyboard::Event),
    Clear,
    Save,
    SaveAs,
    Load,
    SaveFinished(SaveResult),
    LoadFinished(LoadResult),
    Undo,
    Redo,
    InsertConstant(String),
    InsertFunction(String),
    AcceptCompletion(usize),
    DismissCompletion,
    ShowDetails(ExpressionId),
    Quit,
    ShowAbout,
    ShowKeyboardShortcuts,
    WindowClosed(window::Id),
    ConfirmPending,
    CancelConfirmation,
}

pub struct GuiApp {
    pub(crate) main_window: Option<window::Id>,
    pub(crate) help_window: Option<window::Id>,
    pub(crate) details_window: Option<window::Id>,
    pub(crate) confirmation_window: Option<window::Id>,
    pub(crate) details_target: Option<ExpressionId>,
    pub(crate) help_topic: HelpTopic,
    pub(crate) session: AppSession,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) saved_snapshot: EngineSnapshot,
    pub(crate) pending_confirmation: Option<Confirmation>,
}

impl GuiApp {
    pub(crate) fn new() -> (Self, Task<Message>) {
        let (main_window, open_main_window) = window::open(window::Settings {
            size: Size::new(1024.0, 768.0),
            exit_on_close_request: false,
            ..window::Settings::default()
        });

        let session = AppSession::new();
        let saved_snapshot = session.snapshot();

        (
            Self {
                main_window: Some(main_window),
                help_window: None,
                details_window: None,
                confirmation_window: None,
                details_target: None,
                help_topic: HelpTopic::KeyboardShortcuts,
                session,
                current_path: None,
                saved_snapshot,
                pending_confirmation: None,
            },
            open_main_window.discard(),
        )
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => self.input_changed(value),
            Message::OpenEntrySearch => self.open_entry_search(),
            Message::EntrySearchChanged(value) => self.entry_search_changed(value),
            Message::EntryStateFilterChanged(filter) => self.entry_state_filter_changed(filter),
            Message::ClearEntrySearch => self.clear_entry_search(),
            Message::Submit => self.submit_input(),
            Message::NewEntry => self.start_new_entry(),
            Message::Edit(id) => self.start_edit(id),
            Message::CancelEdit => self.cancel_edit(),
            Message::Delete(id) => self.delete_entry(id),
            Message::Recalculate(id) => self.recalculate_entry(id),
            Message::RecalculateAll => self.recalculate_all(),
            Message::InsertReference(id) => self.insert_reference(id),
            Message::Select(id) => self.select_entry(id),
            Message::EntryHovered(id) => self.set_hovered_entry(id),
            Message::EntryUnhovered(id) => self.clear_hovered_entry(id),
            Message::RightClick(window) if self.main_window == Some(window) => {
                self.select_hovered_entry()
            }
            Message::RightClick(_) => effects::UiEffect::None,
            Message::Keyboard(window, event) if self.main_window == Some(window) => {
                self.handle_keyboard_event(event)
            }
            Message::Keyboard(_, _) => effects::UiEffect::None,
            Message::Clear => return self.clear(),
            Message::Save => return self.save(),
            Message::SaveAs => return self.save_as(),
            Message::Load => return self.load(),
            Message::SaveFinished(result) => self.finish_save(result),
            Message::LoadFinished(result) => self.finish_load(result),
            Message::Undo => self.undo(),
            Message::Redo => self.redo(),
            Message::InsertConstant(name) => self.insert_constant(name),
            Message::InsertFunction(name) => self.insert_function(name),
            Message::AcceptCompletion(index) => {
                self.session.accept_completion(index);
                effects::UiEffect::FocusInput
            }
            Message::DismissCompletion => {
                self.session.close_completions();
                effects::UiEffect::None
            }
            Message::ShowDetails(id) => return self.open_details_window(id),
            Message::Quit => return self.quit(),
            Message::ShowAbout => return self.open_help_window(HelpTopic::About),
            Message::ShowKeyboardShortcuts => {
                return self.open_help_window(HelpTopic::KeyboardShortcuts);
            }
            Message::WindowClosed(id) => return self.window_closed(id),
            Message::ConfirmPending => return self.confirm_pending(),
            Message::CancelConfirmation => return self.cancel_confirmation(),
        }
        .into_task(self)
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        effects::subscription(self)
    }
}

impl Deref for GuiApp {
    type Target = AppSession;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl DerefMut for GuiApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}
