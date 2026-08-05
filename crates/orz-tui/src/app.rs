//! TuiApp — the compositor: aggregates widget state, routes focus/keys,
//! stacks dialogs, maintains navigation history, and delegates rendering
//! to `widgets.rs`. The UI is a view model projected from canonical events
//! (CLI_UI_INTERACTION_MODEL_v0.1) — this struct never becomes the kernel.

use std::io;

use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::dialogs::{Dialog, DialogAction};
use crate::events::TuiEvent;
use crate::theme::{MIN_HEIGHT, MIN_WIDTH};
use crate::view_model::{
    ChatInputState, ContentMarker, ContentPane, MenuBar, StatusBar, Toolbar,
};

/// Hard floor below which the layout cannot render (soft floor is
/// MIN_WIDTH×MIN_HEIGHT from the design doc).
pub const ABSOLUTE_MIN_WIDTH: u16 = 60;
pub const ABSOLUTE_MIN_HEIGHT: u16 = 15;

/// Which pane owns keyboard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Chat,
    Dialog,
    Neutral,
}

/// What the user chose in the permission dialog. The app layer maps this to
/// the ACP response via the stored closure (keeps acp types out of the UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOutcome {
    AllowOnce,
    Cancelled,
}

/// A pending interactive permission request awaiting the dialog.
///
/// The respond closure runs on the main loop thread (the ACP handler that
/// installs it runs in a `spawn_local` task on the same thread), so it does
/// not need to be `Send`.
pub struct PendingPermission {
    pub tool: String,
    pub args_summary: String,
    /// Fires the ACP response (set by the client wiring).
    pub respond: Option<Box<dyn FnOnce(PermissionOutcome)>>,
    /// When the dialog was presented — the countdown's start point. Re-stamped
    /// on presentation so QUEUED requests count down from display, not from
    /// arrival (Phase 3 slice #7).
    pub opened_at: std::time::Instant,
}

impl PendingPermission {
    /// New request stamped with the current instant.
    pub fn new(
        tool: impl Into<String>,
        args_summary: impl Into<String>,
        respond: Option<Box<dyn FnOnce(PermissionOutcome)>>,
    ) -> Self {
        Self {
            tool: tool.into(),
            args_summary: args_summary.into(),
            respond,
            opened_at: std::time::Instant::now(),
        }
    }

    /// Seconds remaining until the host's permission-prompt timeout denies
    /// the request. `None` = already expired. The TUI derives the countdown
    /// from its own clock against the host's constant (single source of
    /// truth — `orz_host::permission::PERMISSION_PROMPT_TIMEOUT`); a few ms
    /// of drift between the two timers is benign (Phase 3 slice #7).
    pub fn remaining(&self) -> Option<std::time::Duration> {
        crate::PERMISSION_PROMPT_TIMEOUT
            .checked_sub(self.opened_at.elapsed())
    }
}

/// Outcome of a key event handled by the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOutcome {
    Continue,
    Quit,
    /// The chat input submitted a prompt (runner sends it to the host).
    Prompt(String),
    /// The user asked to stop the running session.
    CancelRun,
}

/// The workbench compositor.
pub struct TuiApp {
    // ── widget state (view model) ──
    pub menu_bar: MenuBar,
    pub toolbar: Toolbar,
    pub content: ContentPane,
    pub marker: ContentMarker,
    pub status: StatusBar,
    pub input: ChatInputState,

    // ── interaction state ──
    pub focus: Focus,
    pub dialog: Option<Dialog>,
    pub pending_permission: Option<PendingPermission>,
    /// One modal at a time — multi-tool rounds can queue several requests
    /// (review P2-2b); the next one is presented when the current closes.
    pub permission_queue: std::collections::VecDeque<PendingPermission>,
    pub running: bool,
    /// `/stop` was typed while running — the runner owns the async cancel
    /// (Phase 3 slice #7; app state only marks intent so key handling stays
    /// sync).
    pub stop_pending: bool,
    pub quit: bool,
    /// Prompt counter for card turn labels (each prompt = one run).
    pub turn_counter: u32,

    // ── navigation history (object URIs, v1 minimal) ──
    pub nav_back: Vec<String>,
    pub nav_forward: Vec<String>,

    // ── view toggles ──
    pub show_marker: bool,

