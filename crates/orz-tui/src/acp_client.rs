//! In-process ACP client — the workbench's wire to orz-host.
//!
//! The TUI and the host live in the same process (design doc §1); they
//! communicate over ACP JSON-RPC through two crossed `tokio::io::duplex`
//! pairs (client→agent and agent→client), exactly like `stdio.rs` does over
//! pipes. Permission prompts arrive as outbound `RequestPermissionRequest`s
//! and are forwarded to the app via `ClientMsg` + a oneshot responder.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol as acp;
use agent_client_protocol::Agent;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use xai_acp_lib::acp_gateway;

use orz_host::acp_server::AcpServer;
use orz_host::session::TrustPolicy;
use orz_host::stdio::StdioAgentHandler;

/// Messages from the ACP wire to the TUI main loop.
#[derive(Debug)]
pub enum ClientMsg {
    /// The host asks the user for a tool permission decision (boxed to keep
    /// the enum small — permission requests are large).
    PermissionRequest {
        request: Box<acp::RequestPermissionRequest>,
        respond: oneshot::Sender<acp::RequestPermissionResponse>,
    },
    /// A streamed agent message chunk (text-delta source, streaming slice).
    SessionNotification { session_id: String, text_chunk: String },
    /// A spawned prompt turn finished (result of the full run).
    ///
    /// `seq` is the prompt counter snapshot at spawn time — the runner's
    /// generation guard: after a cancel, the PREVIOUS run's completion must
    /// not stop the tail / clear `running` / dismiss the dialog of the NEXT
    /// run (Phase 3 slice #7 stale-completion race).
    PromptCompleted {
        seq: u32,
        result: Result<acp::PromptResponse, acp::Error>,
    },
}

/// Client-side ACP handler — the only `acp::Client` in the workbench.
///
/// The blanket `MessageHandler<ClientSide>` impl (agent-client-protocol
/// lib.rs) dispatches inbound agent requests to these methods. RPC
/// dispatcher spawns the handler future, so the read loop stays live while
/// the permission dialog awaits the user.
pub struct TuiClientHandler {
    pub msg_tx: mpsc::UnboundedSender<ClientMsg>,
}

