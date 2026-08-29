//! Context-compaction parameter surface — batch B3 of the controller split
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use crate::controller::{AgentLoopController, chrono_utc_now};
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

/// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the mechanical
/// fold-advance threshold — the estimated request view that triggers one
/// stateful fold advance at the loop-top gap. Default 128K (2026-08-18
/// user adjudication; MRCR-8-needle quality plateau boundary for
/// V4-Flash-Max; allows reading the full key-document set — index 30.5K
/// + ADR 43K + BACKLOG 27.8K ≈ 101K + preamble 8K — without a fold).
pub const DEFAULT_FOLD_TRIGGER_TOKENS: u64 = 128_000;

/// Env override for the fold-advance threshold
/// (`ORZ_FOLD_TRIGGER_TOKENS`). Parsed at controller construction;
/// absent/invalid/zero = the default.
pub fn fold_trigger_tokens_override() -> Option<u64> {
    std::env::var("ORZ_FOLD_TRIGGER_TOKENS")
        .ok()
        .and_then(|s| parse_fold_trigger_tokens(&s))
}

/// Pure parse rule for the fold-threshold env value (tested without env
/// mutation): trimmed, positive integer; absent/invalid/zero → None.
pub(crate) fn parse_fold_trigger_tokens(s: &str) -> Option<u64> {
    s.trim().parse().ok().filter(|v| *v > 0)
}

/// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32): the folded request
/// view keeps only the newest complete rounds within this real-token bridge
/// budget (`[U0][固定指针消息][桥]`); older rounds are appended to the
/// external ledger file as before. Default 8K (实测读/计划轮 1–3K、终端执行
/// 轮 5–7K——多数折叠时刻能整轮装下，截断为例外；S4 以折叠后首请求实际重付
/// 校准换算系数). `recent_tail_rounds` (compaction drain tail) is untouched.
/// Env `ORZ_FOLD_TAIL_TOKENS` overrides (trimmed positive integer;
/// absent/invalid/zero = default). 换算: 真实 token → 字符预算
/// (`action_ledger::FOLD_TAIL_CHARS_PER_TOKEN` = 2，S4 实测校准) → 估计口径
/// (`estimate_messages_tokens`, chars/2)。
pub fn fold_tail_tokens_override() -> Option<u64> {
    std::env::var("ORZ_FOLD_TAIL_TOKENS")
        .ok()
        .and_then(|s| parse_fold_tail_tokens(&s))
}

/// Pure parse rule for the bridge-budget env value (tested without env
/// mutation): trimmed, positive integer; absent/invalid/zero → None.
pub(crate) fn parse_fold_tail_tokens(s: &str) -> Option<u64> {
    s.trim().parse().ok().filter(|v| *v > 0)
}

/// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.6): 折叠后
/// 视图桥的默认真实 token 预算。8K 依据=S4 实测读/计划轮 1–3K、终端执行轮
/// 5–7K——多数折叠时刻能整轮装下（截断为例外；截断频率 >30% 视为桥偏小，
/// S4 校准）；4K 命中率仅多约 0.5pp 但会频繁截断正常终端轮；10K+ 收益递减。
/// `ORZ_FOLD_TAIL_TOKENS` 可配；实现按
/// `action_ledger::FOLD_TAIL_CHARS_PER_TOKEN`（2 字符/真实 token，S4 实测
/// 校准——path-tracing 07:08 运行桥 12,948 字符 → 重付 6,493）换算。
pub const DEFAULT_FOLD_TAIL_TOKENS: u64 = 8_000;

/// A6 §8 C.2 (2026-08-08): default cumulative character cap for the
/// compaction whitelist (16K — user decision; ≈8K tokens ≈ ~9% of the
/// 90K compacted target, small enough not to squeeze the kept rounds).
pub const DEFAULT_WHITELIST_CAP: usize = 16 * 1024;

