use crate::app::{Confirmation, GuiApp, HelpTopic, Message};
use crate::style::menu_button_style;
use iced::widget::{button, column, container, row, rule, text};
use iced::{Element, Fill, Length, window};

impl GuiApp {
    pub(super) fn confirmation_window_view(&self, window: window::Id) -> Element<'_, Message> {
        let Some(confirmation) = self.pending_confirmation else {
            return container(
                column![
                    text("No pending action").size(24),
                    button("Close").on_press(Message::WindowClosed(window)),
                ]
                .spacing(16)
                .padding(24)
                .width(Length::Fill),
            )
            .width(Fill)
            .height(Fill)
            .into();
        };
        let (title, body, action) = confirmation_text(confirmation);

        container(
            column![
                text(title).size(24),
                text(body).size(15),
                row![
                    button("Cancel")
                        .padding([7, 12])
                        .style(|_, status| menu_button_style(status))
                        .on_press(Message::CancelConfirmation),
                    button(action)
                        .padding([7, 12])
                        .style(|_, status| menu_button_style(status))
                        .on_press(Message::ConfirmPending),
                ]
                .spacing(10)
                .align_y(iced::Center),
            ]
            .spacing(16)
            .padding(24)
            .width(Length::Fill),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }

    pub(super) fn help_window_view(&self, window: window::Id) -> Element<'_, Message> {
        let (title, body) = help_text(self.help_topic);

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

fn help_text(topic: HelpTopic) -> (&'static str, &'static str) {
    match topic {
        HelpTopic::KeyboardShortcuts => (
            "Keyboard shortcuts",
            "Ctrl+N: New entry\nCtrl+S: Save\nCtrl+Shift+S: Save As\nCtrl+O: Load\nCtrl+Q: Quit\nCtrl+F: Search entries\nCtrl+Z: Undo\nCtrl+Y or Ctrl+Shift+Z: Redo\nCtrl+E or F2: Edit selected entry\nF5: Recalculate selected entry\nCtrl+R: Recalculate all entries\nDelete: Remove the selected entry\nArrow Up/Down: Move the selection when the input is empty\nEsc: Close completion or search, then cancel edit",
        ),
        HelpTopic::About => (
            "About dagcal",
            "dagcal is a dependency-aware calculator.\n\nExpressions are tracked as stable result IDs such as $1, and dependent entries are recomputed automatically.",
        ),
    }
}

fn confirmation_text(confirmation: Confirmation) -> (&'static str, String, &'static str) {
    match confirmation {
        Confirmation::Clear => (
            "Clear all entries?",
            "All current entries will be removed.".to_string(),
            "Clear",
        ),
        Confirmation::Load => (
            "Load another session?",
            "Unsaved changes will be discarded.".to_string(),
            "Load",
        ),
        Confirmation::Quit | Confirmation::CloseMain(_) => (
            "Quit dagcal?",
            "Unsaved changes will be discarded.".to_string(),
            "Quit",
        ),
    }
}
