use super::*;
use dagcal_app::{
    CompletionDirection, CompletionKind, Draft, EngineSnapshot, EntryState, ExpressionId, Number,
    PersistedEntry, SelectionDirection,
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
    app.session
        .dispatch(AppAction::InputEdited("x = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::InsertReference(ExpressionId::new(1)));

    assert_eq!(app.session.input_source(), "x ");
    assert_eq!(app.session.status(), "Inserted x");
}

#[test]
fn use_inserts_result_id_for_unnamed_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::InsertReference(ExpressionId::new(1)));

    assert_eq!(app.session.input_source(), "$1 ");
    assert_eq!(app.session.status(), "Inserted $1");
}

#[test]
fn menu_constant_inserts_constant_name() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InsertConstant("pi".to_string()));

    assert_eq!(app.session.input_source(), "pi ");
    assert_eq!(app.session.status(), "Inserted pi");
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(1)));
}

#[test]
fn menu_function_inserts_call_template() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InsertFunction("sin".to_string()));

    assert_eq!(app.session.input_source(), "sin() ");
    assert_eq!(app.session.status(), "Inserted sin()");
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(1)));
}

#[test]
fn input_change_opens_completion_for_named_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("subtotal = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::InputChanged("sub".to_string()));

    assert!(app.session.completion_is_open());
    assert!(app.session.completion_candidates().iter().any(|candidate| {
        candidate.label == "subtotal" && candidate.kind == CompletionKind::Entry
    }));
}

#[test]
fn submit_accepts_completion_before_saving_input() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("subtotal = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::InputChanged("sub".to_string()));
    let _ = app.update(Message::Submit);

    assert_eq!(app.session.input_source(), "subtotal");
    assert_eq!(app.session.entries().len(), 2);
    assert_eq!(app.session.entries()[0].name.as_deref(), Some("subtotal"));
    assert_eq!(app.session.entries()[1].source, "");
    assert_eq!(app.session.status(), "Inserted subtotal");
}

#[test]
fn completion_finds_result_references_and_moves_selection() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::InputChanged("$".to_string()));
    assert_eq!(app.session.selected_completion_index(), Some(0));

    let effect = app.handle_keyboard_event(named_key_event(key::Named::ArrowDown));
    assert_eq!(app.session.selected_completion_index(), Some(1));
    assert_eq!(
        effect,
        super::effects::UiEffect::ScrollToCompletionSelectionEdge(CompletionDirection::Next)
    );

    let effect = app.handle_keyboard_event(named_key_event(key::Named::ArrowUp));
    assert_eq!(app.session.selected_completion_index(), Some(0));
    assert_eq!(
        effect,
        super::effects::UiEffect::ScrollToCompletionSelectionEdge(CompletionDirection::Previous)
    );
}

#[test]
fn completion_arrow_navigation_does_not_scroll_when_selection_stays_at_edge() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::InputChanged("$".to_string()));
    assert_eq!(app.session.selected_completion_index(), Some(0));

    let effect = app.handle_keyboard_event(named_key_event(key::Named::ArrowUp));

    assert_eq!(app.session.selected_completion_index(), Some(0));
    assert_eq!(effect, super::effects::UiEffect::None);
}

#[test]
fn tab_accepts_selected_completion() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::InputChanged("$".to_string()));
    let effect = app.handle_keyboard_event(named_key_event(key::Named::Tab));

    assert_eq!(app.session.input_source(), "$1");
    assert_eq!(app.session.status(), "Inserted $1");
    assert!(!app.session.completion_is_open());
    assert_eq!(effect, super::effects::UiEffect::FocusInput);
}

#[test]
fn escape_closes_completion() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::InputChanged("$".to_string()));
    app.handle_keyboard_event(named_key_event(key::Named::Escape));

    assert!(!app.session.completion_is_open());
    assert_eq!(app.session.input_source(), "$");
}

#[test]
fn dismiss_completion_message_closes_completion() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::InputChanged("$".to_string()));
    let _ = app.update(Message::DismissCompletion);

    assert!(!app.session.completion_is_open());
    assert_eq!(app.session.input_source(), "$");
}

