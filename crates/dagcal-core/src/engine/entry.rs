use crate::error::DagcalError;
use crate::id::ExpressionId;
use crate::number::Number;

/// Owned snapshot of one stored engine entry.
///
/// `EntryView` is returned by query methods such as
/// [`Engine::entry`](crate::Engine::entry), [`Engine::entries`](crate::Engine::entries),
/// and [`Engine::remove_entry`](crate::Engine::remove_entry). It contains
/// cloned data so callers can keep it after mutating the engine.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryView {
    /// Stable 1-based expression ID displayed as `$n`.
    pub id: ExpressionId,
    /// Optional user-facing name from a definition such as `name = expr`.
    pub name: Option<String>,
    /// Stored expression source text.
    ///
    /// For named definitions submitted through [`Engine::execute`](crate::Engine::execute),
    /// this is only the right-hand expression, not the `name =` prefix.
    pub source: String,
    /// Latest computed value or structured error for this entry.
    pub state: EntryState,
}

/// Current computation result for an engine entry or execution attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum EntryState {
    /// Expression evaluated successfully to a finite number.
    Value(Number),
    /// Parsing, resolving, dependency analysis, or evaluation failed.
    Error(DagcalError),
}

/// Result returned after executing or setting an entry.
///
/// When an entry is saved, `id` is `Some` and identifies the affected entry.
/// For statement-level parse errors from [`Engine::execute`](crate::Engine::execute),
/// the engine cannot reliably determine a target, so `id` is `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct Execution {
    /// ID of the saved or edited entry, if one was stored.
    pub id: Option<ExpressionId>,
    /// Final state of the execution target, or the parse error when no target
    /// could be saved.
    pub state: EntryState,
}
