//! Hanging-indent word wrap for framed transcript lines.
//!
//! Every block line is built as `outer-pad + gutter + content` (see
//! [`crate::blocks::frame`] and [`crate::stream::Stream::render`]). ratatui's
//! own [`Wrap`](ratatui::widgets::Wrap) restarts each wrapped row at column 0,
//! which drops the gutter and lets continuation text spill into the border
//! column. There is no ratatui knob for a hanging indent, so we pre-wrap the
//! lines ourselves: continuation rows reproduce the leading frame (a vertical
//! `│` border plus padding) so wrapped text stays aligned under the content of
//! its box. The caller then renders with wrapping disabled.

use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};

/// Characters that count as part of a line's leading "frame" prefix: box
/// drawing, chevrons, and whitespace. A leading span made up entirely of these
/// is treated as gutter/indent rather than content.
fn is_frame_char(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '│' | '┃' | '╎' | '╏' | '┊' | '┋'
                | '╭' | '╮' | '╰' | '╯'
                | '┌' | '┐' | '└' | '┘'
                | '├' | '┤' | '┬' | '┴' | '┼'
                | '─' | '━'
                | '▸' | '▾' | '▴' | '◂' | '▹' | '▿'
        )
}

/// A vertical border / corner glyph — its presence means a continuation row
/// should keep a `│` in that column instead of a blank.
fn is_vertical(c: char) -> bool {
    matches!(
        c,
        '│' | '┃' | '╎' | '╏' | '┊' | '┋'
            | '╭' | '╮' | '╰' | '╯'
            | '┌' | '┐' | '└' | '┘'
            | '├' | '┤'
    )
}

/// True if every char in `s` is a frame char (so the span is gutter/indent).
fn is_frame_span(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_frame_char)
}

fn span_width(s: &Span) -> usize {
    s.content.chars().count()
}

/// Build the continuation form of a gutter span: a vertical border becomes a
/// `│` followed by padding; pure indent stays blank. Width is preserved.
fn continuation_of(span: &Span<'static>) -> Span<'static> {
    let w = span_width(span);
    let has_vertical = span.content.chars().any(is_vertical);
    let text = if has_vertical {
        let mut s = String::with_capacity(w);
        s.push('│');
        s.push_str(&" ".repeat(w.saturating_sub(1)));
        s
    } else {
        " ".repeat(w)
    };
    Span::styled(text, span.style)
}

/// Coalesce a run of `(char, Style)` cells into the fewest spans.
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

/// Greedily split content `cells` into segments no wider than `width`,
/// breaking on spaces where possible and hard-breaking over-long tokens.
fn split_segments(cells: &[(char, Style)], width: usize) -> Vec<Vec<(char, Style)>> {
    let n = cells.len();
    let mut segments = Vec::new();
    let mut i = 0;
    while i < n {
        if n - i <= width {
            segments.push(cells[i..].to_vec());
            break;
        }
        let hard_end = i + width;
        // Prefer the last space within the window (but past the start).
        let brk = (i + 1..hard_end).rev().find(|&j| cells[j].0 == ' ');
        let (end, next) = match brk {
            Some(j) => (j, j + 1), // break at the space, drop it
            None => (hard_end, hard_end),
        };
        segments.push(cells[i..end].to_vec());
        i = next;
    }
    if segments.is_empty() {
        segments.push(Vec::new());
    }
    segments
}

/// Wrap one line to `width` columns, preserving the leading frame on every
/// continuation row. Lines that already fit are returned unchanged.
pub fn wrap_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![line];
    }
    let total: usize = line.spans.iter().map(span_width).sum();
    if total <= width {
        return vec![line];
    }

    // Split off the leading gutter/indent spans from the content spans.
    let split = line
        .spans
        .iter()
        .position(|s| !is_frame_span(&s.content))
        .unwrap_or(line.spans.len());
    let prefix = &line.spans[..split];
    let content = &line.spans[split..];

    let prefix_width: usize = prefix.iter().map(span_width).sum();
    let content_width = width.saturating_sub(prefix_width);
    // Nothing sensible to do if the frame already fills the row.
    if content_width < 2 || content.is_empty() {
        return vec![line];
    }

    let cells: Vec<(char, Style)> = content
        .iter()
        .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
        .collect();
    let segments = split_segments(&cells, content_width);

    let prefix_spans: Vec<Span<'static>> = prefix.to_vec();
    let cont_spans: Vec<Span<'static>> = prefix.iter().map(continuation_of).collect();

    let mut out = Vec::with_capacity(segments.len());
    for (i, seg) in segments.iter().enumerate() {
        let mut spans = if i == 0 {
            prefix_spans.clone()
        } else {
            cont_spans.clone()
        };
        spans.extend(cells_to_spans(seg));
        out.push(Line::from(spans));
    }
    out
}

