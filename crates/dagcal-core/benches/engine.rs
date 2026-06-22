use criterion::{Criterion, criterion_group, criterion_main};
use dagcal_core::{Engine, EngineSnapshot, EntryState};
use std::hint::black_box;

fn assert_value(engine: &Engine, target: &str, expected: f64) {
    match engine.state(target) {
        Some(EntryState::Value(actual)) => {
            assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
        }
        other => panic!("expected value for {target}, got {other:?}"),
    }
}

fn populate_linear_chain(engine: &mut Engine, count: usize) {
    assert!(count > 0);
    engine.set_entry("root", "1").unwrap();

    for index in 1..count {
        let name = format!("node_{index}");
        let previous = if index == 1 {
            "root".to_string()
        } else {
            format!("node_{}", index - 1)
        };
        engine.set_entry(name, format!("{previous} + 1")).unwrap();
    }
}

fn populate_branching_graph(engine: &mut Engine, branch_count: usize) {
    assert!(branch_count > 0);
    engine.set_entry("root", "1").unwrap();

    for index in 0..branch_count {
        engine
            .set_entry(format!("branch_{index}"), format!("root + {index}"))
            .unwrap();
    }

    let source = (0..branch_count)
        .map(|index| format!("branch_{index}"))
        .collect::<Vec<_>>()
        .join(" + ");
    engine.set_entry("total", source).unwrap();
}

fn build_snapshot(entry_count: usize) -> EngineSnapshot {
    let mut engine = Engine::new();
    populate_linear_chain(&mut engine, entry_count);
    engine.snapshot()
}

fn bench_eval_once(c: &mut Criterion) {
    let engine = Engine::new();

    c.bench_function("eval_once_simple_expression", |b| {
        b.iter(|| {
            let value = engine
                .eval_once(black_box("sin(pi / 2) + cos(0) + 10 * 4"))
                .unwrap();
            black_box(value);
        });
    });
}

fn bench_execute_definition_chain(c: &mut Criterion) {
    c.bench_function("execute_definition_chain_100", |b| {
        b.iter(|| {
            let mut engine = Engine::new();
            populate_linear_chain(&mut engine, black_box(100));
            assert_value(&engine, "node_99", 100.0);
            black_box(engine);
        });
    });
}

fn bench_recompute_linear_dependents(c: &mut Criterion) {
    c.bench_function("recompute_linear_dependents_100", |b| {
        b.iter_batched(
            || {
                let mut engine = Engine::new();
                populate_linear_chain(&mut engine, 100);
                engine
            },
            |mut engine| {
                engine.set_entry("root", black_box("2")).unwrap();
                assert_value(&engine, "node_99", 101.0);
                black_box(engine);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_recompute_branching_graph(c: &mut Criterion) {
    c.bench_function("recompute_branching_graph_100", |b| {
        b.iter_batched(
            || {
                let mut engine = Engine::new();
                populate_branching_graph(&mut engine, 100);
                engine
            },
            |mut engine| {
                engine.set_entry("root", black_box("2")).unwrap();
                assert_value(&engine, "total", 5150.0);
                black_box(engine);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_snapshot_restore(c: &mut Criterion) {
    let snapshot = build_snapshot(black_box(100));

    c.bench_function("snapshot_restore_100", |b| {
        b.iter(|| {
            let mut engine = Engine::new();
            engine
                .restore_snapshot(black_box(snapshot.clone()))
                .unwrap();
            assert_value(&engine, "node_99", 100.0);
            black_box(engine);
        });
    });
}

criterion_group!(
    benches,
    bench_eval_once,
    bench_execute_definition_chain,
    bench_recompute_linear_dependents,
    bench_recompute_branching_graph,
    bench_snapshot_restore
);
criterion_main!(benches);
