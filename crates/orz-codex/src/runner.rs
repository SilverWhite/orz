//! Runner for the codex-style fallback TUI (Phase 3 slice #12).
//!
//! Mirrors orz-tui's runner discipline: one `tokio::select!` loop
//! (tick × keys × client messages), guaranteed teardown (leave alternate
//! screen, disable raw mode), and an improvement over orz-tui — quitting
//! mid-run first interrupts the turn and drains to its terminal
//! (`turn/completed{interrupted}`) with a bounded wait, so the journal ends
//! on a valid `run_cancelled` instead of an aborted writer.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use futures::StreamExt;
use orz_host::acp_server::AcpServer;
use orz_host::codex_app::CodexAppServer;
use orz_host::session::TrustPolicy;
use orz_loop::gateway::model::ModelGateway;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::time::interval;

use crate::app::{handle_key, CodexApp, KeyOutcome};
use crate::client::{CodexClient, ClientMsg};
use crate::widgets::render_frame;

/// UI tick — dialog countdown rendering and frame refresh.
const TICK: Duration = Duration::from_millis(50);
/// Bounded drain when quitting mid-run (journal integrity: wait for the
/// `run_cancelled` terminal, but never hang the exit).
const QUIT_DRAIN: Duration = Duration::from_secs(5);

/// Configuration for the fallback TUI.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub cwd: PathBuf,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

/// Errors from the fallback TUI.
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("session error: {0}")]
    Session(String),
}

