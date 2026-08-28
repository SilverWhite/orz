//! OrientationSessionState — session-level 7-round orientation producer
//! (ADR-0010 §4.1/§4.2; GAP-INQUIRY-SPLIT, 2026-08-09).
//!
//! The old `OrientationMonitor` emitted one checkpoint event per turn and
//! never injected the block into the conversation (Python-parity interval 1).
//! ADR-0010 §4.2 replaces it with a session-level counter over COMPLETED
//! LOGICAL MODEL ROUNDS: one assistant generation plus its required
//! tool-result replay counts 1 (tool rounds, deny rounds and gate-answer
//! rounds all count 1; multiple tool calls in one response do not split;
//! transport retries never count). The count is session-persistent — only an
//! actual fire resets it; compaction, handoff preparation and session
//! recovery never clear it; the three agents count independently.
//!
//! This slice feeds only the main agent's lane (`AgentRole::Main`); the
//! internal/external lanes are reserved for the subagent-isomorphism slice
//! (GAP-SUBAGENT-RUNTIME, next window) and stay at zero here.
//!
//! Count semantics note (review P3-1, 2026-08-10): a counterexample-gate
//! candidate round (a no-tool round INTERCEPTED by the gate — its text is
//! never committed to the conversation) counts as one completed generation,
//! so a full gate sequence counts 2 (candidate + post-gate answer). This
//! matches the §4.2 unit ("one assistant generation" — the candidate IS a
//! completed generation) and is pinned by the fixture tests.

use serde::{Deserialize, Serialize};

use orz_assurance::orientation::checkpoint::ORIENTATION_BLOCK;

/// THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.6, 用户裁决)：中立问询触发
/// 阈值默认 50（原硬编码 7）——当前注意力窗口衰减低，只需大任务轮方向
/// 检查（第一轮实测模型轮分布：中位 26 / 平均 43 / p90 94；阈值 50 时约
/// 78% 任务零触发、大任务约 1 次）。env `ORZ_ORIENTATION_THRESHOLD` 可
/// 覆盖（A/B 与回退）。
pub const ORIENTATION_THRESHOLD: u32 = 50;

/// Environment override for the orientation fire threshold.
pub const ORIENTATION_THRESHOLD_ENV: &str = "ORZ_ORIENTATION_THRESHOLD";

/// Resolve the orientation threshold from `ORZ_ORIENTATION_THRESHOLD`
/// (completed logical model rounds; invalid/missing → `ORIENTATION_THRESHOLD`).
/// Clamped to a sane floor so an env typo cannot fire every round.
pub fn orientation_threshold_from_env() -> u32 {
    std::env::var(ORIENTATION_THRESHOLD_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .map(|v| v.max(1))
        .unwrap_or(ORIENTATION_THRESHOLD)
}

/// Which agent's round counter — the v0.2 payload `agent_role` enum
/// (`main` | `internal_retrieval` | `external_retrieval`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Main,
    InternalRetrieval,
    ExternalRetrieval,
}

impl AgentRole {
    /// v0.2 payload `agent_role` value (schema enum — never free text).
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Main => "main",
            AgentRole::InternalRetrieval => "internal_retrieval",
            AgentRole::ExternalRetrieval => "external_retrieval",
        }
    }
}

/// One agent lane's orientation state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentOrientationState {
    /// Completed logical model rounds since the last fired orientation
    /// (§4.2 count semantics — see module docs).
    pub completed_rounds: u32,
    /// Lifetime completed rounds — informational, never reset.
    pub total_rounds: u32,
}

/// Lightweight last-fire marker — informational audit trace on the persisted
/// state (the full event lives in the journal; the block text is not
/// duplicated into the sidecar).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastFire {
    pub role: AgentRole,
    pub sequence: u32,
    pub injection_position: String,
}

/// A fired orientation checkpoint — everything the controller needs to
/// journal the v0.2 `orientation_checkpoint` event and inject the block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientationFireRecord {
    pub checkpoint_id: String,
    pub inquiry_family: &'static str,
    pub inquiry_kind: &'static str,
    pub agent_role: AgentRole,
    pub session_id: String,
    pub trigger: &'static str,
    pub completed_turns_since_orientation: u32,
    pub step_index: u64,
    pub message_block: String,
    pub injection_position: String,
}

/// Session-level orientation state — persisted across turns (rides
/// `StoredSession` and the `{cwd}/.gsa/orientation/<session>.json` sidecar;
/// ADR-0010 §4.2: recovery resumes counting, only an actual fire resets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrientationSessionState {
    pub main: AgentOrientationState,
    pub internal: AgentOrientationState,
    pub external: AgentOrientationState,
    /// Fire threshold — §4.2 default 7; kept on the struct so the schema
    /// version of the rule is visible in persisted state.
    pub threshold: u32,
    pub session_id: String,
    /// Monotonic fire sequence across the session — the checkpoint_id suffix.
    pub sequence: u32,
    pub last_fire: Option<LastFire>,
}

