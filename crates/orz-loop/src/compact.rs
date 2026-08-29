//! Context-compaction parameter surface — batch B3 of the controller split
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use crate::controller::{
    AgentLoopController, DEFAULT_FOLD_TAIL_TOKENS, DEFAULT_FOLD_TRIGGER_TOKENS, chrono_utc_now,
};
use crate::gateway::model::{Message, Role};
use crate::host::LoopHost;

/// P0-D (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §2-§6):
/// context-compaction parameters for the redesigned mechanism — the A6
/// parameter set (90K target / 20-round cooldown / 250K fallback) is
/// revoked.
///
/// - Template summary (S3): fires at any loop-top gap when the previous
///   round's MEASURED prompt tokens exceed `trigger_tokens` (192K —
///   2026-08-18 adjudication, ADR-0010 §14.26) with a ≥`min_rounds`
///   (2 model rounds — review fix 2026-08-14) cooldown, or exceed
///   `safety_tokens` (256K fallback — 2026-08-18 adjudication, ADR-0010
///   §14.26) regardless of cooldown.
///   Reduction guards: removable content ≥ `min_compactable` (5K) and kept
///   ≤ `max_reduction_ratio` (0.6) of before. A guard that cannot be
///   satisfied retries across trigger rounds and forces one compaction
///   after `GUARD_RETRY_LIMIT` failures (`guard_failed`). The summary
///   output is a five-section template (≤17K chars), archived under
///   `.gsa/compaction/` with a digest, and the rolling single marker
///   carries the pointer.
/// - Mechanical collapse (S2): every completed OLD tool round collapses
///   into a deterministic action-ledger row in the MODEL-VISIBLE request
///   (zero model calls, `recent_tail_rounds` kept verbatim); the
///   persisted conversation keeps the full records. FUS-LEDGER-FOLD-STATE
///   (2026-08-18, ADR-0010 §14.26): the collapse is stateful — the fold
///   point advances only when the estimated request view reaches
///   `fold_trigger_tokens` (128K), and between advances the request view
///   prefix is byte-stable (pure append), restoring the v1.9 prefix-cache
///   discipline that the old per-request stateless recomputation broke.
/// - Recovery pre-check (D2-2): a restored conversation estimated over
///   `recovery_trigger_tokens` (200K conservative) is mechanically
///   truncated toward `recovery_target_tokens` (160K) before the first
///   request, with the full sidecar copied into the run journal as the
///   audit copy.
///
/// `target_tokens` (legacy A6 90K) is retained only for the pure
/// `compact_messages` unit surface; the loop no longer uses it.
#[derive(Debug, Clone, Copy)]
pub struct ContextCompactConfig {
    pub trigger_tokens: u64,
    pub target_tokens: u64,
    pub min_rounds: u32,
    pub safety_tokens: u64,
    /// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): mechanical
    /// fold-advance threshold — when the ESTIMATED request view
    /// (folded view, chars/2) is ≥ this, the loop folds the next complete
    /// old rounds into the frozen ledger once (loop-top gap, checkpoint
    /// rounds first). Default 128K = the MRCR quality plateau boundary
    /// (V4-Flash-Max 0.870); env `ORZ_FOLD_TRIGGER_TOKENS` overrides.
    pub fold_trigger_tokens: u64,
    /// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32): the bridge
    /// real-token budget of the FOLDED request view after the fixed pointer
    /// message (default `DEFAULT_FOLD_TAIL_TOKENS` = 8K; env
    /// `ORZ_FOLD_TAIL_TOKENS` overrides). `fold_tail_rounds` semantics
    /// retired. Separate from `recent_tail_rounds` (the compaction drain
    /// tail, unchanged).
    pub fold_tail_tokens: u64,
    /// D2-2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6):
    /// a restored conversation is pre-checked before the first request;
    /// when the ESTIMATE exceeds this conservative threshold (min of the
    /// 224K effective input budget and the 200K fallback = 200K), old
    /// whole rounds are mechanically dropped toward `recovery_target_tokens`.
    pub recovery_trigger_tokens: u64,
    /// D2-2: recovery truncation target (160K — under the ordinary summary
    /// trigger, so the first measured round may then drive a template
    /// summary instead of another raw truncation).
    pub recovery_target_tokens: u64,
    /// P0-D review fix (2026-08-14, ADR-0010 v1.14): the end-of-session
    /// compaction gate — a successful run whose FULL conversation estimate
    /// exceeds this threshold compacts once before the sidecar write-back,
    /// pinning the summary marker into the persisted conversation (the
    /// restore pre-check remains the fallback for older sidecars).
    pub session_end_trigger_tokens: u64,
    /// P0-D S2/S3 (2026-08-14, ADR-0010 v1.10): bounded recent tail kept
    /// verbatim in the model-visible collapsed view / after a summary.
    pub recent_tail_rounds: usize,
    /// P0-D S3: minimum droppable content (tokens) for a template summary
    /// (reuse of orz-compaction `min_compactable` guard).
    pub min_compactable: u64,
    /// P0-D S3: maximum kept/before ratio — the summary must reduce by at
    /// least 40% (`max_reduction_ratio` 0.6; reuse of orz-compaction).
    pub max_reduction_ratio: f64,
}

impl Default for ContextCompactConfig {
    fn default() -> Self {
        Self {
            trigger_tokens: 192_000,
            target_tokens: 90_000,
            min_rounds: 2,
            safety_tokens: 256_000,
            fold_trigger_tokens: DEFAULT_FOLD_TRIGGER_TOKENS,
            fold_tail_tokens: DEFAULT_FOLD_TAIL_TOKENS,
            recovery_trigger_tokens: 200_000,
            recovery_target_tokens: 160_000,
            session_end_trigger_tokens: 160_000,
            recent_tail_rounds: 2,
            min_compactable: 5_000,
            max_reduction_ratio: 0.6,
        }
    }
}