/// Run the fallback TUI. Must be called inside a `tokio::task::LocalSet`
/// (the host's permission manager and run tasks use `spawn_local`).
pub async fn run(config: TuiConfig, gateway: Arc<dyn ModelGateway>) -> Result<(), TuiError> {
    // The host stack is identical to orz — same AcpServer machinery, driven
    // here through the codex app-server protocol (§2.4: assurance runs
    // silently inside the host).
    let acp = Arc::new(AcpServer::with_gateway(gateway));
    let parts = CodexAppServer::new_parts(acp, config.cwd, TrustPolicy::Enforce);
    let mut client = CodexClient::connect_inprocess(parts);
    let mut app = CodexApp::new();

    // Raw mode + alternate screen — teardown is guaranteed on every path
    // (the run loop result does not skip it; orz-tui review P1-1 precedent).
    // Implementation review P2-1: the early `?` paths must not leak raw
    // mode — track what was enabled and always unwind.
    let raw_enabled = enable_raw_mode().is_ok();
    let mut stdout = io::stdout();
    let alt_screen = execute!(stdout, EnterAlternateScreen).is_ok();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            if alt_screen {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
            if raw_enabled {
                let _ = disable_raw_mode();
            }
            return Err(TuiError::Io(e));
        }
    };
    let result = run_loop(&mut terminal, &mut client, &mut app).await;
    if alt_screen {
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    }
    if raw_enabled {
        let _ = disable_raw_mode();
    }
    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    client: &mut CodexClient,
    app: &mut CodexApp,
) -> Result<(), TuiError> {
    let mut ticker = interval(TICK);
    let mut keys = EventStream::new();
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                // Approval dialog timeout — deny locally (the host's own
                // 300s bound independently fails closed).
                if app.dialog.as_ref().is_some_and(|d| d.timed_out()) {
                    app.answer_approval();
                    app.system("审批超时——已拒绝");
                }
                render_frame(terminal, app)?;
            }
            key = keys.next() => {
                match key {
                    Some(Ok(Event::Key(k))) if k.kind == KeyEventKind::Press => {
                        match handle_key(app, k) {
                            KeyOutcome::Continue => {}
                            KeyOutcome::Prompt(text) => do_prompt(client, app, text).await,
                            KeyOutcome::Interrupt => do_interrupt(client, app).await,
                            KeyOutcome::AnswerApproval(decision) => do_answer(app, client, decision),
                            KeyOutcome::Quit => {
                                if matches!(app.status, crate::app::RunState::Running | crate::app::RunState::WaitingApproval) {
                                    drain_until_cancelled(client, app).await;
                                }
                                break;
                            }
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => return Err(TuiError::Io(e)),
                    None => break, // stdin EOF
                }
            }
            msg = client.msg_rx.recv() => {
                match msg {
                    Some(msg) => apply_msg(app, msg),
                    None => {
                        app.system("与宿主服务的连接已关闭");
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Submit a prompt: initialize the thread on first use, then start the turn.
async fn do_prompt(client: &mut CodexClient, app: &mut CodexApp, text: String) {
    if !app.submit_prompt(text.clone()) {
        return; // rejected while running — hint already shown
    }
    if app.thread_id.is_none() {
        match client.initialize_and_start_thread().await {
            Ok(thread_id) => app.thread_id = Some(thread_id),
            Err(e) => {
                app.session_failed(format!("会话启动失败: {e}"));
                return;
            }
        }
    }
    match client.start_turn(&text).await {
        Ok(turn_id) => app.begin_turn(turn_id),
        Err(e) => app.session_failed(format!("回合启动失败: {e}")),
    }
}

/// Request cancellation (Ctrl+Z / `/stop`) — immediate feedback; the
/// terminal `turn/completed{interrupted}` arrives via the message loop.
async fn do_interrupt(client: &mut CodexClient, app: &mut CodexApp) {
    if matches!(app.status, crate::app::RunState::Running | crate::app::RunState::WaitingApproval) {
        // An open approval blocks the run inside the permission await — the
        // cancel checkpoint is unreachable until it resolves. Answer deny on
        // the wire first (same reasoning as drain_until_cancelled, review
        // D2); the pending broker entry is released either way.
        if let Some(dialog) = app.dialog.take() {
            client.answer_approval(&dialog.id, "deny");
        }
        match client.interrupt_turn().await {
            Ok(()) => {
                app.status = crate::app::RunState::Running;
                app.system("停止请求已发送（正在取消）");
            }
            Err(e) => app.system(format!("中断请求失败: {e}")),
        }
    } else {
        app.system("当前没有运行");
    }
}

/// Answer the open approval dialog on the wire.
fn do_answer(app: &mut CodexApp, client: &CodexClient, decision: &str) {
    if let Some(dialog) = app.dialog.take() {
        client.answer_approval(&dialog.id, decision);
        if app.status == crate::app::RunState::WaitingApproval {
            app.status = crate::app::RunState::Running;
        }
    }
}

/// Quitting mid-run: interrupt, then drain to the terminal notification with
/// a bounded wait — the journal closes on a valid `run_cancelled` in the
/// common cases (teardown improvement over orz-tui's immediate exit;
/// recorded slice #12).
///
/// Implementation review P1-1: the guarantee is BEST-EFFORT by the
/// cooperative-cancel design (slice #7 record — the controller deliberately
/// has no checkpoints inside permission awaits or tool execution). The
/// approval case is deterministic: when a dialog is open, the run is blocked
/// inside the permission await and cannot reach the cancel checkpoint — we
/// answer `deny` on the wire FIRST so the manager resolves, the run reaches
/// the checkpoint and lands `run_cancelled` inside the window. A LONG tool
/// execution (e.g. a slow bash) still runs to completion before the
/// checkpoint; the 5s bound then expires and the journal may be left
/// partial — same class as orz-tui's immediate exit, accepted (the tool
/// itself is never aborted mid-flight by design).
async fn drain_until_cancelled(client: &mut CodexClient, app: &mut CodexApp) {
    if let Some(dialog) = app.dialog.take() {
        client.answer_approval(&dialog.id, "deny");
    }
    if client.interrupt_turn().await.is_ok() {
        app.system("正在取消当前回合…");
    }
    let drain = tokio::time::sleep(QUIT_DRAIN);
    tokio::pin!(drain);
    loop {
        tokio::select! {
            _ = &mut drain => break, // bounded — never hang the exit
            msg = client.msg_rx.recv() => match msg {
                Some(ClientMsg::TurnCompleted { .. }) => break,
                Some(_) => {}
                None => break,
            },
        }
    }
}

/// Route server→client messages into the app.
fn apply_msg(app: &mut CodexApp, msg: ClientMsg) {
    match msg {
        ClientMsg::ApprovalRequest { id, params } => app.on_approval_request(id, params),
        ClientMsg::ItemDelta {
            turn_id,
            item_id,
            text,
            ..
        } => app.on_item_delta(turn_id, item_id, text),
        ClientMsg::ItemCompleted {
            turn_id,
            item_id,
            text,
            kind,
            ..
        } => app.on_item_completed(turn_id, item_id, text, kind),
        ClientMsg::TurnCompleted {
            turn_id,
            status,
            error,
        } => app.on_turn_completed(turn_id, status, error),
        ClientMsg::ThreadStarted { .. } | ClientMsg::ThreadClosed { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::RunState;
    use orz_host::session::TrustPolicy;
    use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
    use orz_loop::gateway::model::ToolCall;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-codex-runner-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Build host + client + app (the run-loop pieces) for E2E tests — the
    /// loop itself is exercised through the same message pump the runner
    /// uses, without a real terminal.
    fn harness(
        script: Vec<ScriptedResponse>,
        base: &std::path::Path,
    ) -> (CodexClient, CodexApp) {
        let acp = Arc::new(AcpServer::with_gateway(Arc::new(FakeProvider::new(script))));
        let parts = CodexAppServer::new_parts(acp, base.to_path_buf(), TrustPolicy::Skip);
        (CodexClient::connect_inprocess(parts), CodexApp::new())
    }

    /// The runner's submit path minus the terminal.
    async fn submit(client: &mut CodexClient, app: &mut CodexApp, text: &str) {
        do_prompt(client, app, text.to_owned()).await;
    }

    async fn recv_until_turn_completed(client: &mut CodexClient, app: &mut CodexApp) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let msg = client.msg_rx.recv().await.expect("connection alive");
                if matches!(&msg, ClientMsg::TurnCompleted { .. }) {
                    apply_msg(app, msg);
                    return;
                }
                apply_msg(app, msg);
            }
        })
        .await
        .expect("turn completes");
    }

    fn journal_terminal(base: &std::path::Path, thread8: &str) -> String {
        let path = base
            .join(".gsa")
            .join("runs")
            .join(format!("RUN-{thread8}-0"))
            .join("events.jsonl");
        let replay = orz_assurance::replay_journal(&path, None, None, true);
        assert!(replay.valid, "journal {path:?} must be valid: {replay:?}");
        replay.terminal_event.expect("terminal event")
    }

    fn bash_script() -> Vec<ScriptedResponse> {
        vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "run_terminal_cmd".to_string(),
                arguments: serde_json::json!({ "command": "dir" }),
                call_id: "call-1".to_string(),
            }]),
            ScriptedResponse::text("完成（bash 已执行）。"),
            ScriptedResponse::text("完成（bash 已执行）。"),
        ]
    }

    #[tokio::test]
    async fn prompt_streams_to_completed_with_valid_journal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let (mut client, mut app) = harness(
                    vec![
                        ScriptedResponse::text("第一轮草稿。"),
                        ScriptedResponse::text("终局答案。"),
                    ],
                    &base,
                );
                submit(&mut client, &mut app, "你好").await;
                assert_eq!(app.status, RunState::Running);
                assert!(app.thread_id.is_some());

                // Streamed deltas accumulate into one agent message.
                let mut delta_text = String::new();
                let mut completed_text = None;
                loop {
                    let msg = client.msg_rx.recv().await.expect("connection alive");
                    match &msg {
                        ClientMsg::ItemDelta { text, .. } => delta_text.push_str(text),
                        ClientMsg::ItemCompleted { kind, text, .. } if kind == "agentMessage" => {
                            completed_text = Some(text.clone());
                        }
                        ClientMsg::TurnCompleted { .. } => {
                            apply_msg(&mut app, msg);
                            break;
                        }
                        _ => {}
                    }
                }
                assert_eq!(app.status, RunState::Completed);
                assert!(delta_text.contains("终局答案。"), "{delta_text:?}");
                assert_eq!(completed_text.as_deref(), Some("终局答案。"));
                // The user message is rendered from the submission.
                assert!(app.messages.iter().any(|m| m.role == crate::app::Role::User && m.text == "你好"));
                let thread8: String = app.thread_id.as_ref().unwrap().chars().take(8).collect();
                assert_eq!(journal_terminal(&base, &thread8), "run_finished");
            })
            .await;
    }

    #[tokio::test]
    async fn interrupt_flow_ends_interrupted_and_journal_cancelled() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let (mut client, mut app) = harness(
                    vec![
                        ScriptedResponse::text("慢速草稿。"),
                        ScriptedResponse::text("慢速终答。"),
                    ],
                    &base,
                );
                submit(&mut client, &mut app, "取消我").await;
                // Let streaming begin, then interrupt (mirrors the runner's
                // Ctrl+Z path).
                loop {
                    if let ClientMsg::ItemDelta { .. } = client.msg_rx.recv().await.unwrap() {
                        break;
                    }
                }
                do_interrupt(&mut client, &mut app).await;
                assert!(app.messages.iter().any(|m| m.text.contains("正在取消")));
                let thread8: String = app.thread_id.as_ref().unwrap().chars().take(8).collect();
                // Drain to the interrupted terminal.
                let mut interrupted = false;
                tokio::time::timeout(Duration::from_secs(5), async {
                    while !interrupted {
                        let msg = client.msg_rx.recv().await.unwrap();
                        if let ClientMsg::TurnCompleted { status, .. } = &msg {
                            interrupted = status == "interrupted";
                        }
                        apply_msg(&mut app, msg);
                    }
                })
                .await
                .expect("interrupted terminal arrives");
                assert_eq!(app.status, RunState::Interrupted);
                // The last prompt is restored for a retry.
                assert_eq!(app.input.text(), "取消我");
                assert_eq!(journal_terminal(&base, &thread8), "run_cancelled");
            })
            .await;
    }

    #[tokio::test]
    async fn approval_allow_flow_executes_and_deny_flow_aborts() {
        tokio::task::LocalSet::new()
            .run_until(async {
                // allow flow
                let base = test_dir();
                let (mut client, mut app) = harness(bash_script(), &base);
                submit(&mut client, &mut app, "运行 dir").await;
                let approval = loop {
                    if let ClientMsg::ApprovalRequest { id, params } =
                        client.msg_rx.recv().await.unwrap()
                    {
                        assert_eq!(params["bash_command"], "dir");
                        break id;
                    }
                };
                // The runner surfaces the request via apply_msg; the answer
                // travels over the real client to the host.
                client.answer_approval(&approval, "allow_once");
                recv_until_turn_completed(&mut client, &mut app).await;
                assert_eq!(app.status, RunState::Completed);
                let thread8: String = app.thread_id.as_ref().unwrap().chars().take(8).collect();
                let events = std::fs::read_to_string(
                    base.join(".gsa")
                        .join("runs")
                        .join(format!("RUN-{thread8}-0"))
                        .join("events.jsonl"),
                )
                .unwrap();
                assert!(events.contains("\"tool_completed\""), "{events}");

                // deny flow — the tool never starts.
                let base2 = test_dir();
                let (mut client2, mut app2) = harness(bash_script(), &base2);
                submit(&mut client2, &mut app2, "运行 dir").await;
                let approval2 = loop {
                    if let ClientMsg::ApprovalRequest { id, .. } =
                        client2.msg_rx.recv().await.unwrap()
                    {
                        break id;
                    }
                };
                client2.answer_approval(&approval2, "deny");
                recv_until_turn_completed(&mut client2, &mut app2).await;
                let thread8b: String = app2.thread_id.as_ref().unwrap().chars().take(8).collect();
                let events2 = std::fs::read_to_string(
                    base2.join(".gsa")
                        .join("runs")
                        .join(format!("RUN-{thread8b}-0"))
                        .join("events.jsonl"),
                )
                .unwrap();
                assert!(!events2.contains("\"tool_started\""), "{events2}");
                assert!(events2.contains("\"deny\""), "{events2}");
            })
            .await;
    }

    #[tokio::test]
    async fn quit_during_approval_answers_deny_and_lands_cancelled_terminal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let (mut client, mut app) = harness(bash_script(), &base);
                submit(&mut client, &mut app, "运行 dir").await;
                // Wait for the approval request and surface it like the
                // runner's message loop would.
                let approval = loop {
                    if let ClientMsg::ApprovalRequest { id, .. } =
                        client.msg_rx.recv().await.unwrap()
                    {
                        break id;
                    }
                };
                app.on_approval_request(approval.clone(), serde_json::json!({}));
                assert_eq!(app.status, RunState::WaitingApproval);

                // Quit with the dialog open: the drain must answer deny on
                // the wire so the run unblocks and lands run_cancelled
                // (review D2 — without it the drain times out and the
                // journal is left partial).
                drain_until_cancelled(&mut client, &mut app).await;
                let thread8: String = app.thread_id.as_ref().unwrap().chars().take(8).collect();
                assert_eq!(journal_terminal(&base, &thread8), "run_cancelled");
            })
            .await;
    }

    #[tokio::test]
    async fn second_submit_while_running_is_rejected() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let (mut client, mut app) = harness(
                    vec![
                        ScriptedResponse::text("草稿"),
                        ScriptedResponse::text("终答"),
                    ],
                    &base,
                );
                submit(&mut client, &mut app, "第一个").await;
                // Second submit while running → rejected with a hint.
                let before = app.messages.len();
                submit(&mut client, &mut app, "第二个").await;
                assert_eq!(app.messages.len(), before + 1, "only the hint message added");
                assert!(app.messages.last().unwrap().text.contains("正在运行"));
                // Drain to completion so the LocalSet has no pending turns.
                recv_until_turn_completed(&mut client, &mut app).await;
            })
            .await;
    }
}
