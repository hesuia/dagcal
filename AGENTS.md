# Repository Guidelines

## Project Structure & Module Organization
`dagcal` is a Rust Cargo workspace. The shared calculator library lives in `crates/dagcal-core/src`, with focused modules such as `parser.rs`, `eval.rs`, `dependency_graph.rs`, `persistence.rs`, and `engine/` for runtime state, dependency resolution, and recomputation. Frontends are split into `crates/dagcal-repl`, `crates/dagcal-tui`, and `crates/dagcal-gui`. Public API integration tests live in `crates/dagcal-core/tests/public_api.rs`, and benchmarks live in `crates/dagcal-core/benches`. Treat `target/` as generated output.

## Build, Test, and Development Commands
- `cargo run -p dagcal-repl`: start the REPL locally.
- `cargo run -p dagcal-tui`: start the terminal UI.
- `cargo run -p dagcal-gui`: start the GUI frontend.
- `cargo test --workspace`: run the full workspace test suite.
- `cargo test public_api --workspace`: focus on exported engine behavior.
- `cargo bench -p dagcal-core`: run core benchmarks.
- `cargo fmt -- --check`: verify formatting before review.
- `cargo fmt`: apply standard Rust formatting.

## Coding Style & Naming Conventions
Follow standard Rust formatting with 4-space indentation and keep all code `rustfmt`-clean. Use `snake_case` for functions, modules, and test names, and `CamelCase` for types. Prefer domain-specific names such as `ExpressionId`, `EntryState`, and `restore_snapshot`. Keep modules small and responsibility-focused. Avoid unnecessary `clone` and `copy`; pass references where ownership is not required.

## Testing Guidelines
Add unit tests next to implementation for internal behavior. Add or extend integration tests in `crates/dagcal-core/tests` when changing the public engine API. Use descriptive test names such as `public_api_reports_parse_and_cycle_errors_without_losing_valid_entries`. New engine behavior should cover success paths and failure paths, especially parse errors, cycle detection, persistence, and recomputation.

## Commit & Pull Request Guidelines
Recent commit subjects use short imperative phrases such as `Add README` and `Split frontends into crates`. Keep commits focused and use the same style. Pull requests should describe user-visible behavior, mention parser or engine invariants affected, and list verification commands run. Include REPL examples when expression syntax or runtime behavior changes.

## Architecture Notes
The core library models expressions as a dependency graph with stable `$n` result IDs and optional user-defined names. Preserve ID stability when editing persistence, entry removal, or recomputation. Internal engine state, stores, resolvers, dependency graph code, and recomputation logic should use `ExpressionId` as the canonical identifier. Convenience APIs may accept names or `$n` strings, but should resolve to `ExpressionId` immediately and delegate to ID-specific methods.
