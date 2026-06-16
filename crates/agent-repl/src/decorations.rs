//! Per-block visual decorations layered on top of a [`Theme`]. Currently
//! just optional sigils (a glyph or emoji) shown before role labels.
//!
//! ```no_run
//! use agent_repl::{AgentRepl, Theme};
//! let (app, _handle) = AgentRepl::new(Theme::slate());
//! let app = app
//!     .with_assistant_sigil("🤖")
//!     .with_user_sigil("●");
//! ```
//!
//! Default = no sigils on user/assistant (matches the JSX reference).

#[derive(Debug, Default, Clone)]
pub struct Decorations {
    /// Glyph or emoji prepended to the `user` header. None = no sigil.
    pub user_sigil: Option<String>,
    /// Glyph or emoji prepended to the `assistant` header. None = no sigil.
    /// Suggested: `●` for parity with tool blocks, or `🤖` / `🍄` / `🦄`.
    pub assistant_sigil: Option<String>,
}

impl Decorations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn user_sigil(mut self, sigil: impl Into<String>) -> Self {
        self.user_sigil = Some(sigil.into());
        self
    }

    pub fn assistant_sigil(mut self, sigil: impl Into<String>) -> Self {
        self.assistant_sigil = Some(sigil.into());
        self
    }

    pub fn clear_user_sigil(mut self) -> Self {
        self.user_sigil = None;
        self
    }

    pub fn clear_assistant_sigil(mut self) -> Self {
        self.assistant_sigil = None;
        self
    }
}