#[test]
fn status_bar_history_text_reflects_undo_and_redo_availability() {
    let (mut app, _) = GuiApp::new();

    assert_eq!(app.history_status_text(), "Undo: no    Redo: no");

    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    assert_eq!(app.history_status_text(), "Undo: yes    Redo: no");

    app.session.dispatch(AppAction::Undo);
    assert_eq!(app.history_status_text(), "Undo: no    Redo: yes");
}

#[test]
fn entry_search_matches_displayed_id_name_expression_result_and_error_text() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("subtotal = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("$1 * 2".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("1 / 0".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::EntrySearchChanged("$3".to_string()));
    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(3)]);

    app.session
        .dispatch(AppAction::EntrySearchChanged("subtotal".to_string()));
    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(1)]);

    app.session
        .dispatch(AppAction::EntrySearchChanged("$1 * 2".to_string()));
    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(2)]);

    app.session
        .dispatch(AppAction::EntrySearchChanged("20".to_string()));
    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(2)]);

    app.session
        .dispatch(AppAction::EntrySearchChanged("error".to_string()));
    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(3)]);
}

#[test]
fn entry_search_is_closed_by_default_and_opens_with_focus_effect() {
    let (mut app, _) = GuiApp::new();

    assert!(!app.session.entry_search_is_open());

    let effect = app.open_entry_search();

    assert!(app.session.entry_search_is_open());
    assert_eq!(effect, super::effects::UiEffect::FocusEntrySearch);
}

#[test]
fn ctrl_f_opens_entry_search() {
    let (mut app, _) = GuiApp::new();

    let effect = app.handle_keyboard_event(character_key_event("f", keyboard::Modifiers::CTRL));

    assert!(app.session.entry_search_is_open());
    assert_eq!(effect, super::effects::UiEffect::FocusEntrySearch);
}

#[test]
fn entry_state_filter_returns_values_or_errors() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("1 / 0".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::EntryStateFilterChanged(EntryStateFilter::Values));
    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(1)]);

    app.session
        .dispatch(AppAction::EntryStateFilterChanged(EntryStateFilter::Errors));
    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(2)]);
}

#[test]
fn entry_search_and_state_filter_combine() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("subtotal = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("subtotal / 0".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::EntrySearchChanged("subtotal".to_string()));
    app.session
        .dispatch(AppAction::EntryStateFilterChanged(EntryStateFilter::Errors));

    assert_eq!(filtered_entry_ids(&app), vec![ExpressionId::new(2)]);
}

#[test]
fn entry_count_status_reports_visible_and_total_when_filtered() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    assert_eq!(app.session.entry_count_status_text(), "Entries: 2");

    app.session
        .dispatch(AppAction::EntrySearchChanged("20".to_string()));

    assert_eq!(app.session.entry_count_status_text(), "Entries: 1 / 2");
}

#[test]
fn clearing_entry_search_resets_query_and_state_filter() {
    let (mut app, _) = GuiApp::new();
    app.open_entry_search();
    app.session
        .dispatch(AppAction::EntrySearchChanged("error".to_string()));
    app.session
        .dispatch(AppAction::EntryStateFilterChanged(EntryStateFilter::Errors));

    app.session.dispatch(AppAction::ClearEntrySearch);

    assert!(!app.session.entry_search_is_open());
    assert_eq!(app.session.entry_search_query(), "");
    assert_eq!(app.session.entry_state_filter(), EntryStateFilter::All);
}

#[test]
fn escape_closes_entry_search_after_completion_priority() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.open_entry_search();
    app.session
        .dispatch(AppAction::EntrySearchChanged("10".to_string()));

    let _ = app.update(Message::InputChanged("$".to_string()));
    app.handle_keyboard_event(named_key_event(key::Named::Escape));
    assert!(app.session.entry_search_is_open());
    assert_eq!(app.session.entry_search_query(), "10");
    assert!(!app.session.completion_is_open());

    app.handle_keyboard_event(named_key_event(key::Named::Escape));
    assert!(!app.session.entry_search_is_open());
    assert_eq!(app.session.entry_search_query(), "");
}

