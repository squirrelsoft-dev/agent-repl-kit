//! Fixed-at-bottom input composer. State + edit ops in [`state`], rendering
//! in [`render`]. Mirrors `LiveComposer` / `InputFrame` in
//! `docs/repl/input.jsx`.

pub mod render;
pub mod state;

pub use state::{
    Composer, ComposerAction, MenuItem, MenuKind, MAX_VISIBLE_LINES, SLASH_COMMANDS,
};
