#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = dagcal_gui::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
