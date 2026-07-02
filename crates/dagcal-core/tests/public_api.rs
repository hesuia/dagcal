use dagcal_core::{
    CompletionKind, DagcalError, Engine, EngineSnapshot, EntryState, EntryTarget, EvalError,
    ExpressionId, Number, ParseErrorKind, PersistedEntry, PersistenceError, PreviewState,
    ReferenceTarget,
};

fn id(value: usize) -> ExpressionId {
    ExpressionId::new(value)
}

fn assert_value(engine: &Engine, target: ExpressionId, expected: f64) {
    match engine.state(target) {
        Some(EntryState::Value(actual)) => {
            assert!(
                (actual.to_f64() - expected).abs() < 1e-12,
                "{actual} != {expected}"
            );
        }
        other => panic!("expected value for {target}, got {other:?}"),
    }
}

fn assert_eval_error(
    engine: &Engine,
    target: ExpressionId,
    matches: impl FnOnce(&EvalError) -> bool,
) {
    match engine.state(target) {
        Some(EntryState::Error(DagcalError::Eval(err))) if matches(err) => {}
        other => panic!("expected eval error for {target}, got {other:?}"),
    }
}

#[test]
fn user_session_supports_definitions_results_edits_and_recovery() {
    let mut engine = Engine::new();

    let subtotal = engine.execute("subtotal = 100");
    let tax_rate = engine.execute("tax_rate = 0.1");
    let tax = engine.execute("subtotal * tax_rate");
    let total = engine.execute("subtotal + $3");

    assert_eq!(subtotal.id.to_string(), "$1");
    assert_eq!(subtotal.state, EntryState::Value(Number::from(100.0)));
    assert_eq!(tax_rate.id.to_string(), "$2");
    assert_eq!(tax_rate.state, EntryState::Value(Number::from(0.1)));
    assert_eq!(tax.id.to_string(), "$3");
    assert_eq!(total.id.to_string(), "$4");
    let subtotal_id = subtotal.id;
    let tax_rate_id = tax_rate.id;
    let tax_id = tax.id;
    let total_id = total.id;
    assert_value(&engine, subtotal_id, 100.0);
    assert_value(&engine, tax_id, 10.0);
    assert_value(&engine, total_id, 110.0);

    engine.set_entry(subtotal_id, "200").unwrap();
    assert_value(&engine, subtotal_id, 200.0);
    assert_value(&engine, tax_id, 20.0);
    assert_value(&engine, total_id, 220.0);

    engine.remove_entry(tax_rate_id);
    assert_eval_error(
        &engine,
        tax_id,
        |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(target)) if *target == id(2)),
    );
    assert_eval_error(
        &engine,
        total_id,
        |err| matches!(err, EvalError::DependencyError(target) if *target == id(3)),
    );

    engine.set_entry(tax_rate_id, "0.08").unwrap();
    assert_value(&engine, tax_id, 16.0);
    assert_value(&engine, total_id, 216.0);
}

#[test]
fn public_api_supports_entry_targets_and_id_specific_methods() {
    let mut engine = Engine::new();

    engine.set_entry("subtotal", "100").unwrap();
    engine.set_entry("tax", "subtotal * 0.1").unwrap();
    engine.set_entry("subtotal", "200").unwrap();

    let subtotal_id = engine.entry("subtotal").unwrap().id;
    let tax_id = engine.entry("tax").unwrap().id;

    assert_eq!(subtotal_id, id(1));
    assert_eq!(tax_id, id(2));
    assert_value(&engine, subtotal_id, 200.0);
    assert_eq!(
        engine.state("subtotal"),
        Some(&EntryState::Value(Number::from(200.0)))
    );
    assert_eq!(
        engine.state("$2"),
        Some(&EntryState::Value(Number::from(20.0)))
    );
    assert_eq!(
        engine
            .entry(EntryTarget::Name("tax".to_string()))
            .unwrap()
            .id,
        tax_id
    );

    assert!(
        engine
            .set_entry_by_id(subtotal_id, "300")
            .target_error
            .is_none()
    );
    assert_value(&engine, tax_id, 30.0);

    engine.remove_entry("subtotal").unwrap();
    assert_eval_error(
        &engine,
        tax_id,
        |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(target)) if *target == id(1)),
    );

    engine.set_entry("$1", "400").unwrap();
    assert_eq!(
        engine.entry_by_id(subtotal_id).unwrap().name.as_deref(),
        None
    );
    assert_value(&engine, tax_id, 40.0);
    assert!(engine.remove_entry_by_id(subtotal_id).is_some());
}

