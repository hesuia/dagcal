use crate::app::{GuiApp, Message};
use crate::style::{DETAIL_HEIGHT, fixed_line, fixed_scroll_text};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, Length, window};

impl GuiApp {
    pub(super) fn selected_detail_view(&self) -> Element<'_, Message> {
        let Some(id) = self.session.selected else {
            return fixed_line(
                text("Details: select an entry").size(14),
                DETAIL_HEIGHT * 2.0,
            );
        };

        let Some(entry) = self.session.entries.iter().find(|entry| entry.id == id) else {
            return fixed_line(
                text("Details: selected entry is not available").size(14),
                DETAIL_HEIGHT * 2.0,
            );
        };

        row![
            container(fixed_scroll_text(
                &self.selected_compact_text(id, entry),
                DETAIL_HEIGHT * 2.0
            ))
            .width(Length::FillPortion(4)),
            button("Details...").on_press(Message::ShowDetails(id)),
        ]
        .spacing(12)
        .align_y(iced::Center)
        .into()
    }

    pub(super) fn details_window_view(&self, window: window::Id) -> Element<'_, Message> {
        let title = match self.details_target {
            Some(id) => format!("Details for {id}"),
            None => "Details".to_string(),
        };

        container(
            column![
                text(title).size(28),
                scrollable(text(self.details_window_text()).size(15)).height(Length::Fill),
                button("Close").on_press(Message::WindowClosed(window)),
            ]
            .spacing(16)
            .padding(24)
            .width(Fill)
            .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }

    pub(super) fn details_window_text(&self) -> String {
        let Some(id) = self.details_target else {
            return "No entry selected.".to_string();
        };

        let Some(entry) = self.session.entries.iter().find(|entry| entry.id == id) else {
            return format!("{id} is not available.");
        };

        format!(
            "{}\n\n{}",
            self.selected_summary_text(id, entry),
            self.selected_error_text(entry)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_app::ExpressionId;

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

    #[test]
    fn selected_compact_text_does_not_include_full_error_detail() {
        let (mut app, _) = GuiApp::new();
        let _ = app.update(Message::InputChanged("1 / 0".to_string()));
        let _ = app.update(Message::Submit);
        let entry = app.entries[0].clone();

        let detail = app.selected_compact_text(ExpressionId::new(1), &entry);

        assert!(detail.contains("Result: Error"));
        assert!(!detail.contains("Error detail:"));
    }

    #[test]
    fn details_window_text_includes_summary_and_error_detail() {
        let (mut app, _) = GuiApp::new();
        let _ = app.update(Message::InputChanged("1 / 0".to_string()));
        let _ = app.update(Message::Submit);
        app.details_target = Some(ExpressionId::new(1));

        let detail = app.details_window_text();

        assert!(detail.contains("$1  Expression: 1 / 0"));
        assert!(detail.contains("Result: Error"));
        assert!(detail.contains("Error detail:"));
    }

    #[test]
    fn details_window_text_reports_missing_target() {
        let (mut app, _) = GuiApp::new();
        app.details_target = Some(ExpressionId::new(99));

        assert_eq!(app.details_window_text(), "$99 is not available.");
    }
}
