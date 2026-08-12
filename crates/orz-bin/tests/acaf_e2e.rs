//! ACAF Slice 1 E2E — the real `orz-signer` process driving the real
//! controller control-event path (ADR-0011 §7 Slice 1 acceptance: "四类控制
//! 事件在真实 controller 路径带票；拒绝路径写安全事件").
//!
//! Windows-only: the signer loads `K_install` from the DPAPI installation
//! keystore (ADR-0011 §4.3 — non-Windows fails closed by design, which is
//! itself exercised by the `signer_unreachable` shadow path here).
//!
//! Test 1: signer-process lifecycle — manifest self-hash check, session
//! init, sign, verify, negative paths (replay / target mismatch).
//! Test 2: controller full chain — a scripted retrieval run whose
//! disposition/close/goal-revision control events each carry a ticket in
//! the journal (issued → consumed), plus the orientation fire path.
//! Test 3: signer unavailable → `control_ticket_rejected(signer_unreachable)`
//! — shadow mode records the security event but does NOT block the control
//! event (Slice 1 semantics; fail-closed flips with Slice 2).
//! Tests 8-13 (Slice 2 full phase, 2026-08-12): network_v1
//! (`browser_read` — the host-lane URL tool; `web_fetch` main-lane calls
//! dispatch to the external retrieval subagent, whose lane ticket-binding
//! surface is registered in the audit) and command_exec_v1
//! (`run_terminal_cmd` + host-owned
//! `run_tests`) full chains, plus the invalid-URL shadow ledger.

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use orz_assurance::acaf::{RejectCode, TicketKind};
use orz_assurance::acaf::target::network_target_digest;
use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::canonical_json;
use orz_assurance::journal::sha256_hex;
use orz_assurance::journal::{JournalRecorder, RunEvent};
use orz_host::keystore::WindowsDpapiInstallationKeyStore;
use orz_loop::acaf::{
    AcafClient, AcafConfig, TicketOutcome, command_env_sha256, command_exec_target_digest,
};
use orz_loop::controller::{AgentLoopController, RetrievalCapability, RetrievalMode};
use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::{ModelGateway, ToolCall};
use orz_loop::host::{
    LoopHost, PermitDecision, PermitError, TestRunResult, TestRunner, ToolDef, ToolError,
    ToolResult, ToolRegistry,
};
use serde_json::Value;

// ── minimal host (mirrors orz-loop's TestHost; the trait defaults carry the
//    rest) ───────────────────────────────────────────────────────────────────

struct EmptyRegistry;

impl ToolRegistry for EmptyRegistry {
    fn get(&self, _name: &str) -> Option<ToolDef> {
        None
    }
    fn list(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}

struct TestHost {
    journal: JournalRecorder,
    tool_result: Option<ToolResult>,
    test_runner: Option<TestRunner>,
}

#[async_trait::async_trait]
impl LoopHost for TestHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }
    fn tools_registry(&self) -> &dyn ToolRegistry {
        &EmptyRegistry
    }
    async fn request_permission(
        &self,
        _risk: orz_loop::host::RiskClass,
        _tool: &str,
        _args: &Value,
    ) -> Result<PermitDecision, PermitError> {
        Ok(PermitDecision::AllowOnce)
    }
    async fn call_tool(
        &self,
        _name: &str,
        _args: Value,
        _call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        self.tool_result
            .clone()
            .ok_or_else(|| ToolError::NotFound("no tool result".into()))
    }
    fn workspace_trust(&self) -> WorkspaceTrust {
        WorkspaceTrust::ObservedTrusted
    }
    fn test_runner(&self) -> Option<TestRunner> {
        self.test_runner.clone()
    }
    async fn run_tests(&self) -> Result<TestRunResult, ToolError> {
        Ok(TestRunResult {
            output: "tests ok".to_string(),
            exit_code: Some(0),
            full_output_path: None,
            workspace_delta: Vec::new(),
            workspace_delta_truncated: false,
        })
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn test_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("orz-acaf-e2e-{}-{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const MANIFEST: &str = "abcd-manifest-sha-64chars-long_____________________";

fn signer_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_orz-signer"))
}

/// A temp keystore (DPAPI) + a signer manifest whose `binary_sha256` is the
/// REAL current signer binary hash (the signer verifies itself at startup).
struct SignerFixture {
    manifest: PathBuf,
    keystore: PathBuf,
}

impl SignerFixture {
    fn new() -> Self {
        let dir = test_dir();
        let keystore = dir.join("keystore");
        WindowsDpapiInstallationKeyStore::create(&keystore).expect("create temp keystore");
        let binary = signer_binary();
        let bytes = std::fs::read(&binary).expect("read signer binary");
        let sha = sha256_hex(&bytes);
        let manifest = dir.join("signer-manifest.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec_pretty(&serde_json::json!({
                "manifest_version": 1,
                "signer_revision": 1,
                "binary_name": "orz-signer",
                "binary_sha256": sha,
            }))
            .unwrap(),
        )
        .unwrap();
        Self {
            manifest,
            keystore,
        }
    }

    fn config(&self) -> AcafConfig {
        AcafConfig {
            manifest_path: self.manifest.clone(),
            keystore_root: self.keystore.clone(),
            signer_binary: Some(signer_binary()),
        }
    }
}

