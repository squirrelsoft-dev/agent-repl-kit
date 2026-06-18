//! Three-level approval prompt — accept / accept-for-all / deny.
//!
//! The event model surfaces no approval gate on its own. This adds a small,
//! renderer-agnostic prompt type plus the user's choice. The renderer shows a
//! key-hint bar while a prompt is active; the driving task delivers the choice
//! back through a channel (see `ReplHandle::recv_approval`).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A pending approval: what is being approved, and whether "accept for all" is
/// offered (it is for low-risk reads, hidden for session-ephemeral writes).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ApprovalPrompt {
    /// Short headline, e.g. `read_file(path=src/main.rs)`.
    pub title: String,
    /// Optional secondary line, e.g. risk level / how many calls.
    pub detail: Option<String>,
    /// Label for the "accept for all" option. `None` hides that option.
    pub accept_all_label: Option<String>,
}

impl ApprovalPrompt {
    pub fn new(
        title: impl Into<String>,
        detail: Option<String>,
        accept_all_label: Option<String>,
    ) -> Self {
        Self { title: title.into(), detail, accept_all_label }
    }
}

/// The user's response to an [`ApprovalPrompt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ApprovalChoice {
    /// Allow this once.
    Accept,
    /// Allow this and persist a matching allow (only offered for reads).
    AcceptAll,
    /// Deny this turn.
    Deny,
}
