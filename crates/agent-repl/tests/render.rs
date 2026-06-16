//! Smoke tests: render every event type to a `TestBackend` buffer across
//! all four vibes and both modes, asserting the rendered text contains the
//! expected content. This validates the whole event → Lines → Paragraph
//! pipeline without requiring a real terminal.

use agent_repl::{
    blocks, gallery, stream::Stream, AlertLevel, Decorations, Density, DiffKind, DiffLine,
    EntryType, Event, ListEntry, Mode, ReadLine, SearchGroup, SearchHit, SearchResult, Theme,
    TodoItem, TodoState, ToolCall, ToolKind, ToolStyle, Vibe,
};
use ratatui::backend::TestBackend;
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Terminal;

fn sample_events() -> Vec<Event> {
    vec![
        Event::user("Refactor `auth` middleware to use **verifyToken**."),
        Event::assistant("On it."),
        Event::Reasoning {
            text: "Walk the call sites first.".into(),
            ms: Some(800),
            default_open: true,
        },
        Event::status("Running test suite\u{2026}"),
        Event::Alert {
            level: AlertLevel::Warning,
            title: "Deprecation".into(),
            detail: Some("`jwt.decode()` does not verify signatures".into()),
        },
        Event::Tool(ToolCall::new(
            "verifyToken",
            ToolKind::Search {
                result: SearchResult {
                    count: 1,
                    groups: vec![SearchGroup {
                        file: "src/auth/token.ts".into(),
                        hits: vec![SearchHit {
                            line: 22,
                            text: "export async function verifyToken(raw?: string)".into(),
                        }],
                    }],
                },
            },
        )),
        Event::Tool(ToolCall::new(
            "src/auth/token.ts",
            ToolKind::Read {
                path: "src/auth/token.ts".into(),
                lines: 27,
                preview: vec![ReadLine { n: 22, text: "export async function verifyToken(...)".into() }],
            },
        )),
        Event::Tool(ToolCall::new(
            "src/middleware",
            ToolKind::List {
                entries: vec![
                    ListEntry { name: "auth.ts".into(), entry_type: EntryType::File, meta: Some("0.9 KB".into()) },
                    ListEntry { name: "__tests__".into(), entry_type: EntryType::Dir, meta: Some("4 files".into()) },
                ],
            },
        )),
        Event::Tool(ToolCall::new(
            "src/middleware/auth.ts",
            ToolKind::Edit {
                diff: vec![
                    DiffLine { kind: DiffKind::Del, a: Some(8), b: None, text: "  const t = req.headers.x".into() },
                    DiffLine { kind: DiffKind::Add, a: None, b: Some(8), text: "  const user = await verifyToken(req.headers.authorization)".into() },
                    DiffLine { kind: DiffKind::Ctx, a: Some(10), b: Some(10), text: "  req.user = user".into() },
                ],
            },
        )),
        Event::Tool(ToolCall::new(
            "pnpm test auth",
            ToolKind::Bash {
                cmd: "pnpm test auth".into(),
                output: "Tests: 2 passed, 2 total".into(),
                exit: Some(0),
            },
        )),
        Event::Tool(ToolCall::new(
            "plan",
            ToolKind::Todo {
                items: vec![
                    TodoItem { state: TodoState::Done, text: "Read current middleware".into() },
                    TodoItem { state: TodoState::Active, text: "Swap in verifyToken".into() },
                    TodoItem { state: TodoState::Pending, text: "Update tests".into() },
                ],
            },
        )),
    ]
}

fn render_with(theme: Theme, events: Vec<Event>) -> String {
    let mut stream = Stream::default();
    for ev in events {
        stream.push(ev);
    }
    let text = stream.render(&theme, &Decorations::default(), '\u{280B}');
    let backend = TestBackend::new(120, 80);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let p = Paragraph::new(text).wrap(Wrap { trim: false });
            f.render_widget(p, f.area());
        })
        .unwrap();
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