impl OrientationSessionState {
    /// Session state with the env-resolved threshold (production path —
    /// one-shot CLI / ACP sessions both read `ORZ_ORIENTATION_THRESHOLD`).
    pub fn new(session_id: impl Into<String>) -> Self {
        Self::new_with_threshold(session_id, orientation_threshold_from_env())
    }

    /// Explicit-threshold constructor — used by tests and by callers that
    /// must pin the rule version independently of the process env.
    pub fn new_with_threshold(session_id: impl Into<String>, threshold: u32) -> Self {
        Self {
            main: AgentOrientationState::default(),
            internal: AgentOrientationState::default(),
            external: AgentOrientationState::default(),
            threshold,
            session_id: session_id.into(),
            sequence: 0,
            last_fire: None,
        }
    }

    /// Count one completed logical model round for `role`.
    pub fn feed_round(&mut self, role: AgentRole) {
        let st = self.state_mut(role);
        st.completed_rounds += 1;
        st.total_rounds += 1;
    }

    /// Whether `role`'s lane is due for an orientation (>= threshold).
    pub fn should_fire(&self, role: AgentRole) -> bool {
        self.state(role).completed_rounds >= self.threshold
    }

    /// Build the fire record WITHOUT mutating state — the caller journals
    /// the event and injects the block first, then calls `commit_fire`
    /// (review P2-2, 2026-08-10: a journal-write failure must not leave a
    /// reset-but-never-fired counter persisted). `None` = not due yet.
    pub fn build_fire_record(
        &self,
        role: AgentRole,
        run_id: &str,
        injection_position: &str,
    ) -> Option<OrientationFireRecord> {
        if !self.should_fire(role) {
            return None;
        }
        let completed = self.state(role).completed_rounds;
        Some(OrientationFireRecord {
            // `ORIENT-{run_id}-{fire:04}` — matches the v0.2 schema pattern
            // `^ORIENT-[A-Za-z0-9._-]+-[0-9]{4}$` (run ids may contain `-`;
            // the greedy char class backtracks to the numeric suffix).
            // NOTE: the checkpoint_id sequence is 0-based (pre-increment);
            // `last_fire.sequence` below is 1-based (post-increment) — the
            // two "sequence" numbers are deliberately different bases.
            checkpoint_id: format!("ORIENT-{run_id}-{:04}", self.sequence),
            inquiry_family: "neutral",
            inquiry_kind: "orientation_checkpoint",
            agent_role: role,
            session_id: self.session_id.clone(),
            trigger: "completed_turns_interval",
            completed_turns_since_orientation: completed,
            // §5.2 "轮次状态" is carried by completed_turns_since_orientation;
            // step_index stays a producer-local placeholder (0) until a
            // blackboard-derived step exists (audit D3-2).
            step_index: 0,
            message_block: ORIENTATION_BLOCK.to_string(),
            injection_position: injection_position.to_string(),
        })
    }

    /// Commit a fire AFTER the event was journaled and the block injected:
    pub fn commit_fire(&mut self, role: AgentRole, record: &OrientationFireRecord) {
        self.state_mut(role).completed_rounds = 0;
        self.sequence += 1;
        self.last_fire = Some(LastFire {
            role,
            // 1-based (post-increment) — see build_fire_record note.
            sequence: self.sequence,
            injection_position: record.injection_position.clone(),
        });
    }

    fn state(&self, role: AgentRole) -> &AgentOrientationState {
        match role {
            AgentRole::Main => &self.main,
            AgentRole::InternalRetrieval => &self.internal,
            AgentRole::ExternalRetrieval => &self.external,
        }
    }