#[async_trait::async_trait(?Send)]
impl acp::Client for TuiClientHandler {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        let (tx, rx) = oneshot::channel();
        if self
            .msg_tx
            .send(ClientMsg::PermissionRequest {
                request: Box::new(args),
                respond: tx,
            })
            .is_err()
        {
            return Err(acp::Error::new(acp::ErrorCode::InternalError.into(), "permission dialog channel closed"));
        }
        rx.await
            .map_err(|_| acp::Error::new(acp::ErrorCode::InternalError.into(), "permission dialog dropped"))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        // Streaming-ready: forward agent message chunks as text deltas.
        if let acp::SessionUpdate::AgentMessageChunk(chunk) = args.update {
            let text = match chunk.content {
                acp::ContentBlock::Text(t) => t.text,
                _ => String::new(),
            };
            let _ = self.msg_tx.send(ClientMsg::SessionNotification {
                session_id: args.session_id.0.to_string(),
                text_chunk: text,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::EventSource;
    use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
    use orz_loop::gateway::model::ToolCall;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-acp-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn scripted_server(script: Vec<ScriptedResponse>) -> Arc<AcpServer> {
        Arc::new(AcpServer::with_gateway(Arc::new(FakeProvider::new(script))))
    }

    fn bash_script() -> Vec<ScriptedResponse> {
        vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "bash".to_string(),
                arguments: serde_json::json!({"command": "dir"}),
                call_id: "call-1".to_string(),
            }]),
            ScriptedResponse::text("完成（bash 已执行）。"),
            ScriptedResponse::text("完成（bash 已执行）。"),
        ]
    }

    async fn run_prompt_until_permission(
        client: &mut InProcessClient,
        base: &Path,
        text: &str,
    ) -> (acp::RequestPermissionRequest, tokio::sync::oneshot::Sender<acp::RequestPermissionResponse>) {
        client.start_session(base.to_path_buf()).await.unwrap();
        client.prompt_count += 1;
        let session_id = client.session_id.clone().unwrap();
        let seq = client.prompt_count;
        client.spawn_prompt(session_id, text.to_string(), seq);
        // The permission request arrives while the prompt turn runs.
        loop {
            match client.msg_rx.recv().await.unwrap() {
                ClientMsg::PermissionRequest { request, respond } => {
                    return (*request, respond);
                }
                ClientMsg::SessionNotification { .. } => {}
                ClientMsg::PromptCompleted { result, .. } => {
                    panic!("prompt completed before permission: {result:?}");
                }
            }
        }
    }

    fn run_dir_of(client: &InProcessClient, base: &Path) -> PathBuf {
        let session8: String = client
            .session_id
            .as_ref()
            .expect("session started")
            .chars()
            .take(8)
            .collect();
        base.join(".gsa").join("runs").join(format!("RUN-{session8}-0"))
    }

    #[tokio::test]
    async fn allow_once_executes_tool_with_valid_journal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = scripted_server(bash_script());
                let mut client = connect_inprocess(server, TrustPolicy::Skip);

                let (request, respond) = run_prompt_until_permission(&mut client, &base, "运行 dir").await;
                // The host normalizes call ids to `call-<tool>`.
                assert_eq!(request.tool_call.tool_call_id.0.as_ref(), "call-bash");
                assert!(request.options.iter().any(|o| o.kind == acp::PermissionOptionKind::AllowOnce));

                // User allows once.
                let allow_once = request
                    .options
                    .iter()
                    .find(|o| o.kind == acp::PermissionOptionKind::AllowOnce)
                    .unwrap()
                    .option_id
                    .0
                    .to_string();
                respond
                    .send(acp::RequestPermissionResponse::new(
                        acp::RequestPermissionOutcome::Selected(
                            acp::SelectedPermissionOutcome::new(allow_once),
                        ),
                    ))
                    .unwrap();

                // Await the run completion.
                let completed = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { result, .. } => break result,
                        _ => {}
                    }
                };
                assert!(completed.is_ok(), "prompt failed: {completed:?}");

                // Journal: valid chain with allow + tool execution.
                let events_path = run_dir_of(&client, &base).join("events.jsonl");
                let replay = orz_assurance::replay_journal(&events_path, None, None, true);
                assert!(replay.valid, "journal invalid: {:?}", replay.errors);
                let content = std::fs::read_to_string(&events_path).unwrap();
                assert!(content.contains("\"allow_once\""));
                assert!(content.contains("\"tool_started\""));
                assert!(content.contains("\"tool_completed\""));

                // Review P3-4: the journal replays through the projection and
                // the view model carries the model text + tool trace.
                let mut app = crate::app::TuiApp::new();
                let mut src = crate::source::ReplaySource::new(events_path).unwrap();
                for ev in src.poll() {
                    app.accept_event(ev);
                }
                let model_texts: Vec<&str> = app
                    .content
                    .items
                    .iter()
                    .filter_map(|i| match i {
                        crate::view_model::ContentItem::Message(m) if m.role == "模型" => {
                            Some(m.content.as_str())
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    model_texts.iter().any(|t| t.contains("完成")),
                    "model text must reach the view: {model_texts:?}"
                );
                assert!(
                    app.content.tool_trace_names().contains(&"bash"),
                    "tool trace must reach the view"
                );
                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// Streaming slice: chunked model text arrives as incremental
    /// `agent_message_chunk` notifications (arrival rate, msg arm) while
    /// `model_output` lands via the 50ms journal tail (tick arm) — the
    /// controller's pacing guard keeps the two gate-round cards distinct,
    /// deduped, and free of concatenation. Pins the guard: without it the
    /// second round's deltas append to the first round's card.
    #[tokio::test]
    async fn chunked_stream_renders_incremental_cards_then_dedups() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Two identical gate-round texts (the counterexample gate
                // intercepts the first), chunked by 3 chars each.
                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::from_texts(vec!["第一轮回答。", "第一轮回答。"])
                        .with_chunk_size(3),
                )));
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                client.start_session(base.to_path_buf()).await.unwrap();
                client.prompt_count += 1;
                let session_id = client.session_id.clone().unwrap();
                let seq = client.prompt_count;
                client.spawn_prompt(session_id, "回答我".to_string(), seq);

                // Mirror production: deltas at arrival rate, journal events at
                // 50ms tail granularity.
                let mut app = crate::app::TuiApp::new();
                let mut tail =
                    crate::source::JournalTailSource::new(run_dir_of(&client, &base).join("events.jsonl"));
                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(50));
                let completed = loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            for ev in tail.poll() {
                                app.accept_event(ev);
                            }
                        }
                        msg = client.msg_rx.recv() => {
                            match msg.unwrap() {
                                ClientMsg::SessionNotification { text_chunk, .. }
                                    if !text_chunk.is_empty() =>
                                {
                                    app.accept_event(crate::events::TuiEvent::TextDelta {
                                        text: text_chunk,
                                    });
                                }
                                ClientMsg::PromptCompleted { result, .. } => break result,
                                _ => {}
                            }
                        }
                    }
                };
                assert!(completed.is_ok(), "prompt failed: {completed:?}");

                // Project trailing journal events (MO2 dedup, run_finished) —
                // the tail thread polls the file every 50ms.
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
                    let evs = tail.poll();
                    for ev in evs {
                        app.accept_event(ev);
                    }
                    if !tail.is_active() {
                        break;
                    }
                }
                tail.close();

                let model_texts: Vec<String> = app
                    .content
                    .items
                    .iter()
                    .filter_map(|i| match i {
                        crate::view_model::ContentItem::Message(m) if m.role == "模型" => {
                            Some(m.content.clone())
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    model_texts,
                    vec!["第一轮回答。", "第一轮回答。"],
                    "two distinct gate-round cards — no duplication, no concatenation"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// Review P2-2: run ids are reserved before bootstrap, so a failed prompt
    /// must NOT collide with a retried one — two sequential successful
    /// prompts each get a fresh, valid journal.
    #[tokio::test]
    async fn sequential_prompts_get_distinct_valid_journals() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = scripted_server(vec![
                    ScriptedResponse::text("第一轮回答。"),
                    ScriptedResponse::text("第一轮回答。"),
                    ScriptedResponse::text("第二轮回答。"),
                    ScriptedResponse::text("第二轮回答。"),
                ]);
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                client.start_session(base.to_path_buf()).await.unwrap();

                // Prompt 1.
                client.prompt_count += 1;
                let session_id = client.session_id.clone().unwrap();
                let seq = client.prompt_count;
                client.spawn_prompt(session_id.clone(), "问题一".into(), seq);
                let done = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { result, .. } => break result,
                        _ => {}
                    }
                };
                assert!(done.is_ok(), "prompt 1 failed: {done:?}");

                // Prompt 2 — must land in a DIFFERENT run dir.
                let dir1 = run_dir_of(&client, &base);
                client.prompt_count += 1;
                let seq = client.prompt_count;
                client.spawn_prompt(session_id, "问题二".into(), seq);
                let done = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { result, .. } => break result,
                        _ => {}
                    }
                };
                assert!(done.is_ok(), "prompt 2 failed: {done:?}");

                let dir2 = base
                    .join(".gsa")
                    .join("runs")
                    .join(format!(
                        "RUN-{}-1",
                        client.session_id.as_ref().unwrap().chars().take(8).collect::<String>()
                    ));
                assert_ne!(dir1, dir2, "run dirs must be distinct");
                for path in [dir1.join("events.jsonl"), dir2.join("events.jsonl")] {
                    let replay = orz_assurance::replay_journal(&path, None, None, true);
                    assert!(replay.valid, "journal invalid: {:?}", replay.errors);
                    assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));
                }
                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    #[tokio::test]
    async fn cancelled_denies_tool_without_execution() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = scripted_server(bash_script());
                let mut client = connect_inprocess(server, TrustPolicy::Skip);

                let (_request, respond) = run_prompt_until_permission(&mut client, &base, "运行 dir").await;
                respond
                    .send(acp::RequestPermissionResponse::new(
                        acp::RequestPermissionOutcome::Cancelled,
                    ))
                    .unwrap();

                let completed = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { result, .. } => break result,
                        _ => {}
                    }
                };
                assert!(completed.is_ok(), "prompt failed: {completed:?}");

                let events_path = run_dir_of(&client, &base).join("events.jsonl");
                let replay = orz_assurance::replay_journal(&events_path, None, None, true);
                assert!(replay.valid, "journal invalid: {:?}", replay.errors);
                let content = std::fs::read_to_string(&events_path).unwrap();
                assert!(content.contains("\"deny\""));
                assert!(!content.contains("\"tool_started\""), "denied tool must not execute");
                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// Phase 3 slice #7: `request_cancel` over the in-process duplex aborts
    /// the run — the prompt resolves with `StopReason::Cancelled` (a success
    /// response, per the protocol contract), the journal ends with a valid
    /// `run_cancelled` terminal, and replay projects the 已取消 card.
    #[tokio::test]
    async fn cancel_aborts_run_inprocess_with_cancelled_stop_reason() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                    ])
                    .with_chunk_delay(std::time::Duration::from_millis(100)),
                )));
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                client.start_session(base.to_path_buf()).await.unwrap();
                client.prompt_count += 1;
                let seq = client.prompt_count;
                let session_id = client.session_id.clone().unwrap();
                client.spawn_prompt(session_id, "回答我".to_string(), seq);

                // Let the first model round get underway, then cancel —
                // mirrors the runner's Ctrl+Z / /stop path.
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                client.request_cancel().await;

                let completed = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { seq: got, result } => {
                            assert_eq!(got, seq, "completion stamped with its seq");
                            break result;
                        }
                        ClientMsg::SessionNotification { .. } => {}
                        ClientMsg::PermissionRequest { .. } => {}
                    }
                };
                let response = completed.expect("cancel resolves as a success response");
                assert_eq!(
                    response.stop_reason,
                    acp::StopReason::Cancelled,
                    "cancel must resolve with StopReason::Cancelled"
                );

                let events_path = run_dir_of(&client, &base).join("events.jsonl");
                let replay = orz_assurance::replay_journal(&events_path, None, None, true);
                assert!(replay.valid, "journal invalid: {:?}", replay.errors);
                assert_eq!(
                    replay.terminal_event.as_deref(),
                    Some("run_cancelled")
                );

                // Replay projection shows the 已取消 status card.
                let (lines, valid) =
                    crate::runner::replay_to_screen(&events_path, 100, 30).unwrap();
                assert!(valid, "cancelled journal replays clean");
                assert!(lines.join("\n").contains("已取消"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// Phase 3 slice #7 — stale-completion race: a cancel followed
    /// immediately by a second prompt delivers TWO completions; only the one
    /// stamped with the CURRENT prompt counter is honored (the runner's seq
    /// guard drops the stale one — no tail stop, no running clear, no dialog
    /// dismiss). Both journals stay valid: RUN-0 cancelled, RUN-1 finished.
    #[tokio::test]
    async fn stale_prompt_completion_ignored_after_next_run_started() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // run #1 consumes 1–2 texts (cancelled mid-round); run #2
                // needs 2 (counterexample gate + final answer).
                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第二轮回答。"),
                        ScriptedResponse::text("第二轮回答。"),
                    ])
                    .with_chunk_delay(std::time::Duration::from_millis(100)),
                )));
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                client.start_session(base.to_path_buf()).await.unwrap();

                // Run #1 — cancel without draining its completion.
                client.prompt_count += 1;
                let seq1 = client.prompt_count;
                let session_id = client.session_id.clone().unwrap();
                client.spawn_prompt(session_id.clone(), "问题一".into(), seq1);
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                client.request_cancel().await;

                // Run #2 immediately — the stale run #1 completion may still
                // be in flight (the guard's job is order-independent).
                client.prompt_count += 1;
                let seq2 = client.prompt_count;
                client.spawn_prompt(session_id, "问题二".into(), seq2);

                // Mirror the runner's generation guard: only the completion
                // stamped with the CURRENT prompt counter drives UI state.
                let current = client.prompt_count;
                assert_eq!(current, seq2);
                let second_done = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { seq, result } => {
                            if seq != current {
                                // Stale completion from run #1 — dropped.
                                assert_eq!(seq, seq1, "only run #1's seq can be stale");
                                assert_eq!(
                                    result.as_ref().unwrap().stop_reason,
                                    acp::StopReason::Cancelled
                                );
                                continue;
                            }
                            break result;
                        }
                        ClientMsg::SessionNotification { .. } => {}
                        ClientMsg::PermissionRequest { .. } => {}
                    }
                };
                assert!(second_done.is_ok(), "run #2 completes: {second_done:?}");

                // Two run dirs, both valid — the reserved-counter invariant
                // holds on the cancelled path too.
                let runs_dir = base.join(".gsa").join("runs");
                let mut dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .collect();
                dirs.sort();
                assert_eq!(dirs.len(), 2, "two runs, two journals");
                for (dir, terminal) in [
                    (dirs[0].clone(), "run_cancelled"),
                    (dirs[1].clone(), "run_finished"),
                ] {
                    let replay = orz_assurance::replay_journal(
                        &dir.join("events.jsonl"),
                        None,
                        None,
                        true,
                    );
                    assert!(replay.valid, "{dir:?} invalid: {:?}", replay.errors);
                    assert_eq!(
                        replay.terminal_event.as_deref(),
                        Some(terminal),
                        "{dir:?}"
                    );
                }

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }
}

