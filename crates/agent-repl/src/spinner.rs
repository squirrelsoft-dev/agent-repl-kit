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
