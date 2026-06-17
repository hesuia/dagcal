use crate::ast::Expr;
use crate::dependency_graph::ReferenceGraph;
use crate::error::{DagcalError, EvalError};
use crate::eval::eval_expr;
use crate::function::FunctionRegistry;
use crate::id::{ExpressionId, ExpressionIdGenerator};
use crate::parser::parse_expression;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: ExpressionId,
    pub label: String,
    pub source: String,
    pub ast: Option<Expr>,
    pub references: BTreeSet<String>,
    pub state: EntryState,
}

impl Entry {
    fn from_parsed(id: ExpressionId, label: String, source: String, ast: Expr) -> Self {
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

    fn from_parse_error(id: ExpressionId, label: String, source: String, err: DagcalError) -> Self {
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
    labels: HashMap<String, ExpressionId>,
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

    pub fn register_function<F>(&mut self, name: impl Into<String>, arity: usize, body: F)
    where
        F: Fn(&[f64]) -> Result<f64, EvalError> + Send + Sync + 'static,
    {
        self.functions.register(name, arity, body);
        self.recompute_all();
    }

    pub fn set_constant(&mut self, name: impl Into<String>, value: f64) {
        self.constants.insert(name.into(), value);
        self.recompute_all();
    }

    pub fn set_expr(
        &mut self,
        id: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<(), DagcalError> {
        let label = id.into();
        let source = source.into();
        let id = self.resolve_or_create_id(&label);
        let entry = match parse_expression(&source) {
            Ok(ast) => Entry::from_parsed(id, label.clone(), source, ast),
            Err(err) => Entry::from_parse_error(id, label.clone(), source, err.clone()),
        };

        self.entries.insert(id, entry);
        self.rebuild_dependency_graph();
        self.recompute_affected(&id);

        self.state_result(&id)
    }

    pub fn append_expr(&mut self, source: impl Into<String>) -> (String, EntryState) {
        let id = self.id_generator.next();
        let label = self.allocate_result_label();
        self.labels.insert(label.clone(), id);
        let _ = self.set_expr(label.clone(), source);
        let state = self.get(&label).cloned().unwrap_or_else(|| {
            EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(
                label.clone(),
            )))
        });

        (label, state)
    }

    pub fn remove_expr(&mut self, label: &str) -> Option<Entry> {
        let id = self.labels.get(label).copied()?;
        let affected = self.collect_affected(id);
        self.labels.remove(label);
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
        self.labels.get(label).and_then(|id| self.entries.get(id))
    }

    /// Returns a stored expression by its internal ID.
    pub fn entry_by_id(&self, id: ExpressionId) -> Option<&Entry> {
        self.entries.get(&id)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &Entry)> {
        self.entries
            .values()
            .map(|entry| (entry.label.as_str(), entry))
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
        let mut resolve = |name: &str| self.resolve_reference(name);
        eval_expr(&ast, &self.functions, &mut resolve).map_err(DagcalError::Eval)
    }

    fn recompute_all(&mut self) {
        let ids = self.entries.keys().copied().collect::<BTreeSet<_>>();
        self.recompute_ids(ids);
    }

    fn allocate_result_label(&mut self) -> String {
        loop {
            let label = format!("${}", self.next_result_index);
            self.next_result_index += 1;
            if !self.labels.contains_key(&label) {
                return label;
            }
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
        let cycle_nodes = self.dependency_graph.cycle_report().cycle_nodes;
        for id in ids.intersection(&cycle_nodes) {
            if let Some(entry) = self.entries.get_mut(id) {
                entry.state = EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(
                    entry.label.clone(),
                )));
            }
        }

        for current in self.dependency_graph.evaluation_order(ids) {
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
        if self
            .dependency_graph
            .cycle_report()
            .cycle_nodes
            .contains(id)
        {
            return EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(
                self.label_for_id(*id),
            )));
        }

