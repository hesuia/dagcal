use super::store::EntryStore;
use crate::ast::{ParsedExpr, ParsedReference, ResolvedExpr};
use crate::error::EvalError;
use std::collections::HashMap;

pub(super) struct Resolver<'a> {
    store: &'a EntryStore,
    constants: &'a HashMap<String, f64>,
}

impl<'a> Resolver<'a> {
    pub(super) fn new(store: &'a EntryStore, constants: &'a HashMap<String, f64>) -> Self {
        Self { store, constants }
    }

    pub(super) fn resolve_expr(&self, expr: ParsedExpr) -> Result<ResolvedExpr, EvalError> {
        match expr {
            ParsedExpr::Number(value) => Ok(ResolvedExpr::Number(value)),
            ParsedExpr::Reference(reference) => self.resolve_reference(reference),
            ParsedExpr::Unary { op, rhs } => Ok(ResolvedExpr::Unary {
                op,
                rhs: Box::new(self.resolve_expr(*rhs)?),
            }),
            ParsedExpr::Binary { lhs, op, rhs } => Ok(ResolvedExpr::Binary {
                lhs: Box::new(self.resolve_expr(*lhs)?),
                op,
                rhs: Box::new(self.resolve_expr(*rhs)?),
            }),
            ParsedExpr::Call { name, args } => Ok(ResolvedExpr::Call {
                name,
                args: args
                    .into_iter()
                    .map(|arg| self.resolve_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
        }
    }

    fn resolve_reference(&self, reference: ParsedReference) -> Result<ResolvedExpr, EvalError> {
        match reference {
            ParsedReference::Id(id) => Ok(ResolvedExpr::EntryReference(id)),
            ParsedReference::Name(name) => {
                if let Some(id) = self.store.name_id(&name) {
                    Ok(ResolvedExpr::EntryReference(id))
                } else if self.constants.contains_key(&name) {
                    Ok(ResolvedExpr::Constant(name))
                } else {
                    Err(EvalError::UnknownReference(name))
                }
            }
        }
    }
}
