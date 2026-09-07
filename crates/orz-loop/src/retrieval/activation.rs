//! Retrieval activation registry + lifecycle persistence — batch B4 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use orz_assurance::{EventType, sha256_hex};

use crate::agents::SubagentRole;
use crate::controller::{AgentLoopController, AgentLoopError, EventWriter, PendingDisposition};
use crate::gateway::model::{Message, Role};
use crate::prompt::{is_injected_block_text, is_restore_retained_block};
use crate::retrieval::effort::EffortTier;
use crate::retrieval::evidence::StructuredCommittedResult;

impl AgentLoopController {
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): best-effort structured-result
    /// artifact persist — `{journal_dir}/retrieval-results/
    /// {activation_id}-r{rev}-{digest8}.json` (ADR-0010 §3.3.5 archive_ref;
    /// the close record cites the real artifact). Failures only WARN — the
    /// journal event is the authoritative record.
    pub(crate) fn persist_result_artifact(
        &self,
        writer: &EventWriter<'_>,
        committed: &StructuredCommittedResult,
        activation_id: &str,
        contract_revision: u32,
    ) -> Option<String> {
        let journal_dir = writer.journal_dir()?;
        let dir = journal_dir.join("retrieval-results");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!("result artifact dir create failed ({}): {e}", dir.display());
            return None;
        }
        let name = format!(
            "{activation_id}-r{contract_revision}-{}.json",
            &committed.result_digest[..8]
        );
        let path = dir.join(&name);
        match serde_json::to_string_pretty(&committed.payload) {
            // 0p S2 复审 P1-2 修复（B5 第 5 漏斗，2026-09-07，ADR-0010
            // §14.61）：检索结果工件与 journal 同目录（runs/<run>/
            // retrieval-results/），却绕过 journal record 漏斗直写盘——
            // 落盘前接 orz-secrets 机械脱敏，key 不落卷不变量闭合旁路。
            Ok(json) => {
                match std::fs::write(&path, orz_secrets::redact_secrets(&json).as_bytes()) {
                    Ok(()) => Some(path.to_string_lossy().to_string()),
                    Err(e) => {
                        tracing::warn!("result artifact write failed ({}): {e}", path.display());
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("result artifact serialize failed: {e}");
                None
            }
        }
    }
}

/// GAP-SUBAGENT-RUNTIME (2026-08-10) — ADR-0010 §3.3 subagent lifecycle
/// state (per-role; one activation per role, §11.3 one seat per role).
#[derive(Debug, Default)]
pub(crate) struct ActivationRegistry {
    /// Monotonic per-role sequence — the `activation_id` suffix.
    pub(crate) next_seq: HashMap<SubagentRole, u32>,
    /// Live activations. The state is REMOVED while a retrieval task is
    /// running (so the std::Mutex guard never crosses an await) and
    /// re-inserted when the task ends; a Closed activation is replaced by
    /// the next creation (new seq).
    pub(crate) states: HashMap<SubagentRole, ActivationState>,
}

impl ActivationRegistry {
    #[allow(dead_code)] // read by the M4 disposition handler
    pub(crate) fn state(&self, role: SubagentRole) -> Option<&ActivationState> {
        self.states.get(&role)
    }

