//! Tiny markdown → ratatui Lines. Mirrors the subset supported by
//! `docs/repl/markdown.jsx`: paragraphs, headings, fenced code, blockquotes,
//! lists (- / *, 1.), inline **bold**, *italic*, `code`, [text](url).
//!
//! pulldown-cmark would handle this, but we want bit-for-bit parity with
//! the JSX so the demo matches the HTML reference visually.

use agent_repl_core::Palette;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::style::{color, fg, fg_bg};

pub fn render(text: &str, palette: &Palette) -> Vec<Line<'static>> {
    let body = fg(palette.text);
    let dim = fg(palette.text_dim);
    let faint = fg(palette.text_faint);
    let code_style = Style::default()
        .fg(color(palette.accent))
        .bg(color(palette.bg_inset));
    let pre_style = fg_bg(palette.text, palette.bg_inset);
    let quote_style = fg(palette.text_dim);
    let link_style = fg(palette.accent).add_modifier(Modifier::UNDERLINED);

    let mut out: Vec<Line<'static>> = Vec::new();
    let lines: Vec<&str> = text.split('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // fenced code block ```
        if line.trim_start().starts_with("```") {
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                let s = lines[i].to_string();
                out.push(Line::from(vec![
                    Span::styled("  ".to_string(), faint),
                    Span::styled(s, pre_style),
                ]));
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            } // skip closing ```
            continue;
        }
        // headings
        if let Some(rest) = strip_heading(line) {
            let (level, body_text) = rest;
            let prefix = "#".repeat(level);
            let mut spans = vec![Span::styled(format!("{prefix} "), faint)];
            spans.extend(inline(body_text, palette, body.add_modifier(Modifier::BOLD), code_style, link_style));
            out.push(Line::from(spans));
            i += 1;
            continue;
        }
        // blockquote
        if line.trim_start().starts_with('>') {
            let mut buf = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                let stripped = strip_quote(lines[i]);
                buf.push(stripped.to_string());
                i += 1;
            }
            for piece in buf {
                let mut spans = vec![Span::styled("┃ ".to_string(), fg(palette.accent_soft))];
                spans.extend(inline(&piece, palette, quote_style, code_style, link_style));
                out.push(Line::from(spans));
            }
            continue;
        }
        // unordered list
        if let Some(item) = strip_bullet(line) {
            let mut spans = vec![Span::styled("• ".to_string(), fg(palette.accent))];
            spans.extend(inline(item, palette, body, code_style, link_style));
            out.push(Line::from(spans));
            i += 1;
            continue;
        }
        // ordered list
        if let Some((n, item)) = strip_ordered(line) {
            let mut spans = vec![Span::styled(format!("{n}. "), fg(palette.accent))];
            spans.extend(inline(item, palette, body, code_style, link_style));
            out.push(Line::from(spans));
            i += 1;
            continue;
        }
        // blank
        if line.trim().is_empty() {
            out.push(Line::raw(""));
            i += 1;
            continue;
        }
        // paragraph: gather until blank or block-start
        let mut buf = vec![line];
        i += 1;
        while i < lines.len() && !is_block_start(lines[i]) {
            buf.push(lines[i]);
            i += 1;
        }
        // emit each gathered raw line as its own Line, joined inline by Paragraph wrap
        for piece in buf {
            let spans = inline(piece, palette, body, code_style, link_style);
            out.push(Line::from(spans));
        }
        let _ = dim; // silence unused
    }
    out
}

fn is_block_start(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```")
        || t.is_empty()
        || strip_heading(line).is_some()
        || t.starts_with('>')
        || strip_bullet(line).is_some()
        || strip_ordered(line).is_some()
}

fn strip_heading(line: &str) -> Option<(usize, &str)> {
    let mut chars = line.chars();
    let mut level = 0;
    while let Some('#') = chars.clone().next() {
        chars.next();
        level += 1;
        if level == 4 {
            break;
        }
    }
    if level == 0 {
        return None;
    }
    let rest = chars.as_str();
    let stripped = rest.strip_prefix(' ')?;
    Some((level, stripped))
}

fn strip_quote(line: &str) -> &str {
    let trimmed = line.trim_start();
    let after = trimmed.strip_prefix('>').unwrap_or(trimmed);
    after.strip_prefix(' ').unwrap_or(after)
}

fn strip_bullet(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for prefix in ["- ", "* "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

fn strip_ordered(line: &str) -> Option<(u32, &str)> {
    let trimmed = line.trim_start();
    let dot = trimmed.find('.')?;
    let (num, rest) = trimmed.split_at(dot);
    let n: u32 = num.parse().ok()?;
    let rest = rest.strip_prefix('.')?.strip_prefix(' ')?;
    Some((n, rest))
}

fn inline(
    text: &str,
    _palette: &Palette,
    body: Style,
    code: Style,
    link: Style,
) -> Vec<Span<'static>> {
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut bold = false;
    let mut italic = false;
    let mut chars = text.chars().peekable();
    let flush = |buf: &mut String, out: &mut Vec<Span<'static>>, bold: bool, italic: bool, body: Style| {
        if buf.is_empty() {
            return;
        }
        let mut s = body;
        if bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        if italic {
            s = s.add_modifier(Modifier::ITALIC);
        }
        out.push(Span::styled(std::mem::take(buf), s));
    };
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                flush(&mut buf, &mut out, bold, italic, body);
                bold = !bold;
            }
            '*' => {
                flush(&mut buf, &mut out, bold, italic, body);
                italic = !italic;
            }
            '`' => {
                flush(&mut buf, &mut out, bold, italic, body);
                let mut code_buf = String::new();
                while let Some(&n) = chars.peek() {
                    if n == '`' {
                        chars.next();
                        break;
                    }
                    code_buf.push(n);
                    chars.next();
                }
                out.push(Span::styled(code_buf, code));
            }
            '[' => {
                // attempt to parse [text](url)
                let mut label = String::new();
                let mut closed = false;
                while let Some(&n) = chars.peek() {
                    if n == ']' {
                        chars.next();
                        closed = true;
                        break;
                    }
                    label.push(n);
                    chars.next();
                }
                if closed && chars.peek() == Some(&'(') {
                    chars.next();
                    let mut _url = String::new();
                    while let Some(&n) = chars.peek() {
                        if n == ')' {
                            chars.next();
                            break;
                        }
                        _url.push(n);
                        chars.next();
                    }
                    flush(&mut buf, &mut out, bold, italic, body);
                    out.push(Span::styled(label, link));
                } else {
                    // not a link; emit as literal
                    buf.push('[');
                    buf.push_str(&label);
                    if closed {
                        buf.push(']');
                    }
                }
            }
            _ => buf.push(c),
        }
    }
    flush(&mut buf, &mut out, bold, italic, body);
    out
}
