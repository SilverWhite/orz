//! App state for the codex-style fallback TUI (Phase 3 slice #12).
//!
//! Minimal by design (§2.4): a message list, the prompt input, a status line
//! and the approval dialog — **no assurance panels** (explorer/properties/
//! find/markers/snapshots/run-history). The assurance layer runs silently in
//! orz-host; this app only renders what the codex protocol surfaces
//! (streamed items, turn terminals, approval requests).

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serde_json::Value;
use xai_ratatui_textarea::TextArea;

/// Permission-prompt bound — the orz-host constant is the single source of
/// truth (the host denies at ~300s; the dialog countdown mirrors it).
pub use orz_host::permission::PERMISSION_PROMPT_TIMEOUT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Idle,
    Running,
    WaitingApproval,
    Interrupted,
    Failed,
    Completed,
}

impl RunState {
    pub fn label(self) -> &'static str {
        match self {
            RunState::Idle => "空闲",
            RunState::Running => "运行中",
            RunState::WaitingApproval => "等待审批",
            RunState::Interrupted => "已中断",
            RunState::Failed => "失败",
            RunState::Completed => "完成",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Agent,
    System,
}

#[derive(Debug)]
pub struct MessageEntry {
    pub role: Role,
    pub text: String,
    /// Agent message currently receiving deltas.
    pub streaming: bool,
}

/// The approval dialog — mirrors orz-tui's permission dialog interaction
/// (允许一次 / 允许 / 拒绝 + countdown), answered over the codex channel.
#[derive(Debug)]
pub struct ApprovalDialog {
    pub id: Value,
    pub params: Value,
    pub opened_at: Instant,
    pub selected: usize,
}

impl ApprovalDialog {
    pub const OPTIONS: [&'static str; 3] = ["允许一次", "允许", "拒绝"];
    pub const DECISIONS: [&'static str; 3] = ["allow_once", "allow", "deny"];

    pub fn timed_out(&self) -> bool {
        self.opened_at.elapsed() >= PERMISSION_PROMPT_TIMEOUT
    }

    pub fn remaining(&self) -> Duration {
        PERMISSION_PROMPT_TIMEOUT.saturating_sub(self.opened_at.elapsed())
    }

    pub fn cycle(&mut self, delta: isize) {
        let len = Self::OPTIONS.len() as isize;
        self.selected = ((self.selected as isize + delta).rem_euclid(len)) as usize;
    }

    /// The wire decision for the currently selected option.
    pub fn decision(&self) -> &'static str {
        Self::DECISIONS[self.selected]
    }
}

/// The fallback TUI's app state.
pub struct CodexApp {
    pub messages: Vec<MessageEntry>,
    pub input: TextArea,
    pub status: RunState,
    pub status_detail: Option<String>,
    pub dialog: Option<ApprovalDialog>,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub last_prompt: Option<String>,
    /// The streaming agent item: (turn_id, item_id, accumulated text).
    pub current_item: Option<(String, String, String)>,
    /// Sealed item ids — late deltas for these are dropped (the completion
    /// text is authoritative; recorded slice #12 boundary).
    sealed_items: HashSet<String>,
}

impl Default for CodexApp {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexApp {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: TextArea::new(),
            status: RunState::Idle,
            status_detail: None,
            dialog: None,
            thread_id: None,
            turn_id: None,
            last_prompt: None,
            current_item: None,
            sealed_items: HashSet::new(),
        }
    }

    pub fn push_message(&mut self, role: Role, text: String) {
        self.messages.push(MessageEntry {
            role,
            text,
            streaming: false,
        });
    }

    pub fn system(&mut self, text: impl Into<String>) {
        self.push_message(Role::System, text.into());
    }

    /// Accept a submitted prompt. Returns `false` (with a hint message) while
    /// a turn is running or an approval is pending.
    pub fn submit_prompt(&mut self, text: String) -> bool {
        if matches!(self.status, RunState::Running | RunState::WaitingApproval) {
            self.system("正在运行——请等待完成或按 Ctrl+Z 中断");
            return false;
        }
        let text = text.trim().to_owned();
        if text.is_empty() {
            return false;
        }
        self.push_message(Role::User, text.clone());
        self.last_prompt = Some(text);
        self.status = RunState::Running;
        self.turn_id = None;
        true
    }