/// The wired client: connection + message channel + session state.
pub struct InProcessClient {
    pub conn: Arc<acp::ClientSideConnection>,
    pub msg_rx: mpsc::UnboundedReceiver<ClientMsg>,
    msg_tx: mpsc::UnboundedSender<ClientMsg>,
    pub session_id: Option<String>,
    pub prompt_count: u32,
}

/// Wire the TUI client to the in-process AcpServer and spawn the three
/// driver tasks (client IO, agent IO, outbound gateway). Must run inside a
/// LocalSet (`spawn_local`).
pub fn connect_inprocess(server: Arc<AcpServer>, policy: TrustPolicy) -> InProcessClient {
    // Crossed duplex pairs: (client→agent) and (agent→client).
    let (client_write, agent_read) = tokio::io::duplex(1 << 16);
    let (agent_write, client_read) = tokio::io::duplex(1 << 16);

    let (msg_tx, msg_rx) = mpsc::unbounded_channel();

    // Client side — the TUI.
    let handler = TuiClientHandler { msg_tx: msg_tx.clone() };
    let (conn, client_io) = acp::ClientSideConnection::new(
        handler,
        client_write.compat_write(),
        client_read.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );

    // Agent side — reuses the host's stdio handler over the duplex.
    let agent_handler = StdioAgentHandler::with_trust_policy(server.clone(), policy);
    let (agent_conn, agent_io) = acp::AgentSideConnection::new(
        agent_handler,
        agent_write.compat_write(),
        agent_read.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );

    // Outbound gateway — permission prompts travel this path.
    let (sender, receiver) = acp_gateway::<acp::AgentSide, _>(agent_conn);
    server.set_gateway(sender);

    tokio::task::spawn_local(client_io);
    tokio::task::spawn_local(agent_io);
    tokio::task::spawn_local(receiver.run());

    InProcessClient {
        conn: Arc::new(conn),
        msg_rx,
        msg_tx,
        session_id: None,
        prompt_count: 0,
    }
}

