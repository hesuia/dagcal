# dagcal

`dagcal` is a dependency-aware calculator written in Rust. It keeps every saved
expression as a stable `$n` entry, lets later expressions refer to earlier
results or named definitions, and recomputes dependent entries when a source
entry changes.

The workspace currently contains:

- `crates/dagcal-core`: parser, evaluator, dependency graph, persistence, and public engine API.
- `crates/dagcal-repl`: line-oriented REPL frontend.
- `crates/dagcal-tui`: terminal UI frontend built with `ratatui`.
- `crates/dagcal-gui`: desktop GUI frontend built with `iced`.

## Features

- Arithmetic expressions with `+`, `-`, `*`, `/`, `%`, `^`, unary signs, and parentheses.
- Stable result references such as `$1`, `$2`, and `$10`.
- Named definitions such as `subtotal = 1200`.
- Automatic recomputation of dependent expressions after edits or removals.
- Exact rational arithmetic where possible, with float boundaries for constants and math functions.
- Decimal, binary, octal, and hexadecimal numeric literals, including fractional forms.
- Built-in math functions such as `sqrt`, `sin`, `pow`, `sum`, `avg`, `min`, and `max`.
- Structured parse, evaluation, cycle, and persistence errors from the core API.

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

TUI keys:

- `i`: insert a new expression.
- `e`: edit the selected entry.
- `j`/`Down` and `k`/`Up`: move the selection.
- `/`: search entries; type a query, `Tab` cycles All/Values/Errors, and `Esc` or `Enter` closes search.
- `p`: insert the selected entry reference into the input.
- `Tab`: accept the selected completion while inserting or editing.
- `R`: recalculate the selected entry.
- `A`: recalculate all entries.
- `d`: delete the selected entry.
- `u`/`r`: undo and redo.
- `c`: clear entries.
- `q`: quit.

Run the GUI:

```sh
cargo run -p dagcal-gui
```

## REPL Commands

- `:help`: show command help.
- `:list`: show saved expressions and current states.
- `:set <id> <expr>`: edit an entry by name or `$n` result ID.
- `:remove <id>`: remove an entry by name or `$n` result ID.
- `:clear`: clear all expressions.
- `:quit` or `:exit`: exit the REPL.

## Expression Syntax

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

Names must start with an ASCII letter or `_`, followed by ASCII letters,
digits, or `_`.

## Core API Example

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

## Development

Run the full test suite:

```sh
cargo test --workspace
```

Run only the public API integration tests:

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

## Architecture Notes

The core library models expressions as a dependency graph. `ExpressionId` is the
canonical internal identifier, displayed to users as stable `$n` references.
Public convenience APIs may accept names or `$n` strings, but they resolve those
targets to `ExpressionId` before delegating to ID-specific engine behavior.

When an entry is edited, removed, restored, or affected by a runtime symbol
change, the engine recomputes only the affected dependency subgraph and keeps
unaffected entries stable.
