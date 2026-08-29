//! Retrieval activation registry + lifecycle persistence — batch B4 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use orz_assurance::{EventType, sha256_hex};

use crate::agents::SubagentRole;
use crate::controller::{
    AgentLoopController, AgentLoopError, EventWriter, PendingDisposition, StructuredCommittedResult,
};
use crate::gateway::model::{Message, Role};
use crate::prompt::{is_injected_block_text, is_restore_retained_block};

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
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => Some(path.to_string_lossy().to_string()),
                Err(e) => {
                    tracing::warn!("result artifact write failed ({}): {e}", path.display());
                    None
                }
            },
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
