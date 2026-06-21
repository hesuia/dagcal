use super::store::EntryStore;
use crate::ast::{ParsedExpr, ParsedReference, ResolvedExpr};
use crate::error::EvalError;
use std::collections::HashMap;

pub(super) fn resolve_expr(
    store: &EntryStore,
    constants: &HashMap<String, f64>,
    expr: ParsedExpr,
) -> Result<ResolvedExpr, EvalError> {
    match expr {
        ParsedExpr::Number(value) => Ok(ResolvedExpr::Number(value)),
        ParsedExpr::Reference(reference) => resolve_reference(store, constants, reference),
        ParsedExpr::Unary { op, rhs } => Ok(ResolvedExpr::Unary {
            op,
            rhs: Box::new(resolve_expr(store, constants, *rhs)?),
        }),
        ParsedExpr::Binary { lhs, op, rhs } => Ok(ResolvedExpr::Binary {
            lhs: Box::new(resolve_expr(store, constants, *lhs)?),
            op,
            rhs: Box::new(resolve_expr(store, constants, *rhs)?),
        }),
        ParsedExpr::Call { name, args } => Ok(ResolvedExpr::Call {
            name,
            args: args
                .into_iter()
                .map(|arg| resolve_expr(store, constants, arg))
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn resolve_reference(
    store: &EntryStore,
    constants: &HashMap<String, f64>,
    reference: ParsedReference,
) -> Result<ResolvedExpr, EvalError> {
    match reference {
        ParsedReference::Id(id) => Ok(ResolvedExpr::EntryReference(id)),
        ParsedReference::Name(name) => {
            if let Some(id) = store.name_id(&name) {
                Ok(ResolvedExpr::EntryReference(id))
            } else if constants.contains_key(&name) {
                Ok(ResolvedExpr::Constant(name))
            } else {
                Err(EvalError::UnknownReference(name))
            }
        }
    }
}
