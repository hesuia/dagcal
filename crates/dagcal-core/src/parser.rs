use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::error::DagcalError;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
struct DagParser;

pub fn parse_expression(source: &str) -> Result<Expr, DagcalError> {
    let mut pairs = DagParser::parse(Rule::calculation, source)
        .map_err(|err| DagcalError::Parse(err.to_string()))?;
    let pair = pairs
        .next()
        .ok_or_else(|| DagcalError::Parse("empty expression".to_string()))?;
    let expr = pair
        .into_inner()
        .next()
        .ok_or_else(|| DagcalError::Parse("empty expression".to_string()))?;
    build_expr(expr)
}

fn build_expr(pair: Pair<'_, Rule>) -> Result<Expr, DagcalError> {
    match pair.as_rule() {
        Rule::expr => build_only_child(pair),
        Rule::add => build_left_assoc(pair),
        Rule::mul => build_left_assoc(pair),
        Rule::unary => build_unary(pair),
        Rule::pow => build_pow(pair),
        Rule::primary => build_only_child(pair),
        Rule::function_call => build_function_call(pair),
        Rule::number => pair
            .as_str()
            .parse::<f64>()
            .map(Expr::Number)
            .map_err(|err| {
                DagcalError::Parse(format!("invalid number `{}`: {err}", pair.as_str()))
            }),
        Rule::ident | Rule::result_ref => Ok(Expr::Reference(pair.as_str().to_string())),
        _ => Err(DagcalError::Parse(format!(
            "unexpected parser rule {:?}",
            pair.as_rule()
        ))),
    }
}

fn build_only_child(pair: Pair<'_, Rule>) -> Result<Expr, DagcalError> {
    let child = pair
        .into_inner()
        .next()
        .ok_or_else(|| DagcalError::Parse("expected expression".to_string()))?;
    build_expr(child)
}

fn build_left_assoc(pair: Pair<'_, Rule>) -> Result<Expr, DagcalError> {
    let mut inner = pair.into_inner();
    let first = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected left-hand expression".to_string()))?;
    let mut expr = build_expr(first)?;

    while let Some(op) = inner.next() {
        let rhs = inner
            .next()
            .ok_or_else(|| DagcalError::Parse("expected right-hand expression".to_string()))?;
        expr = Expr::Binary {
            lhs: Box::new(expr),
            op: binary_op(op.as_str())?,
            rhs: Box::new(build_expr(rhs)?),
        };
    }

    Ok(expr)
}

fn build_unary(pair: Pair<'_, Rule>) -> Result<Expr, DagcalError> {
    let mut ops = Vec::new();
    let mut rhs = None;

    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::unary_op => ops.push(unary_op(child.as_str())?),
            _ => rhs = Some(build_expr(child)?),
        }
    }

    let mut expr = rhs.ok_or_else(|| DagcalError::Parse("expected unary operand".to_string()))?;
    for op in ops.into_iter().rev() {
        expr = Expr::Unary {
            op,
            rhs: Box::new(expr),
        };
    }
    Ok(expr)
}

fn build_pow(pair: Pair<'_, Rule>) -> Result<Expr, DagcalError> {
    let mut inner = pair.into_inner();
    let lhs = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected power base".to_string()))?;
    let mut expr = build_expr(lhs)?;

    if let Some(rhs) = inner.next() {
        expr = Expr::Binary {
            lhs: Box::new(expr),
            op: BinaryOp::Pow,
            rhs: Box::new(build_expr(rhs)?),
        };
    }

    Ok(expr)
}

fn build_function_call(pair: Pair<'_, Rule>) -> Result<Expr, DagcalError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected function name".to_string()))?
        .as_str()
        .to_string();
    let args = inner.map(build_expr).collect::<Result<Vec<_>, _>>()?;
    Ok(Expr::Call { name, args })
}

fn unary_op(op: &str) -> Result<UnaryOp, DagcalError> {
    match op {
        "+" => Ok(UnaryOp::Plus),
        "-" => Ok(UnaryOp::Minus),
        _ => Err(DagcalError::Parse(format!("unknown unary operator `{op}`"))),
    }
}

fn binary_op(op: &str) -> Result<BinaryOp, DagcalError> {
    match op {
        "+" => Ok(BinaryOp::Add),
        "-" => Ok(BinaryOp::Sub),
        "*" => Ok(BinaryOp::Mul),
        "/" => Ok(BinaryOp::Div),
        "%" => Ok(BinaryOp::Rem),
        "^" => Ok(BinaryOp::Pow),
        _ => Err(DagcalError::Parse(format!(
            "unknown binary operator `{op}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn parses_operator_precedence() {
        let expr = parse_expression("1 + 2 * 3").unwrap();

        assert_eq!(
            expr,
            Expr::Binary {
                lhs: Box::new(Expr::Number(1.0)),
                op: BinaryOp::Add,
                rhs: Box::new(Expr::Binary {
                    lhs: Box::new(Expr::Number(2.0)),
                    op: BinaryOp::Mul,
                    rhs: Box::new(Expr::Number(3.0)),
                }),
            }
        );
    }

    #[test]
    fn parses_power_as_right_associative() {
        let expr = parse_expression("2 ^ 3 ^ 2").unwrap();

        assert_eq!(
            expr,
            Expr::Binary {
                lhs: Box::new(Expr::Number(2.0)),
                op: BinaryOp::Pow,
                rhs: Box::new(Expr::Binary {
                    lhs: Box::new(Expr::Number(3.0)),
                    op: BinaryOp::Pow,
                    rhs: Box::new(Expr::Number(2.0)),
                }),
            }
        );
    }

    #[test]
    fn parses_function_calls_and_references() {
        let expr = parse_expression("sin(pi) + x").unwrap();

        assert_eq!(
            expr.references(),
            BTreeSet::from(["pi".to_string(), "x".to_string()])
        );
    }

    #[test]
    fn parses_dollar_result_references() {
        let expr = parse_expression("$1 + $20 * subtotal").unwrap();

        assert_eq!(
            expr.references(),
            BTreeSet::from(["$1".to_string(), "$20".to_string(), "subtotal".to_string()])
        );
    }

    #[test]
    fn parses_whitespace_and_nested_function_arguments() {
        let expr = parse_expression(" \n max(1, min(x, 2 + y)) \t ").unwrap();

        assert_eq!(
            expr.references(),
            BTreeSet::from(["x".to_string(), "y".to_string()])
        );
    }

    #[test]
    fn rejects_invalid_syntax() {
        assert!(parse_expression("").is_err());
        assert!(parse_expression("1 +").is_err());
        assert!(parse_expression("sin(,1)").is_err());
        assert!(parse_expression("1 2").is_err());
        assert!(parse_expression("1..2").is_err());
        assert!(parse_expression("$").is_err());
        assert!(parse_expression("$abc").is_err());
    }
}