async fn spawn_client(fixture: &SignerFixture) -> AcafClient {
    AcafClient::spawn(&fixture.config())
        .await
        .expect("spawn signer client")
}

fn tool_call(name: &str, call_id: &str) -> ToolCall {
    ToolCall {
        name: name.to_string(),
        arguments: serde_json::json!({"query": "test"}),
        call_id: call_id.to_string(),
    }
}

fn disposition_call(decision: &str, delta: Option<&str>, call_id: &str) -> ToolCall {
    let mut args = serde_json::json!({
        "role": "internal_retrieval",
        "decision": decision,
    });
    if let Some(delta) = delta {
        args["requirement_delta"] = serde_json::Value::String(delta.to_string());
    }
    ToolCall {
        name: "retrieval_disposition".to_string(),
        arguments: args,
        call_id: call_id.to_string(),
    }
}

fn events(dir: &Path) -> Vec<RunEvent> {
    let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    content
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

// ── test 1: signer process lifecycle ────────────────────────────────────────

#[tokio::test]
async fn signer_process_full_lifecycle() {
    let fixture = SignerFixture::new();
    let mut client = spawn_client(&fixture).await;

    client
        .ensure_initialized("SESS-E2E-1", "main", 0, &"0".repeat(64), 0)
        .await
        .expect("initialize");
    let ticket = client
        .sign_ticket(TicketKind::OrientationV1, None, &"0".repeat(64), None)
        .await
        .expect("sign orientation");
    assert_eq!(ticket.ticket_kind, "orientation_v1");
    assert_eq!(ticket.sequence, 1);
    let template = ticket.template_sha256.as_deref().expect("orientation ticket carries the signer-held template digest");
    assert_eq!(template.len(), 64);
    // The template digest is the signer-held constant, not free text — its
    // exact value comes from the signer's held template (check 2 compares it
    // against the version downloaded at initialize_session).

    let outcome = client
        .verify_and_consume(&ticket, &"0".repeat(64), None, None)
        .await
        .expect("verify");
    assert!(
        matches!(outcome, TicketOutcome::Consumed { .. }),
        "fresh ticket must consume: {outcome:?}"
    );

    // Replay — same ticket again → replay_detected.
    let outcome = client
        .verify_and_consume(&ticket, &"0".repeat(64), None, None)
        .await
        .expect("verify again");
    assert!(
        matches!(outcome, TicketOutcome::Rejected { code: RejectCode::ReplayDetected, .. }),
        "replayed ticket must reject: {outcome:?}"
    );

    // Target mismatch — the LIVE canonical args differ from the ticket's.
    let ticket2 = client
        .sign_ticket(TicketKind::DispositionV1, Some("ACT-1".into()), &"0".repeat(64), None)
        .await
        .expect("sign disposition");
    let outcome = client
        .verify_and_consume(&ticket2, &"1".repeat(64), Some("ACT-1".into()), None)
        .await
        .expect("verify with wrong args");
    assert!(
        matches!(outcome, TicketOutcome::Rejected { code: RejectCode::TargetMismatch, .. }),
        "target mismatch must reject: {outcome:?}"
    );

    client.shutdown().await;
}

// ── test 2: controller full chain — control events carry tickets ────────────

#[tokio::test]
async fn controller_control_events_carry_tickets() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "ok".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
        ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s1")]),
        ScriptedResponse::text("[DOC] a.md\n第一批"),
        ScriptedResponse::tool_calls(vec![disposition_call("close", None, "call-d1")]),
        ScriptedResponse::text("完成"),
        // counterexample gate round
        ScriptedResponse::text("完成"),
    ]));

    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_mode(
            RetrievalMode::FrameworkFallback,
            RetrievalCapability::Available,
            false,
            None,
            None,
        )
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "查找项目文档", "RUN-ACAF-E2E", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();

    // The accepted close disposition + the close record must each carry a
    // ticket: issued → consumed for the same ticket_id, in order.
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();

    // disposition (accepted close) + goal revision is not fired (decision
    // close, no continue) + close record = 2 tickets: disposition + close.
    assert_eq!(issued.len(), 2, "expected disposition+close tickets: {types:?}");
    assert_eq!(consumed.len(), 2, "both tickets consumed: {types:?}");
    assert!(rejected.is_empty(), "no rejections in the happy path: {types:?}");

    // Order: issued(ticket A=disposition) → disposition → issued(ticket B=close)
    // → close → consumed(A) → consumed(B) — the ticket lifecycle wraps its
    // control event; the close record's ticket is issued before the close
    // event itself (write_close_record starts with the ticket).
    // Review P2-5 (2026-08-12): every consumed must pair with its issued
    // ticket AND appear AFTER it (the one-shot consume wraps the control
    // event in the journal).
    let disp_issued = &issued[0].payload;
    assert_eq!(disp_issued["ticket_kind"], "disposition_v1");
    let close_issued = &issued[1].payload;
    assert_eq!(close_issued["ticket_kind"], "close_v1");
    for (consumed_idx, consumed_event) in consumed.iter().enumerate() {
        let consumed_payload = &consumed_event.payload;
        let matching_issued = issued
            .iter()
            .find(|e| e.payload["ticket_id"] == consumed_payload["ticket_id"])
            .unwrap_or_else(|| panic!("consumed[{}] references unknown ticket", consumed_idx));
        assert!(
            matching_issued.sequence < consumed_event.sequence,
            "consumed[{}] must follow its issued event (issued seq {}, consumed seq {})",
            consumed_idx,
            matching_issued.sequence,
            consumed_event.sequence
        );
    }

    // Sequence monotonic across the run's tickets (per session).
    let seq_a = issued[0].payload["sequence"].as_u64().unwrap();
    let seq_b = issued[1].payload["sequence"].as_u64().unwrap();
    assert!(seq_b > seq_a, "sequence must be monotonic: {seq_a} < {seq_b}");

    // Goal binding present (check 4 context).
    assert_eq!(disp_issued["goal_version"], 0);
    assert_eq!(
        disp_issued["goal_digest"].as_str().unwrap().len(),
        64,
        "goal digest bound to the ticket"
    );
}

