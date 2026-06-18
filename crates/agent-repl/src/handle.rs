//! Channel-based control surface for the REPL. Agent task pushes events
//! in; user input flows out via `recv_input`.

use std::sync::atomic::{AtomicU64, Ordering};

use agent_repl_core::{ApprovalChoice, ApprovalPrompt, Event, FormAnswers, QuestionForm, ToolCall};
use tokio::sync::{mpsc, Mutex};

use crate::stream::ToolId;

#[derive(Debug)]
pub(crate) enum Msg {
    Append(Event),
    AppendTool(ToolId, ToolCall),
    UpdateTool(ToolId, ToolCall),
    SetWorking(bool),
    // looper bolt-on: show/hide a three-level approval prompt.
    SetApproval(Option<ApprovalPrompt>),
    // looper bolt-on: show/hide a tabbed question form.
    SetQuestions(Option<QuestionForm>),
}

/// Opaque handle to a tool block already in the stream. Used to update it
/// later (e.g. when the tool finishes).
#[derive(Debug, Clone, Copy)]
pub struct ToolHandle(pub(crate) ToolId);

pub struct ReplHandle {
    pub(crate) tx: mpsc::UnboundedSender<Msg>,
    pub(crate) input_rx: Mutex<mpsc::UnboundedReceiver<String>>,
    pub(crate) next_id: AtomicU64,
    // looper bolt-ons: Esc-abort signal + approval-choice delivery, both flowing
    // from the renderer's key handler back to the driving task.
    pub(crate) abort_rx: Mutex<mpsc::UnboundedReceiver<()>>,
    pub(crate) approval_rx: Mutex<mpsc::UnboundedReceiver<ApprovalChoice>>,
    // looper bolt-on: completed question-form answers flowing back.
    pub(crate) answers_rx: Mutex<mpsc::UnboundedReceiver<FormAnswers>>,
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

    /// Await the next line of user input. Returns `None` if the REPL has
    /// exited. Flips the composer to `working = false` while it waits so
    /// the prompt is responsive.
    pub async fn recv_input(&self) -> Option<String> {
        let _ = self.tx.send(Msg::SetWorking(false));
        let mut rx = self.input_rx.lock().await;
        rx.recv().await
    }

    // ---- looper bolt-ons --------------------------------------------------

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

    /// Drop any abort signals queued before a turn started, so a stale Esc
    /// can't abort the next turn.
    pub fn drain_abort(&self) {
        if let Ok(mut rx) = self.abort_rx.try_lock() {
            while rx.try_recv().is_ok() {}
        }
    }
}
