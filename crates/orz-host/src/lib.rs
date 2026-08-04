//! orz-host — Thin core that bridges Grok providers to the self-built agent loop.
//!
//! Phase 0: Skeleton only — compiles against kept Grok providers.
//! Phase 1+: ACP server, LoopHost implementation, session lifecycle, journal writer.
//!
//! Dependency direction (Codex discipline):
//!   orz-host → {orz-loop, orz-assurance, Grok providers}
//!   No Grok crate depends on orz-host.

pub mod acp_server;
pub mod session;
pub mod approval;
pub mod stdio;
pub mod tools;
pub mod permission;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::journal::JournalRecorder;
use orz_loop::host::{LoopHost, PermitDecision, PermitError, RiskClass, ToolError, ToolRegistry, ToolResult};

use crate::permission::PermissionBridge;
use crate::tools::ToolsetRegistry;

/// Full LoopHost implementation over the Grok providers.
///
/// Wires the journal, the finalized toolset, the workspace-trust observation,
/// and the IP6 permission bridge into the orz-loop contract. Persistence,
/// hooks, MCP, and credentials remain default stubs (chat-state/hooks wiring
/// in Phase 3).
pub struct OrzHost {
    journal: JournalRecorder,
    registry: ToolsetRegistry,
    workspace_trust: WorkspaceTrust,
    permission: Option<PermissionBridge>,
}

impl OrzHost {
    /// Build a host for a session working directory.
    ///
    /// `workspace_trust` comes from the session bootstrap (see
    /// `session::check_workspace_trust`). No permission bridge — every
    /// `request_permission` fails closed to `Deny` (LoopHost default,
    /// review P1-1: a host that forgets to wire the bridge must not
    /// silently auto-allow `bash`).
    pub fn new(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
    ) -> Result<Self, String> {
        Self::with_permission(journal, cwd, workspace_trust, None)
    }

    /// Build a host with an optional IP6 permission bridge.
    ///
    /// `Some(bridge)` delegates `request_permission` to the Grok permission
    /// manager (Read auto-allow; headless Ask → Deny); `None` keeps the
    /// fail-closed default.
    pub fn with_permission(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        permission: Option<PermissionBridge>,
    ) -> Result<Self, String> {
        let toolset = tools::build_toolset(cwd)?;
        Ok(Self {
            journal,
            registry: ToolsetRegistry::new(toolset),
            workspace_trust,
            permission,
        })
    }

    /// Build a host with an IP6 permission bridge in one step.
    ///
    /// `gateway` is the outbound ACP sender when interactive (`--stdio`);
    /// `None` (headless `-p`/`--plan`) substitutes a dead gateway — Read
    /// auto-allows, `Ask` fails closed to `Deny`.
    ///
    /// Note: `PermissionBridge::spawn` runs the manager actor via
    /// `spawn_local`, so callers must be inside a `tokio::task::LocalSet`.
    pub fn with_bridge(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
    ) -> Result<Self, String> {
        let bridge = PermissionBridge::spawn(session_id, gateway, cwd, journal.clone())?;
        Self::with_permission(journal, cwd, workspace_trust, Some(bridge))
    }

    /// The underlying finalized toolset (for direct dispatch).
    pub fn toolset(&self) -> &Arc<orz_tools::registry::types::FinalizedToolset> {
        self.registry.toolset()
    }
}

