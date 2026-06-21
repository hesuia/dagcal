use crate::ast::{BinaryOp, ParsedExpr, ParsedReference, ParsedStatement, UnaryOp};
use crate::error::DagcalError;
use crate::id::ExpressionId;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
struct DagParser;

pub fn parse_expression(source: &str) -> Result<ParsedExpr, DagcalError> {
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

pub fn parse_statement(source: &str) -> Result<ParsedStatement, DagcalError> {
    let mut pairs = DagParser::parse(Rule::statement, source)
        .map_err(|err| DagcalError::Parse(err.to_string()))?;
    let pair = pairs
        .next()
        .ok_or_else(|| DagcalError::Parse("empty statement".to_string()))?;
    let statement = pair
        .into_inner()
        .next()
        .ok_or_else(|| DagcalError::Parse("empty statement".to_string()))?;

    match statement.as_rule() {
        Rule::definition => build_definition(statement),
        _ => build_expr(statement).map(ParsedStatement::Expression),
    }
}

fn build_definition(pair: Pair<'_, Rule>) -> Result<ParsedStatement, DagcalError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected definition name".to_string()))?;
    let expr = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected definition expression".to_string()))?;

    Ok(ParsedStatement::Definition {
        name: name.as_str().to_string(),
        expr: build_expr(expr)?,
    })
}

fn build_expr(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
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
            .map(ParsedExpr::Number)
            .map_err(|err| {
                DagcalError::Parse(format!("invalid number `{}`: {err}", pair.as_str()))
            }),
        Rule::ident => Ok(ParsedExpr::Reference(ParsedReference::Name(
            pair.as_str().to_string(),
        ))),
        Rule::result_ref => {
            let id = parse_result_ref(pair.as_str())?;
            Ok(ParsedExpr::Reference(ParsedReference::Id(id)))
        }
        _ => Err(DagcalError::Parse(format!(
            "unexpected parser rule {:?}",
            pair.as_rule()
        ))),
    }
}

fn parse_result_ref(input: &str) -> Result<ExpressionId, DagcalError> {
    let digits = input
        .strip_prefix('$')
        .ok_or_else(|| DagcalError::Parse(format!("invalid result reference `{input}`")))?;
    let value = digits
        .parse::<usize>()
        .map_err(|_| DagcalError::Parse(format!("invalid result reference `{input}`")))?;
    if value == 0 {
        return Err(DagcalError::Parse(format!(
            "invalid result reference `{input}`"
        )));
    }
    Ok(ExpressionId::new(value))
}

fn build_only_child(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
    let child = pair
        .into_inner()
        .next()
        .ok_or_else(|| DagcalError::Parse("expected expression".to_string()))?;
    build_expr(child)
}

fn build_left_assoc(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
    let mut inner = pair.into_inner();
    let first = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected left-hand expression".to_string()))?;
    let mut expr = build_expr(first)?;

    while let Some(op) = inner.next() {
        let rhs = inner
            .next()
            .ok_or_else(|| DagcalError::Parse("expected right-hand expression".to_string()))?;
        expr = ParsedExpr::Binary {
            lhs: Box::new(expr),
            op: binary_op(op.as_str())?,
            rhs: Box::new(build_expr(rhs)?),
        };
    }

    Ok(expr)
}

fn build_unary(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
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
        expr = ParsedExpr::Unary {
            op,
            rhs: Box::new(expr),
        };
    }
    Ok(expr)
}

fn build_pow(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
    let mut inner = pair.into_inner();
    let lhs = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected power base".to_string()))?;
    let mut expr = build_expr(lhs)?;

    if let Some(rhs) = inner.next() {
        expr = ParsedExpr::Binary {
            lhs: Box::new(expr),
            op: BinaryOp::Pow,
            rhs: Box::new(build_expr(rhs)?),
        };
    }

    Ok(expr)
}

