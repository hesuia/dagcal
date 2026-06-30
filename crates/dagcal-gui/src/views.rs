mod details;
mod entries;
mod input;
mod menu;

use crate::app::{GuiApp, Message};
use iced::widget::{button, column, container, rule, text};
use iced::{Element, Fill, Length, window};

impl GuiApp {
    pub(crate) fn title(&self, window: window::Id) -> String {
        if self.help_window == Some(window) {
            return match self.help_topic {
                crate::app::HelpTopic::KeyboardShortcuts => "dagcal Help - Keyboard shortcuts",
                crate::app::HelpTopic::About => "dagcal Help - About",
            }
            .to_string();
        }

        "dagcal".to_string()
    }

    pub(crate) fn view(&self, window: window::Id) -> Element<'_, Message> {
        if self.help_window == Some(window) {
            return self.help_window_view(window);
        }

        self.main_window_view()
    }

    fn main_window_view(&self) -> Element<'_, Message> {
        container(
            column![
                self.menu_bar_view(),
                self.entries_view(),
                rule::horizontal(1),
                self.input_view(),
                self.selected_detail_view(),
            ]
            .spacing(12)
            .padding(16),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn help_window_view(&self, window: window::Id) -> Element<'_, Message> {
        let (title, body) = match self.help_topic {
            crate::app::HelpTopic::KeyboardShortcuts => (
                "Keyboard shortcuts",
                "Ctrl+Z: Undo\nCtrl+Y: Redo\nDelete: Remove the selected entry\nArrow Up/Down: Move the selection when the input is empty",
            ),
            crate::app::HelpTopic::About => (
                "About dagcal",
                "dagcal is a dependency-aware calculator.\n\nExpressions are tracked as stable result IDs such as $1, and dependent entries are recomputed automatically.",
            ),
        };

        container(
            column![
                text(title).size(28),
                rule::horizontal(1),
                text(body).size(16),
                button("Close").on_press(Message::WindowClosed(window)),
            ]
            .spacing(16)
            .padding(24)
            .width(Length::Fill),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }
}
