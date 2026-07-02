use super::actions::SelectionDirection;
use super::*;
use dagcal_core::{
    CompletionKind, EngineSnapshot, EntryState, ExpressionId, Number, PersistedEntry,
};
use iced::keyboard::{self, Key, key};
use std::path::PathBuf;

#[test]
fn draft_inserts_result_reference_without_replacing_saved_source() {
    let mut draft = Draft::default();

    draft.set("1 +".to_string());
    draft.insert_token("$1");

    assert_eq!(draft.source(), "1 +$1 ");
}

#[test]
fn use_inserts_name_for_named_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("x = 10".to_string());
    app.submit_input();

    app.insert_reference(ExpressionId::new(1));

    assert_eq!(app.input.source(), "x ");
    assert_eq!(app.status, "Inserted x");
}

#[test]
fn use_inserts_result_id_for_unnamed_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    app.insert_reference(ExpressionId::new(1));

    assert_eq!(app.input.source(), "$1 ");
    assert_eq!(app.status, "Inserted $1");
}

#[test]
fn menu_constant_inserts_constant_name() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InsertConstant("pi".to_string()));

    assert_eq!(app.input.source(), "pi ");
    assert_eq!(app.status, "Inserted pi");
    assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
}

#[test]
fn menu_function_inserts_call_template() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InsertFunction("sin".to_string()));

    assert_eq!(app.input.source(), "sin() ");
    assert_eq!(app.status, "Inserted sin()");
    assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
}

#[test]
fn input_change_opens_completion_for_named_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("subtotal = 10".to_string());
    app.submit_input();

    let _ = app.update(Message::InputChanged("sub".to_string()));

    assert!(app.completion_is_open());
    assert!(app.completion_candidates().iter().any(|candidate| {
        candidate.label == "subtotal" && candidate.kind == CompletionKind::Entry
    }));
}

#[test]
fn submit_accepts_completion_before_saving_input() {
    let (mut app, _) = GuiApp::new();
    app.input.set("subtotal = 10".to_string());
    app.submit_input();

    let _ = app.update(Message::InputChanged("sub".to_string()));
    let _ = app.update(Message::Submit);

    assert_eq!(app.input.source(), "subtotal");
    assert_eq!(app.entries.len(), 2);
    assert_eq!(app.entries[0].name.as_deref(), Some("subtotal"));
    assert_eq!(app.entries[1].source, "");
    assert_eq!(app.status, "Inserted subtotal");
}

#[test]
fn completion_finds_result_references_and_moves_selection() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    let _ = app.update(Message::InputChanged("$".to_string()));
    assert_eq!(app.selected_completion_index(), Some(0));

    app.handle_keyboard_event(named_key_event(key::Named::ArrowDown));
    assert_eq!(app.selected_completion_index(), Some(1));

    app.handle_keyboard_event(named_key_event(key::Named::ArrowUp));
    assert_eq!(app.selected_completion_index(), Some(0));
}

#[test]
fn tab_accepts_selected_completion() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    let _ = app.update(Message::InputChanged("$".to_string()));
    let effect = app.handle_keyboard_event(named_key_event(key::Named::Tab));

    assert_eq!(app.input.source(), "$1");
    assert_eq!(app.status, "Inserted $1");
    assert!(!app.completion_is_open());
    assert_eq!(effect, super::effects::UiEffect::FocusInput);
}

#[test]
fn escape_closes_completion() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    let _ = app.update(Message::InputChanged("$".to_string()));
    app.handle_keyboard_event(named_key_event(key::Named::Escape));

    assert!(!app.completion_is_open());
    assert_eq!(app.input.source(), "$");
}

#[test]
fn dismiss_completion_message_closes_completion() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    let _ = app.update(Message::InputChanged("$".to_string()));
    let _ = app.update(Message::DismissCompletion);

    assert!(!app.completion_is_open());
    assert_eq!(app.input.source(), "$");
}

#[test]
fn status_bar_history_text_reflects_undo_and_redo_availability() {
    let (mut app, _) = GuiApp::new();

    assert_eq!(app.history_status_text(), "Undo: no    Redo: no");

    app.input.set("10".to_string());
    app.submit_input();
    assert_eq!(app.history_status_text(), "Undo: yes    Redo: no");

    app.undo();
    assert_eq!(app.history_status_text(), "Undo: no    Redo: yes");
}

