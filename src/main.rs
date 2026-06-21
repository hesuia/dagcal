use dagcal_core::{Engine, EntryState, Execution};
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
                    Some((id, source)) => {
                        self.set_and_print(id, source);
                    }
                    None => println!("usage: :set <id> <expr>"),
                }
                true
            }
            ":remove" => {
                let id = input[":remove".len()..].trim();
                if id.is_empty() {
                    println!("usage: :remove <id>");
                } else if self.engine.remove_entry(id).is_some() {
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
        print_execution(&self.engine.execute(input));
    }

    fn set_and_print(&mut self, id: &str, source: &str) {
        match self.engine.set_entry(id, source) {
            Ok(execution) => print_state(id, &execution.state),
            Err(err) => println!("{id} = error: {err}"),
        }
    }

    fn print_entries(&self) {
        let entries = self.engine.entries();

        if entries.is_empty() {
            println!("no entries");
            return;
        }

        for entry in entries {
            print!("{} = {} => ", entry.label, entry.source);
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

fn print_state(id: &str, state: &EntryState) {
    print!("{id} = ");
    print_state_value(state);
}

fn print_execution(execution: &Execution) {
    match &execution.label {
        Some(label) => print_state(&label.to_string(), &execution.state),
        None => print_state_value(&execution.state),
    }
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
}