#[test]
fn public_api_sets_statement_by_id_as_named_definition() {
    let mut engine = Engine::new();

    let result = engine.set_statement_by_id(id(1), "x = 2");

    assert!(result.target_error.is_none());
    assert_eq!(result.execution.id, id(1));
    let entry = engine.entry_by_id(id(1)).unwrap();
    assert_eq!(entry.name.as_deref(), Some("x"));
    assert_eq!(entry.source, "2");
    assert_value(&engine, id(1), 2.0);
    assert_eq!(engine.entry("x").unwrap().id, id(1));
}

#[test]
fn public_api_set_statement_by_id_renames_without_leaving_old_name() {
    let mut engine = Engine::new();
    engine.set_statement_by_id(id(1), "x = 2");
    engine.execute("x + 1");

    let result = engine.set_statement_by_id(id(1), "y = 3");

    assert!(result.target_error.is_none());
    assert!(engine.entry("x").is_none());
    assert_eq!(engine.entry("y").unwrap().id, id(1));
    assert_value(&engine, id(1), 3.0);
    assert_value(&engine, id(2), 4.0);

    let stale_name = engine.execute("x + 1");
    assert_eval_error(
        &engine,
        stale_name.id,
        |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Name(name)) if name == "x"),
    );
}

#[test]
fn public_api_set_statement_by_id_stores_statement_parse_errors_on_target() {
    let mut engine = Engine::new();

    let result = engine.set_statement_by_id(id(1), "broken = 1 +");

    assert_eq!(result.execution.id, id(1));
    assert!(result.target_error.is_some());
    let entry = engine.entry_by_id(id(1)).unwrap();
    assert_eq!(entry.name, None);
    assert_eq!(entry.source, "broken = 1 +");
    assert!(matches!(
        entry.state,
        EntryState::Error(DagcalError::Parse(_))
    ));
}

#[test]
fn public_api_reports_invalid_entry_targets_as_structured_parse_errors() {
    let mut engine = Engine::new();

    let err = engine.set_entry("$0", "1").unwrap_err();

    match err {
        DagcalError::Parse(err) => {
            assert_eq!(err.kind, ParseErrorKind::InvalidEntryTarget);
            let span = err.span.unwrap();
            assert_eq!(span.start.byte, 0);
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1);
            assert_eq!(span.end.byte, 2);
        }
        other => panic!("expected parse error, got {other:?}"),
    }
}

#[test]
fn public_api_set_entry_reports_saved_target_errors_without_losing_execution() {
    let mut engine = Engine::new();

    let valid = engine.set_entry("subtotal", "100").unwrap();
    assert_eq!(valid.execution.id, id(1));
    assert!(valid.target_error.is_none());

    let broken = engine.set_entry("subtotal", "1 +").unwrap();

    assert_eq!(broken.execution.id, id(1));
    assert_eq!(broken.execution.affected_ids, [id(1)].into_iter().collect());
    assert!(matches!(broken.target_error, Some(DagcalError::Parse(_))));
    assert!(matches!(
        engine.state("subtotal"),
        Some(EntryState::Error(DagcalError::Parse(_)))
    ));
}

#[test]
fn public_api_set_entry_by_id_returns_execution_for_error_states() {
    let mut engine = Engine::new();

    let base = engine.execute("base = 10").id;
    let dependent = engine.execute("base * 2").id;

    let result = engine.set_entry_by_id(base, "1 +");

    assert_eq!(result.execution.id, base);
    assert_eq!(
        result.execution.affected_ids,
        [base, dependent].into_iter().collect()
    );
    assert!(result.target_error.is_some());
    assert!(matches!(
        engine.state(base),
        Some(EntryState::Error(DagcalError::Parse(_)))
    ));
    assert_eval_error(
        &engine,
        dependent,
        |err| matches!(err, EvalError::DependencyError(target) if *target == base),
    );
}