    /// FUS-TOOL-PROBE P0-A-2 (design §2.0; 审查复核 2026-08-13):
    /// `retrieval_disposition` probe chain — complete only while an
    /// activation carries an UNDISPOSED pending assessment. Active
    /// mid-task, Closed and restored-without-assessment activations have
    /// nothing to dispose, so they fail closed; after an accepted continue
    /// the pending stays as the consumed (decided) record, also incomplete.
    /// The call-time gate remains the final backstop for any disposition
    /// the probe still over-approximates (design invariant 2).
    pub(crate) fn has_live(&self) -> bool {
        self.states
            .values()
            .any(|a| a.pending.as_ref().is_some_and(|p| p.decided.is_none()))
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): serialize the live registry to the
    /// sidecar snapshot JSON (Closed activations excluded — the next
    /// creation starts a new seq). `origin_run_id` records where each
    /// activation was last active (the restore audit route).
    pub(crate) fn snapshot_json(&self, origin_run_id: &str) -> serde_json::Value {
        let snapshot = StoredActivationSnapshot {
            next_seq: self
                .next_seq
                .iter()
                .map(|(role, seq)| (role.as_str().to_string(), *seq))
                .collect(),
            activations: self
                .states
                .values()
                .filter(|a| a.status != ActivationStatus::Closed)
                .map(|a| StoredActivation {
                    activation_id: a.activation_id.clone(),
                    parent_session_id: a.parent_session_id.clone(),
                    subagent_session_id: a.subagent_session_id.clone(),
                    contract_id: a.contract_id.clone(),
                    contract_revision: a.contract_revision,
                    status: a.status,
                    tool_rounds_used: a.tool_rounds_used,
                    candidate_urls: a.candidate_urls.clone(),
                    result_digest: a.result_digest.clone(),
                    result_archive_ref: a.result_archive_ref.clone(),
                    next_goal: a.next_goal.clone(),
                    pending_assessment_id: a.pending.as_ref().map(|p| p.assessment_id.clone()),
                    pending_expected_contract_revision: a
                        .pending
                        .as_ref()
                        .map(|p| p.expected_contract_revision)
                        .unwrap_or(0),
                    origin_run_id: origin_run_id.to_string(),
                    // Review D2-1/P3-4 (three-agent 2026-08-10): the
                    // subagent conversation gets the same injection-block
                    // filter as the main lane (the shared loop injects
                    // budget/orientation blocks into it too — persisting
                    // them would replay stale mechanical scaffolding on
                    // restore).
                    conversation: a
                        .conversation
                        .iter()
                        // D3-1 (2026-08-14): same restore-retention rule as
                        // the main lane — marker/whitelist survive, other
                        // mechanical injected blocks stay filtered.
                        .filter(|m| {
                            !(m.role == Role::User
                                && is_injected_block_text(&m.content)
                                && !is_restore_retained_block(&m.content))
                        })
                        .cloned()
                        .collect(),
                })
                .collect(),
        };
        serde_json::to_value(snapshot).unwrap_or(serde_json::Value::Null)
    }

    /// Seed the registry from a sidecar snapshot; returns the restored
    /// activations (journaled as `retrieval_activation_restored` by the
    /// caller). A malformed snapshot seeds nothing (the sidecar load already
    /// warned).
    pub(crate) fn seed_from_json(&mut self, value: &serde_json::Value) -> Vec<StoredActivation> {
        let Ok(snapshot) = serde_json::from_value::<StoredActivationSnapshot>(value.clone()) else {
            return Vec::new();
        };
        for (role_str, seq) in &snapshot.next_seq {
            if let Some(role) = subagent_role_from_str(role_str) {
                self.next_seq.insert(role, *seq);
            }
        }
        let mut restored = Vec::new();
        for stored in snapshot.activations {
            // The role rides the activation_id (`retrieval-{role}-…`).
            let role = stored
                .activation_id
                .split('-')
                .nth(1)
                .and_then(subagent_role_from_str);
            let Some(role) = role else {
                tracing::warn!(
                    "activation sidecar: unparseable activation_id {} — skipped",
                    stored.activation_id
                );
                continue;
            };
            if self.states.contains_key(&role) {
                tracing::warn!(
                    "activation sidecar: duplicate activation for role {} — skipped",
                    role.as_str()
                );
                continue;
            }
            let pending = stored
                .pending_assessment_id
                .as_ref()
                .map(|a_id| PendingDisposition {
                    assessment_id: a_id.clone(),
                    expected_contract_revision: stored.pending_expected_contract_revision,
                    decided: None,
                });
            // GAP-CONVERSATION-RESTORE (2026-08-10): the conversation rides
            // the sidecar now — a restored activation resumes with its full
            // subagent context (registered boundary closed; the `submitted`
            // replay ledger still does not ride the sidecar — cross-run
            // idempotent replay stays in-process, D-6 update).
            self.states.insert(
                role,
                ActivationState {
                    activation_id: stored.activation_id.clone(),
                    parent_session_id: stored.parent_session_id.clone(),
                    subagent_session_id: stored.subagent_session_id.clone(),
                    contract_id: stored.contract_id.clone(),
                    contract_revision: stored.contract_revision,
                    status: stored.status,
                    conversation: stored.conversation.clone(),
                    pending,
                    next_goal: stored.next_goal.clone(),
                    result_digest: stored.result_digest.clone(),
                    submitted: Vec::new(),
                    tool_rounds_used: stored.tool_rounds_used,
                    candidate_urls: stored.candidate_urls.clone(),
                    result_archive_ref: stored.result_archive_ref.clone(),
                    // 恢复激活不携带 effort（下一次派发重新计算并覆盖）。
                    effort: None,
                },
            );
            restored.push(stored);
        }
        restored
    }
}

