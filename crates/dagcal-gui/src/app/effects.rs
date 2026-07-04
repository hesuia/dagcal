use super::{GuiApp, Message};
use iced::{Subscription, Task, event, mouse};

pub(crate) const EXPRESSION_INPUT_ID: &str = "expression-input";
pub(crate) const ENTRY_SEARCH_INPUT_ID: &str = "entry-search-input";
pub(crate) const ENTRIES_SCROLLABLE_ID: &str = "entries-scrollable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiEffect {
    None,
    FocusInput,
    FocusEntrySearch,
    ScrollToSelection,
}

impl UiEffect {
    pub(super) fn into_task(self, app: &GuiApp) -> Task<Message> {
        match self {
            Self::None => Task::none(),
            Self::FocusInput => focus_expression_input(),
            Self::FocusEntrySearch => focus_entry_search_input(),
            Self::ScrollToSelection => app.scroll_entries_to_selection(),
        }
    }
}

impl From<dagcal_app::SessionChange> for UiEffect {
    fn from(change: dagcal_app::SessionChange) -> Self {
        match change {
            dagcal_app::SessionChange::None => Self::None,
            dagcal_app::SessionChange::FocusInput => Self::FocusInput,
            dagcal_app::SessionChange::FocusEntrySearch => Self::FocusEntrySearch,
            dagcal_app::SessionChange::ScrollToSelection => Self::ScrollToSelection,
        }
    }
}

pub(super) fn subscription(_app: &GuiApp) -> Subscription<Message> {
    let right_clicks = event::listen_with(|event, _status, window| match event {
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
            Some(Message::RightClick(window))
        }
        _ => None,
    });
    let window_closes = iced::window::close_requests().map(Message::WindowClosed);

    let keyboard_events = event::listen_with(|event, _status, window| match event {
        iced::Event::Keyboard(event) => Some(Message::Keyboard(window, event)),
        _ => None,
    });
    let input_events = Subscription::batch([keyboard_events, right_clicks]);

    Subscription::batch([input_events, window_closes])
}

fn focus_expression_input() -> Task<Message> {
    Task::batch([
        iced::widget::operation::focus(EXPRESSION_INPUT_ID),
        iced::widget::operation::move_cursor_to_end(EXPRESSION_INPUT_ID),
    ])
}

fn focus_entry_search_input() -> Task<Message> {
    Task::batch([
        iced::widget::operation::focus(ENTRY_SEARCH_INPUT_ID),
        iced::widget::operation::move_cursor_to_end(ENTRY_SEARCH_INPUT_ID),
    ])
}
