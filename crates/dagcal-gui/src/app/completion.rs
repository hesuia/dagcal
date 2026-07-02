use dagcal_core::{CompletionItem, CompletionKind};

use super::GuiApp;

const MAX_COMPLETIONS: usize = 8;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CompletionState {
    token_range: Option<CompletionToken>,
    items: Vec<CompletionCandidate>,
    selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompletionToken {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionCandidate {
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
    pub(crate) insert: String,
    pub(crate) kind: CompletionKind,
}

impl GuiApp {
    pub(crate) fn completion_candidates(&self) -> &[CompletionCandidate] {
        self.completion.items()
    }

    pub(crate) fn selected_completion_index(&self) -> Option<usize> {
        self.completion.selected_index()
    }

    pub(crate) fn completion_is_open(&self) -> bool {
        self.completion.is_open()
    }

    pub(super) fn refresh_completions(&mut self) {
        self.completion =
            CompletionState::for_source(self.input.source(), self.engine.completion_items());
    }

    pub(super) fn close_completions(&mut self) {
        self.completion.clear();
    }

    pub(super) fn move_completion_selection(&mut self, direction: CompletionDirection) {
        self.completion.move_selection(direction);
    }

    pub(super) fn accept_selected_completion(&mut self) -> bool {
        let Some((range, candidate)) = self.completion.selected_candidate() else {
            return false;
        };

        self.input
            .replace_range(range.start..range.end, &candidate.insert);
        self.close_completions();
        self.ensure_empty_draft_entry();
        self.status = format!("Inserted {}", candidate.insert);
        true
    }

    pub(super) fn accept_completion(&mut self, index: usize) {
        if self.completion.select(index) {
            self.accept_selected_completion();
        }
    }
}

impl CompletionState {
    fn items(&self) -> &[CompletionCandidate] {
        &self.items
    }

    fn is_open(&self) -> bool {
        !self.items.is_empty()
    }

    fn selected_index(&self) -> Option<usize> {
        self.is_open().then_some(self.selected)
    }

    fn clear(&mut self) {
        self.token_range = None;
        self.items.clear();
        self.selected = 0;
    }

    fn select(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }

        self.selected = index;
        true
    }

    fn move_selection(&mut self, direction: CompletionDirection) {
        if self.items.is_empty() {
            return;
        }

        self.selected = match direction {
            CompletionDirection::Previous => self.selected.saturating_sub(1),
            CompletionDirection::Next => (self.selected + 1).min(self.items.len() - 1),
        };
    }

    fn selected_candidate(&self) -> Option<(CompletionToken, CompletionCandidate)> {
        Some((self.token_range?, self.items.get(self.selected)?.clone()))
    }

    fn for_source(source: &str, items: Vec<CompletionItem>) -> Self {
        let Some(token) = completion_token(source) else {
            return Self::default();
        };

        let prefix = &source[token.start..token.end];
        let candidates = items
            .into_iter()
            .filter(|item| completion_matches(item, prefix))
            .take(MAX_COMPLETIONS)
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompletionDirection {
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

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_core::Engine;

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
}