    /// The runner recorded the turn id from the `turn/start` response.
    pub fn begin_turn(&mut self, turn_id: String) {
        self.turn_id = Some(turn_id);
    }

    /// Streamed agent text — appended to the active item, or a fresh
    /// streaming message. Deltas for sealed items are dropped.
    pub fn on_item_delta(&mut self, turn_id: String, item_id: String, text: String) {
        if self.sealed_items.contains(&item_id) {
            return;
        }
        if let Some((t, id, acc)) = &mut self.current_item
            && *t == turn_id
            && *id == item_id
        {
            acc.push_str(&text);
            if let Some(last) = self.messages.last_mut() {
                last.text.push_str(&text);
            }
            return;
        }
        self.current_item = Some((turn_id, item_id, text.clone()));
        self.messages.push(MessageEntry {
            role: Role::Agent,
            text,
            streaming: true,
        });
    }

    /// A completed item seals its id and, for agent messages, replaces the
    /// streamed accumulation with the authoritative completion text.
    pub fn on_item_completed(
        &mut self,
        turn_id: String,
        item_id: String,
        text: String,
        kind: String,
    ) {
        if kind != "agentMessage" {
            // userMessage items echo our own submissions — the app renders
            // its own, so the echo is ignored.
            return;
        }
        self.sealed_items.insert(item_id.clone());
        if let Some((t, id, _)) = &self.current_item
            && *t == turn_id
            && *id == item_id
        {
            if let Some(last) = self.messages.last_mut() {
                last.text = text.clone();
                last.streaming = false;
            }
            self.current_item = None;
            return;
        }
        // No streamed accumulation (empty/gate-intercepted output) — create
        // the message directly.
        self.push_message(Role::Agent, text);
    }

    /// A turn reached its terminal state.
    pub fn on_turn_completed(&mut self, turn_id: String, status: String, error: Option<String>) {
        self.status = match status.as_str() {
            "completed" => RunState::Completed,
            "interrupted" => {
                // Restore the last prompt for an easy retry (one-shot — the
                // user may have typed something new in the meantime).
                if let Some(prompt) = self.last_prompt.clone() {
                    self.input.set_text(&prompt);
                }
                RunState::Interrupted
            }
            _ => RunState::Failed,
        };
        self.status_detail = error;
        if self.turn_id.as_deref() == Some(turn_id.as_str()) {
            self.turn_id = None;
        }
        self.current_item = None;
    }

    /// A permission request arrived — surface the dialog.
    pub fn on_approval_request(&mut self, id: Value, params: Value) {
        self.dialog = Some(ApprovalDialog {
            id,
            params,
            opened_at: Instant::now(),
            selected: 0,
        });
        self.status = RunState::WaitingApproval;
    }

    /// Consume the dialog (the runner answers the approval on the wire).
    pub fn answer_approval(&mut self) {
        self.dialog = None;
        if self.status == RunState::WaitingApproval {
            self.status = RunState::Running;
        }
    }

    /// The user asked to stop (Ctrl+Z or `/stop`).
    pub fn cancel_requested(&mut self) {
        if matches!(self.status, RunState::Running | RunState::WaitingApproval) {
            self.dialog = None;
            self.status = RunState::Running;
            self.system("停止请求已发送（正在取消）");
        } else {
            self.system("当前没有运行");
        }
    }

