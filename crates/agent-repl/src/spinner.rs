//! Braille spinner. Same frames as `docs/repl/blocks.jsx` line 8.

use std::time::{Duration, Instant};

pub const FRAMES: &[char] = &[
    '\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}',
    '\u{2834}', '\u{2826}', '\u{2827}', '\u{2807}', '\u{280F}',
];

/// Returns the current spinner glyph given a monotonic instant. Frames
/// advance every 80ms — same cadence as the JSX reference.
pub fn glyph_at(t: Instant) -> char {
    let elapsed = t.elapsed();
    frame_for(elapsed)
}

pub fn frame_for(elapsed: Duration) -> char {
    let i = (elapsed.as_millis() / 80) as usize % FRAMES.len();
    FRAMES[i]
}

/// Index of `frame` within [`FRAMES`] (0 if absent). Lets callers derive a
/// phase from the same clock that already drives the spinner glyph — e.g. to
/// pulse a tool's status dot in lockstep without threading the elapsed time.
pub fn index_of(frame: char) -> usize {
    FRAMES.iter().position(|&c| c == frame).unwrap_or(0)
}

/// A smooth breathing value in `0.0..=1.0` for the given spinner glyph: `0` at
/// the start of the cycle, `1` at the midpoint, back to `0` — a cosine ease.
pub fn pulse_for(frame: char) -> f32 {
    let t = index_of(frame) as f32 / FRAMES.len() as f32;
    0.5 - 0.5 * (t * std::f32::consts::TAU).cos()
}
