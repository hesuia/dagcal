use crate::ast::{BinaryOp, ParsedExpr, ParsedReference, ParsedStatement, UnaryOp};
use crate::error::{DagcalError, ParseError, ParseErrorKind, SourcePosition, SourceSpan};
use crate::id::ExpressionId;
use pest::Parser;
use pest::Span;
use pest::error::{
    Error as PestError, InputLocation as PestInputLocation, LineColLocation as PestLineColLocation,
};
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
struct DagParser;

pub fn parse_expression(source: &str) -> Result<ParsedExpr, DagcalError> {
    if source.trim().is_empty() {
        return Err(parse_error(
            ParseErrorKind::EmptyInput,
            source,
            "empty expression",
        ));
    }

    let mut pairs = DagParser::parse(Rule::calculation, source).map_err(parse_pest_error)?;
    let pair = pairs
        .next()
        .ok_or_else(|| parse_error(ParseErrorKind::EmptyInput, source, "empty expression"))?;
    let expr = pair
        .into_inner()
        .next()
        .ok_or_else(|| parse_error(ParseErrorKind::EmptyInput, source, "empty expression"))?;
    build_expr(expr)
}

pub fn parse_statement(source: &str) -> Result<ParsedStatement, DagcalError> {
    if source.trim().is_empty() {
        return Err(parse_error(
            ParseErrorKind::EmptyInput,
            source,
            "empty statement",
        ));
    }

    let mut pairs = DagParser::parse(Rule::statement, source).map_err(parse_pest_error)?;
    let pair = pairs
        .next()
        .ok_or_else(|| parse_error(ParseErrorKind::EmptyInput, source, "empty statement"))?;
    let statement = pair
        .into_inner()
        .next()
        .ok_or_else(|| parse_error(ParseErrorKind::EmptyInput, source, "empty statement"))?;

    match statement.as_rule() {
        Rule::definition => build_definition(statement),
        _ => build_expr(statement).map(ParsedStatement::Expression),
    }
}

fn build_definition(pair: Pair<'_, Rule>) -> Result<ParsedStatement, DagcalError> {
    let mut inner = pair.into_inner();
    let name = inner.next().ok_or_else(|| {
        parse_error_without_span(
            ParseErrorKind::MissingExpression,
            "expected definition name",
        )
    })?;
    let expr = inner.next().ok_or_else(|| {
        parse_error_without_span(
            ParseErrorKind::MissingExpression,
            "expected definition expression",
        )
    })?;

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
        Rule::number => {
            let input = pair.as_str();
            let span = source_span_from_pest_span(pair.as_span());
            input.parse::<f64>().map(ParsedExpr::Number).map_err(|err| {
                parse_error_with_span(
                    ParseErrorKind::InvalidNumber,
                    format!("invalid number `{input}`: {err}"),
                    span,
                )
            })
        }
        Rule::ident => Ok(ParsedExpr::Reference(ParsedReference::Name(
            pair.as_str().to_string(),
        ))),
        Rule::result_ref => {
            let id = parse_result_ref(pair)?;
            Ok(ParsedExpr::Reference(ParsedReference::Id(id)))
        }
        _ => Err(parse_error_without_span(
            ParseErrorKind::UnexpectedRule,
            format!("unexpected parser rule {:?}", pair.as_rule()),
        )),
    }
}

fn parse_result_ref(pair: Pair<'_, Rule>) -> Result<ExpressionId, DagcalError> {
    let input = pair.as_str();
    let span = source_span_from_pest_span(pair.as_span());
    let digits = input
        .strip_prefix('$')
        .ok_or_else(|| invalid_reference_error(input, span.clone()))?;
    let value = digits
        .parse::<usize>()
        .map_err(|_| invalid_reference_error(input, span.clone()))?;
    if value == 0 {
        return Err(invalid_reference_error(input, span));
    }
    Ok(ExpressionId::new(value))
}

fn build_only_child(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
    let child = pair.into_inner().next().ok_or_else(|| {
        parse_error_without_span(ParseErrorKind::MissingExpression, "expected expression")
    })?;
    build_expr(child)
}