    /// Failed a prompt/session operation — surface the error and stop the
    /// spinner.
    pub fn session_failed(&mut self, error: impl Into<String>) {
        self.status = RunState::Failed;
        self.status_detail = Some(error.into());
    }
}

/// What the runner should do with a key press.
pub enum KeyOutcome {
    Continue,
    /// Submit the prompt (text already taken from the input).
    Prompt(String),
    /// Request cancellation of the running turn.
    Interrupt,
    /// Answer the open approval dialog with a decision.
    AnswerApproval(&'static str),
    Quit,
}

/// Key routing. With a dialog open the keys cycle/answer it; otherwise Enter
/// submits, Ctrl+Z interrupts, Ctrl+C quits, and everything else goes to the
/// textarea.
pub fn handle_key(app: &mut CodexApp, key: KeyEvent) -> KeyOutcome {
    if key.kind != KeyEventKind::Press {
        return KeyOutcome::Continue;
    }
    if let Some(dialog) = &mut app.dialog {
        return match key.code {
            // Ctrl+C / Ctrl+Z stay live while the dialog is open (the status
            // line advertises them; review D3-2).
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                KeyOutcome::Quit
            }
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                KeyOutcome::Interrupt
            }
            KeyCode::Enter => KeyOutcome::AnswerApproval(dialog.decision()),
            KeyCode::Esc => KeyOutcome::AnswerApproval("deny"),
            KeyCode::Tab | KeyCode::Right => {
                dialog.cycle(1);
                KeyOutcome::Continue
            }
            KeyCode::Left => {
                dialog.cycle(-1);
                KeyOutcome::Continue
            }
            _ => KeyOutcome::Continue,
        };
    }
    match key.code {
        KeyCode::Enter => {
            let line = app.input.text().to_string();
            if line.trim().is_empty() {
                return KeyOutcome::Continue;
            }
            // `/stop` typed like orz-tui's slash command — interrupt.
            if line.trim() == "/stop" {
                app.input.set_text("");
                return KeyOutcome::Interrupt;
            }
            app.input.set_text("");
            KeyOutcome::Prompt(line)
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyOutcome::Quit,
        KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyOutcome::Interrupt
        }
        _ => {
            app.input.input(key);
            KeyOutcome::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt_msg(app: &CodexApp, text: &str) -> bool {
        app.messages
            .iter()
            .any(|m| m.role == Role::User && m.text == text)
    }

    #[test]
    fn submit_accepts_and_guards_while_running() {
        let mut app = CodexApp::new();
        assert!(app.submit_prompt("你好".to_owned()));
        assert!(prompt_msg(&app, "你好"));
        assert_eq!(app.status, RunState::Running);
        // While running, a second submit is rejected with a hint.
        assert!(!app.submit_prompt("第二个".to_owned()));
        assert!(!prompt_msg(&app, "第二个"));
        assert!(app.messages.iter().any(|m| m.text.contains("正在运行")));
    }

    #[test]
    fn submit_trims_empty_and_whitespace() {
        let mut app = CodexApp::new();
        assert!(!app.submit_prompt("   ".to_owned()));
        assert!(!app.submit_prompt(String::new()));
        assert!(app.messages.is_empty());
    }

    #[test]
    fn deltas_accumulate_into_one_streaming_message() {
        let mut app = CodexApp::new();
        app.on_item_delta("turn_1".into(), "item_1".into(), "终".into());
        app.on_item_delta("turn_1".into(), "item_1".into(), "局".into());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].text, "终局");
        assert!(app.messages[0].streaming);
    }

