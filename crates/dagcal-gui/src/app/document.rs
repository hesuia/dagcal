use super::GuiApp;
use std::path::Path;

impl GuiApp {
    pub(crate) fn is_dirty(&self) -> bool {
        self.session.is_dirty_against(&self.saved_snapshot)
    }

    pub(crate) fn main_title(&self) -> String {
        let dirty = if self.is_dirty() { "* " } else { "" };
        format!("{dirty}dagcal - {}", self.document_name())
    }

    pub(crate) fn file_status_text(&self) -> String {
        let state = if self.is_dirty() {
            "Unsaved changes"
        } else {
            "Saved"
        };

        format!("File: {}    {state}", self.document_name())
    }

    pub(crate) fn history_status_text(&self) -> String {
        format!(
            "Undo: {}    Redo: {}",
            availability_label(self.session.can_undo()),
            availability_label(self.session.can_redo())
        )
    }

    fn document_name(&self) -> String {
        self.current_path
            .as_deref()
            .map(path_label)
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

pub(super) fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn availability_label(available: bool) -> &'static str {
    if available { "yes" } else { "no" }
}
