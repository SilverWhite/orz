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
use orz_loop::host::{LoopHost, ToolError, ToolRegistry, ToolResult};

use crate::tools::ToolsetRegistry;

/// Full LoopHost implementation over the Grok providers.
///
/// Wires the journal, the finalized toolset, and the workspace-trust
/// observation into the orz-loop contract. Permission, persistence, hooks,
/// MCP, and credentials remain default stubs in Phase 2 (permission bridge
/// lands with IP6; chat-state/hooks wiring in Phase 3).
pub struct OrzHost {
    journal: JournalRecorder,
    registry: ToolsetRegistry,
    workspace_trust: WorkspaceTrust,
}

impl OrzHost {
    /// Build a host for a session working directory.
    ///
    /// `workspace_trust` comes from the session bootstrap (see
    /// `session::check_workspace_trust`).
    pub fn new(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
    ) -> Result<Self, String> {
        let toolset = tools::build_toolset(cwd)?;
        Ok(Self {
            journal,
            registry: ToolsetRegistry::new(toolset),
            workspace_trust,
        })
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

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::from_texts(vec!["完成"]));
        let controller = AgentLoopController::with_gateway(gateway);
        let (response, _, _) = controller
            .run_turn(&host, "hi", "RUN-T", "manifest-sha", 0, None)
            .await
            .expect("run turn");
        assert_eq!(response, "完成");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
