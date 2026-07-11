use super::{App, Mode};
use crate::app::formatting::availability_label;
use dagcal_app::{CompletionCandidate, EntryStateFilter, EntryView, formatting};

impl App {
    pub fn entries(&self) -> Vec<EntryView> {
        self.session.entries().to_vec()
    }

    pub fn visible_entries(&self) -> Vec<EntryView> {
        self.session.filtered_entries_iter().cloned().collect()
    }

    pub fn selected(&self) -> usize {
        self.selected_visible_index().unwrap_or(0)
    }

    pub fn selected_visible_index(&self) -> Option<usize> {
        let selected = self.session.selected_id()?;
        self.session
            .filtered_entries_iter()
            .position(|entry| entry.id == selected)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn input(&self) -> &str {
        self.session.input_source()
    }

    pub fn search_query(&self) -> &str {
        self.session.entry_search_query()
    }

    pub fn search_is_open(&self) -> bool {
        self.session.entry_search_is_open()
    }

    pub fn entry_state_filter(&self) -> EntryStateFilter {
        self.session.entry_state_filter()
    }

    pub fn entry_count_status_text(&self) -> String {
        self.session.entry_count_status_text()
    }

    pub fn history_status_text(&self) -> String {
        format!(
            "Undo: {}    Redo: {}",
            availability_label(self.session.can_undo()),
            availability_label(self.session.can_redo())
        )
    }

    pub fn status(&self) -> &str {
        self.session.status()
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn completion_is_open(&self) -> bool {
        self.session.completion_is_open()
    }

    pub fn selected_completion_index(&self) -> Option<usize> {
        self.session.selected_completion_index()
    }

    pub fn completion_candidates(&self) -> &[CompletionCandidate] {
        self.session.completion_candidates()
    }

    pub fn resolved_input(&self) -> String {
        formatting::resolved_source(self.input(), self.session.entries())
    }

    pub fn preview_summary(&self) -> String {
        let source = self.input().trim();
        if source.is_empty() {
            return "Preview: empty".to_string();
        }

        match self.session.preview(source) {
            Ok(value) => format!("Preview: {value}"),
            Err(err) => format!("Preview error: {err}"),
        }
    }

    pub fn selected_detail_text(&self) -> String {
        let Some(id) = self.session.selected_id() else {
            return "Details: select an entry".to_string();
        };

        let Some(entry) = self.session.entry(id) else {
            return "Details: selected entry is not available".to_string();
        };

        let mut detail = self.session.selected_compact_text(id, entry);
        let error = self.session.selected_error_text(entry);
        if error != "Error detail: none" {
            detail.push('\n');
            detail.push_str(&error);
        }
        detail
    }
}
