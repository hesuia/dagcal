mod actions;
mod completion;
mod draft;
mod effects;

#[cfg(test)]
mod tests;

pub(crate) use draft::Draft;
pub(crate) use effects::{ENTRIES_SCROLLABLE_ID, ENTRY_SEARCH_INPUT_ID, EXPRESSION_INPUT_ID};

use self::completion::CompletionState;
use dagcal_core::{Engine, EngineSnapshot, EntryView, ExpressionId};
use iced::{Size, Subscription, Task, keyboard, window};
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
    Keyboard(window::Id, keyboard::Event),
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

#[derive(Debug, Clone)]
pub enum SaveResult {
    Cancelled,
    Saved(PathBuf, EngineSnapshot),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum LoadResult {
    Cancelled,
    Loaded(PathBuf, EngineSnapshot),
    Failed(String),
}

pub struct GuiApp {
    pub(crate) main_window: Option<window::Id>,
    pub(crate) help_window: Option<window::Id>,
    pub(crate) details_window: Option<window::Id>,
    pub(crate) details_target: Option<ExpressionId>,
    pub(crate) help_topic: HelpTopic,
    pub(crate) engine: Engine,
    pub(crate) entries: Vec<EntryView>,
    pub(crate) entry_search_open: bool,
    pub(crate) entry_search_query: String,
    pub(crate) entry_state_filter: EntryStateFilter,
    pub(crate) input: Draft,
    pub(crate) editing: Option<ExpressionId>,
    pub(crate) draft_entry: Option<ExpressionId>,
    pub(crate) selected: Option<ExpressionId>,
    pub(crate) hovered_entry: Option<ExpressionId>,
    pub(crate) status: String,
    pub(crate) completion: CompletionState,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) saved_snapshot: EngineSnapshot,
    pub(crate) pending_confirmation: Option<Confirmation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HelpTopic {
    KeyboardShortcuts,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryStateFilter {
    All,
    Values,
    Errors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Confirmation {
    Delete(ExpressionId),
    Clear,
    Load,
    Quit,
    CloseMain(window::Id),
}

impl GuiApp {
    pub(crate) fn new() -> (Self, Task<Message>) {
        let (main_window, open_main_window) = window::open(window::Settings {
            size: Size::new(1024.0, 768.0),
            exit_on_close_request: false,
            ..window::Settings::default()
        });

        let engine = Engine::new();
        let saved_snapshot = engine.snapshot();

        (
            Self {
                main_window: Some(main_window),
                help_window: None,
                details_window: None,
                details_target: None,
                help_topic: HelpTopic::KeyboardShortcuts,
                engine,
                entries: Vec::new(),
                entry_search_open: false,
                entry_search_query: String::new(),
                entry_state_filter: EntryStateFilter::All,
                input: Draft::default(),
                editing: None,
                draft_entry: None,
                selected: None,
                hovered_entry: None,
                status: "Ready".to_string(),
                completion: CompletionState::default(),
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
            Message::Clear => self.clear(),
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
                self.accept_completion(index);
                effects::UiEffect::FocusInput
            }
            Message::DismissCompletion => {
                self.close_completions();
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
            Message::CancelConfirmation => self.cancel_confirmation(),
        }
        .into_task(self)
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        effects::subscription(self)
    }
}

impl GuiApp {
    fn open_help_window(&mut self, topic: HelpTopic) -> Task<Message> {
        self.help_topic = topic;

        if self.help_window.is_some() {
            self.status = "Help is already open".to_string();
            return Task::none();
        }

        let (id, open_window) = window::open(window::Settings {
            size: Size::new(520.0, 420.0),
            min_size: Some(Size::new(420.0, 320.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        });

        self.help_window = Some(id);
        self.status = "Opened help".to_string();

        open_window.discard()
    }

    fn open_details_window(&mut self, id: ExpressionId) -> Task<Message> {
        self.details_target = Some(id);

        if self.details_window.is_some() {
            self.status = format!("Showing details for {id}");
            return Task::none();
        }

        let (window, open_window) = window::open(window::Settings {
            size: Size::new(640.0, 460.0),
            min_size: Some(Size::new(480.0, 320.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        });

        self.details_window = Some(window);
        self.status = format!("Opened details for {id}");

        open_window.discard()
    }

    fn window_closed(&mut self, id: window::Id) -> Task<Message> {
        if self.help_window == Some(id) {
            self.help_window = None;
            return window::close(id);
        }

        if self.details_window == Some(id) {
            self.details_window = None;
            self.details_target = None;
            return window::close(id);
        }

        if self.main_window == Some(id) {
            if self.is_dirty() {
                self.request_confirmation(Confirmation::CloseMain(id));
                return Task::none();
            }
            self.main_window = None;
            return iced::exit();
        }

        Task::none()
    }
}
