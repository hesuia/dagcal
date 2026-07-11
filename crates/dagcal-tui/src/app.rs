mod actions;
mod formatting;
mod selectors;

#[cfg(test)]
mod tests;

use dagcal_app::AppSession;

pub use formatting::{expression_source, filter_label, kind_label, state_summary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Edit,
    Search,
}

pub struct App {
    session: AppSession,
    mode: Mode,
    should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut session = AppSession::new();
        session.dispatch(dagcal_app::AppAction::SetStatus(
            "i: insert  e: edit  /: search  p: use  R: recalc  A: recalc all  q: quit".to_string(),
        ));
        Self {
            session,
            mode: Mode::Normal,
            should_quit: false,
        }
    }
}
