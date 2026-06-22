use crate::error::EvalError;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

type FunctionBody = dyn Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static;

/// Arity contract for a callable function.
///
/// Function signatures are used by the evaluator before invoking a function
/// body. They are also exposed in [`EvalError::ArityMismatch`] so callers can
/// render precise diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionSignature {
    /// Function must receive exactly this many arguments.
    Exact(usize),
    /// Function accepts at least `min` arguments.
    Variadic { min: usize },
}

impl FunctionSignature {
    /// Creates a signature that accepts exactly `arity` arguments.
    pub fn exact(arity: usize) -> Self {
        Self::Exact(arity)
    }

    /// Creates a signature that accepts `min` or more arguments.
    pub fn variadic(min: usize) -> Self {
        Self::Variadic { min }
    }

    /// Returns whether this signature accepts `actual` arguments.
    pub fn accepts(&self, actual: usize) -> bool {
        match self {
            Self::Exact(expected) => *expected == actual,
            Self::Variadic { min } => actual >= *min,
        }
    }
}

impl fmt::Display for FunctionSignature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(arity) => write!(formatter, "{arity} argument(s)"),
            Self::Variadic { min } => write!(formatter, "at least {min} argument(s)"),
        }
    }
}

#[derive(Clone)]
pub struct Function {
    signature: FunctionSignature,
    body: Arc<FunctionBody>,
}

impl Function {
    pub fn new<F>(signature: FunctionSignature, body: F) -> Self
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        Self {
            signature,
            body: Arc::new(body),
        }
    }

    pub fn signature(&self) -> &FunctionSignature {
        &self.signature
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

        registry.register_variadic("sum", 0, |args| Ok(args.iter().sum()));
        registry.register_variadic("avg", 1, |args| {
            Ok(args.iter().sum::<f64>() / args.len() as f64)
        });
        registry.register_variadic("max", 1, |args| {
            Ok(args
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, |max, value| max.max(value)))
        });
        registry.register_variadic("min", 1, |args| {
            Ok(args
                .iter()
                .copied()
                .fold(f64::INFINITY, |min, value| min.min(value)))
        });
        registry
    }

    pub fn register<F>(&mut self, name: impl Into<String>, signature: FunctionSignature, body: F)
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        self.functions
            .insert(name.into(), Function::new(signature, body));
    }

    fn register_unary(&mut self, name: impl Into<String>, body: fn(f64) -> f64) {
        let name = name.into();
        let function_name = name.clone();
        self.register(name, FunctionSignature::exact(1), move |args| {
            finite_function_result(&function_name, body(args[0]))
        });
    }

    fn register_binary(&mut self, name: impl Into<String>, body: fn(f64, f64) -> f64) {
        let name = name.into();
        let function_name = name.clone();
        self.register(name, FunctionSignature::exact(2), move |args| {
            finite_function_result(&function_name, body(args[0], args[1]))
        });
    }

    fn register_variadic<F>(&mut self, name: impl Into<String>, min: usize, body: F)
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        let name = name.into();
        let function_name = name.clone();
        self.register(name, FunctionSignature::variadic(min), move |args| {
            finite_function_result(&function_name, body(args)?)
        });
    }

    pub fn get(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }
}

fn finite_function_result(name: &str, value: f64) -> Result<f64, EvalError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EvalError::Math(format!(
            "function `{name}` produced non-finite result"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }

    fn call_standard(name: &str, args: &[f64]) -> Result<f64, EvalError> {
        FunctionRegistry::standard().get(name).unwrap().call(args)
    }

    #[test]
    fn exact_signatures_accept_only_their_arity() {
        let signature = FunctionSignature::exact(2);

        assert!(!signature.accepts(1));
        assert!(signature.accepts(2));
        assert!(!signature.accepts(3));
        assert_eq!(signature.to_string(), "2 argument(s)");
    }

    #[test]
    fn variadic_signatures_accept_minimum_or_more() {
        let signature = FunctionSignature::variadic(1);

        assert!(!signature.accepts(0));
        assert!(signature.accepts(1));
        assert!(signature.accepts(3));
        assert_eq!(signature.to_string(), "at least 1 argument(s)");
    }

    #[test]
    fn registered_functions_store_signature_and_body() {
        let mut registry = FunctionRegistry::new();

        registry.register("double", FunctionSignature::exact(1), |args| {
            Ok(args[0] * 2.0)
        });

        let function = registry.get("double").unwrap();
        assert_eq!(function.signature(), &FunctionSignature::Exact(1));
        assert_eq!(function.call(&[21.0]), Ok(42.0));
        assert!(registry.get("missing").is_none());
    }

    #[test]
    fn standard_unary_functions_return_finite_results() {
        assert_close(
            call_standard("sin", &[std::f64::consts::FRAC_PI_2]).unwrap(),
            1.0,
        );
        assert_close(call_standard("cos", &[0.0]).unwrap(), 1.0);
        assert_close(call_standard("ln", &[std::f64::consts::E]).unwrap(), 1.0);
        assert_close(call_standard("log", &[100.0]).unwrap(), 2.0);
        assert_close(call_standard("sqrt", &[9.0]).unwrap(), 3.0);
        assert_close(call_standard("abs", &[-3.5]).unwrap(), 3.5);
        assert_close(call_standard("floor", &[1.9]).unwrap(), 1.0);
        assert_close(call_standard("ceil", &[1.1]).unwrap(), 2.0);
    }

    #[test]
    fn standard_binary_functions_return_finite_results() {
        assert_close(
            call_standard("atan2", &[1.0, 1.0]).unwrap(),
            std::f64::consts::FRAC_PI_4,
        );
        assert_close(call_standard("hypot", &[3.0, 4.0]).unwrap(), 5.0);
        assert_close(call_standard("pow", &[2.0, 3.0]).unwrap(), 8.0);
        assert_close(call_standard("logn", &[8.0, 2.0]).unwrap(), 3.0);
    }

    #[test]
    fn standard_variadic_functions_handle_empty_and_non_empty_inputs() {
        assert_close(call_standard("sum", &[]).unwrap(), 0.0);
        assert_close(call_standard("sum", &[1.0, 2.0, 3.0]).unwrap(), 6.0);
        assert_close(call_standard("avg", &[2.0, 4.0, 6.0]).unwrap(), 4.0);
        assert_close(call_standard("min", &[3.0, 1.0, 2.0]).unwrap(), 1.0);
        assert_close(call_standard("max", &[3.0, 1.0, 2.0]).unwrap(), 3.0);
    }

    #[test]
    fn standard_functions_reject_non_finite_results_consistently() {
        assert!(matches!(
            call_standard("sqrt", &[-1.0]),
            Err(EvalError::Math(message))
                if message == "function `sqrt` produced non-finite result"
        ));
        assert!(matches!(
            call_standard("ln", &[-1.0]),
            Err(EvalError::Math(message))
                if message == "function `ln` produced non-finite result"
        ));
        assert!(matches!(
            call_standard("log", &[0.0]),
            Err(EvalError::Math(message))
                if message == "function `log` produced non-finite result"
        ));
        assert!(matches!(
            call_standard("acos", &[2.0]),
            Err(EvalError::Math(message))
                if message == "function `acos` produced non-finite result"
        ));
    }
}