#[test]
fn renders_every_event_across_every_vibe_and_mode() {
    let events = sample_events();
    for &v in &[Vibe::Phosphor, Vibe::Slate, Vibe::Spectrum, Vibe::Ember] {
        for &m in &[Mode::Dark, Mode::Light] {
            for &s in &[ToolStyle::Inline, ToolStyle::Card, ToolStyle::Collapsed] {
                for &d in &[Density::Comfortable, Density::Compact] {
                    let theme = Theme::new(v).with_mode(m).with_tool_style(s).with_density(d);
                    let out = render_with(theme, events.clone());
                    // Spot-check: every render must contain key role labels and
                    // tool names so we know the dispatcher worked.
                    assert!(out.contains("user"), "{v:?}/{m:?}/{s:?}/{d:?}");
                    assert!(out.contains("assistant"), "{v:?}/{m:?}/{s:?}/{d:?}");
                    assert!(out.contains("search"), "{v:?}/{m:?}/{s:?}/{d:?}");
                    assert!(out.contains("read"), "{v:?}/{m:?}/{s:?}/{d:?}");
                    assert!(out.contains("bash"), "{v:?}/{m:?}/{s:?}/{d:?}");
                    assert!(out.contains("edit"), "{v:?}/{m:?}/{s:?}/{d:?}");
                    assert!(out.contains("todo"), "{v:?}/{m:?}/{s:?}/{d:?}");
                }
            }
        }
    }
}

#[test]
fn collapsed_hides_tool_body_until_running() {
    let theme = Theme::slate().collapsed();
    let events = vec![Event::Tool(ToolCall::new(
        "pnpm test auth",
        ToolKind::Bash {
            cmd: "pnpm test auth".into(),
            output: "should-not-show-when-collapsed".into(),
            exit: Some(0),
        },
    ))];
    let out = render_with(theme, events);
    assert!(out.contains("bash"));
    // Body suppressed in collapsed style:
    assert!(!out.contains("should-not-show-when-collapsed"));
}

#[test]
fn inline_style_includes_hue_gutter() {
    let theme = Theme::slate().inline();
    let events = vec![Event::Tool(ToolCall::new(
        "pnpm test",
        ToolKind::Bash {
            cmd: "pnpm test".into(),
            output: "ok".into(),
            exit: Some(0),
        },
    ))];
    let out = render_with(theme, events);
    // Gutter glyph from `frame_inline`:
    assert!(out.contains('│'), "expected gutter glyph in inline render");
}

#[test]
fn card_style_includes_corner_glyphs() {
    let theme = Theme::slate().card();
    let events = vec![Event::Tool(ToolCall::new(
        "pnpm test",
        ToolKind::Bash {
            cmd: "pnpm test".into(),
            output: "ok".into(),
            exit: Some(0),
        },
    ))];
    let out = render_with(theme, events);
    assert!(out.contains('╭'), "expected top corner");
    assert!(out.contains('╰'), "expected bottom corner");
}

#[test]
fn blocks_render_is_pure_and_does_not_panic() {
    // Exhaustive shape check for every event variant against `blocks::render`.
    let theme = Theme::default();
    for ev in sample_events() {
        let lines = blocks::render(&ev, &theme, &Decorations::default(), '\u{280B}', true, false);
        assert!(!lines.is_empty(), "no lines for {ev:?}");
    }
}

// -----------------------------------------------------------------------------
// Slice 3: focus + expand + Gallery
// -----------------------------------------------------------------------------

fn stream_with_one_of_each() -> Stream {
    let mut s = Stream::default();
    s.push(Event::user("hi"));
    s.push(Event::assistant("hello"));
    s.push(Event::Tool(ToolCall::new(
        "first",
        ToolKind::Bash { cmd: "echo a".into(), output: "a".into(), exit: Some(0) },
    )));
    s.push(Event::user("again"));
    s.push(Event::Tool(ToolCall::new(
        "second",
        ToolKind::Bash { cmd: "echo b".into(), output: "b".into(), exit: Some(0) },
    )));
    s
}

