//! Runner — the LocalSet entry point that wires the workbench together.
//!
//! Constructs the AcpServer + in-process ACP duplex, attaches the journal
//! tail, and drives the `tokio::select!` loop: 50 ms render tick × crossterm
//! key events × client messages. Production raw-mode/alternate-screen setup
//! lives here; everything else is testable with a TestBackend.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossterm::event::{Event, KeyEventKind};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use agent_client_protocol as acp;

use orz_host::acp_server::AcpServer;
use orz_host::session::TrustPolicy;
use orz_loop::gateway::model::ModelGateway;

use crate::acp_client::{connect_inprocess, ClientMsg, InProcessClient};
use crate::app::{KeyOutcome, PendingPermission, PermissionOutcome, TuiApp};
use crate::events::TuiEvent;
use crate::input::handle_key;
use crate::source::{EventSource, JournalTailSource, ReplaySource};
use crate::widgets::{compose_screen, render_frame};

/// Render tick — Python drain loop parity (50 ms).
pub const TICK: std::time::Duration = std::time::Duration::from_millis(50);
/// Journal-appearance grace period before the fallback run-dir scan (plan §4).
pub const TAIL_FALLBACK_AFTER: std::time::Duration = std::time::Duration::from_secs(2);

/// Find the newest `{cwd}/.gsa/runs/RUN-{session8}-{n}/events.jsonl` — the
/// fallback when the deterministic run-dir path never produced events
/// (defense against counter drift; the host reserves run ids before
/// bootstrap, so this is normally unreachable).
fn find_newest_run_dir(cwd: &Path, session8: &str) -> Option<PathBuf> {
    let runs = cwd.join(".gsa").join("runs");
    let prefix = format!("RUN-{session8}-");
    let mut best: Option<(u64, PathBuf)> = None;
    for entry in std::fs::read_dir(runs).ok()? {
        let Ok(e) = entry else { continue };
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix(&prefix) else {
            continue;
        };
        if let Ok(n) = rest.parse::<u64>() {
            if best.as_ref().is_none_or(|(b, _)| n > *b) {
                best = Some((n, e.path().join("events.jsonl")));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Workbench configuration.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    /// Session working directory (host session cwd; journals land in
    /// `{cwd}/.gsa/runs/`).
    pub cwd: PathBuf,
    /// Replay a journal file instead of starting a live session.
    pub replay: Option<PathBuf>,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            replay: None,
        }
    }
}

/// Error type for the workbench entry points.
#[derive(Debug)]
pub enum TuiError {
    Io(io::Error),
    /// Replay path missing/unreadable.
    Replay(String),
    /// The live session failed to start (trust, transport).
    Session(String),
}

impl From<io::Error> for TuiError {
    fn from(e: io::Error) -> Self {
        TuiError::Io(e)
    }
}

impl std::fmt::Display for TuiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TuiError::Io(e) => write!(f, "tui io error: {e}"),
            TuiError::Replay(p) => write!(f, "replay failed: {p}"),
            TuiError::Session(s) => write!(f, "session error: {s}"),
        }
    }
}

impl std::error::Error for TuiError {}

/// Replay a journal through the projection and compose the screen — pure,
/// headless (used by `--replay` and tests).
pub fn replay_to_screen(path: &Path, width: u16, height: u16) -> Result<(Vec<String>, bool), TuiError> {
    let src = ReplaySource::new(path.to_path_buf())
        .map_err(|e| TuiError::Replay(e.to_string()))?;
    let valid = src.valid;
    let mut app = TuiApp::new();
    let mut src = Box::new(src) as Box<dyn EventSource>;
    let mut events = src.poll();
    // Replay emits the whole journal at once; drain nothing else.
    for ev in events.drain(..) {
        app.accept_event(ev);
    }
    Ok((compose_screen(&app, width, height), valid))
}