    // ── event log (diagnostics surface, not the content pane) ──
    pub events_log: Vec<String>,
    pub max_events_log: usize,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            menu_bar: MenuBar::new(),
            toolbar: Toolbar::new(),
            content: ContentPane::default(),
            marker: ContentMarker::default(),
            status: StatusBar::new(),
            input: ChatInputState::new(),
            focus: Focus::Chat,
            dialog: None,
            pending_permission: None,
            permission_queue: std::collections::VecDeque::new(),
            running: false,
            stop_pending: false,
            quit: false,
            turn_counter: 0,
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
            show_marker: true,
            events_log: Vec::new(),
            max_events_log: 500,
        }
    }

    /// Whether the current viewport can host the layout at all.
    pub fn viewport_too_small(w: u16, h: u16) -> bool {
        w < ABSOLUTE_MIN_WIDTH || h < ABSOLUTE_MIN_HEIGHT
    }

    /// Design-target viewport check (informational).
    pub fn viewport_below_min(w: u16, h: u16) -> bool {
        w < MIN_WIDTH || h < MIN_HEIGHT
    }

    // ── event ingestion ──

    /// Accept one projected event: append to the event log and let the
    /// projection layer mutate the view model (returns status messages).
    pub fn accept_event(&mut self, event: TuiEvent) -> Vec<String> {
        self.events_log.push(event.to_string());
        // Trim after push so the log never exceeds max (review P3 #5: drain
        // must not exceed len even when max_events_log is 0).
        let excess = self.events_log.len().saturating_sub(self.max_events_log);
        if excess > 0 {
            self.events_log.drain(0..excess);
        }
        crate::projection::apply_event(self, event)
    }

    // ── dialog management ──

    /// Open a dialog, stealing focus.
    pub fn open_dialog(&mut self, dialog: Dialog) {
        self.focus = Focus::Dialog;
        self.dialog = Some(dialog);
    }

    /// Close the active dialog, returning focus to the chat input, then
    /// present the next queued permission request (one modal at a time).
    pub fn close_dialog(&mut self) {
        self.dialog = None;
        self.pending_permission = None;
        self.focus = Focus::Chat;
        self.next_permission();
    }

    /// Esc path: fire a Cancelled response for a pending permission (Python
    /// Esc = cancel), then close the dialog.
    pub fn cancel_dialog(&mut self) {
        if let Some(pp) = self.pending_permission.take()
            && let Some(respond) = pp.respond
        {
            respond(PermissionOutcome::Cancelled);
        }
        self.close_dialog();
    }

    /// User confirmed the dialog's selected action.
    pub fn confirm_dialog(&mut self) -> Option<DialogAction> {
        let action = self
            .dialog
            .as_ref()
            .and_then(|d| d.selected_action())
            .cloned();
        match &action {
            Some(DialogAction::AllowOnce { .. }) => {
                if let Some(pp) = self.pending_permission.take()
                    && let Some(respond) = pp.respond
                {
                    respond(PermissionOutcome::AllowOnce);
                }
            }
            Some(DialogAction::Cancel) => {
                if let Some(pp) = self.pending_permission.take()
                    && let Some(respond) = pp.respond
                {
                    respond(PermissionOutcome::Cancelled);
                }
            }
            _ => {}
        }
        self.close_dialog();
        action
    }

    /// Show the permission dialog for a pending request. If another modal
    /// is already open, the request is queued (review P2-2b — the first
    /// request must never be silently dropped).
    pub fn show_permission_dialog(&mut self, pending: PendingPermission) {
        if self.pending_permission.is_some() || self.dialog.is_some() {
            self.permission_queue.push_back(pending);
            return;
        }
        self.present_permission(pending);
    }

    fn present_permission(&mut self, mut pending: PendingPermission) {
        // Countdown restarts at presentation — a queued request's dialog
        // starts its 300s from the moment it is shown (Phase 3 slice #7).
        pending.opened_at = std::time::Instant::now();
        self.status.set_run_state("等待审批", true);
        let mut message = vec![
            format!("工具: {}", pending.tool),
            format!("参数: {}", pending.args_summary),
        ];
        if pending.args_summary.is_empty() {
            message.pop();
            message.push("参数: (无摘要)".into());
        }
        let dialog = Dialog::new(
            "工具权限请求",
            message,
            vec![
                DialogAction::AllowOnce { option_id: "allow-once".into() },
                DialogAction::Cancel,
            ],
        );
        self.pending_permission = Some(pending);
        self.open_dialog(dialog);
    }

    fn next_permission(&mut self) {
        if let Some(next) = self.permission_queue.pop_front() {
            self.present_permission(next);
        }
    }

    /// Whether the pending permission dialog has exceeded the host's 300s
    /// prompt timeout (Phase 3 slice #7). The runner polls this on its tick
    /// and dismisses the dialog — the host has already denied (or denies
    /// milliseconds later), so the run continues without an orphan dialog.
    pub fn permission_timed_out(&self) -> bool {
        self.pending_permission
            .as_ref()
            .is_some_and(|pp| pp.remaining().is_none())
    }

    /// Close a *permission* dialog without responding (run ended, timeout).
    /// Non-permission dialogs (e.g. /help) are left alone; the run-state
    /// segment keeps whatever the terminal event set (完成/失败) instead of
    /// being overwritten back to 空闲.
    pub fn dismiss_permission_dialog(&mut self, reason: &str) {
        if self.pending_permission.take().is_some() {
            self.dialog = None;
            self.focus = Focus::Chat;
            if self.status.items.iter().any(|i| i.label == "等待审批") {
                self.status.set_run_state("空闲", true);
            }
            self.content
                .add_system_message(&format!("请求已取消（{reason}）"), true);
            self.next_permission();
        }
    }

    // ── navigation history ──

    /// Push an object URI onto the back stack (v1: /open records only).
    pub fn push_uri(&mut self, uri: &str) {
        self.nav_back.push(uri.to_string());
        self.nav_forward.clear();
        self.toolbar.set_enabled("后退", !self.nav_back.is_empty());
    }

    pub fn nav_backward(&mut self) -> Option<String> {
        let uri = self.nav_back.pop()?;
        self.nav_forward.push(uri.clone());
        self.toolbar.set_enabled("前进", !self.nav_forward.is_empty());
        self.toolbar.set_enabled("后退", !self.nav_back.is_empty());
        Some(uri)
    }

    pub fn nav_forward_move(&mut self) -> Option<String> {
        let uri = self.nav_forward.pop()?;
        self.nav_back.push(uri.clone());
        self.toolbar.set_enabled("前进", !self.nav_forward.is_empty());
        self.toolbar.set_enabled("后退", !self.nav_back.is_empty());
        Some(uri)
    }

    // ── slash commands (v1 subset) ──

    /// Dispatch a `/command` line; returns status messages.
    pub fn dispatch_command(&mut self, line: &str) -> Vec<String> {
        let (slash, arg) = match line.split_once(' ') {
            Some((s, a)) => (s, a.trim()),
            None => (line, ""),
        };
        let mut messages = Vec::new();
        match slash {
            "/help" => {
                self.open_dialog(crate::dialogs::help_dialog());
                messages.push("帮助已打开".into());
            }
            "/status" => {
                let state = if self.running { "运行中" } else { "空闲" };
                messages.push(format!(
                    "状态: {state} | 来源 {} | 消息 {} 条",
                    self.status.items[3].label,
                    self.content.items.len()
                ));
            }
            "/stop" => {
                if self.running {
                    // The runner owns the async cancel; mark intent here so
                    // key handling stays synchronous (Phase 3 slice #7).
                    self.stop_pending = true;
                } else {
                    messages.push("当前没有运行".into());
                }
            }
            "/open" => {
                if arg.is_empty() {
                    messages.push("用法: /open <uri>".into());
                } else {
                    self.push_uri(arg);
                    self.content
                        .add_system_message(&format!("[打开] {arg}"), false);
                    messages.push(format!("已打开: {arg}"));
                }
            }
            "/toggle-markers" => {
                self.show_marker = !self.show_marker;
                messages.push(if self.show_marker {
                    "标记栏已显示".into()
                } else {
                    "标记栏已隐藏".into()
                });
            }
            "/toggle-explorer" | "/toggle-events" => {
                messages.push("探索器/事件栏尚未实现（后续切片）".into());
            }
            _ => {
                messages.push(format!("未知命令: {slash}（/help 查看）"));
            }
        }
        messages
    }

    // ── rendering ──

    /// Render the full frame through the shared widgets module.
    pub fn render<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        crate::widgets::render_frame(terminal, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn fired_cell() -> Rc<Cell<Option<PermissionOutcome>>> {
        Rc::new(Cell::new(None))
    }

    fn respond(cell: &Rc<Cell<Option<PermissionOutcome>>>) -> Box<dyn FnOnce(PermissionOutcome)> {
        let cell = cell.clone();
        Box::new(move |o| cell.set(Some(o)))
    }

    #[test]
    fn dialog_focus_and_confirm() {
        let mut app = TuiApp::new();
        let fired = fired_cell();
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(respond(&fired)),
        ));
        assert_eq!(app.focus, Focus::Dialog);
        assert!(app.dialog.is_some());
        assert_eq!(app.status.items[5].label, "等待审批");

        // Default selected action = Allow Once.
        app.confirm_dialog();
        assert_eq!(fired.get(), Some(PermissionOutcome::AllowOnce));
        assert_eq!(app.focus, Focus::Chat);
        assert!(app.dialog.is_none());
    }

    #[test]
    fn esc_cancel_fires_cancelled() {
        let mut app = TuiApp::new();
        let fired = fired_cell();
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(respond(&fired)),
        ));
        // Select Cancel via key path (dialog.select_next + confirm).
        app.dialog.as_mut().unwrap().select_next();
        app.confirm_dialog();
        assert_eq!(fired.get(), Some(PermissionOutcome::Cancelled));
    }

    #[test]
    fn second_permission_request_is_queued_not_dropped() {
        // Review P2-2b: a multi-tool round can deliver two requests back to
        // back — the first must never be silently overwritten.
        let mut app = TuiApp::new();
        let first = fired_cell();
        let second = fired_cell();
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(respond(&first)),
        ));
        app.show_permission_dialog(PendingPermission::new(
            "edit_file",
            "lib.rs",
            Some(respond(&second)),
        ));
        assert_eq!(app.permission_queue.len(), 1, "second request queued");

        // Resolve the first (Allow Once) → the queued one is presented.
        app.confirm_dialog();
        assert_eq!(first.get(), Some(PermissionOutcome::AllowOnce));
        assert_eq!(second.get(), None);
        assert!(app.dialog.is_some(), "queued request now shown");
        assert_eq!(app.permission_queue.len(), 0);

        // Esc on the second → Cancelled, no more queue.
        app.cancel_dialog();
        assert_eq!(second.get(), Some(PermissionOutcome::Cancelled));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn dismiss_keeps_terminal_run_state() {
        // Review P3 #1: run ended → 完成; dismissing a stale permission
        // dialog must not overwrite it back to 空闲.
        let mut app = TuiApp::new();
        let fired = fired_cell();
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(respond(&fired)),
        ));
        app.status.set_run_state("完成", true);
        app.dismiss_permission_dialog("运行已结束");
        assert_eq!(app.status.items[5].label, "完成");
        assert_eq!(fired.get(), None, "dismiss never fires a response");
    }

    #[test]
    fn dismiss_does_not_close_non_permission_dialogs() {
        // Review P3 #2: a run ending must not force-close e.g. the /help
        // overlay.
        let mut app = TuiApp::new();
        app.open_dialog(crate::dialogs::help_dialog());
        app.dismiss_permission_dialog("运行已结束");
        assert!(app.dialog.is_some(), "help overlay stays open");
    }

    #[test]
    fn dismiss_clears_without_firing() {
        let mut app = TuiApp::new();
        let fired = Rc::new(Cell::new(false));
        let fired_clone = fired.clone();
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(Box::new(move |_o| fired_clone.set(true))),
        ));
        app.dismiss_permission_dialog("运行已结束");
        assert!(!fired.get());
        assert!(app.dialog.is_none());
        assert_eq!(app.focus, Focus::Chat);
    }

    #[test]
    fn nav_history_updates_toolbar() {
        let mut app = TuiApp::new();
        assert!(!app.toolbar.items[0].enabled); // 后退 disabled
        app.push_uri("run://2026-08-05/0001");
        assert!(app.toolbar.items[0].enabled);
        assert_eq!(app.nav_backward(), Some("run://2026-08-05/0001".into()));
        assert!(app.toolbar.items[1].enabled); // 前进 enabled
        assert_eq!(app.nav_forward_move(), Some("run://2026-08-05/0001".into()));
    }

    #[test]
    fn events_log_is_bounded() {
        let mut app = TuiApp::new();
        app.max_events_log = 3;
        for i in 0..5 {
            app.accept_event(TuiEvent::StatusUpdate {
                label: format!("e{i}"),
                ok: true,
            });
        }
        assert_eq!(app.events_log.len(), 3);

        // max_events_log = 0 must not panic (review P3 #5).
        let mut app = TuiApp::new();
        app.max_events_log = 0;
        app.accept_event(TuiEvent::StatusUpdate { label: "x".into(), ok: true });
        assert!(app.events_log.is_empty());
    }

    #[test]
    fn viewport_floors() {
        assert!(TuiApp::viewport_too_small(30, 10));
        assert!(!TuiApp::viewport_too_small(80, 24));
        assert!(TuiApp::viewport_below_min(80, 24));
        assert!(!TuiApp::viewport_below_min(140, 40));
    }

    #[test]
    fn close_dialog_returns_to_chat() {
        let mut app = TuiApp::new();
        app.open_dialog(crate::dialogs::help_dialog());
        assert_eq!(app.focus, Focus::Dialog);
        app.close_dialog();
        assert_eq!(app.focus, Focus::Chat);
        assert!(app.dialog.is_none());
    }

    // ── Phase 3 slice #7: /stop intent + permission countdown ─────────────

    #[test]
    fn stop_command_marks_pending_only_when_running() {
        let mut app = TuiApp::new();
        // Idle: no pending flag, feedback message.
        let msgs = app.dispatch_command("/stop");
        assert!(!app.stop_pending);
        assert!(msgs.iter().any(|m| m == "当前没有运行"), "{msgs:?}");

        // Running: marks intent for the runner; no message.
        app.running = true;
        let msgs = app.dispatch_command("/stop");
        assert!(app.stop_pending, "/stop while running marks intent");
        assert!(msgs.is_empty(), "the runner owns the feedback: {msgs:?}");
    }

    #[test]
    fn pending_permission_remaining_expires_at_timeout() {
        use std::time::{Duration, Instant};

        // Fresh dialog: counts down from ~300s (host single source of truth).
        let fresh = PendingPermission::new("bash", "dir", None);
        let r = fresh.remaining().unwrap();
        assert!(
            r > Duration::from_secs(299),
            "fresh dialog has ~300s left, got {r:?}"
        );

        // Backdated 299s: ~1s left (no clock injection — Instant arithmetic).
        let mut old = PendingPermission::new("bash", "dir", None);
        old.opened_at = Instant::now() - Duration::from_secs(299);
        let r = old.remaining().unwrap();
        assert!(r.as_secs() <= 1, "299s elapsed leaves ≤1s, got {r:?}");

        // Backdated 301s: expired.
        let mut expired = PendingPermission::new("bash", "dir", None);
        expired.opened_at = Instant::now() - Duration::from_secs(301);
        assert!(expired.remaining().is_none(), "301s elapsed = expired");
    }

    #[test]
    fn timed_out_permission_dismisses_without_firing_and_presents_queued() {
        use std::time::{Duration, Instant};

        let mut app = TuiApp::new();
        let fired = fired_cell();
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(respond(&fired)),
        ));
        // Backdate AFTER presentation — `present_permission` re-stamps the
        // open instant (the countdown starts when the dialog is shown).
        let expired = app.pending_permission.as_mut().unwrap();
        expired.opened_at = Instant::now() - Duration::from_secs(301);
        // A second request queues behind the expired one.
        app.show_permission_dialog(PendingPermission::new("edit_file", "lib.rs", None));
        assert!(app.permission_timed_out(), "expired dialog detected");

        app.dismiss_permission_dialog("已超时");
        assert_eq!(
            fired.get(),
            None,
            "timeout dismissal never fires a response"
        );
        let messages: Vec<String> = app
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                crate::view_model::ContentItem::Message(m)
                    if m.role == "系统" =>
                {
                    Some(m.content.clone())
                }
                _ => None,
            })
            .collect();
        assert!(
            messages.iter().any(|m| m.contains("请求已取消（已超时）")),
            "timeout dismissal surfaces the reason: {messages:?}"
        );
        assert!(app.dialog.is_some(), "queued request now presented");
        assert_eq!(app.permission_queue.len(), 0);
        // The queued request counts down from PRESENTATION (re-stamped).
        let shown = app.pending_permission.as_ref().unwrap();
        assert_eq!(shown.tool, "edit_file");
        assert!(
            shown.remaining().unwrap() > Duration::from_secs(299),
            "queued dialog restarts its ~300s countdown on presentation"
        );
    }
}