#[test]
fn public_api_executes_standalone_number_literals() {
    let mut engine = Engine::new();

    let integer = engine.execute("10");
    let decimal = engine.execute("4.2");
    let binary = engine.execute("0b1001.1101");
    let octal = engine.execute("0o10.4");
    let hexadecimal = engine.execute("0xA.F");

    assert_eq!(integer.id.to_string(), "$1");
    assert_eq!(integer.state, EntryState::Value(Number::from(10.0)));
    assert_eq!(decimal.id.to_string(), "$2");
    assert_eq!(decimal.state, EntryState::Value(Number::from(4.2)));
    assert_eq!(binary.id.to_string(), "$3");
    assert_eq!(binary.state, EntryState::Value(Number::from(9.8125)));
    assert_eq!(octal.id.to_string(), "$4");
    assert_eq!(octal.state, EntryState::Value(Number::from(8.5)));
    assert_eq!(hexadecimal.id.to_string(), "$5");
    assert_eq!(hexadecimal.state, EntryState::Value(Number::from(10.9375)));
    assert_value(&engine, integer.id, 10.0);
    assert_value(&engine, decimal.id, 4.2);
    assert_value(&engine, binary.id, 9.8125);
    assert_value(&engine, octal.id, 8.5);
    assert_value(&engine, hexadecimal.id, 10.9375);
    assert_eq!(
        engine.eval_once("0xA.F + 0b.1").unwrap(),
        Number::from(11.4375)
    );
}

#[test]
fn public_api_preserves_exact_fraction_arithmetic() {
    let mut engine = Engine::new();

    let decimal_sum = engine.execute("0.1 + 0.2");
    let divided_then_scaled = engine.execute("1 / 3 * 3");
    let based_sum = engine.execute("0xA.F + 0b.1");

    assert_eq!(
        decimal_sum.state,
        EntryState::Value(Number::rational(3, 10))
    );
    assert_eq!(
        divided_then_scaled.state,
        EntryState::Value(Number::from(1))
    );
    assert_eq!(
        based_sum.state,
        EntryState::Value(Number::rational(183, 16))
    );
}

#[test]
fn public_api_keeps_approximate_results_at_float_boundaries() {
    let mut engine = Engine::new();

    let pi_plus_one = engine.execute("pi + 1");
    let sine = engine.execute("sin(pi / 2)");

    assert!(matches!(
        pi_plus_one.state,
        EntryState::Value(Number::Float(_))
    ));
    assert!(matches!(sine.state, EntryState::Value(Number::Float(_))));
    assert_value(&engine, sine.id, 1.0);
}

#[test]
fn public_api_reports_parse_and_cycle_errors_without_losing_valid_entries() {
    let mut engine = Engine::new();

    let valid = engine.execute("base = 10");
    let parse_error = engine.execute("broken = 1 +");
    let cycle_a = engine.execute("a = 1");
    let cycle_b = engine.execute("b = 2");
    let cycle_a_id = cycle_a.id;
    let cycle_b_id = cycle_b.id;
    engine.set_entry(cycle_a_id, "b + 1").unwrap();
    assert!(
        engine
            .set_entry(cycle_b_id, "a + 1")
            .unwrap()
            .target_error
            .is_some()
    );
    let dependent = engine.execute("a + base");

    assert_eq!(valid.state, EntryState::Value(Number::from(10.0)));
    let parse_error_id = parse_error.id;
    assert_eq!(parse_error_id.to_string(), "$2");
    match parse_error.state {
        EntryState::Error(DagcalError::Parse(err)) => {
            assert_eq!(err.kind, ParseErrorKind::Syntax);
            assert!(err.span.is_some());
        }
        other => panic!("expected parse error, got {other:?}"),
    }
    assert!(matches!(
        engine.state(parse_error_id),
        Some(EntryState::Error(DagcalError::Parse(_)))
    ));
    assert_eq!(cycle_a.state, EntryState::Value(Number::from(1.0)));
    assert_eq!(cycle_b.state, EntryState::Value(Number::from(2.0)));
    assert!(matches!(
        engine.state(cycle_b_id),
        Some(EntryState::Error(DagcalError::Eval(
            EvalError::CycleDetected(target)
        ))) if *target == cycle_b_id
    ));
    let dependent_id = dependent.id;
    assert_eq!(dependent_id.to_string(), "$5");
    assert_eval_error(
        &engine,
        dependent_id,
        |err| matches!(err, EvalError::DependencyError(target) if *target == id(3)),
    );
    assert_value(&engine, valid.id, 10.0);
}

