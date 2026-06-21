use super::entry::{Entry, EntryState, EntryView};
use super::target::EntryTarget;
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

    pub(super) fn resolve_or_create_id(
        &mut self,
        target: EntryTarget,
    ) -> (ExpressionId, Option<String>) {
        match target {
            EntryTarget::Id(id) => {
                self.id_generator.reserve_through(id.value());
                let name = self.entries.get(&id).and_then(|entry| entry.name.clone());
                (id, name)
            }
            EntryTarget::Name(name) => {
                if let Some(id) = self.names.get(&name).copied() {
                    return (id, Some(name));
                }

                let id = self.allocate_id();
                self.names.insert(name.clone(), id);
                (id, Some(name))
            }
        }
    }

    pub(super) fn insert(&mut self, id: ExpressionId, entry: Entry) {
        self.entries.insert(id, entry);
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

    pub(super) fn entry_for_target(&self, target: &EntryTarget) -> Option<&Entry> {
        self.id_for_target(target).and_then(|id| self.entry(id))
    }

    pub(super) fn id_for_target(&self, target: &EntryTarget) -> Option<ExpressionId> {
        match target {
            EntryTarget::Id(id) => Some(*id),
            EntryTarget::Name(name) => self.names.get(name).copied(),
        }
    }

    pub(super) fn name_id(&self, name: &str) -> Option<ExpressionId> {
        self.names.get(name).copied()
    }

    pub(super) fn state(&self, id: ExpressionId) -> Option<&EntryState> {
        self.entry(id).map(|entry| &entry.state)
    }

    pub(super) fn entry_view(&self, id: ExpressionId) -> Option<EntryView> {
        self.entry(id).map(EntryView::from)
    }

    pub(super) fn entry_view_for_target(&self, target: &EntryTarget) -> Option<EntryView> {
        self.entry_for_target(target).map(EntryView::from)
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

    pub(super) fn ids(&self) -> BTreeSet<ExpressionId> {
        self.entries.keys().copied().collect()
    }

    pub(super) fn dependency_entries(
        &self,
    ) -> impl Iterator<Item = (ExpressionId, BTreeSet<ExpressionId>)> + '_ {
        self.entries
            .iter()
            .map(|(id, entry)| (*id, entry.references.clone()))
    }

    pub(super) fn label_for_id(&self, id: ExpressionId) -> String {
        self.entry(id)
            .map(|entry| entry.label.to_string())
            .unwrap_or_else(|| id.to_string())
    }

    #[cfg(test)]
    pub(super) fn raw_entry(&self, id: ExpressionId) -> Option<&Entry> {
        self.entry(id)
    }
}
