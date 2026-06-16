//! Generic framing helpers shared by every block kind (tools and messages).
//! A frame wraps a header line + body lines in the visual style picked by
//! the active [`ToolStyle`]:
//!
//! - `Inline` — a colored gutter `│` on the left of every line.
//! - `Card` — `╭─ header` top, `│  body` sides, `╰─` bottom.
//! - `Collapsed` — chevron + header only; body hidden unless `expanded`.
//!   Non-collapsible blocks (e.g. assistant prose) treat this as `Inline`
//!   so prose is never silently hidden.

use agent_repl_core::{Palette, Rgb, ToolStyle};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::style::{color, fg};

/// What a frame call needs to know about the block being framed.
#[derive(Debug)]
pub struct Spec {
    pub header: Line<'static>,
    pub body: Vec<Line<'static>>,
    pub hue: Rgb,
    /// `true` = collapsed-style may legitimately hide the body (tools).
    /// `false` = falls back to inline-style for collapsed (prose / alerts).
    pub collapsible: bool,
}

pub fn apply(
    spec: Spec,
    style: ToolStyle,
    palette: &Palette,
    expanded: bool,
    focused: bool,
) -> Vec<Line<'static>> {
    match style {
        ToolStyle::Inline => frame_inline(spec.header, spec.body, spec.hue),
        ToolStyle::Card => frame_card(spec.header, spec.body, spec.hue),
        ToolStyle::Collapsed => {
            if spec.collapsible {
                frame_collapsed(spec.header, spec.body, expanded, focused, spec.hue, palette)
            } else {
                // Non-collapsible blocks (assistant prose, alerts) keep their
                // body visible — fall through to inline framing.
                frame_inline(spec.header, spec.body, spec.hue)
            }
        }
    }
}

fn frame_inline(
    header: Line<'static>,
    body: Vec<Line<'static>>,
    hue: Rgb,
) -> Vec<Line<'static>> {
    let gutter = || Span::styled("│ ".to_string(), fg(hue));
    let mut out = Vec::with_capacity(body.len() + 1);
    out.push(prepend(gutter(), header));
    for line in body {
        out.push(prepend(gutter(), line));
    }
    out
}

fn frame_card(
    header: Line<'static>,
    body: Vec<Line<'static>>,
    hue: Rgb,
) -> Vec<Line<'static>> {
    let top = Span::styled("╭─ ".to_string(), fg(hue));
    let side = || Span::styled("│  ".to_string(), fg(hue));
    let bottom = Span::styled("╰─".to_string(), fg(hue));

    let mut out = Vec::with_capacity(body.len() + 2);
    out.push(prepend(top, header));
    for line in body {
        out.push(prepend(side(), line));
    }
    out.push(Line::from(vec![bottom]));
    out
}

fn frame_collapsed(
    header: Line<'static>,
    body: Vec<Line<'static>>,
    expanded: bool,
    focused: bool,
    hue: Rgb,
    palette: &Palette,
) -> Vec<Line<'static>> {
    let chev_glyph = if expanded { "▾ " } else { "▸ " };
    let chev_color = if focused { palette.accent } else { hue };
    let chev = Span::styled(
        chev_glyph.to_string(),
        Style::default()
            .fg(color(chev_color))
            .add_modifier(Modifier::BOLD),
    );
    let mut out = Vec::new();
    out.push(prepend(chev, header));
    if expanded {
        for line in body {
            out.push(prepend(
                Span::styled("  ".to_string(), Style::default()),
                line,
            ));
        }
    }
    out
}

fn prepend(span: Span<'static>, line: Line<'static>) -> Line<'static> {
    let mut spans = vec![span];
    spans.extend(line.spans);
    Line::from(spans)
}
