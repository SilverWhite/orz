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
pub mod keystore;
pub mod codex_app;
pub mod codex_permission;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol as acp;
use async_trait::async_trait;
use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::journal::JournalRecorder;
// NOTE: `PermitError` (orz_loop::host) is the LoopHost contract error; the
// assurance permit error is aliased to keep the two distinct.
use orz_assurance::permit::{
    PermitEnvelope, PermitError as PermitSigningError, PermitSigner, PermitStore,
    SensitiveActionPermit,
};
use orz_loop::host::{
    LoopHost, PermitDecision, PermitError, RiskClass, ToolError, ToolRegistry, ToolResult,
};

use crate::keystore::MemoryInstallationKeyStore;
use crate::permission::PermissionBridge;
use crate::tools::ToolsetRegistry;
use orz_workspace::permission::PermissionHookTransport;

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
    /// Keystore-backed P1 permit signer (DPAPI install key; test-only
    /// memory store when not injected). Consumed by `issue_permit`.
    permit_signer: Arc<dyn PermitSigner>,
    /// Root for permit artifacts — `{cwd}/.gsa/` (mirrors the Python
    /// namespace layout `<root>/one_shot_permit/`).
    permit_store_root: PathBuf,
    /// Live-client session id (ACP `session_notification` target for
    /// streamed text deltas). `None` = headless — deltas are no-ops.
    session_id: Option<String>,
    /// Live-client outbound gateway (clone; the original goes to the
    /// permission bridge). `None` = headless — deltas are no-ops.
    gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
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
            // Default: test-only memory signer (no permit is issued unless
            // `issue_permit` is called) — the session bootstrap injects the
            // DPAPI keystore-backed signer via `with_permit_signer`.
            permit_signer: Arc::new(MemoryInstallationKeyStore::new()),
            permit_store_root: cwd.join(".gsa"),
            session_id: None,
            gateway: None,
        })
    }

    /// Inject the session's keystore-backed permit signer (see
    /// `session::bootstrap_session`). Without injection the host signs with
    /// a fresh in-memory key per run.
    pub fn with_permit_signer(mut self, signer: Arc<dyn PermitSigner>) -> Self {
        self.permit_signer = signer;
        self
    }

    /// The active permit signer (for inspection / verification).
    pub fn permit_signer(&self) -> &Arc<dyn PermitSigner> {
        &self.permit_signer
    }

    /// Issue a P1 one-shot sensitive-action permit signed by the
    /// keystore-backed signer, persisted under `{cwd}/.gsa/one_shot_permit/`.
    ///
    /// `envelope` carries the session's conversation/envelope identity and
    /// expiry clamp (the session security envelope in production).
    ///
    /// KNOWN GAP (2026-08-05 review P2-1): the Python authority verifies the
    /// envelope before issuance (`verify_security_envelope`: schema +
    /// `installation_key_id` match + signature). orz has no security-envelope
    /// module yet, so the envelope is caller-supplied plain data — the
    /// approval path (still a stub) must verify envelope authenticity before
    /// calling this when it lands. Until then, issuing is only sound when the
    /// caller owns the envelope construction.
    #[allow(clippy::too_many_arguments)]
    pub fn issue_permit(
        &self,
        envelope: &PermitEnvelope,
        confirmation_sha256: &str,
        action_sha256: &str,
        target_sha256: &str,
        impact_scope_sha256: &str,
        attempt: u32,
        ttl_seconds: u64,
    ) -> Result<SensitiveActionPermit, PermitSigningError> {
        PermitStore::new(self.permit_store_root.clone()).issue(
            self.permit_signer.as_ref(),
            envelope,
            confirmation_sha256,
            action_sha256,
            target_sha256,
            impact_scope_sha256,
            attempt,
            ttl_seconds,
            None,
        )
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
        Self::with_bridge_and_hub(session_id, journal, cwd, workspace_trust, gateway, None)
    }

    /// `with_bridge` variant that additionally threads an interactive
    /// permission transport (`hub` — Phase 3 slice #12: the codex app-server
    /// approval surface). `None` keeps the ACP-gateway behavior; the rest is
    /// identical.
    pub fn with_bridge_and_hub(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
        hub: Option<Arc<dyn PermissionHookTransport>>,
    ) -> Result<Self, String> {
        // Clone the live-client sender before handing the original to the
        // bridge — the streamed text-delta path uses its own sender clone
        // (the permission manager gets the original).
        let live_gateway = gateway.clone();
        let bridge = PermissionBridge::spawn_with_hub(session_id, gateway, hub, cwd)?;
        let mut host = Self::with_permission(journal, cwd, workspace_trust, Some(bridge))?;
        host.session_id = Some(session_id.to_string());
        host.gateway = live_gateway;
        Ok(host)
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

    /// Forward a streamed model text chunk to the live ACP client as an
    /// `agent_message_chunk` notification — the TUI renders it incrementally
    /// into the current model card (streaming slice). Deltas are live-only,
    /// never journaled (Python `text_delta` precedent); headless hosts
    /// (no gateway) drop the chunk silently.
    fn on_text_delta(&self, text: &str) {
        let (Some(gateway), Some(session_id)) = (&self.gateway, &self.session_id) else {
            return;
        };
        let notification = acp::SessionNotification::new(
            acp::SessionId::new(session_id.clone()),
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(
                acp::ContentBlock::Text(acp::TextContent::new(text.to_string())),
            )),
        );
        // Fire-and-forget: the `acp::Agent::session_notification` trait method
        // is `#[async_trait(?Send)]` (its future is not Send), so it cannot be
        // awaited inside this Send-bound LoopHost impl — `forward_fire_and_forget`
        // is the documented bypass. A dropped receiver discards the chunk.
        if !gateway.forward_fire_and_forget(notification) {
            tracing::debug!("on_text_delta: gateway receiver dropped, chunk discarded");
        }
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

    /// P1 permit keystore injection: a permit issued through the host signs
    /// with the injected keystore signer and self-verifies; the artifact
    /// lands under `{cwd}/.gsa/one_shot_permit/` (Python namespace layout).
    #[tokio::test]
    async fn issue_permit_uses_injected_keystore_signer() {
        use orz_assurance::permit::verify_sensitive_action_permit;

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted)
            .expect("host build")
            .with_permit_signer(Arc::new(
                MemoryInstallationKeyStore::from_secret(&[0x42u8; 32]).unwrap(),
            ));

        let envelope = PermitEnvelope {
            conversation_id: "conv-1".into(),
            envelope_id: "env-1".into(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        };
        let digest = |s: &str| orz_assurance::sha256_hex(s.as_bytes());
        let permit = host
            .issue_permit(
                &envelope,
                &digest("confirmation"),
                &digest("action"),
                &digest("target"),
                &digest("impact scope"),
                1,
                300,
            )
            .expect("issue permit");

        assert!(
            permit.integrity.key_id.starts_with("KEY-"),
            "key id: {}",
            permit.integrity.key_id
        );
        let verification = verify_sensitive_action_permit(
            host.permit_signer().as_ref(),
            &envelope,
            &permit,
            None,
        );
        assert!(verification.valid, "{:?}", verification.errors);
        let artifact = dir
            .join(".gsa")
            .join("one_shot_permit")
            .join(format!("{}.issued.json", permit.permit_id));
        assert!(artifact.is_file(), "missing permit artifact");

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

    /// Streaming slice: `on_text_delta` forwards the chunk to the live
    /// client as an `agent_message_chunk` session notification, in order.
    #[tokio::test]
    async fn on_text_delta_forwards_session_notification_to_gateway() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let (tx, mut rx) =
                    tokio::sync::mpsc::unbounded_channel::<xai_acp_lib::AcpClientMessage>();
                let sender = xai_acp_lib::AcpAgentGatewaySender::new(tx);
                let host = OrzHost::with_bridge(
                    "sess-1",
                    JournalRecorder::new(dir.join("j")),
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                    Some(sender),
                )
                .expect("host with bridge");

                host.on_text_delta("你好");
                host.on_text_delta("世界");

                for expected in ["你好", "世界"] {
                    let msg = rx.recv().await.expect("notification");
                    match msg {
                        xai_acp_lib::AcpClientMessage::SessionNotification(args) => {
                            assert_eq!(args.request.session_id.0.as_ref(), "sess-1");
                            match args.request.update {
                                acp::SessionUpdate::AgentMessageChunk(chunk) => {
                                    match chunk.content {
                                        acp::ContentBlock::Text(t) => {
                                            assert_eq!(t.text, expected);
                                        }
                                        other => panic!("unexpected content block: {other:?}"),
                                    }
                                }
                                other => panic!("unexpected update: {other:?}"),
                            }
                        }
                        other => panic!("unexpected message: {other:?}"),
                    }
                }

                // Headless host (no gateway): on_text_delta is a silent no-op.
                let headless = OrzHost::new(
                    JournalRecorder::new(dir.join("j2")),
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                )
                .expect("headless host");
                headless.on_text_delta("丢弃");

                let _ = std::fs::remove_dir_all(&dir);
            })
            .await
    }
}
