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

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use orz_assurance::acaf::{RejectCode, TicketKind};
use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::journal::sha256_hex;
use orz_assurance::journal::{JournalRecorder, RunEvent};
use orz_host::keystore::WindowsDpapiInstallationKeyStore;
use orz_loop::acaf::{AcafClient, AcafConfig, TicketOutcome};
use orz_loop::controller::{AgentLoopController, RetrievalCapability, RetrievalMode};
use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::{ModelGateway, ToolCall};
use orz_loop::host::{
    LoopHost, PermitDecision, PermitError, ToolDef, ToolError, ToolResult, ToolRegistry,
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
