use crate::error::DagcalError;
use crate::function::FunctionSignature;
use crate::id::ExpressionId;
use crate::number::Number;
use std::collections::BTreeSet;

/// Borrowed, allocation-free view of a stored engine entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryRef<'a> {
    /// Stable expression ID.
    pub id: ExpressionId,
    /// Optional definition name.
    pub name: Option<&'a str>,
    /// Stored expression source.
    pub source: &'a str,
    /// Latest evaluation state.
    pub state: &'a EntryState,
}

impl EntryRef<'_> {
    /// Clones this borrowed view into an independently owned value.
    pub fn into_owned(self) -> EntryView {
        EntryView {
            id: self.id,
            name: self.name.map(str::to_owned),
            source: self.source.to_owned(),
            state: self.state.clone(),
        }
    }
}

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

/// Result returned after explicitly setting an entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SetEntryResult {
    /// Saved entry execution report.
    pub execution: Execution,
    /// Target error when the saved entry's final state is an error.
    ///
    /// `None` means the target evaluated to a value. `Some` means the source
    /// was still saved and can be inspected or edited later.
    pub target_error: Option<DagcalError>,
}

/// Non-mutating parse/resolve analysis for source input.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionPreview {
    /// Source text that was analyzed.
    pub source: String,
    /// Parse/resolve status for the source.
    pub state: PreviewState,
    /// Stored entries referenced by the source after name resolution.
    pub entry_references: BTreeSet<ExpressionId>,
    /// Runtime constants referenced by the source.
    pub constant_references: BTreeSet<String>,
    /// Functions called by the source.
    pub function_references: BTreeSet<String>,
}

/// Parse/resolve status for an [`ExpressionPreview`].
#[derive(Debug, Clone, PartialEq)]
pub enum PreviewState {
    /// The source parsed and resolved successfully.
    Valid,
    /// The source could not be parsed or resolved.
    Error(DagcalError),
}

/// Completion candidate category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    /// Named stored entry.
    Entry,
    /// `$n` result reference.
    Result,
    /// Runtime constant.
    Constant,
    /// Registered function.
    Function,
}

/// Completion candidate for frontends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// Text inserted or matched by the UI.
    pub label: String,
    /// Candidate category.
    pub kind: CompletionKind,
    /// Optional display detail such as an ID or function signature.
    pub detail: Option<String>,
    /// Optional current result for entry-backed candidates.
    pub result: Option<String>,
}

impl CompletionItem {
    pub(crate) fn entry(label: String, id: ExpressionId, result: Option<String>) -> Self {
        Self {
            label,
            kind: CompletionKind::Entry,
            detail: Some(id.to_string()),
            result,
        }
    }

    pub(crate) fn result(id: ExpressionId, name: Option<&str>, result: Option<String>) -> Self {
        Self {
            label: id.to_string(),
            kind: CompletionKind::Result,
            detail: name.map(str::to_string),
            result,
        }
    }

    pub(crate) fn constant(label: String) -> Self {
        Self {
            label,
            kind: CompletionKind::Constant,
            detail: None,
            result: None,
        }
    }

    pub(crate) fn function(label: String, signature: FunctionSignature) -> Self {
        Self {
            label,
            kind: CompletionKind::Function,
            detail: Some(signature.to_string()),
            result: None,
        }
    }
}
