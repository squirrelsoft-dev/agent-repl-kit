//! Composer state and edit operations. Multi-line buffer with cursor,
//! scroll window, and slash / `@file` menu state derived from the buffer.

use crossterm::event::{KeyCode, KeyModifiers};

/// Maximum number of buffer rows the composer field shows at once.
/// Adding lines beyond this scrolls the field, not the composer chrome.
pub const MAX_VISIBLE_LINES: usize = 10;

/// Slash commands offered by the popup menu. Sourced from
/// `docs/repl/input.jsx` (the `SLASH` constant).
pub const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/clear", "Clear the conversation"),
    ("/compact", "Summarize & free up context"),
    ("/model", "Switch the active model"),
    ("/diff", "Review pending changes"),
    ("/undo", "Revert the last edit"),
    ("/run", "Run a shell command"),
];

#[derive(Debug, Clone)]
pub struct Composer {
    /// One entry per logical line. Always at least one (possibly empty) line.
    lines: Vec<String>,
    /// Cursor line index into `lines` (0-based).
    cursor_line: usize,
    /// Cursor column as a *char* index into `lines[cursor_line]`.
    cursor_col: usize,
    /// Top of the visible window inside the field.
    scroll_top: usize,
    /// Currently selected row in the active menu (slash or `@file`).
    /// Always clamped to the filtered item list.
    menu_selected: usize,
    /// Whether the agent is currently working. Field locks while `true`.
    pub working: bool,
    /// Footer context: model name shown as a pill.
    pub model: String,
    /// Footer context: cwd shorthand (e.g. `"~/project"`).
    pub cwd: String,
    /// Footer context: git branch if known.
    pub branch: Option<String>,
    /// Pool of file names available for `@file` completion.
    pub file_completions: Vec<String>,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll_top: 0,
            menu_selected: 0,
            working: false,
            model: "agent".into(),
            cwd: String::new(),
            branch: None,
            file_completions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    Slash,
    At,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub value: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerAction {
    /// Key consumed; nothing else to do.
    Consumed,
    /// User pressed Enter on a non-empty buffer — here's the submitted text.
    Submit(String),
    /// Key wasn't claimed by the composer; caller may handle it (e.g. theme
    /// cycling, scroll, quit).
    PassThrough,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    // ---- footer setters / accessors ----

    pub fn set_model(&mut self, m: impl Into<String>) {
        self.model = m.into();
    }
    pub fn set_cwd(&mut self, c: impl Into<String>) {
        self.cwd = c.into();
    }
    pub fn set_branch(&mut self, b: Option<String>) {
        self.branch = b;
    }
    pub fn set_working(&mut self, w: bool) {
        self.working = w;
    }
    pub fn set_file_completions(&mut self, files: Vec<String>) {
        self.file_completions = files;
        self.clamp_menu_selected();
    }

    // ---- buffer accessors ----

    pub fn lines(&self) -> &[String] {
        &self.lines
    }
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    pub fn visible_line_count(&self) -> usize {
        self.lines.len().clamp(1, MAX_VISIBLE_LINES)
    }
    pub fn scroll_top(&self) -> usize {
        self.scroll_top
    }
    pub fn cursor_line(&self) -> usize {
        self.cursor_line
    }
    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.is_empty())
    }
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_top = 0;
        self.menu_selected = 0;
    }

    // ---- menu derivation ----

    /// Which menu (if any) should show given the current buffer state.
    pub fn menu_kind(&self) -> Option<MenuKind> {
        // Slash: single-line buffer that starts with `/`.
        if self.lines.len() == 1 && self.lines[0].starts_with('/') {
            return Some(MenuKind::Slash);
        }
        // At: cursor inside an `@token` on the current line.
        if self.find_at_token().is_some() && !self.file_completions.is_empty() {
            return Some(MenuKind::At);
        }
        None
    }

    /// Returns `(char_pos_of_@, query_after_@)` if the cursor is inside an
    /// `@token` on the current line.
    fn find_at_token(&self) -> Option<(usize, String)> {
        let line = self.lines.get(self.cursor_line)?;
        let chars: Vec<char> = line.chars().collect();
        let cursor = self.cursor_col.min(chars.len());
        if cursor == 0 {
            return None;
        }
        let mut i = cursor;
        while i > 0 {
            let c = chars[i - 1];
            if c.is_whitespace() {
                return None;
            }
            if c == '@' {
                let query: String = chars[i..cursor].iter().collect();
                return Some((i - 1, query));
            }
            i -= 1;
        }
        None
    }

    pub fn menu_items(&self) -> Vec<MenuItem> {
        match self.menu_kind() {
            Some(MenuKind::Slash) => {
                let q = self.lines[0].strip_prefix('/').unwrap_or("").to_lowercase();
                SLASH_COMMANDS
                    .iter()
                    .filter(|(cmd, _)| cmd[1..].to_lowercase().starts_with(&q))
                    .map(|(cmd, desc)| MenuItem {
                        value: (*cmd).to_string(),
                        description: (*desc).to_string(),
                    })
                    .collect()
            }
            Some(MenuKind::At) => {
                let q = self
                    .find_at_token()
                    .map(|(_, q)| q.to_lowercase())
                    .unwrap_or_default();
                self.file_completions
                    .iter()
                    .filter(|f| f.to_lowercase().contains(&q))
                    .map(|f| MenuItem {
                        value: f.clone(),
                        description: String::new(),
                    })
                    .collect()
            }
            None => Vec::new(),
        }
    }

    pub fn menu_selected(&self) -> usize {
        self.menu_selected
    }

    pub fn menu_open(&self) -> bool {
        self.menu_kind().is_some() && !self.menu_items().is_empty()
    }

    fn menu_next(&mut self) {
        let n = self.menu_items().len();
        if n == 0 {
            return;
        }
        self.menu_selected = (self.menu_selected + 1) % n;
    }

    fn menu_prev(&mut self) {
        let n = self.menu_items().len();
        if n == 0 {
            return;
        }
        self.menu_selected = if self.menu_selected == 0 {
            n - 1
        } else {
            self.menu_selected - 1
        };
    }

    fn clamp_menu_selected(&mut self) {
        let n = self.menu_items().len();
        if n == 0 {
            self.menu_selected = 0;
        } else if self.menu_selected >= n {
            self.menu_selected = n - 1;
        }
    }

    /// Replace the relevant slice of the buffer with the chosen menu item.
    fn menu_accept(&mut self) -> ComposerAction {
        let kind = match self.menu_kind() {
            Some(k) => k,
            None => return ComposerAction::Consumed,
        };
        let items = self.menu_items();
        let item = match items.get(self.menu_selected) {
            Some(i) => i.clone(),
            None => return ComposerAction::Consumed,
        };
        match kind {
            MenuKind::Slash => {
                self.lines[0] = format!("{} ", item.value);
                self.cursor_col = self.lines[0].chars().count();
            }
            MenuKind::At => {
                let (at_pos, _) = self.find_at_token().unwrap();
                let chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
                let before: String = chars[..at_pos].iter().collect();
                let after: String = chars[self.cursor_col.min(chars.len())..].iter().collect();
                self.lines[self.cursor_line] = format!("{before}@{} {after}", item.value);
                self.cursor_col = at_pos + 1 + item.value.chars().count() + 1;
            }
        }
        self.menu_selected = 0;
        ComposerAction::Consumed
    }

    // ---- key dispatch ----

    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> ComposerAction {
        if mods.contains(KeyModifiers::CONTROL) {
            return ComposerAction::PassThrough;
        }

        if self.working {
            if code == KeyCode::Esc {
                return ComposerAction::PassThrough;
            }
            return ComposerAction::Consumed;
        }

        // Menu navigation takes precedence when a menu is open.
        if self.menu_open() {
            match code {
                KeyCode::Up => {
                    self.menu_prev();
                    return ComposerAction::Consumed;
                }
                KeyCode::Down => {
                    self.menu_next();
                    return ComposerAction::Consumed;
                }
                KeyCode::Tab => return self.menu_accept(),
                KeyCode::Enter if !mods.contains(KeyModifiers::SHIFT) => return self.menu_accept(),
                KeyCode::Esc => {
                    // Dismiss menu visually by clearing the trigger token.
                    // Keeping the buffer intact would just reopen the menu
                    // on the next render. The least-surprise behavior is to
                    // strip the trigger so typing can continue.
                    self.dismiss_menu_trigger();
                    return ComposerAction::Consumed;
                }
                _ => {} // fall through to text editing (live filter)
            }
        }

        // Shift+Enter or Alt+Enter → newline.
        if code == KeyCode::Enter
            && (mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::ALT))
        {
            self.insert_newline();
            return ComposerAction::Consumed;
        }

        match code {
            KeyCode::Enter => {
                if self.is_empty() {
                    return ComposerAction::Consumed;
                }
                let text = self.text();
                self.clear();
                ComposerAction::Submit(text)
            }
            KeyCode::Esc => {
                if self.is_empty() {
                    ComposerAction::PassThrough
                } else {
                    self.clear();
                    ComposerAction::Consumed
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
                ComposerAction::Consumed
            }
            KeyCode::Backspace => {
                self.backspace();
                ComposerAction::Consumed
            }
            KeyCode::Delete => {
                self.delete();
                ComposerAction::Consumed
            }
            KeyCode::Left => {
                self.cursor_left();
                ComposerAction::Consumed
            }
            KeyCode::Right => {
                self.cursor_right();
                ComposerAction::Consumed
            }
            KeyCode::Up => {
                if self.cursor_line == 0 {
                    ComposerAction::PassThrough
                } else {
                    self.cursor_up();
                    ComposerAction::Consumed
                }
            }
            KeyCode::Down => {
                if self.cursor_line + 1 >= self.lines.len() {
                    ComposerAction::PassThrough
                } else {
                    self.cursor_down();
                    ComposerAction::Consumed
                }
            }
            KeyCode::Home => {
                self.cursor_col = 0;
                ComposerAction::Consumed
            }
            KeyCode::End => {
                self.cursor_col = self.lines[self.cursor_line].chars().count();
                ComposerAction::Consumed
            }
            _ => ComposerAction::PassThrough,
        }
    }

    fn dismiss_menu_trigger(&mut self) {
        // For slash: clear the whole buffer. For at: remove the @token under
        // cursor. The user just hit Esc — they're saying "not now".
        match self.menu_kind() {
            Some(MenuKind::Slash) => self.clear(),
            Some(MenuKind::At) => {
                if let Some((at_pos, _)) = self.find_at_token() {
                    let chars: Vec<char> = self.lines[self.cursor_line].chars().collect();
                    let before: String = chars[..at_pos].iter().collect();
                    let after: String = chars[self.cursor_col.min(chars.len())..].iter().collect();
                    self.lines[self.cursor_line] = format!("{before}{after}");
                    self.cursor_col = at_pos;
                }
            }
            None => {}
        }
        self.menu_selected = 0;
    }

    // ---- edit ops ----

    fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_line];
        let byte_pos = char_idx_to_byte(line, self.cursor_col);
        line.insert(byte_pos, c);
        self.cursor_col += 1;
        self.after_edit();
    }

    fn insert_newline(&mut self) {
        let line = &mut self.lines[self.cursor_line];
        let byte_pos = char_idx_to_byte(line, self.cursor_col);
        let rest = line.split_off(byte_pos);
        self.lines.insert(self.cursor_line + 1, rest);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.after_edit();
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_line];
            let start = char_idx_to_byte(line, self.cursor_col - 1);
            let end = char_idx_to_byte(line, self.cursor_col);
            line.replace_range(start..end, "");
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            let removed = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            let prev_len = self.lines[self.cursor_line].chars().count();
            self.lines[self.cursor_line].push_str(&removed);
            self.cursor_col = prev_len;
        }
        self.after_edit();
    }

    fn delete(&mut self) {
        let len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col < len {
            let line = &mut self.lines[self.cursor_line];
            let start = char_idx_to_byte(line, self.cursor_col);
            let end = char_idx_to_byte(line, self.cursor_col + 1);
            line.replace_range(start..end, "");
        } else if self.cursor_line + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next);
        }
        self.after_edit();
    }

    fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].chars().count();
        }
        self.update_scroll();
    }

    fn cursor_right(&mut self) {
        let len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
        self.update_scroll();
    }

    fn cursor_up(&mut self) {
        if self.cursor_line == 0 {
            return;
        }
        self.cursor_line -= 1;
        let len = self.lines[self.cursor_line].chars().count();
        self.cursor_col = self.cursor_col.min(len);
        self.update_scroll();
    }

    fn cursor_down(&mut self) {
        if self.cursor_line + 1 >= self.lines.len() {
            return;
        }
        self.cursor_line += 1;
        let len = self.lines[self.cursor_line].chars().count();
        self.cursor_col = self.cursor_col.min(len);
        self.update_scroll();
    }

    fn after_edit(&mut self) {
        self.update_scroll();
        self.clamp_menu_selected();
    }

    fn update_scroll(&mut self) {
        if self.cursor_line < self.scroll_top {
            self.scroll_top = self.cursor_line;
        }
        if self.cursor_line >= self.scroll_top + MAX_VISIBLE_LINES {
            self.scroll_top = self.cursor_line + 1 - MAX_VISIBLE_LINES;
        }
    }
}

fn char_idx_to_byte(s: &str, idx: usize) -> usize {
    s.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(s.len())
}