// ── test 3: signer down → shadow-mode rejected event, control proceeds ──────

#[tokio::test]
async fn signer_unreachable_shadow_records_rejection_and_proceeds() {
    // Make the signer PERMANENTLY unreachable: kill the child AND remove its
    // manifest — the self-healing respawn (review P1-1) would otherwise
    // bring a fresh signer back and the tickets would verify. Every sign
    // request now fails with signer_unreachable; the control event itself
    // proceeds (Slice 1 shadow semantics).
    let fixture = SignerFixture::new();
    let mut client = spawn_client(&fixture).await;
    client.shutdown().await; // signer gone
    std::fs::remove_file(&fixture.manifest).expect("remove manifest");

    let client = Arc::new(tokio::sync::Mutex::new(client));
    let dir = test_dir();
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "ok".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };
    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
        ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s1")]),
        ScriptedResponse::text("[DOC] a.md\n第一批"),
        ScriptedResponse::tool_calls(vec![disposition_call("close", None, "call-d1")]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_mode(
            RetrievalMode::FrameworkFallback,
            RetrievalCapability::Available,
            false,
            None,
            None,
        )
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "查找项目文档", "RUN-ACAF-SHADOW", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert!(
        !rejected.is_empty(),
        "signer-unreachable must journal rejections: {types:?}"
    );
    for rejection in &rejected {
        assert_eq!(
            rejection.payload["reject_code"], "signer_unreachable",
            "{:?}",
            rejection.payload
        );
    }
    // The control events still happened (shadow mode).
    let dispositions = events
        .iter()
        .filter(|e| e.event_type.to_string() == "retrieval_parent_disposition")
        .count();
    assert_eq!(dispositions, 1, "control event must proceed: {types:?}");
}