/// Role from the wire/snapshot string (`internal_retrieval` /
/// `external_retrieval`).
fn subagent_role_from_str(s: &str) -> Option<SubagentRole> {
    match s {
        "internal_retrieval" => Some(SubagentRole::InternalRetrieval),
        "external_retrieval" => Some(SubagentRole::ExternalRetrieval),
        _ => None,
    }
}

/// One retrieval activation (ADR-0010 §3.3 state machine).
#[derive(Debug)]
pub(crate) struct ActivationState {
    pub activation_id: String,
    pub parent_session_id: String,
    pub subagent_session_id: String,
    pub contract_id: String,
    pub contract_revision: u32,
    pub status: ActivationStatus,
    /// The subagent session's conversation — persists across rounds and
    /// across `continue` iterations of the same activation (multi-turn,
    /// §3.3.2); preserved on every terminal path (§4.4: never deleted on
    /// reset).
    pub conversation: Vec<Message>,
    /// The assessment awaiting (or already receiving) a parent
    /// disposition — `Some` only in `AwaitingDisposition`.
    pub pending: Option<PendingDisposition>,
    /// `continue(requirement_delta)` — the next retrieval task's goal.
    pub next_goal: Option<String>,
    /// Most recent result digest (close record / §4.4 idempotency key).
    pub result_digest: Option<String>,
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：本激活
    /// 最近一次派发的委托契约 effort 档（机械分档；close record 登记可选
    /// `effort` 字段）。`continue` 重入覆盖为最新档。
    pub effort: Option<EffortTier>,
    /// Submitted disposition ids → canonical full payloads, ACROSS
    /// assessments (activation-lifetime). §4.4 replay idempotency: a
    /// replayed id must journal byte-identical payload — a late replay of
    /// a pre-continue disposition must be recognized even after the
    /// pending was replaced.
    pub submitted: Vec<(String, Vec<u8>)>,
    /// Tool rounds consumed by this activation's sessions (user
    /// adjudication 2026-08-10, review F5): a `continue` re-entry is the
    /// SAME retrieval session — the 120-round budget accumulates across
    /// dispatches and resets only when the activation closes (a new
    /// activation starts at 0). Read as `initial_tool_rounds` by the
    /// shared loop and written back from `LoopOutcome.tool_rounds`.
    pub tool_rounds_used: u32,
    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the activation's
    /// candidate URLs — exact-string dedup, first-seen order,
    /// per-activation accumulation (design §1.1; shared count domain for
    /// the web_fetch family + browser_read, design §1.3). A `continue`
    /// re-entry is the SAME retrieval session, so the count accumulates
    /// across dispatches and resets only when the activation closes; it
    /// rides the sidecar like `tool_rounds_used` (cross-run restore keeps
    /// the cap meaningful). Moved into the dispatch's shared counter while
    /// the subagent loop runs and written back on every path.
    pub candidate_urls: Vec<String>,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): the committed structured result's
    /// artifact path (ADR-0010 §3.3.5 archive_ref — the close record cites
    /// the real artifact instead of the `run-journal:{run_id}` placeholder).
    /// `None` when no result was committed (terminal closes keep the
    /// journal reference).
    pub result_archive_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivationStatus {
    /// Retrieval may run (new task or continue).
    Active,
    /// Result formed + assessed — the parent must submit a structured
    /// disposition before any further retrieval (ADR-0010 §4.4).
    AwaitingDisposition,
    /// Close committed — live state cleared; the next retrieval creates
    /// a new activation (new seq, revision 0).
    Closed,
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): the serializable activation snapshot —
/// persisted via the acp_server sidecar (`.gsa/activations/<session8>.json`).
/// State-machine fields ride the sidecar; GAP-CONVERSATION-RESTORE (2026-08-10)
/// adds the subagent conversation (a restored activation resumes with its
/// context). The in-process replay ledger (`submitted`) still does not ride
/// the sidecar (registered boundary: cross-run disposition replay-idempotency
/// is in-process only — a new run's dispositions derive fresh ids from their
/// call ids).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct StoredActivationSnapshot {
    #[serde(default)]
    pub next_seq: HashMap<String, u32>,
    #[serde(default)]
    pub activations: Vec<StoredActivation>,
}

