use crate::ast::{Expr, Statement};
use crate::dependency_graph::ReferenceGraph;
use crate::error::{DagcalError, EvalError};
use crate::eval::eval_expr;
use crate::function::{FunctionRegistry, FunctionSignature};
use crate::id::{ExpressionId, ExpressionIdGenerator};
use crate::label::EntryLabel;
use crate::parser::{parse_expression, parse_statement};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: ExpressionId,
    pub label: EntryLabel,
    pub source: String,
    pub ast: Option<Expr>,
    pub references: BTreeSet<EntryLabel>,
    pub state: EntryState,
}

impl Entry {
    fn from_parsed(id: ExpressionId, label: EntryLabel, source: String, ast: Expr) -> Self {
        Self {
            id,
            label,
            source,
            references: ast.references(),
            ast: Some(ast),
            state: EntryState::Error(DagcalError::Eval(EvalError::DependencyError(
                id.to_string(),
            ))),
        }
    }

    fn from_parse_error(
        id: ExpressionId,
        label: EntryLabel,
        source: String,
        err: DagcalError,
    ) -> Self {
        Self {
            id,
            label,
            source,
            ast: None,
            references: BTreeSet::new(),
            state: EntryState::Error(err),
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

pub struct Engine {
    entries: HashMap<ExpressionId, Entry>,
    labels: HashMap<EntryLabel, ExpressionId>,
    constants: HashMap<String, f64>,
    functions: FunctionRegistry,
    dependency_graph: ReferenceGraph,
    id_generator: ExpressionIdGenerator,
    next_result_index: usize,
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
            labels: HashMap::new(),
            constants: HashMap::from([
                ("e".to_string(), std::f64::consts::E),
                ("pi".to_string(), std::f64::consts::PI),
            ]),
            functions: FunctionRegistry::standard(),
            dependency_graph: ReferenceGraph::new(),
            id_generator: ExpressionIdGenerator::new(),
            next_result_index: 1,
        }
    }

    /// Executes user input as either a named definition (`name = expr`) or an expression.
    ///
    /// Named definitions update or create the named entry. Plain expressions are appended as the
    /// next available `$n` result entry.
    pub fn execute(&mut self, input: &str) -> Execution {
        match parse_statement(input) {
            Ok(Statement::Definition { name, expr }) => {
                let source = definition_source(input, &name);
                self.save_parsed_entry(name, source, expr)
            }
            Ok(Statement::Expression(expr)) => {
                let label = self.allocate_result_label();
                self.save_parsed_entry(label, input.trim().to_string(), expr)
            }
            Err(err) => Execution {
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
        let label = EntryLabel::parse(label.as_ref())?;
        let source = source.into();
        let execution = match parse_expression(&source) {
            Ok(ast) => self.save_parsed_entry(label, source, ast),
            Err(err) => self.save_parse_error(label, source, err),
        };

        match execution.state {
            EntryState::Value(_) => Ok(execution.state),
            EntryState::Error(err) => Err(err),
        }
    }

    pub fn remove_entry(&mut self, label: &str) -> Option<Entry> {
        let label = EntryLabel::parse(label).ok()?;
        let id = self.labels.get(&label).copied()?;
        let affected = self.collect_affected(id);
        self.labels.remove(&label);
        let removed = self.entries.remove(&id);
        if removed.is_some() {
            self.rebuild_dependency_graph();
            self.recompute_ids(affected);
        }
        removed
    }

    pub fn get(&self, label: &str) -> Option<&EntryState> {
        self.entry(label).map(|entry| &entry.state)
    }

    /// Returns the current state for an expression by its internal ID.
    pub fn get_by_id(&self, id: ExpressionId) -> Option<&EntryState> {
        self.entry_by_id(id).map(|entry| &entry.state)
    }

    pub fn entry(&self, label: &str) -> Option<&Entry> {
        let label = EntryLabel::parse(label).ok()?;
        self.entry_for_label(&label)
    }

    /// Returns a stored expression by its internal ID.
    pub fn entry_by_id(&self, id: ExpressionId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&EntryLabel, &Entry)> {
        self.entries.values().map(|entry| (&entry.label, entry))
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
        let ast = parse_expression(source)?;
        let mut resolve = |name: &EntryLabel| self.resolve_reference(name);
        eval_expr(&ast, &self.functions, &mut resolve).map_err(DagcalError::Eval)
    }

    fn recompute_all(&mut self) {
        let ids = self.entries.keys().copied().collect::<BTreeSet<_>>();
        self.recompute_ids(ids);
    }

    fn allocate_result_label(&mut self) -> EntryLabel {
        loop {
            let label = EntryLabel::result(self.next_result_index);
            self.next_result_index += 1;
            if !self.labels.contains_key(&label) {
                return label;
            }
        }
    }

    fn save_parsed_entry(&mut self, label: EntryLabel, source: String, ast: Expr) -> Execution {
        let id = self.resolve_or_create_id(&label);
        let entry = Entry::from_parsed(id, label.clone(), source, ast);
        self.entries.insert(id, entry);
        self.rebuild_dependency_graph();
        self.recompute_affected(&id);

        Execution {
            label: Some(label),
            state: self.entries[&id].state.clone(),
        }
    }

    fn save_parse_error(
        &mut self,
        label: EntryLabel,
        source: String,
        err: DagcalError,
    ) -> Execution {
        let id = self.resolve_or_create_id(&label);
        let entry = Entry::from_parse_error(id, label.clone(), source, err);
        self.entries.insert(id, entry);
        self.rebuild_dependency_graph();
        self.recompute_affected(&id);

        Execution {
            label: Some(label),
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

        let mut resolve = |name: &EntryLabel| self.resolve_reference(name);
        let result = eval_expr(ast, &self.functions, &mut resolve);

        match result {
            Ok(value) => EntryState::Value(value),
            Err(err) => EntryState::Error(DagcalError::Eval(err)),
        }
    }

    fn resolve_reference(&self, name: &EntryLabel) -> Result<f64, EvalError> {
        if let Some(entry) = self.entry_for_label(name) {
            match &entry.state {
                EntryState::Value(value) => Ok(*value),
                EntryState::Error(_) => Err(EvalError::DependencyError(name.to_string())),
            }
        } else if let Some(value) = name
            .constant_name()
            .and_then(|constant| self.constants.get(constant))
        {
            Ok(*value)
        } else {
            Err(EvalError::UnknownReference(name.to_string()))
        }
    }

    fn entry_for_label(&self, label: &EntryLabel) -> Option<&Entry> {
        self.labels.get(label).and_then(|id| self.entries.get(id))
    }

    fn rebuild_dependency_graph(&mut self) {
        let labels = &self.labels;
        self.dependency_graph
            .rebuild(self.entries.iter().map(|(id, entry)| {
                let references = entry
                    .references
                    .iter()
                    .filter_map(|reference| labels.get(reference).copied())
                    .collect::<BTreeSet<_>>();
                (*id, references)
            }));
    }

    fn resolve_or_create_id(&mut self, label: &EntryLabel) -> ExpressionId {
        if let Some(id) = self.labels.get(label) {
            return *id;
        }

        let id = self.id_generator.next();
        if let Some(index) = label.result_index() {
            self.reserve_result_label_through(index);
        }
        self.labels.insert(label.clone(), id);
        id
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

    fn reserve_result_label_through(&mut self, index: usize) {
        self.next_result_index = self.next_result_index.max(index + 1);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Execution {
    pub label: Option<EntryLabel>,
    pub state: EntryState,
}

fn definition_source(input: &str, name: &EntryLabel) -> String {
    let Some((left, right)) = input.split_once('=') else {
        return input.trim().to_string();
    };

    if left.trim() == name.to_string() {
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
            |err| matches!(err, EvalError::UnknownReference(name) if name == "a"),
        );
    }

    #[test]
    fn removing_shadowing_entry_reveals_constant_to_dependents() {
        let mut engine = Engine::new();

        engine.set_entry("pi", "3").unwrap();
        engine.set_entry("x", "pi + 1").unwrap();
        assert_value(&engine, "x", 4.0);

        engine.remove_entry("pi");

        assert_value(&engine, "x", std::f64::consts::PI + 1.0);
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
            |err| matches!(err, EvalError::DependencyError(name) if name == "a"),
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
            |err| matches!(err, EvalError::UnknownReference(name) if name == "discount"),
        );
        assert_eval_error(
            &engine,
            "summary",
            |err| matches!(err, EvalError::DependencyError(name) if name == "net"),
        );

        engine.set_entry("discount", "8").unwrap();

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
            |err| matches!(err, EvalError::DependencyError(name) if name == "fee"),
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
            ))) if name == "a"
        ));
    }

