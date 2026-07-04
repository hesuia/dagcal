mod app;
mod keyboard;
mod terminal;
mod views;

use std::io;

pub fn run() -> io::Result<()> {
    terminal::run()
}
