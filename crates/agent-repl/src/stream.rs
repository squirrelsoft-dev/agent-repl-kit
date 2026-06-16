//! Append-only event stream + scroll/focus state. Renders to a single
//! `Text` per frame so a single ratatui `Paragraph` lays out the whole
//! transcript.

use std::collections::HashMap;

use agent_repl_core::{Event, Theme, ToolCall, ToolStyle};
use ratatui::text::{Line, Text};

use crate::blocks;
use crate::decorations::Decorations;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToolId(pub u64);

/// Per-item UI state that lives alongside the immutable event payload.
#[derive(Debug, Default, Clone)]
pub struct ItemUi {
    /// User-explicit expansion override. Only consulted in `ToolStyle::Collapsed`.
    /// `Some(true)` = pinned open, `Some(false)` = pinned closed.
    pub user_expanded: Option<bool>,
}

#[derive(Debug, Default)]
pub struct Stream {
    items: Vec<Event>,
    item_ui: Vec<ItemUi>,
    tool_index: HashMap<u64, usize>,
    /// `Some` = user has scrolled up and pinned position. `None` = follow tail.
    scroll: Option<u16>,
    /// Index of the currently focused tool item (Tab cycles through tools).
    focus_idx: Option<usize>,
}

impl Stream {
    pub fn push(&mut self, event: Event) {
        self.items.push(event);
        self.item_ui.push(ItemUi::default());
    }

    pub fn push_tool(&mut self, id: ToolId, call: ToolCall) {
        let idx = self.items.len();
        self.tool_index.insert(id.0, idx);
        self.items.push(Event::Tool(call));
        self.item_ui.push(ItemUi::default());
    }

    pub fn update_tool(&mut self, id: ToolId, call: ToolCall) {
        if let Some(&idx) = self.tool_index.get(&id.0) {
            self.items[idx] = Event::Tool(call);
            // Preserve `item_ui[idx]` — user's expansion choice survives updates.
        }
    }

    // ---- focus navigation (collapsed-style only, but harmless elsewhere) ----

    /// Move focus to the next tool item, wrapping. No-op when no tool items.
    pub fn focus_next(&mut self) {
        let n = self.items.len();
        if n == 0 {
            return;
        }
        let start = self.focus_idx.map(|i| (i + 1) % n).unwrap_or(0);
        for offset in 0..n {
            let idx = (start + offset) % n;
            if matches!(self.items[idx], Event::Tool(_)) {
                self.focus_idx = Some(idx);
                return;
            }
        }
        // No tool items found.
        self.focus_idx = None;
    }

    /// Move focus to the previous tool item, wrapping.
    pub fn focus_prev(&mut self) {
        let n = self.items.len();
        if n == 0 {
            return;
        }
        let start = match self.focus_idx {
            Some(i) if i > 0 => i - 1,
            Some(_) => n - 1,
            None => n - 1,
        };
        for offset in 0..n {
            let idx = (start + n - offset) % n;
            if matches!(self.items[idx], Event::Tool(_)) {
                self.focus_idx = Some(idx);
                return;
            }
        }
        self.focus_idx = None;
    }

    pub fn clear_focus(&mut self) {
        self.focus_idx = None;
    }

    pub fn focused_idx(&self) -> Option<usize> {
        self.focus_idx
    }

    /// Toggle `user_expanded` on the focused item (no-op if no focus).
    pub fn toggle_focused_expansion(&mut self) {
        if let Some(idx) = self.focus_idx {
            let current = self.is_user_expanded(idx).unwrap_or(false);
            self.item_ui[idx].user_expanded = Some(!current);
        }
    }

    fn is_user_expanded(&self, idx: usize) -> Option<bool> {
        self.item_ui.get(idx).and_then(|u| u.user_expanded)
    }

    /// Should a tool block at `idx` show its body?
    ///
    /// - Non-collapsed styles: always (mirrors JSX).
    /// - Collapsed + running: always (force open while in flight).
    /// - Collapsed + finished: only when user has pinned open.
    fn is_expanded(&self, idx: usize, theme: &Theme) -> bool {
        if theme.tool_style != ToolStyle::Collapsed {
            return true;
        }
        if let Event::Tool(call) = &self.items[idx] {
            if call.running {
                return true;
            }
            return self.item_ui[idx].user_expanded.unwrap_or(false);
        }
        true
    }

    // ---- rendering ----

    pub fn render(
        &self,
        theme: &Theme,
        deco: &Decorations,
        spinner_frame: char,
    ) -> Text<'static> {
        let gap = theme.spacing.gap as usize + 1;
        let outer_pad = "  "; // 2-column left breathing room for every block
        let mut all: Vec<Line<'static>> = Vec::new();
        for (i, ev) in self.items.iter().enumerate() {
            if i > 0 {
                for _ in 0..gap {
                    all.push(Line::raw(""));
                }
            }
            let expanded = self.is_expanded(i, theme);
            let focused = self.focus_idx == Some(i);
            for raw in blocks::render(ev, theme, deco, spinner_frame, expanded, focused) {
                let mut spans: Vec<ratatui::text::Span<'static>> =
                    vec![ratatui::text::Span::raw(outer_pad.to_string())];
                spans.extend(raw.spans);
                all.push(Line::from(spans));
            }
        }
        Text::from(all)
    }

    // ---- scroll ----

    pub fn scroll_offset(&self, content_height: u16, viewport_height: u16) -> u16 {
        let max = content_height.saturating_sub(viewport_height);
        match self.scroll {
            Some(s) => s.min(max),
            None => max, // follow tail
        }
    }

    pub fn scroll_up(&mut self, n: u16, current: u16) {
        self.scroll = Some(current.saturating_sub(n));
    }

    pub fn scroll_down(&mut self, n: u16, current: u16, max: u16) {
        let next = current.saturating_add(n).min(max);
        if next >= max {
            self.scroll = None;
        } else {
            self.scroll = Some(next);
        }
    }

    pub fn jump_top(&mut self) {
        self.scroll = Some(0);
    }

    pub fn jump_bottom(&mut self) {
        self.scroll = None;
    }

    pub fn is_following(&self) -> bool {
        self.scroll.is_none()
    }

    // ---- introspection (testing) ----

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
