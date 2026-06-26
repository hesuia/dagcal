use super::entry::EntryState;
use crate::id::ExpressionId;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(super) struct ResultCache {
    states: HashMap<ExpressionId, EntryState>,
}

impl ResultCache {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn set(&mut self, id: ExpressionId, state: EntryState) {
        self.states.insert(id, state);
    }

    pub(super) fn get(&self, id: ExpressionId) -> Option<&EntryState> {
        self.states.get(&id)
    }

    pub(super) fn remove(&mut self, id: ExpressionId) -> Option<EntryState> {
        self.states.remove(&id)
    }
}