#[test]
fn public_api_supports_runtime_extensions_used_by_frontends() {
    let mut engine = Engine::new();

    let before_function = engine.execute("triple(14)");
    engine.set_constant("tau", 6.0);
    let before_constant = engine.execute("tau / 2");

    assert_eq!(before_function.id.to_string(), "$1");
    assert_eq!(before_constant.id.to_string(), "$2");
    assert_eval_error(
        &engine,
        before_function.id,
        |err| matches!(err, EvalError::UnknownFunction(name) if name == "triple"),
    );
    assert_value(&engine, before_constant.id, 3.0);

    engine.register_fixed_function("triple", 1, |args| Ok(args[0].clone() * Number::from(3)));
    engine.set_constant("tau", std::f64::consts::TAU);

    assert_value(&engine, before_function.id, 42.0);
    assert_value(&engine, before_constant.id, std::f64::consts::PI);
}

#[test]
fn public_api_normalizes_non_finite_runtime_extensions_to_math_errors() {
    let mut engine = Engine::new();

    engine.set_constant("tau", 6.0);
    let constant = engine.execute("tau + 1");
    let function = engine.execute("explode()");

    engine.set_constant("tau", f64::NAN);
    engine.register_fixed_function("explode", 0, |_| Ok(Number::Float(f64::INFINITY)));

    assert_eq!(constant.id.to_string(), "$1");
    assert_eq!(function.id.to_string(), "$2");
    assert_eval_error(
        &engine,
        constant.id,
        |err| matches!(err, EvalError::Math(message) if message == "constant `tau` produced non-finite result"),
    );
    assert_eval_error(
        &engine,
        function.id,
        |err| matches!(err, EvalError::Math(message) if message == "function `explode` produced non-finite result"),
    );
}

#[test]
fn public_api_exposes_entries_for_frontend_state_rendering() {
    let mut engine = Engine::new();

    engine.execute("subtotal = 120");
    engine.execute("tax = subtotal * 0.1");
    let total = engine.execute("subtotal + tax");
    let total_id = total.id;

    assert_eq!(
        engine.state(total_id),
        Some(&EntryState::Value(Number::from(132.0)))
    );

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

    assert_eq!(
        entries,
        vec![
            (
                "$1".to_string(),
                "120".to_string(),
                EntryState::Value(Number::from(120.0)),
            ),
            (
                "$2".to_string(),
                "subtotal * 0.1".to_string(),
                EntryState::Value(Number::from(12.0)),
            ),
            (
                "$3".to_string(),
                "subtotal + tax".to_string(),
                EntryState::Value(Number::from(132.0)),
            ),
        ]
    );
}

#[test]
fn public_api_reports_changed_entries_for_frontend_updates() {
    let mut engine = Engine::new();

    let subtotal = engine.execute("subtotal = 120");
    let tax = engine.execute("tax = subtotal * 0.1");
    let total = engine.execute("subtotal + tax");
    let unrelated = engine.execute("5");

    assert_eq!(subtotal.affected_ids, [id(1)].into_iter().collect());
    assert_eq!(total.affected_ids, [id(3)].into_iter().collect());

    let updated = engine.set_entry("subtotal", "200").unwrap();
    assert_eq!(updated.execution.id, subtotal.id);
    assert!(updated.target_error.is_none());
    assert_eq!(
        updated.execution.affected_ids,
        [subtotal.id, tax.id, total.id].into_iter().collect()
    );
    assert_value(&engine, tax.id, 20.0);
    assert_value(&engine, total.id, 220.0);
    assert_value(&engine, unrelated.id, 5.0);

    let broken = engine.execute("1 +");
    assert_eq!(broken.affected_ids, [id(5)].into_iter().collect());
    assert!(matches!(
        broken.state,
        EntryState::Error(DagcalError::Parse(_))
    ));
}

