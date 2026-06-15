use crate::ast::Expr;
use crate::error::{DagcalError, EvalError};
use crate::eval::eval_expr;
use crate::function::FunctionRegistry;
use crate::parser::parse_expression;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Entry {
    pub source: String,
    pub ast: Option<Expr>,
    pub references: BTreeSet<String>,
    pub state: EntryState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryState {
    Value(f64),
    Error(DagcalError),
}

pub struct Engine {
    entries: HashMap<String, Entry>,
    constants: HashMap<String, f64>,
    functions: FunctionRegistry,
    reverse_dependencies: HashMap<String, BTreeSet<String>>,
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
            constants: HashMap::from([
                ("e".to_string(), std::f64::consts::E),
                ("pi".to_string(), std::f64::consts::PI),
            ]),
            functions: FunctionRegistry::standard(),
            reverse_dependencies: HashMap::new(),
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
        let id = id.into();
        let source = source.into();
        let parsed = parse_expression(&source);

        let entry = match parsed {
            Ok(ast) => Entry {
                source,
                references: ast.references(),
                ast: Some(ast),
                state: EntryState::Error(DagcalError::Eval(EvalError::DependencyError(id.clone()))),
            },
            Err(err) => Entry {
                source,
                ast: None,
                references: BTreeSet::new(),
                state: EntryState::Error(err.clone()),
            },
        };

        self.entries.insert(id.clone(), entry);
        self.rebuild_reverse_dependencies();
        self.recompute_affected(&id);

        match &self.entries[&id].state {
            EntryState::Value(_) => Ok(()),
            EntryState::Error(err) => Err(err.clone()),
        }
    }

    pub fn append_expr(&mut self, source: impl Into<String>) -> (String, EntryState) {
        let id = self.allocate_result_id();
        let _ = self.set_expr(id.clone(), source);
        let state = self.get(&id).cloned().unwrap_or_else(|| {
            EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(id.clone())))
        });

        (id, state)
    }

    pub fn remove_expr(&mut self, id: &str) -> Option<Entry> {
        let affected = self.collect_affected(id);
        let removed = self.entries.remove(id);
        if removed.is_some() {
            self.rebuild_reverse_dependencies();
            self.recompute_ids(affected);
        }
        removed
    }

    pub fn get(&self, id: &str) -> Option<&EntryState> {
        self.entries.get(id).map(|entry| &entry.state)
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.get(id)
    }

    pub fn eval_once(&self, source: &str) -> Result<f64, DagcalError> {
        let ast = parse_expression(source)?;
        let mut resolve = |name: &str| self.resolve_reference(name);
        eval_expr(&ast, &self.functions, &mut resolve).map_err(DagcalError::Eval)
    }

    fn recompute_all(&mut self) {
        let ids = self.entries.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            let state = self.evaluate_entry(&id, &mut Vec::new());
            if let Some(entry) = self.entries.get_mut(&id) {
                entry.state = state;
            }
        }
    }

    fn allocate_result_id(&mut self) -> String {
        loop {
            let id = format!("${}", self.next_result_index);
            self.next_result_index += 1;
            if !self.entries.contains_key(&id) {
                return id;
            }
        }
    }

    fn recompute_affected(&mut self, id: &str) {
        let affected = self.collect_affected(id);
        self.recompute_ids(affected);
    }

    fn collect_affected(&self, id: &str) -> BTreeSet<String> {
        let mut affected = BTreeSet::from([id.to_string()]);
        let mut stack = vec![id.to_string()];

        while let Some(current) = stack.pop() {
            if let Some(dependents) = self.reverse_dependencies.get(&current) {
                for dependent in dependents {
                    if affected.insert(dependent.clone()) {
                        stack.push(dependent.clone());
                    }
                }
            }
        }

        affected
    }

    fn recompute_ids(&mut self, ids: BTreeSet<String>) {
        for current in ids {
            let state = self.evaluate_entry(&current, &mut Vec::new());
            if let Some(entry) = self.entries.get_mut(&current) {
                entry.state = state;
            }
        }
    }

    fn evaluate_entry(&self, id: &str, stack: &mut Vec<String>) -> EntryState {
        if stack.iter().any(|seen| seen == id) {
            return EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(id.to_string())));
        }

        let Some(entry) = self.entries.get(id) else {
            return EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(
                id.to_string(),
            )));
        };

        let Some(ast) = &entry.ast else {
            return entry.state.clone();
        };

        stack.push(id.to_string());
        let mut resolve = |name: &str| self.resolve_reference_with_stack(name, stack);
        let result = eval_expr(ast, &self.functions, &mut resolve);
        stack.pop();

        match result {
            Ok(value) => EntryState::Value(value),
            Err(err) => EntryState::Error(DagcalError::Eval(err)),
        }
    }

    fn resolve_reference(&self, name: &str) -> Result<f64, EvalError> {
        self.resolve_reference_with_stack(name, &mut Vec::new())
    }

    fn resolve_reference_with_stack(
        &self,
        name: &str,
        stack: &mut Vec<String>,
    ) -> Result<f64, EvalError> {
        if self.entries.contains_key(name) {
            match self.evaluate_entry(name, stack) {
                EntryState::Value(value) => Ok(value),
                EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(id))) => {
                    Err(EvalError::CycleDetected(id))
                }
                EntryState::Error(_) => Err(EvalError::DependencyError(name.to_string())),
            }
        } else if let Some(value) = self.constants.get(name) {
            Ok(*value)
        } else {
            Err(EvalError::UnknownReference(name.to_string()))
        }
    }

    fn rebuild_reverse_dependencies(&mut self) {
        let ids = self.entries.keys().cloned().collect::<HashSet<_>>();
        self.reverse_dependencies.clear();

        for (id, entry) in &self.entries {
            for reference in &entry.references {
                if ids.contains(reference) {
                    self.reverse_dependencies
                        .entry(reference.clone())
                        .or_default()
                        .insert(id.clone());
                }
            }
        }
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
    fn appending_skips_existing_numbered_results() {
        let mut engine = Engine::new();

        engine.set_expr("$1", "100").unwrap();
        let (id, state) = engine.append_expr("$1 + 1");

        assert_eq!(id, "$2");
        assert_eq!(state, EntryState::Value(101.0));
    }
}