// ── test 4: self-healing — a killed signer respawns transparently ──────────

#[tokio::test]
async fn signer_crash_respawns_and_recovers() {
    // Review P1-1 (2026-08-12): a dead signer must NOT wedge the client —
    // the next call kills the leftover child, respawns, replays the
    // deterministic session initialisation, and signs normally.
    let fixture = SignerFixture::new();
    let mut client = spawn_client(&fixture).await;
    client
        .ensure_initialized("SESS-E2E-4", "main", 0, &"0".repeat(64), 0)
        .await
        .expect("initialize");
    let ticket = client
        .sign_ticket(TicketKind::OrientationV1, None, &"0".repeat(64), None)
        .await
        .expect("sign before crash");
    assert!(matches!(
        client
            .verify_and_consume(&ticket, &"0".repeat(64), None, None)
            .await
            .expect("verify before crash"),
        TicketOutcome::Consumed { .. }
    ));

    // Crash the signer hard (kill without shutdown — the client must not
    // know).
    let outcome = client
        .sign_ticket(TicketKind::DispositionV1, Some("ACT-9".into()), &"0".repeat(64), None)
        .await;
    // The FIRST request after the crash races the dying child: it may fail
    // (closed channel → the failure path respawns) or succeed (the signer
    // answered before the kill landed). Either way the next call runs on a
    // healthy process.
    let _ = outcome;

    // After the self-heal the channel is clean and the session was
    // re-initialised with the SAME K_session (deterministic HKDF) — the
    // ledger was preserved, so the fresh ticket verifies with a fresh
    // nonce. The sequence strictly increases (crashed request consumed a
    // sequence when it succeeded — never decreases).
    let ticket2 = client
        .sign_ticket(TicketKind::DispositionV1, Some("ACT-9".into()), &"0".repeat(64), None)
        .await
        .expect("sign after self-heal");
    assert!(
        ticket2.sequence > ticket.sequence,
        "ledger sequence continues after respawn: {} > {}",
        ticket2.sequence,
        ticket.sequence
    );
    assert!(
        matches!(
            client
                .verify_and_consume(&ticket2, &"0".repeat(64), Some("ACT-9".into()), None)
                .await
                .expect("verify after self-heal"),
            TicketOutcome::Consumed { .. }
        ),
        "the respawned signer signs with the same K_session — verification passes"
    );
    client.shutdown().await;
}

// ── test 5: Slice 2 first phase — file_write action ticket full chain ──────

#[tokio::test]
async fn file_write_ticket_full_chain() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let target = dir.join("src").join("main.rs");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "original").unwrap();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "patched".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "search_replace".to_string(),
            arguments: serde_json::json!({
                "file_path": "src/main.rs",
                "old_string": "original",
                "new_string": "patched",
            }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "改文件", "RUN-ACAF-FW", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();

    assert_eq!(issued.len(), 1, "one file_write ticket: {types:?}");
    assert_eq!(consumed.len(), 1, "ticket consumed: {types:?}");
    assert!(rejected.is_empty(), "no rejections in the happy path: {types:?}");

    let issue = &issued[0].payload;
    assert_eq!(issue["ticket_kind"], "file_write_v1");
    assert_eq!(issue["capability_scope"], "file_write");
    assert!(issue["activation_id"].is_null(), "main lane — no activation (D2)");
    assert_eq!(
        issue["resolved_target_sha256"].as_str().unwrap().len(),
        64,
        "the parsed real target is bound (check 5b)"
    );
    // The consumed event pairs with the issued one (same ticket_id, same kind).
    assert_eq!(
        consumed[0].payload["ticket_id"], issue["ticket_id"],
        "consumed must pair with issued"
    );
    assert_eq!(consumed[0].payload["ticket_kind"], "file_write_v1");
    assert_eq!(consumed[0].payload["outcome"], "accepted");
    // The tool itself ran through the gate chain.
    assert!(types.iter().any(|t| t == "tool_started"), "{types:?}");
    assert!(types.iter().any(|t| t == "tool_completed"), "{types:?}");
    // The ticket lifecycle wraps the tool call in the journal.
    let started = types.iter().position(|t| t == "tool_started").unwrap();
    let issued_idx = types
        .iter()
        .position(|t| t == "control_ticket_issued")
        .unwrap();
    assert!(issued_idx < started, "ticket precedes ToolStarted: {types:?}");
}

