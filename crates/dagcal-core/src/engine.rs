mod compiled;
mod dependencies;
mod entry;
mod recompute;
mod repository;
mod resolver;
mod results;
mod runtime;
mod symbol;
mod target;

use self::compiled::{CompiledEntry, CompiledEntryStore};
use self::dependencies::DependencyIndex;
use self::recompute::{EvaluationRunner, RecomputePlanner};
use self::repository::{EntryRecord, EntryRepository};
use self::resolver::resolve_expr;
use self::results::ResultCache;
use self::runtime::RuntimeEnvironment;
use self::symbol::SymbolTable;
pub use self::target::{EntryTarget, IntoEntryTarget};
use crate::ast::{ParsedExpr, ParsedStatement};
use crate::error::{DagcalError, EvalError, PersistenceError};
use crate::function::FunctionSignature;
use crate::id::ExpressionId;
use crate::number::Number;
use crate::parser::{parse_expression, parse_statement};
use crate::persistence::{ENGINE_SNAPSHOT_VERSION, EngineSnapshot, PersistedEntry};
use std::collections::{BTreeSet, HashSet};

pub use self::entry::{EntryRemoval, EntryState, EntryView, Execution};

/// Dependency-cycle information for the current engine state.
///
/// The engine stores dependencies internally by [`ExpressionId`], and cycle
/// diagnostics expose those IDs directly. Callers that need display labels can
/// format IDs as `$n` or consult [`Engine::entries`] for optional names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CycleDiagnostics {
    /// Strongly connected groups that currently form cycles.
    ///
    /// Each set contains the IDs for entries in one cycle.
    pub cycles: Vec<BTreeSet<ExpressionId>>,
    /// Every entry that is directly part of at least one cycle.
    pub cycle_nodes: BTreeSet<ExpressionId>,
    /// Entries that are not themselves cyclic but cannot be evaluated because
    /// they depend on a cyclic entry.
    pub dependent_nodes: BTreeSet<ExpressionId>,
}

struct Session {
    entries: EntryRepository,
    compiled: CompiledEntryStore,
    dependencies: DependencyIndex,
    results: ResultCache,
    runtime: RuntimeEnvironment,
}

impl Session {
    fn new() -> Self {
        Self {
            entries: EntryRepository::new(),
            compiled: CompiledEntryStore::new(),
            dependencies: DependencyIndex::new(),
            results: ResultCache::new(),
            runtime: RuntimeEnvironment::new(),
        }
    }

    fn resolve_expr(&self, expr: ParsedExpr) -> Result<crate::ast::ResolvedExpr, EvalError> {
        resolve_expr(&SymbolTable::new(&self.entries, &self.runtime), expr)
    }

    fn rebuild_dependencies(&mut self) {
        self.dependencies
            .rebuild(self.compiled.dependency_entries());
    }

    fn affected_by(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        RecomputePlanner::new(&self.dependencies).affected_by(id)
    }

    fn dependencies_of(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.dependencies.dependencies_of(id)
    }

    fn dependents_of(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.dependencies.dependents_of(id)
    }

    fn affected_by_any(
        &self,
        ids: impl IntoIterator<Item = ExpressionId>,
    ) -> BTreeSet<ExpressionId> {
        RecomputePlanner::new(&self.dependencies).affected_by_any(ids)
    }

    fn recompute_ids(&mut self, ids: BTreeSet<ExpressionId>) {
        let Self {
            compiled,
            dependencies,
            results,
            runtime,
            ..
        } = self;
        EvaluationRunner::new(dependencies, compiled, runtime).recompute_ids(ids, results);
    }

    fn state(&self, id: ExpressionId) -> Option<&EntryState> {
        self.results.get(id)
    }

    fn entry_view(&self, id: ExpressionId) -> Option<EntryView> {
        let record = self.entries.record(id)?;
        Some(EntryView {
            id: record.id,
            name: record.name.clone(),
            source: record.source.clone(),
            state: self.results.get(id)?.clone(),
        })
    }

    fn entries(&self) -> Vec<EntryView> {
        self.entries
            .records()
            .into_iter()
            .filter_map(|record| self.entry_view(record.id))
            .collect()
    }

    fn entry_ids(&self) -> Vec<ExpressionId> {
        self.entries.sorted_ids()
    }

    fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn entry_at_index(&self, index: usize) -> Option<EntryView> {
        self.entries
            .id_at_index(index)
            .and_then(|id| self.entry_view(id))
    }
}

/// Stateful calculation engine with dependency tracking and recomputation.
///
/// `Engine` is the main public API for a calculator session. It owns:
///
/// - an entry store containing source text, stable IDs, optional names, and
///   current [`EntryState`] values;
/// - an evaluation context containing constants and functions;
/// - a dependency graph used to recompute only entries affected by changes.
///
/// Entries are identified by stable 1-based [`ExpressionId`] values displayed
/// as `$1`, `$2`, and so on. Named definitions such as `tax = subtotal * 0.1`
/// can be referenced by name in later expressions and addressed by name in
/// convenience APIs. Internally, references and recomputation use expression
/// IDs. References are resolved to entry IDs when an expression is saved, so
/// removing a name does not silently rebind existing expressions to a constant
/// or later entry with the same name.
///
/// Failed entries are part of normal engine state. Syntax errors, unknown
/// references, unknown functions, cycles, division by zero, and other
/// evaluation failures are stored as [`EntryState::Error`]. Dependents of a
/// failed entry receive dependency errors until the failing entry is repaired.
///
/// # Recalculation model
///
/// Editing an entry rebuilds the dependency graph and recomputes the edited
/// entry plus transitive dependents. Removing an entry recomputes entries that
/// previously depended on it. Updating a constant or registering/replacing a
/// function recomputes entries that referenced that symbol. Entries outside the
/// affected set keep their previous state.
///
/// # Examples
///
/// ```
/// use dagcal_core::{Engine, EntryState, Number};
///
/// let mut engine = Engine::new();
/// let subtotal = engine.execute("subtotal = 100").id;
/// let tax = engine.execute("tax = subtotal * 0.1").id;
///
/// assert_eq!(engine.state("tax"), Some(&EntryState::Value(Number::from(10.0))));
///
/// engine.set_entry("subtotal", "200").unwrap();
/// assert_eq!(engine.state(tax), Some(&EntryState::Value(Number::from(20.0))));
/// ```
pub struct Engine {
    session: Session,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Creates an empty engine with the standard function registry and default
    /// constants.
    ///
    /// The first plain expression or named definition saved into this engine
    /// receives ID `$1`.
    pub fn new() -> Self {
        Self {
            session: Session::new(),
        }
    }

