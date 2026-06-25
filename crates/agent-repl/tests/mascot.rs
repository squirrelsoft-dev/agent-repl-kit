//! Integration test: the composer reserves a right strip for an attached
//! mascot, renders the mascot there, and confines typed text to the left so it
//! never overdraws the creature. Exercised with the built-in `BallMascot`.

use std::time::Duration;

use agent_repl::composer::render::{self as crender, MascotPaint};
use agent_repl::composer::Composer;
use agent_repl::mascot::{BallMascot, Mascot, MascotState};
use agent_repl::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

const W: u16 = 56;
/// The orb's top arc `╭` — a stable marker of the mascot's left edge.
const ARC: char = '\u{256D}';

fn render_rows(composer: &Composer, mascot_state: MascotState) -> Vec<String> {
    let theme = Theme::slate().dark().card();
    let height = crender::required_height(composer, &theme, W);
    let mascot = BallMascot;
    let backend = TestBackend::new(W, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = Rect { x: 0, y: 0, width: W, height };
            crender::render(
                composer,
                &theme,
                f,
                area,
                Some(MascotPaint {
                    mascot: &mascot,
                    state: mascot_state,
                    elapsed: Duration::from_millis(600),
                }),
            );
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
        .collect()
}

fn typed(s: &str) -> Composer {
    let mut c = Composer::default();
    let (w, h) = BallMascot.size();
    c.set_min_visible_lines(h as usize);
    c.set_reserved_right(w + 2);
    for ch in s.chars() {
        c.handle_key(KeyCode::Char(ch), KeyModifiers::NONE);
    }
    c
}

#[test]
fn mascot_renders_in_the_right_strip() {
    let rows = render_rows(&typed("hello"), MascotState::Success);
    let joined = rows.join("\n");
    // The orb's arc and happy face are present...
    assert!(joined.contains(ARC), "missing top arc:\n{joined}");
    assert!(joined.contains("(^"), "missing success face:\n{joined}");
    // ...and it sits in the right half of the field.
    let arc_row = rows.iter().find(|r| r.contains(ARC)).unwrap();
    let col = arc_row.chars().position(|c| c == ARC).unwrap();
    assert!(col > (W / 2) as usize, "mascot should be on the right, was col {col}");
}

#[test]
fn typed_text_wraps_and_never_overdraws_the_mascot() {
    // A long line that would overrun the whole width if it didn't wrap.
    let long = "x".repeat(200);
    let rows = render_rows(&typed(&long), MascotState::Idle);

    // Column where the mascot starts (its top arc).
    let arc_row = rows.iter().find(|r| r.contains(ARC)).unwrap();
    let mascot_col = arc_row.chars().position(|c| c == ARC).unwrap();

    // The text soft-wraps instead of being clipped away: more than one row
    // carries typed 'x's.
    let text_rows = rows.iter().filter(|r| r.contains('x')).count();
    assert!(text_rows > 1, "expected the long line to wrap across rows, got {text_rows}");

    // And on every row the run of 'x's ends before the mascot's strip — text
    // is confined to the left and never overdraws the creature.
    for row in rows.iter().filter(|r| r.contains('x')) {
        let last_x = row.chars().enumerate().filter(|&(_, c)| c == 'x').map(|(i, _)| i).last().unwrap();
        assert!(
            last_x < mascot_col,
            "wrapped text (last x @ {last_x}) ran into the mascot (@ {mascot_col})"
        );
    }
}
