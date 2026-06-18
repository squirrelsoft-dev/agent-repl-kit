//! Renderer + interaction state for the agent's question box — a tabbed form
//! that floats above the composer, mirroring the permissions box.
//!
//! The agent hands over a [`QuestionForm`] (see `agent_repl_core::question`);
//! [`QuestionState`] owns the live interaction (cursor, selections, freeform
//! text, active tab) and turns key events into either a redraw or a finished
//! [`FormAnswers`].
//!
//! Each question is a tab. With more than one question a trailing **Submit**
//! tab is appended where the user reviews every answer before sending.
//!
//! Keys (while the box is up):
//! * `↑`/`↓` — move the row cursor
//! * `Space` — toggle (multi) / pick (single) the cursor row
//! * `Enter` — single: pick the row; then advance to the next tab, or submit on
//!   the last tab
//! * `Tab`/`→`, `Shift-Tab`/`←` — switch tabs
//! * typing — edits the freeform ("Other" / custom) row when it is focused
//! * `Esc` — cancel (handled by the app as an abort)

use std::collections::BTreeSet;

use agent_repl_core::{
    FormAnswers, QuestionAnswer, QuestionForm, QuestionKind, Theme,
};
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::style::{color, fg};

/// What a key press did to the form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionAction {
    /// Redraw; the form is still open.
    Continue,
    /// The user submitted; here are the collected answers.
    Submit(FormAnswers),
}

/// Live interaction state for one question (tab).
#[derive(Debug, Default, Clone)]
struct TabState {
    /// Highlighted row (`0..row_count`).
    cursor: usize,
    /// Single-select: the picked row, if any.
    chosen: Option<usize>,
    /// Multi-select: the toggled rows.
    toggled: BTreeSet<usize>,
    /// Text typed into the freeform row.
    freeform_text: String,
}

/// A tabbed question form plus the user's in-progress answers.
#[derive(Debug, Clone)]
pub struct QuestionState {
    form: QuestionForm,
    tabs: Vec<TabState>,
    /// Active tab (`0..tab_count`). May be the synthetic submit tab.
    active: usize,
}

impl QuestionState {
    /// Build interaction state for `form`. Caller guarantees a non-empty form.
    pub fn new(form: QuestionForm) -> Self {
        let tabs = form.questions.iter().map(|_| TabState::default()).collect();
        Self { form, tabs, active: 0 }
    }

    // ---- tab math --------------------------------------------------------

    fn question_count(&self) -> usize {
        self.form.questions.len()
    }

    /// A submit tab is only added when there is more than one question.
    fn has_submit_tab(&self) -> bool {
        self.question_count() > 1
    }

    fn tab_count(&self) -> usize {
        self.question_count() + usize::from(self.has_submit_tab())
    }

    fn submit_tab_index(&self) -> usize {
        self.question_count()
    }

    fn on_submit_tab(&self) -> bool {
        self.has_submit_tab() && self.active == self.submit_tab_index()
    }

    fn last_tab_index(&self) -> usize {
        self.tab_count().saturating_sub(1)
    }

    fn is_last_tab(&self, i: usize) -> bool {
        i == self.last_tab_index()
    }

    fn row_count(&self, q: usize) -> usize {
        let q = &self.form.questions[q];
        q.options.len() + usize::from(q.freeform.is_some())
    }

    fn next_tab(&mut self) {
        let n = self.tab_count();
        self.active = (self.active + 1) % n;
    }

    fn prev_tab(&mut self) {
        let n = self.tab_count();
        self.active = (self.active + n - 1) % n;
    }

    // ---- key handling ----------------------------------------------------

    /// Feed one key to the form. The app intercepts `Esc` (→ abort) before
    /// calling this, so `Esc` is not handled here.
    pub fn handle_key(&mut self, code: KeyCode) -> QuestionAction {
        match code {
            KeyCode::Tab | KeyCode::Right => {
                self.next_tab();
                QuestionAction::Continue
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.prev_tab();
                QuestionAction::Continue
            }
            _ if self.on_submit_tab() => {
                if matches!(code, KeyCode::Enter) {
                    QuestionAction::Submit(self.answers())
                } else {
                    QuestionAction::Continue
                }
            }
            _ => self.handle_question_key(code),
        }
    }

