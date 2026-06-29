mod details;
mod entries;
mod input;
mod menu;

use crate::app::{GuiApp, Message};
use iced::widget::{column, container, rule};
use iced::{Element, Fill};

impl GuiApp {
    pub(crate) fn view(&self) -> Element<'_, Message> {
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
}