#[test]
fn help_menu_messages_open_help_window() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::ShowAbout);
    assert!(app.help_window.is_some());
    assert_eq!(app.help_topic, HelpTopic::About);
    assert_eq!(app.status, "Opened help");

    let _ = app.update(Message::ShowKeyboardShortcuts);
    assert_eq!(app.help_topic, HelpTopic::KeyboardShortcuts);
    assert_eq!(app.status, "Help is already open");
}

#[test]
fn show_details_opens_details_window_for_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    let _ = app.update(Message::ShowDetails(ExpressionId::new(1)));

    assert!(app.details_window.is_some());
    assert_eq!(app.details_target, Some(ExpressionId::new(1)));
    assert_eq!(app.status, "Opened details for $1");
}

#[test]
fn show_details_reuses_open_details_window() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    let _ = app.update(Message::ShowDetails(ExpressionId::new(1)));
    let window = app.details_window;
    let _ = app.update(Message::ShowDetails(ExpressionId::new(2)));

    assert_eq!(app.details_window, window);
    assert_eq!(app.details_target, Some(ExpressionId::new(2)));
    assert_eq!(app.status, "Showing details for $2");
}

#[test]
fn closing_details_window_clears_details_state() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    let _ = app.update(Message::ShowDetails(ExpressionId::new(1)));
    let window = app.details_window.unwrap();

    let _ = app.update(Message::WindowClosed(window));

    assert_eq!(app.details_window, None);
    assert_eq!(app.details_target, None);
}

#[test]
fn selecting_entry_updates_selection_without_entering_edit() {
    let (mut app, _) = GuiApp::new();
    app.input.set("21".to_string());
    app.submit_input();
    app.status = "Ready".to_string();

    app.select_entry(ExpressionId::new(1));

    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
    assert_eq!(app.status, "Ready");
}

#[test]
fn right_click_selects_hovered_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();
    app.selected = Some(ExpressionId::new(1));

    app.hovered_entry = Some(ExpressionId::new(2));
    app.select_hovered_entry();

    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
}

#[test]
fn selecting_different_entry_cancels_edit() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    app.select_entry(ExpressionId::new(2));

    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
    assert_eq!(app.status, "Edit cancelled");
}

#[test]
fn selecting_edited_entry_keeps_edit_active() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    app.select_entry(ExpressionId::new(1));

    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "10");
    assert_eq!(app.status, "Editing $1");
}

#[test]
fn right_click_ignores_cleared_hovered_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.hovered_entry = Some(ExpressionId::new(1));
    app.clear_hovered_entry(ExpressionId::new(1));

    app.selected = None;
    app.select_hovered_entry();

    assert_eq!(app.selected, None);
}

#[test]
fn arrow_navigation_selects_first_or_last_entry_when_none_selected() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();
    app.selected = None;
    app.editing = None;
    app.input.clear();

    app.move_selection(SelectionDirection::Next);
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");

    app.selected = None;
    app.editing = None;
    app.input.clear();
    app.move_selection(SelectionDirection::Previous);
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
}

#[test]
fn arrow_navigation_moves_selection_and_stops_at_edges() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    app.selected = Some(ExpressionId::new(1));
    app.move_selection(SelectionDirection::Next);
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");

    app.move_selection(SelectionDirection::Next);
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");

    app.move_selection(SelectionDirection::Previous);
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");

    app.move_selection(SelectionDirection::Previous);
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
}

#[test]
fn arrow_navigation_from_edit_cancels_edit_when_selection_changes() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    app.move_selection(SelectionDirection::Next);

    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
    assert_eq!(app.status, "Edit cancelled");
}

#[test]
fn arrow_navigation_at_edge_keeps_edit_active_when_selection_does_not_change() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(2));
    app.move_selection(SelectionDirection::Next);

    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, Some(ExpressionId::new(2)));
    assert_eq!(app.input.source(), "20");
    assert_eq!(app.status, "Editing $2");
}

#[test]
fn arrow_navigation_is_disabled_while_selected_input_is_modified() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    app.select_entry(ExpressionId::new(1));
    app.input.set("draft".to_string());
    app.move_selection(SelectionDirection::Next);
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "draft");
}

#[test]
fn submit_edit_recomputes_dependents() {
    let (mut app, _) = GuiApp::new();
    app.input.set("base = 10".to_string());
    app.submit_input();
    app.input.set("$1 * 2".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    assert_eq!(app.input.source(), "base = 10");

    app.input.set("base = 20".to_string());
    app.submit_input();

    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(20)));
    assert_eq!(app.entries[0].source, "20");
    assert_eq!(app.entries[1].state, EntryState::Value(Number::from(40)));
}