#[test]
fn public_api_reports_removed_entry_and_recomputed_dependents() {
    let mut engine = Engine::new();

    let subtotal = engine.execute("subtotal = 100");
    let tax = engine.execute("subtotal * 0.1");
    let total = engine.execute("subtotal + $2");

    let removal = engine.remove_entry(subtotal.id).unwrap();

    assert_eq!(removal.removed_entry.id, subtotal.id);
    assert_eq!(removal.removed_entry.source, "100");
    assert_eq!(
        removal.affected_ids,
        [subtotal.id, tax.id, total.id].into_iter().collect()
    );
    assert!(engine.entry(subtotal.id).is_none());
    assert_eval_error(
        &engine,
        tax.id,
        |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(target)) if *target == subtotal.id),
    );
    assert_eval_error(
        &engine,
        total.id,
        |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(target)) if *target == subtotal.id),
    );
}

#[test]
fn public_api_exposes_sorted_entry_accessors_without_full_listing() {
    let mut engine = Engine::new();

    let first = engine.execute("10");
    let second = engine.execute("20");
    let third = engine.execute("30");
    engine.remove_entry(second.id);

    assert_eq!(engine.entry_count(), 2);
    assert_eq!(engine.entry_ids(), vec![first.id, third.id]);
    assert_eq!(engine.entry_at_index(0).unwrap().id, first.id);
    assert_eq!(engine.entry_at_index(1).unwrap().id, third.id);
    assert!(engine.entry_at_index(2).is_none());
}

#[test]
fn public_api_exposes_dependency_queries_by_expression_id() {
    let mut engine = Engine::new();

    let subtotal = engine.execute("subtotal = 100");
    let tax = engine.execute("tax = subtotal * 0.1");
    let discount = engine.execute("discount = subtotal * 0.05");
    let total = engine.execute("subtotal + tax - discount");
    let unrelated = engine.execute("1");

    assert_eq!(
        engine.dependencies_of(total.id),
        [subtotal.id, tax.id, discount.id].into_iter().collect()
    );
    assert_eq!(
        engine.dependents_of(subtotal.id),
        [tax.id, discount.id, total.id].into_iter().collect()
    );
    assert_eq!(
        engine.affected_by(subtotal.id),
        [subtotal.id, tax.id, discount.id, total.id]
            .into_iter()
            .collect()
    );
    assert!(engine.dependencies_of(unrelated.id).is_empty());
    assert!(engine.dependents_of(unrelated.id).is_empty());
}

#[test]
fn public_api_manual_recompute_resolves_names_defined_later() {
    let mut engine = Engine::new();

    let expression = engine.execute("x + 1");
    let dependent = engine.execute("$1 * 2");
    assert_eval_error(
        &engine,
        expression.id,
        |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Name(name)) if name == "x"),
    );

    engine.execute("x = 3");
    let affected = engine
        .recompute_entry_by_id(expression.id)
        .expect("entry should exist");

    assert_eq!(
        affected,
        [expression.id, dependent.id].into_iter().collect()
    );
    assert_value(&engine, expression.id, 4.0);
    assert_value(&engine, dependent.id, 8.0);
}

#[test]
fn public_api_previews_expressions_without_mutating_engine_state() {
    let mut engine = Engine::new();

    let subtotal = engine.execute("subtotal = 100").id;
    engine.set_constant("tau", std::f64::consts::TAU);
    engine.register_fixed_function("triple", 1, |args| Ok(args[0].clone() * Number::from(3)));
    let before_count = engine.entry_count();

    let preview = engine.preview_expression("subtotal + tau + triple(2)");

    assert_eq!(preview.source, "subtotal + tau + triple(2)");
    assert_eq!(preview.state, PreviewState::Valid);
    assert_eq!(preview.entry_references, [subtotal].into_iter().collect());
    assert_eq!(
        preview.constant_references,
        ["tau".to_string()].into_iter().collect()
    );
    assert_eq!(
        preview.function_references,
        ["triple".to_string()].into_iter().collect()
    );
    assert_eq!(engine.entry_count(), before_count);
}

