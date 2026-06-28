use crate::style::{reference_color, warning_color};
use dagcal_core::{EntryState, EntryView, ExpressionId};
use iced::{Color, Font};
use std::collections::BTreeSet;

pub(crate) fn state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(err) => format!("error: {err}"),
    }
}

pub(crate) fn table_state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(_) => "Error".to_string(),
    }
}

pub(crate) fn entry_expression_source(entry: &EntryView) -> String {
    match &entry.name {
        Some(name) => format!("{name} = {}", entry.source),
        None => entry.source.clone(),
    }
}

pub(crate) fn entry_reference_token(entry: &EntryView) -> String {
    entry.name.clone().unwrap_or_else(|| entry.id.to_string())
}

pub(crate) fn resolved_source(source: &str, entries: &[EntryView]) -> String {
    let mut resolved = String::new();
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

        resolved.push_str(&source[normal_start..start]);
        let id = ExpressionId::new(index);
        resolved.push_str(&reference_replacement(id, entries).label);
        normal_start = end;
    }

    resolved.push_str(&source[normal_start..]);
    resolved
}

pub(crate) fn expression_spans(
    source: &str,
    entries: &[EntryView],
) -> Vec<iced::widget::text::Span<'static, (), Font>> {
    let mut spans = Vec::new();
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
            spans.push(iced::widget::span(source[normal_start..start].to_string()));
        }

        let display = reference_replacement(ExpressionId::new(index), entries);
        spans.push(iced::widget::span(display.label).color(display.color));
        normal_start = end;
    }

    if normal_start < source.len() {
        spans.push(iced::widget::span(source[normal_start..].to_string()));
    }

    if spans.is_empty() {
        spans.push(iced::widget::span(source.to_string()));
    }

    spans
}

struct ReferenceReplacement {
    label: String,
    color: Color,
}

fn reference_replacement(id: ExpressionId, entries: &[EntryView]) -> ReferenceReplacement {
    match entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| &entry.state)
    {
        Some(EntryState::Value(value)) => ReferenceReplacement {
            label: value.to_string(),
            color: reference_color(),
        },
        Some(EntryState::Error(_)) => ReferenceReplacement {
            label: format!("<error {id}>"),
            color: warning_color(),
        },
        None => ReferenceReplacement {
            label: format!("<missing {id}>"),
            color: warning_color(),
        },
    }
}

pub(crate) fn needs_space_before_reference(source: &str) -> bool {
    source
        .chars()
        .last()
        .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, '(' | '+' | '-' | '*' | '/' | '^'))
}

pub(crate) fn entry_set_summary(ids: &BTreeSet<ExpressionId>, entries: &[EntryView]) -> String {
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

fn compact_state_summary(state: &EntryState) -> String {
    match state {
        EntryState::Value(value) => value.to_string(),
        EntryState::Error(_) => "error".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_core::Engine;

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
}
