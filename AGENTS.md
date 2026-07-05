# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 Cargo workspace. Source lives under `crates/`:

- `crates/dagcal-core`: parser, evaluator, dependency graph, persistence, public engine API, plus `src/syntax.pest`.
- `crates/dagcal-app`: shared UI-agnostic application state used by frontends.
- `crates/dagcal-repl`: line-oriented command-line frontend.
- `crates/dagcal-tui`: terminal UI using `ratatui` and `crossterm`.
- `crates/dagcal-gui`: desktop GUI using `iced`.

Tests are colocated as Rust unit tests, with integration tests in paths such as `crates/dagcal-core/tests/public_api.rs`. Benchmarks live in `crates/dagcal-core/benches/`.

## Build, Test, and Development Commands

- `cargo test --workspace`: run the full workspace test suite.
- `cargo test public_api --workspace`: run the public API integration tests.
- `cargo run -p dagcal-repl`: start the REPL frontend.
- `cargo run -p dagcal-tui`: start the terminal UI.
- `cargo run -p dagcal-gui`: start the desktop GUI.
- `cargo fmt -- --check`: verify Rust formatting.
- `cargo fmt`: apply standard Rust formatting.
- `cargo deny check`: validate dependency license and advisory policy from `deny.toml`.
- `cargo bench -p dagcal-core`: run core engine benchmarks.

## Coding Style & Naming Conventions

Use standard `rustfmt` formatting and Rust 2024 idioms. Prefer focused modules that match existing boundaries: engine logic in `dagcal-core`, reusable frontend state in `dagcal-app`, and UI-specific behavior in the relevant frontend crate. Use `snake_case` for functions, modules, and variables; `PascalCase` for types and traits.
Avoid unnecessary use of `clone` or numorous `copy`.
Keep functions small and focused, and document public APIs with doc comments.

## Testing Guidelines

Add unit tests near the behavior they cover and integration tests for public API guarantees. When changing parser, evaluation, dependency, persistence, or recomputation behavior, include tests in `dagcal-core`. For shared application actions or frontend state transitions, cover `dagcal-app` or the relevant frontend test module. Run `cargo test --workspace` before submitting changes.

## Commit & Pull Request Guidelines

Recent commits use short, imperative subjects such as `Split TUI into focused modules` and `Add shared dagcal-app crate`. Keep commit messages concise and scoped to one change. Pull requests should explain the motivation, summarize behavioral changes, list test commands run, and include screenshots or terminal notes for visible TUI/GUI changes. Link issues when applicable and call out compatibility or persistence-format impacts explicitly.

## Security & Configuration Tips

Do not commit generated build output from `target/` or local runtime data. Keep dependency changes intentional, and update `deny.toml` only when the license or advisory policy itself needs to change.
