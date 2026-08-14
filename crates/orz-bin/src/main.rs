// orz — assurance-first CLI agent workbench.
//
// Phase 1: minimal `-p/--prompt` entry point that satisfies the Phase 1
// acceptance criterion: `orz -p "hello"` → a valid hash-chained events.jsonl.
//
// Phase 2: scripted FakeProvider is the offline main path (deterministic gate
// chain, no real model/network). Full CLI (subcommands, config, TUI wiring,
// `--stdio` ACP server) arrives in Phase 2-4.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use orz_host::session::{SessionHandle, bootstrap_session};
use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::{Message, ModelGateway, Role, ToolCall};

fn main() {
    // L1 (2026-08-08 write placement): redirect `$GROK_HOME` off the user
    // directory to the orz install dir (degradation chain → `{cwd}/.gsa/
    // grok-home` → user dir). MUST run before any `orz_config::grok_home()`
    // call (OnceLock — late injection is a no-op). Design:
    // docs/WRITE_PLACEMENT_AND_GRILL_DESIGN_2026-08-08.md §1/§2.
    let placement = orz_host::grok_home::redirect_grok_home(
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );
    // Last resort (§2): only the user dir was writable — surface it (rare:
    // requires the install dir AND `{cwd}/.gsa` both unwritable).
    if matches!(
        placement,
        orz_host::grok_home::GrokHomePlacement::UserFallback
    ) {
        eprintln!(
            "warning: install dir and cwd/.gsa both unwritable — $GROK_HOME stays on the user directory (last resort)"
        );
    }
    // Minimal arg parsing (no clap yet — Phase 2+ adds the real CLI)
    let args: Vec<String> = std::env::args().collect();
    // `--real`: real DeepSeek transport for every entry (TUI / -p / --plan /
    // --stdio). Env flag so build_gateway stays the single decision point —
    // mirrors the `--fake-provider` → ORZ_FAKE_TOOL precedent. Fail-closed
    // lives in build_gateway: no credential → error exit, never a fake
    // fallback (alpha test ruling 2026-08-06).
    if args.iter().any(|a| a == "--real") {
        if args.iter().any(|a| a == "--fake-provider") {
            eprintln!("error: --real and --fake-provider are mutually exclusive");
            std::process::exit(2);
        }
        // SAFETY: single-threaded before any runtime starts (edition 2024 —
        // run_tui's --fake-provider precedent).
        unsafe {
            std::env::set_var("ORZ_REAL", "1");
        }
    }
    // `--allow-write` (2026-08-06 polyglot harness): headless runs grant
    // local file edits (Benchmark policy — reads + search_replace/write
    // auto-allow; bash/network still fail closed). Explicit opt-in; the
    // TUI/stdio interactive paths never read this env.
    if args.iter().any(|a| a == "--allow-write") {
        unsafe {
            std::env::set_var("ORZ_ALLOW_WRITE", "1");
        }
    }
    // P0-2 (2026-08-08 stall guards): `--max-wallclock <sec>` → env
    // (precedent: `--allow-write` → `ORZ_ALLOW_WRITE`). Model-invisible
    // total-time budget for headless runs; on expiry the run ends itself
    // with a `run_invalidated{status: wallclock}` terminal instead of the
    // harness's process-out hard kill (no terminal, no journal).
    if let Some(pos) = args.iter().position(|a| a == "--max-wallclock") {
        let secs = match args.get(pos + 1) {
            Some(s) => s.clone(),
            None => {
                eprintln!("error: --max-wallclock requires a number of seconds");
                std::process::exit(2);
            }
        };
        unsafe {
            std::env::set_var("ORZ_MAX_WALLCLOCK", secs);
        }
    }
    // GAP-RETRIEVAL-TOOLS (2026-08-10): `--retrieval-mode <off|local_browser|
    // framework_fallback>` — the session-level retrieval mode (ADR-0010
    // §3.7.1) for sessions created by this process (stdio/TUI). Explicit
    // selection only; invalid values exit 2.
    if let Some(pos) = args.iter().position(|a| a == "--retrieval-mode") {
        let mode = match args.get(pos + 1) {
            Some(s) => s.clone(),
            None => {
                eprintln!("error: --retrieval-mode requires off|local_browser|framework_fallback");
                std::process::exit(2);
            }
        };
        if !["off", "local_browser", "framework_fallback"].contains(&mode.as_str()) {
            eprintln!("error: --retrieval-mode must be off|local_browser|framework_fallback");
            std::process::exit(2);
        }
        unsafe {
            std::env::set_var("ORZ_RETRIEVAL_MODE", mode);
        }
    }
    if args.iter().any(|a| a == "--stdio") {
        run_stdio();
        return;
    }
    // `--replay` is a standalone static mode and takes precedence over the
    // other flags (review P3 #11: `--replay x -p hi` must not silently run
    // a live session).
    if let Some(pos) = args.iter().position(|a| a == "--replay") {
        let path = match args.get(pos + 1) {
            Some(p) => PathBuf::from(p),
            None => {
                eprintln!("error: --replay requires a journal path");
                std::process::exit(2);
            }
        };
        run_replay_entry(&path);
        return;
    }
    // Bare `orz` (or `--run-root` / `--fake-provider` only) launches the
    // assurance workbench TUI — Python `gsa` precedent.
    let cli_mode = args
        .iter()
        .any(|a| a == "-p" || a == "--prompt" || a == "--plan");
    if !cli_mode {
        run_tui();
        return;
    }
    // Headless `-p`/`--plan` diagnostics (2026-08-08): init the same stderr
    // tracing subscriber as the TUI so transport retries/watchdog events are
    // visible when a headless run hangs or fails (TB2 dna-assembly hang had
    // zero stderr because no subscriber existed). `try_init` is idempotent —
    // the TUI path never reaches here and `run_tui` re-inits harmlessly.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();

    let prompt = parse_prompt(&args).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        eprintln!(
            "usage: orz -p \"<prompt>\"  (or --prompt <prompt>; --stdio for ACP; bare orz for TUI)"
        );
        eprintln!(
            "       --real selects the real DeepSeek transport (ADR-0006 credential registry)"
        );
        eprintln!(
            "       --allow-write grants headless local file edits (harness; bash/network still denied)"
        );
        eprintln!(
            "       --max-wallclock <sec> bounds the whole run (model-invisible; run_invalidated on expiry)"
        );
        eprintln!(
            "       stall watchdog: ORZ_STALL_TIMEOUT=<sec> no-activity window (default 360, 0 disables)"
        );
        eprintln!(
            "       per-tool timeout: ORZ_TOOL_TIMEOUT_SECS=<sec> (default 300, 0 disables; interactive escape hatch)"
        );
        std::process::exit(2);
    });
    // P0-2/P1-1 (2026-08-08 stall guards): resolve the guards once for
    // every headless entry — a malformed value is fail-closed (exit 2),
    // never a silently disabled guard.
    let wallclock = max_wallclock().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    let stall_timeout = max_stall_timeout().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });

    if args.iter().any(|a| a == "--plan") {
        run_plan(&prompt, wallclock, stall_timeout);
        return;
    }

    // Bootstrap a runtime for the async journal writer. The IP6 permission
    // bridge spawns its manager actor via `spawn_local` — everything runs
    // inside a LocalSet (same shape as `run_stdio`).
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            std::process::exit(1);
        }
    };

    let local = tokio::task::LocalSet::new();
    let result = local.block_on(&rt, run(&prompt, wallclock, stall_timeout));

    match result {
        Ok((response, events_path)) => {
            println!("{response}");
            println!();
            println!("journal: {}", events_path.display());
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

/// Parse `--run-root <dir>` (default: current dir).
fn parse_run_root(args: &[String]) -> Result<PathBuf, String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--run-root" {
            return args
                .get(i + 1)
                .map(PathBuf::from)
                .ok_or_else(|| "missing value after --run-root".to_string());
        }
    }
    std::env::current_dir().map_err(|e| e.to_string())
}

/// Bare-`orz` entry: launch the assurance workbench TUI with the in-process
/// ACP host. `--fake-provider` forces the scripted tool path.
fn run_tui() {
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--fake-provider") {
        // Reuse the scripted-tool branch of build_gateway (unsafe in 2024
        // edition; single-threaded before the runtime starts).
        unsafe {
            std::env::set_var("ORZ_FAKE_TOOL", "1");
        }
    }
    let cwd = match parse_run_root(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            std::process::exit(1);
        }
    };
    let local = tokio::task::LocalSet::new();
    let cfg = orz_tui::TuiConfig { cwd, replay: None };
    let result = local.block_on(&rt, orz_tui::run(cfg, build_gateway()));
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// `--replay <journal>`: static replay of a journal through the projection.
fn run_replay_entry(path: &std::path::Path) {
    if let Err(e) = orz_tui::run_replay(path) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): read `ORZ_RETRIEVAL_MODE`
/// (set by `--retrieval-mode`) — the session-level retrieval mode for
/// sessions created by this process; `None` = the `off` default.
fn retrieval_mode_from_env() -> Option<orz_loop::controller::RetrievalMode> {
    std::env::var("ORZ_RETRIEVAL_MODE")
        .ok()
        .and_then(|v| orz_loop::controller::RetrievalMode::from_wire(Some(&v)))
}

/// ACAF Slice 1 (ADR-0011 §4.4): spawn the signer-process client when the
/// launch chain configures it (`ORZ_ACAF_MANIFEST` + `ORZ_ACAF_KEYSTORE`
/// both set). Unconfigured → `None` (unticketed control events, zero
/// behaviour change). The signer binary defaults to `orz-signer` on PATH
/// (`ORZ_ACAF_BINARY` overrides — the trusted launcher pins the path).
async fn build_acaf_client() -> Result<
    Option<std::sync::Arc<tokio::sync::Mutex<orz_loop::acaf::AcafClient>>>,
    Box<dyn std::error::Error>,
> {
    let manifest = std::env::var("ORZ_ACAF_MANIFEST").ok();
    let keystore = std::env::var("ORZ_ACAF_KEYSTORE").ok();
    let (Some(manifest), Some(keystore)) = (manifest, keystore) else {
        return Ok(None);
    };
    let binary = std::env::var("ORZ_ACAF_BINARY")
        .ok()
        .map(std::path::PathBuf::from);
    let client = orz_loop::acaf::AcafClient::spawn(&orz_loop::acaf::AcafConfig {
        manifest_path: std::path::PathBuf::from(manifest),
        keystore_root: std::path::PathBuf::from(keystore),
        signer_binary: binary,
    })
    .await?;
    Ok(Some(std::sync::Arc::new(tokio::sync::Mutex::new(client))))
}