        let Some(entry) = self.entries.get(id) else {
            return EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(
                self.label_for_id(*id),
            )));
        };

        let Some(ast) = &entry.ast else {
            return entry.state.clone();
        };

        let mut resolve = |name: &str| self.resolve_reference(name);
        let result = eval_expr(ast, &self.functions, &mut resolve);

        match result {
            Ok(value) => EntryState::Value(value),
            Err(err) => EntryState::Error(DagcalError::Eval(err)),
        }
    }

    fn resolve_reference(&self, name: &str) -> Result<f64, EvalError> {
        if let Some(entry) = self.entry(name) {
            match &entry.state {
                EntryState::Value(value) => Ok(*value),
                EntryState::Error(_) => Err(EvalError::DependencyError(name.to_string())),
            }
        } else if let Some(value) = self.constants.get(name) {
            Ok(*value)
        } else {
            Err(EvalError::UnknownReference(name.to_string()))
        }
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

    fn state_result(&self, id: &ExpressionId) -> Result<(), DagcalError> {
        match &self.entries[id].state {
            EntryState::Value(_) => Ok(()),
            EntryState::Error(err) => Err(err.clone()),
        }
    }

    fn resolve_or_create_id(&mut self, label: &str) -> ExpressionId {
        if let Some(id) = self.labels.get(label) {
            return *id;
        }

        let id = self.id_generator.next();
        if let Some(index) = parse_result_index(label) {
            self.reserve_result_label_through(index);
        }
        self.labels.insert(label.to_string(), id);
        id
    }

    fn label_for_id(&self, id: ExpressionId) -> String {
        self.entries
            .get(&id)
            .map(|entry| entry.label.clone())
            .unwrap_or_else(|| id.to_string())
    }

    fn labels_for_ids(&self, ids: &BTreeSet<ExpressionId>) -> BTreeSet<String> {
        ids.iter().map(|id| self.label_for_id(*id)).collect()
    }

    fn reserve_result_label_through(&mut self, index: usize) {
        self.next_result_index = self.next_result_index.max(index + 1);
    }
}