/// One persisted activation (state-machine fields + conversation).
///
/// GAP-CONVERSATION-RESTORE (2026-08-10): `conversation` rides the sidecar
/// now — a restored activation resumes with its full subagent context
/// (ADR-0010 §3.1: the recovery mechanism is shared by all three agents).
/// The `submitted` in-process replay ledger STILL does not ride the sidecar
/// (D-6 update — cross-run idempotent replay stays in-process; disposition
/// ids derive from call ids, naturally collision-free). `#[serde(default)]`
/// keeps old sidecars (no `conversation` key) parseable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredActivation {
    pub activation_id: String,
    pub parent_session_id: String,
    pub subagent_session_id: String,
    pub contract_id: String,
    pub contract_revision: u32,
    pub status: ActivationStatus,
    pub tool_rounds_used: u32,
    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the activation's
    /// deduplicated candidate URLs (shared count domain — web_fetch family
    /// + browser_read) — rides the sidecar so a cross-run `continue`
    /// resumes with the same candidate budget (matches `tool_rounds_used`
    /// lifecycle). `#[serde(default)]` keeps old sidecars parseable.
    #[serde(default)]
    pub candidate_urls: Vec<String>,
    #[serde(default)]
    pub result_digest: Option<String>,
    #[serde(default)]
    pub result_archive_ref: Option<String>,
    #[serde(default)]
    pub next_goal: Option<String>,
    #[serde(default)]
    pub pending_assessment_id: Option<String>,
    #[serde(default)]
    pub pending_expected_contract_revision: u32,
    /// The run where this activation was last active (restore audit route).
    #[serde(default)]
    pub origin_run_id: String,
    /// GAP-CONVERSATION-RESTORE: the subagent's conversation (state-machine
    /// companion) — resumes across runs via the activation sidecar.
    #[serde(default)]
    pub conversation: Vec<Message>,
}

