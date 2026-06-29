//! Body renderers for each tool kind. Each returns `Vec<Line>`; the parent
//! `tool` module wraps them in inline/card/collapsed framing.

use agent_repl_core::{
    DiffKind, DiffLine, EntryType, ListEntry, Palette, ReadLine, SearchResult, TodoItem, TodoState,
};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use crate::style::{color, fg, fg_bg};

pub fn diff_body(diff: &[DiffLine], palette: &Palette) -> Vec<Line<'static>> {
    let add_fg = fg(palette.success);
    let del_fg = fg(palette.danger);
    let ctx_fg = fg(palette.text_dim);
    let ln_style = fg(palette.text_faint);
    let mut out = Vec::with_capacity(diff.len());
    for l in diff {
        let (sign, sign_style, text_style) = match l.kind {
            DiffKind::Add => ("+", add_fg, fg(palette.text).add_modifier(Modifier::BOLD)),
            DiffKind::Del => ("\u{2212}", del_fg, fg(palette.text_dim)),
            DiffKind::Ctx => (" ", ctx_fg, fg(palette.text_dim)),
        };
        let a = l.a.map(|n| n.to_string()).unwrap_or_default();
        let b = l.b.map(|n| n.to_string()).unwrap_or_default();
        out.push(Line::from(vec![
            Span::styled(format!("{:>4} ", a), ln_style),
            Span::styled(format!("{:>4} ", b), ln_style),
            Span::styled(format!("{} ", sign), sign_style),
            Span::styled(l.text.clone(), text_style),
        ]));
    }
    out
}

pub fn bash_body(cmd: &str, output: &str, palette: &Palette) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    out.push(Line::from(vec![
        Span::styled("$ ".to_string(), fg(palette.accent)),
        Span::styled(cmd.to_string(), fg(palette.text).add_modifier(Modifier::BOLD)),
    ]));
    if !output.is_empty() {
        let pre = fg_bg(palette.text_dim, palette.bg_inset);
        for raw in output.split('\n') {
            out.push(Line::from(Span::styled(raw.to_string(), pre)));
        }
    }
    out
}

pub fn search_body(result: &SearchResult, palette: &Palette) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let file_style = fg(palette.text).add_modifier(Modifier::BOLD);
    let ln_style = fg(palette.text_faint);
    let text_style = fg(palette.text_dim);
    for g in &result.groups {
        out.push(Line::from(Span::styled(g.file.clone(), file_style)));
        for h in &g.hits {
            out.push(Line::from(vec![
                Span::styled(format!("  {:>4} ", h.line), ln_style),
                Span::styled(h.text.clone(), text_style),
            ]));
        }
    }
    out
}

pub fn list_body(entries: &[ListEntry], palette: &Palette) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(entries.len());
    let dir_style = fg(palette.accent).add_modifier(Modifier::BOLD);
    let file_style = fg(palette.text);
    let meta_style = fg(palette.text_faint);
    for e in entries {
        let is_dir = matches!(e.entry_type, EntryType::Dir);
        let glyph = if is_dir { "▸ " } else { "· " };
        let name_style = if is_dir { dir_style } else { file_style };
        let suffix = if is_dir { "/" } else { "" };
        let mut spans = vec![
            Span::styled(glyph.to_string(), name_style),
            Span::styled(format!("{}{}", e.name, suffix), name_style),
        ];
        if let Some(meta) = &e.meta {
            spans.push(Span::styled(format!("  {}", meta), meta_style));
        }
        out.push(Line::from(spans));
    }
    out
}

pub fn read_body(
    path: &str,
    lines: usize,
    preview: &[ReadLine],
    palette: &Palette,
) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(preview.len() + 1);
    let ln_style = fg(palette.text_faint);
    let text_style = fg(palette.text_dim);
    for p in preview {
        out.push(Line::from(vec![
            Span::styled(format!("  {:>4} ", p.n), ln_style),
            Span::styled(p.text.clone(), text_style),
        ]));
    }
    out.push(Line::from(Span::styled(
        format!("  Read {} lines from {}", lines, path),
        fg(palette.text_faint).add_modifier(Modifier::DIM),
    )));
    out
}

pub fn todo_body(items: &[TodoItem], palette: &Palette) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(items.len());
    let done = fg(palette.success);
    let active = fg(palette.accent).add_modifier(Modifier::BOLD);
    let pending = fg(palette.text_faint);
    for it in items {
        let (glyph, style) = match it.state {
            TodoState::Done => ("✓ ", done),
            TodoState::Active => ("▸ ", active),
            TodoState::Pending => ("○ ", pending),
        };
        out.push(Line::from(vec![
            Span::styled(glyph.to_string(), style),
            Span::styled(it.text.clone(), style),
        ]));
    }
    out
}

pub fn web_body(url: &str, summary: Option<&str>, palette: &Palette) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    out.push(Line::from(vec![
        Span::styled("→ ".to_string(), fg(palette.t_web)),
        Span::styled(
            url.to_string(),
            fg(palette.accent).add_modifier(Modifier::UNDERLINED),
        ),
    ]));
    if let Some(s) = summary {
        out.push(Line::from(Span::styled(s.to_string(), fg(palette.text_dim))));
    }
    out
}

/// Generic body for a tool with no dedicated component: an optional one-line
/// call detail (e.g. compact args), then the result text — no `$` shell framing
/// (that's what `bash_body` is for). Keeps an unmapped tool readable as a plain
/// info block.
pub fn info_body(detail: &str, output: &str, palette: &Palette) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    if !detail.is_empty() {
        out.push(Line::from(Span::styled(detail.to_string(), fg(palette.text_dim))));
    }
    if !output.is_empty() {
        let pre = fg_bg(palette.text_dim, palette.bg_inset);
        for raw in output.split('\n') {
            out.push(Line::from(Span::styled(raw.to_string(), pre)));
        }
    }
    out
}

pub fn tool_summary(call: &agent_repl_core::ToolCall) -> String {
    use agent_repl_core::ToolKind as K;
    match &call.kind {
        K::Edit { diff } | K::Write { diff } => {
            let add = diff.iter().filter(|l| matches!(l.kind, DiffKind::Add)).count();
            let del = diff.iter().filter(|l| matches!(l.kind, DiffKind::Del)).count();
            format!("+{add} \u{2212}{del}")
        }
        K::Bash { exit, .. } => format!("exit {}", exit.unwrap_or(0)),
        K::Search { result } => format!("{} matches", result.count),
        K::List { entries } => format!("{} items", entries.len()),
        K::Read { lines, .. } => format!("{lines} lines"),
        K::Todo { items } => {
            let done = items.iter().filter(|i| matches!(i.state, TodoState::Done)).count();
            format!("{}/{}", done, items.len())
        }
        K::Web { .. } => "fetched".to_string(),
        K::Info { .. } => String::new(),
    }
}

// Silences "unused" warning when `color` is only conditionally referenced.
const _: fn() = || {
    let _ = color;
};