#[test]
fn public_api_previews_parse_and_resolve_errors_without_mutating_engine_state() {
    let mut engine = Engine::new();

    engine.execute("base = 10");
    let before = engine.snapshot();

    let parse_error = engine.preview_expression("1 +");
    assert!(matches!(
        parse_error.state,
        PreviewState::Error(DagcalError::Parse(_))
    ));

    let resolve_error = engine.preview_expression("missing + 1");
    assert!(matches!(
        resolve_error.state,
        PreviewState::Error(DagcalError::Eval(EvalError::UnknownReference(
            ReferenceTarget::Name(name)
        ))) if name == "missing"
    ));
    assert_eq!(engine.snapshot(), before);
}

#[test]
fn public_api_exposes_completion_items_for_frontends() {
    let mut engine = Engine::new();

    let subtotal = engine.execute("subtotal = 100").id;
    let plain = engine.execute("subtotal * 2").id;
    engine.set_constant("tau", std::f64::consts::TAU);
    engine.register_fixed_function("triple", 1, |args| Ok(args[0].clone() * Number::from(3)));

    let items = engine.completion_items();

    assert!(items.iter().any(|item| {
        item.kind == CompletionKind::Entry
            && item.label == "subtotal"
            && item.detail == Some(subtotal.to_string())
    }));
    assert!(items.iter().any(|item| {
        item.kind == CompletionKind::Result
            && item.label == plain.to_string()
            && item.detail.is_none()
    }));
    assert!(
        items
            .iter()
            .any(|item| { item.kind == CompletionKind::Constant && item.label == "tau" })
    );
    assert!(items.iter().any(|item| {
        item.kind == CompletionKind::Function
            && item.label == "triple"
            && item.detail.as_deref() == Some("1 argument(s)")
    }));
}

#[test]
fn public_api_keeps_numbered_results_stable_across_removal_and_append() {
    let mut engine = Engine::new();

    let first = engine.execute("2");
    let second = engine.execute("$1 + 3");
    let third = engine.execute("$1 * $2");

    assert_eq!(first.id.to_string(), "$1");
    assert_eq!(second.id.to_string(), "$2");
    assert_eq!(third.id.to_string(), "$3");
    let second_id = second.id;
    let third_id = third.id;
    assert_value(&engine, third_id, 10.0);

    engine.remove_entry(second_id);
    assert_eval_error(
        &engine,
        third_id,
        |err| matches!(err, EvalError::UnknownReference(ReferenceTarget::Id(target)) if *target == id(2)),
    );

    let fourth = engine.execute("$1 + 10");
    assert_eq!(fourth.id.to_string(), "$4");
    assert_value(&engine, fourth.id, 12.0);

    assert!(
        engine
            .set_entry(second_id, "$1 + 4")
            .unwrap()
            .target_error
            .is_none()
    );
    assert_value(&engine, third_id, 12.0);
}

#[test]
fn public_api_undo_redo_restores_appended_entries_with_stable_ids() {
    let mut engine = Engine::new();

    let first = engine.execute("10").id;
    let second = engine.execute("$1 * 2").id;
    assert_eq!(second, id(2));
    assert!(engine.can_undo());
    assert!(!engine.can_redo());

    assert!(engine.undo());
    assert!(engine.entry_by_id(second).is_none());
    assert_eq!(
        engine.state_by_id(first),
        Some(&EntryState::Value(Number::from(10)))
    );
    assert!(engine.can_redo());

    assert!(engine.redo());
    assert_eq!(
        engine.state_by_id(second),
        Some(&EntryState::Value(Number::from(20)))
    );
}

