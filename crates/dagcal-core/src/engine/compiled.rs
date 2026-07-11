use crate::ast::{ExpressionAnalysis, ResolvedExpr};
use crate::error::DagcalError;
use crate::id::ExpressionId;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub(super) struct CompiledEntry {
    expr: Option<ResolvedExpr>,
    analysis: ExpressionAnalysis,
    error: Option<DagcalError>,
}

impl CompiledEntry {
    pub(super) fn from_resolved(expr: ResolvedExpr) -> Self {
        let analysis = expr.analyze();
        Self {
            expr: Some(expr),
            analysis,
            error: None,
        }
    }

    pub(super) fn from_error(error: DagcalError) -> Self {
        Self {
            expr: None,
            analysis: ExpressionAnalysis::default(),
            error: Some(error),
        }
    }

    pub(super) fn expr(&self) -> Option<&ResolvedExpr> {
        self.expr.as_ref()
    }

    pub(super) fn error(&self) -> Option<&DagcalError> {
        self.error.as_ref()
    }

    pub(super) fn entry_references(&self) -> &BTreeSet<ExpressionId> {
        &self.analysis.entry_references
    }

    #[cfg(test)]
    pub(super) fn analysis(&self) -> &ExpressionAnalysis {
        &self.analysis
    }
}

#[derive(Debug, Default)]
pub(super) struct CompiledEntryStore {
    entries: HashMap<ExpressionId, CompiledEntry>,
}

impl CompiledEntryStore {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn insert(&mut self, id: ExpressionId, entry: CompiledEntry) {
        self.entries.insert(id, entry);
    }

    pub(super) fn remove(&mut self, id: ExpressionId) -> Option<CompiledEntry> {
        self.entries.remove(&id)
    }

    pub(super) fn get(&self, id: ExpressionId) -> Option<&CompiledEntry> {
        self.entries.get(&id)
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
}