    #[test]
    fn detects_cycles() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("a", "b + 1").is_err());
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
            |err| matches!(err, EvalError::CycleDetected(name) if name == "a"),
        );

        let diagnostics = engine.cycle_diagnostics();
        assert_eq!(diagnostics.cycles, vec![BTreeSet::from(["a".to_string()])]);
        assert_eq!(diagnostics.cycle_nodes, BTreeSet::from(["a".to_string()]));
        assert!(diagnostics.dependent_nodes.is_empty());
    }

    #[test]
    fn reports_cycle_nodes_and_all_dependents() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("a", "b + 1").is_err());
        assert!(engine.set_entry("b", "a + 1").is_err());
        assert!(engine.set_entry("c", "a + 1").is_err());
        assert!(engine.set_entry("d", "c + 1").is_err());
        engine.set_entry("ok", "10").unwrap();

        let diagnostics = engine.cycle_diagnostics();

        assert_eq!(
            diagnostics.cycles,
            vec![BTreeSet::from(["a".to_string(), "b".to_string()])]
        );
        assert_eq!(
            diagnostics.cycle_nodes,
            BTreeSet::from(["a".to_string(), "b".to_string()])
        );
        assert_eq!(
            diagnostics.dependent_nodes,
            BTreeSet::from(["c".to_string(), "d".to_string()])
        );
        assert_eval_error(
            &engine,
            "c",
            |err| matches!(err, EvalError::DependencyError(name) if name == "a"),
        );
        assert_eval_error(
            &engine,
            "d",
            |err| matches!(err, EvalError::DependencyError(name) if name == "c"),
        );
        assert_value(&engine, "ok", 10.0);
    }

    #[test]
    fn reports_multiple_independent_cycles() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("a", "b + 1").is_err());
        assert!(engine.set_entry("b", "a + 1").is_err());
        assert!(engine.set_entry("x", "y + 1").is_err());
        assert!(engine.set_entry("y", "x + 1").is_err());
        assert!(engine.set_entry("z", "x + a").is_err());

        let diagnostics = engine.cycle_diagnostics();

        assert_eq!(
            diagnostics.cycles,
            vec![
                BTreeSet::from(["a".to_string(), "b".to_string()]),
                BTreeSet::from(["x".to_string(), "y".to_string()])
            ]
        );
        assert_eq!(
            diagnostics.cycle_nodes,
            BTreeSet::from([
                "a".to_string(),
                "b".to_string(),
                "x".to_string(),
                "y".to_string()
            ])
        );
        assert_eq!(
            diagnostics.dependent_nodes,
            BTreeSet::from(["z".to_string()])
        );
    }

    #[test]
    fn clearing_cycle_clears_diagnostics_and_recomputes_dependents() {
        let mut engine = Engine::new();

        assert!(engine.set_entry("a", "b + 1").is_err());
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
        assert!(engine.set_entry("cycle_left", "cycle_right + 1").is_err());
        assert!(engine.set_entry("cycle_right", "cycle_left + 1").is_err());

        engine.set_entry("z", "10").unwrap();

        assert_value(&engine, "z", 10.0);
        assert_value(&engine, "a", 11.0);
        assert_eval_error(
            &engine,
            "cycle_left",
            |err| matches!(err, EvalError::CycleDetected(name) if name == "cycle_left"),
        );
        assert_eval_error(
            &engine,
            "cycle_right",
            |err| matches!(err, EvalError::CycleDetected(name) if name == "cycle_right"),
        );
    }

    #[test]
    fn execute_defines_named_entries_and_appends_expressions() {
        let mut engine = Engine::new();

        let subtotal = engine.execute("subtotal = 100");
        let taxed = engine.execute("subtotal * 1.1");

        assert_eq!(execution_label(&subtotal), "subtotal");
        assert_eq!(subtotal.state, EntryState::Value(100.0));
        assert_eq!(execution_label(&taxed), "$1");
        assert_eq!(taxed.state, EntryState::Value(110.00000000000001));
        assert_eq!(engine.entry("subtotal").unwrap().source, "100");
        assert_eq!(engine.entry("$1").unwrap().source, "subtotal * 1.1");
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
    fn exposes_entries_for_read_only_listing() {
        let mut engine = Engine::new();

        engine.set_entry("subtotal", "100").unwrap();
        engine.execute("subtotal * 1.1");

        let mut entries = engine
            .entries()
            .map(|(id, entry)| (id.to_string(), entry.source.clone(), entry.state.clone()))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(&right.0));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "$1");
        assert_eq!(entries[0].1, "subtotal * 1.1");
        assert_eq!(entries[0].2, EntryState::Value(110.00000000000001));
        assert_eq!(entries[1].0, "subtotal");
        assert_eq!(entries[1].1, "100");
        assert_eq!(entries[1].2, EntryState::Value(100.0));
    }
}
