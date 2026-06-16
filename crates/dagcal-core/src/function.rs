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
        registry.register_unary("abs", f64::abs);
        registry.register_unary("sqrt", f64::sqrt);
        registry.register_unary("cbrt", f64::cbrt);
        registry.register_unary("floor", f64::floor);
        registry.register_unary("ceil", f64::ceil);
        registry.register_unary("round", f64::round);
        registry.register_unary("trunc", f64::trunc);
        registry.register_unary("fract", f64::fract);
        registry.register_unary("signum", f64::signum);
        registry.register_unary("recip", f64::recip);

        registry.register_unary("sin", f64::sin);
        registry.register_unary("cos", f64::cos);
        registry.register_unary("tan", f64::tan);
        registry.register_unary("asin", f64::asin);
        registry.register_unary("acos", f64::acos);
        registry.register_unary("atan", f64::atan);
        registry.register_unary("sinh", f64::sinh);
        registry.register_unary("cosh", f64::cosh);
        registry.register_unary("tanh", f64::tanh);
        registry.register_unary("asinh", f64::asinh);
        registry.register_unary("acosh", f64::acosh);
        registry.register_unary("atanh", f64::atanh);

        registry.register_unary("exp", f64::exp);
        registry.register_unary("exp2", f64::exp2);
        registry.register_unary("ln", f64::ln);
        registry.register_unary("log", f64::log10);
        registry.register_unary("log2", f64::log2);

        registry.register_unary("to_radians", f64::to_radians);
        registry.register_unary("to_degrees", f64::to_degrees);

        registry.register_binary("atan2", f64::atan2);
        registry.register_binary("hypot", f64::hypot);
        registry.register_binary("pow", f64::powf);
        registry.register_binary("logn", f64::log);
        registry.register_binary("copysign", f64::copysign);
        registry.register_binary("max", f64::max);
        registry.register_binary("min", f64::min);
        registry
    }

    pub fn register<F>(&mut self, name: impl Into<String>, arity: usize, body: F)
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        self.functions
            .insert(name.into(), Function::new(arity, body));
    }

    fn register_unary(&mut self, name: impl Into<String>, body: fn(f64) -> f64) {
        self.register(name, 1, move |args| Ok(body(args[0])));
    }

    fn register_binary(&mut self, name: impl Into<String>, body: fn(f64, f64) -> f64) {
        self.register(name, 2, move |args| Ok(body(args[0], args[1])));
    }

    pub fn get(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }
}
