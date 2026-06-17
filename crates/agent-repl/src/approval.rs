//! Renderer for the three-level approval prompt — a "permissions box" that
//! floats directly above the composer while a gate is pending. The box lists
//! the offered choices (Yes / Always / No); the user picks with ↑↓ + Enter or
//! the `y`/`a`/`n` (and `1`/`2`/`3`) shortcuts. The chosen [`ApprovalChoice`]
//! is delivered back to the driving task via `ReplHandle::recv_approval`.

use agent_repl_core::{ApprovalChoice, ApprovalPrompt, Theme};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::style::{color, fg};

/// The selectable choices for a prompt, in display order. The "Always" option
/// is only present when the prompt offers an `accept_all_label`.
pub fn options(prompt: &ApprovalPrompt) -> Vec<(ApprovalChoice, String)> {
    let mut opts = vec![(ApprovalChoice::Accept, "Yes".to_string())];
    if let Some(label) = &prompt.accept_all_label {
        opts.push((ApprovalChoice::AcceptAll, format!("Always \u{2014} {label}")));
    }
    opts.push((ApprovalChoice::Deny, "No".to_string()));
    opts
}

/// Total rows the permissions box occupies: top border + title + optional
/// detail line + spacer + one row per option + bottom border.
pub fn required_height(prompt: &ApprovalPrompt) -> u16 {
    let detail = u16::from(prompt.detail.is_some());
    let opts = options(prompt).len() as u16;
    1 + 1 + detail + 1 + opts + 1
}

/// Render the permissions box into `area`. `selected` is the highlighted
/// option index (clamped to the available options).
pub fn render(
    prompt: &ApprovalPrompt,
    selected: usize,
    theme: &Theme,
    frame: &mut Frame,
    area: Rect,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let p = &theme.palette;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(fg(p.warning).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            " permission required ".to_string(),
            fg(p.warning).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(color(p.bg_raised)));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let opts = options(prompt);
    let selected = selected.min(opts.len().saturating_sub(1));

    let mut lines: Vec<Line<'static>> = Vec::new();

    // What is being approved.
    lines.push(Line::from(vec![
        Span::raw("  ".to_string()),
        Span::styled(prompt.title.clone(), fg(p.text).add_modifier(Modifier::BOLD)),
    ]));
    if let Some(detail) = &prompt.detail {
        lines.push(Line::from(vec![
            Span::raw("  ".to_string()),
            Span::styled(detail.clone(), fg(p.text_dim)),
        ]));
    }
    lines.push(Line::raw(""));

    // One row per choice, the selected one highlighted like the slash menu.
    for (i, (_choice, label)) in opts.iter().enumerate() {
        let is_sel = i == selected;
        let marker = if is_sel { "\u{276F} " } else { "  " };
        let label_style = if is_sel {
            Style::default()
                .fg(color(p.accent))
                .bg(color(p.accent_soft))
                .add_modifier(Modifier::BOLD)
        } else {
            fg(p.text_dim)
        };
        let num_style = if is_sel {
            fg(p.accent).add_modifier(Modifier::BOLD)
        } else {
            fg(p.text_faint)
        };
        lines.push(Line::from(vec![
            Span::raw("  ".to_string()),
            Span::styled(marker.to_string(), fg(p.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}. ", i + 1), num_style),
            Span::styled(label.clone(), label_style),
        ]));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}
