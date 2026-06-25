use super::entry::EntryState;
use super::store::EntryStore;
use crate::ast::ResolvedExpr;
use crate::error::EvalError;
use crate::eval::eval_expr;
use crate::function::{FunctionRegistry, FunctionSignature};
use crate::id::ExpressionId;
use crate::number::Number;
use std::collections::HashMap;

pub(super) struct EvaluationContext {
    constants: HashMap<String, Number>,
    functions: FunctionRegistry,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EvaluationContext {
    pub(super) fn new() -> Self {
        Self {
            constants: HashMap::from([
                ("e".to_string(), Number::Float(std::f64::consts::E)),
                ("pi".to_string(), Number::Float(std::f64::consts::PI)),
            ]),
            functions: FunctionRegistry::standard(),
        }
    }

    pub(super) fn constants(&self) -> &HashMap<String, Number> {
        &self.constants
    }

    pub(super) fn register_function<F>(
        &mut self,
        name: impl Into<String>,
        signature: FunctionSignature,
        body: F,
    ) where
        F: Fn(&[Number]) -> Result<Number, EvalError> + Send + Sync + 'static,
    {
        self.functions.register(name, signature, body);
    }

    pub(super) fn set_constant(&mut self, name: impl Into<String>, value: impl Into<Number>) {
        self.constants.insert(name.into(), value.into());
    }

    pub(super) fn eval_expr(
        &self,
        ast: &ResolvedExpr,
        store: &EntryStore,
    ) -> Result<Number, EvalError> {
        let mut resolve_entry = |id| resolve_entry_reference(id, store);
        let mut resolve_constant = |name: &str| {
            self.constants
                .get(name)
                .cloned()
                .ok_or_else(|| EvalError::UnknownReference(name.to_string()))
        };
        eval_expr(
            ast,
            &self.functions,
            &mut resolve_entry,
            &mut resolve_constant,
        )
    }
}

fn resolve_entry_reference(id: ExpressionId, store: &EntryStore) -> Result<Number, EvalError> {
    if let Some(entry) = store.entry(id) {
        match &entry.state {
            EntryState::Value(value) => Ok(value.clone()),
            EntryState::Error(_) => Err(EvalError::DependencyError(store.display_name_for_id(id))),
        }
    } else {
        Err(EvalError::UnknownReference(format!("${}", id.value())))
    }
}