    /// Captures the persistent session data needed to rebuild this engine.
    ///
    /// Snapshots contain entry IDs, optional names, original source text, and
    /// the current snapshot format version. They intentionally do not store
    /// computed values or dependency graph internals; those are rebuilt by
    /// [`Engine::restore_snapshot`] or [`Engine::from_snapshot`].
    ///
    /// Runtime constants and user-registered functions are not included in the
    /// snapshot. Restore into an engine configured with the same runtime
    /// extensions when persisted expressions depend on them.
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot::new(
            self.entries()
                .into_iter()
                .map(|entry| PersistedEntry {
                    id: entry.id.value(),
                    name: entry.name,
                    source: entry.source,
                })
                .collect(),
        )
    }

    /// Builds a new engine from a previously captured snapshot.
    ///
    /// This is equivalent to creating [`Engine::new`] and then calling
    /// [`Engine::restore_snapshot`]. Snapshot validation happens before the
    /// returned engine is exposed.
    pub fn from_snapshot(snapshot: EngineSnapshot) -> Result<Self, DagcalError> {
        let mut engine = Self::new();
        engine.restore_snapshot(snapshot)?;
        Ok(engine)
    }

    /// Replaces this engine's entries with data from a snapshot.
    ///
    /// The snapshot is first validated for version compatibility, nonzero
    /// unique IDs, valid unique names, and then restored into a temporary
    /// engine. This receiver is replaced only after restoration succeeds, so a
    /// validation failure leaves the current engine unchanged.
    ///
    /// Restored expressions are parsed and resolved in ID order after all IDs
    /// and names are reserved. This lets references between persisted entries
    /// resolve even when the referenced entry appears later in the snapshot.
    /// Values and errors are recomputed from source text after the dependency
    /// graph is rebuilt.
    pub fn restore_snapshot(&mut self, snapshot: EngineSnapshot) -> Result<(), DagcalError> {
        let entries = validate_snapshot(snapshot)?;
        let mut restored = Self::new();

        for entry in &entries {
            restored
                .session
                .entries
                .reserve_restored_entry(ExpressionId::new(entry.id), entry.name.as_deref());
        }

        for entry in entries {
            let id = ExpressionId::new(entry.id);
            let record = EntryRecord::new(id, entry.name, entry.source.clone());
            let compiled = match parse_expression(&entry.source) {
                Ok(ast) => match restored.session.resolve_expr(ast) {
                    Ok(ast) => CompiledEntry::from_resolved(ast),
                    Err(err) => CompiledEntry::from_error(DagcalError::Eval(err)),
                },
                Err(err) => CompiledEntry::from_error(err),
            };
            restored.session.entries.upsert(record);
            restored.session.compiled.insert(id, compiled);
        }

        restored.session.rebuild_dependencies();
        let ids = restored.session.entries.ids().collect();
        restored.session.recompute_ids(ids);

        *self = restored;
        Ok(())
    }

    /// Executes user input as either a named definition (`name = expr`) or a
    /// plain expression.
    ///
    /// Named definitions update or create the named entry. The stored source for
    /// a definition is the expression on the right side of `=`, not the complete
    /// input line. Plain expressions and statement-level parse errors are
    /// appended as the next available `$n` result entry.
    ///
    /// A statement-level parse error is saved as an unnamed error entry so the
    /// original input can be edited later.
    pub fn execute(&mut self, input: &str) -> Execution {
        match parse_statement(input) {
            Ok(ParsedStatement::Definition { name, expr }) => {
                let source = definition_source(input, &name);
                let (id, name) = self.session.entries.resolve_or_create_name(name);
                self.save_parsed_entry(id, name, source, expr)
            }
            Ok(ParsedStatement::Expression(expr)) => {
                let id = self.session.entries.allocate_id();
                self.save_parsed_entry(id, None, input.trim().to_string(), expr)
            }
            Err(err) => {
                let id = self.session.entries.allocate_id();
                self.save_parse_error(id, None, input.trim().to_string(), err)
            }
        }
    }

    fn register_function<F>(
        &mut self,
        name: impl Into<String>,
        signature: FunctionSignature,
        body: F,
    ) where
        F: Fn(&[Number]) -> Result<Number, EvalError> + Send + Sync + 'static,
    {
        let name = name.into();
        self.session
            .runtime
            .register_function(name.clone(), signature, body);
        self.recompute_function_references(&name);
    }

    /// Registers or replaces a fixed-arity function and recomputes entries that
    /// reference it.
    ///
    /// The function receives evaluated argument values as an `&[Number]` whose
    /// length is exactly `arity`. Return [`EvalError`] to surface a structured
    /// evaluation failure. Returning non-finite float values is normalized by the
    /// evaluator into [`EvalError::Math`].
    pub fn register_fixed_function<F>(&mut self, name: impl Into<String>, arity: usize, body: F)
    where
        F: Fn(&[Number]) -> Result<Number, EvalError> + Send + Sync + 'static,
    {
        self.register_function(name, FunctionSignature::exact(arity), body);
    }

    /// Registers or replaces a variadic function and recomputes entries that
    /// reference it.
    ///
    /// The function accepts at least `min` evaluated arguments. Arity validation
    /// happens before `body` is called.
    pub fn register_variadic_function<F>(&mut self, name: impl Into<String>, min: usize, body: F)
    where
        F: Fn(&[Number]) -> Result<Number, EvalError> + Send + Sync + 'static,
    {
        self.register_function(name, FunctionSignature::variadic(min), body);
    }

    /// Sets or replaces a runtime constant and recomputes entries that
    /// reference it.
    ///
    /// Entries take precedence over constants when a name exists in both
    /// places. If `value` is non-finite, affected entries report
    /// [`EvalError::Math`] instead of producing `NaN` or infinity.
    pub fn set_constant(&mut self, name: impl Into<String>, value: impl Into<Number>) {
        let name = name.into();
        self.session.runtime.set_constant(name.clone(), value);
        self.recompute_constant_references(&name);
    }

    /// Sets or edits an entry by `$n` result reference, name, or stable ID.
    ///
    /// If the target exists, its source is replaced. If it does not exist, a
    /// new unnamed entry is created or a removed numbered result is restored.
    /// The saved entry is recomputed along with its transitive dependents.
    ///
    /// Unlike [`Engine::execute`], this method has an explicit target, so parse
    /// errors are stored on that target. The method returns `Ok` only when the
    /// saved target's final state is [`EntryState::Value`]. It returns `Err`
    /// when the target was saved as [`EntryState::Error`], but the errored entry
    /// remains in the engine for later inspection or repair.
    pub fn set_entry<T>(
        &mut self,
        target: T,
        source: impl Into<String>,
    ) -> Result<Execution, DagcalError>
    where
        T: IntoEntryTarget,
    {
        let (id, name) = self.resolve_or_create_target(target.into_entry_target()?);
        self.set_entry_for_id(id, name, source)
    }

    /// Sets or edits an entry by stable expression ID.
    pub fn set_entry_by_id(
        &mut self,
        id: ExpressionId,
        source: impl Into<String>,
    ) -> Result<Execution, DagcalError> {
        let name = self.session.entries.name_for_id(id).cloned();
        self.session.entries.reserve_id(id);
        self.set_entry_for_id(id, name, source)
    }

    fn set_entry_for_id(
        &mut self,
        id: ExpressionId,
        name: Option<String>,
        source: impl Into<String>,
    ) -> Result<Execution, DagcalError> {
        let source = source.into();
        let execution = match parse_expression(&source) {
            Ok(ast) => self.save_parsed_entry(id, name, source, ast),
            Err(err) => self.save_parse_error(id, name, source, err),
        };

        match &execution.state {
            EntryState::Value(_) => Ok(execution),
            EntryState::Error(err) => Err(err.clone()),
        }
    }

    /// Removes an entry by `$n` result reference, name, or stable ID.
    ///
    /// Returns the removed entry and recomputation report when the target existed.
    /// Entries that depended on the removed ID are recomputed and typically
    /// become unknown-reference or dependency errors. Removing an entry does not
    /// renumber later `$n` results; future plain expressions continue from the
    /// highest allocated ID.
    pub fn remove_entry<T>(&mut self, target: T) -> Option<EntryRemoval>
    where
        T: IntoEntryTarget,
    {
        let id = self.resolve_existing_target(target.into_entry_target().ok()?)?;
        self.remove_entry_by_id(id)
    }

    /// Removes an entry by stable expression ID.
    pub fn remove_entry_by_id(&mut self, id: ExpressionId) -> Option<EntryRemoval> {
        let removed_entry = self.session.entry_view(id)?;
        let affected = self.session.affected_by(id);
        self.session.entries.remove(id)?;
        self.session.compiled.remove(id);
        self.session.results.remove(id);
        self.session.rebuild_dependencies();
        self.session.recompute_ids(affected.clone());
        Some(EntryRemoval {
            removed_entry,
            affected_ids: affected,
        })
    }

    /// Returns the current state for an entry by `$n`, name, or stable ID.
    ///
    /// Returns `None` when no matching entry exists.
    pub fn state<T>(&self, target: T) -> Option<&EntryState>
    where
        T: IntoEntryTarget,
    {
        let id = self.resolve_existing_target(target.into_entry_target().ok()?)?;
        self.state_by_id(id)
    }

    /// Returns the current state for an entry by its stable ID.
    pub fn state_by_id(&self, id: ExpressionId) -> Option<&EntryState> {
        self.session.state(id)
    }

    /// Returns a renderable view of an entry by `$n`, name, or stable ID.
    ///
    /// The returned [`EntryView`] is an owned snapshot of metadata and state, so
    /// callers can keep it after later engine mutations.
    pub fn entry<T>(&self, target: T) -> Option<EntryView>
    where
        T: IntoEntryTarget,
    {
        let id = self.resolve_existing_target(target.into_entry_target().ok()?)?;
        self.entry_by_id(id)
    }

    /// Returns a renderable view of an entry by its stable ID.
    pub fn entry_by_id(&self, id: ExpressionId) -> Option<EntryView> {
        self.session.entry_view(id)
    }

    /// Returns all stored entries sorted by their stable ID.
    ///
    /// Removed entries are not included. The returned views are owned snapshots
    /// of the engine state at the time of the call.
    ///
    /// This method uses `clone` to return a vector of owned [`EntryView`] values. Callers that need to iterate over entries without cloning can use [`Engine::entry_ids`] and [`Engine::entry_by_id`].
    pub fn entries(&self) -> Vec<EntryView> {
        self.session.entries()
    }

    /// Returns all stored entry IDs sorted by stable ID.
    pub fn entry_ids(&self) -> Vec<ExpressionId> {
        self.session.entry_ids()
    }

    /// Returns the number of stored entries.
    pub fn entry_count(&self) -> usize {
        self.session.entry_count()
    }

    /// Returns a renderable view for the entry at a sorted display index.
    pub fn entry_at_index(&self, index: usize) -> Option<EntryView> {
        self.session.entry_at_index(index)
    }

    /// Returns the entries that the given entry directly depends on.
    pub fn dependencies_of(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.session.dependencies_of(id)
    }

    /// Returns transitive dependents of the given entry.
    pub fn dependents_of(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.session.dependents_of(id)
    }

    /// Returns the given entry plus all transitive dependents that would be
    /// recomputed after editing it.
    pub fn affected_by(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.session.affected_by(id)
    }

    /// Returns the current dependency-cycle report using expression IDs.
    ///
    /// Cycle diagnostics are derived from the latest dependency graph. Callers
    /// can use this alongside [`Engine::entries`] to highlight cyclic entries
    /// and entries blocked by cycles.
    pub fn cycle_diagnostics(&self) -> CycleDiagnostics {
        let report = self.session.dependencies.cycle_report();

        CycleDiagnostics {
            cycles: report.cycles,
            cycle_nodes: report.cycle_nodes,
            dependent_nodes: report.dependent_nodes,
        }
    }

    /// Parses and evaluates a source expression without storing it.
    ///
    /// This uses the current entries, constants, and function registry for
    /// reference resolution, but it does not allocate an ID, change dependency
    /// tracking, or recompute stored entries.
    pub fn eval_once(&self, source: &str) -> Result<Number, DagcalError> {
        let ast = self
            .session
            .resolve_expr(parse_expression(source)?)
            .map_err(DagcalError::Eval)?;
        self::recompute::eval_once(&ast, &self.session.runtime, &self.session.results)
            .map_err(DagcalError::Eval)
    }

    fn recompute_constant_references(&mut self, name: &str) {
        let roots = self.session.compiled.ids_referencing_constant(name);
        let affected = self.session.affected_by_any(roots);
        self.session.recompute_ids(affected);
    }

    fn recompute_function_references(&mut self, name: &str) {
        let roots = self.session.compiled.ids_referencing_function(name);
        let affected = self.session.affected_by_any(roots);
        self.session.recompute_ids(affected);
    }

    fn save_parsed_entry(
        &mut self,
        id: ExpressionId,
        name: Option<String>,
        source: String,
        ast: ParsedExpr,
    ) -> Execution {
        let compiled = match self.session.resolve_expr(ast) {
            Ok(ast) => CompiledEntry::from_resolved(ast),
            Err(err) => CompiledEntry::from_error(DagcalError::Eval(err)),
        };
        self.save_entry(id, name, source, compiled)
    }

    fn save_parse_error(
        &mut self,
        id: ExpressionId,
        name: Option<String>,
        source: String,
        err: DagcalError,
    ) -> Execution {
        self.save_entry(id, name, source, CompiledEntry::from_error(err))
    }

    fn save_entry(
        &mut self,
        id: ExpressionId,
        name: Option<String>,
        source: String,
        compiled: CompiledEntry,
    ) -> Execution {
        self.session
            .entries
            .upsert(EntryRecord::new(id, name, source));
        self.session.compiled.insert(id, compiled);
        self.session.rebuild_dependencies();
        let affected_ids = self.session.affected_by(id);
        self.session.recompute_ids(affected_ids.clone());

        Execution {
            id,
            state: self
                .session
                .state(id)
                .expect("saved entry should exist")
                .clone(),
            affected_ids,
        }
    }

    fn resolve_or_create_target(&mut self, target: EntryTarget) -> (ExpressionId, Option<String>) {
        match target {
            EntryTarget::Id(id) => {
                let name = self.session.entries.name_for_id(id).cloned();
                self.session.entries.reserve_id(id);
                (id, name)
            }
            EntryTarget::Name(name) => self.session.entries.resolve_or_create_name(name),
        }
    }

    fn resolve_existing_target(&self, target: EntryTarget) -> Option<ExpressionId> {
        match target {
            EntryTarget::Id(id) => Some(id),
            EntryTarget::Name(name) => self.session.entries.name_id(&name),
        }
    }
}

