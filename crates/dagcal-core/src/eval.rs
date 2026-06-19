use crate::ast::{BinaryOp, Expr, Reference, UnaryOp};
use crate::error::EvalError;
use crate::function::FunctionRegistry;

pub(crate) fn eval_expr(
    expr: &Expr,
    functions: &FunctionRegistry,
    resolve: &mut dyn FnMut(&Reference) -> Result<f64, EvalError>,
) -> Result<f64, EvalError> {
    match expr {
        Expr::Number(value) => Ok(*value),
        Expr::Reference(name) => resolve(name),
        Expr::Unary { op, rhs } => {
            let rhs = eval_expr(rhs, functions, resolve)?;
            match op {
                UnaryOp::Plus => Ok(rhs),
                UnaryOp::Minus => Ok(-rhs),
            }
        }
        Expr::Binary { lhs, op, rhs } => {
            let lhs = eval_expr(lhs, functions, resolve)?;
            let rhs = eval_expr(rhs, functions, resolve)?;
            match op {
                BinaryOp::Add => Ok(lhs + rhs),
                BinaryOp::Sub => Ok(lhs - rhs),
                BinaryOp::Mul => Ok(lhs * rhs),
                BinaryOp::Div => {
                    if rhs == 0.0 {
                        Err(EvalError::DivisionByZero)
                    } else {
                        Ok(lhs / rhs)
                    }
                }
                BinaryOp::Rem => {
                    if rhs == 0.0 {
                        Err(EvalError::RemainderByZero)
                    } else {
                        Ok(lhs % rhs)
                    }
                }
                BinaryOp::Pow => finite_operator_result("power", lhs.powf(rhs)),
            }
        }
        Expr::Call { name, args } => {
            let function = functions
                .get(name)
                .ok_or_else(|| EvalError::UnknownFunction(name.clone()))?;
            if !function.signature().accepts(args.len()) {
                return Err(EvalError::ArityMismatch {
                    name: name.clone(),
                    expected: function.signature().clone(),
                    actual: args.len(),
                });
            }

            let mut evaluated_args = Vec::with_capacity(args.len());
            for arg in args {
                evaluated_args.push(eval_expr(arg, functions, resolve)?);
            }
            function.call(&evaluated_args)
        }
    }
}

fn finite_operator_result(name: &str, value: f64) -> Result<f64, EvalError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EvalError::Math(format!(
            "{name} operation produced non-finite result"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DagcalError;
    use crate::function::FunctionSignature;
    use crate::id::ExpressionId;
    use crate::parser::parse_expression;
    use std::collections::HashMap;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }

    fn eval_with_refs(
        source: &str,
        refs: &[(&str, f64)],
        functions: &FunctionRegistry,
    ) -> Result<f64, EvalError> {
        let expr = parse_expression(source).unwrap();
        let refs = refs
            .iter()
            .map(|(name, value)| (parse_test_reference(name), *value))
            .collect::<HashMap<_, _>>();
        let mut resolve = |reference: &Reference| {
            refs.get(reference)
                .copied()
                .ok_or_else(|| EvalError::UnknownReference(reference.display_name()))
        };

        eval_expr(&expr, functions, &mut resolve)
    }

    fn parse_test_reference(input: &str) -> Reference {
        if let Some(digits) = input.strip_prefix('$') {
            return Reference::Id(ExpressionId::new(digits.parse().unwrap()));
        }

        Reference::Name(input.to_string())
    }

    fn eval_standard(source: &str) -> Result<f64, EvalError> {
        let functions = FunctionRegistry::standard();
        eval_with_refs(
            source,
            &[("pi", std::f64::consts::PI), ("e", std::f64::consts::E)],
            &functions,
        )
    }

    #[test]
    fn evaluates_basic_arithmetic() {
        assert_close(eval_standard("1 + 2 * 3").unwrap(), 7.0);
        assert_close(eval_standard("(1 + 2) * 3").unwrap(), 9.0);
        assert_close(eval_standard("10 % 4").unwrap(), 2.0);
        assert_close(eval_standard("2 ^ 3 ^ 2").unwrap(), 512.0);
        assert_close(eval_standard("-2 ^ 2").unwrap(), -4.0);
        assert_close(eval_standard("+5 - -2").unwrap(), 7.0);
    }

    #[test]
    fn evaluates_function_calls_after_arguments() {
        let mut functions = FunctionRegistry::new();
        functions.register("weighted", FunctionSignature::exact(2), |args| {
            Ok(args[0] + args[1] * 10.0)
        });

        assert_close(
            eval_with_refs("weighted(x + 1, y)", &[("x", 2.0), ("y", 4.0)], &functions).unwrap(),
            43.0,
        );
    }

    #[test]
    fn returns_explicit_errors_for_invalid_operations_and_references() {
        assert_eq!(eval_standard("1 / 0"), Err(EvalError::DivisionByZero));
        assert_eq!(eval_standard("1 % 0"), Err(EvalError::RemainderByZero));
        assert!(matches!(
            eval_standard("missing + 1"),
            Err(EvalError::UnknownReference(name)) if name == "missing"
        ));
        assert!(matches!(
            eval_standard("nope(1)"),
            Err(EvalError::UnknownFunction(name)) if name == "nope"
        ));
    }

    #[test]
    fn returns_arity_mismatch_before_evaluating_arguments() {
        let functions = FunctionRegistry::standard();

        assert!(matches!(
            eval_with_refs("sin()", &[], &functions),
            Err(EvalError::ArityMismatch {
                name,
                expected: FunctionSignature::Exact(1),
                actual: 0,
            }) if name == "sin"
        ));
        assert!(matches!(
            eval_with_refs("avg()", &[], &functions),
            Err(EvalError::ArityMismatch {
                name,
                expected: FunctionSignature::Variadic { min: 1 },
                actual: 0,
            }) if name == "avg"
        ));
        assert!(matches!(
            eval_with_refs("sin(missing)", &[], &functions),
            Err(EvalError::UnknownReference(name)) if name == "missing"
        ));
    }

    #[test]
    fn standardizes_non_finite_operator_results() {
        assert!(matches!(
            eval_standard("1e308 ^ 2"),
            Err(EvalError::Math(message))
                if message == "power operation produced non-finite result"
        ));
    }

    #[test]
    fn propagates_parseable_labels_from_result_references() -> Result<(), DagcalError> {
        let functions = FunctionRegistry::standard();

        assert_close(
            eval_with_refs("$1 + $20", &[("$1", 2.0), ("$20", 5.0)], &functions)?,
            7.0,
        );

        Ok(())
    }
}
