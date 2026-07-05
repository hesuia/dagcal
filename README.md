# dagcal

`dagcal` is a dependency-aware calculator written in Rust. Every saved
expression gets a stable result ID such as `$1`, later expressions can refer to
earlier results or named definitions, and the engine recomputes dependent
entries automatically when a source entry changes.

The repository is a Cargo workspace with a reusable core engine and multiple
frontends:

- `dagcal-core`: parser, evaluator, dependency graph, persistence, recomputation, and the public engine API.
- `dagcal-app`: UI-agnostic session state, filtering, completion, draft editing, and formatting shared by frontends.
- `dagcal-repl`: line-oriented command-line frontend.
- `dagcal-tui`: terminal UI built with `ratatui` and `crossterm`.
- `dagcal-gui`: desktop GUI built with `iced`.

## Highlights

- Stable result references like `$1`, `$2`, and `$10`.
- Named definitions like `subtotal = 1200`.
- Automatic recomputation of dependent entries after edits and removals.
- Undo and redo support.
- Exact rational arithmetic where possible, with float boundaries for constants and math functions.
- Decimal, binary, octal, and hexadecimal literals, including fractional forms.
- Built-in math functions and extensible runtime constants/functions from the core API.
- Session snapshot persistence in the engine, with save/load support in the GUI.
- Structured parse, evaluation, cycle, and persistence errors.

## Quick Start

Run the REPL:

```sh
cargo run -p dagcal-repl
```

Example session:

```text
dagcal repl. Type :help for commands, :quit to exit.
> subtotal = 1200
$1 = 1200
> tax = subtotal * 0.1
$2 = 120
> subtotal + tax
$3 = 1320
> :set subtotal 1500
subtotal = 1500
> :list
$1 = 1500 => 1500
$2 = subtotal * 0.1 => 150
$3 = subtotal + tax => 1650
```

Run the TUI:

```sh
cargo run -p dagcal-tui
```

Run the GUI:

```sh
cargo run -p dagcal-gui
```

## Frontends

### REPL

The REPL is the simplest way to explore engine behavior.

Commands:

- `:help`: show command help.
- `:list`: show saved expressions and their current states.
- `:set <id> <expr>`: edit an entry by name or `$n`.
- `:remove <id>`: remove an entry by name or `$n`.
- `:clear`: clear all expressions.
- `:undo`: undo the last change.
- `:redo`: redo the last undone change.
- `:quit` or `:exit`: exit the REPL.

### TUI

The terminal UI is focused on fast keyboard-driven editing.

- `i`: insert a new expression.
- `e`: edit the selected entry.
- `j` / `Down`, `k` / `Up`: move selection.
- `/`: open entry search.
- `Tab`: accept the selected completion while editing.
- `p`: insert the selected entry reference into the input.
- `R`: recalculate the selected entry.
- `A`: recalculate all entries.
- `d`: delete the selected entry.
- `u` / `r`: undo and redo.
- `c`: clear all entries.
- `q`: quit.

While search is open, type to filter entries and use `Tab` to cycle the
All/Values/Errors filter. `Esc` or `Enter` closes search.

### GUI

The GUI adds desktop-oriented session management and menus on top of the shared
application state:

- Save, Save As, and Load for JSON session snapshots.
- Dirty-state tracking and discard confirmation before destructive actions.
- Entry search with All/Values/Errors filtering.
- Inline completions for entries, `$n` references, constants, and functions.
- Per-entry actions such as edit, delete, recalculate, and reference insertion.
- Keyboard shortcuts including `Ctrl+F`, `Ctrl+Z`, `Ctrl+Y`, `Esc`, and `Delete`.

## Expression Syntax

Examples:

```text
1 + 2 * 3
(1 + 2) * 3
$1 + 10
subtotal = 1200
tax = subtotal * 0.1
sum(1, 2, 3, tax)
0b1001.1101
0o10.4
0xA.F
```

Supported syntax includes:

- Arithmetic operators: `+`, `-`, `*`, `/`, `%`, `^`
- Unary `+` and `-`
- Parentheses
- Stable `$n` result references
- Named definitions and named references
- Function calls
- Decimal, binary, octal, and hexadecimal numeric literals

Names must start with an ASCII letter or `_`, followed by ASCII letters,
digits, or `_`.

## Core API

`dagcal-core` keeps the public API intentionally small. `Engine` owns the full
session state, while `EntryState`, `EntryView`, and snapshot types expose the
current calculator state to applications.

```rust
use dagcal_core::{Engine, EntryState, Number};

let mut engine = Engine::new();

let subtotal = engine.execute("subtotal = 100");
let tax = engine.execute("tax = subtotal * 0.1");
let total = engine.execute("subtotal + tax");

assert_eq!(subtotal.state, EntryState::Value(Number::from(100.0)));
assert_eq!(tax.state, EntryState::Value(Number::from(10.0)));
assert_eq!(total.state, EntryState::Value(Number::from(110.0)));

engine.set_entry("subtotal", "200").unwrap();

assert_eq!(
    engine.state(total.id),
    Some(&EntryState::Value(Number::from(220.0)))
);
```

The engine also supports:

- Editing and removing entries by name, `$n`, or `ExpressionId`
- Manual recalculation of one entry or all entries
- Undo / redo history
- Snapshot export and restore
- Runtime registration of constants and functions

## Persistence

The engine can serialize a session as an `EngineSnapshot`. The GUI uses this to
save and load JSON files, and other frontends can use the same snapshot API if
they need persistence.

Snapshots preserve source text, stable IDs, and names. Values are recomputed
when a snapshot is restored.

## Development

Run the full test suite:

```sh
cargo test --workspace
```

Run the public API integration tests:

```sh
cargo test public_api --workspace
```

Check formatting:

```sh
cargo fmt -- --check
```

Apply formatting:

```sh
cargo fmt
```

Check dependency and advisory policy:

```sh
cargo deny check
```

Run core benchmarks:

```sh
cargo bench -p dagcal-core
```

Build optimized distribution binaries:

```sh
cargo build -p dagcal-gui --profile dist-gui
cargo build -p dagcal-tui --profile dist-tui
```

## Architecture Notes

Internally, the engine models saved expressions as a dependency graph.
`ExpressionId` is the canonical internal identifier and is shown to users as a
stable `$n` reference.

When an entry is edited, removed, restored from history, or affected by a
runtime constant/function change, the engine recomputes only the affected part
of the graph and leaves unrelated entries untouched.
