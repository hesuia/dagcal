mod app;
mod tui;

fn main() {
    if let Err(err) = tui::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
