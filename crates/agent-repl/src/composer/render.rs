//! Composer rendering. Multi-line field with chip-styled `@tokens`, a
//! slash / `@file` menu popup, footer with model pill + key hints.

use std::time::Duration;

use agent_repl_core::{Density, Theme, ToolStyle};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::composer::state::{Composer, FieldLayout, MenuKind};
use crate::mascot::{Mascot, MascotState};
use crate::style::{color, fg};

/// Columns the field spends on its leading gutter before typed text: 2 of pad
/// plus the `❯ ` sigil (or, on continuation rows, an equal blank indent).
const PREFIX_COLS: u16 = 4;

/// What the composer needs to paint a mascot in its reserved right strip.
#[derive(Debug)]
pub struct MascotPaint<'a> {
    pub mascot: &'a dyn Mascot,
    pub state: MascotState,
    pub elapsed: Duration,
}

/// Inner width of the composer block for a given outer `area_width`: the full
/// width for the borderless [`ToolStyle::Inline`] frame, two columns less for
/// the fully-bordered styles.
fn inner_width(area_width: u16, theme: &Theme) -> u16 {
    match theme.tool_style {
        ToolStyle::Inline => area_width,
        _ => area_width.saturating_sub(2),
    }
}

/// Columns available for typed text per row, given the field's inner width:
/// the inner width minus the reserved right strip (e.g. for a mascot) minus the
/// leading gutter. Floored at one so wrapping always makes progress.
fn text_cols(inner_width: u16, reserved: u16) -> u16 {
    let reserved = reserved.min(inner_width.saturating_sub(1));
    inner_width
        .saturating_sub(reserved)
        .saturating_sub(PREFIX_COLS)
        .max(1)
}

/// Total rows the composer needs for a given outer `area_width` and theme.
/// Grows with the buffer — counting soft-wrapped rows, not just logical lines —
/// capped at [`MAX_VISIBLE_LINES`].
pub fn required_height(composer: &Composer, theme: &Theme, area_width: u16) -> u16 {
    let cols = text_cols(inner_width(area_width, theme), composer.reserved_right());
    let visible = composer.visible_line_count(cols) as u16;
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

    // Wrap the buffer to the columns left after the reserved strip + gutter, so
    // the field height (and scrolling) tracks soft-wrapped rows, not just the
    // logical line count.
    let cols = text_cols(inner.width, composer.reserved_right());
    let layout = composer.layout(cols);
    let visible = composer.clamp_visible(layout.rows.len()) as u16;
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

    draw_field(composer, theme, frame, text_area, &layout);
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
        MenuKind::Mode => "modes",
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

fn draw_field(
    composer: &Composer,
    theme: &Theme,
    frame: &mut Frame,
    area: Rect,
    layout: &FieldLayout,
) {
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
    let visible = area.height as usize;
    let show_placeholder = composer.is_empty() && !composer.working;
    // Scroll the wrapped rows to keep the cursor's row in view (the field only
    // follows the caret; there is no independent scroll), so derive the window
    // top from the cursor each frame.
    let scroll_top = scroll_top_for(layout.cursor_row, layout.rows.len(), visible);

    let mut rows: Vec<Line<'static>> = Vec::with_capacity(visible);
    for vi in 0..visible {
        let ri = scroll_top + vi;
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
        if let Some(vrow) = layout.rows.get(ri) {
            if is_first_row && show_placeholder {
                spans.push(Span::styled(placeholder.to_string(), placeholder_style));
            } else {
                // Style the whole logical line into cells (so an `@token` split
                // across a wrap keeps its chip styling on both rows), then take
                // just this visual row's slice.
                let cells = styled_cells(&lines[vrow.line], body_style, chip_style);
                let slice = &cells[vrow.start.min(cells.len())..vrow.end.min(cells.len())];
                spans.extend(cells_to_spans(slice));
            }
        }
        // Rows past the buffer stay blank so the field keeps its declared height.
        rows.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(Text::from(rows)), area);

    // The field stays editable while the agent works (type-ahead), so show the
    // cursor in both states — the "working" indicator is the spinner line above,
    // not a hidden cursor.
    {
        let cursor_vrow = layout.cursor_row.saturating_sub(scroll_top);
        if layout.cursor_row >= scroll_top && cursor_vrow < area.height as usize {
            // Every row reserves the same 4-col gutter, so the cursor's screen
            // column is the gutter plus its char offset within its visual row.
            let cx = area.x + PREFIX_COLS + layout.cursor_col as u16;
            let cy = area.y + cursor_vrow as u16;
            if cx < area.x + area.width {
                frame.set_cursor_position(Position { x: cx, y: cy });
            }
        }
    }
}

/// Topmost visual row to show so `cursor_row` stays inside a `height`-row
/// window. Everything fits ⇒ top is 0; otherwise the cursor is pinned no lower
/// than the last visible row and never scrolled past the end of the content.
fn scroll_top_for(cursor_row: usize, total: usize, height: usize) -> usize {
    if height == 0 || total <= height {
        return 0;
    }
    let max_top = total - height;
    cursor_row.saturating_sub(height - 1).min(max_top)
}

/// Style a logical line into per-char `(char, Style)` cells: `@token`s that
/// begin at a word boundary get the chip style, everything else the body style.
/// Working at cell granularity lets the renderer slice a token cleanly across a
/// soft-wrap boundary.
fn styled_cells(line: &str, body: Style, chip: Style) -> Vec<(char, Style)> {
    let chars: Vec<char> = line.chars().collect();
    let mut cells: Vec<(char, Style)> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let prev_is_boundary = i == 0 || chars[i - 1].is_whitespace();
        if chars[i] == '@' && prev_is_boundary {
            cells.push((chars[i], chip));
            let mut j = i + 1;
            while j < chars.len() && !chars[j].is_whitespace() {
                cells.push((chars[j], chip));
                j += 1;
            }
            i = j;
        } else {
            cells.push((chars[i], body));
            i += 1;
        }
    }
    cells
}

/// Coalesce a run of styled cells into the fewest spans.
fn cells_to_spans(cells: &[(char, Style)]) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut cur: Option<Style> = None;
    for &(c, style) in cells {
        match cur {
            Some(s) if s == style => buf.push(c),
            _ => {
                if let Some(s) = cur {
                    spans.push(Span::styled(std::mem::take(&mut buf), s));
                }
                buf.push(c);
                cur = Some(style);
            }
        }
    }
    if let Some(s) = cur {
        spans.push(Span::styled(buf, s));
    }
    spans
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
    if let Some(tokens) = &composer.tokens {
        left_spans.push(sep.clone());
        left_spans.push(Span::styled(format!("\u{26C1} {tokens}"), fg(p.text_dim)));
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
