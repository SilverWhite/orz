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
//! Test 2: controller full chain — THIN-HARNESS-REDESIGN R1 (2026-08-27,
//! 设计 §4.4)「每次调用即闭环」：检索派发结果形成后立即 auto_close（CloseV1
//! 票据 issued → consumed + close record，terminal_reason=auto_close）；
//! disposition close/continue 往返退役——跨 run 恢复的 AwaitingDisposition
//! 激活经休眠 handler（handle_parent_disposition）带 DispositionV1 /
//! GoalRevisionV1 票据（见 goal_revision_continue_flow_re_derives_session_key
//! 与 fail_closed_*_goal_revision_* 测试）。
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

use orz_assurance::acaf::target::network_target_digest;
use orz_assurance::acaf::{RejectCode, TicketKind};
use orz_assurance::canonical_json;
use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::journal::sha256_hex;
use orz_assurance::journal::{JournalRecorder, RunEvent};
use orz_host::keystore::WindowsDpapiInstallationKeyStore;
use orz_loop::acaf::{
    AcafClient, AcafConfig, TicketOutcome, command_env_sha256, command_exec_target_digest,
};
use orz_loop::controller::AgentLoopController;
use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::{ModelGateway, ToolCall};
use orz_loop::host::{
    LoopHost, PermitDecision, PermitError, TestRunResult, TestRunner, ToolDef, ToolError,
    ToolRegistry, ToolResult,
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
            timed_out: false,
            full_output_path: None,
            output_encoding: None,
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
        Self { manifest, keystore }
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

/// THIN-HARNESS-REDESIGN R1 (2026-08-27, 设计 §4.4): disposition
/// close/continue 往返已从生产流程退役，但跨 run 恢复的
/// AwaitingDisposition 激活仍经休眠 handler（handle_parent_disposition）
/// 延续——这是 DispositionV1 / GoalRevisionV1 票据路径的唯一可达入口。
/// 本快照模拟上一 run 遗留的待决激活（sidecar 形态，与 controller
/// `stored_activation_old_sidecar_no_conversation_field` 测试一致）。
fn seeded_internal_activation_snapshot() -> serde_json::Value {
    serde_json::json!({
        "next_seq": {"internal_retrieval": 1},
        "activations": [{
            "activation_id": "retrieval-internal_retrieval-sess-abc-00",
            "parent_session_id": "sess-abcdef123456",
            "subagent_session_id": "SUB-internal_retrieval-sess-abc",
            "contract_id": "retrieval-contract-internal_retrieval",
            "contract_revision": 0,
            "status": "awaiting_disposition",
            "tool_rounds_used": 2,
            "result_digest": "b".repeat(64),
            "pending_assessment_id": "ASSESS-PREV-1",
            "pending_expected_contract_revision": 0,
            "origin_run_id": "RUN-PREV-0001",
        }]
    })
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
        .sign_ticket(TicketKind::OrientationV1, None, &"0".repeat(64), None, None)
        .await
        .expect("sign orientation");
    assert_eq!(ticket.ticket_kind, "orientation_v1");
    assert_eq!(ticket.sequence, 1);
    let template = ticket
        .template_sha256
        .as_deref()
        .expect("orientation ticket carries the signer-held template digest");
    assert_eq!(template.len(), 64);
    // The template digest is the signer-held constant, not free text — its
    // exact value comes from the signer's held template (check 2 compares it
    // against the version downloaded at initialize_session).

    let outcome = client
        .verify_and_consume(&ticket, &"0".repeat(64), None, None, None)
        .await
        .expect("verify");
    assert!(
        matches!(outcome, TicketOutcome::Consumed { .. }),
        "fresh ticket must consume: {outcome:?}"
    );

    // Replay — same ticket again → replay_detected.
    let outcome = client
        .verify_and_consume(&ticket, &"0".repeat(64), None, None, None)
        .await
        .expect("verify again");
    assert!(
        matches!(
            outcome,
            TicketOutcome::Rejected {
                code: RejectCode::ReplayDetected,
                ..
            }
        ),
        "replayed ticket must reject: {outcome:?}"
    );

    // Target mismatch — the LIVE canonical args differ from the ticket's.
    // P0-0x S2: an initial-round orientation ticket binds the second
    // built-in template — sign + verify under the SAME trigger consumes.
    let initial = client
        .sign_ticket(
            TicketKind::OrientationV1,
            None,
            &"0".repeat(64),
            None,
            Some("initial_round"),
        )
        .await
        .expect("sign initial-round orientation");
    assert_ne!(
        initial.template_sha256, ticket.template_sha256,
        "the initial-round ticket must bind a different built-in template"
    );
    let outcome = client
        .verify_and_consume(&initial, &"0".repeat(64), None, None, Some("initial_round"))
        .await
        .expect("verify initial-round ticket");
    assert!(
        matches!(outcome, TicketOutcome::Consumed { .. }),
        "initial-round ticket must consume under its own trigger: {outcome:?}"
    );

    // P0-0x S2 negative — a ticket minted under one trigger must NOT verify
    // under the other: a PERIODIC-digest ticket presented with
    // `trigger=initial_round` fails check 2 (`template_mismatch`) and never
    // consumes (the signer's expected digest is selected by the verifier's
    // trigger, so the two built-ins cannot be crossed).
    let periodic = client
        .sign_ticket(TicketKind::OrientationV1, None, &"0".repeat(64), None, None)
        .await
        .expect("sign periodic orientation");
    assert_eq!(
        periodic.template_sha256, ticket.template_sha256,
        "an absent trigger must keep binding the periodic built-in"
    );
    let outcome = client
        .verify_and_consume(
            &periodic,
            &"0".repeat(64),
            None,
            None,
            Some("initial_round"),
        )
        .await
        .expect("verify periodic ticket as the initial round");
    assert!(
        matches!(
            outcome,
            TicketOutcome::Rejected {
                code: RejectCode::TemplateMismatch,
                ..
            }
        ),
        "periodic ticket verified as the initial round must reject with template_mismatch: {outcome:?}"
    );

    let ticket2 = client
        .sign_ticket(
            TicketKind::DispositionV1,
            Some("ACT-1".into()),
            &"0".repeat(64),
            None,
            None,
        )
        .await
        .expect("sign disposition");
    let outcome = client
        .verify_and_consume(&ticket2, &"1".repeat(64), Some("ACT-1".into()), None, None)
        .await
        .expect("verify with wrong args");
    assert!(
        matches!(
            outcome,
            TicketOutcome::Rejected {
                code: RejectCode::TargetMismatch,
                ..
            }
        ),
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf_fail_closed(false)
        .with_retrieval_enabled(true)
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "查找项目文档",
            "RUN-ACAF-E2E",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();

    // R1 (2026-08-27, 设计 §4.4): 每次调用即闭环——检索结果形成后立即
    // auto_close（CloseV1 票据 + close record）；disposition close/continue
    // 往返退役，迟到的 retrieval_disposition 调用被机械拒绝
    // （no_pending_assessment，零 disposition 事件、零二次 close）。本测试
    // 锁定 auto_close 控制事件票据链：CloseV1 issued → consumed，无
    // DispositionV1/GoalRevisionV1。
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

    // auto_close = 一条 CloseV1 票据。
    assert_eq!(issued.len(), 1, "one auto-close ticket: {types:?}");
    assert_eq!(consumed.len(), 1, "auto-close ticket consumed: {types:?}");
    assert!(
        rejected.is_empty(),
        "no rejections in the happy path: {types:?}"
    );
    let close_issued = &issued[0].payload;
    assert_eq!(close_issued["ticket_kind"], "close_v1");

    // close record: auto_close 引用 assessment + result_digest，不引用
    // disposition。
    let closes: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "retrieval_close_record")
        .collect();
    assert_eq!(closes.len(), 1, "one auto_close close record: {types:?}");
    let close = &closes[0].payload;
    assert_eq!(
        close.get("terminal_reason").and_then(|v| v.as_str()),
        Some("auto_close")
    );
    assert_eq!(
        close.get("validated_disposition_id"),
        Some(&serde_json::Value::Null),
        "auto_close 不引用 disposition"
    );
    assert!(close.get("assessment_id").is_some());
    let digest = close.get("result_digest").and_then(|v| v.as_str()).unwrap();
    assert_eq!(digest.len(), 64, "64-hex sha256 for auto_close");

    // 迟到的 disposition 调用（脚本 call-d1）被机械拒绝——零 disposition
    // 事件、零二次 close。
    assert_eq!(
        events
            .iter()
            .filter(|e| e.event_type.to_string() == "retrieval_parent_disposition")
            .count(),
        0,
        "disposition round-trip retired: {types:?}"
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| {
                e.event_type.to_string() == "tool_completed"
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("no_pending_assessment")
            })
            .count(),
        1,
        "late disposition mechanically refused: {types:?}"
    );

    // Consumed must pair with its issued ticket AND appear after it
    // (Review P2-5, 2026-08-12: the one-shot consume wraps the control
    // event in the journal).
    let consumed_payload = &consumed[0].payload;
    assert_eq!(
        consumed_payload["ticket_id"], close_issued["ticket_id"],
        "consumed references the issued ticket: {types:?}"
    );
    assert!(
        issued[0].sequence < consumed[0].sequence,
        "consumed must follow its issued event (issued seq {}, consumed seq {})",
        issued[0].sequence,
        consumed[0].sequence
    );

    // Goal binding present (check 4 context).
    assert_eq!(close_issued["goal_version"], 0);
    assert_eq!(
        close_issued["goal_digest"].as_str().unwrap().len(),
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf_fail_closed(false)
        .with_retrieval_enabled(true)
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "查找项目文档",
            "RUN-ACAF-SHADOW",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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
    // R1 (2026-08-27, 设计 §4.4): 每次调用即闭环——auto_close 的 CloseV1
    // 票据在 signer 不可达时被拒（shadow 留痕），但控制事件仍继续：close
    // record 照常落（shadow 模式 rejection 不阻断控制事件，Slice 1 语义）。
    let rejected_kinds: Vec<&str> = rejected
        .iter()
        .map(|e| e.payload["ticket_kind"].as_str().unwrap())
        .collect();
    assert!(
        rejected_kinds.contains(&"close_v1"),
        "auto-close CloseV1 rejection must be journaled: {rejected_kinds:?}"
    );
    let closes: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "retrieval_close_record")
        .collect();
    assert_eq!(
        closes.len(),
        1,
        "shadow mode: the auto-close control event proceeds: {types:?}"
    );
    assert_eq!(
        closes[0].payload["terminal_reason"], "auto_close",
        "{:?}",
        closes[0].payload
    );
    // 零 disposition 事件（往返退役）；迟到的 disposition 调用被机械拒绝。
    assert_eq!(
        events
            .iter()
            .filter(|e| e.event_type.to_string() == "retrieval_parent_disposition")
            .count(),
        0,
        "disposition round-trip retired: {types:?}"
    );
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
        .sign_ticket(TicketKind::OrientationV1, None, &"0".repeat(64), None, None)
        .await
        .expect("sign before crash");
    assert!(matches!(
        client
            .verify_and_consume(&ticket, &"0".repeat(64), None, None, None)
            .await
            .expect("verify before crash"),
        TicketOutcome::Consumed { .. }
    ));

    // Crash the signer hard (kill without shutdown — the client must not
    // know). 2026-08-16 review fix: the old test only CLAIMED to crash the
    // signer — no kill ever happened, so the respawn branch was never
    // exercised and the "sequence continues" assertion tested nothing.
    // `crash_for_test` makes the crash real.
    client.crash_for_test().await;

    // The FIRST request after the crash hits the dead channel: it fails and
    // the failure path respawns + re-initialises the session (deterministic
    // HKDF → same K_session). The original request is NOT retried.
    let outcome = client
        .sign_ticket(
            TicketKind::DispositionV1,
            Some("ACT-9".into()),
            &"0".repeat(64),
            None,
            None,
        )
        .await;
    assert!(
        outcome.is_err(),
        "the first post-crash request must observe the dead channel"
    );

    // After the self-heal the channel is clean. The signer process restarted,
    // so its per-session sequence restarted at 1 — the ledger high-water was
    // reset with it (2026-08-16 review fix), so the fresh ticket verifies
    // instead of being replay-rejected until the old high-water was
    // exceeded. A fresh nonce keeps the one-shot guarantee within the epoch.
    let ticket2 = client
        .sign_ticket(
            TicketKind::DispositionV1,
            Some("ACT-9".into()),
            &"0".repeat(64),
            None,
            None,
        )
        .await
        .expect("sign after self-heal");
    assert!(
        ticket2.sequence == 1,
        "the respawned signer restarts the per-session sequence: {}",
        ticket2.sequence
    );
    assert_ne!(ticket2.nonce, ticket.nonce, "fresh nonce after respawn");
    assert!(
        matches!(
            client
                .verify_and_consume(&ticket2, &"0".repeat(64), Some("ACT-9".into()), None, None)
                .await
                .expect("verify after self-heal"),
            TicketOutcome::Consumed { .. }
        ),
        "the respawned signer signs with the same K_session — verification passes"
    );
    client.shutdown().await;
}