#[test]
fn recalculate_entry_refreshes_cached_target_and_dependents() {
    let (mut app, _) = GuiApp::new();
    app.input.set("x + 1".to_string());
    app.submit_input();
    app.input.set("$1 * 2".to_string());
    app.submit_input();

    app.input.set("x = 3".to_string());
    app.submit_input();
    app.recalculate_entry(ExpressionId::new(1));

    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(4)));
    assert_eq!(app.entries[1].state, EntryState::Value(Number::from(8)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
    assert_eq!(app.status, "Recalculated $1");
}

#[test]
fn recalculate_missing_entry_reports_unavailable_status() {
    let (mut app, _) = GuiApp::new();

    app.recalculate_entry(ExpressionId::new(99));

    assert_eq!(app.status, "$99 is not available");
}

#[test]
fn recalculate_all_refreshes_all_cached_entries() {
    let (mut app, _) = GuiApp::new();
    app.input.set("left + 1".to_string());
    app.submit_input();
    app.input.set("right + 2".to_string());
    app.submit_input();
    app.input.set("left = 10".to_string());
    app.submit_input();
    app.input.set("right = 20".to_string());
    app.submit_input();
    app.selected = Some(ExpressionId::new(1));

    app.recalculate_all();

    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(11)));
    assert_eq!(app.entries[1].state, EntryState::Value(Number::from(22)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
    assert_eq!(app.status, "Recalculated all entries");
}

#[test]
fn submit_after_start_edit_updates_existing_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    app.input.set("30".to_string());
    app.submit_input();

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(30)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "30");
}

#[test]
fn edit_input_does_not_update_result_column_before_submit() {
    let (mut app, _) = GuiApp::new();
    app.input.set("base = 10".to_string());
    app.submit_input();
    app.input.set("$1 * 2".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    let _ = app.update(Message::InputChanged("base = 20".to_string()));

    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(10)));
    assert_eq!(app.entries[0].source, "10");
    assert_eq!(app.entries[1].state, EntryState::Value(Number::from(20)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "base = 20");
}

#[test]
fn edit_input_does_not_show_parse_errors_before_submit() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    let _ = app.update(Message::InputChanged("1 +".to_string()));

    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(10)));
    assert_eq!(app.entries[0].source, "10");
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.editing, Some(ExpressionId::new(1)));
}

#[test]
fn delete_keeps_later_ids_available() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    app.delete_entry(ExpressionId::new(1));
    assert_eq!(
        app.pending_confirmation,
        Some(Confirmation::Delete(ExpressionId::new(1)))
    );
    let _ = app.update(Message::ConfirmPending);

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(2));
}

#[test]
fn delete_key_removes_selected_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();
    app.editing = None;
    app.input.clear();
    app.selected = Some(ExpressionId::new(1));

    app.handle_keyboard_event(keyboard::Event::KeyPressed {
        key: Key::Named(key::Named::Delete),
        modified_key: Key::Named(key::Named::Delete),
        physical_key: key::Physical::Code(key::Code::Delete),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    });

    assert_eq!(
        app.pending_confirmation,
        Some(Confirmation::Delete(ExpressionId::new(1)))
    );
    let _ = app.update(Message::ConfirmPending);

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(2));
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.status, "Removed $1");
}

#[test]
fn undo_and_redo_refresh_entries_and_reset_edit_state() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.start_edit(ExpressionId::new(1));

    app.undo();
    assert!(app.entries.is_empty());
    assert_eq!(app.selected, None);
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
    assert_eq!(app.status, "Undone");

    app.redo();
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(10)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.status, "Redone");
}

#[test]
fn keyboard_shortcuts_trigger_undo_and_redo() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    app.handle_keyboard_event(character_key_event("z", keyboard::Modifiers::CTRL));
    assert!(app.entries.is_empty());
    assert_eq!(app.status, "Undone");

    app.handle_keyboard_event(character_key_event("y", keyboard::Modifiers::CTRL));
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(10)));
    assert_eq!(app.status, "Redone");
}

#[test]
fn load_result_replaces_entries_and_resets_edit_state() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.start_edit(ExpressionId::new(1));
    app.hovered_entry = Some(ExpressionId::new(1));
    app.draft_entry = Some(ExpressionId::new(3));

    let mut loaded = dagcal_core::Engine::new();
    loaded.execute("x = 2");
    loaded.execute("x + 3");

    app.finish_load(LoadResult::Loaded(
        PathBuf::from("session.json"),
        loaded.snapshot(),
    ));

    assert_eq!(app.entries.len(), 2);
    assert_eq!(app.entries[0].name.as_deref(), Some("x"));
    assert_eq!(app.entries[1].state, EntryState::Value(Number::from(5)));
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.draft_entry, None);
    assert_eq!(app.hovered_entry, None);
    assert_eq!(app.input.source(), "");
    assert_eq!(app.status, "Loaded session.json");

    app.undo();
    assert_eq!(app.status, "Nothing to undo");
    assert_eq!(app.entries.len(), 2);
}