#[test]
fn help_menu_messages_open_help_window() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::ShowAbout);
    assert!(app.help_window.is_some());
    assert_eq!(app.help_topic, HelpTopic::About);
    assert_eq!(app.session.status(), "Opened help");

    let _ = app.update(Message::ShowKeyboardShortcuts);
    assert_eq!(app.help_topic, HelpTopic::KeyboardShortcuts);
    assert_eq!(app.session.status(), "Help is already open");
}

#[test]
fn show_details_opens_details_window_for_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::ShowDetails(ExpressionId::new(1)));

    assert!(app.details_window.is_some());
    assert_eq!(app.details_target, Some(ExpressionId::new(1)));
    assert_eq!(app.session.status(), "Opened details for $1");
}

#[test]
fn show_details_reuses_open_details_window() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::ShowDetails(ExpressionId::new(1)));
    let window = app.details_window;
    let _ = app.update(Message::ShowDetails(ExpressionId::new(2)));

    assert_eq!(app.details_window, window);
    assert_eq!(app.details_target, Some(ExpressionId::new(2)));
    assert_eq!(app.session.status(), "Showing details for $2");
}

#[test]
fn closing_details_window_clears_details_state() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    let _ = app.update(Message::ShowDetails(ExpressionId::new(1)));
    let window = app.details_window.unwrap();

    let _ = app.update(Message::WindowClosed(window));

    assert_eq!(app.details_window, None);
    assert_eq!(app.details_target, None);
}

#[test]
fn selecting_entry_updates_selection_without_entering_edit() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("21".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::SetStatus("Ready".to_string()));

    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.status(), "Ready");
}

#[test]
fn right_click_selects_hovered_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    app.session
        .dispatch(AppAction::SetHoveredEntry(ExpressionId::new(2)));
    app.session.dispatch(AppAction::SelectHoveredEntry);

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
}

#[test]
fn selecting_different_entry_cancels_edit() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(2)));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.status(), "Edit cancelled");
}

#[test]
fn selecting_edited_entry_keeps_edit_active() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "10");
    assert_eq!(app.session.status(), "Editing $1");
}

#[test]
fn right_click_ignores_cleared_hovered_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::SetHoveredEntry(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::ClearHoveredEntry(ExpressionId::new(1)));

    app.session.dispatch(AppAction::ClearSelection);
    app.session.dispatch(AppAction::SelectHoveredEntry);

    assert_eq!(app.session.selected_id(), None);
}

#[test]
fn arrow_navigation_selects_first_or_last_entry_when_none_selected() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session.dispatch(AppAction::ClearSelection);
    app.session.dispatch(AppAction::CancelEdit);
    app.session.dispatch(AppAction::ResetInput);

    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Next));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");

    app.session.dispatch(AppAction::ClearSelection);
    app.session.dispatch(AppAction::CancelEdit);
    app.session.dispatch(AppAction::ResetInput);
    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Previous));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
}

#[test]
fn arrow_navigation_moves_selection_and_stops_at_edges() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Next));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");

    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Next));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");

    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Previous));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");

    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Previous));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
}

#[test]
fn keyboard_arrow_navigation_scrolls_moved_selection_to_edge() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));
    app.session.dispatch(AppAction::CancelEdit);
    app.session.dispatch(AppAction::ResetInput);

    let effect = app.handle_keyboard_event(named_key_event(key::Named::ArrowDown));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(
        effect,
        super::effects::UiEffect::ScrollToSelectionEdge(SelectionDirection::Next)
    );

    let effect = app.handle_keyboard_event(named_key_event(key::Named::ArrowUp));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(
        effect,
        super::effects::UiEffect::ScrollToSelectionEdge(SelectionDirection::Previous)
    );
}

#[test]
fn keyboard_arrow_navigation_does_not_scroll_when_selection_stays_at_edge() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));
    app.session.dispatch(AppAction::CancelEdit);
    app.session.dispatch(AppAction::ResetInput);

    let effect = app.handle_keyboard_event(named_key_event(key::Named::ArrowDown));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(effect, super::effects::UiEffect::None);
}

#[test]
fn arrow_navigation_from_edit_cancels_edit_when_selection_changes() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Next));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.status(), "Edit cancelled");
}

#[test]
fn arrow_navigation_at_edge_keeps_edit_active_when_selection_does_not_change() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(2)));
    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Next));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.input_source(), "20");
    assert_eq!(app.session.status(), "Editing $2");
}