fn build_left_assoc(pair: Pair<'_, Rule>) -> Result<ParsedExpr, DagcalError> {
    let mut inner = pair.into_inner();
    let first = inner.next().ok_or_else(|| {
        parse_error_without_span(
            ParseErrorKind::MissingExpression,
            "expected left-hand expression",
        )
    })?;
    let mut expr = build_expr(first)?;

    while let Some(op) = inner.next() {
        let rhs = inner.next().ok_or_else(|| {
            parse_error_without_span(
                ParseErrorKind::MissingExpression,
                "expected right-hand expression",
            )
        })?;
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

    let mut expr = rhs.ok_or_else(|| {
        parse_error_without_span(ParseErrorKind::MissingExpression, "expected unary operand")
    })?;
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
    let lhs = inner.next().ok_or_else(|| {
        parse_error_without_span(ParseErrorKind::MissingExpression, "expected power base")
    })?;
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
        .ok_or_else(|| {
            parse_error_without_span(ParseErrorKind::MissingExpression, "expected function name")
        })?
        .as_str()
        .to_string();
    let args = inner.map(build_expr).collect::<Result<Vec<_>, _>>()?;
    Ok(ParsedExpr::Call { name, args })
}

fn unary_op(op: &str) -> Result<UnaryOp, DagcalError> {
    match op {
        "+" => Ok(UnaryOp::Plus),
        "-" => Ok(UnaryOp::Minus),
        _ => Err(parse_error_without_span(
            ParseErrorKind::UnknownOperator,
            format!("unknown unary operator `{op}`"),
        )),
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
        _ => Err(parse_error_without_span(
            ParseErrorKind::UnknownOperator,
            format!("unknown binary operator `{op}`"),
        )),
    }
}

fn parse_pest_error(err: PestError<Rule>) -> DagcalError {
    // Grammar failures stay in ParseError so the caller can show source location.
    let span = source_span_from_pest_error(&err);
    DagcalError::Parse(ParseError::new(ParseErrorKind::Syntax, err.to_string()).with_span(span))
}

fn parse_error(kind: ParseErrorKind, source: &str, message: impl Into<String>) -> DagcalError {
    DagcalError::Parse(ParseError::at_input(kind, source, message))
}

fn parse_error_with_span(
    kind: ParseErrorKind,
    message: impl Into<String>,
    span: SourceSpan,
) -> DagcalError {
    DagcalError::Parse(ParseError::new(kind, message).with_span(span))
}

fn parse_error_without_span(kind: ParseErrorKind, message: impl Into<String>) -> DagcalError {
    DagcalError::Parse(ParseError::new(kind, message))
}

fn invalid_reference_error(input: &str, span: SourceSpan) -> DagcalError {
    parse_error_with_span(
        ParseErrorKind::InvalidReference,
        format!("invalid result reference `{input}`"),
        span,
    )
}

fn source_span_from_pest_error(err: &PestError<Rule>) -> SourceSpan {
    match (&err.location, &err.line_col) {
        (PestInputLocation::Pos(byte), PestLineColLocation::Pos((line, column))) => {
            let pos = SourcePosition::new(*byte, *line, *column);
            SourceSpan::new(pos.clone(), pos)
        }
        (
            PestInputLocation::Span((start_byte, end_byte)),
            PestLineColLocation::Span((start_line, start_column), (end_line, end_column)),
        ) => SourceSpan::new(
            SourcePosition::new(*start_byte, *start_line, *start_column),
            SourcePosition::new(*end_byte, *end_line, *end_column),
        ),
        (PestInputLocation::Pos(byte), PestLineColLocation::Span((line, column), _)) => {
            let pos = SourcePosition::new(*byte, *line, *column);
            SourceSpan::new(pos.clone(), pos)
        }
        (
            PestInputLocation::Span((start_byte, end_byte)),
            PestLineColLocation::Pos((line, column)),
        ) => SourceSpan::new(
            SourcePosition::new(*start_byte, *line, *column),
            SourcePosition::new(*end_byte, *line, *column),
        ),
    }
}

fn source_span_from_pest_span(span: Span<'_>) -> SourceSpan {
    let (start, end) = span.split();
    let (start_line, start_column) = start.line_col();
    let (end_line, end_column) = end.line_col();
    SourceSpan::new(
        SourcePosition::new(span.start(), start_line, start_column),
        SourcePosition::new(span.end(), end_line, end_column),
    )
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
    fn parses_standalone_number_literals() {
        assert_eq!(parse_expression("10").unwrap(), ParsedExpr::Number(10.0));
        assert_eq!(parse_expression("4.2").unwrap(), ParsedExpr::Number(4.2));
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
    fn reports_empty_expression_with_location() {
        let err = parse_expression("   ").unwrap_err();

        match err {
            DagcalError::Parse(err) => {
                assert_eq!(err.kind, ParseErrorKind::EmptyInput);
                let span = err.span.unwrap();
                assert_eq!(span.start.byte, 0);
                assert_eq!(span.start.line, 1);
                assert_eq!(span.start.column, 1);
            }
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    #[test]
    fn reports_invalid_result_reference_with_location() {
        let err = parse_expression("$0").unwrap_err();

        match err {
            DagcalError::Parse(err) => {
                assert_eq!(err.kind, ParseErrorKind::InvalidReference);
                let span = err.span.unwrap();
                assert_eq!(span.start.byte, 0);
                assert_eq!(span.end.byte, 2);
            }
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    #[test]
    fn reports_syntax_errors_with_location() {
        let err = parse_statement("broken = 1 +").unwrap_err();

        match err {
            DagcalError::Parse(err) => {
                assert_eq!(err.kind, ParseErrorKind::Syntax);
                let span = err.span.unwrap();
                assert_eq!(span.start.line, 1);
                assert!(span.start.column > 1);
            }
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    #[test]
    fn reports_unexpected_rule_for_unhandled_parser_pairs() {
        let pair = DagParser::parse(Rule::calculation, "1")
            .unwrap()
            .next()
            .unwrap();
        let err = build_expr(pair).unwrap_err();

        assert_parse_error_kind(err, ParseErrorKind::UnexpectedRule);
    }

    #[test]
    fn reports_missing_expression_for_empty_parser_pairs() {
        let pair = DagParser::parse(Rule::result_ref, "$1")
            .unwrap()
            .next()
            .unwrap();
        let err = build_only_child(pair).unwrap_err();

        assert_parse_error_kind(err, ParseErrorKind::MissingExpression);
    }

    #[test]
    fn reports_unknown_operator_for_unrecognized_operator_tokens() {
        assert_parse_error_kind(unary_op("!").unwrap_err(), ParseErrorKind::UnknownOperator);
        assert_parse_error_kind(
            binary_op("??").unwrap_err(),
            ParseErrorKind::UnknownOperator,
        );
    }

    #[test]
    fn reports_invalid_number_for_number_conversion_failures() {
        let span = SourceSpan::for_input("not-a-number");
        let err = parse_error_with_span(
            ParseErrorKind::InvalidNumber,
            "invalid number `not-a-number`",
            span,
        );

        assert_parse_error_kind(err, ParseErrorKind::InvalidNumber);
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

    fn assert_parse_error_kind(err: DagcalError, expected: ParseErrorKind) {
        match err {
            DagcalError::Parse(err) => assert_eq!(err.kind, expected),
            other => panic!("expected parse error, got {other:?}"),
        }
    }
}
