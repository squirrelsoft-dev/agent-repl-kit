//! `AgentRepl` — owns the terminal, drains messages from the agent task,
//! handles user input, redraws each frame.

use std::io::{self, Stdout};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use agent_repl_core::{ApprovalChoice, ApprovalPrompt, FormAnswers, Theme, TodoItem};
use anyhow::Result;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event as CtEvent, KeyCode, KeyEventKind,
    KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Frame, Terminal};
use tokio::sync::{mpsc, Mutex};

use crate::approval as approval_box;
use crate::composer::render::MascotPaint;
use crate::composer::{render as composer_render, Composer, ComposerAction};
use crate::decorations::Decorations;
use crate::gallery;
use crate::handle::{Msg, ReplHandle};
use crate::mascot::{Mascot, MascotState};
use crate::question::{QuestionAction, QuestionState};
use crate::spinner;
use crate::stream::Stream;
use crate::style::{color, fg};
use crate::tasks;

type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Blank columns kept between the input text and an attached mascot.
const MASCOT_GAP: u16 = 2;

/// Which stream is currently visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    /// The live transcript the agent is driving via [`ReplHandle`].
    Live,
    /// A static sampler with one of every block kind. Press `F6` to enter.
    Gallery,
}

pub struct AgentRepl {
    theme: Theme,
    deco: Decorations,
    stream: Stream,
    gallery: Option<Stream>,
    view: AppView,
    composer: Composer,
    rx: mpsc::UnboundedReceiver<Msg>,
    input_tx: mpsc::UnboundedSender<String>,
    start: Instant,
    last_content_height: u16,
    last_viewport_height: u16,
    // Esc-abort + a pending approval prompt.
    abort_tx: mpsc::UnboundedSender<()>,
    approval_tx: mpsc::UnboundedSender<ApprovalChoice>,
    working: bool,
    approval: Option<ApprovalPrompt>,
    // Highlighted option in the permissions box (↑↓ navigation).
    approval_selected: usize,
    // A tabbed question form + the channel its answers flow out.
    questions: Option<QuestionState>,
    answers_tx: mpsc::UnboundedSender<FormAnswers>,
    // Shift+Tab "cycle mode" requests flowing out to the driver.
    mode_cycle_tx: mpsc::UnboundedSender<()>,
    // Mid-run messages flowing out: Enter-while-working steers the running
    // turn; Alt+Enter-while-working queues a follow-up for after the run.
    steer_tx: mpsc::UnboundedSender<String>,
    follow_up_tx: mpsc::UnboundedSender<String>,
    // Optional mascot drawn in the composer's right strip + its expression.
    mascot: Option<Box<dyn Mascot>>,
    mascot_state: MascotState,
    // Sticky task-list panel above the working line (empty ⇒ hidden).
    tasks: Vec<TodoItem>,
    // When the current turn started working — drives the live `(Ns …)` timer on
    // the working line. `None` while idle.
    work_started: Option<Instant>,
    // Driver-supplied working-line activity detail (e.g. `"↓ 1.8k tokens"`),
    // shown after the timer. Cleared when work stops.
    activity: Option<String>,
}

impl std::fmt::Debug for AgentRepl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRepl")
            .field("theme", &self.theme)
            .field("view", &self.view)
            .field("stream", &self.stream)
            .field("composer", &self.composer)
            .finish_non_exhaustive()
    }
}