/// Slice 2 fail-closed switch (2026-08-13): enabled only by an explicit
/// `ORZ_ACAF_FAIL_CLOSED=1` / `=true` (case-insensitive). Any other value
/// (including `0`, `false`, or unset) keeps the default shadow mode —
/// value semantics, not presence: `ORZ_ACAF_FAIL_CLOSED=0` must not
/// silently flip a security gate on (review fix 2026-08-13).
fn acaf_fail_closed_enabled() -> bool {
    std::env::var("ORZ_ACAF_FAIL_CLOSED")
        .map(|v| v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// ACP stdio server entry: serve `session/new` + `session/prompt` over the
/// persistent stdio JSON-RPC stream until the client closes stdin.
fn run_stdio() {
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            std::process::exit(1);
        }
    };
    let local = tokio::task::LocalSet::new();
    let result = local.block_on(&rt, async {
        let server = std::sync::Arc::new(orz_host::acp_server::AcpServer::with_gateway(
            build_gateway(),
        ));
        orz_host::stdio::run_stdio_server(server, retrieval_mode_from_env()).await
    });
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// Plan-mode entry: walk the plan state machine (enter → submit → approve),
/// journal the plan events, then execute the turn under the approved plan.
/// Demonstrates the plan/action two-level approval separation end-to-end.
///
/// P0-2/P1-1 (2026-08-08 stall guards): `wallclock` bounds the plan phase
/// AND the execution turn (bootstrap excluded); the stall watchdog ends the
/// run on window-long silence (journal events, model wire frames and tool
/// execution all keep the heartbeat alive — the plan gate's own stream
/// included). On either expiry the run future is dropped and a graceful
/// `run_invalidated{status: wallclock|stall}` terminal is recorded
/// continuing the chain from the file.
fn run_plan(prompt: &str, wallclock: Option<Duration>, stall_timeout: Option<Duration>) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            std::process::exit(1);
        }
    };
    // The IP6 permission bridge needs a LocalSet (spawn_local manager).
    let local = tokio::task::LocalSet::new();
    let result = local.block_on(&rt, async {
        let run_id = format!("RUN-PLAN-{}", timestamp_suffix());
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let handle = bootstrap_session(
            &run_id,
            Some(cwd.clone()),
            orz_host::session::TrustPolicy::Enforce,
        )
        .await
        .map_err(|e| e.to_string())?;

        // P1-1: the heartbeat covers the plan gate round too — its stream
        // frames must keep the watchdog armed during a long gate think.
        let heartbeat = orz_loop::gateway::model::ActivityClock::new();
        let work = async {
            let mut seq = handle.next_sequence;
            let mut prev_hash = handle.last_event_sha256.clone();
            let gateway = build_gateway();
            // v1.15 (2026-08-14): the blackboard plan-epoch archive lives
            // under the session cwd's `.gsa/blackboard`. A fresh CLI run
            // starts its epoch AFTER every epoch already archived there, so
            // workspace-level epoch numbers stay unique across runs.
            let blackboard_archive_dir = cwd.join(".gsa").join("blackboard");
            let plan_epoch = orz_loop::epoch::next_plan_epoch_from_archive(&blackboard_archive_dir);

            // Plan phase — on error the journal must still terminate
            // (review P2-1): record RunFailed continuing the chain, then
            // exit.
            let artifact = match run_plan_phase(
                &handle,
                &mut seq,
                &mut prev_hash,
                &run_id,
                prompt,
                &gateway,
                &heartbeat,
                plan_epoch,
            )
            .await
            {
                Ok(artifact) => artifact,
                Err(e) => {
                    record_plan_failure(&handle, seq, prev_hash.clone(), &e).await;
                    let _ = handle.journal.shutdown_async().await;
                    return Err(e);
                }
            };

            // Execute under the approved plan — real host + IP6 bridge,
            // sharing the gateway instance (its script continues after the
            // gate round).
            let host = build_cli_host(&handle, &run_id, &cwd)?;
            // A4 (2026-08-08): the approved plan maps into the blackboard
            // plan section — goal = the task prompt, steps = the plan
            // sections — so the resident status line and blackboard_read
            // (plan partition) see it. The section mapping is deterministic
            // (plan artifacts are mechanically derived today; model-generated
            // plans later keep the same ingestion point).
            let controller = orz_loop::AgentLoopController::with_gateway(gateway)
                .with_snapshot_store(Some(handle.snapshot_store.clone()))
                .with_blackboard_archive_dir(Some(blackboard_archive_dir))
                .with_plan(
                    artifact.plan_id.clone(),
                    plan_epoch,
                    prompt.to_string(),
                    artifact
                        .sections
                        .iter()
                        .map(|s| s.title.clone())
                        .collect(),
                );
            let (response, _, _) = controller
                .run_turn_with_guards(
                    &host,
                    prompt,
                    &handle.run_id,
                    &handle.run_manifest_sha256,
                    seq,
                    prev_hash,
                    None,
                    Some(&heartbeat),
                    // GAP-INQUIRY-SPLIT: one-shot CLI runs carry no session
                    // orientation state (the ACP server hosts sessions).
                    // GAP-CONVERSATION-RESTORE: one-shot CLI runs carry no
                    // session conversation either.
                    None,
                    None,
                )
                .await
                .map_err(|e| e.to_string())?;
            handle
                .journal
                .shutdown_async()
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((response, handle.journal_dir.join("events.jsonl")))
        };
        let guarded = async {
            match stall_timeout {
                Some(timeout) => tokio::select! {
                    r = work => r,
                    _ = stall_fire(&heartbeat, timeout) => {
                        tracing::warn!(?timeout, "stall watchdog fired — no activity for the window");
                        let note = record_guard_terminal(&handle, "stall").await?;
                        let _ = handle.journal.shutdown_async().await;
                        Ok::<_, String>((note, handle.journal_dir.join("events.jsonl")))
                    }
                },
                None => work.await,
            }
        };
        match wallclock {
            Some(budget) => match tokio::time::timeout(budget, guarded).await {
                Ok(result) => result,
                Err(_) => {
                    tracing::warn!(
                        ?budget,
                        "max-wallclock reached — recording run_invalidated terminal"
                    );
                    let note = record_guard_terminal(&handle, "wallclock").await?;
                    let _ = handle.journal.shutdown_async().await;
                    Ok::<_, String>((note, handle.journal_dir.join("events.jsonl")))
                }
            },
            None => guarded.await,
        }
    });

    match result {
        Ok((response, events_path)) => {
            println!("{response}");
            println!();
            println!("journal: {}", events_path.display());
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

/// Plan phase: enter the state machine → counterexample gate round (§4.6.1,
/// before the plan write) → submit → approve, journaling the plan events and
/// advancing the chain (`seq`/`prev_hash`). Returns the approved artifact on
/// success (the caller ingests it into the blackboard plan section — A4);
/// the caller terminates the journal on error (review P2-1: plan-phase
/// failures must not leave the journal without a terminal event).
async fn run_plan_phase(
    handle: &orz_host::session::SessionHandle,
    seq: &mut u64,
    prev_hash: &mut Option<String>,
    run_id: &str,
    prompt: &str,
    gateway: &Arc<dyn ModelGateway>,
    heartbeat: &orz_loop::gateway::model::ActivityClock,
    plan_epoch: u64,
) -> Result<orz_assurance::plan::PlanArtifact, String> {
    let mut sm = orz_assurance::plan::PlanStateMachine::new();
    sm.enter_planning(None).map_err(|e| format!("plan: {e}"))?;
    let artifact = plan_artifact_from_prompt(run_id, prompt);
    let verification = orz_assurance::plan::verify_plan_artifact(&artifact);
    if !verification.valid {
        return Err(format!("plan artifact invalid: {:?}", verification.errors));
    }

    // §4.6.1: counterexample gate BEFORE the plan write — one model round
    // with the plan text in context. The artifact is mechanically derived
    // (no model involvement today), so the gate fires as evidence only;
    // it becomes a blocking link of the plan approval gate chain once
    // plans are model-generated. The shared gateway's script order is
    // gate round first, then the execution turn.
    //
    // P8 (FIX_PLAN 2026-08-06, D-7): this used the non-streaming `generate`,
    // which carried no read timeout — a stalled wire could block `-p`
    // indefinitely. All model rounds now go through `generate_stream` (the
    // idle watchdog / total budget live there); the gate's text deltas go
    // nowhere (deltas are never journaled).
    let gate_response = gateway
        .generate_stream(
            orz_loop::gateway::model::ModelRequest {
                system: orz_loop::prompt::BASE_SYSTEM_PROMPT.to_string(),
                messages: vec![
                    Message {
                        role: Role::User,
                        content: render_plan_for_gate(&artifact),
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    },
                    Message {
                        role: Role::User,
                        content: orz_loop::prompt::COUNTEREXAMPLE_GATE_PLAN_BLOCK.to_string(),
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    },
                ],
                tools: Vec::new(),
                // F-07 (2026-08-07 review): the gate is a FAST preflight round
                // (max_tokens=1024) — under the default thinking-max config the
                // reasoning would eat the whole budget and force the empty-
                // content retry chain (up to 3 model calls per `-p`). Explicit
                // override: thinking disabled, all output routed to content.
                max_tokens: 1024,
                thinking: Some(orz_loop::gateway::model::ThinkingMode::Disabled),
            },
            None,
            // P1-1 (2026-08-08 stall guards): the gate's own wire frames
            // keep the stall heartbeat alive.
            Some(heartbeat),
            &mut |_| {},
        )
        .await
        .map_err(|e| format!("plan gate: {e}"))?;
    let h0 = record_plan_event(
        handle,
        *seq,
        prev_hash.clone(),
        orz_assurance::EventType::CounterexampleGate,
        serde_json::json!({
            "position": "plan_write",
            "message_block": orz_loop::prompt::COUNTEREXAMPLE_GATE_PLAN_BLOCK,
            "once_only": false,
            "model_response": gate_response.text.unwrap_or_default(),
        }),
    )
    .await?;
    *seq += 1;
    *prev_hash = Some(h0);

    sm.submit_plan(artifact.clone())
        .map_err(|e| format!("plan: {e}"))?;
    let h1 = record_plan_event(
        handle,
        *seq,
        prev_hash.clone(),
        orz_assurance::EventType::PlanProposed,
        serde_json::json!({
            "plan_id": artifact.plan_id,
            "task_id": artifact.task_id,
            "sections": artifact.sections.len(),
        }),
    )
    .await?;
    *seq += 1;
    *prev_hash = Some(h1);
    let approval = sm
        .approve("user", Some("manual"))
        .map_err(|e| format!("plan: {e}"))?;
    let h2 = record_plan_event(
        handle,
        *seq,
        prev_hash.clone(),
        orz_assurance::EventType::PlanApproved,
        serde_json::json!({
            "plan_id": approval.plan_id,
            "authority": approval.authority,
            "decision": approval.decision.as_str(),
            "execution_policy": sm.approval_policy,
            "plan_epoch": plan_epoch,
        }),
    )
    .await?;
    *seq += 1;
    *prev_hash = Some(h2);
    Ok(artifact)
}

/// Record a terminal RunFailed continuing the hash chain — the plan-phase
/// failure path (review P2-1). Best effort: if the journal itself is dead the
/// original error is returned regardless by the caller.
async fn record_plan_failure(
    handle: &orz_host::session::SessionHandle,
    seq: u64,
    prev_hash: Option<String>,
    error: &str,
) {
    let _ = record_plan_event(
        handle,
        seq,
        prev_hash,
        orz_assurance::EventType::RunFailed,
        serde_json::json!({"error": error}),
    )
    .await;
}

/// Write one plan-mode event, continuing the session hash chain.
/// Returns the new event hash (the next event's `previous_event_sha256`).
async fn record_plan_event(
    handle: &orz_host::session::SessionHandle,
    seq: u64,
    prev_hash: Option<String>,
    event_type: orz_assurance::EventType,
    payload: serde_json::Value,
) -> Result<String, String> {
    let mut event = orz_assurance::RunEvent::new_v02(
        handle.run_id.clone(),
        seq,
        event_type,
        handle.run_manifest_sha256.clone(),
        prev_hash,
        orz_assurance::EventTrack::V02.payload_schema_id().into(),
        payload,
        orz_assurance::Redaction::None,
        chrono_utc_now(),
    );
    orz_assurance::seal_event(&mut event).map_err(|e| e.to_string())?;
    let hash = event.event_sha256.clone();
    handle
        .journal
        .record_async(event)
        .await
        .map_err(|e| e.to_string())?;
    Ok(hash)
}

/// Parse the max-wallclock budget from a raw value (`ORZ_MAX_WALLCLOCK`).
/// `None`/`0` = unbounded (default); anything non-numeric is an error
/// (fail-closed — a typo must not silently disable the guard).
fn parse_max_wallclock(raw: Option<String>) -> Result<Option<Duration>, String> {
    let Some(raw) = raw else { return Ok(None) };
    let secs: u64 = raw
        .trim()
        .parse()
        .map_err(|_| format!("ORZ_MAX_WALLCLOCK must be a number of seconds, got {raw:?}"))?;
    if secs == 0 {
        Ok(None)
    } else {
        Ok(Some(Duration::from_secs(secs)))
    }
}

/// The resolved max-wallclock budget (from `--max-wallclock` → env).
fn max_wallclock() -> Result<Option<Duration>, String> {
    parse_max_wallclock(std::env::var("ORZ_MAX_WALLCLOCK").ok())
}

/// Parse the stall-watchdog timeout from `ORZ_STALL_TIMEOUT` (seconds).
/// Missing = the 5min default; `0` = disabled. Non-numeric is an error
/// (fail-closed — a typo must not silently disable the guard).
fn parse_max_stall_timeout(raw: Option<String>) -> Result<Option<Duration>, String> {
    let Some(raw) = raw else {
        return Ok(Some(STALL_TIMEOUT_DEFAULT));
    };
    let secs: u64 = raw
        .trim()
        .parse()
        .map_err(|_| format!("ORZ_STALL_TIMEOUT must be a number of seconds, got {raw:?}"))?;
    if secs == 0 {
        // 0 = explicitly disabled.
        Ok(None)
    } else {
        Ok(Some(Duration::from_secs(secs)))
    }
}

/// The resolved stall watchdog timeout (from `ORZ_STALL_TIMEOUT`, default
/// 5min, `0` disables).
fn max_stall_timeout() -> Result<Option<Duration>, String> {
    parse_max_stall_timeout(std::env::var("ORZ_STALL_TIMEOUT").ok())
}

/// P1-1 (2026-08-08 stall guards): default no-activity watchdog window.
///
/// 360s — NOT the plan's literal 5min — to keep the guards deterministic:
/// the per-tool timeout (`TOOL_CALL_TIMEOUT`, 300s) fires FIRST on a hung
/// tool, so the tool path always ends with the P0-1 semantics (tree kill +
/// model continues) and the stall watchdog only ever fires on silence
/// OUTSIDE a tool call (between-round code, permission waits, retry-chain
/// backpressure). With equal windows the two guards raced (2026-08-08
/// review P2-1/D2-1) and a stall win would skip the tree kill entirely.
pub const STALL_TIMEOUT_DEFAULT: Duration = Duration::from_secs(360);

/// P1-1: the stall watchdog — fires once the heartbeat has been silent
/// for `timeout`. Polls every 500ms (a poll is cheap; the granularity is
/// irrelevant at the 5min scale).
async fn stall_fire(heartbeat: &orz_loop::gateway::model::ActivityClock, timeout: Duration) {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if heartbeat.idle() >= timeout {
            return;
        }
    }
}

/// P0-2/P1-1 (2026-08-08 stall guards): graceful guard terminal — the run
/// future was dropped by the wallclock deadline or the stall watchdog, so
/// the caller-owned chain state (inside the controller's `EventWriter`) is
/// lost. Recover the chain position from the journal FILE and append
/// `run_invalidated{status: <guard>}` so the journal ends on a terminal
/// event and replays valid.
///
/// Ordering: `flush_async` FIRST — the recorder channel is FIFO, so every
/// event the dropped controller queued lands before the flush completes;
/// reading the file afterwards yields a consistent chain end. The write
/// goes through `record_plan_event` (same `run-event-v0.2.schema.json`
/// payload schema as the controller's writer — both flipped to the v0.2
/// track by GAP-INQUIRY-SPLIT) so the hash chain, event_id derivation
/// (`{:03}`) and terminal semantics all match the production path.
///
/// If the file ALREADY ends on a terminal event (the run finished in the
/// same instant the guard fired — a benign select race), nothing is
/// written and a note is returned instead of an error.
async fn record_guard_terminal(
    handle: &orz_host::session::SessionHandle,
    status: &str,
) -> Result<String, String> {
    handle
        .journal
        .flush_async()
        .await
        .map_err(|e| format!("{status}: journal flush: {e}"))?;
    let content = std::fs::read_to_string(handle.journal_dir.join("events.jsonl"))
        .map_err(|e| format!("{status}: read journal: {e}"))?;
    let last = content
        .lines()
        .last()
        .ok_or_else(|| format!("{status}: empty journal"))?;
    let last: serde_json::Value =
        serde_json::from_str(last).map_err(|e| format!("{status}: parse last event: {e}"))?;
    let last_type = last["event_type"].as_str().unwrap_or_default();
    if matches!(
        last_type,
        "run_finished" | "run_failed" | "run_cancelled" | "run_invalidated"
    ) {
        return Ok(format!(
            "({status}) guard fired but the run already terminated ({last_type}) — no extra event"
        ));
    }
    let seq = last["sequence"]
        .as_u64()
        .ok_or_else(|| format!("{status}: last event has no sequence"))?
        + 1;
    let prev_hash = last["event_sha256"]
        .as_str()
        .ok_or_else(|| format!("{status}: last event has no event_sha256"))?
        .to_string();
    record_plan_event(
        handle,
        seq,
        Some(prev_hash),
        orz_assurance::EventType::RunInvalidated,
        serde_json::json!({"status": status}),
    )
    .await?;
    Ok(format!(
        "({status}) guard fired — run_invalidated{{status: {status}}}"
    ))
}

/// Derive a 4-section plan artifact from the prompt (Phase 2 minimal shape).
fn plan_artifact_from_prompt(run_id: &str, prompt: &str) -> orz_assurance::plan::PlanArtifact {
    let section = |title: &str, content: &str| orz_assurance::plan::PlanSection {
        title: title.to_string(),
        content_md: content.to_string(),
        evidence_status: "derived".to_string(),
        source_refs: Vec::new(),
    };
    orz_assurance::plan::PlanArtifact::new(
        format!("PLAN-{run_id}"),
        format!("TASK-{run_id}"),
        run_id,
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string()),
        "manual",
        vec![
            section("前期调查", &format!("任务背景：{prompt}")),
            section("具体计划", &format!("按提示执行：{prompt}")),
            section(
                "具体设计",
                "使用内置工具链（read_file / run_terminal_cmd / grep）",
            ),
            section("实施方案", &format!("直接执行提示：{prompt}")),
        ],
    )
}

