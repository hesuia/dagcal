use crate::ast::{BinaryOp, ResolvedExpr, UnaryOp};
use crate::error::EvalError;
use crate::function::FunctionRegistry;
use crate::id::ExpressionId;
use crate::number::Number;

pub(crate) fn eval_expr(
    expr: &ResolvedExpr,
    functions: &FunctionRegistry,
    resolve_entry: &mut dyn FnMut(ExpressionId) -> Result<Number, EvalError>,
    resolve_constant: &mut dyn FnMut(&str) -> Result<Number, EvalError>,
) -> Result<Number, EvalError> {
    match expr {
        ResolvedExpr::Number(value) => Ok(value.clone()),
        ResolvedExpr::EntryReference(id) => finite_value(resolve_entry(*id)?, || {
            format!("reference `${}` produced non-finite result", id.value())
        }),
        ResolvedExpr::Constant(name) => finite_value(resolve_constant(name)?, || {
            format!("constant `{name}` produced non-finite result")
        }),
        ResolvedExpr::Unary { op, rhs } => {
            let rhs = eval_expr(rhs, functions, resolve_entry, resolve_constant)?;
            match op {
                UnaryOp::Plus => finite_value(rhs, || {
                    "unary plus operation produced non-finite result".to_string()
                }),
                UnaryOp::Minus => finite_value(-rhs, || {
                    "unary minus operation produced non-finite result".to_string()
                }),
            }
        }
        ResolvedExpr::Binary { lhs, op, rhs } => {
            let lhs = eval_expr(lhs, functions, resolve_entry, resolve_constant)?;
            let rhs = eval_expr(rhs, functions, resolve_entry, resolve_constant)?;
            match op {
                BinaryOp::Add => finite_value(lhs + rhs, || {
                    "addition operation produced non-finite result".to_string()
                }),
                BinaryOp::Sub => finite_value(lhs - rhs, || {
                    "subtraction operation produced non-finite result".to_string()
                }),
                BinaryOp::Mul => finite_value(lhs * rhs, || {
                    "multiplication operation produced non-finite result".to_string()
                }),
                BinaryOp::Div => {
                    if rhs.is_zero() {
                        Err(EvalError::DivisionByZero)
                    } else {
                        finite_value(lhs / rhs, || {
                            "division operation produced non-finite result".to_string()
                        })
                    }
                }
                BinaryOp::Rem => {
                    if rhs.is_zero() {
                        Err(EvalError::RemainderByZero)
                    } else {
                        finite_value(lhs % rhs, || {
                            "remainder operation produced non-finite result".to_string()
                        })
                    }
                }
                BinaryOp::Pow => lhs.pow(rhs),
            }
        }
        ResolvedExpr::Call { name, args } => {
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
                evaluated_args.push(eval_expr(arg, functions, resolve_entry, resolve_constant)?);
            }
            finite_value(function.call(&evaluated_args)?, || {
                format!("function `{name}` produced non-finite result")
            })
        }
    }
}

