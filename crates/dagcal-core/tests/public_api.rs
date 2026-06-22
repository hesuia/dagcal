use dagcal_core::{
    DagcalError, Engine, EngineSnapshot, EntryState, EvalError, ParseErrorKind, PersistedEntry,
    PersistenceError,
};

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
fn public_api_executes_standalone_number_literals() {
    let mut engine = Engine::new();

    let integer = engine.execute("10");
    let decimal = engine.execute("4.2");

    assert_eq!(integer.id.unwrap().to_string(), "$1");
    assert_eq!(integer.state, EntryState::Value(10.0));
    assert_eq!(decimal.id.unwrap().to_string(), "$2");
    assert_eq!(decimal.state, EntryState::Value(4.2));
    assert_value(&engine, "$1", 10.0);
    assert_value(&engine, "$2", 4.2);
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
    match parse_error.state {
        EntryState::Error(DagcalError::Parse(err)) => {
            assert_eq!(err.kind, ParseErrorKind::Syntax);
            assert!(err.span.is_some());
        }
        other => panic!("expected parse error, got {other:?}"),
    }
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

    assert_value(&restored, "$1", 100.0);
    assert_value(&restored, "$2", 0.1);
    assert_value(&restored, "$3", 10.0);
    assert_value(&restored, "$4", 110.0);
    assert_value(&restored, "subtotal", 100.0);

    restored.set_entry("subtotal", "200").unwrap();
    assert_value(&restored, "$3", 20.0);
    assert_value(&restored, "$4", 220.0);
}

#[test]
fn public_api_restore_preserves_removed_id_gaps_and_next_append_id() {
    let mut engine = Engine::new();

    engine.execute("1");
    engine.execute("2");
    engine.execute("3");
    engine.remove_entry("$2");

    let mut restored = Engine::from_snapshot(engine.snapshot()).unwrap();
    let next = restored.execute("$1 + $3");

    assert!(restored.entry("$2").is_none());
    assert_eq!(next.id.unwrap().to_string(), "$4");
    assert_value(&restored, "$4", 4.0);
}

#[test]
fn public_api_restore_rebuilds_cycle_diagnostics() {
    let mut engine = Engine::new();

    engine.execute("a = 1");
    engine.execute("b = 2");
    engine.set_entry("a", "b + 1").unwrap();
    assert!(engine.set_entry("b", "a + 1").is_err());

    let restored = Engine::from_snapshot(engine.snapshot()).unwrap();
    let diagnostics = restored.cycle_diagnostics();

    assert!(matches!(
        restored.state("a"),
        Some(EntryState::Error(DagcalError::Eval(
            EvalError::CycleDetected(_)
        )))
    ));
    assert_eq!(
        diagnostics.cycle_nodes,
        ["$1".to_string(), "$2".to_string()].into_iter().collect()
    );
}

#[test]
fn public_api_restore_preserves_stored_parse_error_entries() {
    let mut engine = Engine::new();

    assert!(engine.set_entry("broken", "1 +").is_err());
    let restored = Engine::from_snapshot(engine.snapshot()).unwrap();

    match restored.state("broken") {
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