// ── test 4b: multi-session isolation (2026-08-16 review fix P2-I2) ─────────

#[tokio::test]
async fn sessions_are_isolated_across_switching() {
    // The ACP server shares ONE signer client across sessions. Switching
    // back to an already-initialised session must NOT reset its one-shot
    // ledger — the old single-slot cache re-derived + reset on every switch,
    // so a session's consumed nonces / sequence high-water were forgotten.
    let fixture = SignerFixture::new();
    let mut client = spawn_client(&fixture).await;

    // Session A: init → sign seq 1 → consume.
    client
        .ensure_initialized("SESS-A", "main", 0, &"0".repeat(64), 0)
        .await
        .expect("init A");
    let ta = client
        .sign_ticket(TicketKind::OrientationV1, None, &"0".repeat(64), None, None)
        .await
        .expect("sign A1");
    assert_eq!(ta.sequence, 1);
    assert!(matches!(
        client
            .verify_and_consume(&ta, &"0".repeat(64), None, None, None)
            .await
            .expect("consume A1"),
        TicketOutcome::Consumed { .. }
    ));

    // Session B: independent sequence starting at 1 with its own context.
    client
        .ensure_initialized("SESS-B", "main", 0, &"1".repeat(64), 0)
        .await
        .expect("init B");
    let tb = client
        .sign_ticket(TicketKind::OrientationV1, None, &"0".repeat(64), None, None)
        .await
        .expect("sign B1");
    assert_eq!(tb.sequence, 1, "B has its own sequence");
    assert!(matches!(
        client
            .verify_and_consume(&tb, &"0".repeat(64), None, None, None)
            .await
            .expect("consume B1"),
        TicketOutcome::Consumed { .. }
    ));

    // Switch BACK to A: cached epoch → no re-derivation → sequence continues
    // at 2 and A's ledger still remembers its consumed nonce.
    client
        .ensure_initialized("SESS-A", "main", 0, &"0".repeat(64), 0)
        .await
        .expect("re-init A");
    let ta2 = client
        .sign_ticket(
            TicketKind::DispositionV1,
            Some("ACT-1".into()),
            &"0".repeat(64),
            None,
            None,
        )
        .await
        .expect("sign A2");
    assert_eq!(
        ta2.sequence, 2,
        "session A's sequence must continue after switching away and back"
    );
    assert!(matches!(
        client
            .verify_and_consume(&ta2, &"0".repeat(64), Some("ACT-1".into()), None, None)
            .await
            .expect("consume A2"),
        TicketOutcome::Consumed { .. }
    ));

    // A's first ticket must STILL be replay-rejected — the ledger was not
    // reset by the A→B→A switching.
    let replay = client
        .verify_and_consume(&ta, &"0".repeat(64), None, None, None)
        .await
        .expect("replay A1");
    assert!(
        matches!(
            replay,
            TicketOutcome::Rejected {
                code: RejectCode::ReplayDetected,
                ..
            }
        ),
        "replay of A's consumed ticket must be rejected after switching: {replay:?}"
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf_fail_closed(false)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "改文件",
            "RUN-ACAF-FW",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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
    assert!(
        rejected.is_empty(),
        "no rejections in the happy path: {types:?}"
    );

    let issue = &issued[0].payload;
    assert_eq!(issue["ticket_kind"], "file_write_v1");
    assert_eq!(issue["capability_scope"], "file_write");
    assert!(
        issue["activation_id"].is_null(),
        "main lane — no activation (D2)"
    );
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
    assert!(
        issued_idx < started,
        "ticket precedes ToolStarted: {types:?}"
    );
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf_fail_closed(false)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "改文件",
            "RUN-ACAF-SHADOW-FW",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let old_goal = "查找项目文档";
    let delta = "继续查第二批";
    let old_goal_digest =
        sha256_hex(&canonical_json(&serde_json::json!({ "goal": old_goal })).unwrap());
    let new_goal_digest =
        sha256_hex(&canonical_json(&serde_json::json!({ "goal": delta })).unwrap());

    // R1 (2026-08-27, 设计 §4.4): disposition 往返退役——GoalRevisionV1 仅
    // 经跨 run 恢复的 AwaitingDisposition 激活（休眠 handler）可达。种子
    // 一个上一 run 遗留的待决激活，脚本：continue（DispositionV1 +
    // GoalRevisionV1，旧 goal 上下文）→ 父代理 wrap-up → 重新派发
    // （continue 语义复用同一激活）→ 新结果 auto_close（CloseV1，新 goal
    // 上下文，K_session 重派生后序列重启）。
    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![disposition_call("continue", Some(delta), "call-d1")]),
        ScriptedResponse::text("继续"), // parent wrap-up after the accepted continue
        // Re-entry into the same activation (continue semantics).
        ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
        ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s2")]),
        ScriptedResponse::text("[DOC] a.md\n第二批"), // subagent result → auto_close
        ScriptedResponse::text("完成"),
        // counterexample gate round
        ScriptedResponse::text("完成"),
    ]));

    let controller = AgentLoopController::with_gateway(gateway)
        .with_acaf_fail_closed(false)
        .with_activation_snapshot(Some(&seeded_internal_activation_snapshot()))
        .with_retrieval_enabled(true)
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            old_goal,
            "RUN-ACAF-CONT",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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

    // 3 tickets: disposition-continue + goal_revision (OLD context) then
    // auto-close CloseV1 (NEW context, after the re-derivation).
    assert_eq!(issued.len(), 3, "expected 3 tickets: {types:?}");
    assert_eq!(consumed.len(), 3, "all tickets consumed: {types:?}");
    assert!(
        rejected.is_empty(),
        "no rejections on the happy path: {types:?}"
    );

    let kinds: Vec<&str> = issued
        .iter()
        .map(|e| e.payload["ticket_kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        vec!["disposition_v1", "goal_revision_v1", "close_v1"],
        "ticket order across the continue flow: {kinds:?}"
    );

    // OLD-context tickets (disposition + goal revision): goal_version 0,
    // the run-start goal digest.
    for (i, ticket) in issued.iter().take(2).enumerate() {
        assert_eq!(
            ticket.payload["goal_version"], 0,
            "ticket[{i}] binds the old goal"
        );
        assert_eq!(
            ticket.payload["goal_digest"].as_str().unwrap(),
            old_goal_digest,
            "ticket[{i}] old goal digest"
        );
    }
    // The goal_revision ticket carries the activation binding.
    assert!(
        issued[1].payload["activation_id"]
            .as_str()
            .unwrap()
            .starts_with("retrieval-"),
        "goal_revision ticket binds the activation: {:?}",
        issued[1].payload
    );

    // NEW-context ticket (auto-close CloseV1, after K_session
    // re-derivation): goal_version 1, the continue delta as the new goal
    // binding.
    let new_ctx = &issued[2].payload;
    assert_eq!(
        new_ctx["goal_version"], 1,
        "close_v1 binds the new goal: {types:?}"
    );
    assert_eq!(
        new_ctx["goal_digest"].as_str().unwrap(),
        new_goal_digest,
        "close_v1 new goal digest"
    );

    // Ledger epoch: the re-derivation restarts the signer sequence — the
    // close ticket re-starts at 1 (per-K_session epoch, Slice 1 D2-1).
    let seqs: Vec<u64> = issued
        .iter()
        .map(|e| e.payload["sequence"].as_u64().unwrap())
        .collect();
    assert_eq!(
        seqs,
        vec![1, 2, 1],
        "sequence restarts on the new epoch: {seqs:?}"
    );

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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "example guide" }),
            call_id: "call-1".to_string(),
        }]),
        // Subagent: the local_browser second segment — the REAL host
        // browser_read (P0-B step 4: candidate-counted, lane-bound).
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({
                "url": "HTTP://Example.COM:80/docs/guide?q=1#frag",
            }),
            call_id: "call-s1".to_string(),
        }]),
        ScriptedResponse::text("[SOURCE] HTTP://Example.COM:80/docs/guide?q=1#frag\n内容"),
        // Main: close the external activation.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "retrieval_disposition".to_string(),
            arguments: serde_json::json!({
                "role": "external_retrieval",
                "decision": "close",
            }),
            call_id: "call-d1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_acaf_fail_closed(false)
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "抓取网页",
            "RUN-ACAF-NET",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let network_issued: Vec<_> = issued
        .iter()
        .filter(|e| e.payload["ticket_kind"] == "network_v1")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();

    assert_eq!(
        network_issued.len(),
        1,
        "one lane network ticket: {types:?}"
    );
    assert!(
        rejected.is_empty(),
        "no rejections on the happy path: {types:?}"
    );

    let issue = &network_issued[0].payload;
    assert_eq!(issue["ticket_kind"], "network_v1");
    assert_eq!(issue["capability_scope"], "network");
    let activation = issue["activation_id"]
        .as_str()
        .expect("D-13: network ticket must bind the lane activation");
    assert!(
        activation.starts_with("retrieval-external_retrieval-") && activation.ends_with("-00"),
        "unexpected activation binding: {activation}"
    );
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
    assert!(
        consumed.iter().any(|e| {
            e.payload["ticket_id"] == issue["ticket_id"]
                && e.payload["ticket_kind"] == "network_v1"
                && e.payload["outcome"] == "accepted"
        }),
        "network ticket must be consumed: {types:?}"
    );
    let browser_started = events
        .iter()
        .position(|e| {
            e.event_type.to_string() == "tool_started" && e.payload["tool"] == "browser_read"
        })
        .unwrap();
    let issued_idx = events
        .iter()
        .position(|e| {
            e.event_type.to_string() == "control_ticket_issued"
                && e.payload["ticket_kind"] == "network_v1"
        })
        .unwrap();
    assert!(
        issued_idx < browser_started,
        "ticket precedes ToolStarted: {types:?}"
    );
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf_fail_closed(false)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "运行命令",
            "RUN-ACAF-CMD",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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
    assert!(
        rejected.is_empty(),
        "no rejections on the happy path: {types:?}"
    );

    let issue = &issued[0].payload;
    assert_eq!(issue["ticket_kind"], "command_exec_v1");
    assert_eq!(issue["capability_scope"], "command_exec");
    assert!(
        issue["activation_id"].is_null(),
        "main lane — no activation (D2)"
    );
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
    assert!(
        issued_idx < started,
        "ticket precedes ToolStarted: {types:?}"
    );
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf_fail_closed(false)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "运行测试",
            "RUN-ACAF-RT",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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

    assert_eq!(
        issued.len(),
        1,
        "one run_tests command_exec ticket: {types:?}"
    );
    assert_eq!(consumed.len(), 1, "ticket consumed: {types:?}");
    assert!(
        rejected.is_empty(),
        "no rejections on the happy path: {types:?}"
    );

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
    assert!(
        issued_idx < started,
        "ticket precedes ToolStarted: {types:?}"
    );
    // The fixed command is still declared on ToolStarted.
    let started_event = &events[started];
    assert_eq!(started_event.payload["fixed_command"], "python -m pytest");
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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "example" }),
            call_id: "call-1".to_string(),
        }]),
        // Subagent: the lane's browser_read carries an unticketable URL.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({
                "url": "not a url",
            }),
            call_id: "call-s1".to_string(),
        }]),
        ScriptedResponse::text("[SOURCE] not a url\n内容"),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_acaf_fail_closed(false)
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "抓取网页",
            "RUN-ACAF-NET-BAD",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert_eq!(
        rejected.len(),
        1,
        "one unticketable-URL rejection: {types:?}"
    );
    let rejection = &rejected[0].payload;
    assert_eq!(rejection["ticket_kind"], "network_v1");
    assert_eq!(rejection["reject_code"], "target_mismatch");
    assert!(
        rejection["ticket_id"].is_null(),
        "no ticket was issued: {rejection:?}"
    );
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf_fail_closed(false)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "运行命令",
            "RUN-ACAF-CMD-EMPTY",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert_eq!(
        rejected.len(),
        1,
        "one unticketable-command rejection: {types:?}"
    );
    let rejection = &rejected[0].payload;
    assert_eq!(rejection["ticket_kind"], "command_exec_v1");
    assert_eq!(rejection["reject_code"], "target_mismatch");
    assert!(
        rejection["ticket_id"].is_null(),
        "no ticket was issued: {rejection:?}"
    );
    // Shadow mode: the tool still executes.
    assert!(
        types.iter().any(|t| t == "tool_started") && types.iter().any(|t| t == "tool_completed"),
        "tool proceeds under shadow mode: {types:?}"
    );
}