/// Static replay mode: compose and print the screen, then exit.
pub fn run_replay(path: &Path) -> Result<(), TuiError> {
    let (lines, valid) = replay_to_screen(path, 100, 30)?;
    for line in lines {
        println!("{line}");
    }
    println!();
    println!("日志链验证: {}", if valid { "有效" } else { "无效" });
    Ok(())
}

/// The live workbench loop. Must run inside a LocalSet (`spawn_local`).
///
/// Terminal setup and teardown bracket `run_loop`, so raw mode and the
/// alternate screen are ALWAYS restored — even when the loop exits on an
/// error (review P1-1: `?` inside the loop used to skip the teardown).
pub async fn run(config: TuiConfig, gateway: Arc<dyn ModelGateway>) -> Result<(), TuiError> {
    let server = Arc::new(AcpServer::with_gateway(gateway));
    let mut client = connect_inprocess(server, TrustPolicy::Enforce);
    let mut app = TuiApp::new();

    // Terminal setup (production only — tests never reach here).
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut keys = crossterm::event::EventStream::new();
    let mut ticker = tokio::time::interval(TICK);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    // Ensure the screen starts clean.
    terminal.clear()?;
    render_frame(&mut terminal, &mut app)?;

    let result = run_loop(
        &mut client,
        &mut app,
        &config.cwd,
        &mut terminal,
        &mut keys,
        &mut ticker,
    )
    .await;

    // Teardown — always runs, error or not.
    client.request_cancel().await;
    let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    let _ = crossterm::terminal::disable_raw_mode();
    result
}

/// Tail bookkeeping shared with the main loop.
struct TailState {
    tail: Option<JournalTailSource>,
    was_active: bool,
    started: Option<std::time::Instant>,
    ever_seen: bool,
}

impl TailState {
    fn new() -> Self {
        Self {
            tail: None,
            was_active: false,
            started: None,
            ever_seen: false,
        }
    }

    fn start(&mut self, path: PathBuf) {
        if let Some(mut old) = self.tail.take() {
            old.close();
        }
        self.tail = Some(JournalTailSource::new(path));
        self.was_active = true;
        self.started = Some(std::time::Instant::now());
        self.ever_seen = false;
    }

    fn stop(&mut self) {
        if let Some(mut t) = self.tail.take() {
            t.close();
        }
        self.was_active = false;
        self.started = None;
    }
}

