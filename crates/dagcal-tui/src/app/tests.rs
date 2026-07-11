use super::*;
use dagcal_app::{AppAction, EntryState, EntryStateFilter, ExpressionId, Number};

fn add_entry(app: &mut App, source: &str) {
    app.session
        .dispatch(AppAction::InputEdited(source.to_string()));
    app.session.dispatch(AppAction::SubmitInput);
}

#[test]
fn insert_adds_entries_and_selects_the_latest_result() {
    let mut app = App::new();

    app.start_insert();
    for ch in "1 + 2".chars() {
        app.push_input(ch);
    }
    app.submit_input();

    app.start_insert();
    for ch in "$1 * 4".chars() {
        app.push_input(ch);
    }
    app.submit_input();

    let entries = app.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id.to_string(), "$1");
    assert_eq!(entries[0].state, EntryState::Value(Number::from(3)));
    assert_eq!(entries[1].id.to_string(), "$2");
    assert_eq!(entries[1].state, EntryState::Value(Number::from(12)));
    assert_eq!(app.selected(), 1);
}

#[test]
fn edit_updates_selected_source_and_recomputes_dependents() {
    let mut app = App::new();
    add_entry(&mut app, "subtotal = 100");
    add_entry(&mut app, "subtotal * 2");
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    app.start_edit();
    assert_eq!(app.input(), "subtotal = 100");
    app.session.dispatch(AppAction::InputChanged(String::new()));
    for ch in "subtotal = 120".chars() {
        app.push_input(ch);
    }
    app.submit_input();

    let entries = app.entries();
    assert_eq!(entries[0].state, EntryState::Value(Number::from(120)));
    assert_eq!(entries[1].state, EntryState::Value(Number::from(240)));
}

#[test]
fn edit_keeps_invalid_source_as_error_entry() {
    let mut app = App::new();
    add_entry(&mut app, "10");

    app.start_edit();
    app.session.dispatch(AppAction::InputChanged(String::new()));
    for ch in "1 +".chars() {
        app.push_input(ch);
    }
    app.submit_input();

    let entries = app.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "1 +");
    assert!(matches!(entries[0].state, EntryState::Error(_)));
}

#[test]
fn insert_keeps_invalid_source_as_error_entry() {
    let mut app = App::new();

    app.start_insert();
    for ch in "1 +".chars() {
        app.push_input(ch);
    }
    app.submit_input();

    let entries = app.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id.to_string(), "$1");
    assert_eq!(entries[0].source, "1 +");
    assert!(matches!(entries[0].state, EntryState::Error(_)));
    assert_eq!(app.selected(), 0);
}

#[test]
fn delete_preserves_later_ids() {
    let mut app = App::new();
    add_entry(&mut app, "10");
    add_entry(&mut app, "20");
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));
    app.delete_selected();

    let entries = app.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id.to_string(), "$2");
}

#[test]
fn undo_and_redo_refresh_cached_entries() {
    let mut app = App::new();

    app.start_insert();
    for ch in "10".chars() {
        app.push_input(ch);
    }
    app.submit_input();

    app.undo();
    assert!(app.entries().is_empty());
    assert_eq!(app.status(), "undone");

    app.redo();
    let entries = app.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, ExpressionId::new(1));
    assert_eq!(entries[0].state, EntryState::Value(Number::from(10)));
    assert_eq!(app.status(), "redone");
}

#[test]
fn empty_list_actions_do_not_panic() {
    let mut app = App::new();

    app.move_next();
    app.move_previous();
    app.start_edit();
    app.delete_selected();
    app.insert_selected_reference();
    app.recalculate_selected();

    assert!(app.entries().is_empty());
}

#[test]
fn search_filters_visible_entries_and_selection() {
    let mut app = App::new();
    add_entry(&mut app, "subtotal = 10");
    add_entry(&mut app, "subtotal / 0");

    app.open_search();
    for ch in "subtotal".chars() {
        app.push_search(ch);
    }
    app.cycle_entry_state_filter();
    app.cycle_entry_state_filter();

    let visible = app.visible_entries();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, ExpressionId::new(2));
    assert_eq!(app.entry_state_filter(), EntryStateFilter::Errors);
}

#[test]
fn completion_opens_moves_and_accepts_named_entry() {
    let mut app = App::new();
    add_entry(&mut app, "subtotal = 10");

    app.start_insert();
    for ch in "sub".chars() {
        app.push_input(ch);
    }

    assert!(app.completion_is_open());
    assert!(
        app.completion_candidates()
            .iter()
            .any(|candidate| candidate.label == "subtotal")
    );

    app.accept_completion();

    assert_eq!(app.input(), "subtotal");
    assert_eq!(app.status(), "Inserted subtotal");
}

#[test]
fn submit_accepts_completion_before_saving_entry() {
    let mut app = App::new();
    add_entry(&mut app, "subtotal = 10");

    app.start_insert();
    for ch in "sub".chars() {
        app.push_input(ch);
    }
    app.submit_input();

    assert_eq!(app.mode(), Mode::Insert);
    assert_eq!(app.input(), "subtotal");
    assert_eq!(app.entries().len(), 1);
}

#[test]
fn selected_reference_insertion_enters_insert_mode() {
    let mut app = App::new();
    add_entry(&mut app, "subtotal = 10");

    app.insert_selected_reference();

    assert_eq!(app.mode(), Mode::Insert);
    assert_eq!(app.input(), "subtotal ");
    assert_eq!(app.status(), "Inserted subtotal");
}

#[test]
fn recalculate_selected_and_all_update_status() {
    let mut app = App::new();
    add_entry(&mut app, "10");

    app.recalculate_selected();
    assert_eq!(app.status(), "Recalculated $1");

    app.recalculate_all();
    assert_eq!(app.status(), "Recalculated all entries");
}

#[test]
fn preview_and_details_report_values_and_errors() {
    let mut app = App::new();

    app.start_insert();
    for ch in "1 + 2".chars() {
        app.push_input(ch);
    }
    assert_eq!(app.preview_summary(), "Preview: 3");
    app.submit_input();
    assert!(app.selected_detail_text().contains("Result: 3"));

    app.start_insert();
    for ch in "1 / 0".chars() {
        app.push_input(ch);
    }
    app.submit_input();
    assert!(app.selected_detail_text().contains("Error detail:"));
}

#[test]
fn search_escape_clears_query_and_filter() {
    let mut app = App::new();

    app.open_search();
    app.push_search('x');
    app.cycle_entry_state_filter();
    app.close_search();

    assert_eq!(app.mode(), Mode::Normal);
    assert_eq!(app.search_query(), "");
    assert_eq!(app.entry_state_filter(), EntryStateFilter::All);
}
