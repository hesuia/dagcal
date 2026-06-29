use crate::app::{GuiApp, Message};
use crate::formatting::{entry_expression_source, entry_set_summary};
use crate::style::{DETAIL_HEIGHT, fixed_line, fixed_scroll_text};
use dagcal_core::{EntryState, EntryView, ExpressionId};
use iced::widget::{container, row, text};
use iced::{Element, Length};

impl GuiApp {
    pub(super) fn selected_detail_view(&self) -> Element<'_, Message> {
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

    pub(super) fn selected_summary_text(&self, id: ExpressionId, entry: &EntryView) -> String {
        let dependencies = self.engine.dependencies_of(id);
        let dependents = self.engine.dependents_of(id);
        let expression = entry_expression_source(entry);
        let result = selected_result_summary(&entry.state, self.draft_entry == Some(id));

        format!(
            "{id}  Expression: {expression}\n{result}\nDepends on: {}    Used by: {}",
            entry_set_summary(&dependencies, &self.entries),
            entry_set_summary(&dependents, &self.entries)
        )
    }

    pub(super) fn selected_error_text(&self, entry: &EntryView) -> String {
        if self.draft_entry == Some(entry.id) {
            return "Error detail: none".to_string();
        }

        match &entry.state {
            EntryState::Value(_) => "Error detail: none".to_string(),
            EntryState::Error(err) => format!("Error detail:\n{err}"),
        }
    }
}

fn selected_result_summary(state: &EntryState, draft: bool) -> String {
    if draft {
        "Result: None".to_string()
    } else {
        match state {
            EntryState::Value(value) => format!("Result: {value}"),
            EntryState::Error(_) => "Result: Error".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
