use super::entry::{Entry, EntryState, EntryView};
use crate::id::{ExpressionId, ExpressionIdGenerator};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug)]
pub(super) struct EntryStore {
    entries: HashMap<ExpressionId, Entry>,
    names: HashMap<String, ExpressionId>,
    id_generator: ExpressionIdGenerator,
}

impl Default for EntryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EntryStore {
    pub(super) fn new() -> Self {
        Self {
            entries: HashMap::new(),
            names: HashMap::new(),
            id_generator: ExpressionIdGenerator::new(),
        }
    }

    pub(super) fn allocate_id(&mut self) -> ExpressionId {
        loop {
            let id = self.id_generator.next();
            if !self.entries.contains_key(&id) {
                return id;
            }
        }
    }

    pub(super) fn resolve_or_create_name(
        &mut self,
        name: String,
    ) -> (ExpressionId, Option<String>) {
        if let Some(id) = self.names.get(&name).copied() {
            return (id, Some(name));
        }

        let id = self.allocate_id();
        self.names.insert(name.clone(), id);
        (id, Some(name))
    }

    pub(super) fn reserve_id(&mut self, id: ExpressionId) {
        self.id_generator.reserve_through(id.value());
    }

    pub(super) fn insert(&mut self, id: ExpressionId, entry: Entry) {
        self.entries.insert(id, entry);
    }

    pub(super) fn reserve_restored_entry(&mut self, id: ExpressionId, name: Option<&str>) {
        self.reserve_id(id);
        if let Some(name) = name {
            self.names.insert(name.to_string(), id);
        }
    }

    pub(super) fn remove(&mut self, id: ExpressionId) -> Option<Entry> {
        let removed = self.entries.remove(&id);
        if removed.is_some() {
            self.names.retain(|_, entry_id| *entry_id != id);
        }
        removed
    }

    pub(super) fn entry(&self, id: ExpressionId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    pub(super) fn entry_mut(&mut self, id: ExpressionId) -> Option<&mut Entry> {
        self.entries.get_mut(&id)
    }

    pub(super) fn name_id(&self, name: &str) -> Option<ExpressionId> {
        self.names.get(name).copied()
    }

    pub(super) fn name_for_id(&self, id: ExpressionId) -> Option<&String> {
        self.entry(id).and_then(|entry| entry.name.as_ref())
    }

    pub(super) fn state(&self, id: ExpressionId) -> Option<&EntryState> {
        self.entry(id).map(|entry| &entry.state)
    }

    pub(super) fn entry_view(&self, id: ExpressionId) -> Option<EntryView> {
        self.entry(id).map(EntryView::from)
    }

    pub(super) fn entries(&self) -> Vec<EntryView> {
        let mut entries = self
            .entries
            .values()
            .map(EntryView::from)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.id);
        entries
    }

    pub(super) fn dependency_entries(
        &self,
    ) -> impl Iterator<Item = (ExpressionId, BTreeSet<ExpressionId>)> + '_ {
        self.entries
            .iter()
            .map(|(id, entry)| (*id, entry.analysis.entry_references.clone()))
    }

    pub(super) fn ids_referencing_constant(&self, name: &str) -> BTreeSet<ExpressionId> {
        self.entries
            .iter()
            .filter_map(|(id, entry)| {
                entry
                    .analysis
                    .constant_references
                    .contains(name)
                    .then_some(*id)
            })
            .collect()
    }

    pub(super) fn ids_referencing_function(&self, name: &str) -> BTreeSet<ExpressionId> {
        self.entries
            .iter()
            .filter_map(|(id, entry)| {
                entry
                    .analysis
                    .function_references
                    .contains(name)
                    .then_some(*id)
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn raw_entry(&self, id: ExpressionId) -> Option<&Entry> {
        self.entry(id)
    }
}
