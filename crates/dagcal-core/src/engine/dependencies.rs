use crate::dependency_graph::{CycleReport, GraphAnalysis, ReferenceGraph};
use crate::id::ExpressionId;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub(super) struct DependencyIndex {
    graph: ReferenceGraph,
}

impl DependencyIndex {
    pub(super) fn new() -> Self {
        Self {
            graph: ReferenceGraph::new(),
        }
    }

    pub(super) fn rebuild(
        &mut self,
        entries: impl IntoIterator<Item = (ExpressionId, BTreeSet<ExpressionId>)>,
    ) {
        self.graph.rebuild(entries);
    }

    pub(super) fn affected_by(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.graph.affected_by(id)
    }

    pub(super) fn affected_by_any(
        &self,
        ids: impl IntoIterator<Item = ExpressionId>,
    ) -> BTreeSet<ExpressionId> {
        ids.into_iter()
            .flat_map(|id| self.affected_by(id))
            .collect()
    }

    pub(super) fn cycle_report(&self) -> CycleReport {
        self.graph.cycle_report()
    }

    pub(super) fn analyze(&self, ids: &BTreeSet<ExpressionId>) -> GraphAnalysis {
        self.graph.analyze(ids)
    }
}