#[async_trait]
impl LoopHost for OrzHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }

    fn tools_registry(&self) -> &dyn ToolRegistry {
        &self.registry
    }

    fn workspace_trust(&self) -> WorkspaceTrust {
        self.workspace_trust
    }

    async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
        call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        let result = self
            .registry
            .toolset()
            .call(name, args, call_id, None)
            .await
            .map_err(|e| crate::tools::map_tool_error(&e))?;
        Ok(ToolResult {
            output: result.prompt_text,
            exit_code: None,
        })
    }

    async fn request_permission(
        &self,
        risk: RiskClass,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<PermitDecision, PermitError> {
        match &self.permission {
            Some(bridge) => bridge.request(risk, tool, args).await,
            // No bridge wired → fail closed, never auto-allow (review P1-1).
            None => Ok(PermitDecision::Deny),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_loop::gateway::fake::FakeProvider;
    use orz_loop::gateway::model::ModelGateway;
    use orz_loop::AgentLoopController;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-host-lib-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Toolset construction is expensive (builder finalize) — share one.
    static SHARED_TOOLSET: OnceLock<Arc<orz_tools::registry::types::FinalizedToolset>> =
        OnceLock::new();

    fn shared_toolset() -> &'static Arc<orz_tools::registry::types::FinalizedToolset> {
        SHARED_TOOLSET.get_or_init(|| {
            tools::build_toolset(&std::env::temp_dir()).expect("shared toolset")
        })
    }

    #[tokio::test]
    async fn registry_lists_builtin_tools() {
        let toolset = shared_toolset();
        let names: Vec<String> = toolset
            .tool_definitions()
            .iter()
            .map(|d| d.function.name.clone())
            .collect();
        assert!(
            names.iter().any(|n| n == "read_file"),
            "expected read_file in builtin tools, got {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "run_terminal_cmd"),
            "expected run_terminal_cmd (GrokBuild bash), got {names:?}"
        );
    }

    #[tokio::test]
    async fn call_read_file_returns_content() {
        let dir = test_dir();
        let path = dir.join("test.txt");
        std::fs::write(&path, "hello orz tools").unwrap();

        let toolset = shared_toolset();
        let result = toolset
            .call(
                "read_file",
                serde_json::json!({"target_file": path}),
                "call-1",
                None,
            )
            .await
            .expect("read_file call");
        assert!(result.prompt_text.contains("hello orz tools"), "{result:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn orz_host_full_loophost_chain() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join(".gsa").join("runs").join("RUN-T"));
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted)
            .expect("host build");

        // Two texts — the counterexample gate (§4.6) intercepts the first.
        let gateway: Arc<dyn ModelGateway> =
            Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let controller = AgentLoopController::with_gateway(gateway);
        let (response, _, _) = controller
            .run_turn(&host, "hi", "RUN-T", "manifest-sha", 0, None)
            .await
            .expect("run turn");
        assert_eq!(response, "完成");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3 wiring: a host WITHOUT a permission bridge must fail closed —
    /// never auto-allow (review P1-1).
    #[tokio::test]
    async fn host_without_bridge_fails_closed_on_permission() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted).expect("host");

        for (risk, tool) in [
            (RiskClass::ReadOnly, "read_file"),
            (RiskClass::SandboxEscape, "bash"),
        ] {
            let decision = host
                .request_permission(risk, tool, &serde_json::json!({}))
                .await
                .expect("request_permission");
            assert_eq!(
                decision,
                PermitDecision::Deny,
                "no bridge → fail closed for {tool}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3 wiring: host WITH the bridge — Read auto-allows through the
    /// permission manager, Bash (no interactive client) Ask → Deny (IP6).
    #[tokio::test]
    async fn host_with_bridge_auto_allows_read_denies_bash() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let journal = JournalRecorder::new(dir.join("j"));
                let host = OrzHost::with_bridge(
                    "sess-1",
                    journal,
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                    None, // headless dead gateway
                )
                .expect("host with bridge");

                let read = host
                    .request_permission(
                        RiskClass::ReadOnly,
                        "read_file",
                        &serde_json::json!({"target_file": "a.txt"}),
                    )
                    .await
                    .expect("read request");
                assert_eq!(read, PermitDecision::AllowOnce);

                let bash = host
                    .request_permission(
                        RiskClass::SandboxEscape,
                        "run_terminal_cmd",
                        &serde_json::json!({"command": "dir"}),
                    )
                    .await
                    .expect("bash request");
                assert_eq!(bash, PermitDecision::Deny, "headless Ask → Deny");

                let _ = std::fs::remove_dir_all(&dir);
            })
            .await
    }
}
