use crate::id::{ExpressionId, ExpressionIdGenerator};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub(super) struct EntryRecord {
    pub(super) id: ExpressionId,
    pub(super) name: Option<String>,
    pub(super) source: String,
}

impl EntryRecord {
    pub(super) fn new(id: ExpressionId, name: Option<String>, source: String) -> Self {
        Self { id, name, source }
    }
}

#[derive(Debug)]
pub(super) struct EntryRepository {
    records: BTreeMap<ExpressionId, EntryRecord>,
    names: HashMap<String, ExpressionId>,
    id_generator: ExpressionIdGenerator,
}

impl Default for EntryRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl EntryRepository {
    pub(super) fn new() -> Self {
        Self {
            records: BTreeMap::new(),
            names: HashMap::new(),
            id_generator: ExpressionIdGenerator::new(),
        }
    }

    pub(super) fn allocate_id(&mut self) -> ExpressionId {
        loop {
            let id = self.id_generator.next();
            if !self.records.contains_key(&id) {
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

    pub(super) fn reserve_restored_entry(&mut self, id: ExpressionId, name: Option<&str>) {
        self.reserve_id(id);
        if let Some(name) = name {
            self.names.insert(name.to_string(), id);
        }
    }

    pub(super) fn upsert(&mut self, record: EntryRecord) {
        self.names.retain(|_, entry_id| *entry_id != record.id);
        if let Some(name) = &record.name {
            self.names.insert(name.clone(), record.id);
        }
        self.records.insert(record.id, record);
    }

    pub(super) fn remove(&mut self, id: ExpressionId) -> Option<EntryRecord> {
        let removed = self.records.remove(&id);
        if removed.is_some() {
            self.names.retain(|_, entry_id| *entry_id != id);
        }
        removed
    }

    pub(super) fn record(&self, id: ExpressionId) -> Option<&EntryRecord> {
        self.records.get(&id)
    }

    pub(super) fn name_id(&self, name: &str) -> Option<ExpressionId> {
        self.names.get(name).copied()
    }

    pub(super) fn name_for_id(&self, id: ExpressionId) -> Option<&String> {
        self.record(id).and_then(|record| record.name.as_ref())
    }

    pub(super) fn records(&self) -> impl DoubleEndedIterator<Item = &EntryRecord> {
        self.records.values()
    }

    pub(super) fn ids(&self) -> impl Iterator<Item = ExpressionId> + '_ {
        self.records.keys().copied()
    }

    pub(super) fn sorted_ids(&self) -> Vec<ExpressionId> {
        self.ids().collect()
    }

    pub(super) fn len(&self) -> usize {
        self.records.len()
    }

    pub(super) fn id_at_index(&self, index: usize) -> Option<ExpressionId> {
        self.records.keys().nth(index).copied()
    }

    pub(super) fn named_entries(&self) -> Vec<(String, ExpressionId)> {
        let mut names = self
            .names
            .iter()
            .map(|(name, id)| (name.clone(), *id))
            .collect::<Vec<_>>();
        names.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        names
    }
}
