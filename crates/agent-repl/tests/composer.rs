//! Composer state + key-handling tests. Render is exercised at the app
//! level via the existing `render.rs` integration tests.

use agent_repl::composer::{Composer, ComposerAction, MenuKind, MAX_VISIBLE_LINES};
use crossterm::event::{KeyCode, KeyModifiers};

fn key(c: KeyCode) -> (KeyCode, KeyModifiers) {
    (c, KeyModifiers::NONE)
}

fn shift(c: KeyCode) -> (KeyCode, KeyModifiers) {
    (c, KeyModifiers::SHIFT)
}

fn ctrl(c: KeyCode) -> (KeyCode, KeyModifiers) {
    (c, KeyModifiers::CONTROL)
}

fn type_str(c: &mut Composer, s: &str) {
    for ch in s.chars() {
        let action = c.handle_key(KeyCode::Char(ch), KeyModifiers::NONE);
        assert_eq!(action, ComposerAction::Consumed, "char {ch} not consumed");
    }
}

// -----------------------------------------------------------------------------
// state basics
// -----------------------------------------------------------------------------

#[test]
fn empty_by_default() {
    let c = Composer::new();
    assert!(c.is_empty());
    assert_eq!(c.text(), "");
    assert_eq!(c.cursor_line(), 0);
    assert_eq!(c.cursor_col(), 0);
}

#[test]
fn typing_appends_chars_and_advances_cursor() {
    let mut c = Composer::new();
    type_str(&mut c, "hello");
    assert_eq!(c.text(), "hello");
    assert_eq!(c.cursor_col(), 5);
}

#[test]
fn backspace_removes_char_to_left() {
    let mut c = Composer::new();
    type_str(&mut c, "hello");
    let (k, m) = key(KeyCode::Backspace);
    assert_eq!(c.handle_key(k, m), ComposerAction::Consumed);
    assert_eq!(c.text(), "hell");
    assert_eq!(c.cursor_col(), 4);
}

#[test]
fn backspace_at_start_is_noop() {
    let mut c = Composer::new();
    let (k, m) = key(KeyCode::Backspace);
    assert_eq!(c.handle_key(k, m), ComposerAction::Consumed);
    assert_eq!(c.text(), "");
    assert_eq!(c.cursor_col(), 0);
}

#[test]
fn delete_removes_char_to_right() {
    let mut c = Composer::new();
    type_str(&mut c, "hello");
    c.handle_key(KeyCode::Home, KeyModifiers::NONE);
    let (k, m) = key(KeyCode::Delete);
    assert_eq!(c.handle_key(k, m), ComposerAction::Consumed);
    assert_eq!(c.text(), "ello");
    assert_eq!(c.cursor_col(), 0);
}

