use crate::function::FunctionSignature;
use std::fmt;
use thiserror::Error;

/// Top-level error type returned by the public API.
///
/// Parsing failures are reported as [`DagcalError::Parse`], runtime/evaluation
/// failures as [`DagcalError::Eval`], and snapshot validation failures as
/// [`DagcalError::Persistence`].
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

/// Snapshot validation and compatibility failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// Snapshot version does not match [`ENGINE_SNAPSHOT_VERSION`](crate::ENGINE_SNAPSHOT_VERSION).
    #[error("unsupported snapshot version {actual}; expected {expected}")]
    UnsupportedVersion { actual: u32, expected: u32 },

    /// Persisted entry ID was zero.
    #[error("entry id must be 1-based, got {0}")]
    InvalidId(usize),

    /// Two or more persisted entries used the same ID.
    #[error("duplicate entry id ${0}")]
    DuplicateId(usize),

    /// Persisted entry name does not match the allowed identifier syntax.
    #[error("invalid entry name `{0}`")]
    InvalidName(String),

    /// Two or more persisted entries used the same name.
    #[error("duplicate entry name `{0}`")]
    DuplicateName(String),
}

/// Structured parse error with optional source span information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Machine-readable error category.
    pub kind: ParseErrorKind,
    /// Human-readable parse diagnostic.
    pub message: String,
    /// Optional span in the original input.
    pub span: Option<SourceSpan>,
}

impl ParseError {
    /// Creates a parse error without span information.
    pub fn new(kind: ParseErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            span: None,
        }
    }

    /// Attaches source span information to this error.
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Creates a parse error spanning the whole input.
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

/// Machine-readable parse error category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Grammar-level syntax failure reported by the parser.
    Syntax,
    /// The input was empty or whitespace only.
    EmptyInput,
    /// A numeric literal could not be converted to a number.
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

/// 1-based line/column plus byte offset within source input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePosition {
    /// Byte offset from the start of the input.
    pub byte: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
}

impl SourcePosition {
    /// Creates a source position from raw byte, line, and column values.
    pub fn new(byte: usize, line: usize, column: usize) -> Self {
        Self { byte, line, column }
    }
}

/// Half-open span in source input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    /// Inclusive start position.
    pub start: SourcePosition,
    /// Exclusive end position.
    pub end: SourcePosition,
}

impl SourceSpan {
    /// Creates a span from explicit start and end positions.
    pub fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }

    /// Creates a span covering a single-line input string.
    pub fn for_input(input: &str) -> Self {
        Self {
            start: SourcePosition::new(0, 1, 1),
            end: SourcePosition::new(input.len(), 1, input.chars().count() + 1),
        }
    }
}

/// Evaluation and dependency-resolution failures.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum EvalError {
    /// A name or `$n` reference could not be resolved to a stored entry or
    /// constant.
    #[error("unknown reference `{0}`")]
    UnknownReference(String),

    /// A called function name is not registered.
    #[error("unknown function `{0}`")]
    UnknownFunction(String),

    /// A function was called with the wrong number of arguments.
    #[error("function `{name}` expected {expected}, got {actual} argument(s)")]
    ArityMismatch {
        /// Function name as written in the expression.
        name: String,
        /// Registered function signature.
        expected: FunctionSignature,
        /// Number of arguments supplied by the expression.
        actual: usize,
    },

    /// Division operator used a zero divisor.
    #[error("division by zero")]
    DivisionByZero,

    /// Remainder operator used a zero divisor.
    #[error("remainder by zero")]
    RemainderByZero,

    /// This expression could not run because a referenced entry is currently
    /// in an error state.
    #[error("dependency `{0}` failed")]
    DependencyError(String),

    /// This expression participates in, or is blocked by, a dependency cycle.
    #[error("cycle detected involving `{0}`")]
    CycleDetected(String),

    /// Numeric evaluation failed, including non-finite constants or function
    /// results.
    #[error("{0}")]
    Math(String),
}
