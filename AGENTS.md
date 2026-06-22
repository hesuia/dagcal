# Repository Guidelines

## Project Structure & Module Organization
`dagcal` is a small Rust workspace. The CLI entrypoint lives in `src/main.rs` and provides the REPL. Shared calculator logic lives in `crates/dagcal-core/src`, with focused modules such as `parser.rs`, `eval.rs`, `dependency_graph.rs`, and `engine/` for runtime state and recomputation. Public API coverage sits in `crates/dagcal-core/tests/public_api.rs`. Treat `target/` as generated build output and do not commit changes from it.

## Build, Test, and Development Commands
Use `cargo run` to start the REPL locally. Use `cargo test --workspace` to run the full workspace test suite, including the `dagcal-core` integration tests. Use `cargo fmt -- --check` to verify formatting before review, and `cargo fmt` to apply it. Use `cargo test public_api --workspace` when iterating on the exported engine behavior.

## Coding Style & Naming Conventions
Follow standard Rust formatting with 4-space indentation and keep code `rustfmt`-clean. Prefer small modules with explicit responsibilities rather than large mixed files. Use `snake_case` for functions, modules, and test names; `CamelCase` for types; and clear domain names such as `ExpressionId`, `EntryState`, or `restore_snapshot`. Keep error paths structured with existing error enums instead of ad hoc strings.

## Testing Guidelines
Add unit tests next to implementation when validating internal behavior, and add or extend integration tests in `crates/dagcal-core/tests` when changing the public engine API. Match the current descriptive style for test names, for example `public_api_reports_parse_and_cycle_errors_without_losing_valid_entries`. New engine behavior should include both success-path and failure-path coverage.

## Perormance & Coding
Avoid unnecessary use of `clone` and numerous `copy`. Use references where possible.

## Commit & Pull Request Guidelines
Recent history uses short imperative commit subjects such as `Add persistence snapshot support` and `Add API documentation`. Keep commits focused and written in that style. Pull requests should explain the user-visible behavior change, note any parser or engine invariants affected, and list the verification commands you ran. Include REPL examples when a change affects expression syntax or runtime behavior.

## Architecture Notes
The core library models expressions as a dependency graph with stable `$n` result IDs and optional user-defined names. Preserve that stability when editing persistence, entry removal, or recomputation logic, because downstream behavior and tests depend on it.
