use crate::{CompletionDirection, ExpressionId};

/// Frontend-independent effect produced by an [`AppAction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEffect {
    None,
    FocusInput,
    FocusEntrySearch,
    ScrollToSelection,
}

/// Compatibility name for effects returned by focused session methods.
pub type SessionChange = AppEffect;

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
    InputChanged(String),
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
}
