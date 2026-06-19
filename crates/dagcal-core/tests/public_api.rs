use dagcal_core::{DagcalError, Engine, EntryState, EvalError};

fn assert_value(engine: &Engine, label: &str, expected: f64) {
    match engine.get(label) {
        Some(EntryState::Value(actual)) => {
            assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
        }
        other => panic!("expected value for {label}, got {other:?}"),
    }
}

fn assert_eval_error(engine: &Engine, label: &str, matches: impl FnOnce(&EvalError) -> bool) {
    match engine.get(label) {
        Some(EntryState::Error(DagcalError::Eval(err))) if matches(err) => {}
        other => panic!("expected eval error for {label}, got {other:?}"),
    }
}

#[test]
fn user_session_supports_definitions_results_edits_and_recovery() {
    let mut engine = Engine::new();

    let subtotal = engine.execute("subtotal = 100");
    let tax_rate = engine.execute("tax_rate = 0.1");
    let tax = engine.execute("subtotal * tax_rate");
    let total = engine.execute("subtotal + $3");

    assert_eq!(subtotal.label.unwrap().to_string(), "$1");
    assert_eq!(subtotal.state, EntryState::Value(100.0));
    assert_eq!(tax_rate.label.unwrap().to_string(), "$2");
    assert_eq!(tax_rate.state, EntryState::Value(0.1));
    assert_eq!(tax.label.unwrap().to_string(), "$3");
    assert_eq!(total.label.unwrap().to_string(), "$4");
    assert_value(&engine, "$1", 100.0);
    assert_value(&engine, "$3", 10.0);
    assert_value(&engine, "$4", 110.0);

    engine.set_entry("subtotal", "200").unwrap();
    assert_value(&engine, "$1", 200.0);
    assert_value(&engine, "$3", 20.0);
    assert_value(&engine, "$4", 220.0);

    engine.remove_entry("tax_rate");
    assert_eval_error(
        &engine,
        "$3",
        |err| matches!(err, EvalError::UnknownReference(name) if name == "tax_rate"),
    );
    assert_eval_error(
        &engine,
        "$4",
        |err| matches!(err, EvalError::DependencyError(name) if name == "$3"),
    );

    engine.set_entry("tax_rate", "0.08").unwrap();
    assert_value(&engine, "$3", 16.0);
    assert_value(&engine, "$4", 216.0);
}

#[test]
fn public_api_reports_parse_and_cycle_errors_without_losing_valid_entries() {
    let mut engine = Engine::new();

    let valid = engine.execute("base = 10");
    let parse_error = engine.execute("broken = 1 +");
    let cycle_a = engine.execute("a = b + 1");
    let cycle_b = engine.execute("b = a + 1");
    let dependent = engine.execute("a + base");

    assert_eq!(valid.state, EntryState::Value(10.0));
    assert!(parse_error.label.is_none());
    assert!(matches!(
        parse_error.state,
        EntryState::Error(DagcalError::Parse(_))
    ));
    assert!(matches!(
        cycle_a.state,
        EntryState::Error(DagcalError::Eval(EvalError::UnknownReference(_)))
            | EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(_)))
    ));
    assert!(matches!(
        cycle_b.state,
        EntryState::Error(DagcalError::Eval(EvalError::CycleDetected(_)))
    ));
    assert_eq!(dependent.label.unwrap().to_string(), "$4");
    assert_eval_error(
        &engine,
        "$4",
        |err| matches!(err, EvalError::DependencyError(name) if name == "a"),
    );
    assert_value(&engine, "base", 10.0);
}

#[test]
fn public_api_supports_runtime_extensions_used_by_frontends() {
    let mut engine = Engine::new();

    let before_function = engine.execute("triple(14)");
    let before_constant = engine.execute("tau / 2");

    assert_eq!(before_function.label.unwrap().to_string(), "$1");
    assert_eq!(before_constant.label.unwrap().to_string(), "$2");
    assert_eval_error(
        &engine,
        "$1",
        |err| matches!(err, EvalError::UnknownFunction(name) if name == "triple"),
    );
    assert_eval_error(
        &engine,
        "$2",
        |err| matches!(err, EvalError::UnknownReference(name) if name == "tau"),
    );

    engine.register_fixed_function("triple", 1, |args| Ok(args[0] * 3.0));
    engine.set_constant("tau", std::f64::consts::TAU);

    assert_value(&engine, "$1", 42.0);
    assert_value(&engine, "$2", std::f64::consts::PI);
}

#[test]
fn public_api_exposes_entries_for_frontend_state_rendering() {
    let mut engine = Engine::new();

    engine.execute("subtotal = 120");
    engine.execute("tax = subtotal * 0.1");
    let total = engine.execute("subtotal + tax");
    let total_label = total.label.unwrap().to_string();
    let total_id = engine.entry(&total_label).unwrap().id;

    assert_eq!(engine.get_by_id(total_id), Some(&EntryState::Value(132.0)));

    let mut entries = engine
        .entries()
        .map(|(label, entry)| (label.to_string(), entry.source.clone(), entry.state.clone()))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    assert_eq!(
        entries,
        vec![
            (
                "$1".to_string(),
                "120".to_string(),
                EntryState::Value(120.0),
            ),
            (
                "$2".to_string(),
                "subtotal * 0.1".to_string(),
                EntryState::Value(12.0),
            ),
            (
                "$3".to_string(),
                "subtotal + tax".to_string(),
                EntryState::Value(132.0),
            ),
        ]
    );
}

#[test]
fn public_api_keeps_numbered_results_stable_across_removal_and_append() {
    let mut engine = Engine::new();

    let first = engine.execute("2");
    let second = engine.execute("$1 + 3");
    let third = engine.execute("$1 * $2");

    assert_eq!(first.label.unwrap().to_string(), "$1");
    assert_eq!(second.label.unwrap().to_string(), "$2");
    assert_eq!(third.label.unwrap().to_string(), "$3");
    assert_value(&engine, "$3", 10.0);

    engine.remove_entry("$2");
    assert_eval_error(
        &engine,
        "$3",
        |err| matches!(err, EvalError::UnknownReference(name) if name == "$2"),
    );

    let fourth = engine.execute("$1 + 10");
    assert_eq!(fourth.label.unwrap().to_string(), "$4");
    assert_value(&engine, "$4", 12.0);

    engine.set_entry("$2", "$1 + 4").unwrap();
    assert_value(&engine, "$3", 12.0);
}