impl AgentRepl {
    /// Construct a new REPL and a handle the agent task uses to drive it.
    pub fn new(theme: Theme) -> (Self, ReplHandle) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (abort_tx, abort_rx) = mpsc::unbounded_channel();
        let (approval_tx, approval_rx) = mpsc::unbounded_channel();
        let (answers_tx, answers_rx) = mpsc::unbounded_channel();
        let (mode_cycle_tx, mode_cycle_rx) = mpsc::unbounded_channel();
        let (steer_tx, steer_rx) = mpsc::unbounded_channel();
        let (follow_up_tx, follow_up_rx) = mpsc::unbounded_channel();
        let handle = ReplHandle {
            tx,
            input_rx: Mutex::new(input_rx),
            next_id: AtomicU64::new(1),
            abort_rx: Mutex::new(abort_rx),
            approval_rx: Mutex::new(approval_rx),
            answers_rx: Mutex::new(answers_rx),
            mode_cycle_rx: Mutex::new(mode_cycle_rx),
            steer_rx: Mutex::new(steer_rx),
            follow_up_rx: Mutex::new(follow_up_rx),
        };
        let app = Self {
            theme,
            deco: Decorations::default(),
            stream: Stream::default(),
            gallery: None,
            view: AppView::Live,
            composer: Composer::default(),
            rx,
            input_tx,
            start: Instant::now(),
            last_content_height: 0,
            last_viewport_height: 0,
            abort_tx,
            approval_tx,
            working: false,
            approval: None,
            approval_selected: 0,
            questions: None,
            answers_tx,
            mode_cycle_tx,
            steer_tx,
            follow_up_tx,
            mascot: None,
            mascot_state: MascotState::Idle,
            tasks: Vec::new(),
            work_started: None,
            activity: None,
        };
        (app, handle)
    }

    // ---- fluent decoration builders ----

    pub fn with_user_sigil(mut self, sigil: impl Into<String>) -> Self {
        self.deco.user_sigil = Some(sigil.into());
        self
    }

    pub fn with_assistant_sigil(mut self, sigil: impl Into<String>) -> Self {
        self.deco.assistant_sigil = Some(sigil.into());
        self
    }

    pub fn with_decorations(mut self, deco: Decorations) -> Self {
        self.deco = deco;
        self
    }

    // ---- fluent composer-context builders ----

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.composer.set_model(model);
        self
    }
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.composer.set_cwd(cwd);
        self
    }
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.composer.set_branch(Some(branch.into()));
        self
    }

    /// Provide the list of file names available for `@file` completion.
    /// Real apps can refresh this dynamically as the working tree changes.
    pub fn with_file_completions(mut self, files: Vec<String>) -> Self {
        self.composer.set_file_completions(files);
        self
    }

    /// Provide the slash-command catalog shown by the `/` menu, as
    /// `(command, description)` pairs (e.g. `("/mode", "Switch the mode")`; the
    /// leading `/` is included). Overrides the built-in demo list so the menu
    /// reflects the host app's OWN commands. Without this, the kit's placeholder
    /// [`SLASH_COMMANDS`](crate::composer::SLASH_COMMANDS) are shown.
    pub fn with_slash_commands(mut self, commands: Vec<(String, String)>) -> Self {
        self.composer.set_slash_commands(commands);
        self
    }

    /// Install the modes shown by the `/mode` picker, as `(name, description)`
    /// pairs in display order. With a list installed, typing `/mode` opens a
    /// pick-from-a-list menu and Shift+Tab (on an idle composer) cycles the modes;
    /// the host resolves the cycle via [`ReplHandle::recv_mode_cycle`]. Without it,
    /// `/mode` falls back to the generic slash menu and Shift+Tab does nothing.
    pub fn with_mode_completions(mut self, modes: Vec<(String, String)>) -> Self {
        self.composer.set_mode_completions(modes);
        self
    }

    // ---- input-sizing + mascot builders ----

    /// Force the input field to always show at least `n` rows (it still grows
    /// with typed content up to the cap). Useful on its own, or set
    /// automatically by [`Self::with_mascot`].
    pub fn with_min_input_lines(mut self, n: usize) -> Self {
        self.composer.set_min_visible_lines(n);
        self
    }

    /// Reserve `cols` columns on the right of the input field. Typed text lays
    /// out in the remaining width and can't draw into the strip — i.e. this caps
    /// the input text width. Set automatically by [`Self::with_mascot`].
    pub fn with_input_reserved_right(mut self, cols: u16) -> Self {
        self.composer.set_reserved_right(cols);
        self
    }

    /// Attach a [`Mascot`] drawn in the input's right strip. This reserves a
    /// strip as wide as the mascot (plus a 2-column gap) and raises the input's
    /// minimum height to the mascot's height, so text never collides with it.
    pub fn with_mascot<M: Mascot + 'static>(mut self, mascot: M) -> Self {
        let (w, h) = mascot.size();
        self.composer.set_min_visible_lines(h as usize);
        self.composer.set_reserved_right(w + MASCOT_GAP);
        self.mascot = Some(Box::new(mascot));
        self
    }

    /// Run until the user quits. Owns the terminal.
    pub async fn run(mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;
        let res = self.event_loop(&mut terminal).await;
        restore_terminal(&mut terminal)?;
        res
    }

    async fn event_loop(&mut self, terminal: &mut Tui) -> Result<()> {
        loop {
            while let Ok(msg) = self.rx.try_recv() {
                self.apply(msg);
            }

            terminal.draw(|frame| {
                self.draw(frame);
            })?;

            if event::poll(Duration::from_millis(80))? {
                match event::read()? {
                    CtEvent::Key(k) if k.kind != KeyEventKind::Release => {
                        if self.handle_key(k.code, k.modifiers) {
                            return Ok(());
                        }
                    }
                    CtEvent::Resize(_, _) => {}
                    // Bracketed paste: route the whole paste to the composer, which
                    // inlines a single line or shows a `[Pasted text …]` placeholder
                    // for a multi-line paste instead of submitting on each newline.
                    CtEvent::Paste(text) => self.composer.paste(text),
                    _ => {}
                }
            }

            tokio::task::yield_now().await;
        }
    }

    fn apply(&mut self, msg: Msg) {
        match msg {
            Msg::Append(ev) => self.stream.push(ev),
            Msg::AppendTool(id, call) => self.stream.push_tool(id, call),
            Msg::UpdateTool(id, call) => self.stream.update_tool(id, call),
            Msg::SetWorking(w) => {
                self.working = w;
                self.composer.set_working(w);
                // Start the working-line timer on the leading edge (keep an
                // already-running clock so a mid-turn `set_working(true)` doesn't
                // reset it); stop clears the timer + activity detail.
                if w {
                    self.work_started.get_or_insert_with(Instant::now);
                } else {
                    self.work_started = None;
                    self.activity = None;
                }
                // Auto-drive the mascot: start working → Thinking (unless the
                // driver already set a richer active state); stop → back to Idle.
                if w {
                    if self.mascot_state == MascotState::Idle {
                        self.mascot_state = MascotState::Thinking;
                    }
                } else {
                    self.mascot_state = MascotState::Idle;
                }
            }
            Msg::SetApproval(a) => {
                // A fresh prompt always starts with the first option (Yes) selected.
                self.approval_selected = 0;
                self.approval = a;
            }
            Msg::SetQuestions(form) => {
                self.questions = form.map(QuestionState::new);
            }
            Msg::SetMascotState(state) => self.mascot_state = state,
            Msg::SetTasks(tasks) => self.tasks = tasks,
            Msg::SetModel(model) => self.composer.set_model(model),
            Msg::SetBranch(branch) => self.composer.set_branch(branch),
            Msg::SetTokens(tokens) => self.composer.set_tokens(tokens),
            Msg::SetActivity(detail) => self.activity = detail,
            Msg::SetModeCompletions(modes) => self.composer.set_mode_completions(modes),
        }
    }

    fn active(&self) -> &Stream {
        match self.view {
            AppView::Live => &self.stream,
            AppView::Gallery => self
                .gallery
                .as_ref()
                .expect("gallery built when view == Gallery"),
        }
    }

    fn active_mut(&mut self) -> &mut Stream {
        match self.view {
            AppView::Live => &mut self.stream,
            AppView::Gallery => self
                .gallery
                .as_mut()
                .expect("gallery built when view == Gallery"),
        }
    }

    fn enter_gallery(&mut self) {
        if self.gallery.is_none() {
            self.gallery = Some(gallery::build());
        }
        self.view = AppView::Gallery;
    }

    /// Returns `true` when the app should quit after this key.
    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> bool {
        // Ctrl-C: always quit.
        if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
            return true;
        }

        // Esc while the agent is working OR an approval prompt / question form
        // is up emits an ABORT signal. Otherwise Esc dismisses an open menu /
        // clears the composer (handled by the composer below) and, on an idle
        // empty composer, does nothing — it never quits the app (quit is ^C / F10).
        if matches!(code, KeyCode::Esc)
            && !mods.contains(KeyModifiers::CONTROL)
            && (self.working || self.approval.is_some() || self.questions.is_some())
        {
            let _ = self.abort_tx.send(());
            return false;
        }

        // While the permissions box is up it owns the keyboard: ↑↓ (or j/k,
        // Tab/BackTab) move the selection, Enter confirms it, and the y/a/n and
        // 1/2/3 shortcuts resolve a choice directly. Every other key is swallowed.
        if let Some(prompt) = self.approval.as_ref() {
            // `options` returns an owned list, so the borrow of `self.approval`
            // ends here and we're free to mutate `approval_selected` below.
            let opts = approval_box::options(prompt);
            let n = opts.len();
            let mut chosen: Option<ApprovalChoice> = None;
            match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                    self.approval_selected = (self.approval_selected + n - 1) % n;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                    self.approval_selected = (self.approval_selected + 1) % n;
                }
                KeyCode::Enter => chosen = Some(opts[self.approval_selected.min(n - 1)].0),
                KeyCode::Char('y') | KeyCode::Char('Y') => chosen = Some(ApprovalChoice::Accept),
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    chosen = opts
                        .iter()
                        .find(|(c, _)| *c == ApprovalChoice::AcceptAll)
                        .map(|(c, _)| *c);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('d') => {
                    chosen = Some(ApprovalChoice::Deny);
                }
                KeyCode::Char(c @ '1'..='9') => {
                    let idx = c as usize - '1' as usize;
                    if idx < n {
                        chosen = Some(opts[idx].0);
                    }
                }
                _ => {}
            }
            if let Some(c) = chosen {
                let _ = self.approval_tx.send(c);
            }
            return false;
        }

        // While the question form is up it owns the keyboard. The state machine
        // handles tab/option navigation, selection, and freeform typing; a
        // submit returns the answers, which we forward to the driving task.
        if let Some(qs) = self.questions.as_mut() {
            if let QuestionAction::Submit(answers) = qs.handle_key(code) {
                let _ = self.answers_tx.send(answers);
            }
            return false;
        }

        // First chance: the composer.
        match self.composer.handle_key(code, mods) {
            ComposerAction::Consumed => return false,
            ComposerAction::Submit(text) => {
                // User sent a message. Mirror it into the stream as a user
                // event, flip to working, and forward to recv_input().
                self.stream.push(agent_repl_core::Event::user(text.clone()));
                self.working = true;
                self.composer.set_working(true);
                // Start the elapsed-turn clock the instant the user sends, before
                // the driver's own `set_working(true)` echoes back.
                self.work_started = Some(Instant::now());
                self.activity = None;
                let _ = self.input_tx.send(text);
                return false;
            }
            ComposerAction::Steer(text) => {
                // Enter while the agent is working: steer the running turn.
                // Mirror into the stream (tagged so the user sees it's a
                // steer, not a new turn) and forward to recv_steer().
                self.stream
                    .push(agent_repl_core::Event::user(format!("(steering) {text}")));
                let _ = self.steer_tx.send(text);
                return false;
            }
            ComposerAction::QueueFollowUp(text) => {
                // Alt+Enter while working: hold the message until the run ends.
                self.stream
                    .push(agent_repl_core::Event::user(format!("(queued) {text}")));
                let _ = self.follow_up_tx.send(text);
                return false;
            }
            ComposerAction::CycleMode => {
                // Shift+Tab on an idle composer: ask the driver to advance the mode.
                let _ = self.mode_cycle_tx.send(());
                return false;
            }
            ComposerAction::PassThrough => {}
        }

        // Composer didn't claim it. The app's own keys take over.
        let vh = self.last_viewport_height;
        let h = self.last_content_height;
        let max = h.saturating_sub(vh);
        let cur = self.active().scroll_offset(h, vh);

        match code {
            // Esc never quits the app. While working / a prompt is up it aborts
            // (handled above); on an idle, empty composer it is a deliberate no-op
            // so a stray double-tap can't drop the user out of the session. Quit is
            // Ctrl-C (or F10) — the documented exit in the status bar.
            KeyCode::Esc => {}
            KeyCode::F(1) => self.theme = self.theme.cycle_vibe(),
            KeyCode::F(2) => self.theme = self.theme.toggle_mode(),
            KeyCode::F(3) => self.theme = self.theme.cycle_tool_style(),
            KeyCode::F(4) => self.theme = self.theme.toggle_density(),
            KeyCode::F(5) => self.view = AppView::Live,
            KeyCode::F(6) => self.enter_gallery(),
            KeyCode::F(10) => return true,

            // Focus + expand for collapsed tools
            KeyCode::Tab => self.active_mut().focus_next(),
            KeyCode::BackTab => self.active_mut().focus_prev(),
            KeyCode::Char('e') if mods.contains(KeyModifiers::CONTROL) => {
                self.active_mut().toggle_focused_expansion()
            }

            // Scrolling
            KeyCode::PageUp => self
                .active_mut()
                .scroll_up(vh.saturating_sub(2).max(1), cur),
            KeyCode::PageDown => self
                .active_mut()
                .scroll_down(vh.saturating_sub(2).max(1), cur, max),
            KeyCode::Up | KeyCode::Char('k') => self.active_mut().scroll_up(1, cur),
            KeyCode::Down | KeyCode::Char('j') => self.active_mut().scroll_down(1, cur, max),
            _ => {}
        }
        false
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let composer_h = composer_render::required_height(&self.composer, &self.theme, area.width);
        let menu_h = composer_render::menu_height(&self.composer, &self.theme);
        // The permissions box and question form both float directly above the
        // composer while active; each claims zero rows otherwise.
        let approval_h = self
            .approval
            .as_ref()
            .map(approval_box::required_height)
            .unwrap_or(0);
        let questions_h = self
            .questions
            .as_ref()
            .map(QuestionState::required_height)
            .unwrap_or(0);
        // The sticky task-list panel floats directly above the working line
        // while there are tasks (zero rows otherwise).
        let tasks_h = tasks::required_height(&self.tasks);
        // A single "working" line sits directly above the composer while the
        // agent runs (it replaces the old in-field + in-footer spinners).
        let working_h: u16 = if self.working { 1 } else { 0 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(menu_h),
                Constraint::Length(approval_h),
                Constraint::Length(questions_h),
                Constraint::Length(tasks_h),
                Constraint::Length(working_h),
                Constraint::Length(composer_h),
                Constraint::Length(1),
            ])
            .split(area);
        let body_area = chunks[0];
        let menu_area = chunks[1];
        let approval_area = chunks[2];
        let questions_area = chunks[3];
        let tasks_area = chunks[4];
        let working_area = chunks[5];
        let composer_area = chunks[6];
        let status_area = chunks[7];

        let spinner_frame = spinner::frame_for(self.start.elapsed());
        let text = self.active().render(&self.theme, &self.deco, spinner_frame);
        // Pre-wrap with hanging indents so wrapped rows keep each block's gutter
        // instead of spilling into the border column, then render with ratatui's
        // own wrapping disabled (the lines already fit).
        let text = crate::wrap::wrap_text(text, body_area.width);

        let body_style = Style::default()
            .bg(color(self.theme.palette.bg))
            .fg(color(self.theme.palette.text));
        let content_height = text.height() as u16;
        let paragraph = Paragraph::new(text).style(body_style);
        let viewport_height = body_area.height;
        self.last_content_height = content_height;
        self.last_viewport_height = viewport_height;

        let scroll = self
            .active()
            .scroll_offset(content_height, viewport_height);
        frame.render_widget(paragraph.scroll((scroll, 0)), body_area);

        composer_render::render_menu(&self.composer, &self.theme, frame, menu_area);
        if let Some(prompt) = self.approval.as_ref() {
            approval_box::render(
                prompt,
                self.approval_selected,
                &self.theme,
                frame,
                approval_area,
            );
        }
        if let Some(qs) = self.questions.as_ref() {
            qs.render(&self.theme, frame, questions_area);
        }
        tasks::render(&self.tasks, &self.theme, frame, tasks_area);
        self.draw_working_line(frame, working_area, spinner_frame);
        let mascot_paint = self.mascot.as_deref().map(|m| MascotPaint {
            mascot: m,
            state: self.mascot_state,
            elapsed: self.start.elapsed(),
        });
        composer_render::render(&self.composer, &self.theme, frame, composer_area, mascot_paint);
        self.draw_status_bar(frame, status_area);
    }

    /// The single working indicator: a spinner + "Working…" on a fixed line
    /// directly above the composer, shown only while the agent runs (the area is
    /// zero-height otherwise).
    fn draw_working_line(&self, frame: &mut Frame, area: Rect, spinner_frame: char) {
        if area.height == 0 || !self.working {
            return;
        }
        let p = &self.theme.palette;
        let mut spans = vec![
            Span::raw("  ".to_string()),
            Span::styled(
                format!("{spinner_frame} "),
                fg(p.warning).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Working\u{2026}".to_string(),
                fg(p.text_dim).add_modifier(Modifier::ITALIC),
            ),
        ];
        // Live readout in normal text color: `(37s · ↓ 1.8k tokens)`. The timer
        // ticks every frame; the activity detail (after the `·`) is whatever the
        // driver last reported, shown only when present.
        if let Some(started) = self.work_started {
            let secs = started.elapsed().as_secs();
            let detail = match &self.activity {
                Some(a) => format!(" ({secs}s \u{00B7} {a})"),
                None => format!(" ({secs}s)"),
            };
            spans.push(Span::styled(detail, fg(p.text)));
        }
        let line = Line::from(spans);
        // Match the transcript ("threads") background, not the raised composer
        // panel — otherwise this line reads as part of the input box.
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(color(p.bg))),
            area,
        );
    }

    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let p = &self.theme.palette;

        // While the permissions box is up, the status bar carries the
        // navigation hints (the choices themselves live in the box).
        if self.approval.is_some() {
            let line = Line::from(vec![
                Span::styled(
                    " \u{23F8} permission required  ".to_string(),
                    fg(p.warning).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "\u{2191}\u{2193} select".to_string(),
                    fg(p.text),
                ),
                Span::styled("  \u{23CE} confirm".to_string(), fg(p.text)),
                Span::styled("  y/a/n shortcut".to_string(), fg(p.text)),
                Span::styled("  \u{00B7} Esc abort".to_string(), fg(p.text_faint)),
            ]);
            frame.render_widget(
                Paragraph::new(line)
                    .style(Style::default().bg(color(p.bg_raised)).fg(color(p.text_dim))),
                area,
            );
            return;
        }

        // While the question form is up, the status bar carries its key hints.
        if let Some(qs) = self.questions.as_ref() {
            let line = Line::from(vec![
                Span::styled(
                    " \u{2370} question  ".to_string(),
                    fg(p.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(qs.status_hint(), fg(p.text)),
            ]);
            frame.render_widget(
                Paragraph::new(line)
                    .style(Style::default().bg(color(p.bg_raised)).fg(color(p.text_dim))),
                area,
            );
            return;
        }

        let sep = Span::styled(" · ".to_string(), fg(p.text_faint));
        let view_label = match self.view {
            AppView::Live => "live",
            AppView::Gallery => "gallery",
        };
        let mut spans = vec![
            Span::styled(
                format!(" {}", self.theme.info.label),
                fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            sep.clone(),
            Span::styled(
                match self.theme.mode {
                    agent_repl_core::Mode::Dark => "dark",
                    agent_repl_core::Mode::Light => "light",
                }
                .to_string(),
                fg(p.text_dim),
            ),
            sep.clone(),
            Span::styled(
                match self.theme.tool_style {
                    agent_repl_core::ToolStyle::Inline => "inline",
                    agent_repl_core::ToolStyle::Card => "card",
                    agent_repl_core::ToolStyle::Collapsed => "collapsed",
                }
                .to_string(),
                fg(p.text_dim),
            ),
            sep.clone(),
            Span::styled(
                match self.theme.density {
                    agent_repl_core::Density::Comfortable => "comfortable",
                    agent_repl_core::Density::Compact => "compact",
                }
                .to_string(),
                fg(p.text_dim),
            ),
            sep.clone(),
            Span::styled(
                view_label.to_string(),
                fg(p.accent_soft).add_modifier(Modifier::BOLD),
            ),
        ];
        let right = Span::styled(
            "F1\u{2013}F4 theme · F5/F6 view · Tab focus · ^E expand · PgUp/PgDn scroll · ^C quit "
                .to_string(),
            fg(p.text_faint),
        );
        let left_w: u16 = spans.iter().map(|s| s.content.chars().count() as u16).sum();
        let right_w: u16 = right.content.chars().count() as u16;
        let pad = area.width.saturating_sub(left_w + right_w);
        spans.push(Span::raw(" ".repeat(pad as usize)));
        spans.push(right);
        let line = Line::from(spans);
        frame.render_widget(
            Paragraph::new(line)
                .style(Style::default().bg(color(p.bg_raised)).fg(color(p.text_dim))),
            area,
        );
    }

    /// Forward a line of user text out through the input channel (e.g. for
    /// scripted driver tests that bypass the composer).
    pub fn push_input(&self, text: impl Into<String>) {
        let _ = self.input_tx.send(text.into());
    }
}

fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // Bracketed paste: the terminal delivers a paste as ONE `CtEvent::Paste`
    // instead of a keystroke stream, so a newline inside the paste no longer reads
    // as an Enter/submit (see the composer's `paste`).
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableBracketedPaste)?;
    terminal.show_cursor()?;
    Ok(())
}
