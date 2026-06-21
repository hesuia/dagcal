use dagcal_core::{DagcalError, Engine, EntryState, EvalError};

fn assert_value(engine: &Engine, target: &str, expected: f64) {
    match engine.state(target) {
        Some(EntryState::Value(actual)) => {
            assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
        }
        other => panic!("expected value for {target}, got {other:?}"),
    }
}

fn assert_eval_error(engine: &Engine, target: &str, matches: impl FnOnce(&EvalError) -> bool) {
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

    assert_eq!(subtotal.id.unwrap().to_string(), "$1");
    assert_eq!(subtotal.state, EntryState::Value(100.0));
    assert_eq!(tax_rate.id.unwrap().to_string(), "$2");
    assert_eq!(tax_rate.state, EntryState::Value(0.1));
    assert_eq!(tax.id.unwrap().to_string(), "$3");
    assert_eq!(total.id.unwrap().to_string(), "$4");
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
        |err| matches!(err, EvalError::UnknownReference(name) if name == "$2"),
    );
    assert_eval_error(
        &engine,
        "$4",
        |err| matches!(err, EvalError::DependencyError(name) if name == "$3"),
    );

    engine.set_entry("$2", "0.08").unwrap();
    assert_value(&engine, "$3", 16.0);
    assert_value(&engine, "$4", 216.0);
}

#[test]
fn public_api_reports_parse_and_cycle_errors_without_losing_valid_entries() {
    let mut engine = Engine::new();

    let valid = engine.execute("base = 10");
    let parse_error = engine.execute("broken = 1 +");
    let cycle_a = engine.execute("a = 1");
    let cycle_b = engine.execute("b = 2");
    engine.set_entry("a", "b + 1").unwrap();
    assert!(engine.set_entry("b", "a + 1").is_err());
    let dependent = engine.execute("a + base");

    assert_eq!(valid.state, EntryState::Value(10.0));
    assert!(parse_error.id.is_none());
    assert!(matches!(
        parse_error.state,
        EntryState::Error(DagcalError::Parse(_))
    ));
    assert_eq!(cycle_a.state, EntryState::Value(1.0));
    assert_eq!(cycle_b.state, EntryState::Value(2.0));
    assert!(matches!(
        engine.state("b"),
        Some(EntryState::Error(DagcalError::Eval(
            EvalError::CycleDetected(_)
        )))
    ));
    assert_eq!(dependent.id.unwrap().to_string(), "$4");
    assert_eval_error(
        &engine,
        "$4",
        |err| matches!(err, EvalError::DependencyError(name) if name == "$2"),
    );
    assert_value(&engine, "base", 10.0);
}

#[test]
fn public_api_supports_runtime_extensions_used_by_frontends() {
    let mut engine = Engine::new();

    let before_function = engine.execute("triple(14)");
    engine.set_constant("tau", 6.0);
    let before_constant = engine.execute("tau / 2");

    assert_eq!(before_function.id.unwrap().to_string(), "$1");
    assert_eq!(before_constant.id.unwrap().to_string(), "$2");
    assert_eval_error(
        &engine,
        "$1",
        |err| matches!(err, EvalError::UnknownFunction(name) if name == "triple"),
    );
    assert_value(&engine, "$2", 3.0);

    engine.register_fixed_function("triple", 1, |args| Ok(args[0] * 3.0));
    engine.set_constant("tau", std::f64::consts::TAU);

    assert_value(&engine, "$1", 42.0);
    assert_value(&engine, "$2", std::f64::consts::PI);
}

#[test]
fn public_api_normalizes_non_finite_runtime_extensions_to_math_errors() {
    let mut engine = Engine::new();

    engine.set_constant("tau", 6.0);
    let constant = engine.execute("tau + 1");
    let function = engine.execute("explode()");

    engine.set_constant("tau", f64::NAN);
    engine.register_fixed_function("explode", 0, |_| Ok(f64::INFINITY));

    assert_eq!(constant.id.unwrap().to_string(), "$1");
    assert_eq!(function.id.unwrap().to_string(), "$2");
    assert_eval_error(
        &engine,
        "$1",
        |err| matches!(err, EvalError::Math(message) if message == "constant `tau` produced non-finite result"),
    );
    assert_eval_error(
        &engine,
        "$2",
        |err| matches!(err, EvalError::Math(message) if message == "function `explode` produced non-finite result"),
    );
}

#[test]
fn public_api_exposes_entries_for_frontend_state_rendering() {
    let mut engine = Engine::new();

    engine.execute("subtotal = 120");
    engine.execute("tax = subtotal * 0.1");
    let total = engine.execute("subtotal + tax");
    let total_id_display = total.id.unwrap().to_string();
    let total_id = engine.entry(&total_id_display).unwrap().id;

    assert_eq!(
        engine.state_by_id(total_id),
        Some(&EntryState::Value(132.0))
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

    assert_eq!(first.id.unwrap().to_string(), "$1");
    assert_eq!(second.id.unwrap().to_string(), "$2");
    assert_eq!(third.id.unwrap().to_string(), "$3");
    assert_value(&engine, "$3", 10.0);

    engine.remove_entry("$2");
    assert_eval_error(
        &engine,
        "$3",
        |err| matches!(err, EvalError::UnknownReference(name) if name == "$2"),
    );

    let fourth = engine.execute("$1 + 10");
    assert_eq!(fourth.id.unwrap().to_string(), "$4");
    assert_value(&engine, "$4", 12.0);

    engine.set_entry("$2", "$1 + 4").unwrap();
    assert_value(&engine, "$3", 12.0);
}
