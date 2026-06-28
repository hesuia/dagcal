use crate::app::{GuiApp, Message};
use crate::formatting::{
    entry_expression_source, entry_set_summary, expression_spans, resolved_source,
    table_state_summary,
};
use crate::style::{
    DETAIL_HEIGHT, TABLE_TEXT_SIZE, fixed_line, fixed_scroll_text, row_container_style,
    warning_color,
};
use dagcal_core::{EntryState, EntryView, ExpressionId};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, mouse_area, rich_text, row, rule, scrollable, text, text_input,
};
use iced::{Element, Fill, Length};

impl GuiApp {
    pub(crate) fn view(&self) -> Element<'_, Message> {
        let entries = self.entries_view();
        let input = self.input_view();

        container(
            column![
                header_view(),
                entries,
                rule::horizontal(1),
                input,
                self.selected_detail_view(),
            ]
            .spacing(12)
            .padding(16),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn entries_view(&self) -> Element<'_, Message> {
        let mut list = column![entry_header()].spacing(6);

        if self.entries.is_empty() {
            list = list.push(
                container(text("No entries yet").size(16))
                    .padding(12)
                    .width(Fill),
            );
        } else {
            for entry in &self.entries {
                list = list.push(entry_row(
                    entry,
                    &self.entries,
                    self.selected == Some(entry.id),
                ));
            }
        }

        scrollable(list).height(Length::FillPortion(3)).into()
    }

    fn selected_detail_view(&self) -> Element<'_, Message> {
        let Some(id) = self.selected else {
            return fixed_line(
                text("Details: select an entry").size(14),
                DETAIL_HEIGHT * 2.0,
            );
        };

        let Some(entry) = self.entries.iter().find(|entry| entry.id == id) else {
            return fixed_line(
                text("Details: selected entry is not available").size(14),
                DETAIL_HEIGHT * 2.0,
            );
        };

        row![
            container(fixed_scroll_text(
                &self.selected_summary_text(id, entry),
                DETAIL_HEIGHT * 2.0
            ))
            .width(Length::FillPortion(3)),
            container(fixed_scroll_text(
                &self.selected_error_text(entry),
                DETAIL_HEIGHT * 2.0
            ))
            .width(Length::FillPortion(2)),
        ]
        .spacing(12)
        .into()
    }

    fn selected_summary_text(&self, id: ExpressionId, entry: &EntryView) -> String {
        let dependencies = self.engine.dependencies_of(id);
        let dependents = self.engine.dependents_of(id);
        let expression = entry_expression_source(entry);
        let result = match &entry.state {
            EntryState::Value(value) => format!("Result: {value}"),
            EntryState::Error(_) => "Result: Error".to_string(),
        };

        format!(
            "{id}  Expression: {expression}\n{result}\nDepends on: {}    Used by: {}",
            entry_set_summary(&dependencies, &self.entries),
            entry_set_summary(&dependents, &self.entries)
        )
    }

    fn selected_error_text(&self, entry: &EntryView) -> String {
        match &entry.state {
            EntryState::Value(_) => "Error detail: none".to_string(),
            EntryState::Error(err) => format!("Error detail:\n{err}"),
        }
    }

    fn input_view(&self) -> Element<'_, Message> {
        let source = self.input.source();
        let label = match self.editing {
            Some(id) => format!("Edit {id}"),
            None => "New expression".to_string(),
        };
        let resolved = resolved_source(source, &self.entries);
        let preview = self.preview_summary(source);

        let input = text_input("1 + 2, subtotal = 100, or $1 * 3", source)
            .on_input(Message::InputChanged)
            .on_submit(Message::Submit)
            .padding(10)
            .size(18);

        let mut actions = row![button("Save").on_press(Message::Submit)]
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

    fn preview_summary(&self, source: &str) -> String {
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

fn header_view() -> Element<'static, Message> {
    row![
        text("dagcal").size(28),
        text("Expressions are saved as stable $n results").size(14),
        button("Clear").on_press(Message::Clear),
    ]
    .spacing(12)
    .align_y(iced::Center)
    .into()
}

fn entry_header() -> Element<'static, Message> {
    row![
        text("ID").width(Length::Fixed(60.0)),
        text("Expression").width(Length::FillPortion(3)),
        text("Result").width(Length::FillPortion(2)),
        text("Actions").width(Length::Fixed(220.0)),
    ]
    .spacing(8)
    .into()
}

fn entry_row<'a>(
    entry: &'a EntryView,
    entries: &'a [EntryView],
    selected: bool,
) -> Element<'a, Message> {
    mouse_area(
        container(
            row![
                text(if selected {
                    format!("* {}", entry.id)
                } else {
                    format!("  {}", entry.id)
                })
                .width(Length::Fixed(60.0)),
                expression_view(entry, entries),
                result_view(&entry.state),
                row![
                    button("Use").on_press(Message::InsertReference(entry.id)),
                    button("Edit").on_press(Message::Edit(entry.id)),
                    button("Delete").on_press(Message::Delete(entry.id)),
                ]
                .spacing(6)
                .width(Length::Fixed(220.0)),
            ]
            .spacing(8)
            .align_y(iced::Center),
        )
        .padding([4, 6])
        .width(Fill)
        .style(move |_| row_container_style(selected)),
    )
    .on_press(Message::Select(entry.id))
    .into()
}

fn expression_view(entry: &EntryView, entries: &[EntryView]) -> Element<'static, Message> {
    let source = entry_expression_source(entry);

    rich_text(expression_spans(&source, entries))
        .size(TABLE_TEXT_SIZE)
        .width(Length::FillPortion(3))
        .wrapping(Wrapping::WordOrGlyph)
        .into()
}

fn result_view(state: &dagcal_core::EntryState) -> Element<'static, Message> {
    let mut result = text(table_state_summary(state)).size(TABLE_TEXT_SIZE);
    if matches!(state, EntryState::Error(_)) {
        result = result.color(warning_color());
    }

    result
        .width(Length::FillPortion(2))
        .wrapping(Wrapping::WordOrGlyph)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_core::ExpressionId;

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

    #[test]
    fn selected_error_text_includes_full_error() {
        let (mut app, _) = GuiApp::new();
        let _ = app.update(Message::InputChanged("1 / 0".to_string()));
        let _ = app.update(Message::Submit);
        let entry = app.entries[0].clone();

        let detail = app.selected_error_text(&entry);

        assert!(detail.contains("Error detail:"));
        assert!(detail.len() > "Error".len());
    }

    #[test]
    fn selected_summary_text_keeps_error_compact() {
        let (mut app, _) = GuiApp::new();
        let _ = app.update(Message::InputChanged("1 / 0".to_string()));
        let _ = app.update(Message::Submit);
        let entry = app.entries[0].clone();

        let detail = app.selected_summary_text(ExpressionId::new(1), &entry);

        assert!(detail.contains("Result: Error"));
    }
}
