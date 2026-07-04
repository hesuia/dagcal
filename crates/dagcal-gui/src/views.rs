mod details;
mod dialogs;
mod entries;
mod input;
mod menu;

use crate::app::{GuiApp, Message};
use crate::style::status_bar_style;
use iced::widget::{column, container, row, rule, text};
use iced::{Element, Fill, Length, window};

impl GuiApp {
    pub(crate) fn title(&self, window: window::Id) -> String {
        if self.confirmation_window == Some(window) {
            return "dagcal Confirmation".to_string();
        }

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
        if self.confirmation_window == Some(window) {
            return self.confirmation_window_view(window);
        }

        if self.details_window == Some(window) {
            return self.details_window_view(window);
        }

        if self.help_window == Some(window) {
            return self.help_window_view(window);
        }

        self.main_window_view()
    }

    fn main_window_view(&self) -> Element<'_, Message> {
        let content = column![
            self.menu_bar_view(),
            self.entries_view(),
            rule::horizontal(1),
            self.input_view(),
            self.selected_detail_view(),
        ]
        .spacing(12);

        let content = content.push(self.status_bar_view());

        container(content.padding(16))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn status_bar_view(&self) -> Element<'_, Message> {
        container(
            row![
                text(self.session.status.clone())
                    .size(13)
                    .width(Length::FillPortion(3)),
                text(self.entry_count_status_text())
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
}
