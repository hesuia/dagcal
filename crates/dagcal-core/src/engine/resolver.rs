use super::symbol::{ResolvedSymbol, SymbolResolver};
use crate::ast::{ParsedExpr, ParsedReference, ResolvedExpr};
use crate::error::{EvalError, ReferenceTarget};

pub(super) fn resolve_expr<R>(resolver: &R, expr: ParsedExpr) -> Result<ResolvedExpr, EvalError>
where
    R: SymbolResolver,
{
    match expr {
        ParsedExpr::Number(value) => Ok(ResolvedExpr::Number(value)),
        ParsedExpr::Reference(reference) => resolve_reference(resolver, reference),
        ParsedExpr::Unary { op, rhs } => Ok(ResolvedExpr::Unary {
            op,
            rhs: Box::new(resolve_expr(resolver, *rhs)?),
        }),
        ParsedExpr::Binary { lhs, op, rhs } => Ok(ResolvedExpr::Binary {
            lhs: Box::new(resolve_expr(resolver, *lhs)?),
            op,
            rhs: Box::new(resolve_expr(resolver, *rhs)?),
        }),
        ParsedExpr::Call { name, args } => Ok(ResolvedExpr::Call {
            name,
            args: args
                .into_iter()
                .map(|arg| resolve_expr(resolver, arg))
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn resolve_reference<R>(resolver: &R, reference: ParsedReference) -> Result<ResolvedExpr, EvalError>
where
    R: SymbolResolver,
{
    match reference {
        ParsedReference::Id(id) => Ok(ResolvedExpr::EntryReference(id)),
        ParsedReference::Name(name) => match resolver.resolve_name(&name) {
            Some(ResolvedSymbol::Entry(id)) => Ok(ResolvedExpr::EntryReference(id)),
            Some(ResolvedSymbol::Constant(name)) => Ok(ResolvedExpr::Constant(name)),
            None => Err(EvalError::UnknownReference(ReferenceTarget::Name(name))),
        },
    }
}