fn build_function_call(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| DagcalError::Parse("expected function name".to_string()))?
        .as_str()
        .to_string();
    let args = inner.map(build_expr).collect::<Result<Vec<_>, _>>()?;
    Ok(ParsedExpr::Call { name, args })
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
            ParsedExpr::Binary {
                lhs: Box::new(ParsedExpr::Number(1.0)),
                op: BinaryOp::Add,
                rhs: Box::new(ParsedExpr::Binary {
                    lhs: Box::new(ParsedExpr::Number(2.0)),
                    op: BinaryOp::Mul,
                    rhs: Box::new(ParsedExpr::Number(3.0)),
                }),
            }
        );
    }

    #[test]
    fn parses_decimal_and_scientific_notation() {
        let expr = parse_expression(".5 + 1e3 + 2.5E-1 + 1.").unwrap();

        assert_eq!(
            expr,
            ParsedExpr::Binary {
                lhs: Box::new(ParsedExpr::Binary {
                    lhs: Box::new(ParsedExpr::Binary {
                        lhs: Box::new(ParsedExpr::Number(0.5)),
                        op: BinaryOp::Add,
                        rhs: Box::new(ParsedExpr::Number(1000.0)),
                    }),
                    op: BinaryOp::Add,
                    rhs: Box::new(ParsedExpr::Number(0.25)),
                }),
                op: BinaryOp::Add,
                rhs: Box::new(ParsedExpr::Number(1.0)),
            }
        );
    }

    #[test]
    fn parses_power_as_right_associative() {
        let expr = parse_expression("2 ^ 3 ^ 2").unwrap();

        assert_eq!(
            expr,
            ParsedExpr::Binary {
                lhs: Box::new(ParsedExpr::Number(2.0)),
                op: BinaryOp::Pow,
                rhs: Box::new(ParsedExpr::Binary {
                    lhs: Box::new(ParsedExpr::Number(3.0)),
                    op: BinaryOp::Pow,
                    rhs: Box::new(ParsedExpr::Number(2.0)),
                }),
            }
        );
    }

    #[test]
    fn parses_function_calls_and_references() {
        let expr = parse_expression("sin(pi) + x").unwrap();

        assert_eq!(
            expr.references(),
            BTreeSet::from([
                ParsedReference::Name("pi".to_string()),
                ParsedReference::Name("x".to_string())
            ])
        );
    }

    #[test]
    fn parses_dollar_result_references() {
        let expr = parse_expression("$1 + $20 * subtotal").unwrap();

        assert_eq!(
            expr.references(),
            BTreeSet::from([
                ParsedReference::Id(ExpressionId::new(1)),
                ParsedReference::Id(ExpressionId::new(20)),
                ParsedReference::Name("subtotal".to_string())
            ])
        );
    }

    #[test]
    fn parses_whitespace_and_nested_function_arguments() {
        let expr = parse_expression(" \n max(1, min(x, 2 + y)) \t ").unwrap();

        assert_eq!(
            expr.references(),
            BTreeSet::from([
                ParsedReference::Name("x".to_string()),
                ParsedReference::Name("y".to_string())
            ])
        );
    }

    #[test]
    fn parses_named_definition_statements() {
        let statement = parse_statement("subtotal = 100 + tax").unwrap();

        match statement {
            ParsedStatement::Definition { name, expr } => {
                assert_eq!(name, "subtotal");
                assert_eq!(
                    expr.references(),
                    BTreeSet::from([ParsedReference::Name("tax".to_string())])
                );
            }
            other => panic!("expected definition, got {other:?}"),
        }
    }

    #[test]
    fn parses_expression_statements() {
        let statement = parse_statement("$1 + 10").unwrap();

        match statement {
            ParsedStatement::Expression(expr) => {
                assert_eq!(
                    expr.references(),
                    BTreeSet::from([ParsedReference::Id(ExpressionId::new(1))])
                );
            }
            other => panic!("expected expression, got {other:?}"),
        }
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

    #[test]
    fn rejects_invalid_definition_statements() {
        assert!(parse_statement("$1 = 100").is_err());
        assert!(parse_statement("= 1").is_err());
        assert!(parse_statement("x =").is_err());
        assert!(parse_statement("x = y = 1").is_err());
    }
}
