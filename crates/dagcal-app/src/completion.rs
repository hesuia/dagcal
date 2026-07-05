use crate::{CompletionItem, CompletionKind, Draft, Engine, ExpressionId, SessionChange};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionState {
    token_range: Option<CompletionToken>,
    items: Vec<CompletionCandidate>,
    selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionToken {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionCandidate {
    pub label: String,
    pub detail: Option<String>,
    pub insert: String,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionMenuEntry {
    pub label: String,
    pub detail: Option<String>,
}

impl CompletionState {
    pub fn for_source(source: &str, items: Vec<CompletionItem>) -> Self {
        let Some(token) = completion_token(source) else {
            return Self::default();
        };

        let prefix = &source[token.start..token.end];
        let candidates = items
            .into_iter()
            .filter(|item| completion_matches(item, prefix))
            .map(completion_candidate)
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            Self::default()
        } else {
            Self {
                token_range: Some(token),
                items: candidates,
                selected: 0,
            }
        }
    }

    pub fn refresh(&mut self, source: &str, items: Vec<CompletionItem>) {
        *self = Self::for_source(source, items);
    }

    pub fn items(&self) -> &[CompletionCandidate] {
        &self.items
    }

    pub fn is_open(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.is_open().then_some(self.selected)
    }

    pub fn clear(&mut self) {
        self.token_range = None;
        self.items.clear();
        self.selected = 0;
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }

        self.selected = index;
        true
    }

    pub fn move_selection(&mut self, direction: CompletionDirection) {
        if self.items.is_empty() {
            return;
        }

        self.selected = match direction {
            CompletionDirection::Previous => self.selected.saturating_sub(1),
            CompletionDirection::Next => (self.selected + 1).min(self.items.len() - 1),
        };
    }

    pub fn selected_candidate(&self) -> Option<(CompletionToken, CompletionCandidate)> {
        Some((self.token_range?, self.items.get(self.selected)?.clone()))
    }

    pub fn accept_selected(&mut self, draft: &mut Draft) -> Option<String> {
        let (range, candidate) = self.selected_candidate()?;
        draft.replace_range(range.start..range.end, &candidate.insert);
        self.clear();
        Some(candidate.insert)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionDirection {
    Previous,
    Next,
}

fn completion_token(source: &str) -> Option<CompletionToken> {
    let end = source.len();
    let start = source
        .char_indices()
        .rev()
        .find_map(|(index, ch)| (!is_completion_char(ch)).then_some(index + ch.len_utf8()))
        .unwrap_or(0);

    if start == end {
        return None;
    }

    let token = &source[start..end];
    let valid = if let Some(rest) = token.strip_prefix('$') {
        rest.chars().all(|ch| ch.is_ascii_digit())
    } else {
        token
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    };

    valid.then_some(CompletionToken { start, end })
}

fn is_completion_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

fn completion_matches(item: &CompletionItem, prefix: &str) -> bool {
    if prefix == "$" {
        return item.kind == CompletionKind::Result;
    }

    if prefix.starts_with('$') {
        return item.kind == CompletionKind::Result && item.label.starts_with(prefix);
    }

    item.label
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
}

fn completion_candidate(item: CompletionItem) -> CompletionCandidate {
    let insert = match item.kind {
        CompletionKind::Function => format!("{}()", item.label),
        _ => item.label.clone(),
    };

    CompletionCandidate {
        label: item.label,
        detail: item.detail,
        insert,
        kind: item.kind,
    }
}

pub(crate) fn refresh_completion_state(
    completion: &mut CompletionState,
    input: &Draft,
    engine: &Engine,
    exclude_entry_id: Option<ExpressionId>,
) {
    let mut items = engine.completion_items();
    if let Some(id) = exclude_entry_id {
        items.retain(|item| !completion_item_references_entry(item, id));
    }

    completion.refresh(input.source(), items);
}

fn completion_item_references_entry(item: &CompletionItem, id: ExpressionId) -> bool {
    let id = id.to_string();
    match item.kind {
        CompletionKind::Entry => item.detail.as_deref() == Some(id.as_str()),
        CompletionKind::Result => item.label == id,
        CompletionKind::Constant | CompletionKind::Function => false,
    }
}

pub(crate) fn accept_selected_completion(
    completion: &mut CompletionState,
    input: &mut Draft,
    status: &mut String,
) -> Option<SessionChange> {
    let insert = completion.accept_selected(input)?;
    *status = format!("Inserted {insert}");
    Some(SessionChange::FocusInput)
}

pub fn completion_menu_entries_for_kind(
    items: Vec<CompletionItem>,
    kind: CompletionKind,
) -> Vec<CompletionMenuEntry> {
    items
        .into_iter()
        .filter(|item| item.kind == kind)
        .map(|item| CompletionMenuEntry {
            label: item.label,
            detail: item.detail,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_state_finds_named_entries_from_suffix() {
        let mut engine = Engine::new();
        engine.execute("subtotal = 10");

        let state = CompletionState::for_source("sub", engine.completion_items());

        assert_eq!(state.items[0].label, "subtotal");
        assert_eq!(
            state.token_range,
            Some(CompletionToken { start: 0, end: 3 })
        );
    }

    #[test]
    fn completion_state_finds_result_references() {
        let mut engine = Engine::new();
        engine.execute("10");

        let state = CompletionState::for_source("$", engine.completion_items());

        assert!(state.items.iter().any(|item| item.insert == "$1"));
    }

    #[test]
    fn completion_state_keeps_all_matching_candidates() {
        let mut engine = Engine::new();
        for index in 0..10 {
            engine.execute(&format!("item_{index} = {index}"));
        }

        let state = CompletionState::for_source("item", engine.completion_items());

        assert_eq!(
            state
                .items()
                .iter()
                .filter(|item| item.kind == CompletionKind::Entry)
                .count(),
            10
        );
    }

    #[test]
    fn function_completion_inserts_call_template() {
        let engine = Engine::new();

        let state = CompletionState::for_source("si", engine.completion_items());

        assert!(state.items.iter().any(|item| item.insert == "sin()"));
    }

    #[test]
    fn invalid_suffix_has_no_completions() {
        let engine = Engine::new();

        let state = CompletionState::for_source("1", engine.completion_items());

        assert!(!state.is_open());
    }

    #[test]
    fn menu_entries_filter_completion_items_by_kind() {
        let mut engine = Engine::new();
        engine.execute("x = 10");

        let constants =
            completion_menu_entries_for_kind(engine.completion_items(), CompletionKind::Constant);

        assert!(constants.iter().all(|entry| entry.label != "x"));
    }

    #[test]
    fn refresh_completion_state_excludes_current_entry_name_and_result() {
        let mut engine = Engine::new();
        engine.execute("subtotal = 10");
        engine.execute("subtotal * 2");
        let mut state = CompletionState::default();
        let mut input = Draft::default();

        input.set("sub".to_string());
        refresh_completion_state(&mut state, &input, &engine, Some(ExpressionId::new(1)));
        assert!(
            state
                .items()
                .iter()
                .all(|item| item.label != "subtotal" && item.kind != CompletionKind::Entry)
        );

        input.set("$".to_string());
        refresh_completion_state(&mut state, &input, &engine, Some(ExpressionId::new(1)));
        assert!(state.items().iter().all(|item| item.label != "$1"));
        assert!(state.items().iter().any(|item| item.label == "$2"));
    }
}
