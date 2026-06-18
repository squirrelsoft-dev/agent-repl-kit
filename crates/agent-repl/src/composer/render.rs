//! Composer rendering. Multi-line field with chip-styled `@tokens`, a
//! slash / `@file` menu popup, footer with model pill + key hints.

use std::time::Duration;

use agent_repl_core::{Density, Theme, ToolStyle};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::composer::state::{Composer, MenuKind};
use crate::mascot::{Mascot, MascotState};
use crate::style::{color, fg};

/// What the composer needs to paint a mascot in its reserved right strip.
#[derive(Debug)]
pub struct MascotPaint<'a> {
    pub mascot: &'a dyn Mascot,
    pub state: MascotState,
    pub elapsed: Duration,
}

/// Total rows the composer needs given the active theme. Grows with the
/// buffer (capped at [`MAX_VISIBLE_LINES`]).
pub fn required_height(composer: &Composer, theme: &Theme) -> u16 {
    let visible = composer.visible_line_count() as u16;
    let breathing = breathing_rows(theme);
    1 /*top*/ + visible + breathing + 1 /*footer*/ + 1 /*bottom*/
}

/// Rows needed for the menu popup (0 if no menu is open).
pub fn menu_height(composer: &Composer, _theme: &Theme) -> u16 {
    if !composer.menu_open() {
        return 0;
    }
    let n = composer.menu_items().len().min(8) as u16;
    // top border + header + n items + bottom border
    1 + 1 + n + 1
}

fn breathing_rows(theme: &Theme) -> u16 {
    match theme.density {
        Density::Comfortable => 1,
        Density::Compact => 0,
    }
}

pub fn render(
    composer: &Composer,
    theme: &Theme,
    frame: &mut Frame,
    area: Rect,
    mascot: Option<MascotPaint<'_>>,
) {
    let p = &theme.palette;
    let hue = p.accent;

    let block = match theme.tool_style {
        ToolStyle::Inline => Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(fg(p.border)),
        _ => Block::default()
            .borders(Borders::ALL)
            .border_style(fg(hue))
            .style(Style::default().bg(color(p.bg_raised))),
    };
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    let visible = composer.visible_line_count() as u16;
    let breathing = breathing_rows(theme).min(inner.height.saturating_sub(visible + 1));
    let field_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: visible.min(inner.height.saturating_sub(1)),
    };
    let footer_area = Rect {
        x: inner.x,
        y: inner.y + field_area.height + breathing,
        width: inner.width,
        height: 1,
    };

    // Carve a right-hand strip for the mascot; the text gets the rest. Because
    // the field renders into its own narrower rect, typed text is clipped at the
    // strip and can never overdraw the mascot.
    let reserved = composer
        .reserved_right()
        .min(field_area.width.saturating_sub(1));
    let text_area = Rect { width: field_area.width - reserved, ..field_area };

    if let Some(paint) = mascot {
        let (w, _h) = paint.mascot.size();
        let mw = w.min(reserved);
        if mw > 0 {
            // Right-align in the reserved strip, keeping a 1-col right margin off
            // the border when the strip is wide enough (it is for `with_mascot`,
            // which reserves width + gap). The remaining gap sits on the left,
            // between the text and the mascot.
            let margin = u16::from(reserved > mw);
            let mascot_area = Rect {
                x: field_area.x + field_area.width - mw - margin,
                y: field_area.y,
                width: mw,
                height: field_area.height,
            };
            let lines = paint.mascot.render(paint.state, paint.elapsed, p);
            frame.render_widget(Paragraph::new(Text::from(lines)), mascot_area);
        }
    }

    draw_field(composer, theme, frame, text_area);
    draw_footer(composer, theme, frame, footer_area);
}