#[test]
fn load_failure_preserves_current_entries() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    let before = app.engine.snapshot();

    app.finish_load(LoadResult::Failed("could not parse JSON (bad)".to_string()));

    assert_eq!(app.engine.snapshot(), before);
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.status, "Load failed: could not parse JSON (bad)");
}

#[test]
fn save_and_load_cancel_leave_state_unchanged() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    let before = app.engine.snapshot();

    app.finish_save(SaveResult::Cancelled);
    assert_eq!(app.engine.snapshot(), before);
    assert_eq!(app.status, "Save cancelled");

    app.finish_load(LoadResult::Cancelled);
    assert_eq!(app.engine.snapshot(), before);
    assert_eq!(app.status, "Load cancelled");
}

#[test]
fn invalid_loaded_snapshot_is_reported_without_mutating_app() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    let before = app.engine.snapshot();

    let invalid = EngineSnapshot::new(vec![PersistedEntry {
        id: 0,
        name: None,
        source: "1".to_string(),
    }]);
    app.finish_load(LoadResult::Loaded(PathBuf::from("invalid.json"), invalid));

    assert_eq!(app.engine.snapshot(), before);
    assert!(
        app.status
            .starts_with("Load failed: could not restore snapshot")
    );
}

#[test]
fn deleting_edited_entry_selects_fallback_without_entering_edit() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("20".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    app.delete_entry(ExpressionId::new(1));
    let _ = app.update(Message::ConfirmPending);

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
}

#[test]
fn delete_confirmation_cancel_preserves_entry_state() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    let before = app.engine.snapshot();

    app.delete_entry(ExpressionId::new(1));
    assert_eq!(
        app.pending_confirmation,
        Some(Confirmation::Delete(ExpressionId::new(1)))
    );

    let _ = app.update(Message::CancelConfirmation);

    assert_eq!(app.engine.snapshot(), before);
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.pending_confirmation, None);
    assert_eq!(app.status, "Action cancelled");
}

#[test]
fn dirty_state_tracks_saved_snapshot() {
    let (mut app, _) = GuiApp::new();

    assert!(!app.is_dirty());
    assert_eq!(app.main_title(), "dagcal - Untitled");
    assert_eq!(app.file_status_text(), "File: Untitled    Saved");

    app.input.set("10".to_string());
    app.submit_input();

    assert!(app.is_dirty());
    assert_eq!(app.main_title(), "* dagcal - Untitled");
    assert_eq!(app.file_status_text(), "File: Untitled    Unsaved changes");

    let snapshot = app.engine.snapshot();
    app.finish_save(SaveResult::Saved(
        PathBuf::from("session.json"),
        snapshot.clone(),
    ));

    assert!(!app.is_dirty());
    assert_eq!(app.current_path, Some(PathBuf::from("session.json")));
    assert_eq!(app.saved_snapshot, snapshot);
    assert_eq!(app.main_title(), "dagcal - session.json");
    assert_eq!(app.file_status_text(), "File: session.json    Saved");
}

#[test]
fn load_result_marks_loaded_snapshot_clean_and_sets_current_path() {
    let (mut app, _) = GuiApp::new();
    let mut loaded = dagcal_core::Engine::new();
    loaded.execute("x = 2");
    let loaded_snapshot = loaded.snapshot();

    app.finish_load(LoadResult::Loaded(
        PathBuf::from("session.json"),
        loaded_snapshot,
    ));

    assert!(!app.is_dirty());
    assert_eq!(app.current_path, Some(PathBuf::from("session.json")));
    assert_eq!(app.file_status_text(), "File: session.json    Saved");
}

#[test]
fn dirty_load_clear_and_quit_open_confirmation_before_mutating() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    let before = app.engine.snapshot();

    let _ = app.update(Message::Load);
    assert_eq!(app.pending_confirmation, Some(Confirmation::Load));
    assert_eq!(app.engine.snapshot(), before);

    let _ = app.update(Message::CancelConfirmation);
    let _ = app.update(Message::Clear);
    assert_eq!(app.pending_confirmation, Some(Confirmation::Clear));
    assert_eq!(app.engine.snapshot(), before);

    let _ = app.update(Message::CancelConfirmation);
    let _ = app.update(Message::Quit);
    assert_eq!(app.pending_confirmation, Some(Confirmation::Quit));
    assert_eq!(app.engine.snapshot(), before);
}

