use crate::error::EvalError;
use std::collections::HashMap;
use std::sync::Arc;

type FunctionBody = dyn Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static;

#[derive(Clone)]
pub struct Function {
    arity: usize,
    body: Arc<FunctionBody>,
}

impl Function {
    pub fn new<F>(arity: usize, body: F) -> Self
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        Self {
            arity,
            body: Arc::new(body),
        }
    }

    pub fn arity(&self) -> usize {
        self.arity
    }

    pub fn call(&self, args: &[f64]) -> Result<f64, EvalError> {
        (self.body)(args)
    }
}

#[derive(Clone, Default)]
pub struct FunctionRegistry {
    functions: HashMap<String, Function>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn standard() -> Self {
        let mut registry = Self::new();
        registry.register("sin", 1, |args| Ok(args[0].sin()));
        registry.register("cos", 1, |args| Ok(args[0].cos()));
        registry.register("tan", 1, |args| Ok(args[0].tan()));
        registry.register("log", 1, |args| Ok(args[0].log10()));
        registry.register("ln", 1, |args| Ok(args[0].ln()));
        registry.register("exp", 1, |args| Ok(args[0].exp()));
        registry
    }

    pub fn register<F>(&mut self, name: impl Into<String>, arity: usize, body: F)
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        self.functions
            .insert(name.into(), Function::new(arity, body));
    }

    pub fn get(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }
}