/// Result of one explicit context compaction (A6).
pub(crate) struct CompactionStats {
    pub(crate) rounds_dropped: u32,
    pub(crate) messages_dropped: usize,
    pub(crate) messages_kept: usize,
    pub(crate) estimated_tokens_after: u64,
    /// Index at which the caller must insert the compaction marker — the
    /// cut point: after the preamble, before the first kept round.
    pub(crate) marker_index: usize,
}

/// A6: estimated tokens of one message — chars/2 (a conservative CJK-aware
/// guess: CJK ≈ 2 chars/token, English would be ≈ 4 — over-estimating is
/// the safe direction; the real next-round usage measurement is what the
/// trigger uses).
pub(crate) fn estimate_message_tokens(m: &Message) -> u64 {
    let mut chars = m.content.chars().count() as u64;
    if let Some(r) = &m.reasoning_content {
        chars += r.chars().count() as u64;
    }
    for tc in &m.tool_calls {
        chars += tc.name.chars().count() as u64;
        chars += serde_json::to_string(&tc.arguments)
            .map(|s| s.chars().count() as u64)
            .unwrap_or(0);
    }
    chars / 2
}

pub(crate) fn estimate_messages_tokens(messages: &[Message]) -> u64 {
    messages.iter().map(estimate_message_tokens).sum()
}

/// A6 (2026-08-08): explicit context compaction — drop complete OLDER tool
/// rounds so the remaining conversation (preamble + newest rounds) is
/// estimated under `target_tokens`.
///
/// Round = one assistant declaration message (with `tool_calls`) plus every
/// message up to the next declaration — the provider protocol requires each
/// surviving tool reply's `tool_call_id` to match a declaration in history
/// (a round split across the cut would 400 on the next request, 2026-08-06
/// design review D2-1), so compaction never splits a round. The preamble
/// (original user prompt, gate blocks) and at least the NEWEST round are
/// always kept verbatim (精确段保留 — design §5 A6: 最近 K 轮消息原文).
///
/// The caller inserts the marker (`context_compressed_marker`) at
/// `marker_index` and journals the `context_compressed` event.
pub(crate) fn compact_messages(messages: &mut Vec<Message>, target_tokens: u64) -> CompactionStats {
    let round_starts: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .map(|(i, _)| i)
        .collect();
    let noop = || CompactionStats {
        rounds_dropped: 0,
        messages_dropped: 0,
        messages_kept: messages.len(),
        estimated_tokens_after: estimate_messages_tokens(messages),
        marker_index: 0,
    };
    if round_starts.is_empty() {
        return noop();
    }
    let preamble_end = round_starts[0];
    let preamble_tokens = estimate_messages_tokens(&messages[..preamble_end]);
    let mut round_estimates: Vec<u64> = Vec::with_capacity(round_starts.len());
    for (k, &start) in round_starts.iter().enumerate() {
        let end = round_starts.get(k + 1).copied().unwrap_or(messages.len());
        round_estimates.push(estimate_messages_tokens(&messages[start..end]));
    }
    // Walk from the NEWEST round backward, keeping while the total fits the
    // target; the newest round is always kept even when it alone exceeds it
    // (recent context stays exact — the target is an estimate anyway).
    let mut kept_total = preamble_tokens;
    let mut kept_count = 0usize;
    for estimate in round_estimates.iter().rev() {
        if kept_count > 0 && kept_total + estimate > target_tokens {
            break;
        }
        kept_count += 1;
        kept_total += estimate;
    }
    let rounds_dropped = round_starts.len() - kept_count;
    if rounds_dropped == 0 {
        return noop();
    }
    let cut = round_starts[round_starts.len() - kept_count];
    let messages_dropped = messages.drain(preamble_end..cut).count();
    CompactionStats {
        rounds_dropped: rounds_dropped as u32,
        messages_dropped,
        messages_kept: messages.len(),
        estimated_tokens_after: kept_total,
        marker_index: preamble_end,
    }
}
