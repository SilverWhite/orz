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
use orz_loop::gateway::model::{Message, ModelGateway, ToolCall};

/// FUS-BENCHMARK-FULL-EXEC (2026-08-18)：解析 headless benchmark 两轴旗标。
/// 只接受无值精确形式 `--allow-shell` / `--allow-network`；`=value` 形式
/// 显式报错（2026-08-18 审查收口——「静默忽略」绝不允许）。任一带
/// `--allow-write` 缺失即报错（fail-closed 配对，防静默无效）。
fn parse_benchmark_flags(args: &[String]) -> Result<(bool, bool), String> {
    let allow_write = args.iter().any(|a| a == "--allow-write");
    if args
        .iter()
        .any(|a| a.starts_with("--allow-shell=") || a.starts_with("--allow-network="))
    {
        return Err(
            "--allow-shell/--allow-network take no value (use --allow-shell / \
             --allow-network, or ORZ_ALLOW_SHELL / ORZ_ALLOW_NETWORK env presence)"
                .to_string(),
        );
    }
    let allow_shell = args.iter().any(|a| a == "--allow-shell");
    let allow_network = args.iter().any(|a| a == "--allow-network");
    if (allow_shell || allow_network) && !allow_write {
        return Err(
            "--allow-shell/--allow-network require --allow-write (Benchmark policy)".to_string(),
        );
    }
    Ok((allow_shell, allow_network))
}

/// F4 / 0as (2026-09-19): switch the console output code page to UTF-8.
/// Zero-dependency FFI (house precedent: orz-workspace foreign_sessions
/// capability, xai-acp-lib stdin_reader) — a windows-sys edge on orz-bin
/// would churn the shared Cargo.lock this batch must not touch.
#[cfg(windows)]
unsafe fn set_console_output_code_page_utf8() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "SetConsoleOutputCP"]
        fn set_console_output_cp(w_code_page_id: u32) -> i32;
    }
    // SAFETY: kernel32 is always loaded; the call is per-console and
    // best-effort — failure (no console, redirected pipes) is ignored and
    // must never block startup.
    unsafe {
        let _ = set_console_output_cp(65001);
    }
}