#[test]
fn confirming_clear_removes_entries_and_keeps_dirty_state() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    let snapshot = app.engine.snapshot();
    app.finish_save(SaveResult::Saved(PathBuf::from("session.json"), snapshot));
    assert!(!app.is_dirty());

    let _ = app.update(Message::Clear);

    assert!(app.entries.is_empty());
    assert!(app.is_dirty());
    assert_eq!(app.status, "Cleared");
}

#[test]
fn submitting_empty_new_expression_creates_history_entry() {
    let (mut app, _) = GuiApp::new();

    app.submit_input();

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].source, "");
    assert!(matches!(app.entries[0].state, EntryState::Error(_)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "");
}

#[test]
fn new_expression_input_keeps_expression_in_input_and_empty_draft_in_history() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InputChanged("1".to_string()));
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].source, "");
    assert!(matches!(app.entries[0].state, EntryState::Error(_)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "1");

    let _ = app.update(Message::InputChanged("1 +".to_string()));
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].source, "");
    assert!(matches!(app.entries[0].state, EntryState::Error(_)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "1 +");
}

#[test]
fn new_expression_input_saves_named_definition_on_submit() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InputChanged("x=2".to_string()));
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].source, "");
    assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));

    let _ = app.update(Message::Submit);

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].name.as_deref(), Some("x"));
    assert_eq!(app.entries[0].source, "2");
    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(2)));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.draft_entry, None);
}

#[test]
fn edited_entry_can_be_saved_as_named_definition() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();

    app.start_edit(ExpressionId::new(1));
    app.input.set("x=2".to_string());
    app.submit_input();

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].name.as_deref(), Some("x"));
    assert_eq!(app.entries[0].source, "2");
    assert_eq!(app.entries[0].state, EntryState::Value(Number::from(2)));
    assert_eq!(app.input.source(), "x = 2");
}

#[test]
fn new_entry_clears_input_and_creates_empty_draft_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.input.set("draft".to_string());

    let _ = app.update(Message::NewEntry);

    assert_eq!(app.entries.len(), 2);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].source, "10");
    assert_eq!(app.entries[1].id, ExpressionId::new(2));
    assert_eq!(app.entries[1].source, "");
    assert!(matches!(app.entries[1].state, EntryState::Error(_)));
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.draft_entry, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "");
}

#[test]
fn new_entry_from_editing_does_not_overwrite_selected_entry() {
    let (mut app, _) = GuiApp::new();
    app.input.set("10".to_string());
    app.submit_input();
    app.start_edit(ExpressionId::new(1));

    let _ = app.update(Message::NewEntry);
    let _ = app.update(Message::InputChanged("20".to_string()));

    assert_eq!(app.entries.len(), 2);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.entries[0].source, "10");
    assert_eq!(app.entries[1].id, ExpressionId::new(2));
    assert_eq!(app.entries[1].source, "");
    assert!(matches!(app.entries[1].state, EntryState::Error(_)));
    assert_eq!(app.selected, Some(ExpressionId::new(2)));
    assert_eq!(app.draft_entry, Some(ExpressionId::new(2)));
    assert_eq!(app.editing, None);
    assert_eq!(app.input.source(), "20");
}

#[test]
fn new_entry_after_empty_submit_reuses_existing_empty_entry() {
    let (mut app, _) = GuiApp::new();

    app.submit_input();
    let _ = app.update(Message::NewEntry);

    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].id, ExpressionId::new(1));
    assert_eq!(app.selected, Some(ExpressionId::new(1)));
    assert_eq!(app.draft_entry, Some(ExpressionId::new(1)));
    assert_eq!(app.input.source(), "");
    assert_eq!(app.editing, None);
}

fn character_key_event(value: &str, modifiers: keyboard::Modifiers) -> keyboard::Event {
    keyboard::Event::KeyPressed {
        key: Key::Character(value.into()),
        modified_key: Key::Character(value.into()),
        physical_key: key::Physical::Code(key::Code::KeyZ),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    }
}

fn named_key_event(named: key::Named) -> keyboard::Event {
    keyboard::Event::KeyPressed {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: key::Physical::Code(key::Code::Tab),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    }
}
