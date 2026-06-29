//! Tool call wrapper. Builds a [`frame::Spec`] and dispatches to the
//! shared framing helpers. Mirrors `<ToolCall>` in `docs/repl/blocks.jsx`.

use agent_repl_core::{mix, tool_meta, Palette, Rgb, Theme, ToolCall, ToolKind};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::blocks::bodies::{
    bash_body, diff_body, info_body, list_body, read_body, search_body, todo_body, tool_summary,
    web_body,
};
use crate::blocks::frame;
use crate::style::{color, fg};

pub fn render(
    call: &ToolCall,
    theme: &Theme,
    spinner_frame: char,
    expanded: bool,
    focused: bool,
) -> Vec<Line<'static>> {
    let meta = tool_meta(call.kind.id());
    let hue = theme.palette.hue(meta.hue);
    let palette = &theme.palette;

    let header = header_line(call, meta.label, hue, palette, spinner_frame, focused);
    let body = if call.running {
        let label = call
            .run_label
            .clone()
            .unwrap_or_else(|| format!("Running {}\u{2026}", meta.label));
        vec![Line::from(vec![
            Span::styled(format!("{} ", spinner_frame), fg(hue)),
            Span::styled(label, fg(palette.text_dim)),
        ])]
    } else {
        body_for(&call.kind, palette)
    };

    frame::apply(
        frame::Spec { header, body, hue, collapsible: true },
        theme.tool_style,
        palette,
        expanded,
        focused,
    )
}

fn body_for(kind: &ToolKind, palette: &Palette) -> Vec<Line<'static>> {
    match kind {
        ToolKind::Edit { diff } | ToolKind::Write { diff } => diff_body(diff, palette),
        ToolKind::Bash { cmd, output, .. } => bash_body(cmd, output, palette),
        ToolKind::Search { result } => search_body(result, palette),
        ToolKind::List { entries } => list_body(entries, palette),
        ToolKind::Read { path, lines, preview } => read_body(path, *lines, preview, palette),
        ToolKind::Todo { items } => todo_body(items, palette),
        ToolKind::Web { url, summary } => web_body(url, summary.as_deref(), palette),
        ToolKind::Info { detail, output } => info_body(detail, output, palette),
    }
}

fn header_line(
    call: &ToolCall,
    label: &str,
    hue: Rgb,
    palette: &Palette,
    spinner_frame: char,
    focused: bool,
) -> Line<'static> {
    let dot_style = if focused {
        Style::default()
            .fg(color(palette.accent))
            .add_modifier(Modifier::BOLD)
    } else if call.running {
        // A running tool's dot "breathes" between a dim and full hue, in lockstep
        // with the spinner clock, so an in-flight tool is easy to spot at a glance.
        pulsing_dot_style(spinner_frame, hue, palette)
    } else {
        fg(hue)
    };
    let title_style = if focused {
        Style::default()
            .fg(color(palette.accent))
            .add_modifier(Modifier::BOLD)
    } else {
        fg(palette.text).add_modifier(Modifier::BOLD)
    };
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled("● ".to_string(), dot_style),
        Span::styled(
            label.to_string(),
            Style::default().fg(color(hue)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ".to_string(), fg(palette.text_faint)),
        Span::styled(call.title.clone(), title_style),
    ];

    let badge = badge_for(call, palette, spinner_frame);
    if let Some(b) = badge {
        spans.push(Span::styled("   ".to_string(), Style::default()));
        spans.extend(b);
    }
    Line::from(spans)
}

/// The dot color for a running tool: a cosine breathe between a dim hue (≈45%
/// toward the background) and the full hue, phased off the spinner clock.
fn pulsing_dot_style(spinner_frame: char, hue: Rgb, palette: &Palette) -> Style {
    let pulse = crate::spinner::pulse_for(spinner_frame);
    let dim = mix(palette.bg, hue, 0.45);
    let col = mix(dim, hue, pulse);
    fg(col).add_modifier(Modifier::BOLD)
}

fn badge_for(
    call: &ToolCall,
    palette: &Palette,
    spinner_frame: char,
) -> Option<Vec<Span<'static>>> {
    if call.running {
        return Some(vec![
            Span::styled("[".to_string(), fg(palette.text_faint)),
            Span::styled(format!("{} running", spinner_frame), fg(palette.warning)),
            Span::styled("]".to_string(), fg(palette.text_faint)),
        ]);
    }
    if let ToolKind::Bash { exit, .. } = &call.kind {
        let code = exit.unwrap_or(0);
        let style = if code == 0 {
            fg(palette.success)
        } else {
            fg(palette.danger).add_modifier(Modifier::BOLD)
        };
        return Some(vec![
            Span::styled("[".to_string(), fg(palette.text_faint)),
            Span::styled(format!("exit {code}"), style),
            Span::styled("]".to_string(), fg(palette.text_faint)),
        ]);
    }
    let summary = tool_summary(call);
    if summary.is_empty() {
        return None;
    }
    Some(vec![Span::styled(summary, fg(palette.text_faint))])
}