#[test]
fn arrow_navigation_uses_visible_filtered_entries() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("30".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session.dispatch(AppAction::ResetInput);
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    app.session
        .dispatch(AppAction::EntrySearchChanged("20".to_string()));
    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Next));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));

    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(3)));
    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Previous));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
}

#[test]
fn arrow_navigation_is_disabled_while_selected_input_is_modified() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::InputEdited("draft".to_string()));
    app.session
        .dispatch(AppAction::MoveSelection(SelectionDirection::Next));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "draft");
}

#[test]
fn keyboard_navigation_is_ignored_while_input_is_modified() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::InputEdited("draft".to_string()));
    app.handle_keyboard_event(named_key_event(key::Named::ArrowDown));
    app.handle_keyboard_event(named_key_event(key::Named::Delete));

    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.entries().len(), 2);
    assert_eq!(app.session.input_source(), "draft");
}

#[test]
fn submit_edit_recomputes_dependents() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("base = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("$1 * 2".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "base = 10");

    app.session
        .dispatch(AppAction::InputEdited("base = 20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(20))
    );
    assert_eq!(app.session.entries()[0].source, "20");
    assert_eq!(
        app.session.entries()[1].state,
        EntryState::Value(Number::from(40))
    );
}

#[test]
fn recalculate_entry_refreshes_cached_target_and_dependents() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("x + 1".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("$1 * 2".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::InputEdited("x = 3".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::RecalculateEntry(ExpressionId::new(1)));

    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(4))
    );
    assert_eq!(
        app.session.entries()[1].state,
        EntryState::Value(Number::from(8))
    );
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.status(), "Recalculated $1");
}

#[test]
fn recalculate_missing_entry_reports_unavailable_status() {
    let (mut app, _) = GuiApp::new();

    app.session
        .dispatch(AppAction::RecalculateEntry(ExpressionId::new(99)));

    assert_eq!(app.session.status(), "$99 is not available");
}

#[test]
fn recalculate_all_refreshes_all_cached_entries() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("left + 1".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("right + 2".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("left = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("right = 20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    app.session.dispatch(AppAction::RecalculateAll);

    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(11))
    );
    assert_eq!(
        app.session.entries()[1].state,
        EntryState::Value(Number::from(22))
    );
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.status(), "Recalculated all entries");
}

#[test]
fn submit_after_start_edit_updates_existing_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::InputEdited("30".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(30))
    );
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "30");
}

#[test]
fn edit_input_does_not_update_result_column_before_submit() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("base = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("$1 * 2".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    let _ = app.update(Message::InputChanged("base = 20".to_string()));

    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(10))
    );
    assert_eq!(app.session.entries()[0].source, "10");
    assert_eq!(
        app.session.entries()[1].state,
        EntryState::Value(Number::from(20))
    );
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "base = 20");
}

#[test]
fn edit_input_does_not_show_parse_errors_before_submit() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    let _ = app.update(Message::InputChanged("1 +".to_string()));

    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(10))
    );
    assert_eq!(app.session.entries()[0].source, "10");
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.editing_id(), Some(ExpressionId::new(1)));
}

#[test]
fn delete_keeps_later_ids_available() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::DeleteEntry(ExpressionId::new(1)));

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(2));
    assert_eq!(app.pending_confirmation, None);
    assert_eq!(app.confirmation_window, None);
}

#[test]
fn delete_key_removes_selected_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session.dispatch(AppAction::CancelEdit);
    app.session.dispatch(AppAction::ResetInput);
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    app.handle_keyboard_event(keyboard::Event::KeyPressed {
        key: Key::Named(key::Named::Delete),
        modified_key: Key::Named(key::Named::Delete),
        physical_key: key::Physical::Code(key::Code::Delete),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    });

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(2));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.pending_confirmation, None);
    assert_eq!(app.confirmation_window, None);
    assert_eq!(app.session.status(), "Removed $1");
}

#[test]
fn undo_and_redo_refresh_entries_and_reset_edit_state() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));

    app.session.dispatch(AppAction::Undo);
    assert!(app.session.entries().is_empty());
    assert_eq!(app.session.selected_id(), None);
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.status(), "Undone");

    app.session.dispatch(AppAction::Redo);
    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(10))
    );
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.status(), "Redone");
}

