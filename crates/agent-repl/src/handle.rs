//! Channel-based control surface for the REPL. Agent task pushes events
//! in; user input flows out via `recv_input`.

use std::sync::atomic::{AtomicU64, Ordering};

use agent_repl_core::{Event, ToolCall};
use tokio::sync::{mpsc, Mutex};

use crate::stream::ToolId;

#[derive(Debug)]
pub(crate) enum Msg {
    Append(Event),
    AppendTool(ToolId, ToolCall),
    UpdateTool(ToolId, ToolCall),
    SetWorking(bool),
}

/// Opaque handle to a tool block already in the stream. Used to update it
/// later (e.g. when the tool finishes).
#[derive(Debug, Clone, Copy)]
pub struct ToolHandle(pub(crate) ToolId);

pub struct ReplHandle {
    pub(crate) tx: mpsc::UnboundedSender<Msg>,
    pub(crate) input_rx: Mutex<mpsc::UnboundedReceiver<String>>,
    pub(crate) next_id: AtomicU64,
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
}
