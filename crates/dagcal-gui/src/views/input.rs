use crate::app::{EXPRESSION_INPUT_ID, GuiApp, Message};
use crate::formatting::resolved_source;
use crate::style::{DETAIL_HEIGHT, fixed_scroll_text};
use iced::Element;
use iced::widget::{button, column, row, text, text_input};

impl GuiApp {
    pub(super) fn input_view(&self) -> Element<'_, Message> {
        let source = self.input.source();
        let label = match self.editing {
            Some(id) => format!("Edit {id}"),
            None => "New expression".to_string(),
        };
        let resolved = resolved_source(source, &self.entries);
        let preview = self.preview_summary(source);

        let input = text_input("1 + 2, subtotal = 100, or $1 * 3", source)
            .id(EXPRESSION_INPUT_ID)
            .on_input(Message::InputChanged)
            .on_submit(Message::Submit)
            .padding(10)
            .size(18);

        let mut actions = row![
            button("New").on_press(Message::NewEntry),
            button("Save").on_press(Message::Submit)
        ]
        .spacing(8)
        .align_y(iced::Center);

        if self.editing.is_some() {
            actions = actions.push(button("Cancel").on_press(Message::CancelEdit));
        }

        column![
            text(label).size(16),
            input,
            fixed_scroll_text(&format!("Resolved: {resolved}"), DETAIL_HEIGHT),
            fixed_scroll_text(&preview, DETAIL_HEIGHT),
            actions,
        ]
        .spacing(8)
        .into()
    }

    pub(super) fn preview_summary(&self, source: &str) -> String {
        let source = source.trim();
        if source.is_empty() {
            return "Preview: empty".to_string();
        }

        match self.engine.eval_statement_once(source) {
            Ok(value) => format!("Preview: {value}"),
            Err(err) => format!("Preview error: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_summary_accepts_named_definitions() {
        let (app, _) = GuiApp::new();

        assert_eq!(app.preview_summary("x = 1"), "Preview: 1");
    }

    #[test]
    fn preview_summary_keeps_expression_preview() {
        let (app, _) = GuiApp::new();

        assert_eq!(app.preview_summary("1 + 2"), "Preview: 3");
    }
}