#[test]
fn keyboard_shortcuts_trigger_undo_and_redo() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.handle_keyboard_event(character_key_event("z", keyboard::Modifiers::CTRL));
    assert!(app.session.entries().is_empty());
    assert_eq!(app.session.status(), "Undone");

    app.handle_keyboard_event(character_key_event("y", keyboard::Modifiers::CTRL));
    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(10))
    );
    assert_eq!(app.session.status(), "Redone");
}

#[test]
fn keyboard_shift_z_triggers_redo() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.handle_keyboard_event(character_key_event("z", keyboard::Modifiers::CTRL));
    assert!(app.session.entries().is_empty());

    app.handle_keyboard_event(character_key_event(
        "z",
        keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
    ));

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(10))
    );
    assert_eq!(app.session.status(), "Redone");
}

#[test]
fn keyboard_shortcuts_start_new_edit_and_recalculate_entries() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("left + 1".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("left = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session.dispatch(AppAction::ResetInput);
    app.session.dispatch(AppAction::CancelEdit);
    app.session
        .dispatch(AppAction::SelectEntry(ExpressionId::new(1)));

    let _ = app.handle_keyboard_task(character_key_event("e", keyboard::Modifiers::CTRL));
    assert_eq!(app.session.editing_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "left + 1");

    app.handle_keyboard_event(named_key_event(key::Named::Escape));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");

    app.handle_keyboard_event(named_key_event(key::Named::F2));
    assert_eq!(app.session.editing_id(), Some(ExpressionId::new(1)));

    app.handle_keyboard_event(named_key_event(key::Named::Escape));
    app.handle_keyboard_event(named_key_event(key::Named::F5));
    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(11))
    );
    assert_eq!(app.session.status(), "Recalculated $1");

    let _ = app.handle_keyboard_task(character_key_event("n", keyboard::Modifiers::CTRL));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(3)));
}

#[test]
fn keyboard_ctrl_r_recalculates_all_entries() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("left + 1".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("left = 10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.handle_keyboard_task(character_key_event("r", keyboard::Modifiers::CTRL));

    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(11))
    );
    assert_eq!(app.session.status(), "Recalculated all entries");
}

#[test]
fn dirty_file_shortcuts_request_confirmation_before_load_or_quit() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.handle_keyboard_task(character_key_event("o", keyboard::Modifiers::CTRL));
    assert_eq!(app.pending_confirmation, Some(Confirmation::Load));
    assert_eq!(app.session.status(), "Confirm load");

    app.pending_confirmation = None;
    app.confirmation_window = None;

    let _ = app.handle_keyboard_task(character_key_event("q", keyboard::Modifiers::CTRL));
    assert_eq!(app.pending_confirmation, Some(Confirmation::Quit));
    assert_eq!(app.session.status(), "Confirm quit");
}

#[test]
fn load_result_replaces_entries_and_resets_edit_state() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::SetHoveredEntry(ExpressionId::new(1)));

    let mut loaded = dagcal_app::Engine::new();
    loaded.execute("x = 2");
    loaded.execute("x + 3");

    app.finish_load(LoadResult::Loaded(
        PathBuf::from("session.json"),
        loaded.snapshot(),
    ));

    assert_eq!(app.session.entries().len(), 2);
    assert_eq!(app.session.entries()[0].name.as_deref(), Some("x"));
    assert_eq!(
        app.session.entries()[1].state,
        EntryState::Value(Number::from(5))
    );
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.draft_entry_id(), None);
    assert_eq!(app.session.hovered_entry_id(), None);
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.status(), "Loaded session.json");

    app.session.dispatch(AppAction::Undo);
    assert_eq!(app.session.status(), "Nothing to undo");
    assert_eq!(app.session.entries().len(), 2);
}

#[test]
fn load_failure_preserves_current_entries() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    let before = app.session.snapshot();

    app.finish_load(LoadResult::Failed("could not parse JSON (bad)".to_string()));

    assert_eq!(app.session.snapshot(), before);
    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(
        app.session.status(),
        "Load failed: could not parse JSON (bad)"
    );
}