/// 0bt② (2026-09-26): single-source carrier build info line
/// (`--build-info`). The version is the package version the carrier was
/// built from — the same bump-record string the release flow uses.
fn build_info_line() -> String {
    format!(
        "orz-build-info: version={} os={} arch={} profile={}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    )
}

fn main() {
    // 批七（2026-09-25 用户报告 Web 工作台 prompt 崩溃）：`run_agent_loop`
    // 巨型 future（0bt 未竟拆分项②）在真实网关下首 poll 即可打穿 Windows
    // 默认主线程栈（实锚：`thread 'main' has overflowed its stack`，
    // 0xC00000FD）。整个 main 体搬上 64 MiB 显式大栈线程——所有入口
    // （TUI / -p / --stdio / web 子命令链）一并受益；子进程退出码经
    // join 透传。长期治本＝agent_loop 拆分（0bt ②）。
    let child = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(main_inner)
        .expect("spawn main thread (64 MiB stack)");
    match child.join() {
        Ok(()) => {}
        // REV-083-12（2026-09-27，D-12 路线）：本分支仅在 unwind 档可达——
        // 默认 dev/release/release-dist 均 `panic = "abort"`，panic 直接走
        // 平台 abort 路径终止进程，join 永不见 Err。exit(101) 契约面保留
        // 给 unwind 档（x-prod）与显式 stderr 行；契约文档见 README
        // 「Exit codes」节。
        Err(panic) => {
            eprintln!("error: main thread panicked: {panic:?}");
            std::process::exit(101);
        }
    }
}

fn main_inner() {
    // 0bt② (2026-09-26, carries 0bj① m15): `--build-info` — the carrier's own
    // packaging-face version seam. The Rust binary carries no Win32
    // VersionInfo, so `dogfood_launch -DryRun` rendered `carrier=unknown`; the
    // launcher now reads this line instead (`version=` is parsed out). Print
    // and exit 0 before ANY home redirect / session bootstrap (pure stdout,
    // no side effects).
    if std::env::args().any(|a| a == "--build-info") {
        println!("{}", build_info_line());
        return;
    }
    // F4 / 0as (2026-09-19): put the host console into UTF-8 mode on Windows
    // so orz's UTF-8 output (receipts, tool-output echo) is not rendered as
    // mojibake on GBK-codepage consoles, and child processes that follow the
    // console code page (git, …) emit UTF-8 — the decode gate then reads it
    // back cleanly. TUI/ACP lanes are unaffected (crossterm manages its own
    // console modes). Cosmetic host-side fix: the model face was already
    // clean via the mechanical encoding gate. Console CP is per-console, not
    // persisted; failure is ignored (best effort, never fatal).
    #[cfg(windows)]
    unsafe {
        set_console_output_code_page_utf8();
    }
    // 0bw② (2026-09-27, WRITE_CONTROL_MECHANICAL_DESIGN §6 运行时形态):
    // 载体完整性自检——发布打包生成的 carrier-manifest.json 在位时逐文件
    // 复验 sha256；失配/缺失 ⇒ stderr 告警横幅（审计腿，不拒绝启动；
    // 清单缺席＝开发树形态，静默跳过）。只读面，先于一切会话装配。
    // 如实注记（2026-09-27 复审 P3）：本时点 tracing subscriber 尚未初始化
    // （各运行模式在分派后各自 init），下方 `tracing` 调用为前置埋点——
    // 当前有效腿是 stderr 横幅。
    match orz_host::carrier_integrity::verify_at_startup() {
        orz_host::carrier_integrity::IntegrityReport::Violated { findings } => {
            eprintln!(
                "{}",
                orz_host::carrier_integrity::violation_banner(&findings)
            );
            tracing::warn!(findings = ?findings, "carrier integrity check failed");
        }
        orz_host::carrier_integrity::IntegrityReport::Clean { checked } => {
            tracing::debug!(checked, "carrier integrity check passed");
        }
        orz_host::carrier_integrity::IntegrityReport::NoManifest => {}
    }
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
    // FUS-BENCHMARK-FULL-EXEC (2026-08-18): `--allow-shell` /
    // `--allow-network` (headless benchmark only) → ORZ_ALLOW_SHELL /
    // ORZ_ALLOW_NETWORK (precedent: `--allow-write` → ORZ_ALLOW_WRITE).
    // Fail-closed: `=value` 形式显式报错；任一轴未带 `--allow-write` 报错
    // exit 2（2026-08-18 审查收口——静默忽略绝不允许）。
    match parse_benchmark_flags(&args) {
        Ok((allow_shell, allow_network)) => {
            if allow_shell {
                unsafe {
                    std::env::set_var("ORZ_ALLOW_SHELL", "1");
                }
            }
            if allow_network {
                unsafe {
                    std::env::set_var("ORZ_ALLOW_NETWORK", "1");
                }
            }
        }
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(2);
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
    // 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): `--retrieval-enabled` —
    // 会话级独立检索启用门（默认 false = fail-closed；三值模式退役后唯一
    // 授权开关）。旧 `--retrieval-mode <off|local_browser|framework_fallback>`
    // 保留兼容解析（标注废弃）：非 off 旧值仍按 legacy 兼容映射启用（不再
    // 有 lane 语义），并由 `retrieval_enabled_from_env` 打印弃用提示。
    if args.iter().any(|a| a == "--retrieval-enabled") {
        unsafe {
            std::env::set_var("ORZ_RETRIEVAL_ENABLED", "1");
        }
    }
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
    // 0br S2 (`orz web`): Web workbench bridge — loopback server serving the
    // static three-draft UI and pumping ACP over WebSocket to a spawned
    // `orz --stdio` child (crates/orz-web). Dispatch BEFORE the global
    // `--stdio` match below (review R-4: `orz web --stdio` must reach the
    // web subcommand, whose argv parser rejects `--stdio` explicitly, not
    // silently fall into stdio mode); env flags set above (--real etc.)
    // propagate to the child.
    if args.get(1).map(String::as_str) == Some("web") {
        run_web(&args[2..]);
        return;
    }
    // 0br S3 (`orz archive <session8>`): on-demand session archive for the
    // Web workbench "archive active session" action — one-shot packaging of
    // an existing sidecar via the SAME close-archive primitive the agent
    // uses (orz-host `archive_session_on_demand`); no second implementation.
    if args.get(1).map(String::as_str) == Some("archive") {
        run_archive(&args[2..]);
        return;
    }
    // 0br S3 批六 (`orz unarchive|delete-session <session8>`): the Web
    // workbench 回档/删除 actions — same spawn-by-the-bridge shape as
    // `archive`; destructive semantics live in orz-host, the binary only
    // carries exit codes and human-readable results.
    if matches!(
        args.get(1).map(String::as_str),
        Some("unarchive") | Some("delete-session")
    ) {
        run_session_maintenance(args[1].as_str(), &args[2..]);
        return;
    }
    // 0br S3 批七 (`orz trust <cwd>`): the Web workbench 信任窗 — the TUI
    // asks interactively; the headless bridge lane cannot, so the UI confirm
    // dialog relays through the bridge into this subcommand, which persists
    // the grant into the SAME GROK_HOME the prompt child reads (redirect
    // chain already applied in main, below-nothing else to align).
    if args.get(1).map(String::as_str) == Some("trust") {
        run_trust(&args[2..]);
        return;
    }
    // 0bw④ (2026-09-27): 回退窗口机械 undo 面——编辑前快照的 list/restore
    // （不涉 git；`.gsa` 域目标拒绝；restore 覆写前自动存当前内容）。
    if args.get(1).map(String::as_str) == Some("rollback") {
        run_rollback(&args[2..]);
        return;
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
            "       --allow-write selects the headless Benchmark policy (local edits auto-allow; raw shell/network follow the axes below)"
        );
        eprintln!(
            "       --allow-shell / --allow-network open the Benchmark shell/network axes (headless; require --allow-write)"
        );
        eprintln!(
            "       without --allow-write the session keeps the default auto-approve (yolo) posture; the run banner prints the resolved mode"
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

    // REV-083-01 (2026-09-27): the `-p`/`--plan` startup face prints the
    // resolved permission mode. The default posture is auto-approve (yolo),
    // NOT a fail-closed prompt matrix — the help text above and the bridge
    // `build_cli_host` are the same story. Echo only (no policy decision
    // here): the bridge remains the single source of truth.
    eprintln!("{}", permission_mode_banner());

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
    let result = local.block_on(&rt, async {
        // ACAF production flip (2026-08-16): the TUI workbench shares the
        // signer-process client + fail-closed posture of the CLI/ACP paths.
        let acaf = build_acaf_client()
            .await
            .map_err(|e| format!("acaf: {e}"))?;
        let cfg = orz_tui::TuiConfig {
            cwd,
            replay: None,
            acaf,
            acaf_fail_closed: acaf_fail_closed_enabled(),
        };
        orz_tui::run(cfg, build_gateway())
            .await
            .map_err(|e| e.to_string())
    });
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

/// DEPRECATED (0t, ADR-0010 §14.65): read legacy `ORZ_RETRIEVAL_MODE`
/// (set by deprecated `--retrieval-mode`) — 保留兼容解析。
fn retrieval_mode_from_env() -> Option<orz_loop::controller::RetrievalMode> {
    std::env::var("ORZ_RETRIEVAL_MODE")
        .ok()
        .and_then(|v| orz_loop::controller::RetrievalMode::from_wire(Some(&v)))
}

/// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): 会话级检索启用门解析——
/// 优先级：显式 `ORZ_RETRIEVAL_ENABLED`（1/true/yes/on ⇒ 开；0/false/no/off
/// ⇒ 关）> legacy `ORZ_RETRIEVAL_MODE` 兼容映射（非 off ⇒ 开，打印弃用
/// 提示）> 默认关（fail-closed）。
fn retrieval_enabled_from_env() -> bool {
    if let Ok(v) = std::env::var("ORZ_RETRIEVAL_ENABLED") {
        return matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
    }
    if let Some(mode) = retrieval_mode_from_env() {
        eprintln!(
            "warning: --retrieval-mode / ORZ_RETRIEVAL_MODE is deprecated \
             (0t γ, ADR-0010 §14.65) — lane selection is automatic; use \
             --retrieval-enabled / ORZ_RETRIEVAL_ENABLED to control the \
             enable gate"
        );
        return mode.enables_retrieval();
    }
    false
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

/// Slice 2 fail-closed production flip (2026-08-16, user-ruled enablement):
/// fail-closed is the DEFAULT — unset means enforced. Explicit
/// `ORZ_ACAF_FAIL_CLOSED=0|false|no|off` opts back into shadow mode;
/// `1|true|yes|on` confirms enforcement. Any other value is malformed and
/// fails closed (exit 2) — a typo must never silently disable a security
/// gate. Value parsing is single-sourced at
/// `orz_loop::controller::parse_acaf_fail_closed_env` (P0-GOV Task C,
/// 2026-09-04 convergence).
fn acaf_fail_closed_enabled() -> bool {
    match std::env::var("ORZ_ACAF_FAIL_CLOSED") {
        Ok(v) => match orz_loop::controller::parse_acaf_fail_closed_env(&v) {
            Ok(enforce) => {
                // REV-083-09 (2026-09-27): a *legal* opt-out value (=0 and
                // friends) must not slip into shadow silently — announce the
                // posture on the startup face. The value stays honored
                // (explicit opt-out); only visibility changes. A malformed
                // value keeps the exit-2 fail-closed path below.
                if !enforce {
                    eprintln!(
                        "warning: ORZ_ACAF_FAIL_CLOSED={v:?} — ACAF shadow mode: \
                         fail-closed disabled by explicit opt-out (GAP-ACAF-SHADOW-VISIBILITY)"
                    );
                }
                enforce
            }
            Err(()) => {
                eprintln!(
                    "error: ORZ_ACAF_FAIL_CLOSED={v:?} is not a valid value \
                     (1/true/yes/on enforce, 0/false/no/off shadow; unset = enforce)"
                );
                std::process::exit(2);
            }
        },
        Err(_) => true,
    }
}

/// REV-083-01 (2026-09-27): resolve the permission-mode banner for the
/// `-p`/`--plan` startup face. `ORZ_ALLOW_WRITE` switches the bridge to the
/// Benchmark policy (see `build_cli_host`); otherwise the session runs the
/// default Interactive + initial-yolo (auto-approve) posture. Pure echo of
/// the same env the bridge consumes — no policy decision lives here.
fn permission_mode_banner() -> String {
    if std::env::var("ORZ_ALLOW_WRITE").is_ok() {
        format!(
            "[permission] mode=benchmark allow_write=on allow_shell={} allow_network={}",
            std::env::var("ORZ_ALLOW_SHELL").is_ok(),
            std::env::var("ORZ_ALLOW_NETWORK").is_ok()
        )
    } else {
        "[permission] mode=interactive-yolo (default auto-approve; benchmark axes off)".to_string()
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
        // ACAF production flip (2026-08-16): the ACP server path now shares
        // the signer-process client + fail-closed posture of the CLI run
        // path (previously unticketed). Unset `ORZ_ACAF_FAIL_CLOSED` =
        // enforced; an unconfigured fabric then refuses runs (D-15).
        let acaf = build_acaf_client()
            .await
            .map_err(|e| format!("acaf: {e}"))?;
        let server = std::sync::Arc::new(
            orz_host::acp_server::AcpServer::with_gateway(build_gateway())
                .with_acaf(acaf)
                .with_acaf_fail_closed(acaf_fail_closed_enabled()),
        );
        orz_host::stdio::run_stdio_server(server, retrieval_enabled_from_env())
            .await
            .map_err(|e| e.to_string())
    });
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// 0br S3 (`orz archive <session8>`): on-demand session archive entry.
/// Runs in the current working directory (the bridge spawns it with the
/// workspace cwd); stdout carries the human-readable result, exit code
/// carries success. Tracing posture matches `run_web` so failures land in
/// the bridge log.
fn run_archive(rest: &[String]) {
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let Some(session8) = rest.first().filter(|s| !s.starts_with('-')) else {
        eprintln!("error: 用法: orz archive <session8>");
        std::process::exit(2);
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            std::process::exit(1);
        }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match rt.block_on(orz_host::acp_server::archive_session_on_demand(
        &cwd, session8,
    )) {
        Ok(msg) => println!("已归档 {session8}: {msg}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

/// 0br S3 批六（`orz unarchive|delete-session <session8>`，Web 工作台
/// 「回档」「删除」的执行体）：与 `run_archive` 同一 spawn 形态——桥以
/// 工作区 cwd 拉起本子命令，stdout 携带结果、退出码携带成败。删除为
/// 不可恢复操作，确认在前端弹窗完成；此处不做二次确认（桥是唯一调用方）。
fn run_session_maintenance(verb: &str, rest: &[String]) {
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let Some(session8) = rest.first().filter(|s| !s.starts_with('-')) else {
        eprintln!("error: 用法: orz {verb} <session8>");
        std::process::exit(2);
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            std::process::exit(1);
        }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let result = rt.block_on(async {
        if verb == "unarchive" {
            orz_host::acp_server::unarchive_session_on_demand(&cwd, session8).await
        } else {
            orz_host::acp_server::delete_session_on_demand(&cwd, session8).await
        }
    });
    match result {
        Ok(msg) => println!("{msg}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

/// 0br S3 批七（`orz trust <cwd>`，Web 工作台信任窗的执行体）：把工作区
/// 授信写入与 prompt 子进程同一 GROK_HOME 下的信任存储（GROK_HOME 由
/// main 的 redirect 链先行注入，本子命令继承同值——同链同存储）。
/// 注意：必须用调用方传入的**绝对** cwd；相对路径拒绝。
/// 0bw④ (2026-09-27, WRITE_CONTROL_MECHANICAL_DESIGN §6 可实施半边)：
/// 回退窗口机械 undo——`orz rollback list` ／ `orz rollback restore
/// <pointer> [target]`。逻辑单点在 `orz_host::rollback_maintenance`；
/// 本壳只做参数解析与退出码（与 archive/trust 子命令同形态）。
fn run_rollback(rest: &[String]) {
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match rest.first().map(String::as_str) {
        Some("list") => match orz_host::rollback_maintenance::list_rollback(&cwd) {
            Ok(rows) => {
                if rows.is_empty() {
                    println!("回退窗口为空（{}/.gsa/rollback/ 无快照）", cwd.display());
                    return;
                }
                println!("回退窗口（{} 条，新↴旧序见时间戳）：", rows.len());
                for row in rows {
                    let target = row
                        .target
                        .as_deref()
                        .unwrap_or("(无 meta，restore 须显式给目标)");
                    println!(
                        "  {}  {} 字节  目标 {}",
                        row.pointer, row.size_bytes, target
                    );
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        Some("restore") => {
            // 复审 P3（2026-09-27）：未知旗标不静默过滤、多余位置参数不静默
            // 忽略——undo 面不做「猜用户意图」的超范围动作。
            let args: Vec<&String> = rest.iter().skip(1).collect();
            if args.iter().any(|s| s.starts_with('-')) {
                eprintln!(
                    "error: orz rollback restore 不接受旗标: 用法: orz rollback restore <pointer> [target]"
                );
                std::process::exit(2);
            }
            let mut positional = args.into_iter();
            let Some(pointer) = positional.next() else {
                eprintln!("error: 用法: orz rollback restore <pointer> [target]");
                std::process::exit(2);
            };
            let target = match positional.next() {
                Some(t) => Some(t.as_str()),
                None => None,
            };
            if positional.next().is_some() {
                eprintln!("error: 多余参数: 用法: orz rollback restore <pointer> [target]");
                std::process::exit(2);
            }
            match orz_host::rollback_maintenance::restore_rollback(&cwd, pointer, target) {
                Ok(message) => println!("{message}"),
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("error: 用法: orz rollback <list|restore <pointer> [target]>");
            std::process::exit(2);
        }
    }
}

fn run_trust(rest: &[String]) {
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let Some(raw) = rest.first().filter(|s| !s.starts_with('-')) else {
        eprintln!("error: 用法: orz trust <cwd>");
        std::process::exit(2);
    };
    let cwd = PathBuf::from(raw);
    if !cwd.is_absolute() {
        eprintln!("error: 工作区路径必须为绝对路径: {raw}");
        std::process::exit(2);
    }
    match orz_host::session::grant_workspace_trust(&cwd) {
        Ok(msg) => println!("{msg}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

/// 0br S2 (`orz web`): Web workbench bridge entry (crates/orz-web). Same
/// stderr tracing posture as run_stdio so bridge/agent events stay visible;
/// serves on the validated loopback address until Ctrl+C (the spawned
/// `orz --stdio` child dies with the process via kill-on-drop).
fn run_web(rest: &[String]) {
    let _guard = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();
    let config = match orz_web::parse_args(rest) {
        Ok(config) => config,
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
    if let Err(e) = rt.block_on(orz_web::run(config)) {
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
            // F4 (2026-08-15, BACKLOG 6e 复查遗留): same-millisecond
            // concurrent processes must not claim the same epoch — atomically
            // reserve the number with a `.claim-<n>` file; a collision means
            // another process already owns it, so bump and retry (bounded).
            let mut plan_epoch =
                orz_loop::epoch::next_plan_epoch_from_archive(&blackboard_archive_dir);
            let mut claim_attempts = 0usize;
            while !orz_loop::epoch::claim_plan_epoch(&blackboard_archive_dir, plan_epoch) {
                plan_epoch += 1;
                claim_attempts += 1;
                if claim_attempts >= orz_loop::epoch::EPOCH_CLAIM_MAX_ATTEMPTS {
                    return Err(format!(
                        "could not claim a plan epoch after {} attempts ({}): \
                         another process is racing the blackboard archive",
                        claim_attempts,
                        blackboard_archive_dir.display()
                    ));
                }
            }

            // Plan phase — on error the journal must still terminate
            // (review P2-1): record RunFailed continuing the chain, then
            // exit.
            let artifact = match run_plan_phase(
                &handle,
                &mut seq,
                &mut prev_hash,
                &run_id,
                prompt,
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
            // sharing the gateway instance.
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
    plan_epoch: u64,
) -> Result<orz_assurance::plan::PlanArtifact, String> {
    let mut sm = orz_assurance::plan::PlanStateMachine::new();
    sm.enter_planning(None).map_err(|e| format!("plan: {e}"))?;
    let artifact = plan_artifact_from_prompt(run_id, prompt);
    let verification = orz_assurance::plan::verify_plan_artifact(&artifact);
    if !verification.valid {
        return Err(format!("plan artifact invalid: {:?}", verification.errors));
    }

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
    let event = orz_assurance::RunEvent::new_v02(
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
    // 0v-C（2026-09-12）：不在漏斗外预封印——记录器先对 payload 做机械脱敏
    // （sk-shape / URL 归一化等确定性改写）再封印，并返回**落盘**的
    // event_sha256。链必须串这个哈希：调用方自算的哈希描述的是未改写形态，
    // 会让链上链接指向盘上不存在的值（重放报 `previous hash mismatch`）。
    handle
        .journal
        .record_async(event)
        .await
        .map_err(|e| e.to_string())
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
        // review F-C-7 (2026-09-13): `run_terminated` joins the guard's
        // terminal whitelist — a degraded journal's explicit terminal must
        // not be double-terminated (the recorder would refuse the second
        // append and surface a guard error).
        "run_finished" | "run_failed" | "run_cancelled" | "run_invalidated" | "run_terminated"
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
/// so the run ends with a clean `run_finished` (the output-health guard
/// trips on genuinely repeated spans; a clean run stays `completed`).
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
    // TER T2.3 (2026-09-04): `ORZ_FAKE_SCENARIO=<path>` — deterministic
    // scripted-provider scenario for real-tool real-machine verification
    // (auto-bg / idle-kill / cross-call survival under the Windows wall).
    // JSON array of entries: {"text": "..."} or
    // {"tool_calls": [{"name","arguments","call_id"}]}.  Fail-closed: an
    // unreadable or malformed scenario exits 2 — never a silent fallback to
    // the demo script.
    if let Ok(scenario_path) = std::env::var("ORZ_FAKE_SCENARIO") {
        match load_fake_scenario(&scenario_path) {
            Ok(script) => {
                return Arc::new(
                    FakeProvider::new(script).with_chunk_delay(std::time::Duration::from_millis(0)),
                );
            }
            Err(message) => {
                eprintln!("error: {message}");
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

/// Parse a deterministic fake-provider scenario file (see `build_gateway`).
fn load_fake_scenario(path: &str) -> Result<Vec<ScriptedResponse>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("ORZ_FAKE_SCENARIO unreadable: {path}: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("ORZ_FAKE_SCENARIO invalid JSON: {path}: {e}"))?;
    let arr = value
        .as_array()
        .ok_or_else(|| format!("ORZ_FAKE_SCENARIO must be a JSON array: {path}"))?;
    let mut out: Vec<ScriptedResponse> = Vec::with_capacity(arr.len());
    for (i, entry) in arr.iter().enumerate() {
        let obj = entry
            .as_object()
            .ok_or_else(|| format!("ORZ_FAKE_SCENARIO entry {i} must be an object"))?;
        // TER 全面审查 M2W-4 (2026-09-04)：text 与 tool_calls 并存是形状
        // 歧义——静默取 text 会让场景作者误以为 tool_calls 生效；显式 Err
        // (fail-closed)。
        if obj.contains_key("text") && obj.contains_key("tool_calls") {
            return Err(format!(
                "ORZ_FAKE_SCENARIO entry {i} must not mix text and tool_calls"
            ));
        }
        if let Some(text_value) = obj.get("text") {
            let text = text_value
                .as_str()
                .ok_or_else(|| format!("ORZ_FAKE_SCENARIO entry {i} text must be a string"))?;
            out.push(ScriptedResponse::text(text.to_string()));
            continue;
        }
        if let Some(calls_value) = obj.get("tool_calls") {
            let calls = calls_value.as_array().ok_or_else(|| {
                format!("ORZ_FAKE_SCENARIO entry {i} tool_calls must be an array")
            })?;
            if calls.is_empty() {
                // 空批会让 loop 空转一轮且无任何模型面进展——fail-closed。
                return Err(format!(
                    "ORZ_FAKE_SCENARIO entry {i} tool_calls must not be empty"
                ));
            }
            let mut tool_calls: Vec<ToolCall> = Vec::with_capacity(calls.len());
            for (j, call) in calls.iter().enumerate() {
                let call_obj = call.as_object().ok_or_else(|| {
                    format!("ORZ_FAKE_SCENARIO entry {i} tool_calls[{j}] must be an object")
                })?;
                let name = call_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        format!(
                            "ORZ_FAKE_SCENARIO entry {i} tool_calls[{j}] name (string) required"
                        )
                    })?;
                let arguments = match call_obj.get("arguments") {
                    Some(v) if v.is_object() => v.clone(),
                    Some(_) => {
                        return Err(format!(
                            "ORZ_FAKE_SCENARIO entry {i} tool_calls[{j}] arguments must be \
                             an object when present"
                        ));
                    }
                    None => serde_json::json!({}),
                };
                let default_call_id = format!("call-{i}-{j}");
                let call_id = match call_obj.get("call_id") {
                    Some(v) if v.is_string() => v.as_str().unwrap().to_string(),
                    Some(_) => {
                        return Err(format!(
                            "ORZ_FAKE_SCENARIO entry {i} tool_calls[{j}] call_id must be \
                             a string when present"
                        ));
                    }
                    None => default_call_id,
                };
                tool_calls.push(ToolCall {
                    name: name.to_string(),
                    arguments,
                    call_id,
                });
            }
            out.push(ScriptedResponse::tool_calls(tool_calls));
            continue;
        }
        return Err(format!(
            "ORZ_FAKE_SCENARIO entry {i} must have text or tool_calls"
        ));
    }
    if out.is_empty() {
        return Err("ORZ_FAKE_SCENARIO must not be an empty array".to_string());
    }
    Ok(out)
}

/// Render the plan artifact's four sections for the plan-write counterexample
/// gate round (mechanical — no model-generated plan text yet).
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
    let ts = timestamp_suffix();
    let run_id = format!("RUN-CLI-{ts}");
    // 0ak（GAP-INCREMENTAL-ARCHIVE-HEADLESS，2026-09-16 用户裁决采 B）：一次性
    // run 的会话身份——与 run 同秒后缀的 `{ts}-cli`（session8 = ts，与
    // `RUN-CLI-{ts}` journal 目录一眼互认）。每次 `-p` 都是全新会话，跨调用
    // 对话恢复不开启（GAP-CONVERSATION-RESTORE 边界不变）。
    let headless_session_id = format!("{ts}-cli");
    // 会话轴原点（StoredConversation.session_started_at 同义）。
    let session_started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
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
        let retrieval_enabled = retrieval_enabled_from_env();
        let mut host = build_cli_host(&handle, &run_id, &cwd)?;
        // 0t (ADR-0010 §14.65 / 设计 §3.1): 启用会话声明 browser_read 常驻
        // （静态双族工具面）；调用期按需懒启动。
        if retrieval_enabled {
            host.declare_browser_declared();
        }

        // 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): CLI 一次性运行接通
        // 独立检索启用门——`--retrieval-enabled` / `ORZ_RETRIEVAL_ENABLED`
        // 或 legacy `--retrieval-mode`/`ORZ_RETRIEVAL_MODE` 兼容映射（非
        // off ⇒ 开）。模式 A probe/机械降级退役：不再启动期拉起浏览器或
        // 产生 retrieval_mode_transition。
        let controller = orz_loop::AgentLoopController::with_gateway(build_gateway())
            // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.5): plan 门摘除——
            // 首轮直接进入工作工具（与第 2 轮 direct 面一致）；plan 分区
            // 保留在黑板（历史/审计），模型可不读。plan_first 仅作休眠
            // 开关保留（测试/回退），生产不再启用。
            // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console
            // default + direct 受控降级（双模式）随生产路径启用。
            .with_console_default_enabled(true)
            .with_snapshot_store(Some(handle.snapshot_store.clone()))
            .with_retrieval_enabled(retrieval_enabled)
            // ACAF Slice 1 (ADR-0011 §4.4): optional signer-process client
            // (env-gated; unconfigured → unticketed control events, zero
            // behaviour change). Shadow mode by default; Slice 2
            // fail-closed (2026-08-13): `ORZ_ACAF_FAIL_CLOSED` flips the
            // switch (D-14/D-15/D-16 — an unconfigured fabric then refuses
            // to start at all).
            .with_acaf(build_acaf_client().await?)
            .with_acaf_fail_closed(acaf_fail_closed_enabled());
        // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.6): 无头 `-p` 路径按
        // run 创建一次性 `OrientationSessionState`（内存态，无需 sidecar）
        // ——此前传 None 导致跑分路径中立问询永不触发（接线缺口）。阈值
        // 由 `ORZ_ORIENTATION_THRESHOLD` 解析（默认 50）。
        let mut orientation =
            orz_loop::orientation::OrientationSessionState::new(handle.run_id.clone());
        // 0ak（采 B）：一次性 run 现在随线程携带会话对话（空 Vec 起点＝ACP
        // 全新会话同形；「one-shot 不携带会话对话」的设计边界由此改写，
        // ADR-0010 §14.68）。带来的一项内在行为＝长 run 收尾的会话末机械
        // 压缩（session_end，零模型调用）与 ACP 车道同源生效。
        let mut conversation: Vec<Message> = Vec::new();
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
                // THIN-HARNESS-REDESIGN R1: 无头路径已接通 orientation
                // （run 内存态）。
                Some(&mut orientation),
                Some(&mut conversation),
            )
            .await?;

        handle.journal.shutdown_async().await?;
        // 0z S2 §4.2：收尾扫除——本 run 泄漏的工具子进程（breakaway/attach
        // 失败逃出两级 Job 者）在 run 结束时回收（宿主持登记表）。
        host.finalize_process_trees();

        // 0ak（采 B）收尾：会话持久化＋里程碑增量归档——跨 500K 里程碑时落
        // 对话侧车并打包 `.gsa/archives/<session8>.json.gz` 三键包（判定/
        // 打包/ARC 审计复用 ACP 车道同一套原语）。best-effort：内部失败只
        // warn，绝不影响 run 结局；归档成功给用户侧一行 stderr 指引。
        let archive = orz_host::acp_server::headless_session_archive(
            &cwd,
            &headless_session_id,
            &run_id,
            conversation,
            &controller.lif_session_snapshot(),
            Some(session_started_at),
            &controller.blackboard_conversation_snapshot(),
            controller.context_scale_notified_keys(),
            orz_host::session::TrustPolicy::Enforce,
        )
        .await;
        if archive.archived {
            if let Some(path) = &archive.archive_path {
                eprintln!("session archive: {}", path.display());
            }
        }

        // P2-13 B3（2026-09-03，ADR-0010 §14.52 / 设计 §11.2 E9；B3 复审
        // 裁决：只按黑板水位、无压缩轮数门槛）：CLI 单 run = 单会话、无
        // 独立 UI 通道——疲劳提醒降级为 run 结束时的 stderr 附言（不注入
        // 模型消息、不写入对话侧车）。
        let board_bytes = controller.blackboard().read().live_compact_bytes();
        let threshold = orz_loop::fatigue::live_budget_bytes();
        if let Some(decision) =
            orz_loop::fatigue::pending_fatigue_notice(board_bytes, threshold, &[])
        {
            eprintln!("{}", decision.notice.text);
        }

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
/// follow the FUS-BENCHMARK-FULL-EXEC axes (`--allow-shell` →
/// ORZ_ALLOW_SHELL, `--allow-network` → ORZ_ALLOW_NETWORK; both default
/// false — fail closed).
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
        orz_host::permission::PermissionPolicy::Benchmark {
            allow_shell: std::env::var("ORZ_ALLOW_SHELL").is_ok(),
            allow_network: std::env::var("ORZ_ALLOW_NETWORK").is_ok(),
        }
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
    // FUS-HOST-RESOURCE-SAFETY (2026-09-12, 0z S1): 装配期注入资源面——
    // run 级 Job 硬上限（commit / 并发 / CPU，内核强制）+ 派发前资源预检门。
    // 这是真机 `-p` 路径的生产装配点。
    .with_host_resource_safety()
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

    /// TER T2.3 (2026-09-04): the deterministic fake-provider scenario
    /// loader round-trips text/tool_calls entries and is fail-closed on
    /// malformed or empty input.
    #[test]
    fn fake_scenario_loader_round_trip_and_fail_closed() {
        let dir = test_dir();
        let good = dir.join("scenario.json");
        std::fs::write(
            &good,
            r#"[
                {"text": "hello"},
                {"tool_calls": [
                    {"name": "run_terminal_cmd", "arguments": {"command": "echo hi"}, "call_id": "c1"}
                ]},
                {"text": "done"}
            ]"#,
        )
        .unwrap();
        let script = load_fake_scenario(good.to_str().unwrap()).unwrap();
        assert_eq!(script.len(), 3, "three entries must round-trip");
        let bad = dir.join("bad.json");
        std::fs::write(&bad, r#"[{"text": 5}]"#).unwrap();
        assert!(load_fake_scenario(bad.to_str().unwrap()).is_err());
        let empty = dir.join("empty.json");
        std::fs::write(&empty, "[]").unwrap();
        assert!(load_fake_scenario(empty.to_str().unwrap()).is_err());
        // TER 全面审查 M2W-4 (2026-09-04)：三种歧义/空转形状 fail-closed。
        let mixed = dir.join("mixed.json");
        std::fs::write(
            &mixed,
            r#"[{"text": "x", "tool_calls": [{"name": "read_file"}]}]"#,
        )
        .unwrap();
        assert!(
            load_fake_scenario(mixed.to_str().unwrap()).is_err(),
            "text + tool_calls mixed entry must be rejected"
        );
        let empty_calls = dir.join("empty-calls.json");
        std::fs::write(&empty_calls, r#"[{"tool_calls": []}]"#).unwrap();
        assert!(
            load_fake_scenario(empty_calls.to_str().unwrap()).is_err(),
            "empty tool_calls batch must be rejected"
        );
        let bad_call_id = dir.join("bad-call-id.json");
        std::fs::write(
            &bad_call_id,
            r#"[{"tool_calls": [{"name": "read_file", "call_id": 7}]}]"#,
        )
        .unwrap();
        assert!(
            load_fake_scenario(bad_call_id.to_str().unwrap()).is_err(),
            "non-string call_id must be rejected instead of silently defaulted"
        );
        let bad_arguments = dir.join("bad-arguments.json");
        std::fs::write(
            &bad_arguments,
            r#"[{"tool_calls": [{"name": "read_file", "arguments": ["x"]}]}]"#,
        )
        .unwrap();
        assert!(
            load_fake_scenario(bad_arguments.to_str().unwrap()).is_err(),
            "non-object arguments must be rejected"
        );
        assert!(load_fake_scenario("Z:\\no-such-scenario.json").is_err());
    }

    /// ACAF production flip (2026-08-16): fail-closed is the DEFAULT —
    /// unset enforces; only an explicit 0/false/no/off opts back into
    /// shadow; 1/true/yes/on confirms. A malformed value is handled by
    /// `acaf_fail_closed_enabled` as exit(2) (fail-closed), so it is not
    /// asserted here.
    #[test]
    fn acaf_fail_closed_defaults_enforced_with_explicit_opt_out() {
        unsafe { std::env::remove_var("ORZ_ACAF_FAIL_CLOSED") };
        assert!(
            acaf_fail_closed_enabled(),
            "unset ORZ_ACAF_FAIL_CLOSED must enforce fail-closed"
        );
        for off in ["0", "false", "no", "off", "FALSE", "Off"] {
            unsafe { std::env::set_var("ORZ_ACAF_FAIL_CLOSED", off) };
            assert!(!acaf_fail_closed_enabled(), "{off} must opt out");
        }
        for on in ["1", "true", "yes", "on", "TRUE", "On"] {
            unsafe { std::env::set_var("ORZ_ACAF_FAIL_CLOSED", on) };
            assert!(acaf_fail_closed_enabled(), "{on} must enforce");
        }
        unsafe { std::env::remove_var("ORZ_ACAF_FAIL_CLOSED") };
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
        // ACAF 无关的守卫测试：显式关闭 fail-closed（进程环境里的
        // ORZ_ACAF_FAIL_CLOSED 会被同进程的 env 测试改动——测试隔离修复）。
        let controller =
            orz_loop::AgentLoopController::with_gateway(gateway).with_acaf_fail_closed(false);
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
            fn for_new_run(&self) -> Arc<dyn ModelGateway> {
                Arc::new(SilentGateway)
            }

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
        // 同上：守卫测试与 ACAF 无关，显式关闭 fail-closed。
        let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(SilentGateway))
            .with_acaf_fail_closed(false);
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
        // would trip the generation-time output-health guard and end the
        // run with run_invalidated, which would make this test assert the
        // wrong terminal.
        let gateway: Arc<dyn ModelGateway> = Arc::new(
            FakeProvider::from_texts(vec![
                "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu",
                "the second and final answer is entirely different prose without any repeated n-grams whatsoever",
            ])
            .with_chunk_delay(std::time::Duration::from_millis(40)),
        );
        // 同上：守卫测试与 ACAF 无关，显式关闭 fail-closed。
        let controller =
            orz_loop::AgentLoopController::with_gateway(gateway).with_acaf_fail_closed(false);
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
/// --test-threads=1` — with `ORZ_ACAF_FAIL_CLOSED=0` exported: the capture
/// hosts configure no signer fabric, and outside `cfg(test)` of orz-loop the
/// fail-closed default (unset = enforced) would refuse every run
/// (2026-09-11, 0x S2 re-capture note).
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
            _mode: orz_host::local_browser::ReadMode,
        ) -> Result<orz_host::local_browser::PageReadOutcome, orz_host::local_browser::CdpError>
        {
            Ok(orz_host::local_browser::PageReadOutcome {
                final_url: url.to_string(),
                title: "Example".to_string(),
                text: "Example Domain — This domain is for use in illustrative examples."
                    .to_string(),
            })
        }

        async fn control(
            &self,
            action: orz_host::local_browser::BrowserControlAction,
            _timeout: std::time::Duration,
        ) -> Result<orz_host::local_browser::BrowserControlOutcome, orz_host::local_browser::CdpError>
        {
            // Conformance stub：控制动作确定性返回当前落点（read stub 同
            // 形态），供 browser_control 的 conformance capture 使用。
            let url = match &action {
                orz_host::local_browser::BrowserControlAction::Navigate { url } => url.clone(),
                _ => "https://example.com/".to_string(),
            };
            Ok(orz_host::local_browser::BrowserControlOutcome {
                action_status: "ok".to_string(),
                error_class: None,
                nav_phase: "completed".to_string(),
                url: Some(url),
                title: Some("Example".to_string()),
                log: String::new(),
                // 0v S2 追加 search 面（engine / engine_attempts / results）——
                // conformance stub 只走导航级动作，其余字段取 Default（None）。
                ..Default::default()
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
            let page = self
                .read_page(url, orz_host::local_browser::ReadMode::Full)
                .await?;
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

    /// Restore a process env var on drop (scenario-scoped env mutation —
    /// conformance captures run with `--test-threads=1`; the guard keeps a
    /// later assertion failure from leaking the override into other tests).
    struct RestoreEnvOnDrop(&'static str);

    impl Drop for RestoreEnvOnDrop {
        fn drop(&mut self) {
            unsafe { std::env::remove_var(self.0) };
        }
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
                        // 0t 事件面（2026-09-09）：每次模型请求前落
                        // request_header_change（S2 Task 1 起）。
                        "request_header_change",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
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
                        "request_header_change",
                        "model_output",
                        "permission_requested",
                        "permission_decision",
                        "snapshot_created",
                        "tool_started",
                        "tool_completed",
                        // 工具轮后的机械审计更新（2026-09-09 起双条）。
                        "mechanical_audit_update",
                        "mechanical_audit_update",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "run_finished",
                    ],
                    "run_finished",
                );
                copy_journal(&journal_dir, "tool-snapshot-run");
            })
            .await
    }

    /// 3. plan — plan_proposed/plan_approved, then the execution turn under
    /// the approved plan (P2-11 DC 清理 2026-08-31: the plan-write
    /// counterexample gate round is gone).
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
                ]));
                run_plan_phase(&handle, &mut seq, &mut prev_hash, run_id, "hi", 1)
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
                        "plan_proposed",
                        "plan_approved",
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "request_header_change",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
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
                        "request_header_change",
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
                        "request_header_change",
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
                // THIN-HARNESS-REDESIGN R1 (§4.4/§4.6): 检索派发每次调用
                // 即闭环（auto_close），不再有 disposition 往返——主车道每
                // 个派发 = 1 个已完成逻辑模型轮。7 次派发 = 7 轮，阈值 7
                // 在第 7 次派发的 post-tool-batch 安全间隙跨越（唯一一次
                // 触发），随后是软门方向注入 → 终答反例门 → 终答。内部检索
                // 车道同样喂满 7 轮，但其最后文本轮后子代理循环直接收束
                // （无下一 loop-top），不产生内部触发。
                let mut script = Vec::new();
                for i in 0..7 {
                    script.push(ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "retrieve_project_docs".to_string(),
                        arguments: serde_json::json!({"query": format!("查询 {i}")}),
                        call_id: format!("call-{i}"),
                    }]));
                    script.push(ScriptedResponse::text(format!(
                        "[DOC] doc-{i}.md\n检索结果 {i}"
                    )));
                }
                // THIN-HARNESS-REDESIGN R1 (§4.3): orientation 强制模板轮
                // 退役——fire 只注入简短 [ORIENTATION] 块并继续；模型在
                // 下一轮自然回答（此处即终答草稿，被反例门截获后正式终答）。
                script.push(ScriptedResponse::text(
                    "当前任务定位：处理查询批次；下一步：汇总结果",
                ));
                script.push(ScriptedResponse::text("最终汇总完成。"));
                // 2026-09-09：checkpoint 注入轮后模型仍需一个终答文本轮
                // （注入回答 → 反例门 → 终答）；旧脚本止于反例门即耗尽。
                script.push(ScriptedResponse::text("（终答）检索批次已汇总。"));
                let controller = orz_loop::AgentLoopController::with_gateway(Arc::new(
                    FakeProvider::new(script),
                ))
                .with_snapshot_store(Some(handle.snapshot_store.clone()))
                // GAP-RETRIEVAL-TOOLS (2026-08-10): the scenario dispatches
                // retrieval — run under an explicit framework_fallback mode
                // with a fully available capability (the bare default is
                // mode=off, which would refuse every dispatch).
                .with_retrieval_enabled(true);
                // This scenario is the 7-round-crossing proof — the session
                // orientation state MUST be threaded in (a one-shot CLI run
                // would pass None and never fire). R1: 显式 7 保持原触发
                // 语义（生产默认 50，env 可覆盖）。
                let mut orientation =
                    orz_loop::orientation::OrientationSessionState::new_with_threshold(run_id, 7);
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
                    // 0t 事件面（2026-09-09）：每次模型请求前落
                    // request_header_change（S2 Task 1 起）。
                    "request_header_change",
                ];
                // Per retrieval iteration: the shared-loop dispatch (subagent
                // model round inside the parent's wrapper) + the assessment +
                // the auto_close close record (R1 §4.4 — no disposition
                // round, no probe flips). The orientation crosses the
                // 7-round threshold on the 7th retrieve's post-tool-batch
                // gap (the 7th completed main round). 2026-09-09：工具轮后
                // 双 mechanical_audit_update；第 7 轮 checkpoint 夹在两条
                // 审计更新之间（post_tool_batch_gap 语义不变）。
                for i in 0..7 {
                    expected.extend([
                        "model_output",
                        "tool_started",
                        "request_header_change",
                        "model_output",
                        "tool_completed",
                        // GAP-RETRIEVAL-TOOLS (2026-08-10): the committed
                        // structured result precedes the assessment.
                        "retrieval_result_committed",
                        "information_sufficiency_assessment",
                        // R1 (§4.4): auto_close close record per dispatch.
                        "retrieval_close_record",
                        "mechanical_audit_update",
                    ]);
                    // P0-0x (2026-09-11, ADR-0010 §14.66): the very first
                    // action batch closes with the ONE-SHOT initial-round
                    // inquiry (same gap, same event type — trigger differs).
                    if i == 0 {
                        expected.push("orientation_checkpoint");
                    }
                    if i == 6 {
                        expected.push("orientation_checkpoint");
                    }
                    expected.push("mechanical_audit_update");
                }
                expected.extend([
                    "model_output",
                    // 2026-09-09：checkpoint 注入后的回答轮先行，随后才是
                    // 反例门与终答（注入回答 → 汇总轮 → 反例门 → 终答）。
                    "model_output",
                    "counterexample_gate",
                    "model_output",
                    "run_finished",
                ]);
                verify(
                    &handle.journal_dir.join("events.jsonl"),
                    "orientation-fire-run",
                    &expected,
                    "run_finished",
                );

                // The fired checkpoints carry the v0.2 payload shape —
                // inquiry_family=neutral + inquiry_kind const. P0-0x S2
                // (§14.66): two fires share the event type — the one-shot
                // initial-round inquiry (first action batch) and the periodic
                // threshold inquiry (round 7); select by trigger.
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                let orientation_payload = |trigger: &str| -> serde_json::Value {
                    content
                        .lines()
                        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
                        .find(|e| {
                            e["event_type"] == "orientation_checkpoint"
                                && e["payload"]["trigger"] == trigger
                        })
                        .unwrap_or_else(|| panic!("orientation fire with trigger {trigger}"))
                        ["payload"]
                        .clone()
                };
                let initial = orientation_payload("initial_round");
                assert_eq!(initial["inquiry_family"], "neutral");
                assert_eq!(initial["inquiry_kind"], "orientation_checkpoint");
                assert_eq!(initial["agent_role"], "main");
                assert_eq!(initial["completed_turns_since_orientation"], 1);
                assert_eq!(initial["injection_position"], "post_tool_batch_gap");
                assert!(
                    initial["message_block"]
                        .as_str()
                        .unwrap()
                        .starts_with("[INITIAL_ROUND_INQUIRY")
                );
                let p = orientation_payload("completed_turns_interval");
                assert_eq!(p["inquiry_family"], "neutral");
                assert_eq!(p["inquiry_kind"], "orientation_checkpoint");
                assert_eq!(p["agent_role"], "main");
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
                assert_eq!(assessment_lines.len(), 7);
                let last: serde_json::Value = serde_json::from_str(assessment_lines[6]).unwrap();
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

    /// 8. disabled-gate refusal — 0t (2026-09-09, ADR-0010 §14.65 / 设计
    /// §3.1): a default (retrieval not enabled) session refuses a scripted
    /// retrieval dispatch with the explicit `retrieval_not_enabled` error —
    /// ToolCompleted(error) ALONE, no ToolStarted (the enable gate has no
    /// mode state machine to transition; the declaration projection hides
    /// the retrieval family).
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
                        "request_header_change",
                        "model_output",
                        "tool_completed",
                        "mechanical_audit_update",
                        "mechanical_audit_update",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
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
                assert_eq!(refused["payload"]["status"], "error");
                assert_eq!(refused["payload"]["error"], "retrieval_not_enabled");
                assert_eq!(refused["payload"]["target"], "internal_retrieval");
                // The enable gate is not a mode state machine: no
                // transition event, no ToolStarted, no policy_denial
                // envelope (the dispatch gate is the authorization).
                assert!(!content.contains("\"retrieval_mode_transition\""));
                assert!(!content.contains("\"tool_started\""));
                assert!(refused["payload"].get("policy_denial").is_none());
                copy_journal(&handle.journal_dir, "mode-off-refusal");
            })
            .await
    }

    /// 9. local-browser launch failure — 0t (2026-09-09, ADR-0010 §14.65 /
    /// P1 设计 §3.2 场景 S1): an ENABLED session with no usable browser
    /// backend does NOT refuse at a mode/capability gate. The web_search
    /// dispatch starts (web/native family ToolStarted), the external lane
    /// attempts browser_read (host/browser family ToolStarted), the lazy
    /// launch fails, and the loop journals
    /// ToolStarted → browser_launch_result(failure) → ToolCompleted(error
    /// `browser_launch_failed`) — the failure rides the ordinary host error
    /// path (FP-2 real cause), never a capability-status precheck. Scenario
    /// name retained for fixture continuity; semantics = S1 launch failure.
    #[tokio::test]
    #[ignore = "conformance capture — run with: cargo test -p orz-bin -- --ignored conformance_capture"]
    async fn capture_local_browser_capability_error() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = worktree("local-browser-capability");
                // Force discovery to fail loudly without ever touching a
                // real browser: ORZ_BROWSER_PATH set-but-missing is the
                // hermetic S1 (probe_launch never falls back to discovery).
                let missing = base.join("no-such-browser.exe");
                assert!(!missing.exists(), "scenario path must not exist");
                let _env_guard = RestoreEnvOnDrop("ORZ_BROWSER_PATH");
                unsafe {
                    std::env::set_var("ORZ_BROWSER_PATH", missing.to_string_lossy().to_string());
                }
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
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "browser_read".to_string(),
                            arguments: serde_json::json!({"url": "https://example.com/"}),
                            call_id: "call-b1".to_string(),
                        }]),
                        ScriptedResponse::text("[FAIL] 浏览器不可用"),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()))
                    .with_retrieval_enabled(true);
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
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "request_header_change",
                        "model_output",
                        "tool_started",
                        "request_header_change",
                        "model_output",
                        "permission_requested",
                        "permission_decision",
                        "tool_started",
                        "browser_launch_result",
                        "tool_completed",
                        "model_output",
                        "tool_completed",
                        "retrieval_result_committed",
                        "information_sufficiency_assessment",
                        "retrieval_close_record",
                        "mechanical_audit_update",
                        "mechanical_audit_update",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "run_finished",
                    ],
                    "run_finished",
                );
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                // 双族并存：web_search（原生/web 族，external 派发面）与
                // browser_read（浏览器族，host 面）都真实启动——不再有任何
                // capability 预检拒绝。
                let started_tools: Vec<serde_json::Value> = content
                    .lines()
                    .filter(|l| l.contains("\"tool_started\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect();
                assert!(
                    started_tools.iter().any(|e| {
                        e["payload"]["tool"] == "web_search"
                            && e["payload"]["target"] == "external_retrieval"
                    }),
                    "web family dispatch must start under the enabled session"
                );
                assert!(
                    started_tools
                        .iter()
                        .any(|e| e["payload"]["tool"] == "browser_read"),
                    "browser family host tool must start under the enabled session"
                );
                // S1: ToolStarted → browser_launch_result(failure) →
                // ToolCompleted(error browser_launch_failed) 全序 + 稳定码。
                let fact: serde_json::Value = content
                    .lines()
                    .find(|l| l.contains("\"browser_launch_result\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .expect("browser_launch_result failure fact");
                assert_eq!(fact["payload"]["status"], "failure");
                assert!(
                    fact["payload"]["cause"]
                        .as_str()
                        .is_some_and(|c| c.contains("ORZ_BROWSER_PATH")),
                    "real launch cause must ride the fact: {}",
                    fact["payload"]["cause"]
                );
                let completed: Vec<serde_json::Value> = content
                    .lines()
                    .filter(|l| l.contains("\"tool_completed\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect();
                let launched_completed = completed
                    .iter()
                    .find(|e| {
                        e["payload"]["tool"] == "browser_read" && e["payload"]["status"] == "error"
                    })
                    .expect("browser_read launch-failure completion");
                assert_eq!(
                    launched_completed["payload"]["error"],
                    "browser_launch_failed"
                );
                assert!(!content.contains("\"retrieval_mode_transition\""));
                assert!(!content.contains("\"retrieval_capability_unavailable\""));
                copy_journal(&handle.journal_dir, "local-browser-capability");
            })
            .await
    }

    /// 10. local-browser read — 0t (2026-09-09, ADR-0010 §14.65 / 设计
    /// §3.1/§3.2): an ENABLED session with a ready fake browser lane runs
    /// the real host `browser_read` tool inside the external retrieval
    /// subagent. The event stream carries BOTH families' ToolStarted
    /// (web_search dispatch + browser_read host tool); because the session
    /// is already ready (S4), no launch attempt happens and therefore NO
    /// browser_launch_result event is written. The committed structured
    /// result carries REAL full-text web_page evidence.
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
                let controller =
                    orz_loop::AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
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
                        ScriptedResponse::text("[DOC] https://example.com/ 检索完成"),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()))
                    .with_retrieval_enabled(true);
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
                        "tool_availability_check",
                        "run_started",
                        "prompt_submitted",
                        "request_header_change",
                        "model_output",
                        "tool_started",
                        "request_header_change",
                        // Subagent round → the host browser_read tool
                        // (Interactive bridge auto-allow, then execution).
                        "model_output",
                        "permission_requested",
                        "permission_decision",
                        "tool_started",
                        "tool_completed",
                        // The subagent's answer round (result formation).
                        "model_output",
                        "tool_completed",
                        "retrieval_result_committed",
                        "information_sufficiency_assessment",
                        // THIN-HARNESS-REDESIGN R1 (§4.4): auto_close close
                        // record per dispatch——无 disposition 往返、无
                        // retrieval_disposition 探针翻转。
                        "retrieval_close_record",
                        "mechanical_audit_update",
                        "mechanical_audit_update",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "run_finished",
                    ],
                    "run_finished",
                );
                let content =
                    std::fs::read_to_string(handle.journal_dir.join("events.jsonl")).unwrap();
                // 双族并存：web_search 派发与 browser_read host 工具均
                // 产生 ToolStarted（启用门不再有 capability 预检/模式互斥）。
                let started_tools: Vec<serde_json::Value> = content
                    .lines()
                    .filter(|l| l.contains("\"tool_started\""))
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect();
                assert!(
                    started_tools.iter().any(|e| {
                        e["payload"]["tool"] == "web_search"
                            && e["payload"]["target"] == "external_retrieval"
                    }),
                    "web family dispatch ToolStarted must be present"
                );
                assert!(
                    started_tools
                        .iter()
                        .any(|e| e["payload"]["tool"] == "browser_read"),
                    "browser family host ToolStarted must be present"
                );
                // 已就绪会话 = 无启动尝试 → S4 无 fact；同时退役面完全不
                // 再出现（无 transition、无 capability_status）。
                assert!(!content.contains("\"browser_launch_result\""));
                assert!(!content.contains("\"retrieval_mode_transition\""));
                assert!(!content.contains("\"capability_status\""));
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
                let controller =
                    orz_loop::AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "retrieve_project_docs".to_string(),
                            arguments: serde_json::json!({"query": "readme"}),
                            call_id: "call-1".to_string(),
                        }]),
                        // THIN-HARNESS-REDESIGN R1 (§4.1): project_doc_index
                        // 从 host 声明面移除——内部检索子代理用保留工具
                        // read_file 直接点读。
                        ScriptedResponse::tool_calls(vec![ToolCall {
                            name: "read_file".to_string(),
                            arguments: serde_json::json!({
                                "target_file": "README.md",
                            }),
                            call_id: "call-i1".to_string(),
                        }]),
                        ScriptedResponse::text("[DOC] README.md\n检索完成"),
                        ScriptedResponse::text("完成。"),
                        ScriptedResponse::text("完成。"),
                    ])))
                    .with_snapshot_store(Some(handle.snapshot_store.clone()))
                    .with_retrieval_enabled(true);
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
                        "request_header_change",
                        "model_output",
                        "tool_started",
                        // Subagent round → read_file (host): the Interactive
                        // permission bridge records its auto-allow, then the
                        // tool runs.
                        "request_header_change",
                        "model_output",
                        "permission_requested",
                        "permission_decision",
                        "tool_started",
                        "tool_completed",
                        // The subagent's answer round (result formation).
                        "model_output",
                        "tool_completed",
                        "retrieval_result_committed",
                        "information_sufficiency_assessment",
                        // THIN-HARNESS-REDESIGN R1 (§4.4): auto_close close
                        // record per dispatch——无探针翻转。
                        "retrieval_close_record",
                        // 工具轮后的机械审计更新（2026-09-09 起双条）。
                        "mechanical_audit_update",
                        "mechanical_audit_update",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
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
                // GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30):
                // organized_response 已删除——机械 ledger 单轨断言。
                assert!(p.get("organized_response").is_none());
                assert_eq!(p["source_ledger"][0]["source_type"], "local_file");
                assert_eq!(p["source_ledger"][0]["visibility"], "full_text_observed");
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
                    .with_retrieval_enabled(true)
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
                        "request_header_change",
                        "model_output",
                        "tool_started",
                        "retrieval_parent_disposition",
                        "retrieval_close_record",
                        "tool_completed",
                        // 工具轮后的机械审计更新（2026-09-09 起；
                        // disposition 工具单条——无普通工具审计对）。
                        "mechanical_audit_update",
                        "model_output",
                        "counterexample_gate",
                        "model_output",
                        "run_finished",
                    ],
                    "run_finished",
                );
                copy_journal(&handle.journal_dir, "cross-prompt-restore");
            })
            .await
    }
}