impl AgentLoopController {
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): journal one `retrieval_activation_
    /// restored` event per seeded awaiting-disposition activation — the
    /// parent disposition may then close/continue it across runs (the
    /// verifier resolves the assessment reference through the declaration).
    /// Called at the run startup (after the mode transition, before the
    /// availability gate). Cleared after journaling.
    pub(crate) async fn journal_activation_restores(
        &self,
        writer: &mut EventWriter<'_>,
    ) -> Result<(), AgentLoopError> {
        let restores: Vec<StoredActivation> =
            std::mem::take(&mut *self.restored_activations.lock().unwrap());
        let session_id = self.session_id.clone().unwrap_or_default();
        // M3 (review 2026-08-10): chars().take(8), not a byte slice — a
        // multi-byte UTF-8 session id would panic on a non-char boundary.
        // Matches the sidecar path derivation in acp_server.
        let sidecar_ref = format!(
            ".gsa/activations/{}.json",
            session_id.chars().take(8).collect::<String>()
        );
        for (i, stored) in restores.iter().enumerate() {
            let restore_id = format!(
                "RST-ACT-{}-{:02}",
                &sha256_hex(stored.activation_id.as_bytes())[..16],
                i,
            );
            let mut payload = serde_json::json!({
                "restore_id": restore_id,
                "activation_id": stored.activation_id,
                "subagent_session_id": stored.subagent_session_id,
                "contract_id": stored.contract_id,
                "contract_revision": stored.contract_revision,
                "status": match stored.status {
                    ActivationStatus::AwaitingDisposition => "awaiting_disposition",
                    _ => "active",
                },
                "origin_run_id": stored.origin_run_id,
                "sidecar_ref": sidecar_ref,
                "tool_rounds_used": stored.tool_rounds_used,
            });
            if let Some(a_id) = &stored.pending_assessment_id {
                payload["assessment_id"] = serde_json::Value::String(a_id.clone());
            }
            if let Some(digest) = &stored.result_digest {
                payload["result_digest"] = serde_json::Value::String(digest.clone());
            }
            writer
                .record(EventType::RetrievalActivationRestored, payload)
                .await?;
        }
        Ok(())
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): the live registry as sidecar JSON
    /// (Closed excluded) — the acp_server persists it after each run.
    pub fn activation_snapshot_json(&self, origin_run_id: &str) -> serde_json::Value {
        self.activations
            .lock()
            .unwrap()
            .snapshot_json(origin_run_id)
    }

    /// MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): the total
    /// deduplicated retrieval candidate count across live activations plus
    /// the shared candidate cap — the audit report's `retrieval:<n>`
    /// 候选/上限 fact. `None` when no activation carries a candidate
    /// count yet (pure read, never blocks an await).
    pub(crate) fn retrieval_candidate_count(&self) -> Option<(usize, u32)> {
        let reg = self.activations.lock().unwrap();
        let total: usize = reg.states.values().map(|a| a.candidate_urls.len()).sum();
        if total == 0 {
            None
        } else {
            Some((total, self.candidate_cap))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::SubagentRole;
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{Message, ModelGateway, Role, ToolCall};
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;

    // ── GAP-RETRIEVAL-TOOLS (2026-08-10): cross-run activation persistence ──

    /// The registry snapshot round-trips: seed → snapshot → seed keeps the
    /// state-machine fields (status/pending/digest/budget); Closed
    /// activations are excluded from the snapshot.
    #[test]
    fn activation_snapshot_round_trips_state_machine_fields() {
        let mut registry = ActivationRegistry::default();
        registry.next_seq.insert(SubagentRole::InternalRetrieval, 3);
        registry.states.insert(
            SubagentRole::InternalRetrieval,
            ActivationState {
                activation_id: "retrieval-internal_retrieval-sess-abc-02".to_string(),
                parent_session_id: "sess-p".to_string(),
                subagent_session_id: "SUB-internal_retrieval-sess-abc".to_string(),
                contract_id: "retrieval-contract-internal_retrieval".to_string(),
                contract_revision: 1,
                status: ActivationStatus::AwaitingDisposition,
                conversation: vec![Message {
                    role: Role::User,
                    content: "会话内容不入侧车".to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                }],
                pending: Some(PendingDisposition {
                    assessment_id: "ASSESS-1".to_string(),
                    expected_contract_revision: 1,
                    decided: None,
                }),
                next_goal: None,
                result_digest: Some("a".repeat(64)),
                submitted: vec![("DISP-1".to_string(), vec![1, 2, 3])],
                tool_rounds_used: 7,
                candidate_urls: vec![
                    "https://a.example".to_string(),
                    "https://b.example".to_string(),
                ],
                result_archive_ref: Some(".gsa/runs/RUN-X/retrieval-results/a.json".to_string()),
                effort: None,
            },
        );
        // A Closed activation must NOT ride the snapshot.
        registry.states.insert(
            SubagentRole::ExternalRetrieval,
            ActivationState {
                activation_id: "retrieval-external_retrieval-sess-abc-00".to_string(),
                parent_session_id: "sess-p".to_string(),
                subagent_session_id: "SUB-external_retrieval-sess-abc".to_string(),
                contract_id: "retrieval-contract-external_retrieval".to_string(),
                contract_revision: 0,
                status: ActivationStatus::Closed,
                conversation: Vec::new(),
                pending: None,
                next_goal: None,
                result_digest: None,
                submitted: Vec::new(),
                tool_rounds_used: 0,
                candidate_urls: Vec::new(),
                result_archive_ref: None,
                effort: None,
            },
        );

        let json = registry.snapshot_json("RUN-ORIGIN");
        let mut seeded = ActivationRegistry::default();
        let restored = seeded.seed_from_json(&json);
        assert_eq!(restored.len(), 1);
        assert_eq!(
            seeded.next_seq.get(&SubagentRole::InternalRetrieval),
            Some(&3)
        );
        let act = seeded.states.get(&SubagentRole::InternalRetrieval).unwrap();
        assert_eq!(
            act.activation_id,
            "retrieval-internal_retrieval-sess-abc-02"
        );
        assert_eq!(act.contract_revision, 1);
        assert_eq!(act.status, ActivationStatus::AwaitingDisposition);
        assert_eq!(act.pending.as_ref().unwrap().assessment_id, "ASSESS-1");
        assert_eq!(act.tool_rounds_used, 7);
        assert_eq!(
            act.result_archive_ref.as_deref(),
            Some(".gsa/runs/RUN-X/retrieval-results/a.json")
        );
        // P0-B step 2 (2026-08-14): the web_fetch candidate count rides
        // the sidecar — a restored activation resumes the same count
        // (cross-run `continue` keeps the cap meaningful).
        assert_eq!(
            act.candidate_urls,
            vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ]
        );
        // GAP-CONVERSATION-RESTORE (2026-08-10): the conversation now rides
        // the sidecar (D-6 update — it round-trips intact); the replay
        // ledger (`submitted`) still does not ride the sidecar.
        assert_eq!(act.conversation.len(), 1);
        assert_eq!(act.conversation[0].content, "会话内容不入侧车");
        assert!(act.submitted.is_empty());
        // Closed excluded.
        assert!(!seeded.states.contains_key(&SubagentRole::ExternalRetrieval));
    }

    #[test]
    fn has_live_requires_undisposed_pending_assessment() {
        // P0-A-2 审查复核 (2026-08-13): `retrieval_disposition` 探针只
        // 在激活携带未决 pending assessment 时完整 — Active 无 pending、
        // accepted continue 后 pending 已决、Closed 均无待处置内容。
        let mut registry = ActivationRegistry::default();
        let state =
            |status: ActivationStatus, pending: Option<PendingDisposition>| ActivationState {
                activation_id: "retrieval-internal_retrieval-sess-abc-00".to_string(),
                parent_session_id: "sess-p".to_string(),
                subagent_session_id: "SUB-internal_retrieval-sess-abc".to_string(),
                contract_id: "retrieval-contract-internal_retrieval".to_string(),
                contract_revision: 0,
                status,
                conversation: Vec::new(),
                pending,
                next_goal: None,
                result_digest: None,
                submitted: Vec::new(),
                tool_rounds_used: 0,
                candidate_urls: Vec::new(),
                result_archive_ref: None,
                effort: None,
            };
        let undisposed = Some(PendingDisposition {
            assessment_id: "ASSESS-1".to_string(),
            expected_contract_revision: 0,
            decided: None,
        });
        let decided = Some(PendingDisposition {
            assessment_id: "ASSESS-1".to_string(),
            expected_contract_revision: 0,
            decided: Some("continue".to_string()),
        });

        assert!(!registry.has_live(), "empty registry");
        registry.states.insert(
            SubagentRole::InternalRetrieval,
            state(ActivationStatus::Active, None),
        );
        assert!(
            !registry.has_live(),
            "Active mid-task activation has nothing to dispose"
        );
        registry.states.insert(
            SubagentRole::InternalRetrieval,
            state(ActivationStatus::AwaitingDisposition, undisposed),
        );
        assert!(
            registry.has_live(),
            "AwaitingDisposition with an undisposed assessment is complete"
        );
        registry.states.insert(
            SubagentRole::InternalRetrieval,
            state(ActivationStatus::Active, decided),
        );
        assert!(
            !registry.has_live(),
            "accepted continue leaves no undisposed assessment"
        );
        registry.states.insert(
            SubagentRole::InternalRetrieval,
            state(ActivationStatus::Closed, None),
        );
        assert!(
            !registry.has_live(),
            "closed activation has nothing to dispose"
        );
    }

    /// A seeded AwaitingDisposition activation is journaled at the run
    /// startup (`retrieval_activation_restored`), and the parent's
    /// disposition can then CLOSE it across runs — the verifier resolves
    /// the assessment through the restore declaration.
    #[tokio::test]
    async fn restored_activation_journaled_and_disposable_across_runs() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
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
        let disposition_call = ToolCall {
            name: "retrieval_disposition".to_string(),
            arguments: serde_json::json!({
                "role": "internal_retrieval",
                "decision": "close",
            }),
            call_id: "call-d1".to_string(),
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![disposition_call]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway))
            .with_activation_snapshot(Some(&snapshot));
        controller
            .run_turn(&host, "关闭检索", "RUN-RES", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let types = event_types(&dir);
        // Restore event journaled at startup.
        let restores: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalActivationRestored)
            .collect();
        assert_eq!(restores.len(), 1, "{types:?}");
        let rp = &restores[0].payload;
        assert_eq!(
            rp["activation_id"],
            "retrieval-internal_retrieval-sess-abc-00"
        );
        assert_eq!(rp["status"], "awaiting_disposition");
        assert_eq!(rp["assessment_id"], "ASSESS-PREV-1");
        assert_eq!(rp["origin_run_id"], "RUN-PREV-0001");
        assert_eq!(rp["tool_rounds_used"], 3);
        // The restore precedes the disposition.
        let r_index = events
            .iter()
            .position(|e| e.event_type == EventType::RetrievalActivationRestored)
            .unwrap();
        let d_index = events
            .iter()
            .position(|e| e.event_type == EventType::RetrievalParentDisposition)
            .unwrap();
        assert!(r_index < d_index);
        // Cross-run close: disposition outcome accepted + close record.
        let disposition = events
            .iter()
            .find(|e| e.event_type == EventType::RetrievalParentDisposition)
            .unwrap();
        assert_eq!(disposition.payload["outcome"], "accepted");
        assert_eq!(disposition.payload["assessment_id"], "ASSESS-PREV-1");
        let close = events
            .iter()
            .find(|e| e.event_type == EventType::RetrievalCloseRecord)
            .unwrap();
        assert_eq!(close.payload["terminal_reason"], "normal_close");
        // The restored activation closes with the real artifact ref.
        assert!(
            close.payload["archive_ref"]
                .as_str()
                .unwrap()
                .starts_with("run-journal:"),
            "{}",
            close.payload["archive_ref"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ActivationState → snapshot_json → seed_from_json round-trips the
    /// conversation field-for-field.
    #[tokio::test]
    async fn stored_activation_roundtrip_with_conversation() {
        let controller =
            AgentLoopController::with_gateway(Arc::new(FakeProvider::from_texts(vec!["x"])));
        let state = ActivationState {
            activation_id: "retrieval-internal_retrieval-sess-abc-00".to_string(),
            parent_session_id: "sess-abcdef123456".to_string(),
            subagent_session_id: "SUB-internal_retrieval-sess-abc".to_string(),
            contract_id: "retrieval-contract-internal_retrieval".to_string(),
            contract_revision: 0,
            status: ActivationStatus::AwaitingDisposition,
            conversation: vec![conv_message(Role::User, "历史问")],
            pending: None,
            next_goal: None,
            result_digest: None,
            submitted: Vec::new(),
            tool_rounds_used: 1,
            candidate_urls: Vec::new(),
            result_archive_ref: None,
            effort: None,
        };
        controller
            .activations
            .lock()
            .unwrap()
            .states
            .insert(SubagentRole::InternalRetrieval, state);
        let json = controller
            .activations
            .lock()
            .unwrap()
            .snapshot_json("RUN-1");
        let c2 = AgentLoopController::with_gateway(Arc::new(FakeProvider::from_texts(vec!["x"])));
        let restored = c2.activations.lock().unwrap().seed_from_json(&json);
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].conversation.len(), 1);
        assert_eq!(restored[0].conversation[0].content, "历史问");
        assert_eq!(
            restored[0].activation_id,
            "retrieval-internal_retrieval-sess-abc-00"
        );
    }

    /// An OLD sidecar (no `conversation` key) still parses — `serde(default)`
    /// fills an empty conversation (backward compatibility).
    #[tokio::test]
    async fn stored_activation_old_sidecar_no_conversation_field() {
        let snapshot = serde_json::json!({
            "next_seq": {"internal_retrieval": 1},
            "activations": [{
                "activation_id": "retrieval-internal_retrieval-sess-abc-00",
                "parent_session_id": "sess-abcdef123456",
                "subagent_session_id": "SUB-internal_retrieval-sess-abc",
                "contract_id": "retrieval-contract-internal_retrieval",
                "contract_revision": 0,
                "status": "active",
                "tool_rounds_used": 0,
                "origin_run_id": "RUN-PREV-0001",
            }]
        });
        let controller =
            AgentLoopController::with_gateway(Arc::new(FakeProvider::from_texts(vec!["x"])));
        let mut registry = controller.activations.lock().unwrap();
        let restored = registry.seed_from_json(&snapshot);
        assert_eq!(restored.len(), 1);
        assert!(restored[0].conversation.is_empty(), "default empty");
        assert!(
            registry
                .states
                .get(&SubagentRole::InternalRetrieval)
                .unwrap()
                .conversation
                .is_empty()
        );
        drop(registry);
    }

    /// A restored activation with history continues from that history — the
    /// subagent's next request opens with the restored conversation and ends
    /// with the new goal (the registered boundary closes: cross-run continue
    /// no longer starts from scratch).
    #[tokio::test]
    async fn restored_activation_continue_seeds_history() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let snapshot = serde_json::json!({
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
                "conversation": [
                    {"role": "user", "content": "历史检索目标", "tool_call_id": null,
                     "tool_calls": [], "reasoning_content": null},
                    {"role": "assistant", "content": "历史结论", "tool_call_id": null,
                     "tool_calls": [], "reasoning_content": null},
                ]
            }]
        });
        let continue_call = ToolCall {
            name: "retrieval_disposition".to_string(),
            arguments: serde_json::json!({
                "role": "internal_retrieval",
                "decision": "continue",
                "requirement_delta": "补充检索：新需求",
            }),
            call_id: "call-c1".to_string(),
        };
        let retrieve_call = ToolCall {
            name: "retrieve_project_docs".to_string(),
            arguments: serde_json::json!({"query": "继续检索"}),
            call_id: "call-r1".to_string(),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![continue_call]),
            // Parent wrap-up after the accepted disposition.
            ScriptedResponse::text(""),
            // The parent re-dispatches the subagent (continue) — the
            // subagent's requests follow.
            ScriptedResponse::tool_calls(vec![retrieve_call]),
            ScriptedResponse::text("子代理继续完成"),
            ScriptedResponse::text("子代理继续完成"),
            // The parent's wrap-up round after the subagent returns.
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(fake.clone()))
            .with_activation_snapshot(Some(&snapshot));
        controller
            .run_turn(&host, "继续检索", "RUN-CONT", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        // The subagent request opened with the restored history and ended
        // with the new goal.
        let reqs = fake.received_requests();
        let subagent_req = reqs
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.content.contains("历史检索目标"))
            })
            .expect("subagent request with restored history");
        assert!(
            subagent_req
                .messages
                .iter()
                .any(|m| m.content.contains("补充检索")),
            "new goal appended: {:?}",
            subagent_req.messages
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
