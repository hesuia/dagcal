//! Shared UI-agnostic application state for `dagcal` frontends.

mod completion;
mod draft;
pub mod formatting;
mod session;

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
pub use session::{AppSession, EntryStateFilter, SelectionDirection, SessionChange};
