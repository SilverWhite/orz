//! `orz-web` — the Web workbench bridge (0br S2).
//!
//! The bridge is a **local loopback server** that serves the static
//! three-draft UI (embedded assets) and transports ACP between the browser
//! and a `orz --stdio` child over WebSocket. orz stays a standard ACP
//! agent — the bridge owns transport only, never execution facts
//! (ADR-0010 §2.4 条 8 / §2.6 / §14.78 条 5).
//!
//! Design authority: `docs/UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md`,
//! survey `docs/audits/0BR_S1_WEB_FORM_SURVEY_2026-09-24.md` §5/§6.

pub mod acp_pump;
pub mod archives;
pub mod assets;
pub mod journal_tail;
pub mod runs;
pub mod security;
pub mod server;
pub mod trust;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

/// Bridge configuration for the `orz web` entry.
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// Bind target — must be loopback (`security::bind_addr_allowed`).
    pub bind: SocketAddr,
    /// Workspace directory (the child's cwd and the `.gsa` scan root).
    pub cwd: PathBuf,
    /// 批七（2026-09-25 用户报告 prompt 拿到 fake 回应）：真实模型传输。
    /// 置位时桥进程设置 `ORZ_REAL=1`，spawn 的 `orz --stdio` 子进程经
    /// env 继承走真实 DeepSeek 通道（与 TUI `--real` 同一决策点）。
    pub real: bool,
}

/// Default bind target: loopback, survey §6 port candidate.
pub const DEFAULT_BIND: &str = "127.0.0.1:21487";

/// Entry point for `orz web`: validate, build state, serve forever.
///
/// Returns a startup error (bind validation / listen failure); a `Ok(_)`
/// only happens when the server stops, which for this bridge means the
/// process is exiting.
pub async fn run(config: WebConfig) -> Result<(), String> {
    let token = security::generate_token();
    let agent_binary = std::env::current_exe()
        .map_err(|e| format!("无法定位自身二进制（桥需要 spawn `orz --stdio`）: {e}"))?;
    let bind: SocketAddr = server::validate_bind(config.bind).map_err(|e| e.to_string())?;

    // The URL fragment is the token channel: it never reaches tokenless
    // endpoints, and a cross-origin page can neither read our hash nor
    // pass the Host/Origin checks. The URL itself is printed by `serve`
    // once the ACTUAL port is known (`--addr …:0` picks an ephemeral one);
    // the frontend parses it as `#token=<hex>` — keep the two in lockstep.
    println!("orz web: Web 工作台已启动");
    // 批七：模型传输面必须一眼可辨——fake 回应（`(模型)(fake) 已收到请求`）
    // 对真实使用者毫无价值且极易误判为成功。
    if config.real {
        // SAFETY: run() 在任何 runtime/child spawn 之前执行，单线程窗口内
        // 写入（edition 2024 set_var unsafe，与 orz-bin main 的 --real 同型）。
        unsafe {
            std::env::set_var("ORZ_REAL", "1");
        }
        println!("orz web: 模型传输：真实（--real；子进程经 ORZ_REAL 继承）");
    } else if std::env::var_os("ORZ_REAL").is_some() {
        println!("orz web: 模型传输：真实（继承自调用方 ORZ_REAL）");
    } else {
        println!("orz web: ⚠ 模型传输：fake（测试替身，回应无意义）——真实模型请用 `orz web --real`");
    }
    // 批六（2026-09-25 用户报告「旧的归档消失了」）：从错误目录启动时
    // 桥扫的是那个目录的 .gsa——工作区必须印在启动台面上，一眼可辨。
    println!(
        "orz web: 工作区: {}（探索器/运行/归档均按此目录的 .gsa 投影；不符请用 --cwd 指向工作区根）",
        config.cwd.display()
    );
    // 批七（2026-09-25 用户报告 prompt「Internal error」）：ACAF 签名器
    // env 未随终端配置时，fail-closed 门会在首个 prompt 拒跑——启动台面
    // 直接挑明，不等用户撞墙。
    let missing: Vec<&str> = ["ORZ_ACAF_MANIFEST", "ORZ_ACAF_KEYSTORE", "ORZ_ACAF_BINARY"]
        .into_iter()
        .filter(|k| std::env::var(k).map(|v| v.trim().is_empty()).unwrap_or(true))
        .collect();
    if !missing.is_empty() {
        println!(
            "orz web: ⚠ ACAF 签名器 env 未配置（{}）——prompt 将被 fail-closed 拒绝；请从已 provision 的终端启动，或先设置三件 env（orz-acaf-provision 会回显）",
            missing.join(" / ")
        );
    }
    println!("orz web: 仅监听回环地址；Ctrl+C 退出（会一并结束桥接的 orz 会话）");

    let state = server::ServerState {
        token: Arc::new(token),
        cwd: Arc::new(config.cwd),
        agent_binary: Arc::new(agent_binary),
        session: acp_pump::SessionSlot::default(),
    };
    server::serve(state, bind).await.map_err(|e| e.to_string())
}

/// Parse `orz web` argv (the slice after `web`):
/// `--addr 127.0.0.1:21487` / `--cwd <dir>` / `--real`.
pub fn parse_args(args: &[String]) -> Result<WebConfig, String> {
    let mut bind = None;
    let mut cwd = std::env::current_dir().map_err(|e| format!("无法读取当前目录: {e}"))?;
    let mut real = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--addr" => {
                i += 1;
                bind = Some(
                    args.get(i)
                        .ok_or("--addr 需要一个 <ip:port> 值")?
                        .parse()
                        .map_err(|_| format!("--addr 值无效: {}", args[i]))?,
                );
            }
            "--cwd" => {
                i += 1;
                cwd = PathBuf::from(args.get(i).ok_or("--cwd 需要一个目录值")?);
            }
            "--real" => {
                real = true;
            }
            other => {
                return Err(format!(
                    "未知参数 {other}（用法: orz web [--addr ip:port] [--cwd dir] [--real]）"
                ));
            }
        }
        i += 1;
    }
    let bind = bind.unwrap_or_else(|| DEFAULT_BIND.parse().expect("default bind is valid"));
    Ok(WebConfig { bind, cwd, real })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_and_overrides() {
        let cfg = parse_args(&[]).unwrap();
        assert_eq!(cfg.bind.to_string(), DEFAULT_BIND);

        let cfg = parse_args(&[
            "--addr".into(),
            "127.0.0.1:30000".into(),
            "--cwd".into(),
            "D:/tmp".into(),
        ])
        .unwrap();
        assert_eq!(cfg.bind.to_string(), "127.0.0.1:30000");
        assert_eq!(cfg.cwd, PathBuf::from("D:/tmp"));

        assert!(parse_args(&["--addr".into()]).is_err());
        assert!(parse_args(&["--wat".into()]).is_err());
    }
}
