mod ast;
mod dependency_graph;
mod engine;
mod error;
mod eval;
mod function;
mod id;
mod label;
mod parser;

pub use ast::{BinaryOp, Expr, Statement, UnaryOp};
pub use engine::{CycleDiagnostics, Engine, Entry, EntryState, Execution};
pub use error::{DagcalError, EvalError};
pub use function::{Function, FunctionRegistry, FunctionSignature};
pub use id::{ExpressionId, ExpressionIdGenerator};
pub use label::EntryLabel;
pub use parser::{parse_expression, parse_statement};

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }

    #[test]
    fn evaluates_basic_arithmetic() {
        let engine = Engine::new();

        assert_close(engine.eval_once("1 + 2 * 3").unwrap(), 7.0);
        assert_close(engine.eval_once("(1 + 2) * 3").unwrap(), 9.0);
        assert_close(engine.eval_once("10 % 4").unwrap(), 2.0);
        assert_close(engine.eval_once("2 ^ 3 ^ 2").unwrap(), 512.0);
        assert_close(engine.eval_once("-2 ^ 2").unwrap(), -4.0);
    }

    #[test]
    fn evaluates_standard_functions_and_constants() {
        let engine = Engine::new();

        assert_close(engine.eval_once("sin(pi / 2)").unwrap(), 1.0);
        assert_close(engine.eval_once("cos(0)").unwrap(), 1.0);
        assert_close(engine.eval_once("log(100)").unwrap(), 2.0);
        assert_close(engine.eval_once("ln(e)").unwrap(), 1.0);
        assert_close(engine.eval_once("exp(0)").unwrap(), 1.0);
        assert_close(engine.eval_once("sqrt(9)").unwrap(), 3.0);
        assert_close(engine.eval_once("abs(-3.5)").unwrap(), 3.5);
        assert_close(engine.eval_once("floor(1.9)").unwrap(), 1.0);
        assert_close(engine.eval_once("ceil(1.1)").unwrap(), 2.0);
        assert_close(
            engine.eval_once("atan2(1, 1)").unwrap(),
            std::f64::consts::FRAC_PI_4,
        );
        assert_close(engine.eval_once("hypot(3, 4)").unwrap(), 5.0);
        assert_close(engine.eval_once("pow(2, 3)").unwrap(), 8.0);
        assert_close(engine.eval_once("logn(8, 2)").unwrap(), 3.0);
        assert_close(engine.eval_once("sum()").unwrap(), 0.0);
        assert_close(engine.eval_once("sum(1, 2, 3)").unwrap(), 6.0);
        assert_close(engine.eval_once("avg(2, 4, 6)").unwrap(), 4.0);
        assert_close(engine.eval_once("min(3, 1, 2)").unwrap(), 1.0);
        assert_close(engine.eval_once("max(3, 1, 2)").unwrap(), 3.0);
    }

    #[test]
    fn supports_custom_functions() {
        let mut engine = Engine::new();

        engine.register_function("double", FunctionSignature::exact(1), |args| {
            Ok(args[0] * 2.0)
        });

        assert_close(engine.eval_once("double(21)").unwrap(), 42.0);
    }

    #[test]
    fn supports_custom_variadic_functions() {
        let mut engine = Engine::new();

        engine.register_variadic_function("product", 0, |args| Ok(args.iter().product()));

        assert_close(engine.eval_once("product()").unwrap(), 1.0);
        assert_close(engine.eval_once("product(2, 3, 4)").unwrap(), 24.0);
    }

    #[test]
    fn evaluates_decimal_and_scientific_notation() {
        let engine = Engine::new();

        assert_close(engine.eval_once(".5 + 1.").unwrap(), 1.5);
        assert_close(engine.eval_once("1e3 + 2.5E-1").unwrap(), 1000.25);
    }

    #[test]
    fn returns_explicit_eval_errors() {
        let engine = Engine::new();

        assert!(matches!(
            engine.eval_once("1 / 0"),
            Err(DagcalError::Eval(EvalError::DivisionByZero))
        ));
        assert!(matches!(
            engine.eval_once("1 % 0"),
            Err(DagcalError::Eval(EvalError::RemainderByZero))
        ));
        assert!(matches!(
            engine.eval_once("missing + 1"),
            Err(DagcalError::Eval(EvalError::UnknownReference(name))) if name == "missing"
        ));
        assert!(matches!(
            engine.eval_once("nope(1)"),
            Err(DagcalError::Eval(EvalError::UnknownFunction(name))) if name == "nope"
        ));
        assert!(matches!(
            engine.eval_once("sin()"),
            Err(DagcalError::Eval(EvalError::ArityMismatch {
                name,
                expected: FunctionSignature::Exact(1),
                actual: 0,
            })) if name == "sin"
        ));
        assert!(matches!(
            engine.eval_once("avg()"),
            Err(DagcalError::Eval(EvalError::ArityMismatch {
                name,
                expected: FunctionSignature::Variadic { min: 1 },
                actual: 0,
            })) if name == "avg"
        ));
    }

    #[test]
    fn standardizes_non_finite_math_results() {
        let engine = Engine::new();

        assert!(matches!(
            engine.eval_once("sqrt(-1)"),
            Err(DagcalError::Eval(EvalError::Math(message)))
                if message == "function `sqrt` produced non-finite result"
        ));
        assert!(matches!(
            engine.eval_once("ln(-1)"),
            Err(DagcalError::Eval(EvalError::Math(message)))
                if message == "function `ln` produced non-finite result"
        ));
        assert!(matches!(
            engine.eval_once("log(0)"),
            Err(DagcalError::Eval(EvalError::Math(message)))
                if message == "function `log` produced non-finite result"
        ));
        assert!(matches!(
            engine.eval_once("acos(2)"),
            Err(DagcalError::Eval(EvalError::Math(message)))
                if message == "function `acos` produced non-finite result"
        ));
        assert!(matches!(
            engine.eval_once("1e308 ^ 2"),
            Err(DagcalError::Eval(EvalError::Math(message)))
                if message == "power operation produced non-finite result"
        ));
    }

    #[test]
    fn eval_once_can_reference_registered_entries() {
        let mut engine = Engine::new();

        engine.set_entry("subtotal", "40").unwrap();

        assert_close(engine.eval_once("subtotal * 1.1").unwrap(), 44.0);
    }

    #[test]
    fn registering_function_recomputes_existing_entries() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("x", "triple(14)").is_err());
        engine.register_fixed_function("triple", 1, |args| Ok(args[0] * 3.0));

        assert_eq!(engine.get("x"), Some(&EntryState::Value(42.0)));
    }
}
