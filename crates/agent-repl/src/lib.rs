//! `agent-repl` — ratatui-based REPL UI for coding agents. Plug in events
//! via [`ReplHandle`], cycle themes with the four enums in
//! [`agent_repl_core`].

#![warn(missing_debug_implementations)]

pub mod app;
pub mod approval;
pub mod blocks;
pub mod composer;
pub mod decorations;
pub mod gallery;
pub mod handle;
pub mod markdown;
pub mod question;
pub mod spinner;
pub mod stream;
pub mod style;

pub use agent_repl_core::*;
pub use app::{AgentRepl, AppView};
pub use composer::{Composer, ComposerAction};
pub use decorations::Decorations;
pub use handle::{ReplHandle, ToolHandle};
pub use question::{QuestionAction, QuestionState};
pub use stream::ToolId;
