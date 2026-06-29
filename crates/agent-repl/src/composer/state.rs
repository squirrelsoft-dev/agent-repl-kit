//! Composer state and edit operations. Multi-line buffer with cursor,
//! scroll window, and slash / `@file` menu state derived from the buffer.

use crossterm::event::{KeyCode, KeyModifiers};

/// Maximum number of buffer rows the composer field shows at once.
/// Adding lines beyond this scrolls the field, not the composer chrome.
pub const MAX_VISIBLE_LINES: usize = 10;

/// Most `@file` matches shown at once (the pool may be the whole workspace).
const AT_MENU_MAX: usize = 12;

/// Fuzzy subsequence score of `query` (already lowercased) against `candidate`.
/// `None` if `candidate` lacks the query chars in order; higher is better.
/// Contiguous runs and word-boundary starts score up; skipped chars and length
/// score down. An empty query matches everything at score 0.
fn fuzzy_score(candidate: &str, query: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let cand: Vec<char> = candidate.to_lowercase().chars().collect();
    let mut score = 0i32;
    let mut ci = 0usize;
    let mut prev_match: Option<usize> = None;
    for qc in query.chars() {
        let start = ci;
        while ci < cand.len() && cand[ci] != qc {
            ci += 1;
        }
        if ci == cand.len() {
            return None;
        }
        if prev_match == Some(ci.wrapping_sub(1)) {
            score += 8; // contiguous with the previous match
        }
        if ci == 0 || matches!(cand[ci - 1], '/' | '_' | '-' | '.' | ' ') {
            score += 6; // match begins a path/word segment
        }
        score -= (ci - start) as i32; // chars skipped to reach this match
        prev_match = Some(ci);
        ci += 1;
    }
    score -= cand.len() as i32 / 16; // mild preference for tighter candidates
    Some(score)
}

/// The BUILT-IN demo slash commands offered by the popup menu when a host app
/// does not supply its own. Sourced from `docs/repl/input.jsx` (the `SLASH`
/// constant). A real app overrides this via
/// [`AgentRepl::with_slash_commands`](crate::AgentRepl::with_slash_commands) /
/// [`Composer::set_slash_commands`] so the menu lists ITS commands — these are
/// just a placeholder, NOT a contract the host is required to implement.
pub const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/clear", "Clear the conversation"),
    ("/compact", "Summarize & free up context"),
    ("/model", "Switch the active model"),
    ("/diff", "Review pending changes"),
    ("/undo", "Revert the last edit"),
    ("/run", "Run a shell command"),
];

/// The default slash-command catalog (owned), used when a host app installs none.
fn default_slash_commands() -> Vec<(String, String)> {
    SLASH_COMMANDS.iter().map(|(c, d)| (c.to_string(), d.to_string())).collect()
}

#[derive(Debug, Clone)]
pub struct Composer {
    /// One entry per logical line. Always at least one (possibly empty) line.
    lines: Vec<String>,
    /// Cursor line index into `lines` (0-based).
    cursor_line: usize,
    /// Cursor column as a *char* index into `lines[cursor_line]`.
    cursor_col: usize,
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
    /// Footer context: token-usage figure shown as a `⛁ …` chip (e.g.
    /// `"120k/210k"`). Pre-formatted by the host app; `None` hides the chip.
    pub tokens: Option<String>,
    /// Pool of file names available for `@file` completion.
    pub file_completions: Vec<String>,
    /// Slash-command catalog shown by the `/` menu, as `(command, description)`
    /// pairs (each command includes its leading `/`). Defaults to the built-in
    /// [`SLASH_COMMANDS`]; a host app overrides it via
    /// [`set_slash_commands`](Self::set_slash_commands) to advertise its OWN
    /// commands so the menu matches what the app actually handles.
    pub slash_commands: Vec<(String, String)>,
    /// Minimum rows the field reserves even when nearly empty (default 1).
    /// Used to make room for a mascot drawn in the right strip.
    min_visible_lines: usize,
    /// Columns reserved on the right of the field (e.g. for a mascot). Text is
    /// laid out in the remaining width, so it can't draw into this strip.
    reserved_right: u16,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            menu_selected: 0,
            working: false,
            model: "agent".into(),
            cwd: String::new(),
            branch: None,
            tokens: None,
            file_completions: Vec::new(),
            slash_commands: default_slash_commands(),
            min_visible_lines: 1,
            reserved_right: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    Slash,
    At,
}

/// One visual (soft-wrapped) row of the field: a half-open char range
/// `[start, end)` into logical line `line`. A logical line that fits produces a
/// single row; a longer one is broken into several rows of equal width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualRow {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

/// The buffer laid out into visual rows wrapped to a given content width, plus
/// where the cursor falls in that visual space. Produced by [`Composer::layout`]
/// and consumed by the renderer for sizing, scrolling, and cursor placement.
#[derive(Debug, Clone)]
pub struct FieldLayout {
    /// Visual rows, top to bottom. Always at least one.
    pub rows: Vec<VisualRow>,
    /// Index into `rows` holding the cursor.
    pub cursor_row: usize,
    /// Cursor's char offset within its visual row.
    pub cursor_col: usize,
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
    pub fn set_tokens(&mut self, t: Option<String>) {
        self.tokens = t;
    }
    pub fn set_working(&mut self, w: bool) {
        self.working = w;
    }
    pub fn set_file_completions(&mut self, files: Vec<String>) {
        self.file_completions = files;
        self.clamp_menu_selected();
    }

    /// Replace the slash-command catalog shown by the `/` menu. Each entry is
    /// `(command, description)` with the command's leading `/` included.
    pub fn set_slash_commands(&mut self, commands: Vec<(String, String)>) {
        self.slash_commands = commands;
        self.clamp_menu_selected();
    }

