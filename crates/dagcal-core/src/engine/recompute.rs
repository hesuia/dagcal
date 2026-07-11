use super::compiled::CompiledEntryStore;
use super::entry::EntryState;
use super::results::ResultCache;
use super::runtime::RuntimeEnvironment;
use crate::dependency_graph::ReferenceGraph;
use crate::error::{DagcalError, EvalError, ReferenceTarget};
use crate::eval::eval_expr;
use crate::id::ExpressionId;
use crate::number::Number;
use std::collections::BTreeSet;

pub(super) struct RecomputePlanner<'a> {
    dependencies: &'a ReferenceGraph,
}

impl<'a> RecomputePlanner<'a> {
    pub(super) fn new(dependencies: &'a ReferenceGraph) -> Self {
        Self { dependencies }
    }

    pub(super) fn affected_by(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.dependencies.affected_by(id)
    }

    pub(super) fn affected_by_any(
        &self,
        ids: impl IntoIterator<Item = ExpressionId>,
    ) -> BTreeSet<ExpressionId> {
        ids.into_iter()
            .flat_map(|id| self.dependencies.affected_by(id))
            .collect()
    }
}

pub(super) struct EvaluationRunner<'a> {
    dependencies: &'a ReferenceGraph,
    compiled: &'a CompiledEntryStore,
    runtime: &'a RuntimeEnvironment,
}

impl<'a> EvaluationRunner<'a> {
    pub(super) fn new(
        dependencies: &'a ReferenceGraph,
        compiled: &'a CompiledEntryStore,
        runtime: &'a RuntimeEnvironment,
    ) -> Self {
        Self {
            dependencies,
            compiled,
            runtime,
        }
    }

    pub(super) fn recompute_ids(&self, ids: BTreeSet<ExpressionId>, results: &mut ResultCache) {
        let analysis = self.dependencies.analyze(&ids);
        let cycle_nodes = analysis.cycle_report.cycle_nodes;

        for id in ids.intersection(&cycle_nodes) {
            results.set(
                *id,
                EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(*id))),
            );
        }

        for current in analysis.evaluation_order {
            if cycle_nodes.contains(&current) {
                continue;
            }
            if self.compiled.get(current).is_none() {
                results.remove(current);
                continue;
            }

            results.set(current, self.evaluate_entry(current, results));
        }
    }

    fn evaluate_entry(&self, id: ExpressionId, results: &ResultCache) -> EntryState {
        let Some(compiled) = self.compiled.get(id) else {
            return EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(
                ReferenceTarget::Id(id),
            )));
        };

        if let Some(error) = compiled.error() {
            return EntryState::Error(error.clone());
        }

        let Some(expr) = compiled.expr() else {
            return EntryState::Error(DagcalError::Eval(EvalError::DependencyError(id)));
        };

        let mut resolve_entry = |id| resolve_entry_reference(id, results);
        let mut resolve_constant = |name: &str| {
            self.runtime
                .constant(name)
                .ok_or_else(|| EvalError::UnknownReference(ReferenceTarget::Name(name.to_string())))
        };

        match eval_expr(
            expr,
            self.runtime.functions(),
            &mut resolve_entry,
            &mut resolve_constant,
        ) {
            Ok(value) => EntryState::Value(value),
            Err(err) => EntryState::Error(DagcalError::Eval(err)),
        }
    }
}

pub(super) fn eval_once(
    expr: &crate::ast::ResolvedExpr,
    runtime: &RuntimeEnvironment,
    results: &ResultCache,
) -> Result<Number, EvalError> {
    let mut resolve_entry = |id| resolve_entry_reference(id, results);
    let mut resolve_constant = |name: &str| {
        runtime
            .constant(name)
            .ok_or_else(|| EvalError::UnknownReference(ReferenceTarget::Name(name.to_string())))
    };

    eval_expr(
        expr,
        runtime.functions(),
        &mut resolve_entry,
        &mut resolve_constant,
    )
}

fn resolve_entry_reference(id: ExpressionId, results: &ResultCache) -> Result<Number, EvalError> {
    if let Some(state) = results.get(id) {
        match state {
            EntryState::Value(value) => Ok(value.clone()),
            EntryState::Error(_) => Err(EvalError::DependencyError(id)),
        }
    } else {
        Err(EvalError::UnknownReference(ReferenceTarget::Id(id)))
    }
}
