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
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use agent_client_protocol as acp;

use orz_host::acp_server::AcpServer;
use orz_host::session::TrustPolicy;
use orz_loop::gateway::model::ModelGateway;

use crate::acp_client::{ClientMsg, InProcessClient, connect_inprocess};
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
        if let Ok(n) = rest.parse::<u64>()
            && best.as_ref().is_none_or(|(b, _)| n > *b)
        {
            best = Some((n, e.path().join("events.jsonl")));
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
pub fn replay_to_screen(
    path: &Path,
    width: u16,
    height: u16,
) -> Result<(Vec<String>, bool), TuiError> {
    let src = ReplaySource::new(path.to_path_buf()).map_err(|e| TuiError::Replay(e.to_string()))?;
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
    // L1 (2026-08-08 write placement): library-level self-guard — redirect
    // `$GROK_HOME` off the user directory before any `orz_config::grok_home()`
    // call (OnceLock). Idempotent: when an entry already injected (orz-bin
    // main), this is a no-op `EnvRespected`. Design:
    // docs/WRITE_PLACEMENT_AND_GRILL_DESIGN_2026-08-08.md §1/§2.
    let placement = orz_host::grok_home::redirect_grok_home(&config.cwd);
    if matches!(
        placement,
        orz_host::grok_home::GrokHomePlacement::UserFallback
    ) {
        eprintln!(
            "warning: install dir and cwd/.gsa both unwritable — $GROK_HOME stays on the user directory (last resort)"
        );
    }
    let server = Arc::new(AcpServer::with_gateway(gateway));
    // The restore path keeps its own handle — connect_inprocess moves the
    // Arc into the agent-side handler (slice #10).
    let restore_server = server.clone();
    let mut client = connect_inprocess(server, TrustPolicy::Enforce);
    let mut app = TuiApp::new();
    app.cwd = config.cwd.clone();
    // Explorer tree: a one-shot load at startup (v1 snapshot — no live fs
    // refresh; the render path stays fs-free).
    app.explorer.load(&config.cwd);

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
        &restore_server,
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
    crate::title::restore_title();
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
    server: &AcpServer,
    client: &mut InProcessClient,
    app: &mut TuiApp,
    cwd: &Path,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    keys: &mut crossterm::event::EventStream,
    ticker: &mut tokio::time::Interval,
) -> Result<(), TuiError> {
    let mut tail_state = TailState::new();
    // OSC title cache — write only on change (slice #9).
    let mut last_title: Option<String> = None;

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
                    // Permission-dialog timeout (Phase 3 slice #7): the host
                    // denied at ~300s; close the orphaned dialog. The run
                    // continues — 运行中 restored (dismiss alone sets 空闲).
                    tick_timeout_dismiss(app);
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
                // Live event-group counts for the explorer pane (slice #9).
                app.explorer.refresh_events(&app.events_log);
                // OSC terminal title — derived from app state, written only
                // when it changes (no title churn on the 50ms tick).
                let title = crate::title::title_for(app);
                if last_title.as_deref() != Some(title.as_str()) {
                    crate::title::set_title(&title);
                    last_title = Some(title);
                }
                render_frame(terminal, app).map_err(TuiError::from)?;
            }
            key = keys.next() => {
                match key {
                    Some(Ok(Event::Key(k))) if k.kind == KeyEventKind::Press => {
                        match handle_key(app, k) {
                            KeyOutcome::Continue => {
                                // `/stop` typed while running marks intent;
                                // the runner owns the async cancel.
                                if std::mem::take(&mut app.stop_pending) {
                                    do_cancel(client, app).await;
                                }
                                // `/grill-finish` typed (2026-08-08 grill
                                // mode) — the runner owns the async summary
                                // turn + archive.
                                if std::mem::take(&mut app.pending_grill_finish) {
                                    do_finish_grill(server, client, app, terminal).await?;
                                }
                                // A confirmed snapshot restore (slice #10) —
                                // the runner owns the async restore call.
                                // The two intents are mutually exclusive by
                                // the app-side guards; drain both anyway.
                                if let Some(hash) = std::mem::take(&mut app.pending_restore) {
                                    // Review D2-1: the loop suspends for the
                                    // whole restore await, so render a frame
                                    // with 恢复中 BEFORE it — the user sees
                                    // live feedback instead of a freeze.
                                    app.restoring = true;
                                    app.status.set_run_state("恢复中", true);
                                    render_frame(terminal, app).map_err(TuiError::from)?;
                                    do_restore(server, client, app, cwd, hash).await?;
                                }
                            }
                            KeyOutcome::Quit => break Ok(()),
                            KeyOutcome::CancelRun => {
                                do_cancel(client, app).await;
                            }
                            KeyOutcome::Prompt(text) => {
                                // Running guard (review P1-2): a second prompt
                                // while a turn is in flight would collide on
                                // the run-dir counter and corrupt the journal.
                                // Restoring has no host in-flight marker
                                // (record P2-2) — the TUI is the only guard.
                                if app.running || app.restoring {
                                    app.content.add_system_message(
                                        "当前有运行/恢复进行中——请等待完成", false);
                                } else if app.grill_active {
                                    // Grill mode (2026-08-08): chat input is
                                    // the answer to the previous question —
                                    // a full model↔tool loop under the host's
                                    // ReadOnly policy, recorded to the grill
                                    // JSONL, never a run journal.
                                    run_grill_prompt(server, client, app, terminal, text).await?;
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
                    Some(ClientMsg::PromptCompleted { seq, result }) => {
                        // Generation guard (Phase 3 slice #7): after a cancel
                        // the PREVIOUS run's completion must not stop the new
                        // run's tail, clear `running`, or dismiss its dialog.
                        if seq != client.prompt_count {
                            continue;
                        }
                        match result {
                            Ok(response) => {
                                // The journal terminal event drives the UI;
                                // dismiss any orphaned permission dialog
                                // defensively.
                                app.dismiss_permission_dialog("运行已结束");
                                // Slice #9: a cancelled run restores its
                                // prompt for editing (do_cancel already did
                                // this — one-shot semantics make this a
                                // no-op; the seq guard above stays
                                // authoritative for which completion wins).
                                if response.stop_reason == acp::StopReason::Cancelled {
                                    app.restore_last_prompt();
                                }
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
                                app.restore_last_prompt();
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

/// User-requested cancel (Ctrl+Z or `/stop`, Phase 3 slice #7): send the ACP
/// cancel notification and give immediate UI feedback; the journal's
/// `run_cancelled` terminal event drives the rest (projection clears
/// `running`, sets 已取消). A pending permission dialog is dismissed so its
/// dropped oneshot resolves the manager's prompt to Cancelled — the loop's
/// next checkpoint then terminates the run.
async fn do_cancel(client: &mut InProcessClient, app: &mut TuiApp) {
    // A restore has no cancellation token on the host (record P3-7) and no
    // in-flight marker (record P2-2) — refusing here is the only guard: a
    // cancel would otherwise fall into the 2s pending_cancels window and
    // could pre-cancel the NEXT prompt.
    if app.restoring {
        app.content
            .add_system_message("恢复进行中——无法取消（恢复无取消令牌）", false);
        return;
    }
    if app.grill_active {
        // Grill mode (2026-08-08 review D2-6): grill turns carry no cancel
        // token (they are not runs) — surface that instead of the misleading
        // "cancelling" path that would do nothing.
        app.content
            .add_system_message("grill 轮不可取消——请等待本轮完成", false);
        return;
    }
    if app.running {
        client.request_cancel().await;
        app.content
            .add_system_message("停止请求已发送（正在取消）", false);
        app.dismiss_permission_dialog("用户取消");
        // The run is still winding down to its next checkpoint — the
        // dismiss above set 空闲 (from 等待审批); keep the status honest.
        // A queued request presented by the dismiss keeps 等待审批 and must
        // not be overwritten (2026-08-05 review P3-3).
        if app.pending_permission.is_none() {
            app.status.set_run_state("运行中", true);
        }
        // Slice #9: put the cancelled prompt back into the input for
        // editing/retry (Python `_cancel_run` parity — immediate feedback;
        // the completion handler restores too, one-shot semantics make the
        // second call a no-op).
        app.restore_last_prompt();
    } else {
        app.content.add_system_message("当前没有运行", false);
    }
}

/// Tick-driven permission-timeout dismissal (Phase 3 slice #7): the dialog
/// outlived the host's 300s prompt timeout — the host already denied (or
/// denies milliseconds later) — so close it without firing a response. The
/// run continues, so 运行中 is restored (dismiss alone would set 空闲).
fn tick_timeout_dismiss(app: &mut TuiApp) {
    if app.permission_timed_out() {
        app.dismiss_permission_dialog("已超时");
        // The run continues — restore 运行中 (dismiss alone sets 空闲). A
        // queued request presented by the dismiss keeps 等待审批 and must
        // not be overwritten (2026-08-05 review P3-3).
        if app.pending_permission.is_none() {
            app.status.set_run_state("运行中", true);
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
    // Slice #9 stale-flag fix (review P3): a `/stop` mark left over from a
    // run that ended on its own would otherwise cancel THIS run on the next
    // Continue drain. Submitting a prompt clears the intent.
    app.stop_pending = false;
    // Remember the prompt so a cancel can restore it into the input.
    app.last_prompt = Some(text.clone());
    let seq = client.prompt_count;
    client.spawn_prompt(session_id, text, seq);
    Ok(())
}

/// Restore a snapshot into the live session's worktree (slice #10). The
/// restore is its own `RST-` run on the host; its journal is never tailed
/// by the TUI (the tail and discover_sessions both match `RUN-` only), so
/// the returned report is the completion signal.
async fn do_restore(
    server: &AcpServer,
    client: &mut InProcessClient,
    app: &mut TuiApp,
    cwd: &Path,
    hash: String,
) -> Result<(), TuiError> {
    // Defense in depth — the selector's confirm already refuses while
    // running; a run could have started between confirm and this drain.
    // (Only `running` is checked here: the production drain arm pre-sets
    // `restoring` for the pre-render — the guard would refuse itself.)
    if app.running {
        app.content
            .add_system_message("当前运行中——请等待完成后再恢复", false);
        app.restoring = false;
        app.status.set_run_state("空闲", true);
        return Ok(());
    }
    // The host needs the full session UUID; only the client holds it (run
    // dirs encode 8 chars and no session.json markers exist). Mirror
    // run_prompt's auto-start.
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

    // NOTE: do NOT touch client.prompt_count — RST- ids come from the
    // host's independent restore_count, so the client's
    // run_dir_for_next_prompt derivation stays correct (host test
    // restore_and_prompt_run_ids_are_independent pins this).
    app.restoring = true;
    app.status.set_run_state("恢复中", true);
    match server.restore_snapshot(&session_id, &hash, None).await {
        Ok(report) => {
            let n = report
                .get("restored")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let short: String = hash.chars().take(8).collect();
            app.content
                .add_system_message(&format!("[快照恢复] {short} 恢复 {n} 个文件"), false);
        }
        Err(e) => {
            app.content
                .add_system_message(&format!("[快照恢复] 失败（{e}）"), true);
        }
    }
    app.status.set_run_state("空闲", true);
    app.restoring = false;
    Ok(())
}

/// Grill-mode turn (2026-08-08 write-placement slice, design §3): the
/// user's answer runs through the host's grill turn (full model↔tool loop
/// under the ReadOnly policy); the response is projected as a normal model
/// card. The grill session is NOT a run — no journal tail, no run state —
/// so `running` is reused as the in-flight guard (a second input cannot
/// overlap a turn).
async fn run_grill_prompt(
    server: &AcpServer,
    client: &mut InProcessClient,
    app: &mut TuiApp,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    text: String,
) -> Result<(), TuiError> {
    // Auto-start the session (mirror run_prompt / do_restore).
    if client.session_id.is_none() {
        client
            .start_session(app.cwd.clone())
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

    app.running = true;
    app.status.set_run_state("运行中", true);
    app.accept_event(TuiEvent::PromptSubmitted {
        prompt: text.clone(),
        character_count: text.chars().count() as u64,
    });
    // 2026-08-08 review P2-1: the grill turn awaits INLINE (host direct
    // call — no journal tail, no select polling), so render the 运行中 frame
    // BEFORE the await — otherwise the UI freezes silently for the whole
    // round (a multi-tool exploration can take minutes).
    render_frame(terminal, app).map_err(TuiError::from)?;
    match server.run_grill_turn(&session_id, &text).await {
        Ok(response) => {
            app.accept_event(TuiEvent::ModelOutput {
                text: response,
                tool_calls: Vec::new(),
                finish_reason: "stop".into(),
            });
        }
        Err(e) => {
            app.content
                .add_system_message(&format!("[grill] 本轮失败（{e}）"), true);
        }
    }
    app.running = false;
    app.status.set_run_state("空闲", true);
    Ok(())
}

/// `/grill-finish` (2026-08-08): one final summary turn ("共享理解达成" +
/// locked decision list), then the host archives the session and it is
/// cleared (design §3).
async fn do_finish_grill(
    server: &AcpServer,
    client: &mut InProcessClient,
    app: &mut TuiApp,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<(), TuiError> {
    let Some(session_id) = client.session_id.clone() else {
        app.content
            .add_system_message("无活跃会话——grill 无法结束", true);
        app.grill_active = false;
        return Ok(());
    };
    app.running = true;
    app.status.set_run_state("运行中", true);
    // Same inline-await freeze as run_grill_prompt — render the summary
    // turn's 运行中 frame before the await (review P2-1).
    render_frame(terminal, app).map_err(TuiError::from)?;
    match server.finish_grill(&session_id).await {
        Ok(summary) => {
            app.accept_event(TuiEvent::ModelOutput {
                text: summary,
                tool_calls: Vec::new(),
                finish_reason: "stop".into(),
            });
            app.content.add_system_message("[grill] 会话已归档", false);
        }
        Err(e) => {
            app.content
                .add_system_message(&format!("[grill] 结束失败（{e}）"), true);
        }
    }
    app.grill_active = false;
    app.running = false;
    app.status.set_run_state("空闲", true);
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
    app.show_permission_dialog(PendingPermission::new(
        tool,
        args_summary,
        Some(Box::new(move |outcome| {
            let response = match outcome {
                PermissionOutcome::AllowOnce => {
                    acp::RequestPermissionResponse::new(acp::RequestPermissionOutcome::Selected(
                        acp::SelectedPermissionOutcome::new(allow_once_id),
                    ))
                }
                PermissionOutcome::Cancelled => {
                    acp::RequestPermissionResponse::new(acp::RequestPermissionOutcome::Cancelled)
                }
            };
            let _ = respond.send(response);
        })),
    ));
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

    // ── Phase 3 slice #7: cancel + permission timeout ─────────────────────

    /// The tick poll dismisses an expired permission dialog and restores
    /// 运行中 — the run is still in flight (dismiss alone would set 空闲).
    #[test]
    fn tick_dismisses_timed_out_permission_and_restores_running_state() {
        use std::time::{Duration, Instant};

        let mut app = TuiApp::new();
        app.show_permission_dialog(PendingPermission::new("bash", "dir", None));
        // Backdate AFTER presentation — the dialog counts down from show
        // time (present_permission re-stamps the open instant).
        let expired = app.pending_permission.as_mut().unwrap();
        expired.opened_at = Instant::now() - Duration::from_secs(301);
        assert_eq!(app.status.items[5].label, "等待审批");

        tick_timeout_dismiss(&mut app);
        assert!(app.dialog.is_none(), "expired dialog dismissed");
        assert!(app.pending_permission.is_none());
        assert_eq!(
            app.status.items[5].label, "运行中",
            "the run continues past the timeout — status stays honest"
        );
    }

    /// `do_cancel` sends the ACP cancel, gives immediate feedback, and the
    /// run resolves with `StopReason::Cancelled` + a valid `run_cancelled`
    /// journal terminal (in-process duplex, like the host's stdio path).
    #[tokio::test]
    async fn do_cancel_sends_cancel_and_feedback() {
        use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
        use std::time::Duration;

        tokio::task::LocalSet::new()
            .run_until(async {
                let base = std::env::temp_dir().join(format!(
                    "orz-tui-runner-cancel-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
                let _ = std::fs::remove_dir_all(&base);
                std::fs::create_dir_all(&base).unwrap();

                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                    ])
                    .with_chunk_delay(Duration::from_millis(100)),
                )));
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                let mut app = TuiApp::new();
                client.start_session(base.to_path_buf()).await.unwrap();
                client.prompt_count += 1;
                let seq = client.prompt_count;
                let session_id = client.session_id.clone().unwrap();
                client.spawn_prompt(session_id, "回答我".to_string(), seq);
                app.running = true;

                do_cancel(&mut client, &mut app).await;
                let joined: Vec<String> = app
                    .content
                    .items
                    .iter()
                    .filter_map(|i| match i {
                        crate::view_model::ContentItem::Message(m) if m.role == "系统" => {
                            Some(m.content.clone())
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    joined
                        .iter()
                        .any(|m| m.contains("停止请求已发送（正在取消）")),
                    "immediate feedback: {joined:?}"
                );

                // The run resolves with StopReason::Cancelled (a success
                // response — the ACP protocol contract for cancellation).
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
                let response = completed.expect("cancel is a success response");
                assert_eq!(
                    response.stop_reason,
                    acp::StopReason::Cancelled,
                    "cancel resolves the prompt with StopReason::Cancelled"
                );

                let session8: String = client
                    .session_id
                    .as_ref()
                    .unwrap()
                    .chars()
                    .take(8)
                    .collect();
                let events_path = base
                    .join(".gsa")
                    .join("runs")
                    .join(format!("RUN-{session8}-0"))
                    .join("events.jsonl");
                let replay = orz_assurance::replay_journal(&events_path, None, None, true);
                assert!(replay.valid, "journal invalid: {:?}", replay.errors);
                assert_eq!(replay.terminal_event.as_deref(), Some("run_cancelled"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    // ── Phase 3 slice #9: stale stop_pending + restore-on-cancel ──────────

    /// A `/stop` mark left over from a run that finished on its own must not
    /// cancel the NEXT run (review P3 record: the runner drains stop_pending
    /// on Continue). `run_prompt` clears the flag at submit.
    #[tokio::test]
    async fn prompt_submit_clears_stale_stop_pending() {
        use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};

        tokio::task::LocalSet::new()
            .run_until(async {
                let base = std::env::temp_dir().join(format!(
                    "orz-tui-runner-stale-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
                let _ = std::fs::remove_dir_all(&base);
                std::fs::create_dir_all(&base).unwrap();

                let server = Arc::new(AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("完成。"),
                ]))));
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                let mut app = TuiApp::new();
                let mut tail = TailState::new();
                client.start_session(base.to_path_buf()).await.unwrap();

                // The stale scenario: stop_pending survives a finished run.
                app.stop_pending = true;
                app.last_prompt = None;
                run_prompt(&mut client, &mut app, &mut tail, &base, "新提示".into())
                    .await
                    .unwrap();
                assert!(
                    !app.stop_pending,
                    "submitting a prompt clears the stale cancel intent"
                );
                assert_eq!(
                    app.last_prompt.as_deref(),
                    Some("新提示"),
                    "the prompt is remembered for restore-on-cancel"
                );

                // Drain the run so no tail/thread leaks.
                let mut completed = false;
                while !completed {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { .. } => completed = true,
                        ClientMsg::SessionNotification { .. } => {}
                        ClientMsg::PermissionRequest { .. } => {}
                    }
                }
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Cancel restores the submitted prompt into the input — and never
    /// clobbers a user-typed replacement (in-process duplex E2E).
    #[tokio::test]
    async fn cancel_restores_last_prompt_only_when_input_empty() {
        use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
        use std::time::Duration;

        tokio::task::LocalSet::new()
            .run_until(async {
                let base = std::env::temp_dir().join(format!(
                    "orz-tui-runner-restore-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
                let _ = std::fs::remove_dir_all(&base);
                std::fs::create_dir_all(&base).unwrap();

                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                    ])
                    .with_chunk_delay(Duration::from_millis(100)),
                )));
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                let mut app = TuiApp::new();
                client.start_session(base.to_path_buf()).await.unwrap();
                client.prompt_count += 1;
                let seq = client.prompt_count;
                let session_id = client.session_id.clone().unwrap();
                client.spawn_prompt(session_id, "被取消的问题".to_string(), seq);
                app.running = true;
                app.last_prompt = Some("被取消的问题".into());

                do_cancel(&mut client, &mut app).await;
                assert_eq!(
                    app.input.textarea.text(),
                    "被取消的问题",
                    "the cancelled prompt is restored immediately"
                );
                // The user edits it while the run winds down.
                app.input.textarea.set_text("修改后的新问题");

                // The completion arrives with StopReason::Cancelled — the
                // restore must NOT clobber the user's edit (input non-empty).
                let completed = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PromptCompleted { seq: got, result } => {
                            assert_eq!(got, seq);
                            break result;
                        }
                        ClientMsg::SessionNotification { .. } => {}
                        ClientMsg::PermissionRequest { .. } => {}
                    }
                };
                let response = completed.expect("cancel is a success response");
                assert_eq!(response.stop_reason, acp::StopReason::Cancelled);
                // The one-shot restore already consumed last_prompt at
                // do_cancel; the completion handler's call is a no-op.
                app.restore_last_prompt();
                assert_eq!(
                    app.input.textarea.text(),
                    "修改后的新问题",
                    "a user-typed input is never clobbered"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    // ── Phase 3 slice #10: snapshot restore ─────────────────────────────

    fn runner_base(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "orz-tui-runner-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    /// `{base}/.gsa/runs/RST-{session8}-{n}/events.jsonl` — the restore
    /// run's journal (host-side restore_count numbering, slice #8).
    fn rst_journal(client: &InProcessClient, base: &Path, n: u32) -> PathBuf {
        let session8: String = client
            .session_id
            .as_ref()
            .expect("session started")
            .chars()
            .take(8)
            .collect();
        base.join(".gsa")
            .join("runs")
            .join(format!("RST-{session8}-{n}"))
            .join("events.jsonl")
    }

    fn system_messages(app: &TuiApp) -> Vec<String> {
        app.content
            .items
            .iter()
            .filter_map(|i| match i {
                crate::view_model::ContentItem::Message(m) if m.role == "系统" => {
                    Some(m.content.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Full restore through the host (host-test pattern, acp_server.rs):
    /// track a file, mutate it, restore — the worktree returns to the
    /// snapshot, a valid RST- journal is recorded, and the TUI surfaces
    /// the report from the direct call (RST journals are never tailed).
    #[tokio::test]
    async fn do_restore_restores_worktree_and_records_rst_journal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = runner_base("restore");
                std::fs::write(base.join("a.txt"), "v1").unwrap();
                let store = orz_assurance::session::snapshot::SnapshotStore::new(
                    base.join(".gsa").join("snapshots"),
                    base.clone(),
                )
                .unwrap();
                let record = store
                    .track(&[std::path::PathBuf::from("a.txt")])
                    .await
                    .unwrap();
                std::fs::write(base.join("a.txt"), "v2").unwrap();

                let server = Arc::new(AcpServer::new());
                let mut client = connect_inprocess(server.clone(), TrustPolicy::Skip);
                let mut app = TuiApp::new();
                client.start_session(base.to_path_buf()).await.unwrap();

                do_restore(&server, &mut client, &mut app, &base, record.snapshot_hash)
                    .await
                    .unwrap();

                // Worktree restored to the snapshot content.
                assert_eq!(std::fs::read_to_string(base.join("a.txt")).unwrap(), "v1");
                // The restore run's journal is a valid chain ending
                // run_finished (preflight → snapshot_restored → finished).
                let journal = rst_journal(&client, &base, 0);
                let replay = orz_assurance::replay_journal(&journal, None, None, true);
                assert!(replay.valid, "RST journal invalid: {:?}", replay.errors);
                assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));
                let msgs = system_messages(&app);
                assert!(
                    msgs.iter()
                        .any(|m| m.contains("[快照恢复]") && m.contains("1 个文件")),
                    "report surfaced: {msgs:?}"
                );
                assert_eq!(app.status.items[5].label, "空闲");
                assert!(!app.restoring, "restoring flag cleared");
                // Prompt count untouched — the next prompt still lands in
                // RUN-…-0 (host test restore_and_prompt_run_ids_are_independent).
                assert_eq!(client.prompt_count, 0);
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    #[tokio::test]
    async fn do_restore_starts_session_when_none() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = runner_base("autosess");
                std::fs::write(base.join("a.txt"), "v1").unwrap();
                let store = orz_assurance::session::snapshot::SnapshotStore::new(
                    base.join(".gsa").join("snapshots"),
                    base.clone(),
                )
                .unwrap();
                let record = store
                    .track(&[std::path::PathBuf::from("a.txt")])
                    .await
                    .unwrap();

                let server = Arc::new(AcpServer::new());
                let mut client = connect_inprocess(server.clone(), TrustPolicy::Skip);
                let mut app = TuiApp::new();
                assert!(client.session_id.is_none());

                do_restore(&server, &mut client, &mut app, &base, record.snapshot_hash)
                    .await
                    .unwrap();
                assert!(
                    client.session_id.is_some(),
                    "restore auto-starts the session (full UUID required)"
                );
                assert_eq!(std::fs::read_to_string(base.join("a.txt")).unwrap(), "v1");
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    #[tokio::test]
    async fn do_restore_refused_while_running() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = runner_base("refuse");
                let server = Arc::new(AcpServer::new());
                let mut client = connect_inprocess(server.clone(), TrustPolicy::Skip);
                let mut app = TuiApp::new();
                app.running = true;

                do_restore(&server, &mut client, &mut app, &base, "a".repeat(64))
                    .await
                    .unwrap();
                assert!(!base.join(".gsa").exists(), "no host side effects");
                assert!(
                    system_messages(&app)
                        .iter()
                        .any(|m| m.contains("请等待完成后再恢复")),
                    "refusal message surfaced"
                );
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    #[tokio::test]
    async fn do_restore_unknown_hash_surfaces_error_and_returns_idle() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = runner_base("bogus");
                let server = Arc::new(AcpServer::new());
                let mut client = connect_inprocess(server.clone(), TrustPolicy::Skip);
                let mut app = TuiApp::new();
                client.start_session(base.to_path_buf()).await.unwrap();

                do_restore(&server, &mut client, &mut app, &base, "b".repeat(64))
                    .await
                    .unwrap();
                // The host records the failure in the RST journal and
                // returns the original store error — the TUI surfaces it.
                assert!(
                    system_messages(&app)
                        .iter()
                        .any(|m| m.contains("[快照恢复] 失败")),
                    "error surfaced"
                );
                assert_eq!(app.status.items[5].label, "空闲", "status recovers");
                assert!(!app.restoring, "restoring flag cleared on error");
                let journal = rst_journal(&client, &base, 0);
                let replay = orz_assurance::replay_journal(&journal, None, None, true);
                assert!(replay.valid);
                assert_eq!(replay.terminal_event.as_deref(), Some("run_failed"));
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// A restore has no host cancellation token (record P3-7) — the TUI
    /// refuses cancel while restoring instead of letting the cancel fall
    /// into the 2s pending_cancels window and pre-cancel the next prompt.
    #[tokio::test]
    async fn do_cancel_refuses_while_restoring() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = runner_base("cancel-restore");
                let server = Arc::new(AcpServer::new());
                let mut client = connect_inprocess(server, TrustPolicy::Skip);
                let mut app = TuiApp::new();
                app.restoring = true;

                do_cancel(&mut client, &mut app).await;
                assert!(
                    system_messages(&app)
                        .iter()
                        .any(|m| m.contains("恢复进行中——无法取消")),
                    "restore refuses cancellation"
                );
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Full chain E2E: a real mutation run records a snapshot_created
    /// event (allow-once permission), then restore returns the worktree to
    /// the pre-mutation state — the selector's data source and the
    /// restore path working against real journal payloads.
    #[tokio::test]
    async fn restore_after_mutation_run_e2e() {
        use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
        use orz_loop::gateway::model::ToolCall;

        tokio::task::LocalSet::new()
            .run_until(async {
                let base = runner_base("e2e");
                std::fs::write(base.join("a.txt"), "v1").unwrap();

                let server = Arc::new(AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "search_replace".to_string(),
                        arguments: serde_json::json!({
                            "file_path": "a.txt",
                            "old_string": "v1",
                            "new_string": "v2",
                        }),
                        call_id: "call-1".to_string(),
                    }]),
                    // The counterexample gate consumes two identical texts
                    // per round (acp_client.rs quirk).
                    ScriptedResponse::text("完成。"),
                    ScriptedResponse::text("完成。"),
                ]))));
                let mut client = connect_inprocess(server.clone(), TrustPolicy::Skip);
                let mut app = TuiApp::new();
                client.start_session(base.to_path_buf()).await.unwrap();
                client.prompt_count += 1;
                let seq = client.prompt_count;
                let session_id = client.session_id.clone().unwrap();
                client.spawn_prompt(session_id, "把 v1 改成 v2".to_string(), seq);
                app.running = true;

                // Allow the mutation once.
                let completed = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        ClientMsg::PermissionRequest { request, respond } => {
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
                        }
                        ClientMsg::PromptCompleted { result, .. } => break result,
                        ClientMsg::SessionNotification { .. } => {}
                    }
                };
                assert!(completed.is_ok(), "prompt failed: {completed:?}");
                assert_eq!(std::fs::read_to_string(base.join("a.txt")).unwrap(), "v2");

                // The run journal carries the snapshot_created hash.
                let run_dir = base.join(".gsa").join("runs").join(format!(
                    "RUN-{}-0",
                    client
                        .session_id
                        .as_ref()
                        .unwrap()
                        .chars()
                        .take(8)
                        .collect::<String>()
                ));
                let content = std::fs::read_to_string(run_dir.join("events.jsonl")).unwrap();
                let mut hash: Option<String> = None;
                for line in content.lines() {
                    let v: serde_json::Value = serde_json::from_str(line).unwrap();
                    if v.get("event_type").and_then(|t| t.as_str()) == Some("snapshot_created")
                        && let Some(h) = v
                            .get("payload")
                            .and_then(|p| p.get("snapshot_hash"))
                            .and_then(|h| h.as_str())
                    {
                        hash = Some(h.to_string());
                        break;
                    }
                }
                let hash = hash.expect("snapshot_created hash in the journal");

                // Restore through the same direct path the selector arms.
                app.running = false;
                do_restore(&server, &mut client, &mut app, &base, hash)
                    .await
                    .unwrap();
                assert_eq!(
                    std::fs::read_to_string(base.join("a.txt")).unwrap(),
                    "v1",
                    "worktree returned to the pre-mutation state"
                );
                assert!(
                    system_messages(&app)
                        .iter()
                        .any(|m| m.contains("[快照恢复]") && m.contains("1 个文件")),
                    "report surfaced"
                );
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }
}