#[test]
fn arrows_and_home_end_move_cursor() {
    let mut c = Composer::new();
    type_str(&mut c, "hello");
    assert_eq!(c.cursor_col(), 5);
    c.handle_key(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(c.cursor_col(), 4);
    c.handle_key(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(c.cursor_col(), 0);
    c.handle_key(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(c.cursor_col(), 1);
    c.handle_key(KeyCode::End, KeyModifiers::NONE);
    assert_eq!(c.cursor_col(), 5);
}

#[test]
fn cursor_clamps_at_edges() {
    let mut c = Composer::new();
    type_str(&mut c, "ab");
    let (k, m) = key(KeyCode::Right);
    c.handle_key(k, m);
    c.handle_key(k, m);
    c.handle_key(k, m);
    assert_eq!(c.cursor_col(), 2);
    let (k, m) = key(KeyCode::Left);
    c.handle_key(k, m);
    c.handle_key(k, m);
    c.handle_key(k, m);
    assert_eq!(c.cursor_col(), 0);
}

#[test]
fn enter_on_non_empty_submits_and_clears() {
    let mut c = Composer::new();
    type_str(&mut c, "build me a feature");
    let (k, m) = key(KeyCode::Enter);
    match c.handle_key(k, m) {
        ComposerAction::Submit(text) => assert_eq!(text, "build me a feature"),
        other => panic!("expected Submit, got {other:?}"),
    }
    assert!(c.is_empty());
    assert_eq!(c.cursor_col(), 0);
}

#[test]
fn enter_on_empty_buffer_does_nothing() {
    let mut c = Composer::new();
    let (k, m) = key(KeyCode::Enter);
    assert_eq!(c.handle_key(k, m), ComposerAction::Consumed);
}

#[test]
fn esc_clears_non_empty_buffer() {
    let mut c = Composer::new();
    type_str(&mut c, "typing");
    let (k, m) = key(KeyCode::Esc);
    assert_eq!(c.handle_key(k, m), ComposerAction::Consumed);
    assert!(c.is_empty());
}

#[test]
fn esc_on_empty_buffer_passes_through() {
    let mut c = Composer::new();
    let (k, m) = key(KeyCode::Esc);
    assert_eq!(c.handle_key(k, m), ComposerAction::PassThrough);
}

#[test]
fn working_state_swallows_keys_except_esc() {
    let mut c = Composer::new();
    c.set_working(true);
    let (k, m) = key(KeyCode::Char('x'));
    assert_eq!(c.handle_key(k, m), ComposerAction::Consumed);
    assert!(c.is_empty());
    let (k, m) = key(KeyCode::Esc);
    assert_eq!(c.handle_key(k, m), ComposerAction::PassThrough);
}

#[test]
fn ctrl_keys_pass_through_for_the_app() {
    let mut c = Composer::new();
    let (k, m) = ctrl(KeyCode::Char('c'));
    assert_eq!(c.handle_key(k, m), ComposerAction::PassThrough);
    let (k, m) = ctrl(KeyCode::Char('e'));
    assert_eq!(c.handle_key(k, m), ComposerAction::PassThrough);
}

#[test]
fn type_then_edit_in_middle_then_submit() {
    let mut c = Composer::new();
    type_str(&mut c, "hello world");
    c.handle_key(KeyCode::Home, KeyModifiers::NONE);
    for _ in 0..6 {
        c.handle_key(KeyCode::Right, KeyModifiers::NONE);
    }
    type_str(&mut c, "cruel ");
    c.handle_key(KeyCode::End, KeyModifiers::NONE);
    let ComposerAction::Submit(text) = c.handle_key(KeyCode::Enter, KeyModifiers::NONE) else {
        panic!("expected Submit");
    };
    assert_eq!(text, "hello cruel world");
}

#[test]
fn footer_setters_update_state() {
    let mut c = Composer::new();
    c.set_model("opus-4-7");
    c.set_cwd("~/projects/foo");
    c.set_branch(Some("feature/bar".to_string()));
    assert_eq!(c.model, "opus-4-7");
    assert_eq!(c.cwd, "~/projects/foo");
    assert_eq!(c.branch.as_deref(), Some("feature/bar"));
}

// -----------------------------------------------------------------------------
// multi-line
// -----------------------------------------------------------------------------

#[test]
fn shift_enter_inserts_newline_without_submitting() {
    let mut c = Composer::new();
    type_str(&mut c, "line 1");
    let (k, m) = shift(KeyCode::Enter);
    assert_eq!(c.handle_key(k, m), ComposerAction::Consumed);
    type_str(&mut c, "line 2");
    assert_eq!(c.text(), "line 1\nline 2");
    assert_eq!(c.line_count(), 2);
    assert_eq!(c.cursor_line(), 1);
    assert_eq!(c.cursor_col(), 6);
}

#[test]
fn plain_enter_submits_multi_line_buffer() {
    let mut c = Composer::new();
    type_str(&mut c, "first");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "second");
    let ComposerAction::Submit(text) = c.handle_key(KeyCode::Enter, KeyModifiers::NONE) else {
        panic!("expected Submit");
    };
    assert_eq!(text, "first\nsecond");
}

#[test]
fn up_down_navigate_multi_line() {
    let mut c = Composer::new();
    type_str(&mut c, "first");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "second");
    // cursor at (1, 6)
    c.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(c.cursor_line(), 0);
    assert_eq!(c.cursor_col(), 5);
    c.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(c.cursor_line(), 1);
}

#[test]
fn up_at_first_line_passes_through() {
    let mut c = Composer::new();
    type_str(&mut c, "only");
    assert_eq!(
        c.handle_key(KeyCode::Up, KeyModifiers::NONE),
        ComposerAction::PassThrough,
    );
}

#[test]
fn down_at_last_line_passes_through() {
    let mut c = Composer::new();
    type_str(&mut c, "only");
    assert_eq!(
        c.handle_key(KeyCode::Down, KeyModifiers::NONE),
        ComposerAction::PassThrough,
    );
}

#[test]
fn backspace_at_line_start_joins_with_previous() {
    let mut c = Composer::new();
    type_str(&mut c, "ab");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "cd");
    // cursor at (1, 2). Go to start of line 2.
    c.handle_key(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(c.cursor_line(), 1);
    assert_eq!(c.cursor_col(), 0);
    c.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(c.text(), "abcd");
    assert_eq!(c.line_count(), 1);
    assert_eq!(c.cursor_line(), 0);
    assert_eq!(c.cursor_col(), 2);
}

#[test]
fn long_buffer_scrolls_field_window() {
    let mut c = Composer::new();
    for i in 0..(MAX_VISIBLE_LINES + 5) {
        type_str(&mut c, &format!("L{i}"));
        c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    }
    // Last line is empty (after final shift+enter); cursor on it.
    assert!(c.line_count() > MAX_VISIBLE_LINES);
    // Cursor must be visible in the scroll window.
    let st = c.scroll_top();
    assert!(c.cursor_line() >= st);
    assert!(c.cursor_line() < st + MAX_VISIBLE_LINES);
    // visible_line_count is capped.
    assert_eq!(c.visible_line_count(), MAX_VISIBLE_LINES);
}

// -----------------------------------------------------------------------------
// slash menu
// -----------------------------------------------------------------------------

#[test]
fn slash_opens_menu_with_all_commands() {
    let mut c = Composer::new();
    type_str(&mut c, "/");
    assert_eq!(c.menu_kind(), Some(MenuKind::Slash));
    assert!(c.menu_open());
    assert_eq!(c.menu_items().len(), 6);
}

#[test]
fn slash_query_filters_menu() {
    let mut c = Composer::new();
    type_str(&mut c, "/cle");
    let items = c.menu_items();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].value, "/clear");
}