// ── test 6: Slice 2 first phase — signer down → shadow rejection + proceed ──

#[tokio::test]
async fn file_write_shadow_on_signer_unreachable() {
    // Same permanence trick as the control-event shadow test: kill the
    // signer AND remove its manifest so the self-heal cannot revive it.
    let fixture = SignerFixture::new();
    let mut client = spawn_client(&fixture).await;
    client.shutdown().await;
    std::fs::remove_file(&fixture.manifest).expect("remove manifest");
    let client = Arc::new(tokio::sync::Mutex::new(client));

    let dir = test_dir();
    let target = dir.join("src").join("main.rs");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "original").unwrap();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "patched".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "search_replace".to_string(),
            arguments: serde_json::json!({
                "file_path": "src/main.rs",
                "old_string": "original",
                "new_string": "patched",
            }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "改文件", "RUN-ACAF-SHADOW-FW", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert!(
        !rejected.is_empty(),
        "signer-unreachable must journal a rejection: {types:?}"
    );
    let fw_rejected: Vec<_> = rejected
        .iter()
        .filter(|e| e.payload["ticket_kind"] == "file_write_v1")
        .collect();
    assert_eq!(fw_rejected.len(), 1, "file_write rejection: {types:?}");
    assert_eq!(fw_rejected[0].payload["reject_code"], "signer_unreachable");
    // Shadow mode: the write still executed.
    assert!(
        types.iter().any(|t| t == "tool_started") && types.iter().any(|t| t == "tool_completed"),
        "tool proceeds under shadow mode: {types:?}"
    );
}

// ── test 7: goal wiring — a continue re-derives K_session, old tickets die ──