/// Wrap every line of `text` to `width`, preserving each line's frame on its
/// continuation rows.
pub fn wrap_text(text: Text<'static>, width: u16) -> Text<'static> {
    let width = width as usize;
    let mut lines = Vec::with_capacity(text.lines.len());
    for line in text.lines {
        lines.extend(wrap_line(line, width));
    }
    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed_line(gutter: &str, content: &str) -> Line<'static> {
        Line::from(vec![
            Span::raw("  ".to_string()),
            Span::styled(gutter.to_string(), Style::default()),
            Span::raw(content.to_string()),
        ])
    }

    fn text_of(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn short_line_is_unchanged() {
        let line = framed_line("│ ", "hi there");
        let out = wrap_line(line.clone(), 80);
        assert_eq!(out.len(), 1);
        assert_eq!(text_of(&out[0]), text_of(&line));
    }

    #[test]
    fn continuation_rows_repeat_the_gutter() {
        let line = framed_line("│ ", "the quick brown fox jumps over the lazy dog");
        let out = wrap_line(line, 18);
        assert!(out.len() >= 3, "expected multiple rows, got {}", out.len());
        // Every row keeps the "  │ " frame; content never starts in column 0.
        for row in &out {
            assert!(text_of(row).starts_with("  │ "), "row lost gutter: {:?}", text_of(row));
        }
        // The words survive the round-trip (minus the break spaces).
        let joined: String = out
            .iter()
            .map(|r| text_of(r).trim_start_matches("  │ ").trim_end().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(joined, "the quick brown fox jumps over the lazy dog");
    }

    #[test]
    fn card_top_corner_continues_as_vertical_border() {
        // A wrapped card header: first row keeps "╭─ ", continuations get "│  ".
        let line = framed_line("╭─ ", "a fairly long header line that must wrap somewhere");
        let out = wrap_line(line, 20);
        assert!(out.len() >= 2);
        assert!(text_of(&out[0]).starts_with("  ╭─ "));
        for row in &out[1..] {
            assert!(text_of(row).starts_with("  │  "), "bad cont: {:?}", text_of(row));
        }
    }

    #[test]
    fn long_unbroken_token_is_hard_split() {
        let line = framed_line("│ ", "supercalifragilisticexpialidocious");
        let out = wrap_line(line, 14); // content budget 10
        assert!(out.len() >= 3);
        for row in &out {
            assert!(text_of(row).starts_with("  │ "));
        }
    }

    #[test]
    fn pure_indent_continuation_stays_blank() {
        // Reasoning-style body: leading indent, no border glyph.
        let line = Line::from(vec![
            Span::raw("    ".to_string()),
            Span::raw("alpha beta gamma delta epsilon zeta eta theta".to_string()),
        ]);
        let out = wrap_line(line, 16);
        assert!(out.len() >= 2);
        for row in &out {
            assert!(text_of(row).starts_with("    "), "lost indent: {:?}", text_of(row));
            // No stray border was invented for a borderless indent.
            assert!(!text_of(row).contains('│'));
        }
    }

    #[test]
    fn zero_width_is_a_noop() {
        let line = framed_line("│ ", "anything at all here");
        assert_eq!(wrap_line(line.clone(), 0).len(), 1);
    }
}
