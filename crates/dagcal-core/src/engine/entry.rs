use crate::ast::{ExpressionAnalysis, ResolvedExpr};
use crate::error::{DagcalError, EvalError};
use crate::id::ExpressionId;

#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) id: ExpressionId,
    pub(super) name: Option<String>,
    pub(super) source: String,
    pub(super) ast: Option<ResolvedExpr>,
    pub(super) analysis: ExpressionAnalysis,
    pub(super) state: EntryState,
}

impl Entry {
    pub(super) fn from_resolved(
        id: ExpressionId,
        name: Option<String>,
        source: String,
        ast: ResolvedExpr,
    ) -> Self {
        let analysis = ast.analyze();
        Self {
            id,
            name,
            source,
            analysis,
            ast: Some(ast),
            state: EntryState::Error(DagcalError::Eval(EvalError::DependencyError(
                id.to_string(),
            ))),
        }
    }

    pub(super) fn from_parse_error(
        id: ExpressionId,
        name: Option<String>,
        source: String,
        err: DagcalError,
    ) -> Self {
        Self {
            id,
            name,
            source,
            ast: None,
            analysis: ExpressionAnalysis::default(),
            state: EntryState::Error(err),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryView {
    pub id: ExpressionId,
    pub name: Option<String>,
    pub source: String,
    pub state: EntryState,
}

impl From<&Entry> for EntryView {
    fn from(entry: &Entry) -> Self {
        Self {
            id: entry.id,
            name: entry.name.clone(),
            source: entry.source.clone(),
            state: entry.state.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryState {
    Value(f64),
    Error(DagcalError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Execution {
    pub id: Option<ExpressionId>,
    pub state: EntryState,
}
