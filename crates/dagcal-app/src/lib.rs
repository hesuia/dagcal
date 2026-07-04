//! Shared UI-agnostic application state for `dagcal` frontends.

mod completion;
mod draft;
pub mod formatting;
mod session;

pub use completion::{CompletionCandidate, CompletionDirection, CompletionState, CompletionToken};
pub use dagcal_core::{
    CompletionItem, CompletionKind, DagcalError, Engine, EngineSnapshot, EntryRemoval, EntryState,
    EntryTarget, EntryView, Execution, ExpressionId, IntoEntryTarget, Number, PersistedEntry,
    PreviewState, SetEntryResult,
};
pub use draft::Draft;
pub use session::{AppSession, EntryStateFilter, SelectionDirection, SessionChange};
