use crate::ast::{ParsedExpr, ParsedReference, ParsedStatement, ResolvedExpr};
use crate::dependency_graph::ReferenceGraph;
use crate::error::{DagcalError, EvalError};
use crate::eval::eval_expr;
use crate::function::{FunctionRegistry, FunctionSignature};
use crate::id::{ExpressionId, ExpressionIdGenerator};
use crate::label::EntryLabel;
use crate::parser::{parse_expression, parse_statement};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub id: ExpressionId,
    pub label: EntryLabel,
    pub name: Option<String>,
    pub source: String,
    pub ast: Option<ResolvedExpr>,
    pub references: BTreeSet<ExpressionId>,
    pub state: EntryState,
}

impl Entry {
    fn from_resolved(
        id: ExpressionId,
        name: Option<String>,
        source: String,
        ast: ResolvedExpr,
    ) -> Self {
        let references = ast.references();
        Self {
            id,
            label: EntryLabel::result(id.value()),
            name,
            source,
            references,
            ast: Some(ast),
            state: EntryState::Error(DagcalError::Eval(EvalError::DependencyError(
                id.to_string(),
            ))),
        }
    }

    fn from_parse_error(
        id: ExpressionId,
        name: Option<String>,
        source: String,
        err: DagcalError,
    ) -> Self {
        Self {
            id,
            label: EntryLabel::result(id.value()),
            name,
            source,
            ast: None,
            references: BTreeSet::new(),
            state: EntryState::Error(err),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryView {
    pub id: ExpressionId,
    pub label: EntryLabel,
    pub name: Option<String>,
    pub source: String,
    pub state: EntryState,
}

impl From<&Entry> for EntryView {
    fn from(entry: &Entry) -> Self {
        Self {
            id: entry.id,
            label: entry.label.clone(),
            name: entry.name.clone(),
            source: entry.source.clone(),
            state: entry.state.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryState {
    Value(f64),
    Error(DagcalError),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CycleDiagnostics {
    pub cycles: Vec<BTreeSet<String>>,
    pub cycle_nodes: BTreeSet<String>,
    pub dependent_nodes: BTreeSet<String>,
}

#[derive(Debug, Clone)]
enum EntryTarget {
    Id(ExpressionId),
    Name(String),
}

impl EntryTarget {
    fn parse(input: &str) -> Result<Self, DagcalError> {
        match EntryLabel::parse(input)? {
            EntryLabel::Result(index) => Ok(Self::Id(ExpressionId::new(index))),
            EntryLabel::Named(name) => Ok(Self::Name(name)),
        }
    }
}

pub struct Engine {
    entries: HashMap<ExpressionId, Entry>,
    names: HashMap<String, ExpressionId>,
    constants: HashMap<String, f64>,
    functions: FunctionRegistry,
    dependency_graph: ReferenceGraph,
    id_generator: ExpressionIdGenerator,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            names: HashMap::new(),
            constants: HashMap::from([
                ("e".to_string(), std::f64::consts::E),
                ("pi".to_string(), std::f64::consts::PI),
            ]),
            functions: FunctionRegistry::standard(),
            dependency_graph: ReferenceGraph::new(),
            id_generator: ExpressionIdGenerator::new(),
        }
    }

    /// Executes user input as either a named definition (`name = expr`) or an expression.
    ///
    /// Named definitions update or create the named entry. Plain expressions are appended as the
    /// next available `$n` result entry.
    pub fn execute(&mut self, input: &str) -> Execution {
        match parse_statement(input) {
            Ok(ParsedStatement::Definition { name, expr }) => {
                let source = definition_source(input, &name);
                self.save_parsed_entry(EntryTarget::Name(name), source, expr)
            }
            Ok(ParsedStatement::Expression(expr)) => {
                let id = self.allocate_id();
                self.save_parsed_entry(EntryTarget::Id(id), input.trim().to_string(), expr)
            }
            Err(err) => Execution {
                id: None,
                label: None,
                state: EntryState::Error(err),
            },
        }
    }

    pub fn register_function<F>(
        &mut self,
        name: impl Into<String>,
        signature: FunctionSignature,
        body: F,
    ) where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        self.functions.register(name, signature, body);
        self.recompute_all();
    }

    pub fn register_fixed_function<F>(&mut self, name: impl Into<String>, arity: usize, body: F)
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        self.register_function(name, FunctionSignature::exact(arity), body);
    }

    pub fn register_variadic_function<F>(&mut self, name: impl Into<String>, min: usize, body: F)
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        self.register_function(name, FunctionSignature::variadic(min), body);
    }

    pub fn set_constant(&mut self, name: impl Into<String>, value: f64) {
        self.constants.insert(name.into(), value);
        self.recompute_all();
    }

    /// Sets or edits an entry by label.
    ///
    /// This is the explicit editing API for existing named entries and `$n` result entries.
    pub fn set_entry(
        &mut self,
        label: impl AsRef<str>,
        source: impl Into<String>,
    ) -> Result<EntryState, DagcalError> {
        let target = EntryTarget::parse(label.as_ref())?;
        let source = source.into();
        let execution = match parse_expression(&source) {
            Ok(ast) => self.save_parsed_entry(target, source, ast),
            Err(err) => self.save_parse_error(target, source, err),
        };

        match execution.state {
            EntryState::Value(_) => Ok(execution.state),
            EntryState::Error(err) => Err(err),
        }
    }

    pub fn remove_entry(&mut self, label: &str) -> Option<EntryView> {
        let target = EntryTarget::parse(label).ok()?;
        let id = self.id_for_target(&target)?;
        let affected = self.collect_affected(id);
        let removed = self.entries.remove(&id);
        if removed.is_some() {
            self.remove_name_for_id(id);
            self.rebuild_dependency_graph();
            self.recompute_ids(affected);
        }
        removed.as_ref().map(EntryView::from)
    }

    pub fn get(&self, label: &str) -> Option<&EntryState> {
        let target = EntryTarget::parse(label).ok()?;
        self.entry_for_target(&target).map(|entry| &entry.state)
    }

    /// Returns the current state for an expression by its internal ID.
    pub fn get_by_id(&self, id: ExpressionId) -> Option<&EntryState> {
        self.entries.get(&id).map(|entry| &entry.state)
    }

    pub fn entry(&self, label: &str) -> Option<EntryView> {
        let target = EntryTarget::parse(label).ok()?;
        self.entry_for_target(&target).map(EntryView::from)
    }

    /// Returns a stored expression by its internal ID.
    pub fn entry_by_id(&self, id: ExpressionId) -> Option<EntryView> {
        self.entries.get(&id).map(EntryView::from)
    }

    pub fn entries(&self) -> Vec<EntryView> {
        let mut entries = self
            .entries
            .values()
            .map(EntryView::from)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.id);
        entries
    }

    pub fn cycle_diagnostics(&self) -> CycleDiagnostics {
        let report = self.dependency_graph.cycle_report();

        CycleDiagnostics {
            cycles: report
                .cycles
                .into_iter()
                .map(|cycle| self.labels_for_ids(&cycle))
                .collect(),
            cycle_nodes: self.labels_for_ids(&report.cycle_nodes),
            dependent_nodes: self.labels_for_ids(&report.dependent_nodes),
        }
    }

    pub fn eval_once(&self, source: &str) -> Result<f64, DagcalError> {
        let ast = self
            .resolve_expr(parse_expression(source)?)
            .map_err(DagcalError::Eval)?;
        self.eval_resolved_expr(&ast).map_err(DagcalError::Eval)
    }

    fn recompute_all(&mut self) {
        let ids = self.entries.keys().copied().collect::<BTreeSet<_>>();
        self.recompute_ids(ids);
    }

    fn allocate_id(&mut self) -> ExpressionId {
        loop {
            let id = self.id_generator.next();
            if !self.entries.contains_key(&id) {
                return id;
            }
        }
    }

    fn save_parsed_entry(
        &mut self,
        target: EntryTarget,
        source: String,
        ast: ParsedExpr,
    ) -> Execution {
        let (id, name) = self.resolve_or_create_id(target);
        let entry = match self.resolve_expr(ast) {
            Ok(ast) => Entry::from_resolved(id, name, source, ast),
            Err(err) => Entry::from_parse_error(id, name, source, DagcalError::Eval(err)),
        };
        self.entries.insert(id, entry);
        self.rebuild_dependency_graph();
        self.recompute_affected(&id);

        Execution {
            id: Some(id),
            label: Some(EntryLabel::result(id.value())),
            state: self.entries[&id].state.clone(),
        }
    }

    fn save_parse_error(
        &mut self,
        target: EntryTarget,
        source: String,
        err: DagcalError,
    ) -> Execution {
        let (id, name) = self.resolve_or_create_id(target);
        let entry = Entry::from_parse_error(id, name, source, err);
        self.entries.insert(id, entry);
        self.rebuild_dependency_graph();
        self.recompute_affected(&id);

        Execution {
            id: Some(id),
            label: Some(EntryLabel::result(id.value())),
            state: self.entries[&id].state.clone(),
        }
    }

    fn recompute_affected(&mut self, id: &ExpressionId) {
        let affected = self.collect_affected(*id);
        self.recompute_ids(affected);
    }

    fn collect_affected(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        self.dependency_graph.affected_by(id)
    }

    fn recompute_ids(&mut self, ids: BTreeSet<ExpressionId>) {
        let analysis = self.dependency_graph.analyze(&ids);
        let cycle_nodes = analysis.cycle_report.cycle_nodes;

        for id in ids.intersection(&cycle_nodes) {
            if let Some(entry) = self.entries.get_mut(id) {
                entry.state = EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(
                    entry.label.to_string(),
                )));
            }
        }