#[test]
fn save_and_load_cancel_leave_state_unchanged() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    let before = app.session.snapshot();

    app.finish_save(SaveResult::Cancelled);
    assert_eq!(app.session.snapshot(), before);
    assert_eq!(app.session.status(), "Save cancelled");

    app.finish_load(LoadResult::Cancelled);
    assert_eq!(app.session.snapshot(), before);
    assert_eq!(app.session.status(), "Load cancelled");
}

#[test]
fn invalid_loaded_snapshot_is_reported_without_mutating_app() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    let before = app.session.snapshot();

    let invalid = EngineSnapshot::new(vec![PersistedEntry {
        id: 0,
        name: None,
        source: "1".to_string(),
    }]);
    app.finish_load(LoadResult::Loaded(PathBuf::from("invalid.json"), invalid));

    assert_eq!(app.session.snapshot(), before);
    assert!(
        app.session
            .status()
            .starts_with("Load failed: could not restore snapshot")
    );
}

#[test]
fn deleting_edited_entry_selects_fallback_without_entering_edit() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("20".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::DeleteEntry(ExpressionId::new(1)));

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
}

#[test]
fn delete_removes_entry_without_confirmation() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::DeleteEntry(ExpressionId::new(1)));

    assert!(app.session.entry(ExpressionId::new(1)).is_none());
    assert!(app.session.entries().is_empty());
    assert_eq!(app.pending_confirmation, None);
    assert_eq!(app.confirmation_window, None);
    assert_eq!(app.session.status(), "Removed $1");
}

#[test]
fn dirty_state_tracks_saved_snapshot() {
    let (mut app, _) = GuiApp::new();

    assert!(!app.is_dirty());
    assert_eq!(app.main_title(), "dagcal - Untitled");
    assert_eq!(app.file_status_text(), "File: Untitled    Saved");

    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    assert!(app.is_dirty());
    assert_eq!(app.main_title(), "* dagcal - Untitled");
    assert_eq!(app.file_status_text(), "File: Untitled    Unsaved changes");

    let snapshot = app.session.snapshot();
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
    let mut loaded = dagcal_app::Engine::new();
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
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    let before = app.session.snapshot();

    let _ = app.update(Message::Load);
    assert_eq!(app.pending_confirmation, Some(Confirmation::Load));
    assert!(app.confirmation_window.is_some());
    assert_eq!(app.session.snapshot(), before);

    let _ = app.update(Message::CancelConfirmation);
    assert_eq!(app.confirmation_window, None);
    let _ = app.update(Message::Clear);
    assert_eq!(app.pending_confirmation, Some(Confirmation::Clear));
    assert!(app.confirmation_window.is_some());
    assert_eq!(app.session.snapshot(), before);

    let _ = app.update(Message::CancelConfirmation);
    assert_eq!(app.confirmation_window, None);
    let _ = app.update(Message::Quit);
    assert_eq!(app.pending_confirmation, Some(Confirmation::Quit));
    assert!(app.confirmation_window.is_some());
    assert_eq!(app.session.snapshot(), before);
}

#[test]
fn closing_dirty_main_window_opens_confirmation_window() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    let main_window = app.main_window.unwrap();

    let _ = app.update(Message::WindowClosed(main_window));

    assert_eq!(
        app.pending_confirmation,
        Some(Confirmation::CloseMain(main_window))
    );
    assert!(app.confirmation_window.is_some());
    assert_eq!(app.main_window, Some(main_window));
    assert_eq!(app.session.status(), "Confirm quit");
}

#[test]
fn closing_confirmation_window_cancels_pending_action() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let _ = app.update(Message::Load);
    let confirmation_window = app.confirmation_window.unwrap();
    let before = app.session.snapshot();

    let _ = app.update(Message::WindowClosed(confirmation_window));

    assert_eq!(app.pending_confirmation, None);
    assert_eq!(app.confirmation_window, None);
    assert_eq!(app.session.snapshot(), before);
    assert_eq!(app.session.status(), "Action cancelled");
}

#[test]
fn confirming_clear_removes_entries_and_keeps_dirty_state() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    let snapshot = app.session.snapshot();
    app.finish_save(SaveResult::Saved(PathBuf::from("session.json"), snapshot));
    assert!(!app.is_dirty());

    let _ = app.update(Message::Clear);

    assert!(app.session.entries().is_empty());
    assert!(app.is_dirty());
    assert_eq!(app.session.status(), "Cleared");
}