fn chrono_utc_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Scripted provider: default = plain text response; ORZ_FAKE_TOOL=1
/// exercises the full tool loop for the E2E gate-chain acceptance.
///
/// Since Phase 3 wiring the loop demonstrates both IP6 semantics in one
/// run: `read_file` auto-allows through the permission bridge and executes
/// on the real GrokBuild toolset; `bash` (SandboxEscape) has no interactive
/// client headless → denied before execution. `rust-toolchain.toml` is the
/// demo target when run from the orz workspace — small and non-repetitive,
/// so the run ends with a clean `run_finished` (a large file like
/// `Cargo.toml` trips the stagnation ngram guard honestly, and the run
/// correctly ends `run_invalidated`).
///
/// §4.6 (Phase 3): the final-answer counterexample gate adds one model round
/// per turn — the script carries headroom. `--plan` consumes one extra entry
/// up front for the plan-write gate round (with ORZ_FAKE_TOOL=1 that entry is
/// a tool_calls response whose text is None → `model_response` is "" — a
/// demo-path-only artifact of the scripted provider).
fn build_gateway() -> Arc<dyn ModelGateway> {
    // `--real` (main sets ORZ_REAL for every entry): the real DeepSeek
    // transport. Fail-closed — missing credentials exit(2) with the ADR-0006
    // target named, never a silent FakeProvider fallback.
    if std::env::var("ORZ_REAL").is_ok() {
        match orz_loop::gateway::transport::real_gateway_from_credentials() {
            Ok(gateway) => return gateway,
            Err(e) => {
                eprintln!("error: --real requires a DeepSeek API key");
                eprintln!(
                    "       target: {} (Windows Credential Manager, ADR-0006)",
                    orz_loop::gateway::credentials::AGENT_CREDENTIAL_TARGET
                );
                eprintln!("       {e}");
                std::process::exit(2);
            }
        }
    }
    if std::env::var("ORZ_FAKE_TOOL").is_ok() {
        Arc::new(
            FakeProvider::new(vec![
                ScriptedResponse::tool_calls(vec![
                    ToolCall {
                        name: "read_file".to_string(),
                        arguments: serde_json::json!({"target_file": "rust-toolchain.toml"}),
                        call_id: "call-1".to_string(),
                    },
                    ToolCall {
                        name: "bash".to_string(),
                        arguments: serde_json::json!({"command": "dir"}),
                        call_id: "call-2".to_string(),
                    },
                ]),
                ScriptedResponse::tool_calls(vec![
                    ToolCall {
                        name: "read_file".to_string(),
                        arguments: serde_json::json!({"target_file": "rust-toolchain.toml"}),
                        call_id: "call-1".to_string(),
                    },
                    ToolCall {
                        name: "bash".to_string(),
                        arguments: serde_json::json!({"command": "dir"}),
                        call_id: "call-2".to_string(),
                    },
                ]),
                ScriptedResponse::text(
                    "完成（fake 工具路径：read_file 已执行，bash 被权限门拒绝）。",
                ),
                ScriptedResponse::text(
                    "完成（fake 工具路径：read_file 已执行，bash 被权限门拒绝）。",
                ),
            ])
            // Streaming slice: pace the chunks so the TUI demo streams visibly
            // (tool-call rounds have no text → no chunks → no delay).
            .with_chunk_delay(std::time::Duration::from_millis(250)),
        )
    } else {
        Arc::new(FakeProvider::from_texts(vec![
            "(fake) 已收到请求。",
            "(fake) 已收到请求。",
            "(fake) 已收到请求。",
            "(fake) 已收到请求。",
        ]))
    }
}

