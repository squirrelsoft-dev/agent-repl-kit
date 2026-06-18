//! Tests for the tabbed question box: the interaction state machine
//! (`QuestionState`) and its rendering to a `TestBackend` buffer.

use agent_repl::{
    question::QuestionState, Mode, Question, QuestionAction, QuestionAnswer, QuestionForm, Theme,
    Vibe,
};
use crossterm::event::KeyCode;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render(qs: &QuestionState, theme: &Theme) -> String {
    let backend = TestBackend::new(80, qs.required_height().max(1));
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| qs.render(theme, f, f.area())).unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn keys(qs: &mut QuestionState, codes: &[KeyCode]) -> QuestionAction {
    let mut last = QuestionAction::Continue;
    for &c in codes {
        last = qs.handle_key(c);
    }
    last
}

fn single_db() -> Question {
    Question::single(
        "Which database?",
        vec!["Postgres".into(), "MySQL".into(), "SQLite".into()],
    )
}

fn multi_feat() -> Question {
    Question::multi(
        "Which features?",
        vec!["Auth".into(), "Billing".into(), "Search".into()],
    )
}

// ---- rendering --------------------------------------------------------------

#[test]
fn single_with_other_renders_options_and_freeform() {
    let qs = QuestionState::new(QuestionForm::one(single_db().with_other()));
    let out = render(&qs, &Theme::slate().dark().card());
    assert!(out.contains("Which database?"), "{out}");
    assert!(out.contains("Postgres") && out.contains("MySQL") && out.contains("SQLite"), "{out}");
    assert!(out.contains("Other"), "missing Other row:\n{out}");
    // Single-select uses ○ / ◉ glyphs.
    assert!(out.contains('\u{25CB}'), "missing single-select glyph:\n{out}");
}

#[test]
fn multi_with_custom_renders_checkboxes() {
    let qs = QuestionState::new(QuestionForm::one(multi_feat().with_freeform("Something else")));
    let out = render(&qs, &Theme::ember().dark().card());
    assert!(out.contains("Auth") && out.contains("Billing") && out.contains("Search"), "{out}");
    assert!(out.contains("Something else"), "missing custom row:\n{out}");
    // Multi-select uses ☐ / ☑ glyphs.
    assert!(out.contains('\u{2610}'), "missing checkbox glyph:\n{out}");
}

#[test]
fn single_question_has_no_tab_bar_or_submit_tab() {
    let qs = QuestionState::new(QuestionForm::one(single_db()));
    let out = render(&qs, &Theme::slate().dark().card());
    assert!(!out.contains("Submit"), "single question should not show a Submit tab:\n{out}");
}

#[test]
fn multiple_questions_show_tab_bar_and_submit_tab() {
    let form = QuestionForm::new(vec![single_db().with_other(), multi_feat()]);
    let qs = QuestionState::new(form);
    let out = render(&qs, &Theme::slate().dark().card());
    assert!(out.contains("Submit"), "expected a Submit tab:\n{out}");
}

#[test]
fn submit_tab_reviews_every_answer() {
    let form = QuestionForm::new(vec![single_db().with_other(), multi_feat()]);
    let mut qs = QuestionState::new(form);
    // Q1: pick MySQL (Down once → cursor on MySQL), Enter advances to Q2.
    keys(&mut qs, &[KeyCode::Down, KeyCode::Enter]);
    // Q2: toggle Auth, then Tab to the Submit tab.
    keys(&mut qs, &[KeyCode::Char(' '), KeyCode::Tab]);
    let out = render(&qs, &Theme::slate().dark().card());
    assert!(out.contains("Review your answers"), "{out}");
    assert!(out.contains("MySQL"), "submit review missing Q1 answer:\n{out}");
    assert!(out.contains("Auth"), "submit review missing Q2 answer:\n{out}");
    assert!(out.contains("Press"), "submit review missing submit prompt:\n{out}");
}

#[test]
fn renders_across_every_vibe_and_mode_without_panic() {
    let form = QuestionForm::new(vec![
        single_db().with_other().with_detail("primary store"),
        multi_feat().with_freeform("other"),
    ]);
    for &v in &[Vibe::Phosphor, Vibe::Slate, Vibe::Spectrum, Vibe::Ember] {
        for &m in &[Mode::Dark, Mode::Light] {
            let theme = Theme::new(v).with_mode(m).card();
            let mut qs = QuestionState::new(form.clone());
            // walk all three tabs
            for _ in 0..3 {
                let out = render(&qs, &theme);
                assert!(out.contains("Which"), "{v:?}/{m:?}:\n{out}");
                qs.handle_key(KeyCode::Tab);
            }
        }
    }
}

// ---- state machine ----------------------------------------------------------

#[test]
fn single_select_enter_submits_with_chosen_index() {
    let mut qs = QuestionState::new(QuestionForm::one(single_db()));
    // Down → MySQL (index 1), Enter on the only/last tab submits.
    let action = keys(&mut qs, &[KeyCode::Down, KeyCode::Enter]);
    match action {
        QuestionAction::Submit(answers) => {
            assert_eq!(
                answers.answers,
                vec![QuestionAnswer::Single { option: Some(1), other: None }]
            );
        }
        other => panic!("expected submit, got {other:?}"),
    }
}

