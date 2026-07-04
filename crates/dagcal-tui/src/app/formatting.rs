use dagcal_app::{CompletionKind, EntryState, EntryStateFilter, EntryView, formatting};

pub fn state_summary(state: &EntryState) -> String {
    formatting::table_state_summary(state)
}

pub fn expression_source(entry: &EntryView) -> String {
    formatting::entry_expression_source(entry)
}

pub fn kind_label(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Entry => "entry",
        CompletionKind::Result => "result",
        CompletionKind::Constant => "constant",
        CompletionKind::Function => "function",
    }
}

pub fn filter_label(filter: EntryStateFilter) -> &'static str {
    match filter {
        EntryStateFilter::All => "All",
        EntryStateFilter::Values => "Values",
        EntryStateFilter::Errors => "Errors",
    }
}

pub fn availability_label(available: bool) -> &'static str {
    if available { "yes" } else { "no" }
}
