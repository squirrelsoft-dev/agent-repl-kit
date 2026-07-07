//! Channel-based control surface for the REPL. Agent task pushes events
//! in; user input flows out via `recv_input`.

use std::sync::atomic::{AtomicU64, Ordering};

use agent_repl_core::{
    ApprovalChoice, ApprovalPrompt, Event, FormAnswers, QuestionForm, TodoItem, ToolCall,
};
use tokio::sync::{mpsc, Mutex};

use crate::mascot::MascotState;
use crate::stream::ToolId;

#[derive(Debug)]
pub(crate) enum Msg {
    Append(Event),
    AppendTool(ToolId, ToolCall),
    UpdateTool(ToolId, ToolCall),
    SetWorking(bool),
    // Show/hide a three-level approval prompt.
    SetApproval(Option<ApprovalPrompt>),
    // Show/hide a tabbed question form.
    SetQuestions(Option<QuestionForm>),
    // Set the mascot's expression.
    SetMascotState(MascotState),
    // Replace the sticky task-list panel (empty ⇒ hide it).
    SetTasks(Vec<TodoItem>),
    // Update the footer model pill.
    SetModel(String),
    // Update the footer branch pill (`None` ⇒ hide it).
    SetBranch(Option<String>),
    // Update the footer token-usage chip (`None` ⇒ hide it).
    SetTokens(Option<String>),
    // Update the working-line activity detail (`None` ⇒ just the timer).
    SetActivity(Option<String>),
    // Replace the `/mode` picker's mode list (`(name, description)` pairs).
    SetModeCompletions(Vec<(String, String)>),
    // Drop the whole transcript (the host re-renders after a rewind/switch).
    ClearTranscript,
    // Replace the composer's buffer (restore unsent input).
    SetEditorText(String),
}

/// Opaque handle to a tool block already in the stream. Used to update it
/// later (e.g. when the tool finishes).
#[derive(Debug, Clone, Copy)]
pub struct ToolHandle(pub(crate) ToolId);

pub struct ReplHandle {
    pub(crate) tx: mpsc::UnboundedSender<Msg>,
    pub(crate) input_rx: Mutex<mpsc::UnboundedReceiver<String>>,
    pub(crate) next_id: AtomicU64,
    // Esc-abort signal + approval-choice delivery, both flowing from the
    // renderer's key handler back to the driving task.
    pub(crate) abort_rx: Mutex<mpsc::UnboundedReceiver<()>>,
    pub(crate) approval_rx: Mutex<mpsc::UnboundedReceiver<ApprovalChoice>>,
    // Completed question-form answers flowing back.
    pub(crate) answers_rx: Mutex<mpsc::UnboundedReceiver<FormAnswers>>,
    // Shift+Tab "cycle to the next mode" requests from the composer.
    pub(crate) mode_cycle_rx: Mutex<mpsc::UnboundedReceiver<()>>,
    // Mid-run messages: Enter-while-working (steer the running turn) and
    // Alt+Enter-while-working (queue a follow-up for after the run).
    pub(crate) steer_rx: Mutex<mpsc::UnboundedReceiver<String>>,
    pub(crate) follow_up_rx: Mutex<mpsc::UnboundedReceiver<String>>,
}

impl std::fmt::Debug for ReplHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplHandle").finish_non_exhaustive()
    }
}

impl ReplHandle {
    /// Append a non-tool event (user / assistant / reasoning / status / alert).
    pub fn emit(&self, event: Event) {
        let _ = self.tx.send(Msg::Append(event));
    }

    /// Start a tool block in the "running" state. The returned handle can
    /// be used to update or finish it.
    pub fn start_tool(&self, mut call: ToolCall) -> ToolHandle {
        call.running = true;
        let id = ToolId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let _ = self.tx.send(Msg::AppendTool(id, call));
        ToolHandle(id)
    }

    /// Replace an in-flight tool's payload (still running).
    pub fn update_tool(&self, handle: ToolHandle, call: ToolCall) {
        let mut call = call;
        call.running = true;
        let _ = self.tx.send(Msg::UpdateTool(handle.0, call));
    }

    /// Mark a tool finished and swap in its final payload.
    pub fn finish_tool(&self, handle: ToolHandle, mut call: ToolCall) {
        call.running = false;
        let _ = self.tx.send(Msg::UpdateTool(handle.0, call));
    }

