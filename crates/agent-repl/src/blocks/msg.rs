//! Message-type block renderers: user / assistant / reasoning / status / alert.
//!
//! `user`, `assistant`, `error`, and `warning` now follow the active
//! `tool_style` (inline / card / collapsed → inline) so they read as
//! first-class blocks alongside tool calls. `reasoning` and `status`
//! keep their own one-off framing since they have block-specific
//! behavior (reasoning has its own collapse chevron; status is transient).

use agent_repl_core::{AlertLevel, Theme};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::blocks::frame;
use crate::decorations::Decorations;
use crate::markdown;
use crate::style::{color, fg};

pub fn user(text: &str, theme: &Theme, deco: &Decorations) -> Vec<Line<'static>> {
    let p = &theme.palette;
    let hue = p.text_dim;
    let mut header_spans = Vec::new();
    if let Some(sigil) = &deco.user_sigil {
        header_spans.push(Span::styled(
            format!("{sigil} "),
            Style::default().fg(color(hue)).add_modifier(Modifier::BOLD),
        ));
    }
    header_spans.push(Span::styled(
        "user".to_string(),
        Style::default().fg(color(hue)).add_modifier(Modifier::BOLD),
    ));
    let body = markdown::render(text, p);
    frame::apply(
        frame::Spec { header: Line::from(header_spans), body, hue, collapsible: false },
        theme.tool_style,
        p,
        true,
        false,
    )
}

pub fn assistant(
    text: &str,
    theme: &Theme,
    deco: &Decorations,
    streaming: bool,
) -> Vec<Line<'static>> {
    let p = &theme.palette;
    let hue = p.accent;
    let mut header_spans = Vec::new();
    if let Some(sigil) = &deco.assistant_sigil {
        header_spans.push(Span::styled(
            format!("{sigil} "),
            Style::default().fg(color(hue)).add_modifier(Modifier::BOLD),
        ));
    }
    header_spans.push(Span::styled(
        "assistant".to_string(),
        Style::default().fg(color(hue)).add_modifier(Modifier::BOLD),
    ));
    let mut body = markdown::render(text, p);
    if streaming {
        if let Some(last) = body.last_mut() {
            last.spans.push(Span::styled("▍".to_string(), fg(p.accent)));
        }
    }
    frame::apply(
        frame::Spec { header: Line::from(header_spans), body, hue, collapsible: false },
        theme.tool_style,
        p,
        true,
        false,
    )
}

pub fn reasoning(text: &str, ms: Option<u32>, default_open: bool, theme: &Theme) -> Vec<Line<'static>> {
    // Reasoning keeps its own chevron-driven collapse — tool_style framing
    // would clash with it. Render plain with an internal chevron + dim
    // italic body when open.
    let p = &theme.palette;
    let title = match ms {
        Some(ms) => format!("Thought for {:.1}s", (ms as f32) / 1000.0),
        None => "Thought".to_string(),
    };
    let chev = if default_open { "▾" } else { "▸" };
    let mut out = Vec::new();
    out.push(Line::from(vec![
        Span::styled(format!("{} ", chev), fg(p.text_faint)),
        Span::styled(title, fg(p.text_dim).add_modifier(Modifier::ITALIC)),
    ]));
    if default_open {
        for l in markdown::render(text, p) {
            out.push(indent_styled(l, "  ", fg(p.text_dim).add_modifier(Modifier::ITALIC)));
        }
    }
    out
}

pub fn status(text: &str, theme: &Theme, _spinner_frame: char) -> Vec<Line<'static>> {
    let p = &theme.palette;
    // A STATIC sigil — a status line is a persistent transcript entry (e.g.
    // "switched to accept-edits mode"), NOT live progress, so it must not animate
    // with the spinner clock (that made every status message appear to spin
    // forever). The live spinner lives only on the dedicated working line.
    vec![Line::from(vec![
        Span::styled("· ".to_string(), fg(p.accent)),
        Span::styled(text.to_string(), fg(p.text_dim)),
    ])]
}

pub fn alert(level: AlertLevel, title: &str, detail: Option<&str>, theme: &Theme) -> Vec<Line<'static>> {
    let p = &theme.palette;
    let (sym, hue) = match level {
        AlertLevel::Error => ("\u{2715}", p.danger),
        AlertLevel::Warning => ("\u{26A0}", p.warning),
    };
    let header = Line::from(vec![
        Span::styled(
            format!("{} ", sym),
            Style::default()
                .fg(color(hue))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            title.to_string(),
            fg(p.text).add_modifier(Modifier::BOLD),
        ),
    ]);
    let body = detail.map(|d| markdown::render(d, p)).unwrap_or_default();
    frame::apply(
        frame::Spec { header, body, hue, collapsible: false },
        theme.tool_style,
        p,
        true,
        false,
    )
}

fn indent_styled(line: Line<'static>, pre: &str, style: Style) -> Line<'static> {
    let mut spans = vec![Span::styled(pre.to_string(), style)];
    spans.extend(line.spans);
    Line::from(spans)
}
