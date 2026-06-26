use super::repository::EntryRepository;
use super::runtime::RuntimeEnvironment;
use crate::id::ExpressionId;

pub(super) enum ResolvedSymbol {
    Entry(ExpressionId),
    Constant(String),
}

pub(super) trait SymbolResolver {
    fn resolve_name(&self, name: &str) -> Option<ResolvedSymbol>;
}

pub(super) struct SymbolTable<'a> {
    entries: &'a EntryRepository,
    runtime: &'a RuntimeEnvironment,
}

impl<'a> SymbolTable<'a> {
    pub(super) fn new(entries: &'a EntryRepository, runtime: &'a RuntimeEnvironment) -> Self {
        Self { entries, runtime }
    }
}

impl SymbolResolver for SymbolTable<'_> {
    fn resolve_name(&self, name: &str) -> Option<ResolvedSymbol> {
        if let Some(id) = self.entries.name_id(name) {
            Some(ResolvedSymbol::Entry(id))
        } else if self.runtime.has_constant(name) {
            Some(ResolvedSymbol::Constant(name.to_string()))
        } else {
            None
        }
    }
}