// ── test 13: browser_read missing URL key → candidate gate refuses ───────

#[tokio::test]
async fn missing_browser_read_url_refuses_before_acaf_with_count_gate() {
    // P0-B step 4 (2026-08-14): the candidate count gate needs a URL as
    // the count identity, so a missing `url` fails closed BEFORE the ACAF
    // layer — `browser_read_candidate_url_missing`, no ticket events, no
    // ToolStarted (supersedes the old shadow-mode silent skip; stronger
    // guarantee: no unticketed channel and no count-identity gap).
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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "example" }),
            call_id: "call-1".to_string(),
        }]),
        // Subagent: browser_read without a URL — the count gate refuses.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "foo": "bar" }),
            call_id: "call-s1".to_string(),
        }]),
        ScriptedResponse::text("检索完成"),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_acaf_fail_closed(false)
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client));
    controller
        .run_turn(
            &host,
            "抓取网页",
            "RUN-ACAF-NET-NOURL",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    // R1 (2026-08-27, 设计 §4.4): 每次调用即闭环——检索 lane 结果形成后
    // auto_close（CloseV1 票据 + close record）是唯一票据事件。browser_read
    // 的 URL 门拒绝发生在此前：该调用无 ToolStarted、无 network_v1 票据
    // （缺 count-identity 即硬拒绝，拒绝在 ACAF 层之前）——门拒绝不产生
    // 任何票据。
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    assert_eq!(
        issued.len(),
        1,
        "only the lane auto-close ticket: {types:?}"
    );
    assert_eq!(consumed.len(), 1, "auto-close ticket consumed: {types:?}");
    assert_eq!(
        issued[0].payload["ticket_kind"], "close_v1",
        "no network_v1 ticket for the refused browser_read: {types:?}"
    );
    assert!(
        events
            .iter()
            .all(|e| e.event_type.to_string() != "control_ticket_rejected"),
        "no rejections on the happy path: {types:?}"
    );
    // The ToolCompleted carries the stable gate code and the tool never
    // started.
    let refused: Vec<_> = events
        .iter()
        .filter(|e| {
            e.event_type.to_string() == "tool_completed"
                && e.payload["tool"] == "browser_read"
                && e.payload["error"] == "browser_read_candidate_url_missing"
        })
        .collect();
    assert_eq!(refused.len(), 1, "one gate refusal: {types:?}");
    assert!(
        !events.iter().any(|e| {
            e.event_type.to_string() == "tool_started"
                && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-s1")
        }),
        "refused browser_read must not start: {types:?}"
    );
}