#[test]
fn focus_next_cycles_through_tools_skipping_other_events() {
    let mut s = stream_with_one_of_each();
    assert_eq!(s.focused_idx(), None);
    s.focus_next();
    assert_eq!(s.focused_idx(), Some(2));
    s.focus_next();
    assert_eq!(s.focused_idx(), Some(4));
    s.focus_next();
    assert_eq!(s.focused_idx(), Some(2), "should wrap");
}

#[test]
fn focus_prev_cycles_backwards() {
    let mut s = stream_with_one_of_each();
    s.focus_prev();
    assert_eq!(s.focused_idx(), Some(4));
    s.focus_prev();
    assert_eq!(s.focused_idx(), Some(2));
    s.focus_prev();
    assert_eq!(s.focused_idx(), Some(4), "should wrap");
}

#[test]
fn focus_next_no_tools_is_noop() {
    let mut s = Stream::default();
    s.push(Event::user("only"));
    s.focus_next();
    assert_eq!(s.focused_idx(), None);
}

#[test]
fn collapsed_finished_tool_hides_body_by_default() {
    let theme = Theme::slate().collapsed();
    let mut s = Stream::default();
    s.push(Event::Tool(ToolCall::new(
        "pnpm test",
        ToolKind::Bash {
            cmd: "pnpm test".into(),
            output: "BODY-SECRET-MARKER".into(),
            exit: Some(0),
        },
    )));
    let out = render_stream(&s, &theme);
    assert!(!out.contains("BODY-SECRET-MARKER"));
}

#[test]
fn collapsed_running_tool_keeps_body_open() {
    let theme = Theme::slate().collapsed();
    let mut s = Stream::default();
    let mut call = ToolCall::new(
        "long",
        ToolKind::Bash {
            cmd: "sleep 1".into(),
            output: "".into(),
            exit: None,
        },
    );
    call.running = true;
    call.run_label = Some("RUNNING-MARKER".into());
    s.push(Event::Tool(call));
    let out = render_stream(&s, &theme);
    assert!(out.contains("RUNNING-MARKER"));
}

#[test]
fn collapsed_user_expansion_reveals_body() {
    let theme = Theme::slate().collapsed();
    let mut s = Stream::default();
    s.push(Event::Tool(ToolCall::new(
        "pnpm test",
        ToolKind::Bash {
            cmd: "pnpm test".into(),
            output: "EXPANDED-MARKER".into(),
            exit: Some(0),
        },
    )));
    s.focus_next();
    assert_eq!(s.focused_idx(), Some(0));
    s.toggle_focused_expansion();
    let out = render_stream(&s, &theme);
    assert!(out.contains("EXPANDED-MARKER"));
    s.toggle_focused_expansion();
    let out = render_stream(&s, &theme);
    assert!(!out.contains("EXPANDED-MARKER"));
}

#[test]
fn card_style_ignores_expansion_state() {
    let theme = Theme::slate().card();
    let mut s = Stream::default();
    s.push(Event::Tool(ToolCall::new(
        "pnpm test",
        ToolKind::Bash {
            cmd: "pnpm test".into(),
            output: "CARD-BODY-MARKER".into(),
            exit: Some(0),
        },
    )));
    // Body visible without any expansion toggling — card always shows body.
    let out = render_stream(&s, &theme);
    assert!(out.contains("CARD-BODY-MARKER"));
}

#[test]
fn update_tool_preserves_user_expansion() {
    use agent_repl::stream::ToolId;
    let theme = Theme::slate().collapsed();
    let mut s = Stream::default();
    let id = ToolId(1);
    let mut running = ToolCall::new(
        "build",
        ToolKind::Bash { cmd: "cargo build".into(), output: String::new(), exit: None },
    );
    running.running = true;
    s.push_tool(id, running);
    s.focus_next();
    s.toggle_focused_expansion(); // user pins open

    // Tool finishes — payload updated, but expansion choice should survive.
    let done = ToolCall::new(
        "build",
        ToolKind::Bash {
            cmd: "cargo build".into(),
            output: "FINAL-OUTPUT-MARKER".into(),
            exit: Some(0),
        },
    );
    s.update_tool(id, done);
    let out = render_stream(&s, &theme);
    assert!(out.contains("FINAL-OUTPUT-MARKER"));
}