#[test]
fn submitting_empty_new_expression_creates_history_entry() {
    let (mut app, _) = GuiApp::new();

    app.session.dispatch(AppAction::SubmitInput);

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.entries()[0].source, "");
    assert!(matches!(
        app.session.entries()[0].state,
        EntryState::Error(_)
    ));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "");
}

#[test]
fn new_expression_input_keeps_expression_in_input_and_empty_draft_in_history() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InputChanged("1".to_string()));
    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.entries()[0].source, "");
    assert!(matches!(
        app.session.entries()[0].state,
        EntryState::Error(_)
    ));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "1");

    let _ = app.update(Message::InputChanged("1 +".to_string()));
    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.entries()[0].source, "");
    assert!(matches!(
        app.session.entries()[0].state,
        EntryState::Error(_)
    ));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "1 +");
}

#[test]
fn new_expression_input_saves_named_definition_on_submit() {
    let (mut app, _) = GuiApp::new();

    let _ = app.update(Message::InputChanged("x=2".to_string()));
    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].source, "");
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(1)));

    let _ = app.update(Message::Submit);

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.entries()[0].name.as_deref(), Some("x"));
    assert_eq!(app.session.entries()[0].source, "2");
    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(2))
    );
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.draft_entry_id(), None);
}

#[test]
fn edited_entry_can_be_saved_as_named_definition() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));
    app.session
        .dispatch(AppAction::InputEdited("x=2".to_string()));
    app.session.dispatch(AppAction::SubmitInput);

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.entries()[0].name.as_deref(), Some("x"));
    assert_eq!(app.session.entries()[0].source, "2");
    assert_eq!(
        app.session.entries()[0].state,
        EntryState::Value(Number::from(2))
    );
    assert_eq!(app.session.input_source(), "x = 2");
}

#[test]
fn new_entry_clears_input_and_creates_empty_draft_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::InputEdited("draft".to_string()));

    let _ = app.update(Message::NewEntry);

    assert_eq!(app.session.entries().len(), 2);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.entries()[0].source, "10");
    assert_eq!(app.session.entries()[1].id, ExpressionId::new(2));
    assert_eq!(app.session.entries()[1].source, "");
    assert!(matches!(
        app.session.entries()[1].state,
        EntryState::Error(_)
    ));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "");
}

#[test]
fn new_entry_from_editing_does_not_overwrite_selected_entry() {
    let (mut app, _) = GuiApp::new();
    app.session
        .dispatch(AppAction::InputEdited("10".to_string()));
    app.session.dispatch(AppAction::SubmitInput);
    app.session
        .dispatch(AppAction::StartEdit(ExpressionId::new(1)));

    let _ = app.update(Message::NewEntry);
    let _ = app.update(Message::InputChanged("20".to_string()));

    assert_eq!(app.session.entries().len(), 2);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.entries()[0].source, "10");
    assert_eq!(app.session.entries()[1].id, ExpressionId::new(2));
    assert_eq!(app.session.entries()[1].source, "");
    assert!(matches!(
        app.session.entries()[1].state,
        EntryState::Error(_)
    ));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(2)));
    assert_eq!(app.session.editing_id(), None);
    assert_eq!(app.session.input_source(), "20");
}

#[test]
fn new_entry_after_empty_submit_reuses_existing_empty_entry() {
    let (mut app, _) = GuiApp::new();

    app.session.dispatch(AppAction::SubmitInput);
    let _ = app.update(Message::NewEntry);

    assert_eq!(app.session.entries().len(), 1);
    assert_eq!(app.session.entries()[0].id, ExpressionId::new(1));
    assert_eq!(app.session.selected_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.draft_entry_id(), Some(ExpressionId::new(1)));
    assert_eq!(app.session.input_source(), "");
    assert_eq!(app.session.editing_id(), None);
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

fn filtered_entry_ids(app: &GuiApp) -> Vec<ExpressionId> {
    app.session
        .filtered_entries_iter()
        .map(|entry| entry.id)
        .collect()
}