#[test]
fn public_api_undo_restores_edits_and_recomputes_dependents() {
    let mut engine = Engine::new();
    let base = engine.execute("base = 10").id;
    let dependent = engine.execute("base * 2").id;

    engine.set_entry(base, "20").unwrap();
    assert_eq!(
        engine.state_by_id(dependent),
        Some(&EntryState::Value(Number::from(40)))
    );

    assert!(engine.undo());
    assert_eq!(
        engine.state_by_id(base),
        Some(&EntryState::Value(Number::from(10)))
    );
    assert_eq!(
        engine.state_by_id(dependent),
        Some(&EntryState::Value(Number::from(20)))
    );
}

#[test]
fn public_api_undo_restores_removed_entries_and_redo_removes_them_again() {
    let mut engine = Engine::new();
    let base = engine.execute("base = 10").id;
    let dependent = engine.execute("base * 2").id;

    engine.remove_entry(base).unwrap();
    assert!(matches!(
        engine.state_by_id(dependent),
        Some(EntryState::Error(_))
    ));

    assert!(engine.undo());
    assert_eq!(
        engine.state_by_id(base),
        Some(&EntryState::Value(Number::from(10)))
    );
    assert_eq!(
        engine.state_by_id(dependent),
        Some(&EntryState::Value(Number::from(20)))
    );

    assert!(engine.redo());
    assert!(engine.entry_by_id(base).is_none());
    assert!(matches!(
        engine.state_by_id(dependent),
        Some(EntryState::Error(_))
    ));
}

#[test]
fn public_api_new_mutation_after_undo_clears_redo_history() {
    let mut engine = Engine::new();

    engine.execute("1");
    engine.execute("2");
    assert!(engine.undo());
    assert!(engine.can_redo());

    engine.execute("3");
    assert!(!engine.can_redo());
    assert!(!engine.redo());
    assert_eq!(engine.entry_ids(), vec![id(1), id(2)]);
    assert_eq!(
        engine.state_by_id(id(2)),
        Some(&EntryState::Value(Number::from(3)))
    );
}

#[test]
fn public_api_clear_is_undoable() {
    let mut engine = Engine::new();
    engine.execute("1");
    engine.execute("2");

    engine.clear();
    assert_eq!(engine.entry_count(), 0);

    assert!(engine.undo());
    assert_eq!(engine.entry_ids(), vec![id(1), id(2)]);
}

#[test]
fn public_api_failed_noop_operations_do_not_create_history_entries() {
    let mut engine = Engine::new();

    assert!(engine.remove_entry("missing").is_none());
    assert!(!engine.can_undo());

    assert!(engine.set_entry("$0", "1").is_err());
    assert!(!engine.can_undo());
}

#[test]
fn public_api_runtime_extensions_survive_undo_redo_restores() {
    let mut engine = Engine::new();
    engine.register_fixed_function("triple", 1, |args| Ok(args[0].clone() * Number::from(3)));
    engine.set_constant("tau", Number::from(6));

    let value = engine.execute("triple(2) + tau").id;
    assert_eq!(
        engine.state_by_id(value),
        Some(&EntryState::Value(Number::from(12)))
    );

    assert!(engine.undo());
    assert!(engine.redo());
    assert_eq!(
        engine.state_by_id(value),
        Some(&EntryState::Value(Number::from(12)))
    );
}

#[test]
fn public_api_serializes_and_restores_engine_snapshots() {
    let mut engine = Engine::new();

    engine.execute("subtotal = 100");
    engine.execute("tax_rate = 0.1");
    engine.execute("subtotal * tax_rate");
    engine.execute("subtotal + $3");

    let json = serde_json::to_string(&engine.snapshot()).unwrap();
    let snapshot: EngineSnapshot = serde_json::from_str(&json).unwrap();
    let mut restored = Engine::from_snapshot(snapshot).unwrap();

    assert_value(&restored, id(1), 100.0);
    assert_value(&restored, id(2), 0.1);
    assert_value(&restored, id(3), 10.0);
    assert_value(&restored, id(4), 110.0);

    assert!(
        restored
            .set_entry(id(1), "200")
            .unwrap()
            .target_error
            .is_none()
    );
    assert_value(&restored, id(3), 20.0);
    assert_value(&restored, id(4), 220.0);
}

