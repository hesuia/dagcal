use crate::{Engine, EntryState, EntryView, ExpressionId};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceSegment {
    Text(String),
    Value(String),
    Error(String),
    Missing(String),
}

impl ReferenceSegment {
    pub fn text(&self) -> &str {
        match self {
            Self::Text(text) | Self::Value(text) | Self::Error(text) | Self::Missing(text) => text,
        }
    }
}

pub fn state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(err) => format!("error: {err}"),
    }
}

pub fn table_state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(_) => "Error".to_string(),
    }
}

pub fn entry_expression_source(entry: &EntryView) -> String {
    match &entry.name {
        Some(name) => format!("{name} = {}", entry.source),
        None => entry.source.clone(),
    }
}

pub fn entry_reference_token(entry: &EntryView) -> String {
    entry.name.clone().unwrap_or_else(|| entry.id.to_string())
}

pub fn resolved_source(source: &str, entries: &[EntryView]) -> String {
    reference_segments(source, entries)
        .into_iter()
        .map(|segment| segment.text().to_string())
        .collect()
}

pub fn reference_segments(source: &str, entries: &[EntryView]) -> Vec<ReferenceSegment> {
    let mut segments = Vec::new();
    let mut normal_start = 0;
    let mut chars = source.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        if ch != '$' {
            continue;
        }

        let mut number = String::new();
        let mut end = start + ch.len_utf8();
        while let Some((index, next)) = chars.peek().copied() {
            if next.is_ascii_digit() {
                number.push(next);
                end = index + next.len_utf8();
                chars.next();
            } else {
                break;
            }
        }

        if number.is_empty() {
            continue;
        }

        let Some(index) = number.parse::<usize>().ok() else {
            continue;
        };

        if normal_start < start {
            segments.push(ReferenceSegment::Text(
                source[normal_start..start].to_string(),
            ));
        }

        segments.push(reference_replacement(ExpressionId::new(index), entries));
        normal_start = end;
    }

    if normal_start < source.len() {
        segments.push(ReferenceSegment::Text(source[normal_start..].to_string()));
    }

    if segments.is_empty() {
        segments.push(ReferenceSegment::Text(source.to_string()));
    }

    segments
}

fn reference_replacement(id: ExpressionId, entries: &[EntryView]) -> ReferenceSegment {
    match entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| &entry.state)
    {
        Some(EntryState::Value(value)) => ReferenceSegment::Value(value.to_string()),
        Some(EntryState::Error(_)) => ReferenceSegment::Error(format!("<error {id}>")),
        None => ReferenceSegment::Missing(format!("<missing {id}>")),
    }
}

pub fn needs_space_before_reference(source: &str) -> bool {
    source
        .chars()
        .last()
        .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, '(' | '+' | '-' | '*' | '/' | '^'))
}

pub fn entry_set_summary(ids: &BTreeSet<ExpressionId>, entries: &[EntryView]) -> String {
    if ids.is_empty() {
        return "none".to_string();
    }

    ids.iter()
        .map(|id| match entries.iter().find(|entry| entry.id == *id) {
            Some(entry) => format!("{id} = {}", compact_state_summary(&entry.state)),
            None => format!("{id} = missing"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn selected_summary_text(
    engine: &Engine,
    entries: &[EntryView],
    draft_entry: Option<ExpressionId>,
    id: ExpressionId,
    entry: &EntryView,
) -> String {
    let dependencies = engine.dependencies_of(id);
    let dependents = engine.dependents_of(id);
    let expression = entry_expression_source(entry);
    let result = selected_result_summary(&entry.state, draft_entry == Some(id));

    format!(
        "{id}  Expression: {expression}\n{result}\nDepends on: {}    Used by: {}",
        entry_set_summary(&dependencies, entries),
        entry_set_summary(&dependents, entries)
    )
}

pub fn selected_error_text(draft_entry: Option<ExpressionId>, entry: &EntryView) -> String {
    if draft_entry == Some(entry.id) {
        return "Error detail: none".to_string();
    }

    match &entry.state {
        EntryState::Value(_) => "Error detail: none".to_string(),
        EntryState::Error(err) => format!("Error detail:\n{err}"),
    }
}

pub fn selected_compact_text(
    engine: &Engine,
    entries: &[EntryView],
    draft_entry: Option<ExpressionId>,
    id: ExpressionId,
    entry: &EntryView,
) -> String {
    let dependencies = engine.dependencies_of(id);
    let dependents = engine.dependents_of(id);
    let result = selected_result_summary(&entry.state, draft_entry == Some(id));

    format!(
        "{id}  {result}\nDepends on: {}    Used by: {}",
        entry_set_summary(&dependencies, entries),
        entry_set_summary(&dependents, entries)
    )
}

pub fn selected_result_summary(state: &EntryState, draft: bool) -> String {
    if draft {
        "Result: None".to_string()
    } else {
        match state {
            EntryState::Value(value) => format!("Result: {value}"),
            EntryState::Error(_) => "Result: Error".to_string(),
        }
    }
}

fn compact_state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(_) => "error".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, Number};

    #[test]
    fn entry_expression_source_includes_name_for_named_entries() {
        let mut engine = Engine::new();
        engine.execute("x = 10");
        let entries = engine.entries();

        assert_eq!(entry_expression_source(&entries[0]), "x = 10");
    }

    #[test]
    fn entry_reference_token_prefers_name_over_result_id() {
        let mut engine = Engine::new();
        engine.execute("x = 10");
        engine.execute("20");
        let entries = engine.entries();

        assert_eq!(entry_reference_token(&entries[0]), "x");
        assert_eq!(entry_reference_token(&entries[1]), "$2");
    }

    #[test]
    fn resolved_source_replaces_existing_result_references_for_display() {
        let mut engine = Engine::new();
        engine.execute("21");
        let entries = engine.entries();

        assert_eq!(resolved_source("$1 * 2", &entries), "21 * 2");
    }

    #[test]
    fn resolved_source_marks_error_and_missing_references() {
        let mut engine = Engine::new();
        engine.execute("1 / 0");
        let entries = engine.entries();

        assert_eq!(
            resolved_source("$1 + $9", &entries),
            "<error $1> + <missing $9>"
        );
    }

    #[test]
    fn entry_set_summary_reports_values_errors_and_missing_ids() {
        let mut engine = Engine::new();
        engine.execute("21");
        engine.execute("1 / 0");
        let entries = engine.entries();
        let ids = BTreeSet::from([
            ExpressionId::new(1),
            ExpressionId::new(2),
            ExpressionId::new(9),
        ]);

        assert_eq!(
            entry_set_summary(&ids, &entries),
            "$1 = 21, $2 = error, $9 = missing"
        );
    }

    #[test]
    fn table_state_summary_keeps_errors_compact() {
        let mut engine = Engine::new();
        engine.execute("1 / 0");
        let entries = engine.entries();

        assert_eq!(table_state_summary(&entries[0].state), "Error");
    }

    #[test]
    fn selected_result_summary_compacts_errors() {
        assert_eq!(
            selected_result_summary(&EntryState::Value(Number::from(10)), false),
            "Result: 10"
        );
    }
}
