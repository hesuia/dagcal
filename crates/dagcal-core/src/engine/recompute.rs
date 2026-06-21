use super::context::EvaluationContext;
use super::entry::EntryState;
use super::store::EntryStore;
use crate::dependency_graph::ReferenceGraph;
use crate::error::{DagcalError, EvalError};
use crate::id::ExpressionId;
use std::collections::BTreeSet;

pub(super) struct Recomputer {
    graph: ReferenceGraph,
}

impl Default for Recomputer {
    fn default() -> Self {
        Self::new()
    }
}

impl Recomputer {
    pub(super) fn new() -> Self {
        Self {
            graph: ReferenceGraph::new(),
        }
    }

    pub(super) fn rebuild_graph(&mut self, store: &EntryStore) {
        self.graph.rebuild(store.dependency_entries());
    }

    pub(super) fn recompute_all(&mut self, store: &mut EntryStore, context: &EvaluationContext) {
        self.recompute_ids(store.ids(), store, context);
    }

    pub(super) fn recompute_affected(
        &mut self,
        id: ExpressionId,
        store: &mut EntryStore,
        context: &EvaluationContext,
    ) {
        let affected = self.collect_affected(id);
        self.recompute_ids(affected, store, context);
    }

    pub(super) fn collect_affected(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.graph.affected_by(id)
    }

    pub(super) fn cycle_report(&self) -> crate::dependency_graph::CycleReport {
        self.graph.cycle_report()
    }

    pub(super) fn recompute_ids(
        &mut self,
        ids: BTreeSet<ExpressionId>,
        store: &mut EntryStore,
        context: &EvaluationContext,
    ) {
        let analysis = self.graph.analyze(&ids);
        let cycle_nodes = analysis.cycle_report.cycle_nodes;

        for id in ids.intersection(&cycle_nodes) {
            if let Some(entry) = store.entry_mut(*id) {
                entry.state = EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(
                    entry.label.to_string(),
                )));
            }
        }

        for current in analysis.evaluation_order {
            if cycle_nodes.contains(&current) {
                continue;
            }

            let state = evaluate_entry(current, store, context);
            if let Some(entry) = store.entry_mut(current) {
                entry.state = state;
            }
        }
    }
}

fn evaluate_entry(id: ExpressionId, store: &EntryStore, context: &EvaluationContext) -> EntryState {
    let Some(entry) = store.entry(id) else {
        return EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(
            store.label_for_id(id),
        )));
    };

    let Some(ast) = &entry.ast else {
        return entry.state.clone();
    };

    match context.eval_expr(ast, store) {
        Ok(value) => EntryState::Value(value),
        Err(err) => EntryState::Error(DagcalError::Eval(err)),
    }
}
