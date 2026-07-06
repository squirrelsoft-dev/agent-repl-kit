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

// -----------------------------------------------------------------------------
// bracketed paste
// -----------------------------------------------------------------------------

#[test]
fn single_line_paste_inserts_inline() {
    let mut c = Composer::new();
    type_str(&mut c, "foo ");
    c.paste("bar baz".to_string());
    assert_eq!(c.text(), "foo bar baz");
    assert_eq!(c.line_count(), 1);
}

#[test]
fn multiline_paste_shows_placeholder_and_expands_on_submit() {
    let mut c = Composer::new();
    type_str(&mut c, "see ");
    c.paste("line one\nline two\nline three".to_string());
    // Buffer shows a compact one-line placeholder, NOT the raw pasted lines.
    assert_eq!(c.line_count(), 1);
    let buffer = c.lines().join("\n");
    assert!(buffer.contains("[Pasted text #1 +3 lines]"), "buffer: {buffer}");
    assert!(!buffer.contains("line two"));
    // On submit, `text()` expands the placeholder back to the full paste.
    assert_eq!(c.text(), "see line one\nline two\nline three");
}

#[test]
fn crlf_paste_is_normalized() {
    let mut c = Composer::new();
    c.paste("a\r\nb".to_string());
    assert_eq!(c.text(), "a\nb");
}

#[test]
fn multiple_pastes_each_expand_and_clear_resets() {
    let mut c = Composer::new();
    c.paste("x1\nx2".to_string());
    type_str(&mut c, " and ");
    c.paste("y1\ny2".to_string());
    let buffer = c.lines().join("\n");
    assert!(buffer.contains("[Pasted text #1 +2 lines]"));
    assert!(buffer.contains("[Pasted text #2 +2 lines]"));
    assert_eq!(c.text(), "x1\nx2 and y1\ny2");
    c.clear();
    assert!(c.is_empty());
    assert_eq!(c.text(), "");
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
fn working_state_allows_editing_and_enter_steers_the_running_turn() {
    let mut c = Composer::new();
    c.set_working(true);

    // Type-ahead: the draft stays editable while the agent is busy.
    assert_eq!(c.handle_key(KeyCode::Char('h'), KeyModifiers::NONE), ComposerAction::Consumed);
    assert_eq!(c.handle_key(KeyCode::Char('i'), KeyModifiers::NONE), ComposerAction::Consumed);
    assert!(!c.is_empty(), "draft accepts input while working");

    // App-level keys still pass through so scroll / theme stay live mid-run.
    // `Up` on the first line scrolls the transcript rather than the cursor.
    assert_eq!(c.handle_key(KeyCode::Up, KeyModifiers::NONE), ComposerAction::PassThrough);

    // Enter while working STEERS the running turn (pi-style) — the text is
    // handed to the host for mid-run injection and the buffer clears.
    let ComposerAction::Steer(text) = c.handle_key(KeyCode::Enter, KeyModifiers::NONE) else {
        panic!("expected Steer while working");
    };
    assert_eq!(text, "hi");
    assert!(c.is_empty(), "the steered text left the buffer");

    // Once the agent goes idle, Enter submits normally again.
    c.set_working(false);
    type_str(&mut c, "next");
    let ComposerAction::Submit(text) = c.handle_key(KeyCode::Enter, KeyModifiers::NONE) else {
        panic!("expected Submit once idle");
    };
    assert_eq!(text, "next");
}

#[test]
fn enter_on_empty_buffer_while_working_does_nothing() {
    let mut c = Composer::new();
    c.set_working(true);
    assert_eq!(c.handle_key(KeyCode::Enter, KeyModifiers::NONE), ComposerAction::Consumed);
}

#[test]
fn alt_enter_while_working_queues_a_follow_up() {
    let mut c = Composer::new();
    c.set_working(true);
    type_str(&mut c, "after this, run the tests");
    let ComposerAction::QueueFollowUp(text) =
        c.handle_key(KeyCode::Enter, KeyModifiers::ALT)
    else {
        panic!("expected QueueFollowUp while working");
    };
    assert_eq!(text, "after this, run the tests");
    assert!(c.is_empty(), "the queued text left the buffer");
}

#[test]
fn alt_enter_while_idle_is_a_plain_submit() {
    let mut c = Composer::new();
    type_str(&mut c, "hello");
    let ComposerAction::Submit(text) = c.handle_key(KeyCode::Enter, KeyModifiers::ALT) else {
        panic!("expected Submit while idle");
    };
    assert_eq!(text, "hello");
}

#[test]
fn shift_enter_still_inserts_a_newline_while_working() {
    let mut c = Composer::new();
    c.set_working(true);
    type_str(&mut c, "line1");
    assert_eq!(c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT), ComposerAction::Consumed);
    type_str(&mut c, "line2");
    assert_eq!(c.text(), "line1\nline2");
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
fn long_buffer_caps_visible_rows_at_max() {
    let mut c = Composer::new();
    for i in 0..(MAX_VISIBLE_LINES + 5) {
        type_str(&mut c, &format!("L{i}"));
        c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    }
    // Last line is empty (after final shift+enter); cursor on it.
    assert!(c.line_count() > MAX_VISIBLE_LINES);
    // At a wide width nothing wraps: one visual row per logical line.
    let layout = c.layout(80);
    assert_eq!(layout.rows.len(), c.line_count());
    // The visible window is capped, and the cursor lands on the final row.
    assert_eq!(c.visible_line_count(80), MAX_VISIBLE_LINES);
    assert_eq!(layout.cursor_row, c.line_count() - 1);
}

// -----------------------------------------------------------------------------
// soft-wrap
// -----------------------------------------------------------------------------

#[test]
fn long_line_wraps_into_multiple_visual_rows() {
    let mut c = Composer::new();
    type_str(&mut c, &"x".repeat(25));
    // 25 chars at 10 content columns → rows [0,10) [10,20) [20,25).
    let layout = c.layout(10);
    assert_eq!(layout.rows.len(), 3);
    assert_eq!((layout.rows[0].start, layout.rows[0].end), (0, 10));
    assert_eq!((layout.rows[1].start, layout.rows[1].end), (10, 20));
    assert_eq!((layout.rows[2].start, layout.rows[2].end), (20, 25));
    assert!(layout.rows.iter().all(|r| r.line == 0));
    // Cursor at the end (col 25) sits on the last row, 5 cols in.
    assert_eq!(layout.cursor_row, 2);
    assert_eq!(layout.cursor_col, 5);
}

#[test]
fn cursor_at_exact_wrap_boundary_gets_a_trailing_row() {
    let mut c = Composer::new();
    type_str(&mut c, &"x".repeat(20)); // exactly 2 * 10
    let layout = c.layout(10);
    // Two full content rows plus a fresh trailing row for the caret.
    assert_eq!(layout.rows.len(), 3);
    assert_eq!((layout.rows[2].start, layout.rows[2].end), (20, 20));
    assert_eq!(layout.cursor_row, 2);
    assert_eq!(layout.cursor_col, 0);
}

#[test]
fn visible_line_count_counts_wrapped_rows() {
    let mut c = Composer::new();
    type_str(&mut c, &"x".repeat(25));
    // Narrow: 25 chars / 10 cols → 3 visual rows.
    assert_eq!(c.visible_line_count(10), 3);
    // Wide: fits on one row.
    assert_eq!(c.visible_line_count(80), 1);
}

#[test]
fn wrap_maps_cursor_within_a_long_line() {
    let mut c = Composer::new();
    type_str(&mut c, &"x".repeat(25));
    // Walk the cursor back to char 12 (second wrapped row, 2 cols in).
    for _ in 0..13 {
        c.handle_key(KeyCode::Left, KeyModifiers::NONE);
    }
    assert_eq!(c.cursor_col(), 12);
    let layout = c.layout(10);
    assert_eq!(layout.cursor_row, 1);
    assert_eq!(layout.cursor_col, 2);
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
fn set_slash_commands_overrides_the_menu_catalog() {
    let mut c = Composer::new();
    // A host app advertises ITS own commands instead of the built-in demo set.
    c.set_slash_commands(vec![
        ("/mode".into(), "Switch the active mode".into()),
        ("/verify".into(), "Toggle the verified executor".into()),
        ("/new".into(), "Start a new session".into()),
    ]);
    type_str(&mut c, "/");
    let all: Vec<String> = c.menu_items().into_iter().map(|i| i.value).collect();
    assert_eq!(all, vec!["/mode", "/verify", "/new"]);
    // Filtering applies to the custom catalog, not the built-in one.
    type_str(&mut c, "mo");
    let filtered = c.menu_items();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].value, "/mode");
    assert_eq!(filtered[0].description, "Switch the active mode");
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

// -----------------------------------------------------------------------------
// input sizing (min height + reserved right strip for a mascot)
// -----------------------------------------------------------------------------

#[test]
fn min_visible_lines_floors_the_field_height() {
    let mut c = Composer::default();
    assert_eq!(c.visible_line_count(80), 1, "default floor is one row");
    c.set_min_visible_lines(4);
    // An empty buffer still reserves the floor...
    assert_eq!(c.visible_line_count(80), 4);
    // ...and the field still grows with content.
    type_str(&mut c, "one");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "two");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "three");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "four");
    type_str(&mut c, "");
    c.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    type_str(&mut c, "five");
    assert_eq!(c.visible_line_count(80), 5, "grows past the floor with content");
}

