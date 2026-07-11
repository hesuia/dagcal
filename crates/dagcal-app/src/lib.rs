//! Shared UI-agnostic application state for `dagcal` frontends.
//!
//! Frontends send [`AppAction`] values to [`AppSession::dispatch`] and translate
//! the returned [`AppEffect`] values into toolkit-specific focus and scrolling
//! operations. Read-only selectors keep rendering code independent from engine
//! mutation details.

mod action;
mod completion;
mod draft;
pub mod formatting;
mod session;

pub use action::{AppAction, AppEffect, EntryStateFilter, SelectionDirection};
pub use completion::{
    CompletionCandidate, CompletionDirection, CompletionMenuEntry, CompletionState,
    CompletionToken, completion_menu_entries_for_kind,
};
pub use dagcal_core::{
    CompletionItem, CompletionKind, DagcalError, Engine, EngineSnapshot, EntryRemoval, EntryState,
    EntryTarget, EntryView, Execution, ExpressionId, IntoEntryTarget, Number, PersistedEntry,
    PreviewState, SetEntryResult,
};
pub use draft::Draft;
pub use session::AppSession;
