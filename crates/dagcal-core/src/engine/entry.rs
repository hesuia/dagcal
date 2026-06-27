use crate::error::DagcalError;
use crate::id::ExpressionId;
use crate::number::Number;
use std::collections::BTreeSet;

/// Owned snapshot of one stored engine entry.
///
/// `EntryView` is returned by query methods such as
/// [`Engine::entry`](crate::Engine::entry), [`Engine::entries`](crate::Engine::entries),
/// and [`EntryRemoval::removed_entry`]. It contains
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
/// [`Engine::execute`](crate::Engine::execute) saves statement-level parse
/// errors as unnamed entries so their original source can be edited later.
#[derive(Debug, Clone, PartialEq)]
pub struct Execution {
    /// ID of the saved or edited entry.
    pub id: ExpressionId,
    /// Final state of the execution target.
    pub state: EntryState,
    /// Entries recomputed while saving this entry.
    ///
    /// The target ID is always included. Transitive dependents are included
    /// when they were affected by the saved source.
    pub affected_ids: BTreeSet<ExpressionId>,
}

/// Result returned after removing an entry.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryRemoval {
    /// Snapshot of the entry before it was removed.
    pub removed_entry: EntryView,
    /// Entries recomputed after the removal.
    ///
    /// This includes the removed ID and any transitive dependents that were
    /// affected by the removed entry.
    pub affected_ids: BTreeSet<ExpressionId>,
}