#[test]
fn public_api_restore_preserves_removed_id_gaps_and_next_append_id() {
    let mut engine = Engine::new();

    engine.execute("1");
    engine.execute("2");
    engine.execute("3");
    engine.remove_entry(id(2));

    let mut restored = Engine::from_snapshot(engine.snapshot()).unwrap();
    let next = restored.execute("$1 + $3");

    assert!(restored.entry(id(2)).is_none());
    assert_eq!(next.id.to_string(), "$4");
    assert_value(&restored, next.id, 4.0);
}

#[test]
fn public_api_restore_rebuilds_cycle_diagnostics() {
    let mut engine = Engine::new();

    let a = engine.execute("a = 1").id;
    let b = engine.execute("b = 2").id;
    engine.set_entry(a, "b + 1").unwrap();
    assert!(engine.set_entry(b, "a + 1").unwrap().target_error.is_some());

    let restored = Engine::from_snapshot(engine.snapshot()).unwrap();
    let diagnostics = restored.cycle_diagnostics();

    assert!(matches!(
        restored.state(a),
        Some(EntryState::Error(DagcalError::Eval(
            EvalError::CycleDetected(target)
        ))) if *target == a
    ));
    assert_eq!(
        diagnostics.cycle_nodes,
        [id(1), id(2)].into_iter().collect()
    );
}

#[test]
fn public_api_restore_preserves_stored_parse_error_entries() {
    let mut engine = Engine::new();

    let broken = engine.execute("broken = 0").id;
    assert!(
        engine
            .set_entry(broken, "1 +")
            .unwrap()
            .target_error
            .is_some()
    );
    let restored = Engine::from_snapshot(engine.snapshot()).unwrap();

    match restored.state(broken) {
        Some(EntryState::Error(DagcalError::Parse(err))) => {
            assert_eq!(err.kind, ParseErrorKind::Syntax);
        }
        other => panic!("expected stored parse error, got {other:?}"),
    }
}

#[test]
fn public_api_restore_rejects_invalid_snapshots() {
    let unsupported_version = EngineSnapshot {
        version: 999,
        entries: vec![],
    };
    assert!(matches!(
        Engine::from_snapshot(unsupported_version),
        Err(DagcalError::Persistence(
            PersistenceError::UnsupportedVersion { actual: 999, .. }
        ))
    ));

    let invalid_id = EngineSnapshot::new(vec![PersistedEntry {
        id: 0,
        name: None,
        source: "1".to_string(),
    }]);
    assert!(matches!(
        Engine::from_snapshot(invalid_id),
        Err(DagcalError::Persistence(PersistenceError::InvalidId(0)))
    ));

    let duplicate_id = EngineSnapshot::new(vec![
        PersistedEntry {
            id: 1,
            name: None,
            source: "1".to_string(),
        },
        PersistedEntry {
            id: 1,
            name: None,
            source: "2".to_string(),
        },
    ]);
    assert!(matches!(
        Engine::from_snapshot(duplicate_id),
        Err(DagcalError::Persistence(PersistenceError::DuplicateId(1)))
    ));

    let invalid_name = EngineSnapshot::new(vec![PersistedEntry {
        id: 1,
        name: Some("not-valid".to_string()),
        source: "1".to_string(),
    }]);
    assert!(matches!(
        Engine::from_snapshot(invalid_name),
        Err(DagcalError::Persistence(PersistenceError::InvalidName(
            name
        ))) if name == "not-valid"
    ));

    let duplicate_name = EngineSnapshot::new(vec![
        PersistedEntry {
            id: 1,
            name: Some("same".to_string()),
            source: "1".to_string(),
        },
        PersistedEntry {
            id: 2,
            name: Some("same".to_string()),
            source: "2".to_string(),
        },
    ]);
    assert!(matches!(
        Engine::from_snapshot(duplicate_name),
        Err(DagcalError::Persistence(PersistenceError::DuplicateName(
            name
        ))) if name == "same"
    ));
}