/// The event loop proper — borrows the app/client/terminal so `run()` can
/// guarantee teardown after it returns (success or error).
#[allow(clippy::too_many_arguments)]
async fn run_loop(
    client: &mut InProcessClient,
    app: &mut TuiApp,
    cwd: &Path,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    keys: &mut crossterm::event::EventStream,
    ticker: &mut tokio::time::Interval,
) -> Result<(), TuiError> {
    let mut tail_state = TailState::new();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                // Journal tail → projection.
                if let Some(t) = tail_state.tail.as_mut() {
                    let active_now = t.is_active();
                    let events = t.poll();
                    if !events.is_empty() {
                        tail_state.ever_seen = true;
                    }
                    for ev in events {
                        app.accept_event(ev);
                    }
                    if tail_state.was_active && !active_now {
                        app.dismiss_permission_dialog("运行已结束");
                        tail_state.stop();
                    }
                    tail_state.was_active = active_now;
                    // Plan §4 fallback: the deterministic run-dir path never
                    // produced events within the grace period → re-point at
                    // the newest RUN-{session8}-* dir (defense; the host
                    // reserves run ids before bootstrap, so this is normally
                    // unreachable).
                    if !tail_state.ever_seen
                        && let Some(started) = tail_state.started
                        && started.elapsed() > TAIL_FALLBACK_AFTER
                        && let Some(session_id) = &client.session_id
                        && let Some(path) = find_newest_run_dir(
                            cwd,
                            &session_id.chars().take(8).collect::<String>(),
                        )
                    {
                        tail_state.start(path);
                    }
                }
                render_frame(terminal, app).map_err(TuiError::from)?;
            }
            key = keys.next() => {
                match key {
                    Some(Ok(Event::Key(k))) if k.kind == KeyEventKind::Press => {
                        match handle_key(app, k) {
                            KeyOutcome::Continue => {}
                            KeyOutcome::Quit => break Ok(()),
                            KeyOutcome::CancelRun => {
                                client.request_cancel().await;
                                app.content.add_system_message(
                                    "停止请求已发送（运行至完成）", false);
                            }
                            KeyOutcome::Prompt(text) => {
                                // Running guard (review P1-2): a second prompt
                                // while a turn is in flight would collide on
                                // the run-dir counter and corrupt the journal.
                                if app.running {
                                    app.content.add_system_message(
                                        "当前运行中——请等待完成或发送 /stop", false);
                                } else {
                                    run_prompt(client, app, &mut tail_state, cwd, text).await?;
                                }
                            }
                        }
                    }
                    Some(Ok(Event::Resize(..))) => {
                        terminal.autoresize().map_err(TuiError::from)?;
                    }
                    // EOF on the key stream (stdin closed / non-interactive
                    // invocation) is a quit path (review P2-2).
                    None | Some(Err(_)) => break Ok(()),
                    _ => {}
                }
            }
            msg = client.msg_rx.recv() => {
                match msg {
                    Some(ClientMsg::PermissionRequest { request, respond }) => {
                        show_permission(app, *request, respond);
                    }
                    Some(ClientMsg::SessionNotification { text_chunk, .. }) => {
                        if !text_chunk.is_empty() {
                            app.accept_event(TuiEvent::TextDelta { text: text_chunk });
                        }
                    }
                    Some(ClientMsg::PromptCompleted { result }) => {
                        match result {
                            Ok(_) => {
                                // The journal terminal event drives the UI;
                                // dismiss any orphaned permission dialog
                                // defensively.
                                app.dismiss_permission_dialog("运行已结束");
                            }
                            Err(e) => {
                                // Failed prompt (e.g. untrusted cwd): no run
                                // journal exists — stop the tail and return
                                // to the idle state (plan boundary: error
                                // surfaced in the content pane, app usable).
                                tail_state.stop();
                                app.running = false;
                                app.status.set_run_state("空闲", true);
                                app.content.add_system_message(
                                    &format!("[错误] 会话请求失败: {e}"), true);
                                app.dismiss_permission_dialog("运行已结束");
                            }
                        }
                    }
                    None => break Ok(()),
                }
            }
        }
        if app.quit {
            break Ok(());
        }
    }
}

/// Start (or reuse) the ACP session and spawn the prompt + journal tail.
async fn run_prompt(
    client: &mut InProcessClient,
    app: &mut TuiApp,
    tail_state: &mut TailState,
    cwd: &Path,
    text: String,
) -> Result<(), TuiError> {
    if client.session_id.is_none() {
        client
            .start_session(cwd.to_path_buf())
            .await
            .map_err(|e| TuiError::Session(e.to_string()))?;
        app.accept_event(TuiEvent::AcpSessionCreated {
            session_id: client.session_id.clone().unwrap_or_default(),
        });
    }
    let session_id = client
        .session_id
        .clone()
        .ok_or_else(|| TuiError::Session("no session id".into()))?;

    // Journal path for this prompt's run; start the tail BEFORE the prompt
    // task (the tailer retries a missing file until bootstrap creates it).
    if let Some(run_dir) = client.run_dir_for_next_prompt(cwd) {
        tail_state.start(run_dir);
    }
    client.prompt_count += 1;
    app.status.set_run_state("运行中", true);
    app.running = true;
    client.spawn_prompt(session_id, text);
    Ok(())
}

