use crate::app::{
    ENTRIES_SCROLLABLE_ID, ENTRY_ROW_ID_PREFIX, ENTRY_SEARCH_INPUT_ID, EntryStateFilter, GuiApp,
    Message,
};
use crate::formatting::{entry_expression_source, expression_spans, table_state_summary};
use crate::style::{
    TABLE_TEXT_SIZE, context_menu_item_style, context_menu_panel_style, menu_button_style,
    row_container_style, warning_color,
};
use dagcal_app::{EntryState, EntryView, ExpressionId};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, mouse_area, rich_text, row, scrollable, text, text_input,
};
use iced::{Element, Fill, Length};
use iced_aw::ContextMenu;

impl GuiApp {
    pub(super) fn entries_view(&self) -> Element<'_, Message> {
        let filtered_entries: Vec<_> = self.session.filtered_entries_iter().collect();
        let mut entries = column![].spacing(6);
        if self.session.entry_search_is_open() {
            entries = entries.push(entry_filters(self));
        }
        entries = entries.push(entry_header());

        let mut list = column![].spacing(6);

        if self.session.entries().is_empty() {
            list = list.push(
                container(text("No entries yet").size(16))
                    .padding(12)
                    .width(Fill),
            );
        } else if filtered_entries.is_empty() {
            list = list.push(
                container(text("No matching entries").size(16))
                    .padding(12)
                    .width(Fill),
            );
        } else {
            for entry in filtered_entries {
                list = list.push(entry_row(
                    entry,
                    self.session.entries(),
                    self.session.selected_id() == Some(entry.id),
                    self.session.draft_entry_id() == Some(entry.id),
                ));
            }
        }

        entries
            .push(
                scrollable(list)
                    .id(ENTRIES_SCROLLABLE_ID)
                    .height(Length::Fill),
            )
            .height(Length::FillPortion(3))
            .into()
    }
}

fn entry_filters(app: &GuiApp) -> Element<'_, Message> {
    let mut filters = row![
        text_input(
            "Search ID, name, expression, result...",
            app.session.entry_search_query()
        )
        .id(ENTRY_SEARCH_INPUT_ID)
        .on_input(Message::EntrySearchChanged)
        .padding(8)
        .size(15)
        .width(Length::FillPortion(3)),
        filter_button(
            "All",
            EntryStateFilter::All,
            app.session.entry_state_filter()
        ),
        filter_button(
            "Values",
            EntryStateFilter::Values,
            app.session.entry_state_filter()
        ),
        filter_button(
            "Errors",
            EntryStateFilter::Errors,
            app.session.entry_state_filter()
        ),
    ]
    .spacing(8)
    .align_y(iced::Center);

    filters = filters.push(
        button("Clear")
            .padding([7, 10])
            .style(|_, status| menu_button_style(status))
            .on_press(Message::ClearEntrySearch),
    );

    filters.into()
}

fn filter_button(
    label: &'static str,
    filter: EntryStateFilter,
    active_filter: EntryStateFilter,
) -> Element<'static, Message> {
    let label = if filter == active_filter {
        format!("[{label}]")
    } else {
        label.to_string()
    };

    button(text(label))
        .padding([7, 10])
        .style(|_, status| menu_button_style(status))
        .on_press(Message::EntryStateFilterChanged(filter))
        .into()
}

fn entry_header() -> Element<'static, Message> {
    row![
        text("ID").width(Length::Fixed(60.0)),
        text("Expression").width(Length::FillPortion(3)),
        text("Result").width(Length::FillPortion(2)),
        text("Use").width(Length::Fixed(80.0)),
    ]
    .spacing(8)
    .into()
}

fn entry_row<'a>(
    entry: &'a EntryView,
    entries: &'a [EntryView],
    selected: bool,
    draft: bool,
) -> Element<'a, Message> {
    let row = mouse_area(
        container(
            row![
                text(if selected {
                    format!("* {}", entry.id)
                } else {
                    format!("  {}", entry.id)
                })
                .width(Length::Fixed(60.0)),
                expression_view(entry, entries),
                result_view(&entry.state, draft),
                row![button("Use").on_press(Message::InsertReference(entry.id)),]
                    .width(Length::Fixed(80.0)),
            ]
            .spacing(8)
            .align_y(iced::Center),
        )
        .id(format!("{ENTRY_ROW_ID_PREFIX}{}", entry.id))
        .padding([4, 6])
        .width(Fill)
        .style(move |_| row_container_style(selected)),
    )
    .on_press(Message::Select(entry.id))
    .on_double_click(Message::Edit(entry.id))
    .on_enter(Message::EntryHovered(entry.id))
    .on_exit(Message::EntryUnhovered(entry.id));

    ContextMenu::new(row, move || entry_context_menu(entry.id)).into()
}

fn entry_context_menu(id: ExpressionId) -> Element<'static, Message> {
    container(
        column![
            context_menu_item("Details...", Message::ShowDetails(id)),
            context_menu_item("Edit", Message::Edit(id)),
            context_menu_item("Recalculate", Message::Recalculate(id)),
            context_menu_item("Delete", Message::Delete(id)),
        ]
        .spacing(3),
    )
    .padding(5)
    .width(Length::Fixed(160.0))
    .style(|_| context_menu_panel_style())
    .into()
}

fn context_menu_item(label: &'static str, message: Message) -> Element<'static, Message> {
    button(text(label).width(Fill))
        .width(Fill)
        .padding([7, 10])
        .style(|_, status| context_menu_item_style(status))
        .on_press(message)
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

fn result_view(state: &EntryState, draft: bool) -> Element<'static, Message> {
    let mut result = text(table_result_summary(state, draft)).size(TABLE_TEXT_SIZE);
    if !draft && matches!(state, EntryState::Error(_)) {
        result = result.color(warning_color());
    }

    result
        .width(Length::FillPortion(2))
        .wrapping(Wrapping::WordOrGlyph)
        .into()
}

fn table_result_summary(state: &EntryState, draft: bool) -> String {
    if draft {
        "None".to_string()
    } else {
        table_state_summary(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_entry_result_text_does_not_show_error() {
        let (mut app, _) = GuiApp::new();
        let _ = app.update(Message::InputChanged("1 + 2".to_string()));
        let entry = app.session.entries()[0].clone();

        assert_eq!(table_result_summary(&entry.state, true), "None");
        assert!(
            app.session
                .selected_summary_text(entry.id, &entry)
                .contains("Result: None")
        );
        assert_eq!(
            app.session.selected_error_text(&entry),
            "Error detail: none"
        );
    }
}
