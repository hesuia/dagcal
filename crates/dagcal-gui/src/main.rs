fn main() {
    if let Err(err) = dagcal_gui::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