    fn handle_question_key(&mut self, code: KeyCode) -> QuestionAction {
        let active = self.active;
        let q = &self.form.questions[active];
        let kind = q.kind;
        let has_freeform = q.freeform.is_some();
        let freeform_row = q.options.len();
        let rows = self.row_count(active);
        let on_freeform = has_freeform && self.tabs[active].cursor == freeform_row;

        match code {
            KeyCode::Up => {
                let t = &mut self.tabs[active];
                t.cursor = (t.cursor + rows - 1) % rows;
            }
            KeyCode::Down => {
                let t = &mut self.tabs[active];
                t.cursor = (t.cursor + 1) % rows;
            }
            KeyCode::Backspace if on_freeform => {
                self.tabs[active].freeform_text.pop();
            }
            // Any printable char (incl. space) edits the focused freeform row.
            KeyCode::Char(c) if on_freeform => {
                let t = &mut self.tabs[active];
                t.freeform_text.push(c);
                match kind {
                    QuestionKind::Single => t.chosen = Some(freeform_row),
                    QuestionKind::Multi => {
                        t.toggled.insert(freeform_row);
                    }
                }
            }
            // Space toggles / picks the (non-freeform) cursor row.
            KeyCode::Char(' ') => {
                let cur = self.tabs[active].cursor;
                match kind {
                    QuestionKind::Single => self.tabs[active].chosen = Some(cur),
                    QuestionKind::Multi => {
                        let t = &mut self.tabs[active];
                        if !t.toggled.insert(cur) {
                            t.toggled.remove(&cur);
                        }
                    }
                }
            }
            KeyCode::Enter => {
                if kind == QuestionKind::Single {
                    let cur = self.tabs[active].cursor;
                    self.tabs[active].chosen = Some(cur);
                }
                if self.is_last_tab(active) {
                    return QuestionAction::Submit(self.answers());
                }
                self.active += 1;
            }
            _ => {}
        }
        QuestionAction::Continue
    }

    // ---- answers ---------------------------------------------------------

    fn answer_for(&self, i: usize) -> QuestionAnswer {
        let q = &self.form.questions[i];
        let t = &self.tabs[i];
        let freeform_row = q.options.len();
        match q.kind {
            QuestionKind::Single => match t.chosen {
                Some(r) if q.freeform.is_some() && r == freeform_row => {
                    let txt = t.freeform_text.trim();
                    QuestionAnswer::Single {
                        option: None,
                        other: (!txt.is_empty()).then(|| txt.to_string()),
                    }
                }
                Some(r) if r < q.options.len() => {
                    QuestionAnswer::Single { option: Some(r), other: None }
                }
                _ => QuestionAnswer::Single { option: None, other: None },
            },
            QuestionKind::Multi => {
                let mut options: Vec<usize> = t
                    .toggled
                    .iter()
                    .copied()
                    .filter(|&r| r < q.options.len())
                    .collect();
                options.sort_unstable();
                let custom = if q.freeform.is_some() && t.toggled.contains(&freeform_row) {
                    let txt = t.freeform_text.trim();
                    (!txt.is_empty()).then(|| txt.to_string())
                } else {
                    None
                };
                QuestionAnswer::Multi { options, custom }
            }
        }
    }

    /// The collected answers so far, one per question.
    pub fn answers(&self) -> FormAnswers {
        FormAnswers::new((0..self.question_count()).map(|i| self.answer_for(i)).collect())
    }

    // ---- status hint -----------------------------------------------------

    /// A one-line key hint for the status bar, contextual to the active tab.
    pub fn status_hint(&self) -> String {
        let sep = " \u{00B7} ";
        if self.on_submit_tab() {
            return format!("\u{23CE} submit{sep}\u{2190}\u{2192} tabs{sep}Esc cancel");
        }
        let enter = if self.is_last_tab(self.active) { "submit" } else { "next" };
        let q = &self.form.questions[self.active];
        let mut h = String::from("\u{2191}\u{2193} move");
        match q.kind {
            QuestionKind::Single => {
                h.push_str(sep);
                h.push_str(&format!("\u{23CE} pick & {enter}"));
            }
            QuestionKind::Multi => {
                h.push_str(sep);
                h.push_str("Space toggle");
                h.push_str(sep);
                h.push_str(&format!("\u{23CE} {enter}"));
            }
        }
        if self.tab_count() > 1 {
            h.push_str(sep);
            h.push_str("\u{2190}\u{2192} tabs");
        }
        h.push_str(sep);
        h.push_str("Esc cancel");
        h
    }

    // ---- rendering -------------------------------------------------------

    /// Total rows the box needs for the active tab (content + borders).
    pub fn required_height(&self) -> u16 {
        self.content_lines_len() as u16 + 2
    }

    fn content_lines_len(&self) -> usize {
        let tabbar = usize::from(self.tab_count() > 1);
        if self.on_submit_tab() {
            // blank + header + blank + one per question + blank + submit prompt
            tabbar + 1 + 1 + 1 + self.question_count() + 1 + 1
        } else {
            let q = &self.form.questions[self.active];
            let detail = usize::from(q.detail.is_some());
            // blank + title + detail + blank + rows
            tabbar + 1 + 1 + detail + 1 + self.row_count(self.active)
        }
    }

    /// Render the box into `area`.
    pub fn render(&self, theme: &Theme, frame: &mut Frame, area: Rect) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let p = &theme.palette;

        let title = self
            .form
            .intro
            .clone()
            .unwrap_or_else(|| "a question for you".to_string());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(fg(p.accent).add_modifier(Modifier::BOLD))
            .title(Span::styled(
                format!(" {title} "),
                fg(p.accent).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(color(p.bg_raised)));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.height == 0 {
            return;
        }

        let mut lines: Vec<Line<'static>> = Vec::new();
        if self.tab_count() > 1 {
            lines.push(self.tab_bar(theme));
        }
        if self.on_submit_tab() {
            self.push_submit_lines(theme, &mut lines);
        } else {
            self.push_question_lines(theme, &mut lines);
        }

        frame.render_widget(Paragraph::new(Text::from(lines)), inner);
    }

