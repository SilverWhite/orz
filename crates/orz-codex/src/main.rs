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
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut fake_provider = false;

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
            "--fake-provider" => fake_provider = true,
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

    if fake_provider {
        // The demo gateway checks this env flag (mirrors orz-bin's
        // --fake-provider handling). `set_var` is unsafe in edition 2024 —
        // single-threaded, pre-runtime (orz-tui precedent).
        unsafe { std::env::set_var("ORZ_FAKE_TOOL", "1") };
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let local = tokio::task::LocalSet::new();
    let config = orz_codex::TuiConfig { cwd };
    if let Err(e) = local.block_on(&runtime, orz_codex::run(config, build_gateway())) {
        eprintln!("orz-codex: {e}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!(
        "orz-codex — Codex 风格兜底 TUI（设计 §2.4）\n\
         \n\
         usage: orz-codex [--run-root <dir>] [--fake-provider]\n\
         \n\
         --fake-provider   用脚本化假模型演示（工具 + 审批流）\n\
         --run-root <dir>  会话工作目录（默认当前目录）"
    );
}

/// Demo gateway: scripted tool calls + text. Mirrors orz-bin's builder but
/// uses the finalized toolset's real bash name (`run_terminal_cmd`), so the
/// approval flow genuinely executes under `--fake-provider`.
fn build_gateway() -> Arc<dyn ModelGateway> {
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
                ScriptedResponse::text("完成（fake 演示：read_file 已执行，run_terminal_cmd 需审批）。"),
                ScriptedResponse::text("完成（fake 演示：read_file 已执行，run_terminal_cmd 需审批）。"),
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