    /// Convenience: emit a tool one-shot (no running phase).
    pub fn tool(&self, call: ToolCall) {
        let mut call = call;
        call.running = false;
        let id = ToolId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let _ = self.tx.send(Msg::AppendTool(id, call));
    }

    /// Explicitly toggle the composer's working state. Mostly useful at
    /// startup to mark the agent busy before its first `recv_input` call.
    /// Submission and `recv_input` already auto-flip the state.
    pub fn set_working(&self, working: bool) {
        let _ = self.tx.send(Msg::SetWorking(working));
    }

    /// Set the mascot's expression (e.g. `Coding`, `Testing`, `Success`). The
    /// app also moves the mascot between `Idle` and `Thinking` automatically as
    /// work starts and stops; this overrides it for richer states. No-op when no
    /// mascot is attached.
    pub fn set_mascot_state(&self, state: MascotState) {
        let _ = self.tx.send(Msg::SetMascotState(state));
    }

    /// Replace the sticky task-list panel that floats above the working line.
    /// Each [`TodoItem`] carries its own `Done` / `Active` / `Pending` state.
    /// Pass an empty list (or call [`clear_tasks`](Self::clear_tasks)) to hide it.
    pub fn set_tasks(&self, tasks: Vec<TodoItem>) {
        let _ = self.tx.send(Msg::SetTasks(tasks));
    }

    /// Hide the task-list panel (equivalent to `set_tasks(vec![])`).
    pub fn clear_tasks(&self) {
        let _ = self.tx.send(Msg::SetTasks(Vec::new()));
    }

    /// Update the footer model pill at runtime (the `◇ …` chip). The builder-time
    /// [`AgentRepl::with_model`](crate::AgentRepl::with_model) sets the initial
    /// value; this lets the driver change it mid-session (e.g. on a model switch).
    pub fn set_model(&self, model: impl Into<String>) {
        let _ = self.tx.send(Msg::SetModel(model.into()));
    }

    /// Install (or replace) the modes shown by the `/mode` picker menu, as
    /// `(name, description)` pairs in display order. With a list installed, typing
    /// `/mode` opens a pick-from-a-list menu and Shift+Tab cycles the modes. An
    /// empty list disables both (the generic slash menu handles `/mode`).
    pub fn set_mode_completions(&self, modes: Vec<(String, String)>) {
        let _ = self.tx.send(Msg::SetModeCompletions(modes));
    }

    /// Await the next Shift+Tab "cycle mode" request from the composer. `None` if
    /// the REPL exited. The host decides what "next mode" means and typically calls
    /// [`Self::set_branch`] to reflect the new mode in the pill.
    pub async fn recv_mode_cycle(&self) -> Option<()> {
        let mut rx = self.mode_cycle_rx.lock().await;
        rx.recv().await
    }

    /// Update the footer branch pill at runtime (the `⎇ …` chip); `None` hides it.
    /// The builder-time [`AgentRepl::with_branch`](crate::AgentRepl::with_branch)
    /// sets the initial value. Drivers that surface the active mode here can call
    /// this on a mode switch so the pill always reflects the live state.
    pub fn set_branch(&self, branch: Option<String>) {
        let _ = self.tx.send(Msg::SetBranch(branch));
    }

    /// Update the footer token-usage chip at runtime (the `⛁ …` chip); `None`
    /// hides it. The string is pre-formatted by the host app (e.g. a compact
    /// `"120k/210k"` context/total figure) and rendered verbatim after the
    /// branch pill. There is no builder-time setter — the chip stays hidden
    /// until the driver reports usage.
    pub fn set_tokens(&self, tokens: Option<String>) {
        let _ = self.tx.send(Msg::SetTokens(tokens));
    }

    /// Set the working-line activity detail — the text shown after the elapsed
    /// timer while the agent runs (e.g. `"↓ 1.8k tokens"`). The kit owns the
    /// live `(Ns · …)` framing and the ticking timer; the driver supplies only
    /// this inner detail and updates it as work progresses. `None` shows just the
    /// timer. Auto-cleared when the agent stops working ([`Self::set_working`]
    /// `false` / [`Self::recv_input`]), so a driver never needs to reset it.
    pub fn set_activity(&self, detail: Option<String>) {
        let _ = self.tx.send(Msg::SetActivity(detail));
    }