#[test]
fn arrow_keys_move_menu_selection() {
    let mut c = Composer::new();
    type_str(&mut c, "/");
    assert_eq!(c.menu_selected(), 0);
    c.handle_key(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(c.menu_selected(), 1);
    c.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(c.menu_selected(), 0);
    c.handle_key(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(c.menu_selected(), 5, "should wrap to last item");
}

#[test]
fn enter_accepts_selected_slash_command() {
    let mut c = Composer::new();
    type_str(&mut c, "/");
    // First item is /clear
    let ComposerAction::Consumed = c.handle_key(KeyCode::Enter, KeyModifiers::NONE) else {
        panic!("expected Consumed");
    };
    assert_eq!(c.text(), "/clear ");
    assert_eq!(c.cursor_col(), 7);
}

#[test]
fn tab_also_accepts_selected_command() {
    let mut c = Composer::new();
    type_str(&mut c, "/com");
    let _ = c.handle_key(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(c.text(), "/compact ");
}

#[test]
fn esc_dismisses_slash_menu_by_clearing_buffer() {
    let mut c = Composer::new();
    type_str(&mut c, "/cle");
    assert!(c.menu_open());
    c.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(c.is_empty());
    assert!(!c.menu_open());
}

#[test]
fn menu_does_not_open_on_multi_line_buffer() {
    let mut c = Composer::new();
    type_str(&mut c, "first line");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "/clear");
    assert!(!c.menu_open(), "slash should not trigger on continuation line");
}

// -----------------------------------------------------------------------------
// @file menu
// -----------------------------------------------------------------------------

fn composer_with_files() -> Composer {
    let mut c = Composer::new();
    c.set_file_completions(vec![
        "src/main.rs".into(),
        "src/lib.rs".into(),
        "Cargo.toml".into(),
        "README.md".into(),
    ]);
    c
}

#[test]
fn at_token_opens_file_menu() {
    let mut c = composer_with_files();
    type_str(&mut c, "look at @");
    assert_eq!(c.menu_kind(), Some(MenuKind::At));
    assert!(c.menu_open());
    assert_eq!(c.menu_items().len(), 4);
}

#[test]
fn at_query_filters_files_by_substring() {
    let mut c = composer_with_files();
    type_str(&mut c, "open @src");
    let items = c.menu_items();
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.value == "src/main.rs"));
    assert!(items.iter().any(|i| i.value == "src/lib.rs"));
}

#[test]
fn enter_inserts_selected_filename_with_trailing_space() {
    let mut c = composer_with_files();
    type_str(&mut c, "look at @car");
    let _ = c.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(c.text(), "look at @Cargo.toml ");
    assert_eq!(c.cursor_col(), c.text().chars().count());
}

#[test]
fn at_menu_does_not_open_without_completions_pool() {
    let mut c = Composer::new();
    type_str(&mut c, "look at @anything");
    assert!(!c.menu_open(), "no completions configured ⇒ menu suppressed");
}

#[test]
fn at_menu_closes_after_space_after_token() {
    let mut c = composer_with_files();
    type_str(&mut c, "open @src ");
    assert!(!c.menu_open(), "menu should close once cursor is past the token");
}

#[test]
fn esc_in_at_menu_strips_just_the_token() {
    let mut c = composer_with_files();
    type_str(&mut c, "open @src");
    c.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(c.text(), "open ");
    assert!(!c.menu_open());
}
