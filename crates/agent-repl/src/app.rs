//! `AgentRepl` — owns the terminal, drains messages from the agent task,
//! handles user input, redraws each frame.

use std::io::{self, Stdout};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use agent_repl_core::{ApprovalChoice, ApprovalPrompt, FormAnswers, Theme};
use anyhow::Result;
use crossterm::event::{self, Event as CtEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use tokio::sync::{mpsc, Mutex};

use crate::approval as approval_box;
use crate::composer::{render as composer_render, Composer, ComposerAction};
use crate::decorations::Decorations;
use crate::gallery;
use crate::handle::{Msg, ReplHandle};
use crate::question::{QuestionAction, QuestionState};
use crate::spinner;
use crate::stream::Stream;
use crate::style::{color, fg};

type Tui = Terminal<CrosstermBackend<Stdout>>;

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
    // looper bolt-ons: Esc-abort + a pending approval prompt.
    abort_tx: mpsc::UnboundedSender<()>,
    approval_tx: mpsc::UnboundedSender<ApprovalChoice>,
    working: bool,
    approval: Option<ApprovalPrompt>,
    // Highlighted option in the permissions box (↑↓ navigation).
    approval_selected: usize,
    // looper bolt-on: a tabbed question form + the channel its answers flow out.
    questions: Option<QuestionState>,
    answers_tx: mpsc::UnboundedSender<FormAnswers>,
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
        let handle = ReplHandle {
            tx,
            input_rx: Mutex::new(input_rx),
            next_id: AtomicU64::new(1),
            abort_rx: Mutex::new(abort_rx),
            approval_rx: Mutex::new(approval_rx),
            answers_rx: Mutex::new(answers_rx),
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
            }
            Msg::SetApproval(a) => {
                // A fresh prompt always starts with the first option (Yes) selected.
                self.approval_selected = 0;
                self.approval = a;
            }
            Msg::SetQuestions(form) => {
                self.questions = form.map(QuestionState::new);
            }
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

        // looper bolt-on: Esc while the agent is working OR an approval prompt /
        // question form is up emits an ABORT signal instead of quitting. (Esc on
        // an idle, empty composer still quits — that path is unchanged below.)
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
                let _ = self.input_tx.send(text);
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
            KeyCode::Esc => return true, // composer was empty
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
        let composer_h = composer_render::required_height(&self.composer, &self.theme);
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(menu_h),
                Constraint::Length(approval_h),
                Constraint::Length(questions_h),
                Constraint::Length(composer_h),
                Constraint::Length(1),
            ])
            .split(area);
        let body_area = chunks[0];
        let menu_area = chunks[1];
        let approval_area = chunks[2];
        let questions_area = chunks[3];
        let composer_area = chunks[4];
        let status_area = chunks[5];

        let spinner_frame = spinner::frame_for(self.start.elapsed());
        let text = self.active().render(&self.theme, &self.deco, spinner_frame);

        let body_style = Style::default()
            .bg(color(self.theme.palette.bg))
            .fg(color(self.theme.palette.text));
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false }).style(body_style);
        let content_height = paragraph.line_count(body_area.width) as u16;
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
        composer_render::render(
            &self.composer,
            &self.theme,
            frame,
            composer_area,
            spinner_frame,
        );
        self.draw_status_bar(frame, status_area);
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
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
