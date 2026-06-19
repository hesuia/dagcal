# Repository Guidelines

## Project Structure & Module Organization

`dagcal` is a Rust workspace with a small CLI in `src/main.rs` and the main engine crate in `crates/dagcal-core/`. Most contributor work belongs in `crates/dagcal-core/src/`.

Key core modules:
- `lib.rs`: public API exports
- `engine.rs`: entry lifecycle, dependency tracking, recomputation
- `parser.rs` and `syntax.pest`: grammar and parsing
- `eval.rs`, `function.rs`, `error.rs`: evaluation, built-ins, error types
- `tests/public_api.rs`: integration coverage for the exposed API

Keep new code close to the module that owns the behavior instead of adding broad helpers at the workspace root.

## Build, Test, and Development Commands

Run commands from the repository root:

- `cargo check --workspace`: fast compile check across the CLI and core crate
- `cargo test --workspace`: run unit and integration tests
- `cargo test -p dagcal-core`: focus on the core engine during parser/evaluator work
- `cargo run`: start the REPL locally
- `cargo fmt --all`: apply standard Rust formatting

## Coding Style & Naming Conventions

Follow idiomatic Rust: 4-space indentation, `snake_case` for functions/modules/tests, `CamelCase` for types, and concise enums for domain states. Prefer explicit error variants in `error.rs` over string-based failures. When changing parsing behavior, update `syntax.pest`, parser logic, and relevant AST handling together.

Formatting is done with `cargo fmt`. Keep code warning-free under `cargo check`; if you use Clippy locally, fix useful lints before opening a PR.

## Testing Guidelines

Add unit tests beside the code they verify and integration tests in `crates/dagcal-core/tests/` for public API behavior. Name tests by observable behavior, for example `removing_entry_recomputes_dependents_as_errors`. Cover parser precedence, dependency propagation, cycle handling, and recovery after edits or removals.

## Commit & Pull Request Guidelines

Recent history uses short imperative subjects such as `Add public API integration tests` and `Optimize dependency graph analysis`. Follow that pattern and keep each commit scoped to one logical change.

PRs should include a brief behavior summary, the commands you ran (usually `cargo test --workspace`), and notes on grammar or API changes when applicable.
