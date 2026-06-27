use dagcal_core::{Engine, EntryState, EntryView, ExpressionId};
use iced::widget::{button, column, container, row, rule, scrollable, text, text_input};
use iced::{Element, Fill, Length, Task, Theme};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    Submit,
    Edit(ExpressionId),
    CancelEdit,
    Delete(ExpressionId),
    InsertReference(ExpressionId),
    Clear,
}

pub fn run() -> iced::Result {
    iced::application(GuiApp::new, GuiApp::update, GuiApp::view)
        .title("dagcal")
        .theme(app_theme)
        .run()
}

fn app_theme(_: &GuiApp) -> Theme {
    // Theme::TokyoNight
    Theme::KanagawaDragon
}

pub struct GuiApp {
    engine: Engine,
    entries: Vec<EntryView>,
    input: Draft,
    editing: Option<ExpressionId>,
    selected: Option<ExpressionId>,
    status: String,
}

impl GuiApp {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                engine: Engine::new(),
                entries: Vec::new(),
                input: Draft::default(),
                editing: None,
                selected: None,
                status: "Ready".to_string(),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input.set(value);
            }
            Message::Submit => self.submit_input(),
            Message::Edit(id) => self.start_edit(id),
            Message::CancelEdit => self.cancel_edit(),
            Message::Delete(id) => self.delete_entry(id),
            Message::InsertReference(id) => self.insert_reference(id),
            Message::Clear => self.clear(),
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let entries = self.entries_view();
        let input = self.input_view();

        container(
            column![
                header_view(),
                entries,
                rule::horizontal(1),
                input,
                text(&self.status).size(14),
            ]
            .spacing(12)
            .padding(16),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn submit_input(&mut self) {
        let source = self.input.source().trim().to_string();
        if source.is_empty() {
            self.status = "Input is empty".to_string();
            return;
        }

        if let Some(id) = self.editing {
            let result = self.engine.set_entry_by_id(id, source);
            self.refresh_affected(&result.execution.affected_ids);
            self.selected = Some(id);
            self.status = match self.engine.entry_by_id(id) {
                Some(entry) => format!("{id} = {}", state_summary(&entry.state)),
                None => format!("{id} updated"),
            };
        } else {
            let execution = self.engine.execute(&source);
            self.refresh_affected(&execution.affected_ids);
            self.selected = Some(execution.id);
            self.status = format!("{} = {}", execution.id, state_summary(&execution.state));
        }

        self.editing = None;
        self.input.clear();
    }

    fn start_edit(&mut self, id: ExpressionId) {
        match self.engine.entry_by_id(id) {
            Some(entry) => {
                self.input.set(entry.source);
                self.editing = Some(id);
                self.selected = Some(id);
                self.status = format!("Editing {id}");
            }
            None => {
                self.status = format!("{id} is not available");
            }
        }
    }

    fn cancel_edit(&mut self) {
        self.editing = None;
        self.input.clear();
        self.status = "Edit cancelled".to_string();
    }

    fn delete_entry(&mut self, id: ExpressionId) {
        if let Some(removal) = self.engine.remove_entry_by_id(id) {
            self.entries
                .retain(|entry| entry.id != removal.removed_entry.id);
            self.refresh_affected(&removal.affected_ids);
            if self.selected == Some(id) {
                self.selected = self.entries.last().map(|entry| entry.id);
            }
            if self.editing == Some(id) {
                self.editing = None;
                self.input.clear();
            }
            self.status = format!("Removed {id}");
        } else {
            self.status = format!("{id} is not available");
        }
    }

    fn insert_reference(&mut self, id: ExpressionId) {
        self.input.insert_reference(id);
        self.selected = Some(id);
        self.status = format!("Inserted {id}");
    }

    fn clear(&mut self) {
        self.engine = Engine::new();
        self.entries.clear();
        self.input.clear();
        self.editing = None;
        self.selected = None;
        self.status = "Cleared".to_string();
    }

    fn refresh_affected(&mut self, ids: &BTreeSet<ExpressionId>) {
        for id in ids {
            match self.engine.entry_by_id(*id) {
                Some(entry) => self.upsert_cached_entry(entry),
                None => self.entries.retain(|entry| entry.id != *id),
            }
        }
    }

    fn upsert_cached_entry(&mut self, entry: EntryView) {
        match self
            .entries
            .binary_search_by_key(&entry.id, |entry| entry.id)
        {
            Ok(index) => self.entries[index] = entry,
            Err(index) => self.entries.insert(index, entry),
        }
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
                list = list.push(entry_row(entry, self.selected == Some(entry.id)));
            }
        }

        scrollable(list).height(Length::FillPortion(3)).into()
    }

    fn input_view(&self) -> Element<'_, Message> {
        let source = self.input.source();
        let label = match self.editing {
            Some(id) => format!("Edit {id}"),
            None => "New expression".to_string(),
        };
        let resolved = resolved_source(source, &self.entries);
        let preview = self.preview_summary(source);
        let references = reference_chips(source, &self.entries);

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

        let mut refs = row![text("Insert result:").size(14)]
            .spacing(6)
            .align_y(iced::Center);
        for entry in &self.entries {
            refs = refs.push(
                button(text(entry.id.to_string())).on_press(Message::InsertReference(entry.id)),
            );
        }

        column![
            text(label).size(16),
            input,
            text(format!("Resolved: {resolved}")).size(14),
            text(preview).size(14),
            text(references).size(14),
            refs,
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

        match self.engine.eval_once(source) {
            Ok(value) => format!("Preview: {value}"),
            Err(err) => format!("Preview error: {err}"),
        }
    }
}

