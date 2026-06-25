//! Fixed-at-bottom input composer. State + edit ops in [`state`], rendering
//! in [`render`]. Mirrors `LiveComposer` / `InputFrame` in
//! `docs/repl/input.jsx`.

pub mod render;
pub mod state;

pub use state::{
    Composer, ComposerAction, FieldLayout, MenuItem, MenuKind, VisualRow, MAX_VISIBLE_LINES,
    SLASH_COMMANDS,
};
