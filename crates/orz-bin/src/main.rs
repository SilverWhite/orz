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

use orz_host::session::{bootstrap_session, SessionHandle};
use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::{ModelGateway, ToolCall};

fn main() {
    // Minimal arg parsing (no clap yet — Phase 2+ adds the real CLI)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--stdio") {
        run_stdio();
        return;
    }
    let prompt = parse_prompt(&args).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        eprintln!("usage: orz -p \"<prompt>\"  (or --prompt <prompt>; --stdio for ACP)");
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
        let server = std::sync::Arc::new(
            orz_host::acp_server::AcpServer::with_gateway(build_gateway()),
        );
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

        let mut sm = orz_assurance::plan::PlanStateMachine::new();
        sm.enter_planning(None)
            .map_err(|e| format!("plan: {e}"))?;
        let artifact = plan_artifact_from_prompt(&run_id, prompt);
        let verification = orz_assurance::plan::verify_plan_artifact(&artifact);
        if !verification.valid {
            return Err(format!("plan artifact invalid: {:?}", verification.errors));
        }
        sm.submit_plan(artifact.clone())
            .map_err(|e| format!("plan: {e}"))?;
        let mut seq = handle.next_sequence;
        let mut prev_hash = handle.last_event_sha256.clone();
        let h1 = record_plan_event(
            &handle,
            seq,
            prev_hash.clone(),
            orz_assurance::EventType::PlanProposed,
            serde_json::json!({
                "plan_id": artifact.plan_id,
                "task_id": artifact.task_id,
                "sections": artifact.sections.len(),
            }),
        )
        .await?;
        seq += 1;
        prev_hash = Some(h1);
        let approval = sm
            .approve("user", Some("manual"))
            .map_err(|e| format!("plan: {e}"))?;
        let h2 = record_plan_event(
            &handle,
            seq,
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
        seq += 1;
        prev_hash = Some(h2);

        // Execute under the approved plan — real host + IP6 bridge.
        let host = build_cli_host(&handle, &run_id, &cwd)?;
        let controller = orz_loop::AgentLoopController::with_gateway(build_gateway());
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
        handle.run_id.clone().into(),
        seq,
        event_type,
        handle.run_manifest_sha256.clone().into(),
        prev_hash,
        "run-event-v0.1.schema.json".into(),
        payload,
        orz_assurance::Redaction::None,
        chrono_utc_now(),
    );
    orz_assurance::seal_event(&mut event).map_err(|e| e.to_string())?;
    let hash = event.event_sha256.clone();
    handle.journal.record_async(event).await.map_err(|e| e.to_string())?;
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
            section("具体设计", "使用内置工具链（read_file / run_terminal_cmd / grep）"),
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
fn build_gateway() -> Arc<dyn ModelGateway> {
    if std::env::var("ORZ_FAKE_TOOL").is_ok() {
        Arc::new(FakeProvider::new(vec![
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
            ScriptedResponse::text("完成（fake 工具路径：read_file 已执行，bash 被权限门拒绝）。"),
        ]))
    } else {
        Arc::new(FakeProvider::from_texts(vec!["(fake) 已收到请求。"]))
    }
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
    let handle = bootstrap_session(&run_id, Some(cwd.clone()), orz_host::session::TrustPolicy::Enforce)
        .await?;

    // Phase 3 wiring: real OrzHost (GrokBuild toolset + trust) behind the
    // IP6 permission bridge. Headless (`None` gateway): Read auto-allows,
    // Bash Ask → Deny.
    let host = build_cli_host(&handle, &run_id, &cwd)?;

    let controller = orz_loop::AgentLoopController::with_gateway(build_gateway());
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
fn build_cli_host(
    handle: &SessionHandle,
    session_id: &str,
    cwd: &Path,
) -> Result<orz_host::OrzHost, String> {
    orz_host::OrzHost::with_bridge(
        session_id,
        handle.journal.clone(),
        cwd,
        handle.workspace_trust,
        None,
    )
}

/// Short timestamp-based suffix for the run ID (no uuid dep in orz-bin yet).
fn timestamp_suffix() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:08x}", now.as_secs())
}