impl InProcessClient {
    /// initialize + session/new; returns the agent-generated session id.
    pub async fn start_session(&mut self, cwd: PathBuf) -> acp::Result<String> {
        let _init = self
            .conn
            .initialize(acp::InitializeRequest::new(acp::ProtocolVersion::LATEST))
            .await?;
        let ns = self
            .conn
            .new_session(acp::NewSessionRequest::new(cwd))
            .await?;
        let session_id = ns.session_id.0.to_string();
        self.session_id = Some(session_id.clone());
        self.prompt_count = 0;
        Ok(session_id)
    }

    /// The journal path the host will write for the NEXT prompt
    /// (`{cwd}/.gsa/runs/RUN-{session8}-{n}/events.jsonl` — run-id scheme
    /// from acp_server.rs `handle_session_prompt`).
    pub fn run_dir_for_next_prompt(&self, cwd: &Path) -> Option<PathBuf> {
        let session_id = self.session_id.as_ref()?;
        let first8: String = session_id.chars().take(8).collect();
        let n = self.prompt_count;
        Some(
            cwd.join(".gsa")
                .join("runs")
                .join(format!("RUN-{first8}-{n}"))
                .join("events.jsonl"),
        )
    }

    /// Send one prompt and await the full run (blocks until the turn ends —
    /// callers spawn this as a task; live rendering comes from the tail).
    pub async fn prompt(&self, text: &str) -> acp::Result<acp::PromptResponse> {
        let session_id = self
            .session_id
            .clone()
            .ok_or_else(|| acp::Error::new(acp::ErrorCode::InternalError.into(), "no session started"))?;
        self.conn
            .prompt(acp::PromptRequest::new(
                session_id,
                vec![acp::ContentBlock::Text(acp::TextContent::new(text))],
            ))
            .await
    }

    /// Send the ACP `session/cancel` notification (Phase 3 slice #7). The
    /// host cancels the in-flight run cooperatively; idle or already-finished
    /// sessions make this a benign no-op.
    pub async fn request_cancel(&self) {
        let Some(session_id) = self.session_id.clone() else {
            return;
        };
        let _ = self
            .conn
            .cancel(acp::CancelNotification::new(session_id))
            .await;
    }

    /// Spawn the prompt as a local task; reports completion on the channel.
    ///
    /// `seq` stamps the completion with the caller's prompt counter — the
    /// generation guard against stale completions after a cancel.
    pub fn spawn_prompt(&self, session_id: String, text: String, seq: u32) {
        let conn = self.conn.clone();
        let msg_tx = self.msg_tx.clone();
        tokio::task::spawn_local(async move {
            let result = conn
                .prompt(acp::PromptRequest::new(
                    session_id,
                    vec![acp::ContentBlock::Text(acp::TextContent::new(&text))],
                ))
                .await;
            let _ = msg_tx.send(ClientMsg::PromptCompleted { seq, result });
        });
    }
}