#[test]
fn gallery_includes_one_of_each_block_kind() {
    // Use a tall buffer so the whole gallery renders without truncation.
    let s = gallery::build();
    let theme = Theme::slate().card();
    let out = render_stream_with_size(&s, &theme, 160, 200);
    for needle in [
        "user", "assistant", "search", "read", "list",
        "edit", "bash", "todo", "exit 0", "Command failed", "Deprecation", "Running",
    ] {
        assert!(out.contains(needle), "gallery output missing {needle:?}");
    }
}

#[test]
fn user_block_gets_gutter_in_inline_style() {
    let theme = Theme::slate().inline();
    let mut s = Stream::default();
    s.push(Event::user("hello"));
    let out = render_stream(&s, &theme);
    assert!(out.contains('│'), "expected inline gutter on user block");
    assert!(out.contains("user"), "user header missing");
    assert!(out.contains("hello"), "user body missing");
}

#[test]
fn assistant_block_gets_card_corners_in_card_style() {
    let theme = Theme::slate().card();
    let mut s = Stream::default();
    s.push(Event::assistant("Sure thing"));
    let out = render_stream(&s, &theme);
    assert!(out.contains('╭'), "expected card top corner on assistant");
    assert!(out.contains('╰'), "expected card bottom corner on assistant");
    assert!(out.contains("assistant"));
    assert!(out.contains("Sure thing"));
}

#[test]
fn error_alert_gets_framing_in_card_style() {
    let theme = Theme::slate().card();
    let mut s = Stream::default();
    s.push(Event::Alert {
        level: AlertLevel::Error,
        title: "Boom".into(),
        detail: Some("things went sideways".into()),
    });
    let out = render_stream(&s, &theme);
    assert!(out.contains('╭'), "expected card top corner on error");
    assert!(out.contains("Boom"));
    assert!(out.contains("sideways"));
}

#[test]
fn warning_alert_gets_framing_in_inline_style() {
    let theme = Theme::slate().inline();
    let mut s = Stream::default();
    s.push(Event::Alert {
        level: AlertLevel::Warning,
        title: "Heads up".into(),
        detail: Some("watch out".into()),
    });
    let out = render_stream(&s, &theme);
    assert!(out.contains('│'), "expected inline gutter on warning");
    assert!(out.contains("Heads up"));
    assert!(out.contains("watch out"));
}

#[test]
fn assistant_prose_stays_visible_in_collapsed_style() {
    // Collapsed style hides tool bodies but must never hide assistant
    // prose (it's the main content). msg.rs marks it `collapsible: false`,
    // which frame::apply downgrades to inline.
    let theme = Theme::slate().collapsed();
    let mut s = Stream::default();
    s.push(Event::assistant("DO-NOT-HIDE-ME"));
    let out = render_stream(&s, &theme);
    assert!(out.contains("DO-NOT-HIDE-ME"));
}

#[test]
fn assistant_sigil_appears_before_label_when_set() {
    let theme = Theme::slate().card();
    let deco = Decorations::default().assistant_sigil("🤖");
    let mut s = Stream::default();
    s.push(Event::assistant("hi"));
    let out = render_stream_with(&s, &theme, &deco, 80, 20);
    // The sigil must appear, and on the same row as the label.
    for line in out.lines() {
        if line.contains("assistant") {
            assert!(
                line.contains("🤖"),
                "expected sigil on assistant header line: {line:?}",
            );
            return;
        }
    }
    panic!("no assistant header rendered");
}

