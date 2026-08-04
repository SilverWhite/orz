//! Permission bridge — Grok permission manager behind the LoopHost contract.
//!
//! IP6 (hard-gate @ orz-workspace::permission) is the single allowed
//! assurance injection into kept Grok providers. This bridge walks public
//! seams only:
//! - yolo stays off: headless spawn with `initial_yolo = false`, no auto
//!   classifier — no way to sneak past the decision point;
//! - interactive prompting (`PermissionHookTransport`) lands with the TUI
//!   (Phase 3); headless `Ask` fails closed to `Deny`;
//! - every decision is journaled here (GateDecision / PermissionDecision
//!   events are written by the caller/controller).

use std::sync::Arc;

use agent_client_protocol as acp;
use agent_client_protocol::{ToolCallId, ToolCallUpdate, ToolCallUpdateFields};
use orz_assurance::journal::JournalRecorder;
use orz_loop::host::{PermitDecision, PermitError, RiskClass};
use orz_workspace::permission::{
    spawn_permission_manager_with_hub, AccessKind, ClientType, Decision, PermissionHandle,
};
use xai_acp_lib::AcpAgentGatewaySender;

/// Grok permission manager wrapped for the LoopHost contract.
pub struct PermissionBridge {
    handle: PermissionHandle,
    journal: JournalRecorder,
}

impl PermissionBridge {
    /// Spawn the Grok permission manager over the ACP outbound gateway.
    ///
    /// `gateway` comes from the stdio server (`AcpServer::gateway()`); with a
    /// dropped receiver the interactive prompt path fails closed.
    pub fn spawn(
        session_id: &str,
        gateway: AcpAgentGatewaySender,
        cwd: &std::path::Path,
        journal: JournalRecorder,
    ) -> Result<Self, String> {
        let abs_cwd = orz_paths::AbsPathBuf::new(cwd.to_path_buf())
            .map_err(|e| format!("cwd must be absolute: {e}"))?;
        let (handle, _events) = spawn_permission_manager_with_hub(
            acp::SessionId::new(session_id.to_string()),
            gateway,
            abs_cwd,
            ClientType::Generic,
            None, // no managed rules — prompt policy decides; headless Ask → Deny
            Vec::new(), // deny_read_globs
            Vec::new(), // web_fetch_allowed_domains
            false,      // initial_yolo — headless: yolo never on (IP6)
            None,       // client_identifier
            false,      // remember_tool_approvals
            None,       // hub_permission — interactive prompter lands Phase 3
        );
        Ok(Self { handle, journal })
    }

    /// Request permission for a tool call (LoopHost `request_permission`).
    pub async fn request(
        &self,
        _risk: RiskClass,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<PermitDecision, PermitError> {
        let access = access_kind(tool, args);
        let update = ToolCallUpdate::new(
            ToolCallId::new(format!("call-{tool}")),
            ToolCallUpdateFields::new(),
        );
        let decision = self
            .handle
            .request(access, update, None, None, None)
            .await;
        Ok(match decision {
            Decision::Allow => PermitDecision::AllowOnce,
            // Headless: an interactive prompt has no client to answer it —
            // fail closed rather than stall or guess.
            Decision::Ask | Decision::FollowupMessage(_) => PermitDecision::Deny,
            Decision::Reject(_) | Decision::PolicyDeny(_) | Decision::Cancelled => {
                PermitDecision::Deny
            }
        })
    }
}

/// Mechanical tool-name → AccessKind mapping for the permission manager.
/// MCP names contain the `__` server separator; everything else maps by
/// GrokBuild client name (run_terminal_cmd / read_file / grep / web_*).
fn access_kind(tool: &str, args: &serde_json::Value) -> AccessKind {
    if tool.contains("__") {
        AccessKind::MCPTool {
            name: tool.to_string(),
            input: args.clone(),
        }
    } else if tool == "run_terminal_cmd" {
        AccessKind::Bash(
            args.get("command")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string(),
        )
    } else if tool == "read_file" {
        AccessKind::Read(
            args.get("target_file")
                .or_else(|| args.get("path"))
                .and_then(|f| f.as_str())
                .map(str::to_string),
        )
    } else if tool == "list_dir" {
        // Grok Build list_dir parameter is `target_directory` (not
        // `target_file`) — without it the mapped path is always None and
        // deny_read_globs matching is lost.
        AccessKind::Read(
            args.get("target_directory")
                .or_else(|| args.get("target_file"))
                .or_else(|| args.get("path"))
                .and_then(|f| f.as_str())
                .map(str::to_string),
        )
    } else if tool == "grep" {
        AccessKind::Grep {
            path: args
                .get("path")
                .and_then(|p| p.as_str())
                .map(str::to_string),
            glob: None,
        }
    } else if tool == "web_search" {
        AccessKind::WebSearch(
            args.get("query")
                .and_then(|q| q.as_str())
                .unwrap_or_default()
                .to_string(),
        )
    } else if tool == "web_fetch" {
        AccessKind::WebFetch(
            args.get("url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string(),
        )
    } else {
        AccessKind::Edit(format!("{tool}: {args}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-permission-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A gateway whose receiver is dropped immediately: the interactive
    /// prompt path fails closed (permission manager's SendFailed).
    fn dead_gateway() -> AcpAgentGatewaySender {
        xai_acp_lib::AcpGatewaySender::new(tokio::sync::mpsc::unbounded_channel().0)
    }

    /// The permission manager actor spawns local tasks — everything runs
    /// inside one LocalSet per test.
    async fn with_bridge<F, Fut, T>(f: F) -> T
    where
        F: FnOnce(PermissionBridge) -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let journal = JournalRecorder::new(dir.join("j"));
                let bridge =
                    PermissionBridge::spawn("sess-test", dead_gateway(), &dir, journal).unwrap();
                f(bridge).await
            })
            .await
    }

    #[tokio::test]
    async fn ask_with_no_client_fails_closed() {
        // Bash (SandboxEscape) has no allow rule and no interactive client →
        // headless Ask → Deny (fail-closed).
        let decision = with_bridge(|b| async move {
            b.request(
                RiskClass::SandboxEscape,
                "run_terminal_cmd",
                &serde_json::json!({"command": "dir"}),
            )
            .await
            .unwrap()
        })
        .await;
        assert_eq!(decision, PermitDecision::Deny, "headless Ask must fail closed");
    }

    #[tokio::test]
    async fn read_request_auto_allows() {
        // Read-only access is auto-approved by the permission manager
        // (low-risk reads never prompt); Bash without an interactive client
        // fails closed. Risk layering is exactly the point of IP6.
        let decision = with_bridge(|b| async move {
            b.request(
                RiskClass::ReadOnly,
                "read_file",
                &serde_json::json!({"target_file": "a.txt"}),
            )
            .await
            .unwrap()
        })
        .await;
        assert_eq!(decision, PermitDecision::AllowOnce);
    }

    #[test]
    fn access_kind_mapping() {
        assert!(matches!(
            access_kind("read_file", &serde_json::json!({"target_file": "a"})),
            AccessKind::Read(Some(_))
        ));
        assert!(matches!(
            access_kind("run_terminal_cmd", &serde_json::json!({"command": "ls"})),
            AccessKind::Bash(_)
        ));
        assert!(matches!(
            access_kind("web_search", &serde_json::json!({"query": "x"})),
            AccessKind::WebSearch(_)
        ));
        assert!(matches!(
            access_kind("server__tool", &serde_json::json!({})),
            AccessKind::MCPTool { .. }
        ));
    }
}