/// Render the plan artifact's four sections for the plan-write counterexample
/// gate round (mechanical — no model-generated plan text yet).
fn render_plan_for_gate(artifact: &orz_assurance::plan::PlanArtifact) -> String {
    let mut out = String::new();
    for section in &artifact.sections {
        out.push_str(&format!("## {}\n{}\n\n", section.title, section.content_md));
    }
    out
}

/// Parse `-p <prompt>` or `--prompt <prompt>` from argv.
fn parse_prompt(args: &[String]) -> Result<String, String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "-p" || arg == "--prompt" {
            return args
                .get(i + 1)
                .cloned()
                .ok_or_else(|| format!("missing value after {arg}"));
        }
    }
    Err("no prompt provided".into())
}

/// Bootstrap a session and run one turn, returning the response and journal path.
///
/// P0-2/P1-1 (2026-08-08 stall guards): `wallclock` bounds the WHOLE run
/// (bootstrap excluded — it is local and fast) and the stall watchdog ends
/// the run when the process goes silent (no journal event, no model wire
/// frame, no tool execution) for its window. On either expiry the run
/// future is dropped and the caller records a graceful
/// `run_invalidated{status: wallclock|stall}` terminal continuing the
/// chain from the file — the harness's process-out hard kill (no terminal,
/// no journal) becomes a graceful end (terminal + journal). Both guards
/// are model-invisible (time pressure induces premature completion — the
/// plan's explicit exclusion).
async fn run(
    prompt: &str,
    wallclock: Option<Duration>,
    stall_timeout: Option<Duration>,
) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let run_id = format!("RUN-CLI-{}", timestamp_suffix());
    let cwd = std::env::current_dir()?;

    // Journals land in `{cwd}/.gsa/runs/{run_id}/`. Workspace trust is
    // enforced (fail-closed): the current directory must carry repo-local
    // trust config or be recorded in the trust store.
    let handle = bootstrap_session(
        &run_id,
        Some(cwd.clone()),
        orz_host::session::TrustPolicy::Enforce,
    )
    .await?;

    // P1-1: the heartbeat is threaded into the controller (journal events
    // + model rounds + tool execution stamp it) and the gateway (the
    // transport stamps it on every wire frame — reasoning deltas included,
    // which the controller never sees).
    let heartbeat = orz_loop::gateway::model::ActivityClock::new();
    let work = async {
        // Phase 3 wiring: real OrzHost (GrokBuild toolset + trust) behind
        // the IP6 permission bridge. Headless (`None` gateway): Read
        // auto-allows, Bash Ask → Deny.
        let host = build_cli_host(&handle, &run_id, &cwd)?;

        let controller = orz_loop::AgentLoopController::with_gateway(build_gateway())
            .with_snapshot_store(Some(handle.snapshot_store.clone()))
            // ACAF Slice 1 (ADR-0011 §4.4): optional signer-process client
            // (env-gated; unconfigured → unticketed control events, zero
            // behaviour change). Shadow mode by default; Slice 2
            // fail-closed (2026-08-13): `ORZ_ACAF_FAIL_CLOSED` flips the
            // switch (D-14/D-15/D-16 — an unconfigured fabric then refuses
            // to start at all).
            .with_acaf(build_acaf_client().await?)
            .with_acaf_fail_closed(acaf_fail_closed_enabled());
        let (response, _, _) = controller
            .run_turn_with_guards(
                &host,
                prompt,
                &handle.run_id,
                &handle.run_manifest_sha256,
                handle.next_sequence,
                handle.last_event_sha256.clone(),
                None,
                Some(&heartbeat),
                // GAP-INQUIRY-SPLIT: one-shot CLI runs carry no session
                // orientation state (the ACP server hosts sessions).
                // GAP-CONVERSATION-RESTORE: one-shot CLI runs carry no
                // session conversation either.
                None,
                None,
            )
            .await?;

        handle.journal.shutdown_async().await?;

        Ok::<_, Box<dyn std::error::Error>>((response, handle.journal_dir.join("events.jsonl")))
    };
    // Compose the guards: the stall watchdog wraps the work (dropping it on
    // silence); the wallclock wraps the whole thing (dropping it at the
    // deadline). Whichever fires first records its own terminal — the
    // already-terminated check in `record_guard_terminal` absorbs the
    // benign select race when the work completes in the same instant.
    let guarded = async {
        match stall_timeout {
            Some(timeout) => tokio::select! {
                r = work => r,
                _ = stall_fire(&heartbeat, timeout) => {
                    tracing::warn!(?timeout, "stall watchdog fired — no activity for the window");
                    let note = record_guard_terminal(&handle, "stall").await?;
                    handle.journal.shutdown_async().await?;
                    Ok::<_, Box<dyn std::error::Error>>((note, handle.journal_dir.join("events.jsonl")))
                }
            },
            None => work.await,
        }
    };
    match wallclock {
        Some(budget) => match tokio::time::timeout(budget, guarded).await {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(
                    ?budget,
                    "max-wallclock reached — recording run_invalidated terminal"
                );
                let note = record_guard_terminal(&handle, "wallclock").await?;
                handle.journal.shutdown_async().await?;
                Ok((note, handle.journal_dir.join("events.jsonl")))
            }
        },
        None => guarded.await,
    }
}

/// Build the Phase 3 CLI host: OrzHost + IP6 permission bridge with a
/// fail-closed dead gateway (headless — no ACP client to answer prompts).
/// `--allow-write` (ORZ_ALLOW_WRITE, harness opt-in) switches the bridge to
/// the Benchmark policy: reads + local file edits auto-allow, bash/network
/// still fail closed.
///
/// D-9 (FIX_PLAN 2026-08-06): `ORZ_TEST_RUNNER` (harness opt-in) injects a
/// fixed test command — the `run_tests` tool appears in the model's tool
/// declarations and runs the host-owned command (Aider-model feedback loop;
/// test files stay hidden).
fn build_cli_host(
    handle: &SessionHandle,
    session_id: &str,
    cwd: &Path,
) -> Result<orz_host::OrzHost, String> {
    let policy = if std::env::var("ORZ_ALLOW_WRITE").is_ok() {
        orz_host::permission::PermissionPolicy::Benchmark
    } else {
        orz_host::permission::PermissionPolicy::Interactive
    };
    let test_runner = std::env::var("ORZ_TEST_RUNNER").ok().map(|cmd| {
        // argv-style: split on spaces (the harness builds the command; the
        // test path is injected as a single token).
        // RT-002 (2026-08-11): `ORZ_TEST_RUNNER_ENV` — a JSON object of
        // explicit environment allowlist entries for the test process
        // (PYTHONPATH, venv activation, sitecustomize shim, …). The test
        // command runs env_clear()ed with only the platform allowlist plus
        // these entries; host secrets are never inherited.
        let env = std::env::var("ORZ_TEST_RUNNER_ENV")
            .ok()
            .map(|raw| {
                serde_json::from_str::<std::collections::BTreeMap<String, String>>(&raw)
                    .unwrap_or_else(|e| {
                        // P2 (review 2026-08-11): a typo'd harness env JSON must
                        // not silently drop PYTHONPATH/venv entries — the
                        // failure mode ("module not found" in a broken test
                        // environment) is otherwise invisible. Fail-safe: env
                        // stays empty (env_clear still applies), but the harness
                        // sees the parse error in the log.
                        tracing::warn!(raw, "ORZ_TEST_RUNNER_ENV parse failed: {e}");
                        std::collections::BTreeMap::new()
                    })
            })
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<(String, String)>>();
        orz_loop::host::TestRunner {
            command: cmd.split_whitespace().map(str::to_string).collect(),
            timeout: None,
            env,
        }
    });
    // P0-1 (2026-08-08 review D2-3): per-tool timeout escape hatch for the
    // interactive `-p` path — `ORZ_TOOL_TIMEOUT_SECS` (0 = unbounded).
    // Interactive users running legitimately long commands (10min builds)
    // can raise or disable the 5min default without a rebuild; the TUI/
    // stdio paths keep the host default (documented limitation).
    let tool_timeout = std::env::var("ORZ_TOOL_TIMEOUT_SECS")
        .ok()
        .map(|s| s.trim().parse::<u64>())
        .transpose()
        .map_err(|_| "ORZ_TOOL_TIMEOUT_SECS must be a number of seconds".to_string())?
        .filter(|&s| s > 0)
        .map(Duration::from_secs);
    let mut host = orz_host::OrzHost::with_bridge_and_hub_policy(
        session_id,
        handle.journal.clone(),
        cwd,
        handle.workspace_trust,
        None,
        None,
        policy,
    )?
    // P1 permit keystore — the session's DPAPI-backed signer.
    .with_permit_signer(handle.permit_signer.clone())
    // D-9: fixed test-runner command (harness feedback loop).
    .with_test_runner(test_runner);
    if let Some(timeout) = tool_timeout {
        host = host.with_tool_timeout(timeout);
    }
    Ok(host)
}

