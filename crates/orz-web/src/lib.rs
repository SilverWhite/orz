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
/// `--addr 127.0.0.1:21487` / `--cwd <dir>`.
pub fn parse_args(args: &[String]) -> Result<WebConfig, String> {
    let mut bind = None;
    let mut cwd = std::env::current_dir().map_err(|e| format!("无法读取当前目录: {e}"))?;
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
            other => {
                return Err(format!(
                    "未知参数 {other}（用法: orz web [--addr ip:port] [--cwd dir]）"
                ));
            }
        }
        i += 1;
    }
    let bind = bind.unwrap_or_else(|| DEFAULT_BIND.parse().expect("default bind is valid"));
    Ok(WebConfig { bind, cwd })
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