#[test]
fn user_sigil_appears_before_label_when_set() {
    let theme = Theme::slate().card();
    let deco = Decorations::default().user_sigil("●");
    let mut s = Stream::default();
    s.push(Event::user("hello"));
    let out = render_stream_with(&s, &theme, &deco, 80, 20);
    for line in out.lines() {
        if line.contains("user") {
            assert!(
                line.contains('●'),
                "expected sigil on user header line: {line:?}",
            );
            return;
        }
    }
    panic!("no user header rendered");
}

#[test]
fn assistant_has_no_sigil_by_default() {
    // Regression guard: default Decorations leaves the header bare so we
    // don't surprise existing users with a new glyph.
    let theme = Theme::slate().card();
    let mut s = Stream::default();
    s.push(Event::assistant("hi"));
    let out = render_stream(&s, &theme);
    for line in out.lines() {
        if line.contains("assistant") {
            assert!(
                !line.contains('●') && !line.contains("🤖"),
                "expected no sigil on default assistant header line: {line:?}",
            );
            return;
        }
    }
    panic!("no assistant header rendered");
}

#[test]
fn sigil_works_across_every_tool_style() {
    let deco = Decorations::default()
        .user_sigil("●")
        .assistant_sigil("🤖");
    for &style in &[ToolStyle::Inline, ToolStyle::Card, ToolStyle::Collapsed] {
        let theme = Theme::slate().with_tool_style(style);
        let mut s = Stream::default();
        s.push(Event::user("hi"));
        s.push(Event::assistant("hey"));
        let out = render_stream_with(&s, &theme, &deco, 80, 20);
        assert!(out.contains('●'), "user sigil missing under {style:?}");
        assert!(out.contains("🤖"), "assistant sigil missing under {style:?}");
    }
}

#[test]
fn every_block_has_left_outer_padding() {
    // After Stream::render adds 2 cols of left pad, no rendered content
    // (including framing glyphs) should appear in columns 0–1.
    let theme = Theme::slate().card();
    let mut s = Stream::default();
    s.push(Event::user("hello"));
    s.push(Event::assistant("hi"));
    s.push(Event::Tool(ToolCall::new(
        "echo",
        ToolKind::Bash { cmd: "echo".into(), output: "ok".into(), exit: Some(0) },
    )));
    let text = s.render(&theme, &Decorations::default(), '\u{280B}');
    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let p = Paragraph::new(text).wrap(Wrap { trim: false });
            f.render_widget(p, f.area());
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    for y in 0..buf.area.height {
        for x in 0..2 {
            let sym = buf[(x, y)].symbol();
            assert!(
                sym.trim().is_empty(),
                "expected blank at col {x},{y} but found {sym:?}",
            );
        }
    }
}

#[test]
fn render_handles_a_range_of_terminal_sizes_without_panic() {
    let s = gallery::build();
    let theme = Theme::slate().card();
    for &(w, h) in &[(40_u16, 20_u16), (80, 24), (120, 40), (200, 60), (24, 10)] {
        let text = s.render(&theme, &Decorations::default(), '\u{280B}');
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let p = Paragraph::new(text.clone()).wrap(Wrap { trim: false });
                f.render_widget(p, f.area());
            })
            .unwrap();
    }
}

fn render_stream(stream: &Stream, theme: &Theme) -> String {
    render_stream_with(stream, theme, &Decorations::default(), 120, 60)
}

fn render_stream_with_size(stream: &Stream, theme: &Theme, w: u16, h: u16) -> String {
    render_stream_with(stream, theme, &Decorations::default(), w, h)
}

fn render_stream_with(
    stream: &Stream,
    theme: &Theme,
    deco: &Decorations,
    w: u16,
    h: u16,
) -> String {
    let text = stream.render(theme, deco, '\u{280B}');
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let p = Paragraph::new(text).wrap(Wrap { trim: false });
            f.render_widget(p, f.area());
        })
        .unwrap();
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