    // ---- buffer accessors ----

    pub fn lines(&self) -> &[String] {
        &self.lines
    }
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Lay the buffer out into visual rows wrapped to `width` content columns
    /// (the space left for text after the field's prefix gutter). Wrapping is
    /// by char count, so every character — cursor included — maps to exactly one
    /// cell. A line whose length is an exact multiple of `width` gets a trailing
    /// empty row *only* when the cursor rests at that boundary, giving the caret
    /// (and the next keystroke) a home on the fresh row.
    pub fn layout(&self, width: u16) -> FieldLayout {
        let w = (width as usize).max(1);
        let mut rows: Vec<VisualRow> = Vec::with_capacity(self.lines.len());
        let mut cursor_row = 0;
        let mut cursor_col = 0;
        for (li, line) in self.lines.iter().enumerate() {
            let n = line.chars().count();
            let row_start = rows.len();
            if n == 0 {
                rows.push(VisualRow { line: li, start: 0, end: 0 });
            } else {
                let mut s = 0;
                while s < n {
                    let e = (s + w).min(n);
                    rows.push(VisualRow { line: li, start: s, end: e });
                    s = e;
                }
            }
            if li == self.cursor_line {
                let col = self.cursor_col.min(n);
                let within = col / w;
                let content_rows = rows.len() - row_start;
                if within >= content_rows {
                    // Cursor sits one past the last full row (`col == n` and `n`
                    // is a positive multiple of `w`): start a fresh trailing row.
                    rows.push(VisualRow { line: li, start: n, end: n });
                    cursor_row = rows.len() - 1;
                    cursor_col = 0;
                } else {
                    cursor_row = row_start + within;
                    cursor_col = col % w;
                }
            }
        }
        FieldLayout { rows, cursor_row, cursor_col }
    }

    /// Clamp a visual-row count to the field's `[min_visible_lines, MAX]` window.
    pub fn clamp_visible(&self, total_rows: usize) -> usize {
        let floor = self.min_visible_lines.clamp(1, MAX_VISIBLE_LINES);
        total_rows.clamp(floor, MAX_VISIBLE_LINES)
    }

    /// Rows the field shows for content wrapped to `width` content columns:
    /// the wrapped row count, floored by the minimum and capped at the max.
    pub fn visible_line_count(&self, width: u16) -> usize {
        self.clamp_visible(self.layout(width).rows.len())
    }

    /// Minimum rows the field always shows (clamped to `[1, MAX_VISIBLE_LINES]`).
    pub fn set_min_visible_lines(&mut self, n: usize) {
        self.min_visible_lines = n;
    }

    /// Columns reserved on the right of the field for an accessory (e.g. a
    /// mascot). The text area is the remaining width.
    pub fn set_reserved_right(&mut self, cols: u16) {
        self.reserved_right = cols;
    }

    pub fn reserved_right(&self) -> u16 {
        self.reserved_right
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
                self.slash_commands
                    .iter()
                    .filter(|(cmd, _)| {
                        cmd.strip_prefix('/').unwrap_or(cmd).to_lowercase().starts_with(&q)
                    })
                    .map(|(cmd, desc)| MenuItem {
                        value: cmd.clone(),
                        description: desc.clone(),
                    })
                    .collect()
            }
            Some(MenuKind::At) => {
                let q = self
                    .find_at_token()
                    .map(|(_, q)| q.to_lowercase())
                    .unwrap_or_default();
                // Fuzzy subsequence match, best score first, capped so a large
                // workspace can't render a giant menu.
                let mut scored: Vec<(i32, &String)> = self
                    .file_completions
                    .iter()
                    .filter_map(|f| fuzzy_score(f, &q).map(|s| (s, f)))
                    .collect();
                scored.sort_by(|a, b| {
                    b.0.cmp(&a.0)
                        .then_with(|| a.1.len().cmp(&b.1.len()))
                        .then_with(|| a.1.cmp(b.1))
                });
                scored
                    .into_iter()
                    .take(AT_MENU_MAX)
                    .map(|(_, f)| MenuItem {
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
                // Quote names with spaces so a `@"my file.png"` reference parses
                // as one token downstream.
                let value = if item.value.contains(' ') {
                    format!("\"{}\"", item.value)
                } else {
                    item.value.clone()
                };
                self.lines[self.cursor_line] = format!("{before}@{value} {after}");
                self.cursor_col = at_pos + 1 + value.chars().count() + 1;
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

        // While the agent is working the field stays fully usable: the user can
        // edit a draft (type-ahead), and keys the composer doesn't claim
        // (scroll, theme, focus, F-keys) still PassThrough to the app so the TUI
        // stays live mid-run. The ONLY thing `working` defers is SUBMIT — see the
        // `KeyCode::Enter` arm below, which keeps the draft instead of starting a
        // new turn until the agent goes idle.

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
                if self.working {
                    // The agent is busy: keep the draft rather than submit a new
                    // turn mid-run. It sends once the user hits Enter after work
                    // ends (the field is editable throughout).
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
    }

    fn cursor_right(&mut self) {
        let len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    fn cursor_up(&mut self) {
        if self.cursor_line == 0 {
            return;
        }
        self.cursor_line -= 1;
        let len = self.lines[self.cursor_line].chars().count();
        self.cursor_col = self.cursor_col.min(len);
    }

    fn cursor_down(&mut self) {
        if self.cursor_line + 1 >= self.lines.len() {
            return;
        }
        self.cursor_line += 1;
        let len = self.lines[self.cursor_line].chars().count();
        self.cursor_col = self.cursor_col.min(len);
    }

    fn after_edit(&mut self) {
        self.clamp_menu_selected();
    }
}

fn char_idx_to_byte(s: &str, idx: usize) -> usize {
    s.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(s.len())
}
