use crate::function::FunctionSignature;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DagcalError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error(transparent)]
    Eval(#[from] EvalError),
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
