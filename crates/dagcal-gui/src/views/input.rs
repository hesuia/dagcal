use crate::app::{EXPRESSION_INPUT_ID, GuiApp, Message};
use crate::formatting::resolved_source;
use crate::style::{
    DETAIL_HEIGHT, completion_item_style, completion_panel_style, fixed_scroll_text,
};
use dagcal_core::CompletionKind;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Fill, Length};
use iced_aw::{DropDown, drop_down};

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
        let input = DropDown::new(input, self.completion_view(), self.completion_is_open())
            .width(Length::Fill)
            .alignment(drop_down::Alignment::BottomStart)
            .offset(drop_down::Offset { x: 0.0, y: 4.0 })
            .on_dismiss(Message::DismissCompletion);

        let mut actions = row![
            button("New").on_press(Message::NewEntry),
            button("Save").on_press(Message::Submit)
        ]
        .spacing(8)
        .align_y(iced::Center);

        if self.editing.is_some() {
            actions = actions.push(button("Cancel").on_press(Message::CancelEdit));
        }

        column![text(label).size(16), input,]
            .spacing(8)
            .push(fixed_scroll_text(
                &format!("Resolved: {resolved}"),
                DETAIL_HEIGHT,
            ))
            .push(fixed_scroll_text(&preview, DETAIL_HEIGHT))
            .push(actions)
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

    fn completion_view(&self) -> Element<'_, Message> {
        let selected = self.selected_completion_index();
        let mut candidates = column![].spacing(3);

        for (index, candidate) in self.completion_candidates().iter().enumerate() {
            let detail = candidate
                .detail
                .as_deref()
                .map(|detail| format!("  {detail}"))
                .unwrap_or_default();
            let row = row![
                text(kind_label(candidate.kind))
                    .size(12)
                    .width(Length::Fixed(74.0)),
                text(candidate.label.clone())
                    .size(14)
                    .width(Length::FillPortion(2)),
                text(detail).size(12).width(Length::FillPortion(2)),
            ]
            .spacing(8)
            .align_y(iced::Center);

            candidates = candidates.push(
                button(
                    container(row)
                        .padding([5, 8])
                        .width(Fill)
                        .style(move |_| completion_item_style(selected == Some(index))),
                )
                .width(Fill)
                .padding(0)
                .on_press(Message::AcceptCompletion(index)),
            );
        }

        container(candidates)
            .padding(5)
            .width(Fill)
            .style(|_| completion_panel_style())
            .into()
    }
}

fn kind_label(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Entry => "entry",
        CompletionKind::Result => "result",
        CompletionKind::Constant => "constant",
        CompletionKind::Function => "function",
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