#[cfg(test)]
mod benchmark_flags_tests {
    use super::parse_benchmark_flags;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): CLI 检索启用门解析——
    /// 显式 ORZ_RETRIEVAL_ENABLED 优先；legacy ORZ_RETRIEVAL_MODE 非 off
    /// 兼容映射为开；缺省关（fail-closed）。
    #[test]
    fn retrieval_enabled_env_decisions() {
        // SAFETY: test-only env mutation; these tests do not run in
        // parallel with each other's env assertions (cargo runs tests in
        // threads — the vars are unique to this module's tests).
        unsafe {
            std::env::remove_var("ORZ_RETRIEVAL_ENABLED");
            std::env::remove_var("ORZ_RETRIEVAL_MODE");
        }
        assert!(
            !super::retrieval_enabled_from_env(),
            "default = fail-closed"
        );

        unsafe {
            std::env::set_var("ORZ_RETRIEVAL_ENABLED", "1");
        }
        assert!(super::retrieval_enabled_from_env());
        unsafe {
            std::env::set_var("ORZ_RETRIEVAL_ENABLED", "false");
        }
        assert!(!super::retrieval_enabled_from_env(), "explicit off wins");

        unsafe {
            std::env::remove_var("ORZ_RETRIEVAL_ENABLED");
            std::env::set_var("ORZ_RETRIEVAL_MODE", "local_browser");
        }
        assert!(
            super::retrieval_enabled_from_env(),
            "legacy non-off maps to enabled (deprecation path)"
        );
        unsafe {
            std::env::set_var("ORZ_RETRIEVAL_MODE", "off");
        }
        assert!(
            !super::retrieval_enabled_from_env(),
            "legacy off = disabled"
        );
        unsafe {
            std::env::remove_var("ORZ_RETRIEVAL_MODE");
        }
    }

    #[test]
    fn no_flags_default_fail_closed() {
        assert_eq!(parse_benchmark_flags(&args(&[])).unwrap(), (false, false));
        assert_eq!(
            parse_benchmark_flags(&args(&["--real", "-p", "hi"])).unwrap(),
            (false, false)
        );
    }

    #[test]
    fn axes_open_only_with_allow_write() {
        assert_eq!(
            parse_benchmark_flags(&args(&["--allow-write"])).unwrap(),
            (false, false)
        );
        assert_eq!(
            parse_benchmark_flags(&args(&["--allow-write", "--allow-shell"])).unwrap(),
            (true, false)
        );
        assert_eq!(
            parse_benchmark_flags(&args(&["--allow-write", "--allow-network"])).unwrap(),
            (false, true)
        );
        assert_eq!(
            parse_benchmark_flags(&args(&[
                "--allow-write",
                "--allow-shell",
                "--allow-network"
            ]))
            .unwrap(),
            (true, true)
        );
    }

    #[test]
    fn axes_without_allow_write_fail_closed() {
        assert!(parse_benchmark_flags(&args(&["--allow-shell"])).is_err());
        assert!(parse_benchmark_flags(&args(&["--allow-network"])).is_err());
        assert!(parse_benchmark_flags(&args(&["--allow-shell", "--allow-network"])).is_err());
    }

    /// 2026-08-18 审查收口：`=value` 形式显式报错，绝不静默忽略。
    #[test]
    fn value_forms_are_explicit_errors() {
        assert!(parse_benchmark_flags(&args(&["--allow-write", "--allow-shell=1"])).is_err());
        assert!(parse_benchmark_flags(&args(&["--allow-write", "--allow-network=true"])).is_err());
        // 精确形式不受影响。
        assert_eq!(
            parse_benchmark_flags(&args(&["--allow-write", "--allow-shell"])).unwrap(),
            (true, false)
        );
    }
}
