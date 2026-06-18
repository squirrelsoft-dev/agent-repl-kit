//! Pluggable mascot rendered in the right strip of the composer.
//!
//! A [`Mascot`] is a tiny animated creature drawn beside the input field. It
//! reports a fixed cell [`Mascot::size`] (which the composer uses to reserve a
//! right-hand strip and a minimum input height) and renders itself for a given
//! [`MascotState`] and animation clock.
//!
//! [`BallMascot`] — a minimal theme-aware "orb with a face" — ships with the
//! kit as the reference example to copy when wiring in your own. Implement the
//! [`Mascot`] trait to drop in a richer creature of your own.
//!
//! Drive the state with [`crate::ReplHandle::set_mascot_state`]; the app also
//! moves it between [`MascotState::Idle`] and [`MascotState::Thinking`]
//! automatically as the agent starts and finishes work.

use std::time::Duration;

use agent_repl_core::{Palette, Rgb};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::style::fg;

/// What the mascot is currently expressing. The app maps the agent's
/// working/idle flag onto `Idle`/`Thinking`; richer states are set explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MascotState {
    /// Waiting at the prompt.
    #[default]
    Idle,
    /// Generic "working" — the default while the agent runs.
    Thinking,
    /// Editing / writing code.
    Coding,
    /// Running tests.
    Testing,
    /// A turn finished happily.
    Success,
    /// Something went wrong.
    Error,
}

/// A small animated creature drawn in the composer's right strip.
///
/// Implementations are cheap, immutable, and deterministic given
/// `(state, elapsed)` — the renderer calls [`Mascot::render`] every frame with a
/// monotonically increasing `elapsed` so animation is a pure function of time.
pub trait Mascot: std::fmt::Debug {
    /// Fixed `(width, height)` in cells. Drives the reserved strip width and the
    /// composer's minimum input height.
    fn size(&self) -> (u16, u16);

    /// Render the mascot's rows for `state` at animation time `elapsed`. Lines
    /// should be at most `size().0` wide and exactly `size().1` tall.
    fn render(&self, state: MascotState, elapsed: Duration, palette: &Palette) -> Vec<Line<'static>>;
}

fn bold(rgb: Rgb) -> Style {
    fg(rgb).add_modifier(Modifier::BOLD)
}

// =============================================================================
// BallMascot — minimal theme-aware reference implementation
// =============================================================================

/// A minimal 3-row "orb with a face" that blinks and shifts expression by
/// state. Colors follow the active theme palette, so it fits any vibe. This is
/// the reference mascot — copy it as a starting point for your own.
#[derive(Debug, Default, Clone, Copy)]
pub struct BallMascot;

impl BallMascot {
    /// `(left_eye, mouth, right_eye)` for a state's resting face.
    fn face_parts(state: MascotState) -> (&'static str, &'static str, &'static str) {
        match state {
            MascotState::Idle => ("\u{2022}", "\u{203F}", "\u{2022}"), // (•‿•)
            MascotState::Thinking => ("\u{2022}", ".", "\u{2022}"),    // (•.•)
            MascotState::Coding => ("\u{2022}", "_", "\u{2022}"),      // (•_•)
            MascotState::Testing => ("\u{2022}", "o", "\u{2022}"),     // (•o•)
            MascotState::Success => ("^", "\u{203F}", "^"),            // (^‿^)
            MascotState::Error => (">", "_", "<"),                     // (>_<)
        }
    }

    fn color(state: MascotState, p: &Palette) -> Rgb {
        match state {
            MascotState::Idle => p.text_dim,
            MascotState::Thinking => p.info,
            MascotState::Coding => p.accent,
            MascotState::Testing => p.warning,
            MascotState::Success => p.success,
            MascotState::Error => p.danger,
        }
    }
}

impl Mascot for BallMascot {
    fn size(&self) -> (u16, u16) {
        (5, 3)
    }

    fn render(&self, state: MascotState, elapsed: Duration, palette: &Palette) -> Vec<Line<'static>> {
        // A quick blink every ~1.2s (skipped for the already-squinting Error).
        let blink = (elapsed.as_millis() % 1200) < 140 && state != MascotState::Error;
        let (mut le, mouth, mut re) = Self::face_parts(state);
        if blink {
            le = "\u{2013}"; // –
            re = "\u{2013}";
        }
        let face_style = bold(Self::color(state, palette));
        let arc_style = fg(palette.border);
        vec![
            Line::from(vec![Span::styled(" \u{256D}\u{2500}\u{256E}".to_string(), arc_style)]), // ╭─╮
            Line::from(vec![Span::styled(format!("({le}{mouth}{re})"), face_style)]),
            Line::from(vec![Span::styled(" \u{2570}\u{2500}\u{256F}".to_string(), arc_style)]), // ╰─╯
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_repl_core::{palette_for, Mode, Vibe};

    fn pal() -> Palette {
        palette_for(Vibe::Slate, Mode::Dark)
    }

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// Center column of a line's non-space content.
    fn center(line: &Line) -> usize {
        let chars: Vec<char> = line_text(line).chars().collect();
        let first = chars.iter().position(|c| !c.is_whitespace()).unwrap();
        let last = chars.iter().rposition(|c| !c.is_whitespace()).unwrap();
        (first + last) / 2
    }

    #[test]
    fn ball_size_and_centering() {
        let m = BallMascot;
        assert_eq!(m.size(), (5, 3));
        let rows = m.render(MascotState::Idle, Duration::ZERO, &pal());
        assert_eq!(rows.len(), 3);
        // Arc top center aligns with the face center.
        assert_eq!(center(&rows[0]), center(&rows[1]));
        assert_eq!(center(&rows[2]), center(&rows[1]));
    }

    #[test]
    fn ball_face_changes_by_state() {
        let m = BallMascot;
        // Use a non-blink instant so eyes are the resting glyphs.
        let t = Duration::from_millis(600);
        assert!(line_text(&m.render(MascotState::Error, t, &pal())[1]).contains("(>_<)"));
        assert!(line_text(&m.render(MascotState::Success, t, &pal())[1]).contains('^'));
    }

    #[test]
    fn ball_blinks_then_opens() {
        let m = BallMascot;
        let closed = line_text(&m.render(MascotState::Idle, Duration::from_millis(0), &pal())[1]);
        let open = line_text(&m.render(MascotState::Idle, Duration::from_millis(600), &pal())[1]);
        assert!(closed.contains('\u{2013}'), "expected blink at t=0: {closed:?}");
        assert!(!open.contains('\u{2013}'), "expected open eyes at t=600ms: {open:?}");
    }
}
