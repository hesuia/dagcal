//! Core calculation engine for `dagcal`.
//!
//! `dagcal-core` stores calculator entries as a small dependency graph. Each
//! saved expression receives a stable [`ExpressionId`] displayed as `$n`, and a
//! definition such as `subtotal = 100` also receives a user-facing name. Later
//! expressions can refer to either names or `$n` result references:
//!
//! ```
//! use dagcal_core::{Engine, EntryState, Number};
//!
//! let mut engine = Engine::new();
//! engine.execute("subtotal = 100");
//! engine.execute("tax = subtotal * 0.1");
//! let total = engine.execute("subtotal + tax");
//!
//! assert_eq!(total.state, EntryState::Value(Number::from(110.0)));
//! assert_eq!(
//!     engine.state(total.id.unwrap()),
//!     Some(&EntryState::Value(Number::from(110.0)))
//! );
//! ```
//!
//! The [`Engine`] owns the complete session state. When an entry is edited,
//! removed, or when a runtime constant or function changes, the engine
//! recomputes affected dependents and leaves unaffected entries untouched.
//! Invalid entries are retained where possible so frontends can render and edit
//! broken cells while dependent entries report structured [`DagcalError`] values.
//!
//! Supported expression features include numeric literals, arithmetic operators,
//! parentheses, references to previous results, named entries, runtime
//! constants, and standard or user-registered functions. The public API is kept
//! intentionally small: use [`Engine`] for session work, [`EntryView`] and
//! [`EntryState`] for rendering state, [`EngineSnapshot`] for persistence, and
//! [`DagcalError`] plus its nested error types for structured diagnostics.
//!
//! ## Entry lifecycle
//!
//! - [`Engine::execute`] parses user input as either `name = expression` or a
//!   plain expression. Named definitions update their existing entry; plain
//!   expressions append a new `$n` result.
//! - [`Engine::set_entry`] edits a target by name, `$n`, or [`ExpressionId`].
//!   ID-specific variants such as [`Engine::set_entry_by_id`] are available
//!   when callers already have stable IDs.
//! - [`Engine::remove_entry`] deletes a target and recomputes entries that
//!   depended on it. Removed `$n` references remain stable and become unknown
//!   references until restored with [`Engine::set_entry`] or
//!   [`Engine::set_entry_by_id`].
//! - [`Engine::snapshot`] and [`Engine::restore_snapshot`] persist the original
//!   source text, IDs, and names. Values are recomputed on restore.
//!
//! ## Extending evaluation
//!
//! Constants and functions are part of engine state and can be changed at
//! runtime. Existing entries that reference the changed symbol are recomputed:
//!
//! ```
//! use dagcal_core::{Engine, EntryState, Number};
//!
//! let mut engine = Engine::new();
//! let before = engine.execute("triple(14)");
//! assert!(matches!(before.state, EntryState::Error(_)));
//!
//! engine.register_fixed_function("triple", 1, |args| {
//!     Ok(args[0].clone() * Number::from(3))
//! });
//! assert_eq!(
//!     engine.state(before.id.unwrap()),
//!     Some(&EntryState::Value(Number::from(42.0)))
//! );
//! ```
//!
//! Built-in functions and user functions must return finite [`Number`] values.
//! A non-finite float value is reported as [`EvalError::Math`].

mod ast;
mod dependency_graph;
mod engine;
mod error;
mod eval;
mod function;
mod id;
mod number;
mod parser;
mod persistence;

pub use engine::{
    CycleDiagnostics, Engine, EntryState, EntryTarget, EntryView, Execution, IntoEntryTarget,
};
pub use error::{
    DagcalError, EvalError, ParseError, ParseErrorKind, PersistenceError, ReferenceTarget,
    SourcePosition, SourceSpan,
};
pub use function::FunctionSignature;
pub use id::ExpressionId;
pub use number::Number;
pub use persistence::{ENGINE_SNAPSHOT_VERSION, EngineSnapshot, PersistedEntry};