/// Short timestamp-based suffix for the run ID (no uuid dep in orz-bin yet).
fn timestamp_suffix() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:08x}", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_loop::gateway::fake::FakeProvider;
    use orz_loop::gateway::model::{GatewayError, ModelRequest, ModelResponse};

    fn test_dir() -> std::path::PathBuf {
        // 2026-08-08 stall guards: the original name keyed on
        // `pid + current-second` only — two tests started in the same
        // second COLLIDED on one directory (the second bootstrap's
        // remove_dir_all wiped the first's journal mid-flight; observed as
        // a flaky "journal file not found"). The per-call counter makes
        // every test directory unique.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-bin-test-{}-{:08x}-{n:04x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// P2-1 regression: a plan-phase error (the counterexample gate round
    /// fails here) must not leave the journal without a terminal event — the
    /// caller records RunFailed continuing the chain, and the journal replays
    /// as valid with a run_failed terminal.
    #[tokio::test]
    async fn plan_phase_failure_writes_terminal_run_failed() {
        let dir = test_dir();
        let handle = orz_host::session::bootstrap_session(
            "RUN-PLAN-FAIL",
            Some(dir.clone()),
            orz_host::session::TrustPolicy::Skip,
        )
        .await
        .unwrap();

        let mut seq = handle.next_sequence;
        let mut prev_hash = handle.last_event_sha256.clone();
        // Empty script — the gate round exhausts on the first call.
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(Vec::new()));
        let heartbeat = orz_loop::gateway::model::ActivityClock::new();
        let err = run_plan_phase(
            &handle,
            &mut seq,
            &mut prev_hash,
            "RUN-PLAN-FAIL",
            "hi",
            &gateway,
            &heartbeat,
            1,
        )
        .await;
        assert!(err.is_err(), "gate round must fail on an empty script");
        record_plan_failure(&handle, seq, prev_hash.clone(), err.unwrap_err().as_str()).await;
        handle.journal.shutdown_async().await.unwrap();

        let replay = orz_assurance::replay_journal(
            &handle.journal_dir.join("events.jsonl"),
            Some("RUN-PLAN-FAIL"),
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_failed"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-2 (2026-08-08 stall guards): the budget parser is fail-closed —
    /// `None`/`0` = unbounded (default), a non-numeric value is an error
    /// (a typo must never silently disable the guard).
    #[test]
    fn parse_max_wallclock_is_fail_closed() {
        assert_eq!(parse_max_wallclock(None).unwrap(), None);
        assert_eq!(parse_max_wallclock(Some("0".into())).unwrap(), None);
        assert_eq!(
            parse_max_wallclock(Some("90".into())).unwrap(),
            Some(std::time::Duration::from_secs(90))
        );
        assert_eq!(
            parse_max_wallclock(Some("  30 ".into())).unwrap(),
            Some(std::time::Duration::from_secs(30))
        );
        assert!(parse_max_wallclock(Some("abc".into())).is_err());
        assert!(parse_max_wallclock(Some("".into())).is_err());
    }

    /// P0-2 (2026-08-08 stall guards): the wallclock terminal write
    /// continues the hash chain from the journal FILE and ends the run with
    /// `run_invalidated{status: wallclock}` — the journal replays valid.
    #[tokio::test]
    async fn wallclock_terminal_records_run_invalidated_and_replays_valid() {
        let dir = test_dir();
        let handle = orz_host::session::bootstrap_session(
            "RUN-WALLCLOCK",
            Some(dir.clone()),
            orz_host::session::TrustPolicy::Skip,
        )
        .await
        .unwrap();

        let note = record_guard_terminal(&handle, "wallclock").await.unwrap();
        assert!(note.contains("wallclock"), "{note}");
        handle.journal.shutdown_async().await.unwrap();

        let replay = orz_assurance::replay_journal(
            &handle.journal_dir.join("events.jsonl"),
            Some("RUN-WALLCLOCK"),
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_invalidated"));
        let content = std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
        let last: serde_json::Value =
            serde_json::from_str(content.lines().last().unwrap()).unwrap();
        assert_eq!(last["payload"]["status"], "wallclock");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-2 (2026-08-08 stall guards): the REAL wallclock semantics — a run
    /// future dropped mid-model-round (the budget expired while the gateway
    /// was streaming) must still end with a valid chain: the dropped
    /// controller's queued events are flushed, the chain position is
    /// recovered from the file, and the `run_invalidated` terminal is
    /// appended. This is what turns the harness's "killed, no journal" into
    /// "graceful end, journal intact".
    #[tokio::test]
    async fn wallclock_dropped_run_recovers_chain_and_terminates() {
        let dir = test_dir();
        let handle = orz_host::session::bootstrap_session(
            "RUN-WALLCLOCK-DROP",
            Some(dir.clone()),
            orz_host::session::TrustPolicy::Skip,
        )
        .await
        .unwrap();
        let host = orz_host::OrzHost::new(
            handle.journal.clone(),
            &dir,
            orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
        )
        .expect("host");
        // ~80 chars each → ~20 chunks of the 4-char chunker × 150ms ≈ 3s
        // of streaming; the budget fires mid-stream. 400ms is long enough
        // that the run's first journal events have landed even under
        // parallel-test CPU contention (the 100ms first version was flaky:
        // under load the budget could expire before the first event was
        // written), short enough that the stream is nowhere near done.
        let gateway: Arc<dyn ModelGateway> = Arc::new(
            FakeProvider::from_texts(vec![
                "slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow",
                "slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow slow",
            ])
            .with_chunk_delay(std::time::Duration::from_millis(150)),
        );
        let controller = orz_loop::AgentLoopController::with_gateway(gateway);
        let fut = controller.run_turn(
            &host,
            "wallclock 测试",
            &handle.run_id,
            &handle.run_manifest_sha256,
            handle.next_sequence,
            handle.last_event_sha256.clone(),
            None,
            None,
        );
        let timed = tokio::time::timeout(std::time::Duration::from_millis(400), fut).await;
        assert!(timed.is_err(), "budget must expire mid-run: {timed:?}");

        let note = record_guard_terminal(&handle, "wallclock").await.unwrap();
        assert!(note.contains("wallclock"), "{note}");
        handle.journal.shutdown_async().await.unwrap();

        let replay = orz_assurance::replay_journal(
            &handle.journal_dir.join("events.jsonl"),
            Some("RUN-WALLCLOCK-DROP"),
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_invalidated"));
        let content = std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
        let last: serde_json::Value =
            serde_json::from_str(content.lines().last().unwrap()).unwrap();
        assert_eq!(last["payload"]["status"], "wallclock");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P1-1 (2026-08-08 stall guards): a run that goes SILENT (no journal
    /// event, no model wire frame — the gateway's future sleeps forever)
    /// is ended by the stall watchdog with `run_invalidated{status: stall}`
    /// and the journal replays valid.
    #[tokio::test]
    async fn stall_watchdog_terminates_silent_run() {
        let dir = test_dir();
        let handle = orz_host::session::bootstrap_session(
            "RUN-STALL",
            Some(dir.clone()),
            orz_host::session::TrustPolicy::Skip,
        )
        .await
        .unwrap();
        let host = orz_host::OrzHost::new(
            handle.journal.clone(),
            &dir,
            orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
        )
        .expect("host");

        // A gateway that NEVER returns and NEVER stamps the heartbeat — a
        // hung model request with no wire frames.
        struct SilentGateway;
        #[async_trait::async_trait]
        impl ModelGateway for SilentGateway {
            async fn generate(
                &self,
                _request: ModelRequest,
            ) -> Result<ModelResponse, GatewayError> {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok(ModelResponse::text_response("never"))
            }
            async fn generate_stream(
                &self,
                _request: ModelRequest,
                _cancel: Option<&tokio_util::sync::CancellationToken>,
                _heartbeat: Option<&orz_loop::gateway::model::ActivityClock>,
                _on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
            ) -> Result<ModelResponse, GatewayError> {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok(ModelResponse::text_response("never"))
            }
        }
        let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(SilentGateway));
        let heartbeat = orz_loop::gateway::model::ActivityClock::new();
        let fut = controller.run_turn_with_guards(
            &host,
            "stall 测试",
            &handle.run_id,
            &handle.run_manifest_sha256,
            handle.next_sequence,
            handle.last_event_sha256.clone(),
            None,
            Some(&heartbeat),
            None,
            None,
        );
        let mut stalled = false;
        let outcome: Result<(), String> = tokio::select! {
            r = fut => r.map(|_| ()).map_err(|e| e.to_string()),
            _ = stall_fire(&heartbeat, std::time::Duration::from_millis(400)) => {
                stalled = true;
                record_guard_terminal(&handle, "stall").await.map(|_| ())
            }
        };
        assert!(stalled, "stall watchdog must fire: {outcome:?}");
        outcome.unwrap();
        handle.journal.shutdown_async().await.unwrap();

        let replay = orz_assurance::replay_journal(
            &handle.journal_dir.join("events.jsonl"),
            Some("RUN-STALL"),
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_invalidated"));
        let content = std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
        let last: serde_json::Value =
            serde_json::from_str(content.lines().last().unwrap()).unwrap();
        assert_eq!(last["payload"]["status"], "stall");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P1-1 (2026-08-08 stall guards): ACTIVE work must NOT trip the stall
    /// watchdog — the fake provider stamps the heartbeat per delivered
    /// chunk (mirroring the real transport's per-frame stamp), so a slow
    /// but steadily-streaming run completes normally even though its wall
    /// time exceeds the watchdog window.
    #[tokio::test]
    async fn stall_watchdog_does_not_fire_during_active_stream() {
        let dir = test_dir();
        let handle = orz_host::session::bootstrap_session(
            "RUN-STALL-ACTIVE",
            Some(dir.clone()),
            orz_host::session::TrustPolicy::Skip,
        )
        .await
        .unwrap();
        let host = orz_host::OrzHost::new(
            handle.journal.clone(),
            &dir,
            orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
        )
        .expect("host");
        // ~140 chars → 35 chunks × 40ms ≈ 1.4s of streaming per round —
        // far longer than any gap the watchdog sees, but every chunk
        // stamps the heartbeat so it stays asleep. (The window must clear
        // the fsync clusters between rounds and after the last chunk —
        // each journal event is `sync_all`-ed and a cluster can reach
        // ~1.5s under parallel-test load; the 400ms and 1.2s versions
        // false-fired on the between-round and final-tail clusters
        // respectively. The 2.5s window clears both while staying well
        // below the total run time.)
        // The two rounds MUST be textually distinct — identical content
        // would legitimately trip the runtime content-stagnation guard
        // (ngram repetition) and end the run with run_invalidated, which
        // would make this test assert the wrong terminal.
        let gateway: Arc<dyn ModelGateway> = Arc::new(
            FakeProvider::from_texts(vec![
                "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu",
                "the second and final answer is entirely different prose without any repeated n-grams whatsoever",
            ])
            .with_chunk_delay(std::time::Duration::from_millis(40)),
        );
        let controller = orz_loop::AgentLoopController::with_gateway(gateway);
        let heartbeat = orz_loop::gateway::model::ActivityClock::new();
        let fut = controller.run_turn_with_guards(
            &host,
            "active 测试",
            &handle.run_id,
            &handle.run_manifest_sha256,
            handle.next_sequence,
            handle.last_event_sha256.clone(),
            None,
            Some(&heartbeat),
            None,
            None,
        );
        let result = tokio::select! {
            r = fut => r.map(|_| ()).map_err(|e| e.to_string()),
            _ = stall_fire(&heartbeat, std::time::Duration::from_millis(2500)) => {
                Err("stall watchdog fired on an ACTIVE stream".to_string())
            }
        };
        result.expect("active stream must complete normally");
        handle.journal.shutdown_async().await.unwrap();
        let replay = orz_assurance::replay_journal(
            &handle.journal_dir.join("events.jsonl"),
            Some("RUN-STALL-ACTIVE"),
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Phase 3 #7 conformance capture — deterministic REAL run journals for the
/// Rust↔Python cross-validation suite.
///
/// Each `#[ignore]`d test runs one scenario in-process (fake provider, no
/// network, `TrustPolicy::Skip`) and stages `events.jsonl` under
/// `target/conformance-journals/<scenario>/`, self-checking the chain with
/// `replay_journal` first. The staged files are committed to the main repo
/// as `runtime/fixtures/run-event-v0.1/journals/` (dev-time copy, see the
/// slice audit doc) — CI stays pure Python.
///
/// Run: `cargo test -p orz-bin -- --ignored conformance_capture --nocapture
/// --test-threads=1`
#[cfg(test)]
mod conformance_capture {
    use super::*;
    use agent_client_protocol as acp;
    use async_trait::async_trait;
    use orz_host::acp_server::{AcpError, AcpServer};
    use orz_host::session::TrustPolicy;
    use orz_loop::controller::AgentLoopError;

    /// local_browser (2026-08-10): a deterministic fake browser lane — the
    /// conformance scenario must never touch a real browser (fake-provider
    /// discipline); the real lane is covered by the env-gated e2e test.
    struct StubBrowserSession;

    #[async_trait]
    impl orz_host::local_browser::BrowserSession for StubBrowserSession {
        async fn read_page(
            &self,
            url: &str,
        ) -> Result<orz_host::local_browser::PageReadOutcome, orz_host::local_browser::CdpError>
        {
            Ok(orz_host::local_browser::PageReadOutcome {
                final_url: url.to_string(),
                title: "Example".to_string(),
                text: "Example Domain — This domain is for use in illustrative examples."
                    .to_string(),
            })
        }

        async fn download_or_read(
            &self,
            url: &str,
            _download_dir: &std::path::Path,
        ) -> Result<
            orz_host::local_browser::BrowserDownloadOutcome,
            orz_host::local_browser::CdpError,
        > {
            // Conformance stub: never a real download — read the page
            // instead (the stub's read_page is the deterministic path).
            let page = self.read_page(url).await?;
            Ok(orz_host::local_browser::BrowserDownloadOutcome::Page(page))
        }

        fn ready(&self) -> bool {
            true
        }

        async fn shutdown(&self) {}
    }

    /// `{workspace}/target/conformance-journals` — hermetic (gitignored
    /// target/), one dir per scenario; `worktrees/<scenario>/` holds each
    /// scenario's cwd so no capture writes outside the workspace.
    fn staging_root() -> PathBuf {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            manifest.join("../../Cargo.toml").is_file(),
            "workspace marker missing — capture tests must run inside the orz workspace"
        );
        manifest.join("../../target/conformance-journals")
    }

    /// Wipe + recreate `<scenario>/` (re-runnable) and return its path.
    fn scenario_staging(name: &str) -> PathBuf {
        let dir = staging_root().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Wipe + recreate the scenario cwd.
    fn worktree(name: &str) -> PathBuf {
        let dir = staging_root().join("worktrees").join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Copy `{journal_dir}/events.jsonl` into the scenario staging dir.
    fn copy_journal(journal_dir: &Path, name: &str) {
        let to = scenario_staging(name).join("events.jsonl");
        std::fs::copy(journal_dir.join("events.jsonl"), &to).unwrap();
        println!("  captured {name}: {}", to.display());
    }

    /// Self-check with the Rust verifier: chain valid + expected terminal +
    /// EXACT event-type sequence (the staleness signal — a changed loop
    /// structure fails loudly at re-capture time).
    fn verify(journal: &Path, name: &str, expected: &[&str], terminal: &str) {
        let replay = orz_assurance::replay_journal(journal, None, None, true);
        assert!(replay.valid, "{name}: replay invalid: {:?}", replay.errors);
        assert_eq!(
            replay.terminal_event.as_deref(),
            Some(terminal),
            "{name}: unexpected terminal"
        );
        let content = std::fs::read_to_string(journal).unwrap();
        let types: Vec<String> = content
            .lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
            .map(|v| {
                v.get("event_type")
                    .and_then(|t| t.as_str())
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(
            types,
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "{name}: event sequence drifted"
        );
        println!(
            "  {name}: {} events, terminal={terminal}, valid",
            types.len()
        );
    }

    /// 1. plain — scripted text-only turn through the real CLI host.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_plain_run() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("plain-run");
                let run_id = "RUN-PLAIN-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let controller =
                    orz_loop::AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
                        ScriptedResponse::text("X"),
                        ScriptedResponse::text("X"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()));
                controller
                    .run_turn(
                        &host,
                        "hello",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "plain-run",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                copy_journal(&handle.journal_dir, "plain-run");
            })
            .await
    }

    /// 2. tool + snapshot — `search_replace` mutates a.txt through an
    /// allow-once permission grant; the run journal carries the
    /// snapshot_created hash.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_tool_snapshot_run() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("tool-snapshot-run");
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
                let mut client =
                    orz_tui::acp_client::connect_inprocess(server.clone(), TrustPolicy::Skip);
                client.start_session(base.to_path_buf()).await.unwrap();
                client.prompt_count += 1;
                let seq = client.prompt_count;
                let session_id = client.session_id.clone().unwrap();
                client.spawn_prompt(session_id, "把 v1 改成 v2".to_string(), seq);

                let completed = loop {
                    match client.msg_rx.recv().await.unwrap() {
                        orz_tui::acp_client::ClientMsg::PermissionRequest { request, respond } => {
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
                        orz_tui::acp_client::ClientMsg::PromptCompleted { result, .. } => {
                            break result;
                        }
                        orz_tui::acp_client::ClientMsg::SessionNotification { .. } => {}
                    }
                };
                assert!(completed.is_ok(), "prompt failed: {completed:?}");
                assert_eq!(std::fs::read_to_string(base.join("a.txt")).unwrap(), "v2");

                let runs_dir = base.join(".gsa").join("runs");
                let journal_dir = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("RUN-"))
                    .expect("RUN- journal dir");
                verify(
                    &journal_dir.join("events.jsonl"),
                    "tool-snapshot-run",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "permission_requested",
                        "permission_decision",
                        "snapshot_created",
                        "tool_started",
                        "tool_completed",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                copy_journal(&journal_dir, "tool-snapshot-run");
            })
            .await
    }

    /// 3. plan — plan-write gate round + plan_proposed/plan_approved, then
    /// the execution turn under the approved plan.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_plan_run() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("plan-run");
                let run_id = "RUN-PLAN-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let mut seq = handle.next_sequence;
                let mut prev_hash = handle.last_event_sha256.clone();
                let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("X"),
                    ScriptedResponse::text("X"),
                    ScriptedResponse::text("X"),
                    ScriptedResponse::text("X"),
                ]));
                let heartbeat = orz_loop::gateway::model::ActivityClock::new();
                run_plan_phase(
                    &handle,
                    &mut seq,
                    &mut prev_hash,
                    run_id,
                    "hi",
                    &gateway,
                    &heartbeat,
                    1,
                )
                .await
                .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let controller = orz_loop::AgentLoopController::with_gateway(gateway)
                    .with_snapshot_store(Some(handle.snapshot_store.clone()));
                controller
                    .run_turn(
                        &host,
                        "hi",
                        run_id,
                        &handle.run_manifest_sha256,
                        seq,
                        prev_hash,
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "plan-run",
                    &[
                        "run_preflight",
                        "counterexample_gate",
                        "plan_proposed",
                        "plan_approved",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                copy_journal(&handle.journal_dir, "plan-run");
            })
            .await
    }

    /// 4. cancelled — the host's cooperative cancel turns the in-flight
    /// prompt into a `run_cancelled` terminal.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_cancelled_run() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("cancelled-run");
                let provider = Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                    ])
                    .with_chunk_delay(std::time::Duration::from_millis(100)),
                );
                let server = Arc::new(AcpServer::with_gateway(provider.clone()));
                server
                    .handle_session_new("sess-cancel", Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();

                let srv = server.clone();
                let prompt = tokio::task::spawn_local(async move {
                    srv.handle_session_prompt("sess-cancel", "hello").await
                });
                // Event-based wait: cancel once the first model round is
                // actually underway (design review D2 — no timing race on
                // slow machines). The run terminates at the after-round
                // checkpoint.
                let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    if !provider.received_requests().is_empty() {
                        break;
                    }
                    assert!(
                        tokio::time::Instant::now() < deadline,
                        "provider never received a request"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                assert!(server.cancel_current_run("sess-cancel"));

                let result = prompt.await.unwrap();
                assert!(matches!(
                    result,
                    Err(AcpError::AgentLoop(AgentLoopError::Cancelled))
                ));

                let runs_dir = base.join(".gsa").join("runs");
                let journal_dir = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("RUN-"))
                    .expect("RUN- journal dir");
                verify(
                    &journal_dir.join("events.jsonl"),
                    "cancelled-run",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "run_cancelled",
                    ],
                    "run_cancelled",
                );
                copy_journal(&journal_dir, "cancelled-run");
            })
            .await
    }

    /// 5. failed — the fake script exhausts mid-turn; the loop records a
    /// `run_failed` terminal on the error path.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_failed_run() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("failed-run");
                let run_id = "RUN-FAILED-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(
                    FakeProvider::new(Vec::new()),
                ))
                .with_snapshot_store(Some(handle.snapshot_store.clone()));
                let result = controller
                    .run_turn(
                        &host,
                        "hi",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await;
                assert!(result.is_err(), "empty script must fail the turn");
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "failed-run",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "run_failed",
                    ],
                    "run_failed",
                );
                copy_journal(&handle.journal_dir, "failed-run");
            })
            .await
    }

    /// 6. restore — a tracked snapshot is restored through the host; the
    /// RST- run journal ends run_finished (preflight → snapshot_restored →
    /// finished).
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_restore_run() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("restore-run");
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
                let mut client =
                    orz_tui::acp_client::connect_inprocess(server.clone(), TrustPolicy::Skip);
                client.start_session(base.to_path_buf()).await.unwrap();
                let session_id = client.session_id.clone().unwrap();
                let report = server
                    .restore_snapshot(&session_id, &record.snapshot_hash, None)
                    .await
                    .unwrap();
                let restored = report
                    .get("restored")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                assert_eq!(restored, 1, "restore report: {report}");

                let session8: String = session_id.chars().take(8).collect();
                let journal_dir = base
                    .join(".gsa")
                    .join("runs")
                    .join(format!("RST-{session8}-0"));
                verify(
                    &journal_dir.join("events.jsonl"),
                    "restore-run",
                    &["run_preflight", "snapshot_restored", "run_finished"],
                    "run_finished",
                );
                copy_journal(&journal_dir, "restore-run");
            })
            .await
    }

    /// 7. orientation-fire — GAP-INQUIRY-SPLIT: seven retrieval tool rounds
    /// cross the session-level 7-round threshold; the orientation fires in
    /// the post-tool-batch gap of round 7 (the injected block is answered by
    /// the next generate; the counterexample gate then separates the final
    /// answer). Also captures the mechanical `information_sufficiency_assessment`
    /// the subagent path produces (v0.2 track).
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_orientation_fire_run() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("orientation-fire-run");
                let run_id = "RUN-ORIENT-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                // M4 (GAP-SUBAGENT-RUNTIME 2026-08-10): §4.4 — a retrieval
                // while the activation awaits disposition is REFUSED, so the
                // crossing scenario interleaves a disposition after every
                // assessment (4 × continue + 1 × close): the main alternates
                // retrieve → disposition rounds, the subagent consumes one
                // text per retrieve. 10 tool rounds + 2 final rounds = 12
                // completed model rounds — the 7-round threshold crosses
                // exactly once (on the 4th retrieve's gap).
                let mut script = Vec::new();
                for i in 0..5 {
                    script.push(ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "retrieve_project_docs".to_string(),
                        arguments: serde_json::json!({"query": format!("查询 {i}")}),
                        call_id: format!("call-{i}"),
                    }]));
                    script.push(ScriptedResponse::text(format!(
                        "[DOC] doc-{i}.md\n检索结果 {i}"
                    )));
                    if i < 4 {
                        script.push(ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "retrieval_disposition".to_string(),
                            arguments: serde_json::json!({
                                "role": "internal_retrieval",
                                "decision": "continue",
                                "requirement_delta": format!("补充 doc-{} 的线索", i + 1),
                            }),
                            call_id: format!("call-d{i}"),
                        }]));
                    } else {
                        script.push(ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "retrieval_disposition".to_string(),
                            arguments: serde_json::json!({
                                "role": "internal_retrieval",
                                "decision": "close",
                            }),
                            call_id: "call-d4".to_string(),
                        }]));
                    }
                }
                script.push(ScriptedResponse::text(
                    "当前任务定位：处理查询批次；下一步：汇总结果",
                ));
                script.push(ScriptedResponse::text("最终汇总完成。"));
                let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(
                    FakeProvider::new(script),
                ))
                .with_snapshot_store(Some(handle.snapshot_store.clone()))
                // GAP-RETRIEVAL-TOOLS (2026-08-10): the scenario dispatches
                // retrieval — run under an explicit framework_fallback mode
                // with a fully available capability (the bare default is
                // mode=off, which would refuse every dispatch).
                .with_retrieval_mode(
                    orz_loop::controller::RetrievalMode::FrameworkFallback,
                    orz_loop::controller::RetrievalCapability::Available,
                    false,
                    None,
                    None,
                );
                // This scenario is the 7-round-crossing proof — the session
                // orientation state MUST be threaded in (a one-shot CLI run
                // would pass None and never fire).
                let mut orientation = orz_loop::orientation::OrientationSessionState::new(run_id);
                controller
                    .run_turn(
                        &host,
                        "跑 7 轮检索后汇总",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        Some(&mut orientation),
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();

                let mut expected = vec![
                    "run_preflight",
                    "tool_availability_check",
                    "run_started",
                    "prompt_submitted",
                ];
                // Per retrieval iteration: the shared-loop dispatch (subagent
                // model round + its stagnation guard inside the parent's
                // wrapper) + the assessment + the parent's disposition round
                // (control tool — journaled with its own ToolStarted/
                // ToolCompleted). The orientation crosses the 7-round
                // threshold on the 4th retrieve's post-tool-batch gap (the
                // 7th completed main round) — it fires between that
                // iteration's assessment and its disposition round. The last
                // iteration closes (accepted close → close record).
                for i in 0..5 {
                    expected.extend([
                        "model_output",
                        "tool_started",
                        "model_output",
                        "runtime_stagnation_guard",
                        "tool_completed",
                        // GAP-RETRIEVAL-TOOLS (2026-08-10): the committed
                        // structured result precedes the assessment.
                        "retrieval_result_committed",
                        "information_sufficiency_assessment",
                    ]);
                    if i == 3 {
                        expected.push("orientation_checkpoint");
                    }
                    // FUS-TOOL-PROBE P0-A-2 审查复核（2026-08-13）：评估落地后
                    // has_live_activation 置真（未决 pending assessment），
                    // retrieval_disposition 探针翻转 → tool_availability_check。
                    expected.push("tool_availability_check");
                    // The disposition event (and the close record for an
                    // accepted close) are the control call's products — they
                    // land between the tool's start and completion.
                    expected.extend([
                        "model_output",
                        "tool_started",
                        "retrieval_parent_disposition",
                    ]);
                    if i == 4 {
                        expected.push("retrieval_close_record");
                    }
                    expected.push("tool_completed");
                    // FUS-TOOL-PROBE P0-A-2 审查复核（2026-08-13）：disposition
                    // 消费后 pending assessment 清除，探针翻转 → 事件。
                    expected.push("tool_availability_check");
                }
                expected.extend([
                    "model_output",
                    "counterexample_gate",
                    "model_output",
                    "runtime_stagnation_guard",
                    "run_finished",
                ]);
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "orientation-fire-run",
                    &expected,
                    "run_finished",
                );

                // The fired checkpoint carries the v0.2 payload shape —
                // inquiry_family=neutral + inquiry_kind const + the 7-count.
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                let orientation_line = content
                    .lines()
                    .find(|l| l.contains("\"orientation_checkpoint\""))
                    .expect("orientation event present");
                let payload: serde_json::Value = serde_json::from_str(orientation_line).unwrap();
                let p = &payload["payload"];
                assert_eq!(p["inquiry_family"], "neutral");
                assert_eq!(p["inquiry_kind"], "orientation_checkpoint");
                assert_eq!(p["agent_role"], "main");
                assert_eq!(p["trigger"], "completed_turns_interval");
                assert_eq!(p["completed_turns_since_orientation"], 7);
                assert_eq!(p["injection_position"], "post_tool_batch_gap");
                assert!(
                    p["message_block"]
                        .as_str()
                        .unwrap()
                        .starts_with("[ORIENTATION")
                );
                // The mechanical assessment snapshots the PER-ITERATION
                // structured result: each iteration's [DOC] line is one
                // metadata-grade ledger entry (GAP-RETRIEVAL-TOOLS — the
                // result is per-revision; the blackboard section still
                // accumulates).
                let assessment_lines: Vec<&str> = content
                    .lines()
                    .filter(|l| l.contains("\"information_sufficiency_assessment\""))
                    .collect();
                assert_eq!(assessment_lines.len(), 5);
                let last: serde_json::Value = serde_json::from_str(assessment_lines[4]).unwrap();
                assert_eq!(last["payload"]["status"], "indeterminate");
                assert_eq!(last["payload"]["source_counts"]["total"], 1);
                assert_eq!(last["payload"]["source_counts"]["metadata_only"], 1);
                assert_eq!(
                    last["payload"]["reason_codes"][0],
                    "no_mechanical_coverage_requirement"
                );

                copy_journal(&handle.journal_dir, "orientation-fire-run");
            })
            .await
    }

    /// 8. mode-off refusal — GAP-RETRIEVAL-TOOLS: the default mode=off
    /// refuses a scripted retrieval dispatch with the explicit
    /// `retrieval_mode_off` error (ToolCompleted alone — no ToolStarted:
    /// the verifier's mode rule forbids any dispatch after a transition to
    /// off); the model's declaration projection hides the retrieval family.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_mode_off_refusal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("mode-off-refusal");
                let run_id = "RUN-MODEOFF-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let controller =
                    orz_loop::AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "retrieve_project_docs".to_string(),
                            arguments: serde_json::json!({"query": "x"}),
                            call_id: "call-1".to_string(),
                        }]),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()));
                controller
                    .run_turn(
                        &host,
                        "查文档",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "mode-off-refusal",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "tool_completed",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                let refused: serde_json::Value = content
                    .lines()
                    .find(|l| l.contains("\"tool_completed\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .unwrap();
                assert_eq!(refused["payload"]["error"], "retrieval_mode_off");
                assert_eq!(refused["payload"]["target"], "internal_retrieval");
                copy_journal(&handle.journal_dir, "mode-off-refusal");
            })
            .await
    }

    /// 9. local-browser capability — GAP-RETRIEVAL-TOOLS: mode=local_browser
    /// with an unsupported capability fails every retrieval dispatch
    /// explicitly (ToolStarted → ToolCompleted(error,
    /// retrieval_capability_unavailable)) — no silent degradation to the
    /// framework tools.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_local_browser_capability_error() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("local-browser-capability");
                let run_id = "RUN-LBROW-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let controller =
                    orz_loop::AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "web_search".to_string(),
                            arguments: serde_json::json!({"query": "x"}),
                            call_id: "call-1".to_string(),
                        }]),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()))
                    .with_retrieval_mode(
                        orz_loop::controller::RetrievalMode::LocalBrowser,
                        orz_loop::controller::RetrievalCapability::Unsupported(
                            "local_browser_automation_not_implemented".to_string(),
                        ),
                        true,
                        None,
                        None,
                    );
                controller
                    .run_turn(
                        &host,
                        "查资料",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "local-browser-capability",
                    &[
                        "run_preflight",
                        // Bootstrap transition (off → local_browser) journals
                        // before the availability gate.
                        "retrieval_mode_transition",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "tool_started",
                        "tool_completed",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                let refused: serde_json::Value = content
                    .lines()
                    .find(|l| l.contains("\"retrieval_capability_unavailable\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .unwrap();
                assert_eq!(refused["payload"]["status"], "error");
                copy_journal(&handle.journal_dir, "local-browser-capability");
            })
            .await
    }

    /// 10. local-browser read — local_browser (2026-08-10): mode=
    /// local_browser with an AVAILABLE capability runs the real host
    /// `browser_read` tool (fake lane) inside the external retrieval
    /// subagent; the committed structured result carries REAL full-text
    /// web_page evidence and the transition records capability_status
    /// = available.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_local_browser_read() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("local-browser-read");
                let run_id = "RUN-LBROW-READ";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base)
                    .unwrap()
                    .with_browser_session(Arc::new(StubBrowserSession));
                let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "web_search".to_string(),
                            arguments: serde_json::json!({"query": "example domain"}),
                            call_id: "call-1".to_string(),
                        }]),
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "browser_read".to_string(),
                            arguments: serde_json::json!({"url": "https://example.com/"}),
                            call_id: "call-b1".to_string(),
                        }]),
                        ScriptedResponse::text(concat!(
                            "[DOC] https://example.com/ 检索完成\n",
                            "[RESULT_JSON]",
                            r#"{"sections":[{"section_title":"Example","content":"Example Domain","source_ids":["SRC-001"],"claim_strength":"observed"}],"claims":[]}"#,
                            "[/RESULT_JSON]",
                        )),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ]),
                ))
                .with_snapshot_store(Some(handle.snapshot_store.clone()))
                .with_retrieval_mode(
                    orz_loop::controller::RetrievalMode::LocalBrowser,
                    orz_loop::controller::RetrievalCapability::Available,
                    true,
                    None,
                    None,
                );
                controller
                    .run_turn(
                        &host,
                        "用浏览器读 example.com",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "local-browser-read",
                    &[
                        "run_preflight",
                        // Bootstrap transition (off → local_browser) with
                        // capability_status=available.
                        "retrieval_mode_transition",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "tool_started",
                        // Subagent round → the host browser_read tool
                        // (Interactive bridge auto-allow, then execution).
                        "model_output",
                        "permission_requested",
                        "permission_decision",
                        "tool_started",
                        "tool_completed",
                        // The subagent's answer round (result formation).
                        "model_output",
                        "runtime_stagnation_guard",
                        "tool_completed",
                        "retrieval_result_committed",
                        "information_sufficiency_assessment",
                        // FUS-TOOL-PROBE P0-A-2 审查复核（2026-08-13）：
                        // has_live_activation 收紧为"未决 pending assessment"——
                        // 评估落地后 retrieval_disposition 探针翻转，触发
                        // tool_availability_check。
                        "tool_availability_check",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                let transition: serde_json::Value = content
                    .lines()
                    .find(|l| l.contains("\"retrieval_mode_transition\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .unwrap();
                assert_eq!(transition["payload"]["capability_status"], "available");
                let commit: serde_json::Value = content
                    .lines()
                    .find(|l| l.contains("\"retrieval_result_committed\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .unwrap();
                let p = &commit["payload"];
                assert_eq!(p["source_counts"]["full_text_observed"], 1);
                assert_eq!(p["visibility_degraded"], false);
                assert_eq!(p["source_ledger"][0]["source_type"], "web_page");
                assert_eq!(p["source_ledger"][0]["visibility"], "full_text_observed");
                copy_journal(&handle.journal_dir, "local-browser-read");
            })
            .await
    }

    /// 11. real doc retrieval — GAP-RETRIEVAL-TOOLS: the internal lane runs
    /// the REAL project_doc_index tool (workspace doc), producing tool-call
    /// evidence → the committed structured result carries REAL visibility
    /// (full_text_observed) and the assessment consumes the mechanical
    /// counts.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_real_doc_retrieval() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("real-doc-retrieval");
                std::fs::write(base.join("README.md"), "# Readme\n项目文档内容").unwrap();
                let run_id = "RUN-DOC-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "retrieve_project_docs".to_string(),
                            arguments: serde_json::json!({"query": "readme"}),
                            call_id: "call-1".to_string(),
                        }]),
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "project_doc_index".to_string(),
                            arguments: serde_json::json!({
                                "query": "readme",
                                "include_content": "true",
                            }),
                            call_id: "call-i1".to_string(),
                        }]),
                        ScriptedResponse::text(concat!(
                            "[DOC] README.md\n检索完成\n",
                            "[RESULT_JSON]",
                            r#"{"sections":[{"section_title":"Readme","content":"项目文档","source_ids":["SRC-001"],"claim_strength":"observed"}],"claims":[]}"#,
                            "[/RESULT_JSON]",
                        )),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ]),
                ))
                .with_snapshot_store(Some(handle.snapshot_store.clone()))
                .with_retrieval_mode(
                    orz_loop::controller::RetrievalMode::FrameworkFallback,
                    orz_loop::controller::RetrievalCapability::Available,
                    false,
                    None,
                    None,
                );
                controller
                    .run_turn(
                        &host,
                        "查项目文档",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "real-doc-retrieval",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "tool_started",
                        // Subagent round → the real index tool (host): the
                        // Interactive permission bridge records its
                        // auto-allow, then the tool runs.
                        "model_output",
                        "permission_requested",
                        "permission_decision",
                        "tool_started",
                        "tool_completed",
                        // The subagent's answer round (result formation).
                        "model_output",
                        "runtime_stagnation_guard",
                        "tool_completed",
                        "retrieval_result_committed",
                        "information_sufficiency_assessment",
                        // FUS-TOOL-PROBE P0-A-2 审查复核（2026-08-13）：
                        // 评估落地后 retrieval_disposition 探针翻转事件。
                        "tool_availability_check",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                let commit: serde_json::Value = content
                    .lines()
                    .find(|l| l.contains("\"retrieval_result_committed\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .unwrap();
                let p = &commit["payload"];
                assert_eq!(p["source_counts"]["full_text_observed"], 1);
                assert_eq!(p["visibility_degraded"], false);
                assert_eq!(p["organized_response"]["sections"][0]["source_ids"][0], "SRC-001");
                copy_journal(&handle.journal_dir, "real-doc-retrieval");
            })
            .await
    }

    /// 11. cross-run activation restore — GAP-RETRIEVAL-TOOLS: a seeded
    /// AwaitingDisposition activation is journaled as
    /// `retrieval_activation_restored` at startup and the parent's
    /// disposition closes it across runs (the verifier resolves the
    /// assessment through the restore declaration).
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_cross_prompt_activation_restore() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("cross-prompt-restore");
                let run_id = "RUN-RESTORE-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let snapshot = serde_json::json!({
                    "next_seq": {"internal_retrieval": 1},
                    "activations": [{
                        "activation_id": "retrieval-internal_retrieval-sess-abc-00",
                        "parent_session_id": "sess-abcdef123456",
                        "subagent_session_id": "SUB-internal_retrieval-sess-abc",
                        "contract_id": "retrieval-contract-internal_retrieval",
                        "contract_revision": 0,
                        "status": "awaiting_disposition",
                        "tool_rounds_used": 3,
                        "result_digest": "b".repeat(64),
                        "pending_assessment_id": "ASSESS-PREV-1",
                        "pending_expected_contract_revision": 0,
                        "origin_run_id": "RUN-PREV-0001",
                    }]
                });
                let controller =
                    orz_loop::AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "retrieval_disposition".to_string(),
                            arguments: serde_json::json!({
                                "role": "internal_retrieval",
                                "decision": "close",
                            }),
                            call_id: "call-d1".to_string(),
                        }]),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()))
                    .with_retrieval_mode(
                        orz_loop::controller::RetrievalMode::FrameworkFallback,
                        orz_loop::controller::RetrievalCapability::Available,
                        false,
                        None,
                        None,
                    )
                    .with_activation_snapshot(Some(&snapshot));
                controller
                    .run_turn(
                        &host,
                        "关闭检索",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "cross-prompt-restore",
                    &[
                        "run_preflight",
                        "retrieval_activation_restored",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "tool_started",
                        "retrieval_parent_disposition",
                        "retrieval_close_record",
                        "tool_completed",
                        // FUS-TOOL-PROBE P0-A-2 审查复核（2026-08-13）：
                        // disposition 消费后 pending assessment 清除，
                        // retrieval_disposition 探针翻转事件。
                        "tool_availability_check",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                copy_journal(&handle.journal_dir, "cross-prompt-restore");
            })
            .await
    }

    /// 12. pre-handoff checkpoint — GAP-RETRIEVAL-TOOLS: a stagnation
    /// restart decision journals the pre-handoff orientation checkpoint
    /// (ADR-0010 §11.1 — independent lifecycle trigger) before the
    /// run_invalidated terminal.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_pre_handoff_checkpoint() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("pre-handoff-checkpoint");
                let run_id = "RUN-PREHAND-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let pattern = "重复 的 片段 ";
                let repeated = pattern.repeat(11);
                let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(
                    FakeProvider::from_texts(vec![repeated.as_str(), repeated.as_str()]),
                ))
                .with_snapshot_store(Some(handle.snapshot_store.clone()));
                controller
                    .run_turn(
                        &host,
                        "输出结果",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "pre-handoff-checkpoint",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "runtime_stagnation_guard",
                        "orientation_checkpoint",
                        "run_invalidated",
                    ],
                    "run_invalidated",
                );
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                let checkpoint: serde_json::Value = content
                    .lines()
                    .find(|l| l.contains("\"pre_handoff\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .unwrap();
                assert_eq!(checkpoint["payload"]["trigger"], "pre_handoff");
                assert_eq!(checkpoint["payload"]["injection_position"], "pre_terminal");
                copy_journal(&handle.journal_dir, "pre-handoff-checkpoint");
            })
            .await
    }

    /// 14. P0-B step 5 (2026-08-14): a final answer carrying an unknown
    /// `[来源: SRC-999]` marker is blocked by the output-level citation
    /// verifier (ADR-0010 §3.7.9) — the journal records
    /// `citation_validation` with the mechanical reason code and the run
    /// still finishes normally.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_citation_validation_block() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("citation-validation-block");
                let run_id = "RUN-CITE-CONF";
                let handle = bootstrap_session(run_id, Some(base.clone()), TrustPolicy::Skip)
                    .await
                    .unwrap();
                let host = build_cli_host(&handle, run_id, &base).unwrap();
                let controller =
                    orz_loop::AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
                        ScriptedResponse::text("X"),
                        ScriptedResponse::text("最终结论 [来源: SRC-999]"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()));
                let (response, _, _) = controller
                    .run_turn(
                        &host,
                        "hello",
                        run_id,
                        &handle.run_manifest_sha256,
                        handle.next_sequence,
                        handle.last_event_sha256.clone(),
                        None,
                        None,
                    )
                    .await
                    .unwrap();
                assert!(
                    response.starts_with("[CITATION_VALIDATION_FAILED"),
                    "the delivered answer must be the degradation block: {response}"
                );
                handle.journal.shutdown_async().await.unwrap();
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "citation-validation-block",
                    &[
                        "run_preflight",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "citation_validation",
                        "runtime_stagnation_guard",
                        "run_finished",
                    ],
                    "run_finished",
                );
                copy_journal(&handle.journal_dir, "citation-validation-block");
            })
            .await
    }
}
