use crate::app::Message;
use iced::widget::{button, container, scrollable, text};
use iced::{Background, Border, Color, Element, Fill, Length};
use iced_aw::style::Status;

pub(crate) const DETAIL_HEIGHT: f32 = 44.0;
pub(crate) const TABLE_TEXT_SIZE: u32 = 16;

pub(crate) fn fixed_line<'a>(
    content: impl Into<Element<'a, Message>>,
    height: f32,
) -> Element<'a, Message> {
    container(content)
        .height(Length::Fixed(height))
        .width(Fill)
        .into()
}

pub(crate) fn fixed_scroll_text(content: &str, height: f32) -> Element<'static, Message> {
    scroll_text_box_with_size(content, height, Fill, 14)
}

fn scroll_text_box_with_size(
    content: &str,
    height: f32,
    width: Length,
    size: u32,
) -> Element<'static, Message> {
    container(scrollable(text(content.to_string()).size(size)).height(Length::Fixed(height)))
        .height(Length::Fixed(height))
        .width(width)
        .into()
}

pub(crate) fn row_container_style(selected: bool) -> iced::widget::container::Style {
    if selected {
        iced::widget::container::Style::default()
            .background(Background::Color(selected_row_color()))
            .border(
                Border::default()
                    .rounded(4)
                    .width(1)
                    .color(selected_border_color()),
            )
    } else {
        iced::widget::container::Style::default()
    }
}

pub(crate) fn context_menu_panel_style() -> iced::widget::container::Style {
    iced::widget::container::Style::default()
        .background(Background::Color(Color::from_rgb(0.13, 0.15, 0.17)))
        .border(
            Border::default()
                .rounded(4)
                .width(1)
                .color(Color::from_rgb(0.31, 0.36, 0.40)),
        )
}

pub(crate) fn context_menu_item_style(status: button::Status) -> iced::widget::button::Style {
    let background = match status {
        button::Status::Active => Color::from_rgb(0.18, 0.21, 0.24),
        button::Status::Hovered => Color::from_rgb(0.25, 0.32, 0.36),
        button::Status::Pressed => Color::from_rgb(0.20, 0.41, 0.47),
        button::Status::Disabled => Color::from_rgb(0.15, 0.17, 0.19),
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::from_rgb(0.93, 0.95, 0.96),
        border: Border::default().rounded(3),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

pub(crate) fn menu_button_style(status: button::Status) -> iced::widget::button::Style {
    let background = match status {
        button::Status::Active => Color::from_rgb(0.16, 0.18, 0.20),
        button::Status::Hovered => Color::from_rgb(0.23, 0.29, 0.32),
        button::Status::Pressed => Color::from_rgb(0.20, 0.41, 0.47),
        button::Status::Disabled => Color::from_rgb(0.13, 0.15, 0.17),
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::from_rgb(0.93, 0.95, 0.96),
        border: Border::default().rounded(3),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

pub(crate) fn menu_bar_style(_: &iced::Theme, _: Status) -> iced_aw::menu::Style {
    iced_aw::menu::Style {
        bar_background: Background::Color(Color::from_rgb(0.10, 0.12, 0.14)),
        bar_border: Border::default()
            .rounded(4)
            .width(1)
            .color(Color::from_rgb(0.23, 0.28, 0.31)),
        menu_background: Background::Color(Color::from_rgb(0.13, 0.15, 0.17)),
        menu_border: Border::default()
            .rounded(4)
            .width(1)
            .color(Color::from_rgb(0.31, 0.36, 0.40)),
        path: Background::Color(Color::from_rgb(0.25, 0.32, 0.36)),
        path_border: Border::default().rounded(3),
        ..Default::default()
    }
}

pub(crate) fn reference_color() -> Color {
    Color::from_rgb(0.38, 0.78, 0.92)
}

pub(crate) fn warning_color() -> Color {
    Color::from_rgb(0.94, 0.57, 0.36)
}

fn selected_row_color() -> Color {
    Color::from_rgba(0.38, 0.78, 0.92, 0.14)
}

fn selected_border_color() -> Color {
    Color::from_rgba(0.38, 0.78, 0.92, 0.65)
}