    /// Await the next line of user input. Returns `None` if the REPL has
    /// exited. Flips the composer to `working = false` while it waits so
    /// the prompt is responsive.
    pub async fn recv_input(&self) -> Option<String> {
        let _ = self.tx.send(Msg::SetWorking(false));
        let mut rx = self.input_rx.lock().await;
        rx.recv().await
    }

    // ---- interaction surfaces (approval / questions / abort) --------------

    /// Show a three-level approval prompt. While shown, `a/A/d` (and `1/2/3`)
    /// resolve it; `Esc` aborts. The renderer keeps `working` semantics so Esc
    /// maps to abort, not quit.
    pub fn request_approval(&self, prompt: ApprovalPrompt) {
        let _ = self.tx.send(Msg::SetApproval(Some(prompt)));
    }

    /// Dismiss the approval prompt.
    pub fn clear_approval(&self) {
        let _ = self.tx.send(Msg::SetApproval(None));
    }

    /// Await the user's approval choice. `None` if the REPL exited.
    pub async fn recv_approval(&self) -> Option<ApprovalChoice> {
        let mut rx = self.approval_rx.lock().await;
        rx.recv().await
    }

    /// Show a tabbed question form. While shown the box owns the keyboard; the
    /// user navigates tabs/options and submits, and `Esc` aborts (delivered via
    /// [`Self::recv_abort`]). Empty forms are ignored. Pair with
    /// [`Self::recv_answers`] to collect the result.
    pub fn ask_questions(&self, form: QuestionForm) {
        if form.questions.is_empty() {
            return;
        }
        let _ = self.tx.send(Msg::SetQuestions(Some(form)));
    }

    /// Dismiss the question form.
    pub fn clear_questions(&self) {
        let _ = self.tx.send(Msg::SetQuestions(None));
    }

    /// Await the user's submitted answers. `None` if the REPL exited (e.g. the
    /// user aborted with `Esc` and the form was cleared).
    pub async fn recv_answers(&self) -> Option<FormAnswers> {
        let mut rx = self.answers_rx.lock().await;
        rx.recv().await
    }

    /// Await an Esc-abort signal. `None` if the REPL exited.
    pub async fn recv_abort(&self) -> Option<()> {
        let mut rx = self.abort_rx.lock().await;
        rx.recv().await
    }

    /// Await the next mid-run STEERING message (Enter while the agent is
    /// working). `None` if the REPL exited. The host injects it into the
    /// running turn (typically via a queue its harness drains at turn
    /// boundaries).
    pub async fn recv_steer(&self) -> Option<String> {
        let mut rx = self.steer_rx.lock().await;
        rx.recv().await
    }

    /// Drain any queued FOLLOW-UP messages (Alt+Enter while working) without
    /// blocking. The host calls this after a turn finishes and runs each as
    /// the next prompt. Returns an empty vec when the receiver is busy or
    /// nothing is queued.
    pub fn drain_follow_ups(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(mut rx) = self.follow_up_rx.try_lock() {
            while let Ok(text) = rx.try_recv() {
                out.push(text);
            }
        }
        out
    }

    /// Drop any abort signals queued before a turn started, so a stale Esc
    /// can't abort the next turn.
    pub fn drain_abort(&self) {
        if let Ok(mut rx) = self.abort_rx.try_lock() {
            while rx.try_recv().is_ok() {}
        }
    }

    /// Drop every transcript block (stream items, tool blocks, scroll, focus).
    /// For hosts whose conversation state moved somewhere the append-only
    /// transcript can't represent — a rewind, a branch switch, a clear — so
    /// they can re-render the new state from scratch instead of appending
    /// below stale history. Tool handles from before the clear go stale;
    /// updating one is a silent no-op.
    pub fn clear_transcript(&self) {
        let _ = self.tx.send(Msg::ClearTranscript);
    }

    /// Replace the composer's input buffer with `text`, cursor at the end.
    /// For restoring unsent input — e.g. steering/follow-up text the user typed
    /// mid-turn that an Esc-abort would otherwise swallow. Overwrites whatever
    /// is in the editor.
    pub fn set_editor_text(&self, text: impl Into<String>) {
        let _ = self.tx.send(Msg::SetEditorText(text.into()));
    }
}
