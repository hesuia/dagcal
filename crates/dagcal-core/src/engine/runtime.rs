use crate::error::EvalError;
use crate::function::{FunctionRegistry, FunctionSignature};
use crate::number::Number;
use std::collections::HashMap;

pub(super) struct RuntimeEnvironment {
    constants: HashMap<String, Number>,
    functions: FunctionRegistry,
}

impl Default for RuntimeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeEnvironment {
    pub(super) fn new() -> Self {
        Self {
            constants: HashMap::from([
                ("e".to_string(), Number::Float(std::f64::consts::E)),
                ("pi".to_string(), Number::Float(std::f64::consts::PI)),
            ]),
            functions: FunctionRegistry::standard(),
        }
    }

    pub(super) fn has_constant(&self, name: &str) -> bool {
        self.constants.contains_key(name)
    }

    pub(super) fn constant(&self, name: &str) -> Option<Number> {
        self.constants.get(name).cloned()
    }

    pub(super) fn constant_names(&self) -> Vec<String> {
        let mut names = self.constants.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    pub(super) fn functions(&self) -> &FunctionRegistry {
        &self.functions
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
}
