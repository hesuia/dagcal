mod details;
mod entries;
mod input;
mod menu;

use crate::app::{Confirmation, GuiApp, Message};
use crate::style::{menu_button_style, status_bar_style};
use iced::widget::{button, column, container, row, rule, text};
use iced::{Element, Fill, Length, window};
use std::path::Path;

impl GuiApp {
    pub(crate) fn title(&self, window: window::Id) -> String {
        if self.details_window == Some(window) {
            return match self.details_target {
                Some(id) => format!("dagcal Details - {id}"),
                None => "dagcal Details".to_string(),
            };
        }

        if self.help_window == Some(window) {
            return match self.help_topic {
                crate::app::HelpTopic::KeyboardShortcuts => "dagcal Help - Keyboard shortcuts",
                crate::app::HelpTopic::About => "dagcal Help - About",
            }
            .to_string();
        }

        self.main_title()
    }

    pub(crate) fn view(&self, window: window::Id) -> Element<'_, Message> {
        if self.details_window == Some(window) {
            return self.details_window_view(window);
        }

        if self.help_window == Some(window) {
            return self.help_window_view(window);
        }

        self.main_window_view()
    }

    fn main_window_view(&self) -> Element<'_, Message> {
        let mut content = column![
            self.menu_bar_view(),
            self.entries_view(),
            rule::horizontal(1),
            self.input_view(),
            self.selected_detail_view(),
        ]
        .spacing(12);

        if self.pending_confirmation.is_some() {
            content = content.push(self.confirmation_view());
        }

        content = content.push(self.status_bar_view());

        container(content.padding(16))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn status_bar_view(&self) -> Element<'_, Message> {
        container(
            row![
                text(self.status.clone())
                    .size(13)
                    .width(Length::FillPortion(3)),
                text(format!("Entries: {}", self.entries.len()))
                    .size(13)
                    .width(Length::FillPortion(1)),
                text(self.file_status_text())
                    .size(13)
                    .width(Length::FillPortion(2)),
                text(self.history_status_text())
                    .size(13)
                    .width(Length::FillPortion(2)),
            ]
            .spacing(12)
            .align_y(iced::Center),
        )
        .padding([7, 10])
        .width(Fill)
        .style(|_| status_bar_style())
        .into()
    }

    fn confirmation_view(&self) -> Element<'_, Message> {
        let Some(confirmation) = self.pending_confirmation else {
            return container("").into();
        };
        let (title, body, action) = confirmation_text(confirmation);

        container(
            row![
                column![text(title).size(16), text(body).size(13),]
                    .spacing(4)
                    .width(Length::Fill),
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
        )
        .padding(12)
        .width(Fill)
        .style(|_| status_bar_style())
        .into()
    }

    pub(super) fn main_title(&self) -> String {
        let dirty = if self.is_dirty() { "* " } else { "" };
        format!("{dirty}dagcal - {}", self.document_name())
    }

    pub(super) fn file_status_text(&self) -> String {
        let state = if self.is_dirty() {
            "Unsaved changes"
        } else {
            "Saved"
        };

        format!("File: {}    {state}", self.document_name())
    }

    fn document_name(&self) -> String {
        self.current_path
            .as_deref()
            .map(path_label)
            .unwrap_or_else(|| "Untitled".to_string())
    }

    pub(super) fn history_status_text(&self) -> String {
        format!(
            "Undo: {}    Redo: {}",
            availability_label(self.engine.can_undo()),
            availability_label(self.engine.can_redo())
        )
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

fn availability_label(available: bool) -> &'static str {
    if available { "yes" } else { "no" }
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn confirmation_text(confirmation: Confirmation) -> (&'static str, String, &'static str) {
    match confirmation {
        Confirmation::Delete(id) => ("Delete entry?", format!("Delete {id}?"), "Delete"),
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