pub fn render_menu(composer: &Composer, theme: &Theme, frame: &mut Frame, area: Rect) {
    if !composer.menu_open() || area.height == 0 {
        return;
    }
    let p = &theme.palette;
    let items = composer.menu_items();
    let kind = composer.menu_kind().unwrap();
    let header_label = match kind {
        MenuKind::Slash => "commands",
        MenuKind::At => "files",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(fg(p.accent))
        .style(Style::default().bg(color(p.bg_raised)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![
        Span::raw("  ".to_string()),
        Span::styled(
            header_label.to_string(),
            fg(p.text_faint).add_modifier(Modifier::ITALIC),
        ),
    ]));

    let visible_items = inner.height.saturating_sub(1) as usize;
    for (i, item) in items.iter().take(visible_items).enumerate() {
        let is_sel = i == composer.menu_selected();
        let marker = if is_sel { "▸ " } else { "  " };
        let cmd_style = if is_sel {
            Style::default()
                .fg(color(p.accent))
                .bg(color(p.accent_soft))
                .add_modifier(Modifier::BOLD)
        } else {
            fg(p.accent).add_modifier(Modifier::BOLD)
        };
        let desc_style = if is_sel {
            fg(p.text).add_modifier(Modifier::DIM)
        } else {
            fg(p.text_dim)
        };

        let mut spans = vec![
            Span::styled(marker.to_string(), fg(p.accent).add_modifier(Modifier::BOLD)),
            Span::styled(item.value.clone(), cmd_style),
        ];
        if !item.description.is_empty() {
            spans.push(Span::raw("  ".to_string()));
            spans.push(Span::styled(item.description.clone(), desc_style));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

// -----------------------------------------------------------------------------
// field
// -----------------------------------------------------------------------------

fn draw_field(composer: &Composer, theme: &Theme, frame: &mut Frame, area: Rect) {
    let p = &theme.palette;
    // The working state lives on its own line above the composer (see
    // `AgentRepl::draw_working_line`); the field keeps its normal sigil and just
    // hides its placeholder while the agent runs.
    let sigil_glyph = "\u{276F}".to_string(); // ❯
    let sigil_style = fg(p.accent).add_modifier(Modifier::BOLD);
    let pad_str = "  ".to_string();
    let continuation_str = "    ".to_string(); // 2 (pad) + 2 (sigil + space)
    let placeholder = "Ask the agent to do something\u{2026}";

    let chip_style = Style::default()
        .fg(color(p.accent))
        .bg(color(p.accent_soft))
        .add_modifier(Modifier::BOLD);
    let placeholder_style = fg(p.text_faint).add_modifier(Modifier::ITALIC);
    let body_style = fg(p.text);

    let lines = composer.lines();
    let scroll_top = composer.scroll_top();
    let visible = area.height as usize;
    let show_placeholder = composer.is_empty() && !composer.working;

    let mut rows: Vec<Line<'static>> = Vec::with_capacity(visible);
    for vi in 0..visible {
        let line_idx = scroll_top + vi;
        if line_idx >= lines.len() {
            // Pad blank rows so the field area stays its declared height.
            rows.push(Line::raw(""));
            continue;
        }
        let is_first_row = vi == 0;
        let prefix = if is_first_row {
            vec![
                Span::raw(pad_str.clone()),
                Span::styled(format!("{sigil_glyph} "), sigil_style),
            ]
        } else {
            vec![Span::raw(continuation_str.clone())]
        };
        let mut spans = prefix;
        let content = &lines[line_idx];
        if is_first_row && show_placeholder {
            spans.push(Span::styled(placeholder.to_string(), placeholder_style));
        } else {
            spans.extend(render_line_with_chips(content, body_style, chip_style));
        }
        rows.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(Text::from(rows)), area);

    if !composer.working {
        let cursor_vrow = composer.cursor_line().saturating_sub(scroll_top);
        if cursor_vrow < area.height as usize {
            // First visible row uses sigil prefix (4 cols), subsequent rows use
            // continuation indent (also 4 cols) — same offset either way.
            let prefix_cols: u16 = 4;
            let cx = area.x + prefix_cols + composer.cursor_col() as u16;
            let cy = area.y + cursor_vrow as u16;
            if cx < area.x + area.width {
                frame.set_cursor_position(Position { x: cx, y: cy });
            }
        }
    }
}

/// Split a line's text into plain spans + chip-styled spans for any
/// `@token` that starts at word boundaries.
fn render_line_with_chips(
    line: &str,
    body: Style,
    chip: Style,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        let prev_is_boundary = i == 0 || chars[i - 1].is_whitespace();
        if chars[i] == '@' && prev_is_boundary {
            if !buf.is_empty() {
                out.push(Span::styled(std::mem::take(&mut buf), body));
            }
            let mut chip_text = String::from('@');
            let mut j = i + 1;
            while j < chars.len() && !chars[j].is_whitespace() {
                chip_text.push(chars[j]);
                j += 1;
            }
            out.push(Span::styled(chip_text, chip));
            i = j;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        out.push(Span::styled(buf, body));
    }
    out
}

// -----------------------------------------------------------------------------
// footer
// -----------------------------------------------------------------------------

fn draw_footer(composer: &Composer, theme: &Theme, frame: &mut Frame, area: Rect) {
    let p = &theme.palette;
    let sep = Span::raw("  ".to_string());

    // The context pill (model · cwd · branch) stays visible even while working;
    // the working spinner lives on its own line above the composer.
    let pill_style = Style::default()
        .fg(color(p.accent))
        .bg(color(p.accent_soft))
        .add_modifier(Modifier::BOLD);
    let mut left_spans: Vec<Span<'static>> = vec![
        Span::raw("  ".to_string()),
        Span::styled(format!(" \u{25C7} {} ", composer.model), pill_style),
    ];
    if !composer.cwd.is_empty() {
        left_spans.push(sep.clone());
        left_spans.push(Span::styled(composer.cwd.clone(), fg(p.text_dim)));
    }
    if let Some(branch) = &composer.branch {
        left_spans.push(sep.clone());
        left_spans.push(Span::styled(format!("\u{2387} {branch}"), fg(p.text_dim)));
    }

    let right_text = if composer.working {
        "esc interrupt".to_string()
    } else if composer.menu_open() {
        "\u{2191}\u{2193} nav · \u{23CE}/tab accept · esc dismiss ".to_string()
    } else {
        "\u{23CE} send · \u{21E7}\u{23CE} newline · / cmds · @ files ".to_string()
    };
    let right = Span::styled(right_text, fg(p.text_faint));

    let left_w: u16 = left_spans
        .iter()
        .map(|s| s.content.chars().count() as u16)
        .sum();
    let right_w: u16 = right.content.chars().count() as u16;
    let pad = area.width.saturating_sub(left_w + right_w);
    let mut spans = left_spans;
    spans.push(Span::raw(" ".repeat(pad as usize)));
    spans.push(right);

    let footer_style = if matches!(theme.tool_style, ToolStyle::Inline) {
        Style::default().fg(color(p.text_dim))
    } else {
        Style::default().bg(color(p.bg_raised)).fg(color(p.text_dim))
    };
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(footer_style),
        area,
    );
}
