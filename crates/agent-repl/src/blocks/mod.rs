//! Top-level block dispatcher: turn an [`Event`] into a `Vec<Line>` for
//! the active theme.

use agent_repl_core::{Event, Theme};
use ratatui::text::Line;

use crate::decorations::Decorations;

pub mod bodies;
pub mod frame;
pub mod msg;
pub mod tool;

/// Convert an event into a list of styled lines.
///
/// `expanded` and `focused` only affect tool blocks under
/// [`agent_repl_core::ToolStyle::Collapsed`] — passing `true` / `false`
/// for other event types is harmless.
pub fn render(
    ev: &Event,
    theme: &Theme,
    deco: &Decorations,
    spinner_frame: char,
    expanded: bool,
    focused: bool,
) -> Vec<Line<'static>> {
    match ev {
        Event::User { text } => msg::user(text, theme, deco),
        Event::Assistant { text } => msg::assistant(text, theme, deco, false),
        Event::Reasoning { text, ms, default_open } => {
            msg::reasoning(text, *ms, *default_open, theme)
        }
        Event::Status { text } => msg::status(text, theme, spinner_frame),
        Event::Alert { level, title, detail } => {
            msg::alert(*level, title, detail.as_deref(), theme)
        }
        Event::Tool(call) => tool::render(call, theme, spinner_frame, expanded, focused),
    }
}
