use crate::{CompletionDirection, ExpressionId};

/// Frontend-independent effect produced by an [`AppAction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEffect {
    None,
    FocusInput,
    FocusEntrySearch,
    ScrollToSelection,
}

/// Entry-state predicate used by search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStateFilter {
    All,
    Values,
    Errors,
}

/// Direction used when moving the selected entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionDirection {
    Previous,
    Next,
}

/// Frontend-independent application command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    /// Replaces the frontend-visible status without changing calculation state.
    SetStatus(String),
    /// Resets the editor to an empty, non-editing input without creating an entry.
    ResetInput,
    InputChanged(String),
    /// Updates editor text without materializing a GUI-style draft row.
    InputEdited(String),
    OpenEntrySearch,
    CloseEntrySearch,
    EntrySearchChanged(String),
    EntryStateFilterChanged(EntryStateFilter),
    ClearEntrySearch,
    SubmitInput,
    StartEdit(ExpressionId),
    StartNewEntry,
    CancelEdit,
    DeleteEntry(ExpressionId),
    RecalculateEntry(ExpressionId),
    RecalculateAll,
    InsertReference(ExpressionId),
    InsertConstant(String),
    InsertFunction(String),
    SelectEntry(ExpressionId),
    ClearSelection,
    SetHoveredEntry(ExpressionId),
    ClearHoveredEntry(ExpressionId),
    SelectHoveredEntry,
    Clear,
    Undo,
    Redo,
    MoveSelection(SelectionDirection),
    MoveCompletion(CompletionDirection),
    AcceptCompletion(usize),
    CloseCompletions,
    /// Removes an empty draft entry while preserving the current status.
    DiscardEmptyDraft,
}
