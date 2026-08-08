//! `orz-codex` — the Codex-style fallback TUI entry (design §2.4, Phase 3
//! slice #12).
//!
//! An independent binary name (2026-08-05 user ruling): the `orz` binary is
//! unchanged and never activates this; `orz-codex` is the explicit fallback
//! entry. Default-off at runtime — nothing runs until this binary is invoked.
//! One process, one view: the codex-style UI, no assurance panels.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::{ModelGateway, ToolCall};

fn main() {
    // L1 (2026-08-08 write placement): redirect `$GROK_HOME` off the user
    // directory — MUST run before any `orz_config::grok_home()` call
    // (OnceLock). cwd = process-start cwd, before `--run-root` re-anchors
    // (design Q3 ruling). Design:
    // docs/WRITE_PLACEMENT_AND_GRILL_DESIGN_2026-08-08.md §1/§2.
    let placement = orz_host::grok_home::redirect_grok_home(
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );
    if matches!(
        placement,
        orz_host::grok_home::GrokHomePlacement::UserFallback
    ) {
        eprintln!(
            "warning: install dir and cwd/.gsa both unwritable — $GROK_HOME stays on the user directory (last resort)"
        );
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut fake_provider = false;
    let mut real = false;
    let mut sandbox = "workspace-write";

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--run-root" => {
                i += 1;
                if let Some(dir) = args.get(i) {
                    cwd = PathBuf::from(dir);
                } else {
                    eprintln!("--run-root requires a directory");
                    print_help();
                    return;
                }
            }
            "--sandbox" => {
                i += 1;
                if let Some(value) = args.get(i) {
                    if !matches!(value.as_str(), "read-only" | "workspace-write") {
                        eprintln!("--sandbox accepts \"read-only\" or \"workspace-write\"");
                        print_help();
                        std::process::exit(2);
                    }
                    sandbox = value;
                } else {
                    eprintln!("--sandbox requires a value");
                    print_help();
                    std::process::exit(2);
                }
            }
            "--fake-provider" => fake_provider = true,
            "--real" => real = true,
            "-h" | "--help" => {
                print_help();
                return;
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                return;
            }
        }
        i += 1;
    }

    if real && fake_provider {
        eprintln!("--real and --fake-provider are mutually exclusive");
        print_help();
        std::process::exit(2);
    }
    if fake_provider {
        // The demo gateway checks this env flag (mirrors orz-bin's
        // --fake-provider handling). `set_var` is unsafe in edition 2024 —
        // single-threaded, pre-runtime (orz-tui precedent).
        unsafe { std::env::set_var("ORZ_FAKE_TOOL", "1") };
    }
    if real {
        // `--real`: real DeepSeek transport — the production wire (alpha test
        // ruling 2026-08-06). Same env-flag seam as ORZ_FAKE_TOOL so
        // build_gateway stays the single decision point; fail-closed there
        // (no credential → error exit, never a fake fallback).
        unsafe { std::env::set_var("ORZ_REAL", "1") };
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let local = tokio::task::LocalSet::new();
    let config = orz_codex::TuiConfig {
        cwd,
        sandbox: sandbox.to_owned(),
    };
    if let Err(e) = local.block_on(&runtime, orz_codex::run(config, build_gateway())) {
        eprintln!("orz-codex: {e}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!(
        "orz-codex — Codex 风格兜底 TUI（设计 §2.4）\n\
         \n\
         usage: orz-codex [--run-root <dir>] [--sandbox <mode>] [--fake-provider] [--real]\n\
         \n\
         --fake-provider       用脚本化假模型演示（工具 + 审批流）\n\
         --real                真实 DeepSeek 模型（Windows 凭据管理器 orz-deepseek/agent，\n\
                               ADR-0006；无凭据即报错退出，与 --fake-provider 互斥）\n\
         --run-root <dir>      会话工作目录（默认当前目录）\n\
         --sandbox <mode>      线程沙箱：workspace-write（默认，交互审批）| read-only\n\
                               （只读工具自动放行，写/网络静默拒绝；仅线程创建时生效，\n\
                               后续 prompt 复用活动线程，切换需重启）"
    );
}

/// Demo gateway: scripted tool calls + text. Mirrors orz-bin's builder but
/// uses the finalized toolset's real bash name (`run_terminal_cmd`), so the
/// approval flow genuinely executes under `--fake-provider`.
fn build_gateway() -> Arc<dyn ModelGateway> {
    // `--real` (main sets ORZ_REAL): the real DeepSeek transport. Fail-closed
    // — missing credentials exit(2) with the ADR-0006 target named, never a
    // silent FakeProvider fallback.
    if std::env::var("ORZ_REAL").is_ok() {
        match orz_loop::gateway::transport::real_gateway_from_credentials() {
            Ok(gateway) => return gateway,
            Err(e) => {
                eprintln!(
                    "--real requires a DeepSeek API key — target: {} (Windows Credential Manager, ADR-0006)",
                    orz_loop::gateway::credentials::AGENT_CREDENTIAL_TARGET
                );
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    }
    if std::env::var("ORZ_FAKE_TOOL").is_ok() {
        Arc::new(
            FakeProvider::new(vec![
                ScriptedResponse::tool_calls(vec![ToolCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({ "target_file": "rust-toolchain.toml" }),
                    call_id: "call-1".to_string(),
                }]),
                ScriptedResponse::tool_calls(vec![ToolCall {
                    name: "run_terminal_cmd".to_string(),
                    arguments: serde_json::json!({ "command": "dir" }),
                    call_id: "call-2".to_string(),
                }]),
                ScriptedResponse::text(
                    "完成（fake 演示：read_file 已执行，run_terminal_cmd 需审批）。",
                ),
                ScriptedResponse::text(
                    "完成（fake 演示：read_file 已执行，run_terminal_cmd 需审批）。",
                ),
            ])
            .with_chunk_delay(Duration::from_millis(120)),
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
