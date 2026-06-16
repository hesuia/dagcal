# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace for a calculator engine. The root package `dagcal` currently contains a minimal binary entrypoint in [src/main.rs](~/programs/dagcal/src/main.rs). Core logic lives in `crates/dagcal-core/`, which is the main crate contributors should extend.

Inside `crates/dagcal-core/src/`:
- `lib.rs` re-exports the public API
- `engine.rs` manages expressions, references, and recomputation
- `parser.rs` and `syntax.pest` define the grammar and AST parsing
- `eval.rs`, `function.rs`, and `error.rs` cover evaluation, built-ins, and error types

Keep new logic near its ownership boundary instead of adding cross-cutting helpers at the root.

## Build, Test, and Development Commands

- `cargo check --workspace`: fast compile check for the whole workspace
- `cargo test -p dagcal-core`: run the core engine test suite
- `cargo test --workspace`: run all tests
- `cargo fmt --all`: format Rust code before review

Run commands from the repository root.

## Coding Style & Naming Conventions

Use standard Rust style: 4-space indentation, `snake_case` for functions/modules, `CamelCase` for types, and small focused modules. Prefer explicit error types over stringly-typed failures. Keep parser changes aligned with `syntax.pest` and the AST in `parser.rs` and `ast.rs`.

Formatting is enforced with `cargo fmt`. No separate linter is configured yet, so keep code idiomatic and warning-free under `cargo check`.

- Avoid unnecessary clones and prefer references when possible. Use `Result<T, E>` for fallible operations and define custom error types in `error.rs` for clarity.
- For public API functions, document behavior, parameters, and return values with doc comments. For internal functions, focus on clear naming and modularity to convey intent.

## Testing Guidelines

Tests are currently inline unit tests within the relevant source files. Add tests next to the code they validate. Cover parser precedence, evaluation edge cases, dependency propagation, and recomputation after edits or removals.

Name tests by behavior, for example `removing_entry_recomputes_dependents_as_errors`.

## Commit & Pull Request Guidelines

The current history uses short imperative commit subjects, for example: `Add dagcal core calculator crate`. Follow that pattern. Keep commits scoped to one logical change when possible.

For pull requests, include:
- a brief summary of the behavioral change
- test evidence such as `cargo test -p dagcal-core`
- notes on grammar or public API changes when relevant