#[test]
fn single_select_other_captures_typed_text() {
    let mut qs = QuestionState::new(QuestionForm::one(single_db().with_other()));
    // Move to the Other row (index 3 = after 3 options), type, then submit.
    keys(&mut qs, &[KeyCode::Up]); // wrap up from row 0 → last row (Other)
    for c in "MongoDB".chars() {
        qs.handle_key(KeyCode::Char(c));
    }
    let action = qs.handle_key(KeyCode::Enter);
    match action {
        QuestionAction::Submit(answers) => {
            assert_eq!(
                answers.answers,
                vec![QuestionAnswer::Single { option: None, other: Some("MongoDB".into()) }]
            );
        }
        other => panic!("expected submit, got {other:?}"),
    }
}

#[test]
fn multi_select_toggles_collect_sorted_indices() {
    let mut qs = QuestionState::new(QuestionForm::one(multi_feat()));
    // Toggle Search (index 2) first, then Auth (index 0); result must be sorted.
    keys(
        &mut qs,
        &[
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Char(' '), // Search
            KeyCode::Up,
            KeyCode::Up,
            KeyCode::Char(' '), // Auth
        ],
    );
    let action = qs.handle_key(KeyCode::Enter); // last tab → submit
    match action {
        QuestionAction::Submit(answers) => {
            assert_eq!(
                answers.answers,
                vec![QuestionAnswer::Multi { options: vec![0, 2], custom: None }]
            );
        }
        other => panic!("expected submit, got {other:?}"),
    }
}

#[test]
fn multi_select_space_toggles_off_again() {
    let mut qs = QuestionState::new(QuestionForm::one(multi_feat()));
    let action = keys(&mut qs, &[KeyCode::Char(' '), KeyCode::Char(' '), KeyCode::Enter]);
    match action {
        QuestionAction::Submit(answers) => {
            assert_eq!(
                answers.answers,
                vec![QuestionAnswer::Multi { options: vec![], custom: None }]
            );
        }
        other => panic!("expected submit, got {other:?}"),
    }
}

#[test]
fn multi_custom_message_is_captured() {
    let mut qs = QuestionState::new(QuestionForm::one(multi_feat().with_freeform("Other")));
    // Toggle Auth, move to the custom row (index 3), type a message.
    keys(&mut qs, &[KeyCode::Char(' '), KeyCode::Up]); // Up wraps to the custom row
    for c in "GraphQL".chars() {
        qs.handle_key(KeyCode::Char(c));
    }
    let action = qs.handle_key(KeyCode::Enter);
    match action {
        QuestionAction::Submit(answers) => {
            assert_eq!(
                answers.answers,
                vec![QuestionAnswer::Multi { options: vec![0], custom: Some("GraphQL".into()) }]
            );
        }
        other => panic!("expected submit, got {other:?}"),
    }
}

#[test]
fn enter_on_question_tab_advances_instead_of_submitting() {
    let form = QuestionForm::new(vec![single_db(), single_db(), single_db()]);
    let mut qs = QuestionState::new(form);
    // Enter on the first question tab must NOT submit (a Submit tab follows).
    assert_eq!(qs.handle_key(KeyCode::Enter), QuestionAction::Continue);
    assert_eq!(qs.handle_key(KeyCode::Enter), QuestionAction::Continue);
    // Third question tab: still not the last tab (Submit is), so still advances.
    assert_eq!(qs.handle_key(KeyCode::Enter), QuestionAction::Continue);
    // Now on the Submit tab: Enter submits.
    assert!(matches!(qs.handle_key(KeyCode::Enter), QuestionAction::Submit(_)));
}

#[test]
fn tab_navigation_wraps_and_space_does_not_submit() {
    let form = QuestionForm::new(vec![multi_feat(), multi_feat()]);
    let mut qs = QuestionState::new(form);
    // Space on a multi tab toggles, never submits.
    assert_eq!(qs.handle_key(KeyCode::Char(' ')), QuestionAction::Continue);
    // Walk tabs forward past the end; should wrap, never submit on its own.
    for _ in 0..6 {
        assert_eq!(qs.handle_key(KeyCode::Tab), QuestionAction::Continue);
    }
}

#[test]
fn describe_renders_human_readable_answers() {
    let q = single_db();
    let ans = QuestionAnswer::Single { option: Some(2), other: None };
    assert_eq!(ans.describe(&q).as_deref(), Some("SQLite"));

    let qm = multi_feat();
    let ans = QuestionAnswer::Multi { options: vec![0, 2], custom: Some("CDN".into()) };
    assert_eq!(ans.describe(&qm).as_deref(), Some("Auth, Search, CDN"));

    let empty = QuestionAnswer::Single { option: None, other: None };
    assert!(empty.is_empty());
    assert_eq!(empty.describe(&q), None);
}
