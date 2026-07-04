use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
        return;
    }

    match app.mode() {
        Mode::Normal => handle_normal_key(app, key.code),
        Mode::Insert | Mode::Edit => handle_input_key(app, key.code),
        Mode::Search => handle_search_key(app, key.code),
    }
}

fn handle_normal_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('j') | KeyCode::Down => app.move_next(),
        KeyCode::Char('k') | KeyCode::Up => app.move_previous(),
        KeyCode::Char('i') => app.start_insert(),
        KeyCode::Char('e') => app.start_edit(),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('p') => app.insert_selected_reference(),
        KeyCode::Char('d') => app.delete_selected(),
        KeyCode::Char('u') => app.undo(),
        KeyCode::Char('r') => app.redo(),
        KeyCode::Char('R') => app.recalculate_selected(),
        KeyCode::Char('A') => app.recalculate_all(),
        KeyCode::Char('c') => app.clear(),
        _ => {}
    }
}

fn handle_input_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.submit_input(),
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Up if app.completion_is_open() => app.move_completion_previous(),
        KeyCode::Down if app.completion_is_open() => app.move_completion_next(),
        KeyCode::Tab if app.completion_is_open() => app.accept_completion(),
        KeyCode::Backspace => app.backspace_input(),
        KeyCode::Char(ch) => app.push_input(ch),
        _ => {}
    }
}

fn handle_search_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter | KeyCode::Esc => app.close_search(),
        KeyCode::Backspace => app.backspace_search(),
        KeyCode::Tab => app.cycle_entry_state_filter(),
        KeyCode::Char(ch) => app.push_search(ch),
        _ => {}
    }
}