#[test]
fn min_visible_lines_is_capped_at_max() {
    let mut c = Composer::default();
    c.set_min_visible_lines(1000);
    assert_eq!(c.visible_line_count(80), MAX_VISIBLE_LINES);
}

#[test]
fn reserved_right_round_trips() {
    let mut c = Composer::default();
    assert_eq!(c.reserved_right(), 0);
    c.set_reserved_right(9);
    assert_eq!(c.reserved_right(), 9);
}

// -----------------------------------------------------------------------------
// @file fuzzy completion
// -----------------------------------------------------------------------------

#[test]
fn at_menu_opens_only_when_file_pool_is_populated() {
    let mut c = Composer::new();
    type_str(&mut c, "see @ma");
    assert_eq!(c.menu_kind(), None, "no @ menu without a file pool");
    c.set_file_completions(vec!["src/main.rs".into()]);
    assert_eq!(c.menu_kind(), Some(MenuKind::At));
}

#[test]
fn at_menu_fuzzy_matches_subsequence_and_ranks_best_first() {
    let mut c = Composer::new();
    c.set_file_completions(vec![
        "src/main.rs".into(),
        "src/composer/state.rs".into(),
        "To do list.png".into(),
        "README.md".into(),
    ]);
    type_str(&mut c, "look at @tdl");
    let items = c.menu_items();
    // "tdl" is a subsequence of "to do list.png" only.
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].value, "To do list.png");
}

#[test]
fn at_menu_accept_quotes_a_spaced_filename() {
    let mut c = Composer::new();
    c.set_file_completions(vec!["To do list.png".into()]);
    type_str(&mut c, "see @tdl");
    // Tab accepts the top fuzzy match.
    assert_eq!(c.handle_key(KeyCode::Tab, KeyModifiers::NONE), ComposerAction::Consumed);
    assert_eq!(c.lines()[0], "see @\"To do list.png\" ");
}
