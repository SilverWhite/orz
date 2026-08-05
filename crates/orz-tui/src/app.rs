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
    Explorer,
    Dialog,
    /// A tabbed modal (help / find / properties — slice #9).
    Modal,
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
    /// Tabbed modal (help / find / properties — slice #9), mutually
    /// exclusive with the permission dialog.
    pub modal: Option<crate::modals::Modal>,
    pub running: bool,
    /// `/stop` was typed while running — the runner owns the async cancel
    /// (Phase 3 slice #7; app state only marks intent so key handling stays
    /// sync).
    pub stop_pending: bool,
    /// A snapshot restore was confirmed in the selector — the runner owns
    /// the async call (slice #10; same intent-marking pattern as
    /// stop_pending).
    pub pending_restore: Option<String>,
    /// A restore is in flight. The host has no restore in-flight marker
    /// (record P2-2) and no restore cancellation token (record P3-7), so
    /// the TUI is the only guard — it blocks new prompts, re-entry and
    /// cancel while set.
    pub restoring: bool,
    pub quit: bool,
    /// Prompt counter for card turn labels (each prompt = one run).
    pub turn_counter: u32,

    // ── session / run identity ──
    /// Session cwd (explorer tree root; journal scan base). Set by the
    /// runner from TuiConfig; defaults to the process cwd for tests.
    pub cwd: std::path::PathBuf,
    /// ACP session id (set by the AcpSessionCreated projection).
    pub session_id: Option<String>,
    /// Last gate block label — the OSC title's ⚠ state (cleared on
    /// RunStarted; set on GateDecision/IPG block).
    pub gate_block: Option<String>,
    /// Last submitted prompt — restored into the input after a cancel
    /// (slice #9, Python `_cancel_run` parity).
    pub last_prompt: Option<String>,
    /// Last Esc timestamp — the double-Esc (<500 ms) session-list window.
    pub last_esc: Option<std::time::Instant>,

    // ── navigation history (object URIs, v1 minimal) ──
    pub nav_back: Vec<String>,
    pub nav_forward: Vec<String>,

    // ── view toggles ──
    pub show_explorer: bool,
    pub show_marker: bool,
    /// The explorer pane (tree + events + session list).
    pub explorer: crate::explorer::ExplorerPane,

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
            modal: None,
            running: false,
            stop_pending: false,
            pending_restore: None,
            restoring: false,
            quit: false,
            turn_counter: 0,
            cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            session_id: None,
            gate_block: None,
            last_prompt: None,
            last_esc: None,
            nav_back: Vec::new(),
            nav_forward: Vec::new(),
            // User ruling (2026-08-05): both sidebars default hidden —
            // the big-main-window layout is the default; `/toggle-*`
            // reveals the panels.
            show_explorer: false,
            show_marker: false,
            explorer: crate::explorer::ExplorerPane::default(),
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
    /// request must never be silently dropped). A tabbed modal never renders
    /// under/over the permission dialog (slice #9: the permission flow wins
    /// and queues).
    pub fn show_permission_dialog(&mut self, pending: PendingPermission) {
        if self.pending_permission.is_some() || self.dialog.is_some() || self.modal.is_some() {
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

    // ── tabbed modals (slice #9: help / find / properties) ──

    /// A modal can open only when no permission dialog is showing — the
    /// permission flow is blocking (the host is waiting on the response).
    fn modal_available(&self) -> bool {
        self.dialog.is_none() && self.pending_permission.is_none()
    }

    /// Returns true when the modal actually opened (false = a permission
    /// dialog is blocking — refusal message shown).
    pub fn open_help(&mut self) -> bool {
        if !self.modal_available() {
            self.queue_modal_refusal();
            return false;
        }
        let model = self.status.items[4].label.clone();
        self.modal = Some(crate::modals::Modal::Help(
            crate::modals::HelpOverlay::new(&model),
        ));
        self.focus = Focus::Modal;
        true
    }

    pub fn open_find(&mut self) -> bool {
        if !self.modal_available() {
            self.queue_modal_refusal();
            return false;
        }
        self.modal = Some(crate::modals::Modal::Find(Box::default()));
        self.focus = Focus::Modal;
        true
    }

    pub fn open_properties(&mut self) -> bool {
        if !self.modal_available() {
            self.queue_modal_refusal();
            return false;
        }
        self.modal = Some(crate::modals::Modal::Properties(
            crate::modals::PropertiesSheet::from_app(self),
        ));
        self.focus = Focus::Modal;
        true
    }

    /// Open the snapshot selector (slice #10): a fresh read-only scan at
    /// every open (v1 — no live refresh while the modal is open; the
    /// explorer tree has the same one-shot-load precedent). The selector is
    /// the approval surface for restore: restore has no permission flow by
    /// design (the host API is a user-explicit request, slice #8).
    pub fn open_snapshots(&mut self) -> bool {
        if !self.modal_available() {
            self.queue_modal_refusal();
            return false;
        }
        let entries = crate::snapshots::discover_snapshots(&self.cwd);
        self.modal = Some(crate::modals::Modal::Snapshots(
            crate::modals::SnapshotSelector::new(entries),
        ));
        self.focus = Focus::Modal;
        true
    }

    // ── snapshot selector key helpers (input.rs routes here to avoid
    //    borrowing the modal while mutating the app) ──

    pub fn snapshots_select_next(&mut self) {
        if let Some(crate::modals::Modal::Snapshots(s)) = &mut self.modal
            && !s.confirm // review P3-4: lock the selection mid-confirm
        {
            s.select_next();
        }
    }

    pub fn snapshots_select_prev(&mut self) {
        if let Some(crate::modals::Modal::Snapshots(s)) = &mut self.modal
            && !s.confirm
        {
            s.select_prev();
        }
    }

    /// Esc: from the confirm state, cascade back to selection; otherwise
    /// close the selector (and present a queued permission, if any).
    pub fn snapshots_esc(&mut self) {
        if let Some(crate::modals::Modal::Snapshots(s)) = &mut self.modal
            && s.confirm
        {
            s.confirm = false;
            return;
        }
        self.close_modal();
    }

    /// Enter: the first press arms the confirm state on the selected row;
    /// the second records the restore intent (`pending_restore`) and closes
    /// the modal. Refused while a run or another restore is in flight (the
    /// host would reject the restore with InvalidRequest anyway — refuse
    /// here for UX). Returns true when a restore intent was armed.
    pub fn snapshots_confirm(&mut self) -> bool {
        let hash = match &mut self.modal {
            Some(crate::modals::Modal::Snapshots(s)) if s.entries.is_empty() => {
                return false; // empty list: silent no-op, no confirm state
            }
            Some(crate::modals::Modal::Snapshots(s)) if !s.confirm => {
                s.confirm = true;
                return false;
            }
            Some(crate::modals::Modal::Snapshots(s)) => s
                .entries
                .get(s.selected)
                .map(|e| e.hash.clone()),
            _ => None,
        };
        if self.running || self.restoring {
            self.content
                .add_system_message("当前运行中——请等待完成后再恢复", false);
            return false;
        }
        let Some(hash) = hash else {
            return false; // unreachable: entries checked non-empty above
        };
        // Review D2-3: any queued permission belongs to a run that already
        // ended (confirm is refused while running) — its oneshot is dead.
        // Presenting it after close_modal would leave a stale dialog up for
        // the whole 300s countdown for no reason.
        self.permission_queue.clear();
        self.pending_restore = Some(hash);
        self.close_modal(); // presents a queued permission if any
        true
    }

    fn queue_modal_refusal(&mut self) {
        self.content
            .add_system_message("请先处理权限请求", false);
    }

    /// Close the modal, returning to chat, then present a queued permission
    /// request (its 300s countdown still starts at presentation).
    pub fn close_modal(&mut self) {
        self.modal = None;
        self.focus = Focus::Chat;
        self.next_permission();
    }

    /// Tab/←→ key helpers for the tabbed modals (input.rs routes here to
    /// avoid borrowing the modal while mutating the app).
    pub fn modal_next_tab(&mut self) {
        if let Some(m) = &mut self.modal {
            match m {
                crate::modals::Modal::Help(h) => h.sheet.next_tab(),
                crate::modals::Modal::Properties(p) => p.sheet.next_tab(),
                crate::modals::Modal::Find(_) => {}
                crate::modals::Modal::Snapshots(_) => {} // single list, no tabs
            }
        }
    }

    pub fn modal_prev_tab(&mut self) {
        if let Some(m) = &mut self.modal {
            match m {
                crate::modals::Modal::Help(h) => h.sheet.prev_tab(),
                crate::modals::Modal::Properties(p) => p.sheet.prev_tab(),
                crate::modals::Modal::Find(_) => {}
                crate::modals::Modal::Snapshots(_) => {} // single list, no tabs
            }
        }
    }

    pub fn find_focus_row(&self) -> usize {
        match &self.modal {
            Some(crate::modals::Modal::Find(f)) => f.focus_row,
            _ => 0,
        }
    }

    pub fn find_focus_next(&mut self) {
        if let Some(crate::modals::Modal::Find(f)) = &mut self.modal {
            f.focus_next();
        }
    }

    pub fn find_focus_prev(&mut self) {
        if let Some(crate::modals::Modal::Find(f)) = &mut self.modal {
            f.focus_prev();
        }
    }

    pub fn find_cycle_scope(&mut self) {
        if let Some(crate::modals::Modal::Find(f)) = &mut self.modal {
            f.cycle_scope();
        }
    }

    pub fn find_toggle_case(&mut self) {
        if let Some(crate::modals::Modal::Find(f)) = &mut self.modal {
            f.case_sensitive = !f.case_sensitive;
        }
    }

    pub fn find_toggle_regex(&mut self) {
        if let Some(crate::modals::Modal::Find(f)) = &mut self.modal {
            f.use_regex = !f.use_regex;
        }
    }

    /// Delegate an editing key to the Find query textarea (multiline).
    pub fn find_input(&mut self, key: crossterm::event::KeyEvent) {
        if let Some(crate::modals::Modal::Find(f)) = &mut self.modal {
            f.query.input(key);
        }
    }

    /// Execute the Find dialog's search (Enter on the action row): scan the
    /// conversation, replace the marker's search hits, close, and report the
    /// count. Invalid regex patterns surface as a status message.
    pub fn execute_find(&mut self) -> Vec<String> {
        let Some(crate::modals::Modal::Find(find)) = &self.modal else {
            return Vec::new();
        };
        let query = find.query.text().to_string();
        let case = find.case_sensitive;
        let regex = find.use_regex;
        if query.trim().is_empty() {
            self.close_modal();
            return Vec::new();
        }
        match self.content.find(&query, case, regex) {
            Ok(hits) => {
                self.marker.clear_search_hits();
                for idx in &hits {
                    self.marker.add_search_hit(idx + 1);
                }
                self.close_modal();
                let mut msg = format!("查找: {query} — {} 处匹配", hits.len());
                // Review D2-2: the hit markers live in the marker column,
                // which is default-hidden — point at the toggle so the
                // search's output is reachable.
                if !hits.is_empty() && !self.show_marker {
                    msg.push_str("（标记栏已隐藏——/toggle-markers 查看位置）");
                }
                vec![msg]
            }
            Err(e) => {
                self.close_modal();
                vec![format!("查找失败: {e}")]
            }
        }
    }

    // ── input restore (slice #9: cancel → restore the prompt) ──

    /// Restore the last submitted prompt into the chat input — only when
    /// the input is currently empty (a user typing a new prompt is never
    /// clobbered). One-shot: consuming the stored prompt.
    pub fn restore_last_prompt(&mut self) {
        let Some(prompt) = self.last_prompt.take() else {
            return;
        };
        if self.input.textarea.text().is_empty() {
            self.input.textarea.set_text(&prompt);
        }
    }

    // ── navigation history ──

    /// Open an object URI — push onto the back stack + surface the action
    /// (v1: /open and the explorer file rows share this path).
    pub fn open_uri(&mut self, uri: &str) {
        self.push_uri(uri);
        self.content.add_system_message(&format!("[打开] {uri}"), false);
    }

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
                if self.open_help() {
                    messages.push("帮助已打开".into());
                }
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
                    self.open_uri(arg);
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
            "/toggle-explorer" => {
                self.show_explorer = !self.show_explorer;
                messages.push(if self.show_explorer {
                    "探索器已显示".into()
                } else {
                    "探索器已隐藏".into()
                });
            }
            "/toggle-events" => {
                self.explorer.show_events = !self.explorer.show_events;
                messages.push(if self.explorer.show_events {
                    "事件栏已显示".into()
                } else {
                    "事件栏已隐藏".into()
                });
            }
            "/properties" => {
                if self.open_properties() {
                    messages.push("属性已打开".into());
                }
            }
            "/snapshots" => {
                if self.open_snapshots() {
                    messages.push("快照已打开".into());
                }
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
        // Review P3 #2: a run ending must not force-close e.g. a plain
        // dialog (the permission flow only touches its own dialog).
        let mut app = TuiApp::new();
        app.open_dialog(Dialog::new(
            "通知",
            vec!["x".into()],
            vec![DialogAction::Ok],
        ));
        app.dismiss_permission_dialog("运行已结束");
        assert!(app.dialog.is_some(), "non-permission dialog stays open");
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
        app.open_dialog(Dialog::new("x", vec![], vec![DialogAction::Ok]));
        assert_eq!(app.focus, Focus::Dialog);
        app.close_dialog();
        assert_eq!(app.focus, Focus::Chat);
        assert!(app.dialog.is_none());
    }

    // ── Phase 3 slice #9: modals / toggles / input restore ─────────────────

    #[test]
    fn modals_open_close_and_are_mutually_exclusive() {
        let mut app = TuiApp::new();
        assert!(app.open_help());
        assert_eq!(app.focus, Focus::Modal);
        assert!(matches!(
            app.modal,
            Some(crate::modals::Modal::Help(_))
        ));
        // Reopening replaces the current modal (one at a time).
        assert!(app.open_find());
        assert!(matches!(
            app.modal,
            Some(crate::modals::Modal::Find(_))
        ));
        assert!(app.open_properties());
        assert!(matches!(
            app.modal,
            Some(crate::modals::Modal::Properties(_))
        ));
        assert!(app.open_snapshots());
        assert!(matches!(
            app.modal,
            Some(crate::modals::Modal::Snapshots(_))
        ));
        app.close_modal();
        assert!(app.modal.is_none());
        assert_eq!(app.focus, Focus::Chat);
    }

    #[test]
    fn permission_queued_while_modal_open_and_presented_on_close() {
        let mut app = TuiApp::new();
        let fired = fired_cell();
        app.open_find();
        // A permission request arriving under a modal queues (the modal
        // must not be silently dropped).
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(respond(&fired)),
        ));
        assert_eq!(app.permission_queue.len(), 1);
        assert!(app.modal.is_some(), "modal stays up");
        assert!(app.dialog.is_none(), "permission never renders under modal");

        app.close_modal();
        assert!(app.dialog.is_some(), "queued permission presented on close");
        assert_eq!(app.permission_queue.len(), 0);
        assert_eq!(app.status.items[5].label, "等待审批");
    }

    #[test]
    fn modal_refused_while_permission_dialog_showing() {
        let mut app = TuiApp::new();
        app.show_permission_dialog(PendingPermission::new("bash", "dir", None));
        assert!(!app.open_help(), "permission flow is blocking");
        assert!(app.modal.is_none());
        let system = app.content.items.iter().any(|i| {
            matches!(
                i,
                crate::view_model::ContentItem::Message(m)
                    if m.content.contains("请先处理权限请求")
            )
        });
        assert!(system, "refusal message surfaced");
    }

    // ── Phase 3 slice #10: snapshot selector ───────────────────────────

    /// A hermetic cwd with one sealed `snapshot_created` journal (the loop
    /// records these per pre-mutation snapshot).
    fn snapshot_fixture_cwd() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-app-snapshots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let runs = dir.join(".gsa").join("runs").join("RUN-a1b2c3d4-0");
        std::fs::create_dir_all(&runs).unwrap();
        let mut ev = orz_assurance::journal::RunEvent::new(
            "RUN-a1b2c3d4-0".into(),
            0,
            orz_assurance::journal::EventType::SnapshotCreated,
            "m".into(),
            None,
            "run-event-v0.1.schema.json".into(),
            serde_json::json!({
                "tool": "search_replace",
                "targets": ["a.txt"],
                "snapshot_hash": "a".repeat(64)
            }),
            orz_assurance::journal::Redaction::None,
            "2026-08-05T00:00:00Z".into(),
        );
        orz_assurance::seal_event(&mut ev).unwrap();
        std::fs::write(
            runs.join("events.jsonl"),
            format!("{}\n", serde_json::to_string(&ev).unwrap()),
        )
        .unwrap();
        dir
    }

    fn system_has(app: &TuiApp, needle: &str) -> bool {
        app.content.items.iter().any(|i| {
            matches!(
                i,
                crate::view_model::ContentItem::Message(m)
                    if m.role == "系统" && m.content.contains(needle)
            )
        })
    }

    #[test]
    fn open_snapshots_scans_and_sets_focus() {
        let dir = snapshot_fixture_cwd();
        let mut app = TuiApp::new();
        app.cwd = dir.clone();
        assert!(app.open_snapshots());
        assert_eq!(app.focus, Focus::Modal);
        let crate::modals::Modal::Snapshots(sel) = app.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert_eq!(sel.entries.len(), 1);
        assert_eq!(sel.entries[0].hash, "a".repeat(64));
        assert_eq!(sel.entries[0].run_id, "RUN-a1b2c3d4-0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_snapshots_refused_while_permission_showing() {
        let mut app = TuiApp::new();
        app.show_permission_dialog(PendingPermission::new("bash", "dir", None));
        assert!(!app.open_snapshots(), "permission flow is blocking");
        assert!(app.modal.is_none());
        assert!(system_has(&app, "请先处理权限请求"));
    }

    #[test]
    fn snapshots_confirm_two_enter_arms_intent_and_closes() {
        let dir = snapshot_fixture_cwd();
        let mut app = TuiApp::new();
        app.cwd = dir.clone();
        app.open_snapshots();
        // First Enter: confirm state on the selected row, no intent yet.
        assert!(!app.snapshots_confirm());
        let crate::modals::Modal::Snapshots(sel) = app.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert!(sel.confirm);
        assert!(app.pending_restore.is_none());
        // Second Enter: restore intent armed, modal closed, focus back.
        assert!(app.snapshots_confirm());
        assert_eq!(app.pending_restore.as_deref(), Some("a".repeat(64).as_str()));
        assert!(app.modal.is_none());
        assert_eq!(app.focus, Focus::Chat);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshots_esc_cascades_confirm_then_closes() {
        let dir = snapshot_fixture_cwd();
        let mut app = TuiApp::new();
        app.cwd = dir.clone();
        app.open_snapshots();
        app.snapshots_confirm(); // → confirm state
        app.snapshots_esc();
        let crate::modals::Modal::Snapshots(sel) = app.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert!(!sel.confirm, "Esc cascades back to selection");
        assert!(app.modal.is_some(), "modal stays open");
        app.snapshots_esc();
        assert!(app.modal.is_none(), "second Esc closes");
        assert_eq!(app.focus, Focus::Chat);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshots_confirm_refused_while_running() {
        let dir = snapshot_fixture_cwd();
        let mut app = TuiApp::new();
        app.cwd = dir.clone();
        app.running = true;
        app.open_snapshots();
        app.snapshots_confirm(); // first Enter arms confirm
        assert!(!app.snapshots_confirm(), "second Enter refused while running");
        assert!(app.pending_restore.is_none());
        assert!(system_has(&app, "请等待完成后再恢复"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshots_confirm_on_empty_list_is_noop() {
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-app-snapshots-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = TuiApp::new();
        app.cwd = dir.clone();
        app.open_snapshots();
        assert!(!app.snapshots_confirm());
        assert!(!app.snapshots_confirm());
        assert!(app.pending_restore.is_none());
        let crate::modals::Modal::Snapshots(sel) = app.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert!(!sel.confirm, "empty list never arms confirm");
        assert!(app.modal.is_some(), "modal stays open");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn permission_queued_while_snapshot_selector_open() {
        let dir = snapshot_fixture_cwd();
        let mut app = TuiApp::new();
        app.cwd = dir.clone();
        let fired = fired_cell();
        app.open_snapshots();
        // A permission request arriving under the selector queues (the
        // existing modal-guard invariant, slice #9).
        app.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(respond(&fired)),
        ));
        assert_eq!(app.permission_queue.len(), 1);
        assert!(app.modal.is_some(), "modal stays up");
        assert!(app.dialog.is_none(), "permission never renders under modal");
        app.close_modal();
        assert!(app.dialog.is_some(), "queued permission presented on close");
        assert_eq!(app.permission_queue.len(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dismiss_permission_dialog_keeps_modal_closed_state() {
        // dismiss only fires with a permission dialog present — with no
        // pending permission it is a no-op and a modal stays open.
        let mut app = TuiApp::new();
        app.open_help();
        app.dismiss_permission_dialog("运行已结束");
        assert!(app.modal.is_some(), "dismiss never touches modals");
        assert_eq!(app.focus, Focus::Modal);
    }

    #[test]
    fn restore_last_prompt_only_when_input_empty() {
        let mut app = TuiApp::new();
        app.last_prompt = Some("被取消的提示".into());
        // Empty input → restored, one-shot.
        app.restore_last_prompt();
        assert_eq!(app.input.textarea.text(), "被取消的提示");
        assert!(app.last_prompt.is_none());
        // Idempotent: second call is a no-op.
        app.restore_last_prompt();
        assert_eq!(app.input.textarea.text(), "被取消的提示");

        // A user-typed new input is never clobbered.
        app.last_prompt = Some("旧提示".into());
        app.input.textarea.set_text("用户新输入");
        app.restore_last_prompt();
        assert_eq!(app.input.textarea.text(), "用户新输入");
        assert!(app.last_prompt.is_none(), "consumed even when not restored");
    }

    #[test]
    fn execute_find_searches_and_marks_hits() {
        let mut app = TuiApp::new();
        app.content.add_user_message("审查 session 列表");
        app.content.add_model_message("来源可见性", 1, false);
        app.content.add_or_update_tool_trace("read_file", "session.json", "运行中");
        app.open_find();
        let crate::modals::Modal::Find(find) = app.modal.as_mut().unwrap() else {
            panic!("find modal");
        };
        find.query.set_text("session");
        let msgs = app.execute_find();
        assert!(msgs.iter().any(|m| m.contains("2 处匹配")), "{msgs:?}");
        assert!(app.modal.is_none(), "find closes after executing");
        // Hit markers: item 0 (user card) + item 2 (tool line) → L1, L3.
        let hits: Vec<&str> = app
            .marker
            .markers
            .iter()
            .map(|m| m.label.as_str())
            .collect();
        assert!(hits.contains(&"● L1") && hits.contains(&"● L3"), "{hits:?}");

        // A second search replaces the previous hits only.
        let mut app2 = TuiApp::new();
        app2.content.add_user_message("x");
        app2.content.add_user_message("y");
        app2.marker.add_user_input(1);
        app2.open_find();
        let crate::modals::Modal::Find(find) = app2.modal.as_mut().unwrap() else {
            panic!("find modal");
        };
        find.query.set_text("y");
        app2.execute_find();
        let hits: Vec<&str> = app2
            .marker
            .markers
            .iter()
            .map(|m| m.label.as_str())
            .collect();
        assert_eq!(hits, vec!["▸ L1", "● L2"]);
    }

    /// Review D2-2: with the marker bar default-hidden, a find with hits
    /// must point the user at the toggle so the output is reachable.
    #[test]
    fn execute_find_hints_marker_toggle_when_hidden() {
        let mut app = TuiApp::new();
        assert!(!app.show_marker, "marker default hidden");
        app.content.add_user_message("含 关键词 的内容");
        app.open_find();
        let crate::modals::Modal::Find(find) = app.modal.as_mut().unwrap() else {
            panic!("find modal");
        };
        find.query.set_text("关键词");
        let msgs = app.execute_find();
        let joined = msgs.join(" ");
        assert!(joined.contains("1 处匹配"), "{joined}");
        assert!(joined.contains("标记栏已隐藏"), "hint when marker hidden: {joined}");

        // Marker visible → no hint.
        let mut app2 = TuiApp::new();
        app2.show_marker = true;
        app2.content.add_user_message("含 关键词 的内容");
        app2.open_find();
        let crate::modals::Modal::Find(find) = app2.modal.as_mut().unwrap() else {
            panic!("find modal");
        };
        find.query.set_text("关键词");
        let msgs = app2.execute_find();
        assert!(
            msgs.join(" ").contains("1 处匹配")
                && !msgs.join(" ").contains("标记栏已隐藏"),
            "no hint when marker visible: {msgs:?}"
        );
    }

    #[test]
    fn execute_find_invalid_regex_surfaces_error() {
        let mut app = TuiApp::new();
        app.content.add_user_message("内容");
        app.open_find();
        let crate::modals::Modal::Find(find) = app.modal.as_mut().unwrap() else {
            panic!("find modal");
        };
        find.query.set_text("([");
        find.use_regex = true;
        let msgs = app.execute_find();
        assert!(msgs.iter().any(|m| m.contains("正则表达式错误")), "{msgs:?}");
        assert!(app.modal.is_none());
        assert!(app.marker.markers.is_empty(), "no markers on failed search");
    }

    #[test]
    fn toggle_explorer_and_events_flags() {
        let mut app = TuiApp::new();
        assert!(!app.show_explorer, "sidebars default hidden (user ruling)");
        assert!(!app.show_marker, "marker default hidden (user ruling)");

        let msgs = app.dispatch_command("/toggle-explorer");
        assert!(app.show_explorer);
        assert!(msgs.iter().any(|m| m == "探索器已显示"), "{msgs:?}");
        let msgs = app.dispatch_command("/toggle-events");
        assert!(!app.explorer.show_events);
        assert!(msgs.iter().any(|m| m == "事件栏已隐藏"), "{msgs:?}");
        let msgs = app.dispatch_command("/toggle-markers");
        assert!(app.show_marker);
        assert!(msgs.iter().any(|m| m == "标记栏已显示"), "{msgs:?}");
    }

    #[test]
    fn properties_sheet_snapshots_app_state() {
        let mut app = TuiApp::new();
        app.session_id = Some("S-ABC".into());
        app.turn_counter = 2;
        app.status.set_run_state("完成", true);
        let sheet = crate::modals::PropertiesSheet::from_app(&app);
        assert_eq!(
            sheet.sheet.tabs.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["常规", "溯源", "可见性", "依赖", "验证", "历史"]
        );
        let general = &sheet.sheet.tabs[0];
        assert!(
            general
                .fields
                .iter()
                .any(|(k, v)| k == "会话 ID" && v == "S-ABC")
        );
        assert!(
            general.fields.iter().any(|(k, v)| k == "状态" && v == "完成")
        );
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