        for current in analysis.evaluation_order {
            if cycle_nodes.contains(&current) {
                continue;
            }

            let state = self.evaluate_entry(&current);
            if let Some(entry) = self.entries.get_mut(&current) {
                entry.state = state;
            }
        }
    }

    fn evaluate_entry(&self, id: &ExpressionId) -> EntryState {
        let Some(entry) = self.entries.get(id) else {
            return EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(
                self.label_for_id(*id),
            )));
        };

        let Some(ast) = &entry.ast else {
            return entry.state.clone();
        };

        let result = self.eval_resolved_expr(ast);

        match result {
            Ok(value) => EntryState::Value(value),
            Err(err) => EntryState::Error(DagcalError::Eval(err)),
        }
    }

    fn eval_resolved_expr(&self, ast: &ResolvedExpr) -> Result<f64, EvalError> {
        let mut resolve_entry = |id| self.resolve_entry_reference(id);
        let mut resolve_constant = |name: &str| {
            self.constants
                .get(name)
                .copied()
                .ok_or_else(|| EvalError::UnknownReference(name.to_string()))
        };
        eval_expr(
            ast,
            &self.functions,
            &mut resolve_entry,
            &mut resolve_constant,
        )
    }

    fn resolve_entry_reference(&self, id: ExpressionId) -> Result<f64, EvalError> {
        if let Some(entry) = self.entries.get(&id) {
            match &entry.state {
                EntryState::Value(value) => Ok(*value),
                EntryState::Error(_) => Err(EvalError::DependencyError(self.label_for_id(id))),
            }
        } else {
            Err(EvalError::UnknownReference(format!("${}", id.value())))
        }
    }

    fn entry_for_target(&self, target: &EntryTarget) -> Option<&Entry> {
        self.id_for_target(target)
            .and_then(|id| self.entries.get(&id))
    }

    fn rebuild_dependency_graph(&mut self) {
        self.dependency_graph.rebuild(
            self.entries
                .iter()
                .map(|(id, entry)| (*id, entry.references.clone())),
        );
    }

    fn resolve_expr(&self, expr: ParsedExpr) -> Result<ResolvedExpr, EvalError> {
        match expr {
            ParsedExpr::Number(value) => Ok(ResolvedExpr::Number(value)),
            ParsedExpr::Reference(reference) => self.resolve_parsed_reference(reference),
            ParsedExpr::Unary { op, rhs } => Ok(ResolvedExpr::Unary {
                op,
                rhs: Box::new(self.resolve_expr(*rhs)?),
            }),
            ParsedExpr::Binary { lhs, op, rhs } => Ok(ResolvedExpr::Binary {
                lhs: Box::new(self.resolve_expr(*lhs)?),
                op,
                rhs: Box::new(self.resolve_expr(*rhs)?),
            }),
            ParsedExpr::Call { name, args } => Ok(ResolvedExpr::Call {
                name,
                args: args
                    .into_iter()
                    .map(|arg| self.resolve_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
        }
    }

    fn resolve_parsed_reference(
        &self,
        reference: ParsedReference,
    ) -> Result<ResolvedExpr, EvalError> {
        match reference {
            ParsedReference::Id(id) => Ok(ResolvedExpr::EntryReference(id)),
            ParsedReference::Name(name) => {
                if let Some(id) = self.names.get(&name).copied() {
                    Ok(ResolvedExpr::EntryReference(id))
                } else if self.constants.contains_key(&name) {
                    Ok(ResolvedExpr::Constant(name))
                } else {
                    Err(EvalError::UnknownReference(name))
                }
            }
        }
    }

    fn resolve_or_create_id(&mut self, target: EntryTarget) -> (ExpressionId, Option<String>) {
        match target {
            EntryTarget::Id(id) => {
                self.id_generator.reserve_through(id.value());
                let name = self.entries.get(&id).and_then(|entry| entry.name.clone());
                (id, name)
            }
            EntryTarget::Name(name) => {
                if let Some(id) = self.names.get(&name).copied() {
                    return (id, Some(name));
                }

                let id = self.allocate_id();
                self.names.insert(name.clone(), id);
                (id, Some(name))
            }
        }
    }

    fn label_for_id(&self, id: ExpressionId) -> String {
        self.entries
            .get(&id)
            .map(|entry| entry.label.to_string())
            .unwrap_or_else(|| id.to_string())
    }

    fn labels_for_ids(&self, ids: &BTreeSet<ExpressionId>) -> BTreeSet<String> {
        ids.iter().map(|id| self.label_for_id(*id)).collect()
    }

    fn id_for_target(&self, target: &EntryTarget) -> Option<ExpressionId> {
        match target {
            EntryTarget::Id(id) => Some(*id),
            EntryTarget::Name(name) => self.names.get(name).copied(),
        }
    }

    fn remove_name_for_id(&mut self, id: ExpressionId) {
        self.names.retain(|_, entry_id| *entry_id != id);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Execution {
    pub id: Option<ExpressionId>,
    pub label: Option<EntryLabel>,
    pub state: EntryState,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_value(engine: &Engine, id: &str, expected: f64) {
        match engine.get(id) {
            Some(EntryState::Value(actual)) => assert!((actual - expected).abs() < 1e-12),
            other => panic!("expected value for {id}, got {other:?}"),
        }
    }

    fn assert_eval_error(engine: &Engine, id: &str, matches: impl FnOnce(&EvalError) -> bool) {
        match engine.get(id) {
            Some(EntryState::Error(DagcalError::Eval(err))) if matches(err) => {}
            other => panic!("expected eval error for {id}, got {other:?}"),
        }
    }

    fn execution_label(execution: &Execution) -> String {
        execution
            .label
            .as_ref()
            .expect("expected saved execution label")
            .to_string()
    }

    #[test]
    fn updates_dependents_when_source_changes() {
        let mut engine = Engine::new();

        engine.set_entry("a", "1 + 2").unwrap();
        engine.set_entry("b", "a * 2").unwrap();
        engine.set_entry("c", "b + 1").unwrap();
        assert_value(&engine, "c", 7.0);

        engine.set_entry("a", "10").unwrap();
        assert_value(&engine, "b", 20.0);
        assert_value(&engine, "c", 21.0);
    }

    #[test]
    fn user_entries_override_constants() {
        let mut engine = Engine::new();

        engine.set_entry("pi", "3").unwrap();
        engine.set_entry("x", "pi + 1").unwrap();

        assert_value(&engine, "x", 4.0);
    }

    #[test]
    fn removing_entry_recomputes_dependents_as_errors() {
        let mut engine = Engine::new();

        engine.set_entry("a", "2").unwrap();
        engine.set_entry("b", "a + 3").unwrap();
        assert_value(&engine, "b", 5.0);

        engine.remove_entry("a");

        assert_eval_error(
            &engine,
            "b",
            |err| matches!(err, EvalError::UnknownReference(name) if name == "$1"),
        );
    }

    #[test]
    fn removing_shadowing_entry_leaves_dependents_bound_to_removed_id() {
        let mut engine = Engine::new();

        engine.set_entry("pi", "3").unwrap();
        engine.set_entry("x", "pi + 1").unwrap();
        assert_value(&engine, "x", 4.0);

        engine.remove_entry("pi");

        assert_eval_error(
            &engine,
            "x",
            |err| matches!(err, EvalError::UnknownReference(name) if name == "$1"),
        );
    }

    #[test]
    fn parse_errors_propagate_and_recover_after_edit() {
        let mut engine = Engine::new();

        engine.set_entry("a", "1").unwrap();
        engine.set_entry("b", "a + 2").unwrap();
        assert!(engine.set_entry("a", "1 +").is_err());

        assert_eval_error(
            &engine,
            "b",
            |err| matches!(err, EvalError::DependencyError(name) if name == "$1"),
        );

        engine.set_entry("a", "10").unwrap();
        assert_value(&engine, "b", 12.0);
    }

    #[test]
    fn changing_dependencies_drops_old_reverse_dependency() {
        let mut engine = Engine::new();

        engine.set_entry("a", "1").unwrap();
        engine.set_entry("b", "a + 1").unwrap();
        assert_value(&engine, "b", 2.0);

        engine.set_entry("b", "100").unwrap();
        engine.set_entry("a", "10").unwrap();

        assert_value(&engine, "b", 100.0);
    }

    #[test]
    fn recomputes_branching_graph_through_errors_and_recovery() {
        let mut engine = Engine::new();

        engine.set_entry("price", "10").unwrap();
        engine.set_entry("quantity", "3").unwrap();
        engine.set_entry("discount", "2").unwrap();
        engine.set_entry("gross", "price * quantity").unwrap();
        engine.set_entry("net", "gross - discount").unwrap();
        engine.set_entry("fee", "price / (quantity - 1)").unwrap();
        engine
            .set_entry("summary", "net + fee + sin(pi / 2)")
            .unwrap();

        assert_value(&engine, "gross", 30.0);
        assert_value(&engine, "net", 28.0);
        assert_value(&engine, "fee", 5.0);
        assert_value(&engine, "summary", 34.0);

        engine.set_entry("price", "20").unwrap();

        assert_value(&engine, "gross", 60.0);
        assert_value(&engine, "net", 58.0);
        assert_value(&engine, "fee", 10.0);
        assert_value(&engine, "summary", 69.0);

        engine.remove_entry("discount");

        assert_value(&engine, "gross", 60.0);
        assert_value(&engine, "fee", 10.0);
        assert_eval_error(
            &engine,
            "net",
            |err| matches!(err, EvalError::UnknownReference(name) if name == "$3"),
        );
        assert_eval_error(
            &engine,
            "summary",
            |err| matches!(err, EvalError::DependencyError(name) if name == "$5"),
        );

        engine.set_entry("$3", "8").unwrap();

        assert_value(&engine, "net", 52.0);
        assert_value(&engine, "summary", 63.0);

        engine.set_entry("quantity", "1").unwrap();

        assert_value(&engine, "gross", 20.0);
        assert_value(&engine, "net", 12.0);
        assert_eval_error(&engine, "fee", |err| {
            matches!(err, EvalError::DivisionByZero)
        });
        assert_eval_error(
            &engine,
            "summary",
            |err| matches!(err, EvalError::DependencyError(name) if name == "$6"),
        );

        engine.set_entry("quantity", "4").unwrap();

        assert_value(&engine, "gross", 80.0);
        assert_value(&engine, "net", 72.0);
        assert_value(&engine, "fee", 20.0 / 3.0);
        assert_value(&engine, "summary", 72.0 + (20.0 / 3.0) + 1.0);
    }

    #[test]
    fn dependency_errors_propagate() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("a", "missing + 1").is_err());
        assert!(engine.set_entry("b", "a * 2").is_err());

        assert!(matches!(
            engine.get("b"),
            Some(EntryState::Error(DagcalError::Eval(
                EvalError::DependencyError(name)
            ))) if name == "$1"
        ));
    }

    #[test]
    fn detects_cycles() {
        let mut engine = Engine::new();

        engine.set_entry("a", "1").unwrap();
        engine.set_entry("b", "2").unwrap();
        engine.set_entry("a", "b + 1").unwrap();
        assert!(engine.set_entry("b", "a + 1").is_err());

        assert!(matches!(
            engine.get("a"),
            Some(EntryState::Error(DagcalError::Eval(
                EvalError::CycleDetected(_)
            )))
        ));
    }

    #[test]
    fn self_reference_is_cycle() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("a", "a + 1").is_err());

        assert_eval_error(
            &engine,
            "a",
            |err| matches!(err, EvalError::CycleDetected(name) if name == "$1"),
        );

        let diagnostics = engine.cycle_diagnostics();
        assert_eq!(diagnostics.cycles, vec![BTreeSet::from(["$1".to_string()])]);
        assert_eq!(diagnostics.cycle_nodes, BTreeSet::from(["$1".to_string()]));
        assert!(diagnostics.dependent_nodes.is_empty());
    }

    #[test]
    fn reports_cycle_nodes_and_all_dependents() {
        let mut engine = Engine::new();

        engine.set_entry("a", "1").unwrap();
        engine.set_entry("b", "2").unwrap();
        engine.set_entry("c", "3").unwrap();
        engine.set_entry("d", "4").unwrap();
        engine.set_entry("a", "b + 1").unwrap();
        assert!(engine.set_entry("b", "a + 1").is_err());
        assert!(engine.set_entry("c", "a + 1").is_err());
        assert!(engine.set_entry("d", "c + 1").is_err());
        engine.set_entry("ok", "10").unwrap();

        let diagnostics = engine.cycle_diagnostics();

        assert_eq!(
            diagnostics.cycles,
            vec![BTreeSet::from(["$1".to_string(), "$2".to_string()])]
        );
        assert_eq!(
            diagnostics.cycle_nodes,
            BTreeSet::from(["$1".to_string(), "$2".to_string()])
        );
        assert_eq!(
            diagnostics.dependent_nodes,
            BTreeSet::from(["$3".to_string(), "$4".to_string()])
        );
        assert_eval_error(
            &engine,
            "c",
            |err| matches!(err, EvalError::DependencyError(name) if name == "$1"),
        );
        assert_eval_error(
            &engine,
            "d",
            |err| matches!(err, EvalError::DependencyError(name) if name == "$3"),
        );
        assert_value(&engine, "ok", 10.0);
    }

    #[test]
    fn reports_multiple_independent_cycles() {
        let mut engine = Engine::new();

        engine.set_entry("a", "1").unwrap();
        engine.set_entry("b", "2").unwrap();
        engine.set_entry("x", "3").unwrap();
        engine.set_entry("y", "4").unwrap();
        engine.set_entry("z", "5").unwrap();
        engine.set_entry("a", "b + 1").unwrap();
        assert!(engine.set_entry("b", "a + 1").is_err());
        engine.set_entry("x", "y + 1").unwrap();
        assert!(engine.set_entry("y", "x + 1").is_err());
        assert!(engine.set_entry("z", "x + a").is_err());

        let diagnostics = engine.cycle_diagnostics();

        assert_eq!(
            diagnostics.cycles,
            vec![
                BTreeSet::from(["$1".to_string(), "$2".to_string()]),
                BTreeSet::from(["$3".to_string(), "$4".to_string()])
            ]
        );
        assert_eq!(
            diagnostics.cycle_nodes,
            BTreeSet::from([
                "$1".to_string(),
                "$2".to_string(),
                "$3".to_string(),
                "$4".to_string()
            ])
        );
        assert_eq!(
            diagnostics.dependent_nodes,
            BTreeSet::from(["$5".to_string()])
        );
    }

    #[test]
    fn clearing_cycle_clears_diagnostics_and_recomputes_dependents() {
        let mut engine = Engine::new();

        engine.set_entry("a", "1").unwrap();
        engine.set_entry("b", "2").unwrap();
        engine.set_entry("c", "3").unwrap();
        engine.set_entry("a", "b + 1").unwrap();
        assert!(engine.set_entry("b", "a + 1").is_err());
        assert!(engine.set_entry("c", "a + 1").is_err());
        assert!(!engine.cycle_diagnostics().cycle_nodes.is_empty());

        engine.set_entry("a", "1").unwrap();

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

        engine.set_entry("z", "1").unwrap();
        engine.set_entry("a", "z + 1").unwrap();
        engine.set_entry("cycle_left", "1").unwrap();
        engine.set_entry("cycle_right", "2").unwrap();
        engine.set_entry("cycle_left", "cycle_right + 1").unwrap();
        assert!(engine.set_entry("cycle_right", "cycle_left + 1").is_err());

        engine.set_entry("z", "10").unwrap();

        assert_value(&engine, "z", 10.0);
        assert_value(&engine, "a", 11.0);
        assert_eval_error(
            &engine,
            "cycle_left",
            |err| matches!(err, EvalError::CycleDetected(name) if name == "$3"),
        );
        assert_eval_error(
            &engine,
            "cycle_right",
            |err| matches!(err, EvalError::CycleDetected(name) if name == "$4"),
        );
    }

    #[test]
    fn execute_defines_named_entries_and_appends_expressions() {
        let mut engine = Engine::new();

        let subtotal = engine.execute("subtotal = 100");
        let taxed = engine.execute("subtotal * 1.1");

        assert_eq!(execution_label(&subtotal), "$1");
        assert_eq!(subtotal.state, EntryState::Value(100.0));
        assert_eq!(execution_label(&taxed), "$2");
        assert_eq!(taxed.state, EntryState::Value(110.00000000000001));
        assert_eq!(engine.entry("subtotal").unwrap().source, "100");
        assert_eq!(engine.entry("$1").unwrap().source, "100");
        assert_eq!(engine.entry("$2").unwrap().source, "subtotal * 1.1");
    }

    #[test]
    fn execute_rejects_result_label_definitions_without_saving() {
        let mut engine = Engine::new();

        let execution = engine.execute("$1 = 100");

        assert!(execution.label.is_none());
        assert!(matches!(
            execution.state,
            EntryState::Error(DagcalError::Parse(_))
        ));
        assert!(engine.entry("$1").is_none());
    }

    #[test]
    fn appends_numbered_results_and_references_them_with_dollar_syntax() {
        let mut engine = Engine::new();

        let first = engine.execute("1 + 2");
        let second = engine.execute("$1 * 10");

        assert_eq!(execution_label(&first), "$1");
        assert_eq!(execution_label(&second), "$2");
        assert_eq!(first.state, EntryState::Value(3.0));
        assert_eq!(second.state, EntryState::Value(30.0));
        assert_value(&engine, "$2", 30.0);
    }

    #[test]
    fn editing_numbered_result_updates_dollar_dependents() {
        let mut engine = Engine::new();

        engine.execute("2");
        engine.execute("$1 + 3");
        assert_value(&engine, "$2", 5.0);

        engine.set_entry("$1", "10").unwrap();

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

        assert_eq!(execution_label(&first), "$1");
        assert_eq!(execution_label(&second), "$2");
        assert_eq!(execution_label(&third), "$3");
        assert_eq!(execution_label(&fourth), "$4");
        assert_eq!(execution_label(&fifth), "$5");
        assert_value(&engine, "$2", 5.0);
        assert_value(&engine, "$3", 10.0);
        assert_value(&engine, "$4", 16.0);
        assert_eval_error(&engine, "$5", |err| {
            matches!(err, EvalError::DivisionByZero)
        });

        engine.set_entry("$1", "4").unwrap();

        assert_value(&engine, "$2", 7.0);
        assert_value(&engine, "$3", 28.0);
        assert_value(&engine, "$4", 36.0);
        assert_value(&engine, "$5", 18.0);

        engine.remove_entry("$2");

        assert_value(&engine, "$1", 4.0);
        assert_eval_error(
            &engine,
            "$3",
            |err| matches!(err, EvalError::UnknownReference(name) if name == "$2"),
        );
        assert_eval_error(
            &engine,
            "$4",
            |err| matches!(err, EvalError::UnknownReference(name) if name == "$2"),
        );
        assert_eval_error(
            &engine,
            "$5",
            |err| matches!(err, EvalError::DependencyError(name) if name == "$4"),
        );

        engine.set_entry("$2", "$1 + 6").unwrap();

        assert_value(&engine, "$2", 10.0);
        assert_value(&engine, "$3", 40.0);
        assert_value(&engine, "$4", 51.0);
        assert_value(&engine, "$5", 51.0 / 5.0);

        let sixth = engine.execute("$5 + $3");

        assert_eq!(execution_label(&sixth), "$6");
        assert_eq!(sixth.state, EntryState::Value(50.2));
        assert_value(&engine, "$6", 50.2);
    }

    #[test]
    fn appending_skips_existing_numbered_results() {
        let mut engine = Engine::new();

        engine.set_entry("$1", "100").unwrap();
        let execution = engine.execute("$1 + 1");

        assert_eq!(execution_label(&execution), "$2");
        assert_eq!(execution.state, EntryState::Value(101.0));
    }

    #[test]
    fn explicit_numbered_result_reserves_sequential_expression_id() {
        let mut engine = Engine::new();

        engine.set_entry("$5", "100").unwrap();
        let execution = engine.execute("$5 + 1");

        assert_eq!(execution_label(&execution), "$6");
        assert_eq!(execution.state, EntryState::Value(101.0));
    }

    #[test]
    fn exposes_entries_by_internal_expression_id() {
        let mut engine = Engine::new();

        let execution = engine.execute("40 + 2");
        let label = execution_label(&execution);
        let entry = engine.entry(&label).unwrap();
        let id = entry.id;

        assert_eq!(engine.get_by_id(id), Some(&EntryState::Value(42.0)));
        assert_eq!(engine.entry_by_id(id).unwrap().label.to_string(), "$1");
    }

    #[test]
    fn named_labels_resolve_to_internal_expression_ids() {
        let mut engine = Engine::new();

        engine.set_entry("subtotal", "100").unwrap();
        let id = engine.entry("subtotal").unwrap().id;
        engine.set_entry("taxed", "subtotal * 1.1").unwrap();
        engine.set_entry("subtotal", "200").unwrap();

        assert_eq!(engine.entry("subtotal").unwrap().id, id);
        assert_value(&engine, "taxed", 220.00000000000003);
    }

    #[test]
    fn stored_ast_uses_resolved_expression_ids_for_named_references() {
        let mut engine = Engine::new();

        engine.set_entry("subtotal", "100").unwrap();
        let subtotal_id = engine.entry("subtotal").unwrap().id;
        engine.set_entry("taxed", "subtotal * 1.1").unwrap();

        let taxed_id = engine.entry("taxed").unwrap().id;
        let taxed = engine.entries.get(&taxed_id).unwrap();

        assert_eq!(taxed.references, BTreeSet::from([subtotal_id]));
        assert!(matches!(
            taxed.ast,
            Some(ResolvedExpr::Binary { ref lhs, .. })
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

        engine.set_entry("subtotal", "100").unwrap();
        engine.execute("subtotal * 1.1");

        let entries = engine
            .entries()
            .into_iter()
            .map(|entry| {
                (
                    entry.label.to_string(),
                    entry.source.clone(),
                    entry.state.clone(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "$1");
        assert_eq!(entries[0].1, "100");
        assert_eq!(entries[0].2, EntryState::Value(100.0));
        assert_eq!(entries[1].0, "$2");
        assert_eq!(entries[1].1, "subtotal * 1.1");
        assert_eq!(entries[1].2, EntryState::Value(110.00000000000001));
    }

    #[test]
    fn eval_once_can_reference_registered_entries() {
        let mut engine = Engine::new();

        engine.set_entry("subtotal", "40").unwrap();

        assert_eq!(engine.eval_once("subtotal * 1.1").unwrap(), 44.0);
    }

    #[test]
    fn registering_function_recomputes_existing_entries() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("x", "triple(14)").is_err());
        engine.register_fixed_function("triple", 1, |args| Ok(args[0] * 3.0));

        assert_eq!(engine.get("x"), Some(&EntryState::Value(42.0)));
    }

    #[test]
    fn registering_variadic_function_recomputes_existing_entries() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("x", "product(2, 3, 4)").is_err());
        engine.register_variadic_function("product", 0, |args| Ok(args.iter().product()));

        assert_eq!(engine.get("x"), Some(&EntryState::Value(24.0)));
    }

    #[test]
    fn setting_constant_recomputes_existing_entries() {
        let mut engine = Engine::new();

        engine.set_entry("radius", "2").unwrap();
        engine.set_constant("tau", 6.0);
        engine.set_entry("area", "tau * radius ^ 2 / 2").unwrap();
        engine.set_constant("tau", std::f64::consts::TAU);

        assert_eq!(
            engine.get("area"),
            Some(&EntryState::Value(std::f64::consts::TAU * 2.0))
        );
    }

    #[test]
    fn unresolved_names_do_not_recover_when_name_is_defined_later() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("area", "tau * 2").is_err());
        engine.set_constant("tau", std::f64::consts::TAU);

        assert_eval_error(
            &engine,
            "area",
            |err| matches!(err, EvalError::UnknownReference(name) if name == "tau"),
        );
    }
}
