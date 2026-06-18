use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::error::EvalError;
use crate::function::FunctionRegistry;
use crate::label::EntryLabel;

pub(crate) fn eval_expr(
    expr: &Expr,
    functions: &FunctionRegistry,
    resolve: &mut dyn FnMut(&EntryLabel) -> Result<f64, EvalError>,
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
                BinaryOp::Pow => Ok(lhs.powf(rhs)),
            }
        }
        Expr::Call { name, args } => {
            let function = functions
                .get(name)
                .ok_or_else(|| EvalError::UnknownFunction(name.clone()))?;
            if function.arity() != args.len() {
                return Err(EvalError::ArityMismatch {
                    name: name.clone(),
                    expected: function.arity(),
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
