use crate::function::FunctionSignature;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DagcalError {
    /// The input could not be parsed into a valid AST.
    #[error("parse error: {0}")]
    Parse(ParseError),

    /// The AST was valid, but evaluation failed.
    #[error(transparent)]
    Eval(#[from] EvalError),

    /// A persisted engine snapshot could not be restored.
    #[error("persistence error: {0}")]
    Persistence(PersistenceError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    #[error("unsupported snapshot version {actual}; expected {expected}")]
    UnsupportedVersion { actual: u32, expected: u32 },

    #[error("entry id must be 1-based, got {0}")]
    InvalidId(usize),

    #[error("duplicate entry id ${0}")]
    DuplicateId(usize),

    #[error("invalid entry name `{0}`")]
    InvalidName(String),

    #[error("duplicate entry name `{0}`")]
    DuplicateName(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub message: String,
    pub span: Option<SourceSpan>,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn at_input(kind: ParseErrorKind, input: &str, message: impl Into<String>) -> Self {
        Self::new(kind, message).with_span(SourceSpan::for_input(input))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = &self.span {
            write!(
                f,
                "line {}, column {}: {}",
                span.start.line, span.start.column, self.message
            )
        } else {
            f.write_str(&self.message)
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Grammar-level syntax failure reported by the parser.
    Syntax,
    /// The input was empty or whitespace only.
    EmptyInput,
    /// A numeric literal could not be converted to `f64`.
    InvalidNumber,
    /// A `$n` reference was malformed or otherwise invalid.
    InvalidReference,
    /// An entry target such as `name` or `$n` was invalid.
    InvalidEntryTarget,
    /// The parser hit a rule it does not expect to surface directly.
    UnexpectedRule,
    /// An operator token was not recognized.
    UnknownOperator,
    /// The parser expected an expression child but did not find one.
    MissingExpression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePosition {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

impl SourcePosition {
    pub fn new(byte: usize, line: usize, column: usize) -> Self {
        Self { byte, line, column }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    pub fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }

    pub fn for_input(input: &str) -> Self {
        Self {
            start: SourcePosition::new(0, 1, 1),
            end: SourcePosition::new(input.len(), 1, input.chars().count() + 1),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum EvalError {
    #[error("unknown reference `{0}`")]
    UnknownReference(String),

    #[error("unknown function `{0}`")]
    UnknownFunction(String),

    #[error("function `{name}` expected {expected}, got {actual} argument(s)")]
    ArityMismatch {
        name: String,
        expected: FunctionSignature,
        actual: usize,
    },

    #[error("division by zero")]
    DivisionByZero,

    #[error("remainder by zero")]
    RemainderByZero,

    #[error("dependency `{0}` failed")]
    DependencyError(String),

    #[error("cycle detected involving `{0}`")]
    CycleDetected(String),

    #[error("{0}")]
    Math(String),
}