    fn tab_bar(&self, theme: &Theme) -> Line<'static> {
        let p = &theme.palette;
        let mut spans = vec![Span::raw("  ".to_string())];
        for i in 0..self.question_count() {
            let active = self.active == i;
            let answered = !self.answer_for(i).is_empty();
            let mark = if answered { "\u{2713}" } else { " " };
            let label = format!(" {}{} ", i + 1, mark);
            spans.push(chip(label, active, p));
            spans.push(Span::raw(" ".to_string()));
        }
        if self.has_submit_tab() {
            spans.push(Span::styled("\u{2502} ".to_string(), fg(p.border)));
            spans.push(chip(" Submit ".to_string(), self.on_submit_tab(), p));
        }
        Line::from(spans)
    }

    fn push_question_lines(&self, theme: &Theme, lines: &mut Vec<Line<'static>>) {
        let p = &theme.palette;
        let q = &self.form.questions[self.active];
        let t = &self.tabs[self.active];

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  ".to_string()),
            Span::styled(q.title.clone(), fg(p.text).add_modifier(Modifier::BOLD)),
        ]));
        if let Some(detail) = &q.detail {
            lines.push(Line::from(vec![
                Span::raw("  ".to_string()),
                Span::styled(detail.clone(), fg(p.text_dim)),
            ]));
        }
        lines.push(Line::raw(""));

        let freeform_row = q.options.len();
        for row in 0..self.row_count(self.active) {
            let is_cursor = t.cursor == row;
            let is_freeform = q.freeform.is_some() && row == freeform_row;
            let selected = match q.kind {
                QuestionKind::Single => t.chosen == Some(row),
                QuestionKind::Multi => t.toggled.contains(&row),
            };
            let glyph = match q.kind {
                QuestionKind::Single => {
                    if selected {
                        "\u{25C9}"
                    } else {
                        "\u{25CB}"
                    }
                }
                QuestionKind::Multi => {
                    if selected {
                        "\u{2611}"
                    } else {
                        "\u{2610}"
                    }
                }
            };

            let marker = if is_cursor { "\u{276F} " } else { "  " };
            let glyph_style = if selected {
                fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                fg(p.text_faint)
            };
            let label_style = if selected {
                fg(p.accent).add_modifier(Modifier::BOLD)
            } else if is_cursor {
                fg(p.text)
            } else {
                fg(p.text_dim)
            };

            let mut spans = vec![
                Span::raw("  ".to_string()),
                Span::styled(marker.to_string(), fg(p.accent).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{glyph} "), glyph_style),
            ];

            if is_freeform {
                let label = q.freeform.clone().unwrap_or_default();
                spans.push(Span::styled(format!("{label}: "), label_style));
                if t.freeform_text.is_empty() {
                    let placeholder = match q.kind {
                        QuestionKind::Single => "type your answer\u{2026}",
                        QuestionKind::Multi => "type a message\u{2026}",
                    };
                    spans.push(Span::styled(
                        placeholder.to_string(),
                        fg(p.text_faint).add_modifier(Modifier::ITALIC),
                    ));
                } else {
                    spans.push(Span::styled(t.freeform_text.clone(), fg(p.text)));
                }
                if is_cursor {
                    spans.push(Span::styled(
                        "\u{2588}".to_string(),
                        fg(p.accent).add_modifier(Modifier::SLOW_BLINK),
                    ));
                }
            } else {
                spans.push(Span::styled(q.options[row].clone(), label_style));
            }
            lines.push(Line::from(spans));
        }
    }

    fn push_submit_lines(&self, theme: &Theme, lines: &mut Vec<Line<'static>>) {
        let p = &theme.palette;
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  ".to_string()),
            Span::styled(
                "Review your answers".to_string(),
                fg(p.text).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::raw(""));
        for (i, q) in self.form.questions.iter().enumerate() {
            let answer = self.answer_for(i).describe(q);
            let mut spans = vec![
                Span::raw("  ".to_string()),
                Span::styled(format!("{}. ", i + 1), fg(p.accent).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}  ", q.title), fg(p.text_dim)),
            ];
            match answer {
                Some(a) => spans.push(Span::styled(a, fg(p.text).add_modifier(Modifier::BOLD))),
                None => spans.push(Span::styled(
                    "(no answer)".to_string(),
                    fg(p.text_faint).add_modifier(Modifier::ITALIC),
                )),
            }
            lines.push(Line::from(spans));
        }
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  ".to_string()),
            Span::styled(
                "Press \u{23CE} to submit".to_string(),
                fg(p.success).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
}

fn chip(label: String, active: bool, p: &agent_repl_core::Palette) -> Span<'static> {
    if active {
        Span::styled(
            label,
            Style::default()
                .fg(color(p.accent))
                .bg(color(p.accent_soft))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label, fg(p.text_dim))
    }
}