fn definition_source(input: &str, name: &str) -> String {
    let Some((left, right)) = input.split_once('=') else {
        return input.trim().to_string();
    };

    if left.trim() == name {
        right.trim().to_string()
    } else {
        input.trim().to_string()
    }
}

fn validate_snapshot(snapshot: EngineSnapshot) -> Result<Vec<PersistedEntry>, DagcalError> {
    if snapshot.version != ENGINE_SNAPSHOT_VERSION {
        return Err(DagcalError::Persistence(
            PersistenceError::UnsupportedVersion {
                actual: snapshot.version,
                expected: ENGINE_SNAPSHOT_VERSION,
            },
        ));
    }

    let mut ids = HashSet::new();
    let mut names = HashSet::new();

    for entry in &snapshot.entries {
        if entry.id == 0 {
            return Err(DagcalError::Persistence(PersistenceError::InvalidId(
                entry.id,
            )));
        }
        if !ids.insert(entry.id) {
            return Err(DagcalError::Persistence(PersistenceError::DuplicateId(
                entry.id,
            )));
        }
        if let Some(name) = &entry.name {
            if !crate::parser::is_valid_name(name) {
                return Err(DagcalError::Persistence(PersistenceError::InvalidName(
                    name.clone(),
                )));
            }
            if !names.insert(name.clone()) {
                return Err(DagcalError::Persistence(PersistenceError::DuplicateName(
                    name.clone(),
                )));
            }
        }
    }

    let mut entries = snapshot.entries;
    entries.sort_by_key(|entry| entry.id);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ResolvedExpr;
    use crate::error::ReferenceTarget;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn set_entry(
        engine: &mut Engine,
        target: &str,
        source: impl Into<String>,
    ) -> Result<Execution, DagcalError> {
        let source = source.into();
        let id = if let Some(id) = label_id(engine, target) {
            id
        } else {
            let execution = engine.execute(&format!("{target} = 0"));
            execution.id
        };
        Engine::set_entry(engine, id, source)
    }

    fn remove_entry(engine: &mut Engine, target: &str) -> Option<EntryRemoval> {
        let id = label_id(engine, target)?;
        Engine::remove_entry(engine, id)
    }

    fn state<'a>(engine: &'a Engine, target: &str) -> Option<&'a EntryState> {
        let id = label_id(engine, target)?;
        Engine::state(engine, id)
    }

    fn entry(engine: &Engine, target: &str) -> Option<EntryView> {
        let id = label_id(engine, target)?;
        Engine::entry(engine, id)
    }

    fn label_id(engine: &Engine, target: &str) -> Option<ExpressionId> {
        if let Some(digits) = target.strip_prefix('$') {
            let value = digits.parse::<usize>().ok()?;
            return (value > 0).then(|| ExpressionId::new(value));
        }

        engine
            .entries()
            .into_iter()
            .find(|entry| entry.name.as_deref() == Some(target))
            .map(|entry| entry.id)
    }

    fn assert_value(engine: &Engine, id: &str, expected: f64) {
        match state(engine, id) {
            Some(EntryState::Value(actual)) => {
                assert!((actual.to_f64() - expected).abs() < 1e-12)
            }
            other => panic!("expected value for {id}, got {other:?}"),
        }
    }

    fn assert_eval_error(engine: &Engine, id: &str, matches: impl FnOnce(&EvalError) -> bool) {
        match state(engine, id) {
            Some(EntryState::Error(DagcalError::Eval(err))) if matches(err) => {}
            other => panic!("expected eval error for {id}, got {other:?}"),
        }
    }

    fn execution_id_display(execution: &Execution) -> String {
        execution.id.to_string()
    }

    #[test]
    fn updates_dependents_when_source_changes() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "1 + 2").unwrap();
        set_entry(&mut engine, "b", "a * 2").unwrap();
        set_entry(&mut engine, "c", "b + 1").unwrap();
        assert_value(&engine, "c", 7.0);

        set_entry(&mut engine, "a", "10").unwrap();
        assert_value(&engine, "b", 20.0);
        assert_value(&engine, "c", 21.0);
    }

    #[test]
    fn user_entries_override_constants() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "pi", "3").unwrap();
        set_entry(&mut engine, "x", "pi + 1").unwrap();

        assert_value(&engine, "x", 4.0);
    }

    #[test]
    fn removing_entry_recomputes_dependents_as_errors() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "2").unwrap();
        set_entry(&mut engine, "b", "a + 3").unwrap();
        assert_value(&engine, "b", 5.0);

        remove_entry(&mut engine, "a");

        assert_eval_error(
            &engine,
            "b",
            |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(id)) if *id == ExpressionId::new(1)),
        );
    }

    #[test]
    fn removing_shadowing_entry_leaves_dependents_bound_to_removed_id() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "pi", "3").unwrap();
        set_entry(&mut engine, "x", "pi + 1").unwrap();
        assert_value(&engine, "x", 4.0);

        remove_entry(&mut engine, "pi");

        assert_eval_error(
            &engine,
            "x",
            |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(id)) if *id == ExpressionId::new(1)),
        );
    }

    #[test]
    fn parse_errors_propagate_and_recover_after_edit() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "1").unwrap();
        set_entry(&mut engine, "b", "a + 2").unwrap();
        assert!(set_entry(&mut engine, "a", "1 +").is_err());

        assert_eval_error(
            &engine,
            "b",
            |err| matches!(err, EvalError::DependencyError(id) if *id == ExpressionId::new(1)),
        );

        set_entry(&mut engine, "a", "10").unwrap();
        assert_value(&engine, "b", 12.0);
    }

    #[test]
    fn changing_dependencies_drops_old_reverse_dependency() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "1").unwrap();
        set_entry(&mut engine, "b", "a + 1").unwrap();
        assert_value(&engine, "b", 2.0);

        set_entry(&mut engine, "b", "100").unwrap();
        set_entry(&mut engine, "a", "10").unwrap();

        assert_value(&engine, "b", 100.0);
    }

    #[test]
    fn recomputes_branching_graph_through_errors_and_recovery() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "price", "10").unwrap();
        set_entry(&mut engine, "quantity", "3").unwrap();
        set_entry(&mut engine, "discount", "2").unwrap();
        set_entry(&mut engine, "gross", "price * quantity").unwrap();
        set_entry(&mut engine, "net", "gross - discount").unwrap();
        set_entry(&mut engine, "fee", "price / (quantity - 1)").unwrap();
        set_entry(&mut engine, "summary", "net + fee + sin(pi / 2)").unwrap();

        assert_value(&engine, "gross", 30.0);
        assert_value(&engine, "net", 28.0);
        assert_value(&engine, "fee", 5.0);
        assert_value(&engine, "summary", 34.0);

        set_entry(&mut engine, "price", "20").unwrap();

        assert_value(&engine, "gross", 60.0);
        assert_value(&engine, "net", 58.0);
        assert_value(&engine, "fee", 10.0);
        assert_value(&engine, "summary", 69.0);

        remove_entry(&mut engine, "discount");

        assert_value(&engine, "gross", 60.0);
        assert_value(&engine, "fee", 10.0);
        assert_eval_error(
            &engine,
            "net",
            |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(id)) if *id == ExpressionId::new(3)),
        );
        assert_eval_error(
            &engine,
            "summary",
            |err| matches!(err, EvalError::DependencyError(id) if *id == ExpressionId::new(5)),
        );

        set_entry(&mut engine, "$3", "8").unwrap();

        assert_value(&engine, "net", 52.0);
        assert_value(&engine, "summary", 63.0);

        set_entry(&mut engine, "quantity", "1").unwrap();

        assert_value(&engine, "gross", 20.0);
        assert_value(&engine, "net", 12.0);
        assert_eval_error(&engine, "fee", |err| {
            matches!(err, EvalError::DivisionByZero)
        });
        assert_eval_error(
            &engine,
            "summary",
            |err| matches!(err, EvalError::DependencyError(id) if *id == ExpressionId::new(6)),
        );

        set_entry(&mut engine, "quantity", "4").unwrap();

        assert_value(&engine, "gross", 80.0);
        assert_value(&engine, "net", 72.0);
        assert_value(&engine, "fee", 20.0 / 3.0);
        assert_value(&engine, "summary", 72.0 + (20.0 / 3.0) + 1.0);
    }

    #[test]
    fn dependency_errors_propagate() {
        let mut engine = Engine::new();

        assert!(set_entry(&mut engine, "a", "missing + 1").is_err());
        assert!(set_entry(&mut engine, "b", "a * 2").is_err());

        assert!(matches!(
            state(&engine, "b"),
            Some(EntryState::Error(DagcalError::Eval(
                EvalError::DependencyError(id)
            ))) if *id == ExpressionId::new(1)
        ));
    }

    #[test]
    fn detects_cycles() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "1").unwrap();
        set_entry(&mut engine, "b", "2").unwrap();
        set_entry(&mut engine, "a", "b + 1").unwrap();
        assert!(set_entry(&mut engine, "b", "a + 1").is_err());

        assert!(matches!(
            state(&engine, "a"),
            Some(EntryState::Error(DagcalError::Eval(
                EvalError::CycleDetected(_)
            )))
        ));
    }

    #[test]
    fn self_reference_is_cycle() {
        let mut engine = Engine::new();

        assert!(set_entry(&mut engine, "a", "a + 1").is_err());

        assert_eval_error(
            &engine,
            "a",
            |err| matches!(err, EvalError::CycleDetected(id) if *id == ExpressionId::new(1)),
        );

        let diagnostics = engine.cycle_diagnostics();
        assert_eq!(
            diagnostics.cycles,
            vec![BTreeSet::from([ExpressionId::new(1)])]
        );
        assert_eq!(
            diagnostics.cycle_nodes,
            BTreeSet::from([ExpressionId::new(1)])
        );
        assert!(diagnostics.dependent_nodes.is_empty());
    }

    #[test]
    fn reports_cycle_nodes_and_all_dependents() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "1").unwrap();
        set_entry(&mut engine, "b", "2").unwrap();
        set_entry(&mut engine, "c", "3").unwrap();
        set_entry(&mut engine, "d", "4").unwrap();
        set_entry(&mut engine, "a", "b + 1").unwrap();
        assert!(set_entry(&mut engine, "b", "a + 1").is_err());
        assert!(set_entry(&mut engine, "c", "a + 1").is_err());
        assert!(set_entry(&mut engine, "d", "c + 1").is_err());
        set_entry(&mut engine, "ok", "10").unwrap();

        let diagnostics = engine.cycle_diagnostics();

        assert_eq!(
            diagnostics.cycles,
            vec![BTreeSet::from([ExpressionId::new(1), ExpressionId::new(2)])]
        );
        assert_eq!(
            diagnostics.cycle_nodes,
            BTreeSet::from([ExpressionId::new(1), ExpressionId::new(2)])
        );
        assert_eq!(
            diagnostics.dependent_nodes,
            BTreeSet::from([ExpressionId::new(3), ExpressionId::new(4)])
        );
        assert_eval_error(
            &engine,
            "c",
            |err| matches!(err, EvalError::DependencyError(id) if *id == ExpressionId::new(1)),
        );
        assert_eval_error(
            &engine,
            "d",
            |err| matches!(err, EvalError::DependencyError(id) if *id == ExpressionId::new(3)),
        );
        assert_value(&engine, "ok", 10.0);
    }

    #[test]
    fn reports_multiple_independent_cycles() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "1").unwrap();
        set_entry(&mut engine, "b", "2").unwrap();
        set_entry(&mut engine, "x", "3").unwrap();
        set_entry(&mut engine, "y", "4").unwrap();
        set_entry(&mut engine, "z", "5").unwrap();
        set_entry(&mut engine, "a", "b + 1").unwrap();
        assert!(set_entry(&mut engine, "b", "a + 1").is_err());
        set_entry(&mut engine, "x", "y + 1").unwrap();
        assert!(set_entry(&mut engine, "y", "x + 1").is_err());
        assert!(set_entry(&mut engine, "z", "x + a").is_err());

        let diagnostics = engine.cycle_diagnostics();

        assert_eq!(
            diagnostics.cycles,
            vec![
                BTreeSet::from([ExpressionId::new(1), ExpressionId::new(2)]),
                BTreeSet::from([ExpressionId::new(3), ExpressionId::new(4)])
            ]
        );
        assert_eq!(
            diagnostics.cycle_nodes,
            BTreeSet::from([
                ExpressionId::new(1),
                ExpressionId::new(2),
                ExpressionId::new(3),
                ExpressionId::new(4)
            ])
        );
        assert_eq!(
            diagnostics.dependent_nodes,
            BTreeSet::from([ExpressionId::new(5)])
        );
    }

    #[test]
    fn clearing_cycle_clears_diagnostics_and_recomputes_dependents() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "a", "1").unwrap();
        set_entry(&mut engine, "b", "2").unwrap();
        set_entry(&mut engine, "c", "3").unwrap();
        set_entry(&mut engine, "a", "b + 1").unwrap();
        assert!(set_entry(&mut engine, "b", "a + 1").is_err());
        assert!(set_entry(&mut engine, "c", "a + 1").is_err());
        assert!(!engine.cycle_diagnostics().cycle_nodes.is_empty());

        set_entry(&mut engine, "a", "1").unwrap();

        let diagnostics = engine.cycle_diagnostics();
        assert!(diagnostics.cycles.is_empty());
        assert!(diagnostics.cycle_nodes.is_empty());
        assert!(diagnostics.dependent_nodes.is_empty());
        assert_value(&engine, "b", 2.0);
        assert_value(&engine, "c", 2.0);
    }

    #[test]
    fn recomputes_acyclic_entries_in_dependency_order_when_cycles_exist() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "z", "1").unwrap();
        set_entry(&mut engine, "a", "z + 1").unwrap();
        set_entry(&mut engine, "cycle_left", "1").unwrap();
        set_entry(&mut engine, "cycle_right", "2").unwrap();
        set_entry(&mut engine, "cycle_left", "cycle_right + 1").unwrap();
        assert!(set_entry(&mut engine, "cycle_right", "cycle_left + 1").is_err());

        set_entry(&mut engine, "z", "10").unwrap();

        assert_value(&engine, "z", 10.0);
        assert_value(&engine, "a", 11.0);
        assert_eval_error(
            &engine,
            "cycle_left",
            |err| matches!(err, EvalError::CycleDetected(id) if *id == ExpressionId::new(3)),
        );
        assert_eval_error(
            &engine,
            "cycle_right",
            |err| matches!(err, EvalError::CycleDetected(id) if *id == ExpressionId::new(4)),
        );
    }

    #[test]
    fn execute_defines_named_entries_and_appends_expressions() {
        let mut engine = Engine::new();

        let subtotal = engine.execute("subtotal = 100");
        let taxed = engine.execute("subtotal * 1.1");

        assert_eq!(execution_id_display(&subtotal), "$1");
        assert_eq!(
            subtotal.state,
            EntryState::Value(crate::number::Number::from(100.0))
        );
        assert_eq!(execution_id_display(&taxed), "$2");
        assert_eq!(
            taxed.state,
            EntryState::Value(crate::number::Number::from(110.0))
        );
        assert_eq!(entry(&engine, "subtotal").unwrap().source, "100");
        assert_eq!(entry(&engine, "$1").unwrap().source, "100");
        assert_eq!(entry(&engine, "$2").unwrap().source, "subtotal * 1.1");
    }

    #[test]
    fn execute_saves_statement_parse_errors_as_numbered_entries() {
        let mut engine = Engine::new();

        let execution = engine.execute("$1 = 100");

        assert_eq!(execution_id_display(&execution), "$1");
        assert!(matches!(
            execution.state,
            EntryState::Error(DagcalError::Parse(_))
        ));
        let entry = entry(&engine, "$1").expect("parse error should be saved");
        assert_eq!(entry.source, "$1 = 100");
        assert!(matches!(
            entry.state,
            EntryState::Error(DagcalError::Parse(_))
        ));
    }

    #[test]
    fn saved_statement_parse_errors_recover_after_edit() {
        let mut engine = Engine::new();

        let execution = engine.execute("1 +");

        assert_eq!(execution_id_display(&execution), "$1");
        assert!(matches!(
            execution.state,
            EntryState::Error(DagcalError::Parse(_))
        ));
        assert_eq!(entry(&engine, "$1").unwrap().source, "1 +");

        set_entry(&mut engine, "$1", "100").unwrap();

        assert_value(&engine, "$1", 100.0);
    }

    #[test]
    fn appends_numbered_results_and_references_them_with_dollar_syntax() {
        let mut engine = Engine::new();

        let first = engine.execute("1 + 2");
        let second = engine.execute("$1 * 10");

        assert_eq!(execution_id_display(&first), "$1");
        assert_eq!(execution_id_display(&second), "$2");
        assert_eq!(
            first.state,
            EntryState::Value(crate::number::Number::from(3.0))
        );
        assert_eq!(
            second.state,
            EntryState::Value(crate::number::Number::from(30.0))
        );
        assert_value(&engine, "$2", 30.0);
    }

    #[test]
    fn editing_numbered_result_updates_dollar_dependents() {
        let mut engine = Engine::new();

        engine.execute("2");
        engine.execute("$1 + 3");
        assert_value(&engine, "$2", 5.0);

        set_entry(&mut engine, "$1", "10").unwrap();

        assert_value(&engine, "$2", 13.0);
    }

    #[test]
    fn numbered_results_recompute_branching_graph_through_removal_and_reuse() {
        let mut engine = Engine::new();

        let first = engine.execute("2");
        let second = engine.execute("$1 + 3");
        let third = engine.execute("$1 * $2");
        let fourth = engine.execute("$2 + $3 + sin(pi / 2)");
        let fifth = engine.execute("$4 / ($2 - 5)");

        assert_eq!(execution_id_display(&first), "$1");
        assert_eq!(execution_id_display(&second), "$2");
        assert_eq!(execution_id_display(&third), "$3");
        assert_eq!(execution_id_display(&fourth), "$4");
        assert_eq!(execution_id_display(&fifth), "$5");
        assert_value(&engine, "$2", 5.0);
        assert_value(&engine, "$3", 10.0);
        assert_value(&engine, "$4", 16.0);
        assert_eval_error(&engine, "$5", |err| {
            matches!(err, EvalError::DivisionByZero)
        });

        set_entry(&mut engine, "$1", "4").unwrap();

        assert_value(&engine, "$2", 7.0);
        assert_value(&engine, "$3", 28.0);
        assert_value(&engine, "$4", 36.0);
        assert_value(&engine, "$5", 18.0);

        remove_entry(&mut engine, "$2");

        assert_value(&engine, "$1", 4.0);
        assert_eval_error(
            &engine,
            "$3",
            |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(id)) if *id == ExpressionId::new(2)),
        );
        assert_eval_error(
            &engine,
            "$4",
            |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(id)) if *id == ExpressionId::new(2)),
        );
        assert_eval_error(
            &engine,
            "$5",
            |err| matches!(err, EvalError::DependencyError(id) if *id == ExpressionId::new(4)),
        );

        set_entry(&mut engine, "$2", "$1 + 6").unwrap();

        assert_value(&engine, "$2", 10.0);
        assert_value(&engine, "$3", 40.0);
        assert_value(&engine, "$4", 51.0);
        assert_value(&engine, "$5", 51.0 / 5.0);

        let sixth = engine.execute("$5 + $3");

        assert_eq!(execution_id_display(&sixth), "$6");
        assert_eq!(
            sixth.state,
            EntryState::Value(crate::number::Number::from(50.2))
        );
        assert_value(&engine, "$6", 50.2);
    }

    #[test]
    fn removed_numbered_result_stays_unknown_for_later_entries() {
        let mut engine = Engine::new();

        engine.execute("2");
        remove_entry(&mut engine, "$1");
        let later = engine.execute("$1 + 1");

        assert!(matches!(
            later.state,
            EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(ReferenceTarget::Id(id))))
                if id == ExpressionId::new(1)
        ));
    }

    #[test]
    fn appending_skips_existing_numbered_results() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "$1", "100").unwrap();
        let execution = engine.execute("$1 + 1");

        assert_eq!(execution_id_display(&execution), "$2");
        assert_eq!(
            execution.state,
            EntryState::Value(crate::number::Number::from(101.0))
        );
    }

    #[test]
    fn explicit_numbered_result_reserves_sequential_expression_id() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "$5", "100").unwrap();
        let execution = engine.execute("$5 + 1");

        assert_eq!(execution_id_display(&execution), "$6");
        assert_eq!(
            execution.state,
            EntryState::Value(crate::number::Number::from(101.0))
        );
    }

    #[test]
    fn exposes_entries_by_internal_expression_id() {
        let mut engine = Engine::new();

        let execution = engine.execute("40 + 2");
        let id_display = execution_id_display(&execution);
        let entry = entry(&engine, &id_display).unwrap();
        let id = entry.id;

        assert_eq!(
            Engine::state(&engine, id),
            Some(&EntryState::Value(crate::number::Number::from(42.0)))
        );
        assert_eq!(Engine::entry(&engine, id).unwrap().id.to_string(), "$1");
    }

    #[test]
    fn named_labels_resolve_to_internal_expression_ids() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "subtotal", "100").unwrap();
        let id = entry(&engine, "subtotal").unwrap().id;
        set_entry(&mut engine, "taxed", "subtotal * 1.1").unwrap();
        set_entry(&mut engine, "subtotal", "200").unwrap();

        assert_eq!(entry(&engine, "subtotal").unwrap().id, id);
        assert_value(&engine, "taxed", 220.00000000000003);
    }

    #[test]
    fn stored_ast_uses_resolved_expression_ids_for_named_references() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "subtotal", "100").unwrap();
        let subtotal_id = entry(&engine, "subtotal").unwrap().id;
        set_entry(&mut engine, "taxed", "subtotal * 1.1").unwrap();

        let taxed_id = entry(&engine, "taxed").unwrap().id;
        let taxed = engine.session.compiled.get(taxed_id).unwrap();

        assert_eq!(
            taxed.analysis().entry_references,
            BTreeSet::from([subtotal_id])
        );
        assert!(matches!(
            taxed.expr(),
            Some(ResolvedExpr::Binary { lhs, .. })
                if matches!(**lhs, ResolvedExpr::EntryReference(id) if id == subtotal_id)
        ));
    }

    #[test]
    fn dollar_references_target_expression_ids_for_named_entries() {
        let mut engine = Engine::new();

        engine.execute("subtotal = 100");
        engine.execute("tax_rate = 0.1");
        engine.execute("subtotal * $2");

        assert_value(&engine, "$1", 100.0);
        assert_value(&engine, "$2", 0.1);
        assert_value(&engine, "$3", 10.0);
    }

    #[test]
    fn exposes_entries_for_read_only_listing() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "subtotal", "100").unwrap();
        engine.execute("subtotal * 1.1");

        let entries = engine
            .entries()
            .into_iter()
            .map(|entry| {
                (
                    entry.id.to_string(),
                    entry.source.clone(),
                    entry.state.clone(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "$1");
        assert_eq!(entries[0].1, "100");
        assert_eq!(
            entries[0].2,
            EntryState::Value(crate::number::Number::from(100.0))
        );
        assert_eq!(entries[1].0, "$2");
        assert_eq!(entries[1].1, "subtotal * 1.1");
        assert_eq!(
            entries[1].2,
            EntryState::Value(crate::number::Number::from(110.0))
        );
    }

    #[test]
    fn eval_once_can_reference_registered_entries() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "subtotal", "40").unwrap();

        assert_eq!(
            engine.eval_once("subtotal * 1.1").unwrap(),
            crate::number::Number::from(44.0)
        );
    }

    #[test]
    fn registering_function_recomputes_existing_entries() {
        let mut engine = Engine::new();

        assert!(set_entry(&mut engine, "x", "triple(14)").is_err());
        engine.register_fixed_function("triple", 1, |args| {
            Ok(args[0].clone() * crate::number::Number::from(3))
        });

        assert_eq!(
            state(&engine, "x"),
            Some(&EntryState::Value(crate::number::Number::from(42.0)))
        );
    }

    #[test]
    fn registering_function_recomputes_only_function_dependents() {
        let mut engine = Engine::new();
        let probe_calls = Arc::new(AtomicUsize::new(0));
        let probe_calls_for_body = Arc::clone(&probe_calls);

        engine.register_fixed_function("probe", 1, move |args| {
            probe_calls_for_body.fetch_add(1, Ordering::SeqCst);
            Ok(args[0].clone())
        });
        set_entry(&mut engine, "unrelated", "probe(5)").unwrap();
        assert!(set_entry(&mut engine, "x", "triple(14)").is_err());
        assert!(set_entry(&mut engine, "y", "x + 1").is_err());
        assert_eq!(probe_calls.load(Ordering::SeqCst), 1);

        engine.register_fixed_function("triple", 1, |args| {
            Ok(args[0].clone() * crate::number::Number::from(3))
        });

        assert_eq!(
            state(&engine, "x"),
            Some(&EntryState::Value(crate::number::Number::from(42.0)))
        );
        assert_eq!(
            state(&engine, "y"),
            Some(&EntryState::Value(crate::number::Number::from(43.0)))
        );
        assert_eq!(
            state(&engine, "unrelated"),
            Some(&EntryState::Value(crate::number::Number::from(5.0)))
        );
        assert_eq!(probe_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registering_variadic_function_recomputes_existing_entries() {
        let mut engine = Engine::new();

        assert!(set_entry(&mut engine, "x", "product(2, 3, 4)").is_err());
        engine.register_variadic_function("product", 0, |args| {
            Ok(args
                .iter()
                .cloned()
                .fold(crate::number::Number::from(1), |product, value| {
                    product * value
                }))
        });

        assert_eq!(
            state(&engine, "x"),
            Some(&EntryState::Value(crate::number::Number::from(24.0)))
        );
    }

    #[test]
    fn setting_constant_recomputes_existing_entries() {
        let mut engine = Engine::new();

        set_entry(&mut engine, "radius", "2").unwrap();
        engine.set_constant("tau", 6.0);
        set_entry(&mut engine, "area", "tau * radius ^ 2 / 2").unwrap();
        engine.set_constant("tau", std::f64::consts::TAU);

        assert_eq!(
            state(&engine, "area"),
            Some(&EntryState::Value(crate::number::Number::from(
                std::f64::consts::TAU * 2.0
            )))
        );
    }

    #[test]
    fn setting_constant_recomputes_only_constant_dependents() {
        let mut engine = Engine::new();
        let probe_calls = Arc::new(AtomicUsize::new(0));
        let probe_calls_for_body = Arc::clone(&probe_calls);

        engine.register_fixed_function("probe", 1, move |args| {
            probe_calls_for_body.fetch_add(1, Ordering::SeqCst);
            Ok(args[0].clone())
        });
        set_entry(&mut engine, "unrelated", "probe(5)").unwrap();
        set_entry(&mut engine, "radius", "2").unwrap();
        engine.set_constant("tau", 6.0);
        set_entry(&mut engine, "area", "tau * radius ^ 2 / 2").unwrap();
        set_entry(&mut engine, "scaled", "area + 1").unwrap();
        assert_eq!(probe_calls.load(Ordering::SeqCst), 1);

        engine.set_constant("tau", std::f64::consts::TAU);

        assert_eq!(
            state(&engine, "area"),
            Some(&EntryState::Value(crate::number::Number::from(
                std::f64::consts::TAU * 2.0
            )))
        );
        assert_eq!(
            state(&engine, "scaled"),
            Some(&EntryState::Value(crate::number::Number::from(
                std::f64::consts::TAU * 2.0 + 1.0
            )))
        );
        assert_eq!(
            state(&engine, "unrelated"),
            Some(&EntryState::Value(crate::number::Number::from(5.0)))
        );
        assert_eq!(probe_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn unresolved_names_do_not_recover_when_name_is_defined_later() {
        let mut engine = Engine::new();

        assert!(set_entry(&mut engine, "area", "tau * 2").is_err());
        engine.set_constant("tau", std::f64::consts::TAU);

        assert_eval_error(
            &engine,
            "area",
            |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Name(name)) if name == "tau"),
        );
    }
}
