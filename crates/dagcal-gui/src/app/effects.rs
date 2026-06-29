use super::{GuiApp, Message};
use iced::{Subscription, Task, event, keyboard, mouse};

pub(crate) const EXPRESSION_INPUT_ID: &str = "expression-input";
pub(crate) const ENTRIES_SCROLLABLE_ID: &str = "entries-scrollable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiEffect {
    None,
    FocusInput,
    ScrollToSelection,
}

impl UiEffect {
    pub(super) fn into_task(self, app: &GuiApp) -> Task<Message> {
        match self {
            Self::None => Task::none(),
            Self::FocusInput => focus_expression_input(),
            Self::ScrollToSelection => app.scroll_entries_to_selection(),
        }
    }
}

pub(super) fn subscription(app: &GuiApp) -> Subscription<Message> {
    let right_clicks = event::listen_with(|event, _status, _window| match event {
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
            Some(Message::RightClick)
        }
        _ => None,
    });

    if app.selection_navigation_enabled() {
        Subscription::batch([keyboard::listen().map(Message::Keyboard), right_clicks])
    } else {
        right_clicks
    }
}

fn focus_expression_input() -> Task<Message> {
    Task::batch([
        iced::widget::operation::focus(EXPRESSION_INPUT_ID),
        iced::widget::operation::move_cursor_to_end(EXPRESSION_INPUT_ID),
    ])
}
