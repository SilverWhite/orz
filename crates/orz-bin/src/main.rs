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

use orz_host::session::{SessionHandle, bootstrap_session};
use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::{Message, ModelGateway, Role, ToolCall};

fn main() {
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
        std::process::exit(2);
    });

    if args.iter().any(|a| a == "--plan") {
        run_plan(&prompt);
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
    let result = local.block_on(&rt, run(&prompt));

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
        orz_host::stdio::run_stdio_server(server).await
    });
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// Plan-mode entry: walk the plan state machine (enter → submit → approve),
/// journal the plan events, then execute the turn under the approved plan.
/// Demonstrates the plan/action two-level approval separation end-to-end.
fn run_plan(prompt: &str) {
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

        let mut seq = handle.next_sequence;
        let mut prev_hash = handle.last_event_sha256.clone();
        let gateway = build_gateway();

        // Plan phase — on error the journal must still terminate (review
        // P2-1): record RunFailed continuing the chain, then exit.
        if let Err(e) =
            run_plan_phase(&handle, &mut seq, &mut prev_hash, &run_id, prompt, &gateway).await
        {
            record_plan_failure(&handle, seq, prev_hash.clone(), &e).await;
            let _ = handle.journal.shutdown_async().await;
            return Err(e);
        }

        // Execute under the approved plan — real host + IP6 bridge, sharing
        // the gateway instance (its script continues after the gate round).
        let host = build_cli_host(&handle, &run_id, &cwd)?;
        let controller = orz_loop::AgentLoopController::with_gateway(gateway)
            .with_snapshot_store(Some(handle.snapshot_store.clone()));
        let (response, _, _) = controller
            .run_turn(
                &host,
                prompt,
                &handle.run_id,
                &handle.run_manifest_sha256,
                seq,
                prev_hash,
            )
            .await
            .map_err(|e| e.to_string())?;
        handle
            .journal
            .shutdown_async()
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>((response, handle.journal_dir.join("events.jsonl")))
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
/// advancing the chain (`seq`/`prev_hash`). Returns Ok on success; the caller
/// terminates the journal on error (review P2-1: plan-phase failures must not
/// leave the journal without a terminal event).
async fn run_plan_phase(
    handle: &orz_host::session::SessionHandle,
    seq: &mut u64,
    prev_hash: &mut Option<String>,
    run_id: &str,
    prompt: &str,
    gateway: &Arc<dyn ModelGateway>,
) -> Result<(), String> {
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
        }),
    )
    .await?;
    *seq += 1;
    *prev_hash = Some(h2);
    Ok(())
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
    let mut event = orz_assurance::RunEvent::new(
        handle.run_id.clone(),
        seq,
        event_type,
        handle.run_manifest_sha256.clone(),
        prev_hash,
        "run-event-v0.1.schema.json".into(),
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
async fn run(prompt: &str) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
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

    // Phase 3 wiring: real OrzHost (GrokBuild toolset + trust) behind the
    // IP6 permission bridge. Headless (`None` gateway): Read auto-allows,
    // Bash Ask → Deny.
    let host = build_cli_host(&handle, &run_id, &cwd)?;

    let controller = orz_loop::AgentLoopController::with_gateway(build_gateway())
        .with_snapshot_store(Some(handle.snapshot_store.clone()));
    let (response, _, _) = controller
        .run_turn(
            &host,
            prompt,
            &handle.run_id,
            &handle.run_manifest_sha256,
            handle.next_sequence,
            handle.last_event_sha256.clone(),
        )
        .await?;

    handle.journal.shutdown_async().await?;

    Ok((response, handle.journal_dir.join("events.jsonl")))
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
        orz_loop::host::TestRunner {
            command: cmd.split_whitespace().map(str::to_string).collect(),
            timeout: None,
        }
    });
    Ok(orz_host::OrzHost::with_bridge_and_hub_policy(
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
    .with_test_runner(test_runner))
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

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-bin-plan-fail-{}-{:08x}",
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
        let err = run_plan_phase(
            &handle,
            &mut seq,
            &mut prev_hash,
            "RUN-PLAN-FAIL",
            "hi",
            &gateway,
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
    use orz_host::acp_server::{AcpError, AcpServer};
    use orz_host::session::TrustPolicy;
    use orz_loop::controller::AgentLoopError;

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
                        "orientation_checkpoint",
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
                        "orientation_checkpoint",
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
                run_plan_phase(&handle, &mut seq, &mut prev_hash, run_id, "hi", &gateway)
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
                        "orientation_checkpoint",
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
                        "orientation_checkpoint",
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
                        "orientation_checkpoint",
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
}
