use crate::app::Message;
use iced::widget::{container, scrollable, text};
use iced::{Background, Border, Color, Element, Fill, Length};

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