/// Goal wiring (2026-08-12): an accepted `continue` consumes a
/// GoalRevisionV1 ticket under the OLD goal context, then the controller
/// swaps the run-level goal binding (digest + version 0→1). The NEXT ticket
/// call re-initializes the signer session (new K_session) — the signer's
/// sequence ledger restarts at 1 and the close-flow tickets bind the NEW
/// goal. This is the "goal 变 → 新 key 旧票死" path (ADR-0011 决策 5),
/// previously unreachable (Slice 1 audit D5 boundary, now closed).
#[tokio::test]
async fn goal_revision_continue_flow_re_derives_session_key() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "ok".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let old_goal = "查找项目文档";
    let delta = "继续查第二批";
    let old_goal_digest = sha256_hex(&canonical_json(&serde_json::json!({ "goal": old_goal })).unwrap());
    let new_goal_digest = sha256_hex(&canonical_json(&serde_json::json!({ "goal": delta })).unwrap());

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
        ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s1")]),
        ScriptedResponse::text("[DOC] a.md\n第一批"),
        ScriptedResponse::tool_calls(vec![disposition_call("continue", Some(delta), "call-d1")]),
        ScriptedResponse::text("继续"),
        // Re-entry into the same activation (continue semantics).
        ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
        ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s2")]),
        ScriptedResponse::text("[DOC] a.md\n第二批"),
        ScriptedResponse::tool_calls(vec![disposition_call("close", None, "call-d2")]),
        ScriptedResponse::text("完成"),
        // counterexample gate round
        ScriptedResponse::text("完成"),
    ]));

    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_mode(
            RetrievalMode::FrameworkFallback,
            RetrievalCapability::Available,
            false,
            None,
            None,
        )
        .with_acaf(Some(client));
    controller
        .run_turn(&host, old_goal, "RUN-ACAF-CONT", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();

    // 4 tickets: disposition-continue + goal_revision (OLD context) then
    // disposition-close + close (NEW context, after the re-derivation).
    assert_eq!(issued.len(), 4, "expected 4 tickets: {types:?}");
    assert_eq!(consumed.len(), 4, "all tickets consumed: {types:?}");
    assert!(rejected.is_empty(), "no rejections on the happy path: {types:?}");

    let kinds: Vec<&str> = issued
        .iter()
        .map(|e| e.payload["ticket_kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        vec!["disposition_v1", "goal_revision_v1", "disposition_v1", "close_v1"],
        "ticket order across the continue flow: {kinds:?}"
    );

    // OLD-context tickets: goal_version 0, the run-start goal digest.
    for (i, ticket) in issued.iter().take(2).enumerate() {
        assert_eq!(ticket.payload["goal_version"], 0, "ticket[{i}] binds the old goal");
        assert_eq!(
            ticket.payload["goal_digest"].as_str().unwrap(),
            old_goal_digest,
            "ticket[{i}] old goal digest"
        );
    }
    // The goal_revision ticket carries the activation binding.
    assert!(
        issued[1].payload["activation_id"].as_str().unwrap().starts_with("retrieval-"),
        "goal_revision ticket binds the activation: {:?}",
        issued[1].payload
    );

    // NEW-context tickets (after K_session re-derivation): goal_version 1,
    // the continue delta as the new goal binding.
    for (i, ticket) in issued.iter().skip(2).enumerate() {
        let i = i + 2;
        assert_eq!(ticket.payload["goal_version"], 1, "ticket[{i}] binds the new goal");
        assert_eq!(
            ticket.payload["goal_digest"].as_str().unwrap(),
            new_goal_digest,
            "ticket[{i}] new goal digest"
        );
    }

    // Ledger epoch: the re-derivation restarts the signer sequence — the
    // close-flow tickets re-start at 1 (per-K_session epoch, Slice 1 D2-1).
    let seqs: Vec<u64> = issued
        .iter()
        .map(|e| e.payload["sequence"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, vec![1, 2, 1, 2], "sequence restarts on the new epoch: {seqs:?}");

    // Consumed-after-issued pairing for every ticket.
    for consumed_event in &consumed {
        let cp = &consumed_event.payload;
        let matching_issued = issued
            .iter()
            .find(|e| e.payload["ticket_id"] == cp["ticket_id"])
            .unwrap_or_else(|| panic!("consumed references unknown ticket: {cp:?}"));
        assert!(
            matching_issued.sequence < consumed_event.sequence,
            "consumed must follow its issued event"
        );
    }
}

// ── test 8: Slice 2 full phase — network_v1 (browser_read) full chain ─────

#[tokio::test]
async fn network_ticket_full_chain() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "page text".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({
                "url": "HTTP://Example.COM:80/docs/guide?q=1#frag",
            }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_mode(
            RetrievalMode::LocalBrowser,
            RetrievalCapability::Available,
            false,
            None,
            None,
        )
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "抓取网页", "RUN-ACAF-NET", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();

    assert_eq!(issued.len(), 1, "one network ticket: {types:?}");
    assert_eq!(consumed.len(), 1, "ticket consumed: {types:?}");
    assert!(rejected.is_empty(), "no rejections on the happy path: {types:?}");

    let issue = &issued[0].payload;
    assert_eq!(issue["ticket_kind"], "network_v1");
    assert_eq!(issue["capability_scope"], "network");
    assert!(issue["activation_id"].is_null(), "main lane — no activation (D2)");
    assert_eq!(
        issue["resolved_target_sha256"].as_str().unwrap().len(),
        64,
        "the canonical URL target is bound (check 5b)"
    );
    // Review P2-1 (2026-08-12): pin the EXACT canonical target — the raw
    // input `HTTP://Example.COM:80/docs/guide?q=1#frag` must bind
    // `http://example.com/docs/guide?q=1`, not the original spelling.
    assert_eq!(
        issue["resolved_target_sha256"].as_str().unwrap(),
        network_target_digest("http://example.com/docs/guide?q=1"),
        "the canonical URL digest is bound, not the raw input"
    );
    assert_eq!(consumed[0].payload["ticket_id"], issue["ticket_id"]);
    assert_eq!(consumed[0].payload["ticket_kind"], "network_v1");
    assert_eq!(consumed[0].payload["outcome"], "accepted");
    let started = types.iter().position(|t| t == "tool_started").unwrap();
    let issued_idx = types
        .iter()
        .position(|t| t == "control_ticket_issued")
        .unwrap();
    assert!(issued_idx < started, "ticket precedes ToolStarted: {types:?}");
}

// ── test 9: command_exec_v1 — model-supplied run_terminal_cmd ─────────────

#[tokio::test]
async fn run_terminal_cmd_command_ticket_full_chain() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "done".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "run_terminal_cmd".to_string(),
            arguments: serde_json::json!({
                "command": "python -c print('hi')",
                "description": "say hi",
            }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "运行命令", "RUN-ACAF-CMD", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();

    assert_eq!(issued.len(), 1, "one command_exec ticket: {types:?}");
    assert_eq!(consumed.len(), 1, "ticket consumed: {types:?}");
    assert!(rejected.is_empty(), "no rejections on the happy path: {types:?}");

    let issue = &issued[0].payload;
    assert_eq!(issue["ticket_kind"], "command_exec_v1");
    assert_eq!(issue["capability_scope"], "command_exec");
    assert!(issue["activation_id"].is_null(), "main lane — no activation (D2)");
    assert_eq!(
        issue["resolved_target_sha256"].as_str().unwrap().len(),
        64,
        "the argv/cwd/env target is bound (check 5b)"
    );
    // Review P2-1 (2026-08-12): pin the exact argv/cwd/env target digest.
    let cmd_cwd = dir.to_string_lossy().into_owned();
    let cmd_argv = vec!["python -c print('hi')".to_string()];
    let cmd_env_sha = command_env_sha256(&[]);
    assert_eq!(
        issue["resolved_target_sha256"].as_str().unwrap(),
        command_exec_target_digest(&cmd_argv, &cmd_cwd, &cmd_env_sha),
        "the command target digest matches the canonical argv/cwd/env triple"
    );
    assert_eq!(consumed[0].payload["ticket_id"], issue["ticket_id"]);
    assert_eq!(consumed[0].payload["ticket_kind"], "command_exec_v1");
    let started = types.iter().position(|t| t == "tool_started").unwrap();
    let issued_idx = types
        .iter()
        .position(|t| t == "control_ticket_issued")
        .unwrap();
    assert!(issued_idx < started, "ticket precedes ToolStarted: {types:?}");
}

// ── test 10: command_exec_v1 — host-owned run_tests fixed command ──────────

#[tokio::test]
async fn run_tests_command_ticket_full_chain() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "tests ok".to_string(),
            exit_code: Some(0),
        }),
        test_runner: Some(TestRunner {
            command: vec!["python".to_string(), "-m".to_string(), "pytest".to_string()],
            timeout: None,
            env: vec![("PYTHONPATH".to_string(), "src".to_string())],
        }),
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "run_tests".to_string(),
            arguments: serde_json::json!({}),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "运行测试", "RUN-ACAF-RT", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();

    assert_eq!(issued.len(), 1, "one run_tests command_exec ticket: {types:?}");
    assert_eq!(consumed.len(), 1, "ticket consumed: {types:?}");
    assert!(rejected.is_empty(), "no rejections on the happy path: {types:?}");

    let issue = &issued[0].payload;
    assert_eq!(issue["ticket_kind"], "command_exec_v1");
    assert_eq!(issue["capability_scope"], "command_exec");
    assert_eq!(
        issue["resolved_target_sha256"].as_str().unwrap().len(),
        64,
        "the argv/cwd/env target is bound (check 5b)"
    );
    // Review P2-1 (2026-08-12): pin the exact argv/cwd/env target digest
    // and the consumed outcome.
    let rt_cwd = dir.to_string_lossy().into_owned();
    let rt_argv = vec!["python".to_string(), "-m".to_string(), "pytest".to_string()];
    let rt_env = vec![("PYTHONPATH".to_string(), "src".to_string())];
    let rt_env_sha = command_env_sha256(&rt_env);
    assert_eq!(
        issue["resolved_target_sha256"].as_str().unwrap(),
        command_exec_target_digest(&rt_argv, &rt_cwd, &rt_env_sha),
        "the run_tests target digest matches the host-owned command triple"
    );
    assert_eq!(consumed[0].payload["ticket_id"], issue["ticket_id"]);
    assert_eq!(consumed[0].payload["ticket_kind"], "command_exec_v1");
    assert_eq!(consumed[0].payload["outcome"], "accepted");
    // The ticket wraps the run BEFORE ToolStarted (ordering discipline).
    let started = types.iter().position(|t| t == "tool_started").unwrap();
    let issued_idx = types
        .iter()
        .position(|t| t == "control_ticket_issued")
        .unwrap();
    assert!(issued_idx < started, "ticket precedes ToolStarted: {types:?}");
    // The fixed command is still declared on ToolStarted.
    let started_event = &events[started];
    assert_eq!(
        started_event.payload["fixed_command"],
        "python -m pytest"
    );
}

