pub(crate) mod actions;
mod document;
mod effects;
mod file_io;
mod keyboard;
mod windows;

#[cfg(test)]
mod tests;

pub(crate) use dagcal_app::EntryStateFilter;
pub(crate) use effects::{
    COMPLETION_ROW_ID_PREFIX, COMPLETIONS_SCROLLABLE_ID, ENTRIES_SCROLLABLE_ID,
    ENTRY_ROW_ID_PREFIX, ENTRY_SEARCH_INPUT_ID, EXPRESSION_INPUT_ID,
};
pub use file_io::{LoadResult, SaveResult};
pub(crate) use windows::{Confirmation, HelpTopic};

use dagcal_app::{
    AppAction, AppSession, CompletionDirection, EngineSnapshot, ExpressionId, SelectionDirection,
};
use iced::{Size, Subscription, Task, keyboard as iced_keyboard, window};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) enum Message {
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
    SelectionBoundsMeasured(Option<actions::SelectionScrollBounds>, SelectionDirection),
    CompletionBoundsMeasured(Option<actions::SelectionScrollBounds>, CompletionDirection),
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
            Message::InputChanged(value) => return self.dispatch(AppAction::InputChanged(value)),
            Message::OpenEntrySearch => return self.dispatch(AppAction::OpenEntrySearch),
            Message::EntrySearchChanged(value) => {
                return self.dispatch(AppAction::EntrySearchChanged(value));
            }
            Message::EntryStateFilterChanged(filter) => {
                return self.dispatch(AppAction::EntryStateFilterChanged(filter));
            }
            Message::ClearEntrySearch => return self.dispatch(AppAction::ClearEntrySearch),
            Message::Submit => return self.dispatch(AppAction::SubmitInput),
            Message::NewEntry => return self.dispatch(AppAction::StartNewEntry),
            Message::Edit(id) => return self.dispatch(AppAction::StartEdit(id)),
            Message::CancelEdit => return self.dispatch(AppAction::CancelEdit),
            Message::Delete(id) => return self.dispatch(AppAction::DeleteEntry(id)),
            Message::Recalculate(id) => return self.dispatch(AppAction::RecalculateEntry(id)),
            Message::RecalculateAll => return self.dispatch(AppAction::RecalculateAll),
            Message::InsertReference(id) => return self.dispatch(AppAction::InsertReference(id)),
            Message::Select(id) => return self.dispatch(AppAction::SelectEntry(id)),
            Message::EntryHovered(id) => return self.dispatch(AppAction::SetHoveredEntry(id)),
            Message::EntryUnhovered(id) => {
                return self.dispatch(AppAction::ClearHoveredEntry(id));
            }
            Message::RightClick(window) if self.main_window == Some(window) => {
                return self.dispatch(AppAction::SelectHoveredEntry);
            }
            Message::RightClick(_) => effects::UiEffect::None,
            Message::Keyboard(window, event) if self.main_window == Some(window) => {
                return self.handle_keyboard_task(event);
            }
            Message::Keyboard(_, _) => effects::UiEffect::None,
            Message::SelectionBoundsMeasured(bounds, direction) => {
                return self.scroll_entries_by_selection_bounds(bounds, direction);
            }
            Message::CompletionBoundsMeasured(bounds, direction) => {
                return self.scroll_completion_by_selection_bounds(bounds, direction);
            }
            Message::Clear => return self.clear(),
            Message::Save => return self.save(),
            Message::SaveAs => return self.save_as(),
            Message::Load => return self.load(),
            Message::SaveFinished(result) => self.finish_save(result),
            Message::LoadFinished(result) => self.finish_load(result),
            Message::Undo => self.undo(),
            Message::Redo => self.redo(),
            Message::InsertConstant(name) => return self.dispatch(AppAction::InsertConstant(name)),
            Message::InsertFunction(name) => return self.dispatch(AppAction::InsertFunction(name)),
            Message::AcceptCompletion(index) => {
                return self.dispatch(AppAction::AcceptCompletion(index));
            }
            Message::DismissCompletion => return self.dispatch(AppAction::CloseCompletions),
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

    fn dispatch(&mut self, action: AppAction) -> Task<Message> {
        let effects = self.session.dispatch(action);
        effects::app_effects_into_task(self, effects)
    }

    pub(super) fn set_status(&mut self, status: impl Into<String>) {
        self.session.dispatch(AppAction::SetStatus(status.into()));
    }
}