fn parse_result_index(label: &str) -> Option<usize> {
    label.strip_prefix('$')?.parse().ok()
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

    #[test]
    fn updates_dependents_when_source_changes() {
        let mut engine = Engine::new();

        engine.set_expr("a", "1 + 2").unwrap();
        engine.set_expr("b", "a * 2").unwrap();
        engine.set_expr("c", "b + 1").unwrap();
        assert_value(&engine, "c", 7.0);

        engine.set_expr("a", "10").unwrap();
        assert_value(&engine, "b", 20.0);
        assert_value(&engine, "c", 21.0);
    }

    #[test]
    fn user_entries_override_constants() {
        let mut engine = Engine::new();

        engine.set_expr("pi", "3").unwrap();
        engine.set_expr("x", "pi + 1").unwrap();

        assert_value(&engine, "x", 4.0);
    }

    #[test]
    fn removing_entry_recomputes_dependents_as_errors() {
        let mut engine = Engine::new();

        engine.set_expr("a", "2").unwrap();
        engine.set_expr("b", "a + 3").unwrap();
        assert_value(&engine, "b", 5.0);

        engine.remove_expr("a");

        assert_eval_error(
            &engine,
            "b",
            |err| matches!(err, EvalError::UnknownReference(name) if name == "a"),
        );
    }

    #[test]
    fn removing_shadowing_entry_reveals_constant_to_dependents() {
        let mut engine = Engine::new();

        engine.set_expr("pi", "3").unwrap();
        engine.set_expr("x", "pi + 1").unwrap();
        assert_value(&engine, "x", 4.0);

        engine.remove_expr("pi");

        assert_value(&engine, "x", std::f64::consts::PI + 1.0);
    }

    #[test]
    fn parse_errors_propagate_and_recover_after_edit() {
        let mut engine = Engine::new();

        engine.set_expr("a", "1").unwrap();
        engine.set_expr("b", "a + 2").unwrap();
        assert!(engine.set_expr("a", "1 +").is_err());

        assert_eval_error(
            &engine,
            "b",
            |err| matches!(err, EvalError::DependencyError(name) if name == "a"),
        );

        engine.set_expr("a", "10").unwrap();
        assert_value(&engine, "b", 12.0);
    }

    #[test]
    fn changing_dependencies_drops_old_reverse_dependency() {
        let mut engine = Engine::new();

        engine.set_expr("a", "1").unwrap();
        engine.set_expr("b", "a + 1").unwrap();
        assert_value(&engine, "b", 2.0);

        engine.set_expr("b", "100").unwrap();
        engine.set_expr("a", "10").unwrap();

        assert_value(&engine, "b", 100.0);
    }

    #[test]
    fn recomputes_branching_graph_through_errors_and_recovery() {
        let mut engine = Engine::new();

        engine.set_expr("price", "10").unwrap();
        engine.set_expr("quantity", "3").unwrap();
        engine.set_expr("discount", "2").unwrap();
        engine.set_expr("gross", "price * quantity").unwrap();
        engine.set_expr("net", "gross - discount").unwrap();
        engine.set_expr("fee", "price / (quantity - 1)").unwrap();
        engine
            .set_expr("summary", "net + fee + sin(pi / 2)")
            .unwrap();

        assert_value(&engine, "gross", 30.0);
        assert_value(&engine, "net", 28.0);
        assert_value(&engine, "fee", 5.0);
        assert_value(&engine, "summary", 34.0);

        engine.set_expr("price", "20").unwrap();

        assert_value(&engine, "gross", 60.0);
        assert_value(&engine, "net", 58.0);
        assert_value(&engine, "fee", 10.0);
        assert_value(&engine, "summary", 69.0);

        engine.remove_expr("discount");

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

        engine.set_expr("discount", "8").unwrap();

        assert_value(&engine, "net", 52.0);
        assert_value(&engine, "summary", 63.0);

        engine.set_expr("quantity", "1").unwrap();

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

        engine.set_expr("quantity", "4").unwrap();

        assert_value(&engine, "gross", 80.0);
        assert_value(&engine, "net", 72.0);
        assert_value(&engine, "fee", 20.0 / 3.0);
        assert_value(&engine, "summary", 72.0 + (20.0 / 3.0) + 1.0);
    }

    #[test]
    fn dependency_errors_propagate() {
        let mut engine = Engine::new();

        assert!(engine.set_expr("a", "missing + 1").is_err());
        assert!(engine.set_expr("b", "a * 2").is_err());

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

        assert!(engine.set_expr("a", "b + 1").is_err());
        assert!(engine.set_expr("b", "a + 1").is_err());

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

        assert!(engine.set_expr("a", "a + 1").is_err());

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

        assert!(engine.set_expr("a", "b + 1").is_err());
        assert!(engine.set_expr("b", "a + 1").is_err());
        assert!(engine.set_expr("c", "a + 1").is_err());
        assert!(engine.set_expr("d", "c + 1").is_err());
        engine.set_expr("ok", "10").unwrap();

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

        assert!(engine.set_expr("a", "b + 1").is_err());
        assert!(engine.set_expr("b", "a + 1").is_err());
        assert!(engine.set_expr("x", "y + 1").is_err());
        assert!(engine.set_expr("y", "x + 1").is_err());
        assert!(engine.set_expr("z", "x + a").is_err());

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

        assert!(engine.set_expr("a", "b + 1").is_err());
        assert!(engine.set_expr("b", "a + 1").is_err());
        assert!(engine.set_expr("c", "a + 1").is_err());
        assert!(!engine.cycle_diagnostics().cycle_nodes.is_empty());

        engine.set_expr("a", "1").unwrap();

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

        engine.set_expr("z", "1").unwrap();
        engine.set_expr("a", "z + 1").unwrap();
        assert!(engine.set_expr("cycle_left", "cycle_right + 1").is_err());
        assert!(engine.set_expr("cycle_right", "cycle_left + 1").is_err());

        engine.set_expr("z", "10").unwrap();

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
    fn appends_numbered_results_and_references_them_with_dollar_syntax() {
        let mut engine = Engine::new();

        let (first, first_state) = engine.append_expr("1 + 2");
        let (second, second_state) = engine.append_expr("$1 * 10");

        assert_eq!(first, "$1");
        assert_eq!(second, "$2");
        assert_eq!(first_state, EntryState::Value(3.0));
        assert_eq!(second_state, EntryState::Value(30.0));
        assert_value(&engine, "$2", 30.0);
    }

    #[test]
    fn editing_numbered_result_updates_dollar_dependents() {
        let mut engine = Engine::new();

        engine.append_expr("2");
        engine.append_expr("$1 + 3");
        assert_value(&engine, "$2", 5.0);

        engine.set_expr("$1", "10").unwrap();

        assert_value(&engine, "$2", 13.0);
    }

    #[test]
    fn numbered_results_recompute_branching_graph_through_removal_and_reuse() {
        let mut engine = Engine::new();

        let (first, _) = engine.append_expr("2");
        let (second, _) = engine.append_expr("$1 + 3");
        let (third, _) = engine.append_expr("$1 * $2");
        let (fourth, _) = engine.append_expr("$2 + $3 + sin(pi / 2)");
        let (fifth, _) = engine.append_expr("$4 / ($2 - 5)");

        assert_eq!(first, "$1");
        assert_eq!(second, "$2");
        assert_eq!(third, "$3");
        assert_eq!(fourth, "$4");
        assert_eq!(fifth, "$5");
        assert_value(&engine, "$2", 5.0);
        assert_value(&engine, "$3", 10.0);
        assert_value(&engine, "$4", 16.0);
        assert_eval_error(&engine, "$5", |err| {
            matches!(err, EvalError::DivisionByZero)
        });

        engine.set_expr("$1", "4").unwrap();

        assert_value(&engine, "$2", 7.0);
        assert_value(&engine, "$3", 28.0);
        assert_value(&engine, "$4", 36.0);
        assert_value(&engine, "$5", 18.0);

        engine.remove_expr("$2");

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

        engine.set_expr("$2", "$1 + 6").unwrap();

        assert_value(&engine, "$2", 10.0);
        assert_value(&engine, "$3", 40.0);
        assert_value(&engine, "$4", 51.0);
        assert_value(&engine, "$5", 51.0 / 5.0);

        let (sixth, sixth_state) = engine.append_expr("$5 + $3");

        assert_eq!(sixth, "$6");
        assert_eq!(sixth_state, EntryState::Value(50.2));
        assert_value(&engine, "$6", 50.2);
    }

    #[test]
    fn appending_skips_existing_numbered_results() {
        let mut engine = Engine::new();

        engine.set_expr("$1", "100").unwrap();
        let (id, state) = engine.append_expr("$1 + 1");

        assert_eq!(id, "$2");
        assert_eq!(state, EntryState::Value(101.0));
    }

    #[test]
    fn explicit_numbered_result_reserves_sequential_expression_id() {
        let mut engine = Engine::new();

        engine.set_expr("$5", "100").unwrap();
        let (id, state) = engine.append_expr("$5 + 1");

        assert_eq!(id, "$6");
        assert_eq!(state, EntryState::Value(101.0));
    }

    #[test]
    fn exposes_entries_by_internal_expression_id() {
        let mut engine = Engine::new();

        let (label, _) = engine.append_expr("40 + 2");
        let entry = engine.entry(&label).unwrap();
        let id = entry.id;

        assert_eq!(engine.get_by_id(id), Some(&EntryState::Value(42.0)));
        assert_eq!(engine.entry_by_id(id).unwrap().label, "$1");
    }

    #[test]
    fn named_labels_resolve_to_internal_expression_ids() {
        let mut engine = Engine::new();

        engine.set_expr("subtotal", "100").unwrap();
        let id = engine.entry("subtotal").unwrap().id;
        engine.set_expr("taxed", "subtotal * 1.1").unwrap();
        engine.set_expr("subtotal", "200").unwrap();

        assert_eq!(engine.entry("subtotal").unwrap().id, id);
        assert_value(&engine, "taxed", 220.00000000000003);
    }

    #[test]
    fn exposes_entries_for_read_only_listing() {
        let mut engine = Engine::new();

        engine.set_expr("subtotal", "100").unwrap();
        engine.append_expr("subtotal * 1.1");

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