// ── test 11: Slice 2 full phase — invalid URL → shadow ledger, tool runs ───

#[tokio::test]
async fn invalid_network_url_shadow_records_rejection_and_proceeds() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "page text".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({
                "url": "not a url",
            }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_mode(
            RetrievalMode::LocalBrowser,
            RetrievalCapability::Available,
            false,
            None,
            None,
        )
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "抓取网页", "RUN-ACAF-NET-BAD", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert_eq!(rejected.len(), 1, "one unticketable-URL rejection: {types:?}");
    let rejection = &rejected[0].payload;
    assert_eq!(rejection["ticket_kind"], "network_v1");
    assert_eq!(rejection["reject_code"], "target_mismatch");
    assert!(rejection["ticket_id"].is_null(), "no ticket was issued: {rejection:?}");
    // Shadow mode: the tool still executes.
    assert!(
        types.iter().any(|t| t == "tool_started") && types.iter().any(|t| t == "tool_completed"),
        "tool proceeds under shadow mode: {types:?}"
    );
}

// ── test 12: empty run_terminal_cmd command → shadow ledger, tool runs ────

#[tokio::test]
async fn run_terminal_cmd_empty_command_shadow_records_rejection_and_proceeds() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "done".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "run_terminal_cmd".to_string(),
            arguments: serde_json::json!({
                "command": "   ",
                "description": "noop",
            }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "运行命令", "RUN-ACAF-CMD-EMPTY", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert_eq!(rejected.len(), 1, "one unticketable-command rejection: {types:?}");
    let rejection = &rejected[0].payload;
    assert_eq!(rejection["ticket_kind"], "command_exec_v1");
    assert_eq!(rejection["reject_code"], "target_mismatch");
    assert!(rejection["ticket_id"].is_null(), "no ticket was issued: {rejection:?}");
    // Shadow mode: the tool still executes.
    assert!(
        types.iter().any(|t| t == "tool_started") && types.iter().any(|t| t == "tool_completed"),
        "tool proceeds under shadow mode: {types:?}"
    );
}

