use dagcal_core::{Engine, EntryState};
use std::io::{self, Write};

fn main() {
    let mut repl = Repl::new();
    if let Err(err) = repl.run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

struct Repl {
    engine: Engine,
}

impl Repl {
    fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    fn run(&mut self) -> io::Result<()> {
        println!("dagcal repl. Type :help for commands, :quit to exit.");

        let stdin = io::stdin();
        let mut line = String::new();

        loop {
            print!("> ");
            io::stdout().flush()?;

            line.clear();
            let bytes = stdin.read_line(&mut line)?;
            if bytes == 0 {
                println!();
                break;
            }

            let input = line.trim();
            if input.is_empty() {
                continue;
            }

            if !self.handle_input(input) {
                break;
            }
        }

        Ok(())
    }

    fn handle_input(&mut self, input: &str) -> bool {
        if input.starts_with(':') {
            self.handle_command(input)
        } else {
            self.handle_expression(input);
            true
        }
    }

    fn handle_command(&mut self, input: &str) -> bool {
        let command = input.split_whitespace().next().unwrap_or_default();

        match command {
            ":help" => {
                print_help();
                true
            }
            ":quit" | ":exit" => false,
            ":list" => {
                self.print_entries();
                true
            }
            ":clear" => {
                self.engine = Engine::new();
                println!("cleared");
                true
            }
            ":set" => {
                match split_command_args(input[":set".len()..].trim()) {
                    Some((id, source)) if is_valid_entry_id(id) => {
                        self.set_and_print(id, source);
                    }
                    Some((id, _)) => println!("invalid id: {id}"),
                    None => println!("usage: :set <id> <expr>"),
                }
                true
            }
            ":remove" => {
                let id = input[":remove".len()..].trim();
                if id.is_empty() {
                    println!("usage: :remove <id>");
                } else if self.engine.remove_expr(id).is_some() {
                    println!("removed {id}");
                } else {
                    println!("not found: {id}");
                }
                true
            }
            _ => {
                println!("unknown command: {command}");
                println!("type :help for available commands");
                true
            }
        }
    }

    fn handle_expression(&mut self, input: &str) {
        match parse_assignment(input) {
            Assignment::Named { id, source } => self.set_and_print(id, source),
            Assignment::InvalidId(id) => println!("invalid id: {id}"),
            Assignment::Malformed => println!("invalid assignment; use name = expr"),
            Assignment::Expression(source) => {
                let (id, state) = self.engine.append_expr(source);
                print_state(&id, &state);
            }
        }
    }

    fn set_and_print(&mut self, id: &str, source: &str) {
        let _ = self.engine.set_expr(id, source);
        match self.engine.get(id) {
            Some(state) => print_state(id, state),
            None => println!("error: entry was not saved"),
        }
    }

    fn print_entries(&self) {
        let mut entries = self.engine.entries().collect::<Vec<_>>();
        entries.sort_by(|(left, _), (right, _)| compare_entry_ids(left, right));

        if entries.is_empty() {
            println!("no entries");
            return;
        }

        for (id, entry) in entries {
            print!("{id} = {} => ", entry.source);
            print_state_value(&entry.state);
        }
    }
}

fn split_command_args(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    let (id, source) = input.split_once(char::is_whitespace)?;
    let source = source.trim();

    if id.is_empty() || source.is_empty() {
        None
    } else {
        Some((id, source))
    }
}

enum Assignment<'a> {
    Named { id: &'a str, source: &'a str },
    InvalidId(&'a str),
    Malformed,
    Expression(&'a str),
}

fn parse_assignment(input: &str) -> Assignment<'_> {
    let Some((left, right)) = input.split_once('=') else {
        return Assignment::Expression(input);
    };

    if right.contains('=') {
        return Assignment::Malformed;
    }

    let id = left.trim();
    let source = right.trim();
    if id.is_empty() || source.is_empty() {
        return Assignment::Malformed;
    }

    if is_valid_entry_id(id) && !id.starts_with('$') {
        Assignment::Named { id, source }
    } else {
        Assignment::InvalidId(id)
    }
}

fn is_valid_entry_id(id: &str) -> bool {
    is_valid_named_id(id) || is_valid_result_id(id)
}

fn is_valid_named_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_valid_result_id(id: &str) -> bool {
    let Some(digits) = id.strip_prefix('$') else {
        return false;
    };

    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
}

fn compare_entry_ids(left: &str, right: &str) -> std::cmp::Ordering {
    match (parse_result_index(left), parse_result_index(right)) {
        (Some(left_index), Some(right_index)) => left_index.cmp(&right_index),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.cmp(right),
    }
}

fn parse_result_index(id: &str) -> Option<usize> {
    id.strip_prefix('$')?.parse().ok()
}

fn print_state(id: &str, state: &EntryState) {
    print!("{id} = ");
    print_state_value(state);
}

fn print_state_value(state: &EntryState) {
    match state {
        EntryState::Value(value) => println!("{value}"),
        EntryState::Error(err) => println!("error: {err}"),
    }
}

fn print_help() {
    println!("Enter expressions to evaluate and save as $1, $2, ...");
    println!("Use name = expr to save a named expression.");
    println!();
    println!("Examples:");
    println!("  1 + 2 * 3");
    println!("  $1 + 10");
    println!("  subtotal = 1200");
    println!("  tax = subtotal * 0.1");
    println!();
    println!("Commands:");
    println!("  :help              Show this help");
    println!("  :list              Show saved expressions");
    println!("  :set <id> <expr>   Set or edit an expression");
    println!("  :remove <id>       Remove an expression");
    println!("  :clear             Clear all expressions");
    println!("  :quit, :exit       Exit");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_command_arguments_with_extra_spaces() {
        assert_eq!(
            split_command_args("  subtotal   100 + 20  "),
            Some(("subtotal", "100 + 20"))
        );
    }

    #[test]
    fn rejects_incomplete_set_command_arguments() {
        assert_eq!(split_command_args("subtotal"), None);
        assert_eq!(split_command_args("subtotal   "), None);
    }

    #[test]
    fn parses_named_assignments() {
        match parse_assignment("subtotal = 100 + 20") {
            Assignment::Named { id, source } => {
                assert_eq!(id, "subtotal");
                assert_eq!(source, "100 + 20");
            }
            _ => panic!("expected named assignment"),
        }
    }

    #[test]
    fn leaves_plain_expressions_unassigned() {
        match parse_assignment("$1 + 10") {
            Assignment::Expression(source) => assert_eq!(source, "$1 + 10"),
            _ => panic!("expected expression"),
        }
    }
}
