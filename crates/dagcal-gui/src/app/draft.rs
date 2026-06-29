use crate::formatting::needs_space_before_reference;

#[derive(Debug, Default)]
pub(crate) struct Draft {
    source: String,
    loaded_selection: Option<String>,
}

impl Draft {
    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(super) fn set(&mut self, source: String) {
        self.source = source;
        self.loaded_selection = None;
    }

    pub(super) fn load_selection(&mut self, source: String) {
        self.loaded_selection = Some(source.clone());
        self.source = source;
    }

    pub(super) fn clear(&mut self) {
        self.source.clear();
        self.loaded_selection = None;
    }

    pub(super) fn is_loaded_selection(&self) -> bool {
        self.loaded_selection
            .as_deref()
            .is_some_and(|source| source == self.source)
    }

    pub(super) fn insert_token(&mut self, token: &str) {
        if needs_space_before_reference(&self.source) {
            self.source.push(' ');
        }
        self.source.push_str(token);
        self.source.push(' ');
        self.loaded_selection = None;
    }
}
