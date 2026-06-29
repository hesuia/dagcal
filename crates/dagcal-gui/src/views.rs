mod details;
mod entries;
mod input;
mod menu;

use crate::app::{GuiApp, Message};
use iced::widget::{column, container, row, rule, text};
use iced::{Element, Fill};

impl GuiApp {
    pub(crate) fn view(&self) -> Element<'_, Message> {
        container(
            column![
                header_view(),
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
}

fn header_view() -> Element<'static, Message> {
    row![
        text("dagcal").size(28),
        text("Stable expression graph").size(14),
    ]
    .spacing(12)
    .align_y(iced::Center)
    .into()
}