    #[test]
    fn completion_replaces_accumulated_text_and_seals() {
        let mut app = CodexApp::new();
        app.on_item_delta("turn_1".into(), "item_1".into(), "草稿".into());
        app.on_item_completed("turn_1".into(), "item_1".into(), "终局答案。".into(), "agentMessage".into());
        assert_eq!(app.messages[0].text, "终局答案。");
        assert!(!app.messages[0].streaming);
        // A late delta for the sealed item is dropped.
        app.on_item_delta("turn_1".into(), "item_1".into(), "迟到".into());
        assert_eq!(app.messages[0].text, "终局答案。");
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn completion_without_stream_creates_message_directly() {
        let mut app = CodexApp::new();
        app.on_item_completed("turn_1".into(), "item_2".into(), "空输出直接封存".into(), "agentMessage".into());
        assert_eq!(app.messages.len(), 1);
        assert!(!app.messages[0].streaming);
    }

    #[test]
    fn user_message_echo_items_are_ignored() {
        let mut app = CodexApp::new();
        app.on_item_completed("turn_1".into(), "item_1".into(), "你好".into(), "userMessage".into());
        assert!(app.messages.is_empty(), "the app renders its own submissions");
    }

    #[test]
    fn turn_completed_sets_terminal_state_and_clears_turn() {
        let mut app = CodexApp::new();
        app.begin_turn("turn_1".into());
        app.on_turn_completed("turn_1".into(), "completed".into(), None);
        assert_eq!(app.status, RunState::Completed);
        assert!(app.turn_id.is_none());

        let mut app = CodexApp::new();
        app.last_prompt = Some("重试我".into());
        app.begin_turn("turn_1".into());
        app.on_turn_completed("turn_1".into(), "interrupted".into(), None);
        assert_eq!(app.status, RunState::Interrupted);
        // The last prompt is restored for an easy retry.
        assert_eq!(app.input.text(), "重试我");

        let mut app = CodexApp::new();
        app.on_turn_completed("turn_1".into(), "failed".into(), Some("模型出错".into()));
        assert_eq!(app.status, RunState::Failed);
        assert_eq!(app.status_detail.as_deref(), Some("模型出错"));
    }

    #[test]
    fn approval_dialog_cycles_and_decides() {
        let mut app = CodexApp::new();
        app.on_approval_request(serde_json::json!(7), serde_json::json!({ "tool_name": "x" }));
        assert_eq!(app.status, RunState::WaitingApproval);
        let dialog = app.dialog.as_ref().unwrap();
        assert_eq!(dialog.decision(), "allow_once");
        app.dialog.as_mut().unwrap().cycle(1);
        assert_eq!(app.dialog.as_ref().unwrap().decision(), "allow");
        app.dialog.as_mut().unwrap().cycle(1);
        assert_eq!(app.dialog.as_ref().unwrap().decision(), "deny");
        app.dialog.as_mut().unwrap().cycle(1);
        assert_eq!(app.dialog.as_ref().unwrap().decision(), "allow_once", "wraps around");
        app.answer_approval();
        assert!(app.dialog.is_none());
        assert_eq!(app.status, RunState::Running);
    }

    #[test]
    fn cancel_requested_hints_or_acknowledges() {
        let mut app = CodexApp::new();
        app.cancel_requested();
        assert!(app.messages.iter().any(|m| m.text.contains("当前没有运行")));

        let mut app = CodexApp::new();
        app.status = RunState::Running;
        app.cancel_requested();
        assert!(app.messages.iter().any(|m| m.text.contains("正在取消")));
        assert_eq!(app.status, RunState::Running);
    }

    #[test]
    fn key_routing_submits_interrupts_and_quits() {
        use crossterm::event::{KeyCode, KeyModifiers};
        let mut app = CodexApp::new();
        app.input.set_text("你好");
        let outcome = handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(outcome, KeyOutcome::Prompt(t) if t == "你好"));
        assert!(app.input.text().is_empty(), "input cleared on submit");

        let outcome = handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
        );
        assert!(matches!(outcome, KeyOutcome::Interrupt));

        let outcome = handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(matches!(outcome, KeyOutcome::Quit));

        // `/stop` typed like a slash command.
        app.input.set_text("/stop");
        let outcome = handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(outcome, KeyOutcome::Interrupt));
    }

    #[test]
    fn dialog_keys_answer_and_cycle() {
        let mut app = CodexApp::new();
        app.on_approval_request(serde_json::json!(1), serde_json::json!({}));
        // Tab cycles; Enter answers with the selection; Esc denies.
        let outcome = handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(matches!(outcome, KeyOutcome::Continue));
        assert_eq!(app.dialog.as_ref().unwrap().selected, 1);
        let outcome = handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(outcome, KeyOutcome::AnswerApproval("allow")));
        app.answer_approval();

        app.on_approval_request(serde_json::json!(2), serde_json::json!({}));
        let outcome = handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(outcome, KeyOutcome::AnswerApproval("deny")));
    }
}