#[derive(Debug, Default)]
struct Draft {
    source: String,
}

impl Draft {
    fn source(&self) -> &str {
        &self.source
    }

    fn set(&mut self, source: String) {
        self.source = source;
    }

    fn clear(&mut self) {
        self.source.clear();
    }

    fn insert_reference(&mut self, id: ExpressionId) {
        if needs_space_before_reference(&self.source) {
            self.source.push(' ');
        }
        self.source.push_str(&id.to_string());
        self.source.push(' ');
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
        text("Name").width(Length::Fixed(140.0)),
        text("Expression").width(Length::FillPortion(3)),
        text("Result").width(Length::FillPortion(2)),
        text("Actions").width(Length::Fixed(220.0)),
    ]
    .spacing(8)
    .into()
}

fn entry_row(entry: &EntryView, selected: bool) -> Element<'_, Message> {
    let marker = if selected { ">" } else { " " };
    let name = entry.name.as_deref().unwrap_or("");

    row![
        text(format!("{marker} {}", entry.id)).width(Length::Fixed(60.0)),
        text(name.to_string()).width(Length::Fixed(140.0)),
        text(entry.source.clone()).width(Length::FillPortion(3)),
        text(state_summary(&entry.state)).width(Length::FillPortion(2)),
        row![
            button("Use").on_press(Message::InsertReference(entry.id)),
            button("Edit").on_press(Message::Edit(entry.id)),
            button("Delete").on_press(Message::Delete(entry.id)),
        ]
        .spacing(6)
        .width(Length::Fixed(220.0)),
    ]
    .spacing(8)
    .align_y(iced::Center)
    .into()
}

fn state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(err) => format!("error: {err}"),
    }
}

fn reference_chips(source: &str, entries: &[EntryView]) -> String {
    let references = extract_reference_ids(source);
    if references.is_empty() {
        return "References: none".to_string();
    }

    let mut chips = Vec::new();
    for id in references {
        match entries.iter().find(|entry| entry.id == id) {
            Some(entry) => chips.push(format!("{id} = {}", state_summary(&entry.state))),
            None => chips.push(format!("{id} = missing")),
        }
    }

    format!("References: {}", chips.join(", "))
}

fn resolved_source(source: &str, entries: &[EntryView]) -> String {
    let mut resolved = String::new();
    let mut chars = source.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch != '$' {
            resolved.push(ch);
            continue;
        }

        let mut number = String::new();
        while let Some((_, next)) = chars.peek().copied() {
            if next.is_ascii_digit() {
                number.push(next);
                chars.next();
            } else {
                break;
            }
        }

        if number.is_empty() {
            resolved.push('$');
            continue;
        }

        let Some(index) = number.parse::<usize>().ok() else {
            resolved.push('$');
            resolved.push_str(&number);
            continue;
        };

        let id = ExpressionId::new(index);
        match entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| &entry.state)
        {
            Some(EntryState::Value(value)) => resolved.push_str(&value.to_string()),
            Some(EntryState::Error(_)) => resolved.push_str(&format!("{id}<error>")),
            None => resolved.push_str(&format!("{id}<missing>")),
        }
    }

    resolved
}

fn extract_reference_ids(source: &str) -> Vec<ExpressionId> {
    let mut ids = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch != '$' {
            continue;
        }

        let mut number = String::new();
        while let Some((_, next)) = chars.peek().copied() {
            if next.is_ascii_digit() {
                number.push(next);
                chars.next();
            } else {
                break;
            }
        }

        if let Ok(index) = number.parse::<usize>() {
            let id = ExpressionId::new(index);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }

    ids
}

fn needs_space_before_reference(source: &str) -> bool {
    source
        .chars()
        .last()
        .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, '(' | '+' | '-' | '*' | '/' | '^'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_core::Number;

    #[test]
    fn draft_inserts_result_reference_without_replacing_saved_source() {
        let mut draft = Draft::default();

        draft.set("1 +".to_string());
        draft.insert_reference(ExpressionId::new(1));

        assert_eq!(draft.source(), "1 +$1 ");
    }

    #[test]
    fn resolved_source_replaces_existing_result_references_for_display() {
        let mut engine = Engine::new();
        engine.execute("21");
        let entries = engine.entries();

        assert_eq!(resolved_source("$1 * 2", &entries), "21 * 2");
    }

    #[test]
    fn submit_edit_recomputes_dependents() {
        let (mut app, _) = GuiApp::new();
        app.input.set("base = 10".to_string());
        app.submit_input();
        app.input.set("$1 * 2".to_string());
        app.submit_input();

        app.start_edit(ExpressionId::new(1));
        app.input.set("20".to_string());
        app.submit_input();

        assert_eq!(app.entries[0].state, EntryState::Value(Number::from(20)));
        assert_eq!(app.entries[1].state, EntryState::Value(Number::from(40)));
    }

    #[test]
    fn delete_keeps_later_ids_available() {
        let (mut app, _) = GuiApp::new();
        app.input.set("10".to_string());
        app.submit_input();
        app.input.set("20".to_string());
        app.submit_input();

        app.delete_entry(ExpressionId::new(1));

        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].id, ExpressionId::new(2));
    }
}
