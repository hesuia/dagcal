use crate::formatting::needs_space_before_reference;

#[derive(Debug, Default)]
pub struct Draft {
    source: String,
    loaded_selection: Option<String>,
}

impl Draft {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn set(&mut self, source: String) {
        self.source = source;
        self.loaded_selection = None;
    }

    pub fn load_selection(&mut self, source: String) {
        self.loaded_selection = Some(source.clone());
        self.source = source;
    }

    pub fn clear(&mut self) {
        self.source.clear();
        self.loaded_selection = None;
    }

    pub fn is_loaded_selection(&self) -> bool {
        self.loaded_selection
            .as_deref()
            .is_some_and(|source| source == self.source)
    }

    pub fn insert_token(&mut self, token: &str) {
        if needs_space_before_reference(&self.source) {
            self.source.push(' ');
        }
        self.source.push_str(token);
        self.source.push(' ');
        self.loaded_selection = None;
    }

    pub fn replace_range(&mut self, range: std::ops::Range<usize>, replacement: &str) {
        self.source.replace_range(range, replacement);
        self.loaded_selection = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_result_reference_without_replacing_saved_source() {
        let mut draft = Draft::default();

        draft.set("1 +".to_string());
        draft.insert_token("$1");

        assert_eq!(draft.source(), "1 +$1 ");
    }
}
