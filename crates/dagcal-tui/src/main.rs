fn main() {
    if let Err(err) = dagcal_tui::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