// ── Slice 2 fail-closed (2026-08-13): D-14/D-15/D-16 ──────────────────────

/// D-15: fail-closed with an unconfigured fabric is a STARTUP error — no
/// unticketed channel exists (ADR-0011 §2 fail-closed; the explicit
/// downgrade switch is a future option).
#[tokio::test]
async fn fail_closed_startup_refuses_unconfigured_fabric() {
    let dir = test_dir();
    let journal = JournalRecorder::new(dir.clone());
    let host = TestHost {
        journal,
        tool_result: None,
        test_runner: None,
    };
    let gateway: Arc<dyn ModelGateway> =
        Arc::new(FakeProvider::new(vec![ScriptedResponse::text("完成")]));
    let controller = AgentLoopController::with_gateway(gateway).with_acaf_fail_closed(true);
    let err = controller
        .run_turn(
            &host,
            "任何任务",
            "RUN-FC-NOACAF",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect_err("fail-closed + no signer must refuse startup");
    assert!(
        err.to_string().contains("fail-closed"),
        "startup error must name fail-closed: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// ⑦ (Slice 2B §6 checklist, closed 2026-08-16): `web_search` is the
/// registered UNMAPPED network surface — its query goes to the configured
/// provider's own search endpoint (same key / same billing face as the
/// model transport; ADR-0010 §3.7 条 10 — no second search provider), so it
/// carries no third-party URL target and is deliberately excluded from
/// ticketing. Under fail-closed it must execute normally with ZERO
/// control-ticket events (regression lock: the exclusion is explicit, not
/// accidental).
#[tokio::test]
async fn fail_closed_web_search_tool_unticketed_lane_auto_close_ticketed() {
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
            output: "web search result".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "example doc" }),
            call_id: "call-1".to_string(),
        }]),
        // The external retrieval subagent runs the search in its own lane.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "example doc" }),
            call_id: "call-s1".to_string(),
        }]),
        ScriptedResponse::text("检索完成"),
        ScriptedResponse::tool_calls(vec![disposition_call("close", None, "call-d1")]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "检索示例",
            "RUN-FC-WS",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    // R1 (2026-08-27, 设计 §4.4): 每次调用即闭环——检索 lane 结果形成后
    // auto_close（CloseV1 票据 + close record）是唯一票据事件。web_search
    // 工具调用本身无票（ACAF 无 web_search 动作票，URL/命令族才有）；
    // fail-closed 下 lane 的 close_v1 正常签发消费，工具调用面无票执行。
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    assert_eq!(
        issued.len(),
        1,
        "only the lane auto-close ticket: {:?}",
        issued
            .iter()
            .map(|e| e.event_type.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(consumed.len(), 1, "auto-close ticket consumed");
    assert_eq!(
        issued[0].payload["ticket_kind"], "close_v1",
        "web_search itself stays unticketed (no action ticket kind)"
    );
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert!(
        rejected.is_empty(),
        "happy path — no rejections: {:?}",
        rejected
            .iter()
            .map(|e| e.event_type.to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        events.iter().any(|e| {
            e.event_type.to_string() == "tool_started"
                && e.payload.get("tool").and_then(|v| v.as_str()) == Some("web_search")
        }),
        "web_search must execute under fail-closed"
    );
    assert!(
        events.iter().any(|e| {
            e.event_type.to_string() == "tool_completed"
                && e.payload.get("tool").and_then(|v| v.as_str()) == Some("web_search")
        }),
        "web_search must complete under fail-closed"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Review fixes (2026-08-13): sign→verify RPC failure + D-16 rejection ──

/// Review fix (2026-08-13): a verify RPC failure between sign and consume
/// journals EXACTLY ONE `control_ticket_rejected` (signer_unreachable) and,
/// under fail-closed, refuses the tool (no ToolStarted). Regression lock
/// for the double-journal defect found in review.
#[tokio::test]
async fn fail_closed_verify_rpc_failure_journals_once_and_blocks() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));
    // Seam: the NEXT network_v1 verify_and_consume fails as if the signer
    // died between sign and verify (the real signer is self-consistent, so
    // an external kill cannot target this exact point).
    client.lock().await.inject_verify_failure(
        TicketKind::NetworkV1,
        "simulated signer death between sign and verify",
    );

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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "example doc" }),
            call_id: "call-1".to_string(),
        }]),
        // Subagent: the lane's browser_read ticket verify fails.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "url": "https://example.com/doc" }),
            call_id: "call-s1".to_string(),
        }]),
        // Subagent retries after the refusal: the one-shot injected
        // failure is consumed, the ticket verifies, and the tool executes.
        // The blocked call must NOT have consumed a candidate (review fix
        // 2026-08-14 — consumption commits after the ticket gate passes),
        // so this completion carries candidate_count=1.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "url": "https://example.com/doc" }),
            call_id: "call-s2".to_string(),
        }]),
        ScriptedResponse::text("检索完成"),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "抓取网页",
            "RUN-FC-VERIFY",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert_eq!(
        rejected.len(),
        1,
        "verify RPC failure must journal exactly ONE rejection: {types:?} payloads={:?}",
        rejected.iter().map(|e| &e.payload).collect::<Vec<_>>()
    );
    assert_eq!(rejected[0].payload["ticket_kind"], "network_v1");
    assert_eq!(rejected[0].payload["reject_code"], "signer_unreachable");
    assert!(
        rejected[0].payload["ticket_id"].is_null(),
        "pre-signing-style refusal has no ticket: {:?}",
        rejected[0].payload
    );
    assert!(
        !events.iter().any(|e| {
            e.event_type.to_string() == "tool_started"
                && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-s1")
        }),
        "fail-closed must not start browser_read: {types:?}"
    );
    let completed: Vec<_> = events
        .iter()
        .filter(|e| {
            e.event_type.to_string() == "tool_completed" && e.payload["tool"] == "browser_read"
        })
        .collect();
    assert_eq!(
        completed.len(),
        2,
        "blocked + retried completions: {types:?}"
    );
    assert_eq!(
        completed[0].payload["error"],
        "control_ticket_rejected:signer_unreachable"
    );
    // P0-C S3 前置审查修复 (F1): refusal completions carry the non-zero
    // exit_code + status=error required by the Python cross-check.
    assert_eq!(completed[0].payload["exit_code"], 1);
    assert_eq!(completed[0].payload["status"], "error");
    assert_eq!(
        completed[0].payload["policy_denial"]["source"],
        serde_json::json!("acaf")
    );
    assert_eq!(
        completed[0].payload["policy_denial"]["code"],
        serde_json::json!("control_ticket_rejected:signer_unreachable")
    );
    assert!(
        completed[0].payload["policy_denial"]["reason"]
            .as_str()
            .is_some_and(|r| !r.is_empty()),
        "structured denial reason: {:?}",
        completed[0].payload
    );
    assert!(
        completed[0].payload.get("candidate_count").is_none(),
        "a ticket-blocked call consumes no candidate and carries no count: {:?}",
        completed[0].payload
    );
    assert_eq!(completed[1].payload["exit_code"], 0);
    assert_eq!(completed[1].payload["candidate_count"], 1);
    assert_eq!(completed[1].payload["candidate_cap"], 8);
    assert!(
        events.iter().any(|e| {
            e.event_type.to_string() == "tool_started"
                && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-s2")
        }),
        "the retried browser_read must actually execute: {types:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Review fix (2026-08-13): D-16's rejection branch — a rejected
/// GoalRevisionV1 under fail-closed must NOT migrate the goal/contract
/// state (the disposition surfaces unauthorized, exit 1). The injected
/// verify failure forces the rejection at the exact GoalRevisionV1
/// consumption point.
#[tokio::test]
async fn fail_closed_goal_revision_rejected_does_not_migrate() {
    let fixture = SignerFixture::new();
    let client = Arc::new(tokio::sync::Mutex::new(spawn_client(&fixture).await));
    client.lock().await.inject_verify_failure(
        TicketKind::GoalRevisionV1,
        "simulated signer death at goal revision",
    );

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
            output: "ok".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    // R1 (2026-08-27, 设计 §4.4): disposition 往返退役——GoalRevisionV1
    // 仅经跨 run 恢复的 AwaitingDisposition 激活（休眠 handler）可达。
    // 种子一个上一 run 遗留的待决激活，脚本直接提交 continue。
    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "retrieval_disposition".to_string(),
            arguments: serde_json::json!({
                "role": "internal_retrieval",
                "decision": "continue",
                "requirement_delta": "补充检索第二批",
            }),
            call_id: "call-d1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_activation_snapshot(Some(&seeded_internal_activation_snapshot()))
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "检索项目",
            "RUN-FC-D16REJ",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert_eq!(
        rejected.len(),
        1,
        "exactly one rejection (the GoalRevisionV1 verify failure): {types:?}"
    );
    assert_eq!(rejected[0].payload["ticket_kind"], "goal_revision_v1");
    assert_eq!(rejected[0].payload["reject_code"], "signer_unreachable");
    // DispositionV1 consumed; GoalRevisionV1 must NOT be consumed (D-16:
    // rejected goal revision does not migrate state).
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    assert!(
        consumed
            .iter()
            .any(|e| e.payload["ticket_kind"] == "disposition_v1"),
        "DispositionV1 must still consume: {:?}",
        consumed.iter().map(|e| &e.payload).collect::<Vec<_>>()
    );
    assert!(
        !consumed
            .iter()
            .any(|e| e.payload["ticket_kind"] == "goal_revision_v1"),
        "GoalRevisionV1 must NOT consume on rejection: {:?}",
        consumed.iter().map(|e| &e.payload).collect::<Vec<_>>()
    );
    // The disposition tool surfaces unauthorized (exit 1).
    let completed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "tool_completed")
        .collect();
    let disposition = completed
        .iter()
        .find(|e| e.payload["tool"] == "retrieval_disposition")
        .expect("disposition tool completed");
    // D-16 surfaces unauthorized via the disposition tool's exit code (the
    // goal-revision rejection itself is the journaled security event).
    assert_eq!(disposition.payload["exit_code"], 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// D-14: a missing `url` under fail-closed is a HARD refusal — the
/// `control_ticket_rejected(missing_target_argument, null ticket_id)` is
/// journaled, ToolStarted never fires, and the tool does not execute.
#[tokio::test]
async fn fail_closed_missing_url_blocks_network_tool() {
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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "example" }),
            call_id: "call-1".to_string(),
        }]),
        // Subagent: browser_read without a URL — the candidate gate
        // refuses before ACAF (P0-B step 4: count identity missing).
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "foo": "bar" }),
            call_id: "call-s1".to_string(),
        }]),
        ScriptedResponse::text("检索完成"),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "抓取网页",
            "RUN-FC-NOURL",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert!(
        rejected.is_empty(),
        "count gate refuses before any ticket layer: {types:?}"
    );
    // No browser_read ToolStarted → the tool never executed.
    assert!(
        !events.iter().any(|e| {
            e.event_type.to_string() == "tool_started"
                && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-s1")
        }),
        "fail-closed must not start browser_read: {types:?}"
    );
    // The ToolCompleted error surfaces the count-gate refusal to the model.
    let completed: Vec<_> = events
        .iter()
        .filter(|e| {
            e.event_type.to_string() == "tool_completed" && e.payload["tool"] == "browser_read"
        })
        .collect();
    assert_eq!(
        completed[0].payload["error"],
        "browser_read_candidate_url_missing"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// D-14: a missing `command` under fail-closed hard-refuses
/// `run_terminal_cmd` the same way (no ToolStarted, no execution).
#[tokio::test]
async fn fail_closed_missing_command_blocks_run_terminal_cmd() {
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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "run_terminal_cmd".to_string(),
            arguments: serde_json::json!({ "description": "noop" }),
            call_id: "call-1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "运行命令",
            "RUN-FC-NOCMD",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert_eq!(rejected.len(), 1, "one hard refusal: {types:?}");
    let rejection = &rejected[0].payload;
    assert_eq!(rejection["ticket_kind"], "command_exec_v1");
    assert_eq!(rejection["reject_code"], "missing_target_argument");
    assert!(rejection["ticket_id"].is_null());
    assert!(
        !types.iter().any(|t| t == "tool_started"),
        "fail-closed must not start the tool: {types:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fail-closed + signer unreachable: the ticket lifecycle failure becomes a
/// HARD refusal (the shadow counterpart of the registered
/// `file_write_shadow_on_signer_unreachable` test) — `signer_unreachable`
/// is journaled and the tool never starts.
#[tokio::test]
async fn fail_closed_signer_unreachable_blocks_file_write() {
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
            output_encoding: None,
            structured: None,
            ..Default::default()
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
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "改文件",
            "RUN-FC-DEAD",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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
        "dead signer must journal rejections: {types:?}"
    );
    assert!(
        rejected
            .iter()
            .any(|e| e.payload["reject_code"] == "signer_unreachable"),
        "expected signer_unreachable: {:?}",
        rejected.iter().map(|e| &e.payload).collect::<Vec<_>>()
    );
    assert!(
        !types.iter().any(|t| t == "tool_started"),
        "fail-closed must not start the tool: {types:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// D-13 (2026-08-13): a retrieval-lane web_fetch binds its REAL activation
/// on the network_v1 ticket (previously null). The subagent's lane
/// self-execution path carries `activation_id` from the loop profile.
#[tokio::test]
async fn fail_closed_retrieval_lane_web_fetch_binds_activation_d13() {
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
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        // Main: delegate to the external retrieval lane.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_fetch".to_string(),
            arguments: serde_json::json!({ "url": "https://example.com/doc" }),
            call_id: "call-1".to_string(),
        }]),
        // Subagent: lane self-execution — the REAL web_fetch host call.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "web_fetch".to_string(),
            arguments: serde_json::json!({ "url": "https://example.com/doc" }),
            call_id: "call-s1".to_string(),
        }]),
        ScriptedResponse::text("[SOURCE] https://example.com/doc\n内容"),
        // Main: close the external activation.
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "retrieval_disposition".to_string(),
            arguments: serde_json::json!({
                "role": "external_retrieval",
                "decision": "close",
            }),
            call_id: "call-d1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "抓取文档",
            "RUN-ACAF-D13",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
        .await
        .expect("run turn");

    let events = events(&dir);
    let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
    let issued: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_issued")
        .collect();
    let network_issues: Vec<_> = issued
        .iter()
        .filter(|e| e.payload["ticket_kind"] == "network_v1")
        .collect();
    assert_eq!(
        network_issues.len(),
        1,
        "exactly one lane network ticket: {types:?}"
    );
    let activation = network_issues[0].payload["activation_id"]
        .as_str()
        .expect("D-13: network ticket must bind the lane activation");
    assert!(
        activation.starts_with("retrieval-external_retrieval-") && activation.ends_with("-00"),
        "unexpected activation binding: {activation}"
    );
    // The ticket still pairs to a consumed terminal.
    let consumed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_consumed")
        .collect();
    assert!(
        consumed.iter().any(|e| {
            e.payload["ticket_id"] == network_issues[0].payload["ticket_id"]
                && e.payload["ticket_kind"] == "network_v1"
        }),
        "network ticket must be consumed: {:?}",
        consumed.iter().map(|e| &e.payload).collect::<Vec<_>>()
    );
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert!(rejected.is_empty(), "happy path — no rejections: {types:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// D-16 happy path under fail-closed: an ACCEPTED continue consumes BOTH
/// the DispositionV1 and the GoalRevisionV1 tickets and only THEN migrates
/// the goal binding / contract revision (the fail-closed gate is
/// transparent when every ticket verifies).
#[tokio::test]
async fn fail_closed_continue_consumes_goal_revision_ticket() {
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
            output: "ok".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: None,
            ..Default::default()
        }),
        test_runner: None,
    };

    // R1 (2026-08-27, 设计 §4.4): disposition 往返退役——GoalRevisionV1
    // 仅经跨 run 恢复的 AwaitingDisposition 激活（休眠 handler）可达。
    // 种子一个上一 run 遗留的待决激活，脚本直接提交 continue。
    let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "retrieval_disposition".to_string(),
            arguments: serde_json::json!({
                "role": "internal_retrieval",
                "decision": "continue",
                "requirement_delta": "补充检索第二批",
            }),
            call_id: "call-d1".to_string(),
        }]),
        ScriptedResponse::text("完成"),
        ScriptedResponse::text("完成"),
    ]));
    let controller = AgentLoopController::with_gateway(gateway)
        .with_activation_snapshot(Some(&seeded_internal_activation_snapshot()))
        .with_retrieval_enabled(true)
        .with_snapshot_store(Some(store))
        .with_acaf(Some(client))
        .with_acaf_fail_closed(true);
    controller
        .run_turn(
            &host,
            "检索项目",
            "RUN-FC-CONT",
            MANIFEST,
            0,
            None,
            None,
            None,
        )
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
    let goal_issues: Vec<_> = issued
        .iter()
        .filter(|e| e.payload["ticket_kind"] == "goal_revision_v1")
        .collect();
    assert_eq!(
        goal_issues.len(),
        1,
        "continue must carry a GoalRevisionV1 ticket: {types:?}"
    );
    assert!(
        consumed
            .iter()
            .any(|e| e.payload["ticket_id"] == goal_issues[0].payload["ticket_id"]),
        "GoalRevisionV1 must be consumed: {:?}",
        consumed.iter().map(|e| &e.payload).collect::<Vec<_>>()
    );
    let rejected: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "control_ticket_rejected")
        .collect();
    assert!(rejected.is_empty(), "happy path — no rejections: {types:?}");
    // The disposition tool reports the accepted continue (exit 0).
    let completed: Vec<_> = events
        .iter()
        .filter(|e| e.event_type.to_string() == "tool_completed")
        .collect();
    assert!(
        completed.iter().any(|e| {
            e.payload["tool"] == "retrieval_disposition" && e.payload["exit_code"] == 0
        }),
        "continue accepted under fail-closed: {:?}",
        completed.iter().map(|e| &e.payload).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