impl AgentLoopController {
    /// A6 (2026-08-08): override the explicit context-compaction parameters
    /// (tests use tiny values; production keeps the design §5 A6 defaults).
    pub fn with_context_compact(
        mut self,
        trigger_tokens: u64,
        target_tokens: u64,
        min_rounds: u32,
        safety_tokens: u64,
    ) -> Self {
        self.context_compact = ContextCompactConfig {
            trigger_tokens,
            target_tokens,
            min_rounds,
            safety_tokens,
            // FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32): 结构更新
            // 不再重置 fold_tail_tokens ——`with_fold_tail_tokens` 与
            // `with_context_compact` 的调用顺序从此无关（此前
            // `..Default::default()` 会把 fold_tail_rounds 重置回 1）。
            fold_tail_tokens: self.context_compact.fold_tail_tokens,
            ..ContextCompactConfig::default()
        };
        self
    }

    /// D2-2 (2026-08-14): override the recovery pre-check threshold/target
    /// (tests use tiny values; production keeps 200K/160K).
    pub fn with_recovery_compact(mut self, trigger_tokens: u64, target_tokens: u64) -> Self {
        self.context_compact.recovery_trigger_tokens = trigger_tokens;
        self.context_compact.recovery_target_tokens = target_tokens;
        self
    }

    /// P0-D review fix (2026-08-14): override the end-of-session compaction
    /// gate (tests use tiny values; production keeps 160K).
    pub fn with_session_end_trigger(mut self, tokens: u64) -> Self {
        self.context_compact.session_end_trigger_tokens = tokens;
        self
    }

    /// P0-D S2/S3: override the recent-tail length (tests use small values;
    /// production keeps 2).
    pub fn with_recent_tail(mut self, rounds: usize) -> Self {
        self.context_compact.recent_tail_rounds = rounds;
        self
    }

    /// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): pin the
    /// fold-advance threshold (test seam; production reads
    /// `ORZ_FOLD_TRIGGER_TOKENS` at construction, default 128K).
    pub fn with_fold_trigger_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.fold_trigger_tokens = tokens.max(1);
        self
    }

    /// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32): pin the bridge
    /// real-token budget (test seam; production reads
    /// `ORZ_FOLD_TAIL_TOKENS` at construction, default 8K).
    pub fn with_fold_tail_tokens(mut self, tokens: u64) -> Self {
        self.context_compact.fold_tail_tokens = tokens.max(1);
        self
    }

    /// P0-D S3: override the summary reduction guards (tests relax them).
    pub fn with_summary_guards(mut self, min_compactable: u64, max_reduction_ratio: f64) -> Self {
        self.context_compact.min_compactable = min_compactable;
        self.context_compact.max_reduction_ratio = max_reduction_ratio;
        self
    }

    /// A6 §8 C.2 (2026-08-08): override the whitelist character cap
    /// (tests use small values; production keeps DEFAULT_WHITELIST_CAP).
    pub fn with_whitelist_cap(mut self, cap: usize) -> Self {
        self.whitelist_cap = cap;
        self
    }

    /// A6 §8 C.2: keep the resident whitelist message in the conversation's
    /// preamble zone (after the original prompt, before the first tool
    /// declaration) — the compaction mechanism's always-kept preamble then
    /// skips it automatically (user decision: 常驻被压缩机制跳过, never
    /// re-injected at compaction time). New entries update the existing
    /// whitelist message in place.
    pub(crate) fn upsert_whitelist_message(&self, messages: &mut Vec<Message>) {
        let entries = self.whitelist.lock().unwrap();
        if entries.is_empty() {
            return;
        }
        let content = crate::prompt::build_whitelist_block(&entries);
        if let Some(i) = messages
            .iter()
            .position(|m| m.content.starts_with(crate::prompt::WHITELIST_PREFIX))
        {
            messages[i].content = content;
            return;
        }
        let pos = messages
            .iter()
            .position(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
            .unwrap_or(messages.len());
        messages.insert(
            pos,
            Message {
                role: Role::User,
                content,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            },
        );
    }

    /// A6 §8 C.2: mechanical best-effort archive of a whitelist write to
    /// `{journal_dir}/whitelist.jsonl` (JSONL: timestamp + content, plain
    /// text — user decision 明文存档). The run dir is covered by the A5
    /// retention sweep (7 days), so A5 needs no changes. Runtime compaction
    /// reads the in-memory list, never this file; an archive failure is
    /// logged and never blocks the write.
    ///
    /// Review decision (2026-08-08): credential-shaped content is scanned
    /// (`looks_like_api_key`, GAK-CRED-001's detector) before the archive
    /// append — a hit logs a warning as the audit trail but does NOT block
    /// the write (best-effort semantics unchanged; the whitelist is
    /// model-chosen task content, and the scan is a surfaced warning, not
    /// a gate).
    pub(crate) fn archive_whitelist_entry(&self, host: &dyn LoopHost, content: &str) {
        if orz_assurance::credential::looks_like_api_key(content) {
            tracing::warn!(
                "whitelist entry looks credential-shaped (archived anyway — \
                 .gsa is gitignored, retained 7 days by A5)"
            );
        }
        let line = serde_json::json!({
            "timestamp": chrono_utc_now(),
            "content": content,
        });
        let path = host.journal().journal_dir().join("whitelist.jsonl");
        let mut line = line.to_string();
        line.push('\n');
        if let Err(e) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
        {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "whitelist archive append failed (best-effort)"
            );
        }
    }
}