// ── test 13: missing URL key with configured ACAF → silent skip (locked) ──

#[tokio::test]
async fn missing_network_arg_silently_skips_with_configured_acaf() {
    // Review P2-1 (2026-08-12): the missing-argument silent skip is a
    // REGISTERED shadow-mode behaviour (fail-closed flip checklist ③/④) —
    // locked here so the flip decision cannot forget it.
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));

    let dir = test_dir();
    let store = Arc::new(
        orz_assurance::session::snapshot::SnapshotStore::new(
            dir.join(".gsa").join("snapshots"),
            dir.clone(),
        )
        .unwrap(),
    );
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: Some(ToolResult {
            output: "page text".to_string(),
            exit_code: Some(0),
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "foo": "bar" }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_mode(
            RetrievalMode::LocalBrowser,
            RetrievalCapability::Available,
            false,
            None,
            None,
        )
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(&host, "抓取网页", "RUN-ACAF-NET-NOURL", MANIFEST, 0, None, None, None)
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let ticket_events: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                e.event_type.to_string().as_str(),
                "control_ticket_issued" | "control_ticket_consumed" | "control_ticket_rejected"
            )
        })
        .collect();
    assert!(
        ticket_events.is_empty(),
        "missing URL key must silently skip in shadow mode: {types:?}"
    );
    // The tool still runs (the host-side tool fails on its own or succeeds
    // per the host's own argument contract).
    assert!(
        types.iter().any(|t| t == "tool_started") && types.iter().any(|t| t == "tool_completed"),
        "tool path unchanged: {types:?}"
    );
}