fn finite_value<F>(value: Number, message: F) -> Result<Number, EvalError>
where
    F: FnOnce() -> String,
{
    value.finite(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{DagcalError, ReferenceTarget};
    use crate::function::FunctionSignature;
    use std::collections::HashMap;

    fn assert_close(actual: Number, expected: f64) {
        assert!(
            (actual.to_f64() - expected).abs() < 1e-12,
            "{actual} != {expected}"
        );
    }

    fn eval_with_refs(
        expr: &ResolvedExpr,
        refs: &[(ExpressionId, f64)],
        constants: &[(&str, f64)],
        functions: &FunctionRegistry,
    ) -> Result<Number, EvalError> {
        let refs = refs
            .iter()
            .map(|(id, value)| (*id, Number::from(*value)))
            .collect::<HashMap<_, _>>();
        let constants = constants
            .iter()
            .map(|(name, value)| ((*name).to_string(), Number::from(*value)))
            .collect::<HashMap<_, _>>();
        let mut resolve_entry = |id: ExpressionId| {
            refs.get(&id)
                .cloned()
                .ok_or_else(|| EvalError::UnknownReference(ReferenceTarget::Id(id)))
        };
        let mut resolve_constant = |name: &str| {
            constants
                .get(name)
                .cloned()
                .ok_or_else(|| EvalError::UnknownReference(ReferenceTarget::Name(name.to_string())))
        };

        eval_expr(expr, functions, &mut resolve_entry, &mut resolve_constant)
    }

    fn eval_standard(source: &str) -> Result<Number, EvalError> {
        let functions = FunctionRegistry::standard();
        let expr = test_expr(source);
        eval_with_refs(
            &expr,
            &[],
            &[("pi", std::f64::consts::PI), ("e", std::f64::consts::E)],
            &functions,
        )
    }

    fn test_expr(source: &str) -> ResolvedExpr {
        match source {
            "1 + 2 * 3" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1.0))),
                op: BinaryOp::Add,
                rhs: Box::new(ResolvedExpr::Binary {
                    lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
                    op: BinaryOp::Mul,
                    rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(3.0))),
                }),
            },
            "(1 + 2) * 3" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Binary {
                    lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1.0))),
                    op: BinaryOp::Add,
                    rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
                }),
                op: BinaryOp::Mul,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(3.0))),
            },
            "10 % 4" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(10.0))),
                op: BinaryOp::Rem,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(4.0))),
            },
            "2 ^ 3 ^ 2" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
                op: BinaryOp::Pow,
                rhs: Box::new(ResolvedExpr::Binary {
                    lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(3.0))),
                    op: BinaryOp::Pow,
                    rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
                }),
            },
            "-2 ^ 2" => ResolvedExpr::Unary {
                op: UnaryOp::Minus,
                rhs: Box::new(ResolvedExpr::Binary {
                    lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
                    op: BinaryOp::Pow,
                    rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
                }),
            },
            "+5 - -2" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Unary {
                    op: UnaryOp::Plus,
                    rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(5.0))),
                }),
                op: BinaryOp::Sub,
                rhs: Box::new(ResolvedExpr::Unary {
                    op: UnaryOp::Minus,
                    rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
                }),
            },
            "1 / 0" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1.0))),
                op: BinaryOp::Div,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(0.0))),
            },
            "1 % 0" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1.0))),
                op: BinaryOp::Rem,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(0.0))),
            },
            "missing + 1" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Constant("missing".to_string())),
                op: BinaryOp::Add,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1.0))),
            },
            "nope(1)" => ResolvedExpr::Call {
                name: "nope".to_string(),
                args: vec![ResolvedExpr::Number(crate::number::Number::from(1.0))],
            },
            "1e308 ^ 2" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1e308))),
                op: BinaryOp::Pow,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(2.0))),
            },
            "1e308 + 1e308" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1e308))),
                op: BinaryOp::Add,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1e308))),
            },
            "1e308 * 1e308" => ResolvedExpr::Binary {
                lhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1e308))),
                op: BinaryOp::Mul,
                rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1e308))),
            },
            _ => panic!("missing test expression fixture for {source}"),
        }
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
            Ok(args[0].clone() + args[1].clone() * Number::from(10))
        });

        assert_close(
            eval_with_refs(
                &ResolvedExpr::Call {
                    name: "weighted".to_string(),
                    args: vec![
                        ResolvedExpr::Binary {
                            lhs: Box::new(ResolvedExpr::EntryReference(ExpressionId::new(1))),
                            op: BinaryOp::Add,
                            rhs: Box::new(ResolvedExpr::Number(crate::number::Number::from(1.0))),
                        },
                        ResolvedExpr::EntryReference(ExpressionId::new(2)),
                    ],
                },
                &[(ExpressionId::new(1), 2.0), (ExpressionId::new(2), 4.0)],
                &[],
                &functions,
            )
            .unwrap(),
            43.0,
        );
    }

    #[test]
    fn returns_explicit_errors_for_invalid_operations_and_references() {
        assert_eq!(eval_standard("1 / 0"), Err(EvalError::DivisionByZero));
        assert_eq!(eval_standard("1 % 0"), Err(EvalError::RemainderByZero));
        assert!(matches!(
            eval_standard("missing + 1"),
            Err(EvalError::UnknownReference(ReferenceTarget::Name(name))) if name == "missing"
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
            eval_with_refs(
                &ResolvedExpr::Call {
                    name: "sin".to_string(),
                    args: vec![],
                },
                &[],
                &[],
                &functions
            ),
            Err(EvalError::ArityMismatch {
                name,
                expected: FunctionSignature::Exact(1),
                actual: 0,
            }) if name == "sin"
        ));
        assert!(matches!(
            eval_with_refs(
                &ResolvedExpr::Call {
                    name: "avg".to_string(),
                    args: vec![],
                },
                &[],
                &[],
                &functions
            ),
            Err(EvalError::ArityMismatch {
                name,
                expected: FunctionSignature::Variadic { min: 1 },
                actual: 0,
            }) if name == "avg"
        ));
        assert!(matches!(
            eval_with_refs(
                &ResolvedExpr::Call {
                    name: "sin".to_string(),
                    args: vec![ResolvedExpr::Constant("missing".to_string())],
                },
                &[],
                &[],
                &functions
            ),
            Err(EvalError::UnknownReference(ReferenceTarget::Name(name))) if name == "missing"
        ));
    }

    #[test]
    fn standardizes_non_finite_operator_results() {
        assert!(matches!(
            eval_standard("1e308 ^ 2"),
            Err(EvalError::Math(message))
                if message == "power operation produced non-finite result"
        ));
        assert!(matches!(
            eval_standard("1e308 + 1e308"),
            Err(EvalError::Math(message))
                if message == "addition operation produced non-finite result"
        ));
        assert!(matches!(
            eval_standard("1e308 * 1e308"),
            Err(EvalError::Math(message))
                if message == "multiplication operation produced non-finite result"
        ));
    }

    #[test]
    fn standardizes_non_finite_constants_references_and_custom_functions() {
        let mut functions = FunctionRegistry::new();
        functions.register("explode", FunctionSignature::exact(0), |_| {
            Ok(Number::Float(f64::INFINITY))
        });

        assert!(matches!(
            eval_with_refs(
                &ResolvedExpr::Constant("bad".to_string()),
                &[],
                &[("bad", f64::NAN)],
                &functions
            ),
            Err(EvalError::Math(message))
                if message == "constant `bad` produced non-finite result"
        ));
        assert!(matches!(
            eval_with_refs(
                &ResolvedExpr::EntryReference(ExpressionId::new(9)),
                &[(ExpressionId::new(9), f64::INFINITY)],
                &[],
                &functions
            ),
            Err(EvalError::Math(message))
                if message == "reference `$9` produced non-finite result"
        ));
        assert!(matches!(
            eval_with_refs(
                &ResolvedExpr::Call {
                    name: "explode".to_string(),
                    args: vec![],
                },
                &[],
                &[],
                &functions
            ),
            Err(EvalError::Math(message))
                if message == "function `explode` produced non-finite result"
        ));
    }

    #[test]
    fn propagates_parseable_labels_from_result_references() -> Result<(), DagcalError> {
        let functions = FunctionRegistry::standard();

        assert_close(
            eval_with_refs(
                &ResolvedExpr::Binary {
                    lhs: Box::new(ResolvedExpr::EntryReference(ExpressionId::new(1))),
                    op: BinaryOp::Add,
                    rhs: Box::new(ResolvedExpr::EntryReference(ExpressionId::new(20))),
                },
                &[(ExpressionId::new(1), 2.0), (ExpressionId::new(20), 5.0)],
                &[],
                &functions,
            )?,
            7.0,
        );

        Ok(())
    }
}