    fn state_mut(&mut self, role: AgentRole) -> &mut AgentOrientationState {
        match role {
            AgentRole::Main => &mut self.main,
            AgentRole::InternalRetrieval => &mut self.internal,
            AgentRole::ExternalRetrieval => &mut self.external,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_at_threshold_and_resets() {
        // R1: 阈值默认 50；测试用显式 7 保持原触发语义。
        let mut s = OrientationSessionState::new_with_threshold("SESS-1", 7);
        for _ in 0..6 {
            s.feed_round(AgentRole::Main);
        }
        assert!(!s.should_fire(AgentRole::Main));
        s.feed_round(AgentRole::Main);
        assert!(s.should_fire(AgentRole::Main));

        let rec = s
            .build_fire_record(AgentRole::Main, "RUN-1", "post_tool_batch_gap")
            .expect("due");
        assert_eq!(rec.completed_turns_since_orientation, 7);
        assert_eq!(rec.checkpoint_id, "ORIENT-RUN-1-0000");
        assert_eq!(rec.inquiry_family, "neutral");
        assert_eq!(rec.inquiry_kind, "orientation_checkpoint");
        assert_eq!(rec.agent_role, AgentRole::Main);
        assert_eq!(rec.trigger, "completed_turns_interval");
        assert_eq!(rec.message_block, ORIENTATION_BLOCK);
        assert!(rec.message_block.starts_with("[ORIENTATION v0.4]"));

        // Not committed yet — the state is untouched (review P2-2: the
        // journal write happens between build and commit; a failure leaves
        // the counter intact).
        assert!(s.should_fire(AgentRole::Main));

        s.commit_fire(AgentRole::Main, &rec);
        // Fired → reset: no immediate re-fire.
        assert!(!s.should_fire(AgentRole::Main));
        // But total_rounds keeps accumulating (informational).
        assert_eq!(s.main.total_rounds, 7);
        assert_eq!(s.main.completed_rounds, 0);
        assert_eq!(s.sequence, 1);
        assert_eq!(
            s.last_fire,
            Some(LastFire {
                role: AgentRole::Main,
                sequence: 1,
                injection_position: "post_tool_batch_gap".to_string(),
            })
        );
    }

    #[test]
    fn build_without_commit_leaves_state_untouched() {
        let mut s = OrientationSessionState::new_with_threshold("SESS-1", 7);
        for _ in 0..7 {
            s.feed_round(AgentRole::Main);
        }
        // Build only — nothing mutated.
        let rec = s.build_fire_record(AgentRole::Main, "RUN-1", "x").unwrap();
        assert_eq!(s.main.completed_rounds, 7);
        assert_eq!(s.sequence, 0);
        assert!(s.last_fire.is_none());
        // A second build is still due (no commit happened).
        assert!(s.build_fire_record(AgentRole::Main, "RUN-1", "x").is_some());
        let _ = rec;
    }

    #[test]
    fn lanes_are_independent() {
        let mut s = OrientationSessionState::new_with_threshold("SESS-1", 7);
        s.feed_round(AgentRole::InternalRetrieval);
        // Internal/external lanes are reserved (zero feeds this slice) — a
        // non-main feed must never trip the main lane.
        assert!(!s.should_fire(AgentRole::Main));
        assert_eq!(s.internal.total_rounds, 1);
        assert_eq!(s.main.total_rounds, 0);
    }

    #[test]
    fn checkpoint_id_sequence_advances_and_matches_pattern() {
        let mut s = OrientationSessionState::new_with_threshold("SESS-1", 7);
        for _ in 0..7 {
            s.feed_round(AgentRole::Main);
        }
        let r1 = s
            .build_fire_record(AgentRole::Main, "RUN-abc-def", "loop_top_gap")
            .unwrap();
        s.commit_fire(AgentRole::Main, &r1);
        // Run ids may contain `-` — the schema pattern still matches
        // (`[A-Za-z0-9._-]+` backtracks to the numeric suffix).
        assert_eq!(r1.checkpoint_id, "ORIENT-RUN-abc-def-0000");
        assert_eq!(r1.injection_position, "loop_top_gap");

        for _ in 0..7 {
            s.feed_round(AgentRole::Main);
        }
        let r2 = s
            .build_fire_record(AgentRole::Main, "RUN-abc-def", "post_tool_batch_gap")
            .unwrap();
        s.commit_fire(AgentRole::Main, &r2);
        assert_eq!(r2.checkpoint_id, "ORIENT-RUN-abc-def-0001");
        assert_eq!(s.sequence, 2);
    }

    #[test]
    fn serde_round_trip_preserves_state() {
        let mut s = OrientationSessionState::new_with_threshold("SESS-1", 7);
        for _ in 0..3 {
            s.feed_round(AgentRole::Main);
        }
        let json = serde_json::to_string(&s).unwrap();
        let restored: OrientationSessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, s);
        assert_eq!(restored.main.completed_rounds, 3);
        // Recovery resumes counting on the restored value.
        restored_clone_continue(&restored);
    }

    #[test]
    fn threshold_not_persisted_as_magic() {
        // The struct carries the threshold explicitly so persisted state
        // records which rule version produced it. R1: default = 50.
        let s = OrientationSessionState::new_with_threshold("SESS-1", ORIENTATION_THRESHOLD);
        assert_eq!(s.threshold, 50);
    }

    /// R1 (§4.6): `ORZ_ORIENTATION_THRESHOLD` overrides the default 50;
    /// invalid values fall back to the default.
    #[test]
    fn env_threshold_override() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var(ORIENTATION_THRESHOLD_ENV, "12");
        }
        assert_eq!(orientation_threshold_from_env(), 12);
        let s = OrientationSessionState::new("SESS-ENV");
        assert_eq!(s.threshold, 12);
        unsafe {
            std::env::set_var(ORIENTATION_THRESHOLD_ENV, "not-a-number");
        }
        assert_eq!(orientation_threshold_from_env(), ORIENTATION_THRESHOLD);
        unsafe {
            std::env::remove_var(ORIENTATION_THRESHOLD_ENV);
        }
        assert_eq!(orientation_threshold_from_env(), ORIENTATION_THRESHOLD);
    }

    /// Recovery resumes counting from the persisted value (§4.2).
    fn restored_clone_continue(s: &OrientationSessionState) {
        let mut s2 = s.clone();
        for _ in 0..4 {
            s2.feed_round(AgentRole::Main);
        }
        assert!(s2.should_fire(AgentRole::Main));
        assert_eq!(s2.main.completed_rounds, 7);
    }
}