/// Map an ACP permission request into the permission dialog.
fn show_permission(
    app: &mut TuiApp,
    request: acp::RequestPermissionRequest,
    respond: tokio::sync::oneshot::Sender<acp::RequestPermissionResponse>,
) {
    let allow_once_id = request
        .options
        .iter()
        .find(|o| o.kind == acp::PermissionOptionKind::AllowOnce)
        .map(|o| o.option_id.0.to_string())
        .unwrap_or_else(|| "allow-once".to_string());
    let tool = request
        .tool_call
        .fields
        .title
        .clone()
        .unwrap_or_else(|| request.tool_call.tool_call_id.0.to_string());
    let args_summary = request
        .tool_call
        .fields
        .raw_input
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_default();
    app.show_permission_dialog(PendingPermission {
        tool,
        args_summary,
        respond: Some(Box::new(move |outcome| {
            let response = match outcome {
                PermissionOutcome::AllowOnce => {
                    acp::RequestPermissionResponse::new(
                        acp::RequestPermissionOutcome::Selected(
                            acp::SelectedPermissionOutcome::new(allow_once_id),
                        ),
                    )
                }
                PermissionOutcome::Cancelled => acp::RequestPermissionResponse::new(
                    acp::RequestPermissionOutcome::Cancelled,
                ),
            };
            let _ = respond.send(response);
        })),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_scan_picks_newest_run_dir() {
        let dir = std::env::temp_dir().join(format!("orz-tui-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Run dirs live under {cwd}/.gsa/runs/ (the function appends that).
        let runs = dir.join(".gsa").join("runs");
        std::fs::create_dir_all(runs.join("RUN-abcdef01-0")).unwrap();
        std::fs::create_dir_all(runs.join("RUN-abcdef01-5")).unwrap();
        std::fs::create_dir_all(runs.join("RUN-abcdef01-1")).unwrap();
        std::fs::create_dir_all(runs.join("RUN-other99-9")).unwrap();
        assert_eq!(
            find_newest_run_dir(&dir, "abcdef01"),
            Some(runs.join("RUN-abcdef01-5").join("events.jsonl"))
        );
        // Unknown session prefix → None.
        assert_eq!(find_newest_run_dir(&dir, "zzzzzzzz"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replay_to_screen_produces_validated_frame() {
        let dir = std::env::temp_dir().join(format!("orz-tui-replay-run-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        let mut ev0 = orz_assurance::journal::RunEvent::new(
            "RUN-REP02".into(),
            0,
            orz_assurance::journal::EventType::RunStarted,
            "m".into(),
            None,
            "run-event-v0.1.schema.json".into(),
            serde_json::json!({"prompt": "你好"}),
            orz_assurance::journal::Redaction::None,
            "2026-08-05T00:00:00Z".into(),
        );
        orz_assurance::seal_event(&mut ev0).unwrap();
        let mut ev1 = orz_assurance::journal::RunEvent::new(
            "RUN-REP02".into(),
            1,
            orz_assurance::journal::EventType::RunFinished,
            "m".into(),
            Some(ev0.event_sha256.clone()),
            "run-event-v0.1.schema.json".into(),
            serde_json::json!({"status": "completed"}),
            orz_assurance::journal::Redaction::None,
            "2026-08-05T00:00:01Z".into(),
        );
        orz_assurance::seal_event(&mut ev1).unwrap();
        let mut content = String::new();
        content.push_str(&serde_json::to_string(&ev0).unwrap());
        content.push('\n');
        content.push_str(&serde_json::to_string(&ev1).unwrap());
        content.push('\n');
        std::fs::write(&path, content).unwrap();

        let (lines, valid) = replay_to_screen(&path, 100, 30).unwrap();
        assert!(valid);
        let joined = lines.join("\n");
        assert!(joined.contains("你好"), "user card from run_started");
        assert!(joined.contains("[验证] 日志链有效"));

        // Tampered journal → invalid.
        std::fs::write(&path, "{broken}\n").unwrap();
        let (lines, valid) = replay_to_screen(&path, 100, 30).unwrap();
        assert!(!valid);
        let joined = lines.join("\n");
        assert!(joined.contains("日志链无效"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
