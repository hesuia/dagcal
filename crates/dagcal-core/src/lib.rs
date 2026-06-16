mod ast;
mod engine;
mod error;
mod eval;
mod function;
mod parser;

pub use ast::{BinaryOp, Expr, UnaryOp};
pub use engine::{CycleDiagnostics, Engine, Entry, EntryState};
pub use error::{DagcalError, EvalError};
pub use function::{Function, FunctionRegistry};
pub use parser::parse_expression;

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
    }

    #[test]
    fn supports_custom_functions() {
        let mut engine = Engine::new();

        engine.register_function("double", 1, |args| Ok(args[0] * 2.0));

        assert_close(engine.eval_once("double(21)").unwrap(), 42.0);
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
                expected: 1,
                actual: 0,
            })) if name == "sin"
        ));
    }

    #[test]
    fn eval_once_can_reference_registered_entries() {
        let mut engine = Engine::new();

        engine.set_expr("subtotal", "40").unwrap();

        assert_close(engine.eval_once("subtotal * 1.1").unwrap(), 44.0);
    }

    #[test]
    fn registering_function_recomputes_existing_entries() {
        let mut engine = Engine::new();

        assert!(engine.set_expr("x", "triple(14)").is_err());
        engine.register_function("triple", 1, |args| Ok(args[0] * 3.0));

        assert_eq!(engine.get("x"), Some(&EntryState::Value(42.0)));
    }
}
