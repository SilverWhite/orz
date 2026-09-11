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

use orz_assurance::orientation::checkpoint::{INITIAL_ROUND_INQUIRY_BLOCK, ORIENTATION_BLOCK};

/// THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.6, 用户裁决)：中立问询触发
/// 阈值默认 50（原硬编码 7）——当前注意力窗口衰减低，只需大任务轮方向
/// 检查（第一轮实测模型轮分布：中位 26 / 平均 43 / p90 94；阈值 50 时约
/// 78% 任务零触发、大任务约 1 次）。env `ORZ_ORIENTATION_THRESHOLD` 可
/// 覆盖（A/B 与回退）。
pub const ORIENTATION_THRESHOLD: u32 = 50;

/// Environment override for the orientation fire threshold.
pub const ORIENTATION_THRESHOLD_ENV: &str = "ORZ_ORIENTATION_THRESHOLD";

/// P0-0x S1 (ADR-0010 §14.66，2026-09-11 设计定稿)：初始轮中立问询的
/// `trigger` 取值。与周期问询的 `completed_turns_interval` 并列——两者共用
/// `OrientationV1` 票据与 `orientation_checkpoint` 事件面，靠 `trigger`
/// 区分（submit 侧按 trigger 分派 commit 语义）。
pub const TRIGGER_INITIAL_ROUND: &str = "initial_round";

/// 周期问询（阈值 50）的 `trigger` 取值——既有语义，仅登记为常量。
pub const TRIGGER_COMPLETED_TURNS_INTERVAL: &str = "completed_turns_interval";

/// 主车道初始轮问询的注入位置（设计 §3.2：首轮动作批次结束）——同时是
/// 周期问询的两个注入点之一（另一个是 `loop_top_gap`，只承担周期问询）。
pub const ORIENTATION_POST_TOOL_BATCH_GAP: &str = "post_tool_batch_gap";

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

impl OrientationFireRecord {
    /// P0-0x S1: 本条 fire 是否为一次性初始轮问询。commit 语义不同
    /// （初始轮只置一次性标志、**不重置**周期计数；周期问询重置计数）。
    pub fn is_initial_round(&self) -> bool {
        self.trigger == TRIGGER_INITIAL_ROUND
    }
}

/// Session-level orientation state — persisted across turns (rides
/// `StoredSession` and the `{cwd}/.gsa/orientation/<session>.json` sidecar;
/// ADR-0010 §4.2: recovery resumes counting, only an actual fire resets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrientationSessionState {
    pub main: AgentOrientationState,
    pub internal: AgentOrientationState,
    pub external: AgentOrientationState,
    /// P0-0x S1（ADR-0010 §14.66）：初始轮中立问询的一次性标志——
    /// 会话内恰好一次（侧车持久化）；`#[serde(default)]` 保证旧侧车
    /// （无此字段）反序列化为「未触发」，不会因升级而丢会话。
    #[serde(default)]
    pub initial_round_fired: bool,
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
            initial_round_fired: false,
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
            trigger: TRIGGER_COMPLETED_TURNS_INTERVAL,
            completed_turns_since_orientation: completed,
            // §5.2 "轮次状态" is carried by completed_turns_since_orientation;
            // step_index stays a producer-local placeholder (0) until a
            // blackboard-derived step exists (audit D3-2).
            step_index: 0,
            message_block: ORIENTATION_BLOCK.to_string(),
            injection_position: injection_position.to_string(),
        })
    }

    /// P0-0x S1（ADR-0010 §14.66）：构建**初始轮中立问询**的 fire record
    /// —— 与 `build_fire_record` 同纪律（构建不改状态；事件先 journal、
    /// 块先注入，消费点才 commit）。`None` = 已触发过（会话内恰好一次）。
    ///
    /// 只由主车道在 `post_tool_batch_gap`（首个含工具调用的动作批次结束）
    /// 调用；调用方负责 role/时机判定（见 `maybe_fire_orientation`）。
    /// `completed_turns_since_orientation` 取当刻已完成模型轮数（信息性：
    /// 初始轮问询**不重置**该计数，只作为事件面轮次状态）。
    pub fn build_initial_round_fire_record(
        &self,
        role: AgentRole,
        run_id: &str,
        injection_position: &str,
    ) -> Option<OrientationFireRecord> {
        if self.initial_round_fired {
            return None;
        }
        Some(OrientationFireRecord {
            checkpoint_id: format!("ORIENT-{run_id}-{:04}", self.sequence),
            inquiry_family: "neutral",
            inquiry_kind: "orientation_checkpoint",
            agent_role: role,
            session_id: self.session_id.clone(),
            trigger: TRIGGER_INITIAL_ROUND,
            completed_turns_since_orientation: self.state(role).completed_rounds,
            // step_index 同周期问询：生产者本地占位（0）。
            step_index: 0,
            message_block: INITIAL_ROUND_INQUIRY_BLOCK.to_string(),
            injection_position: injection_position.to_string(),
        })
    }

    /// P0-0x S1：初始轮问询在**消费点**提交——只置一次性标志并推进
    /// fire 序号（checkpoint_id 单调唯一），**绝不触碰周期计数**
    /// （§3.2：与阈值 50 的周期问询互不影响、互不重置）。
    pub fn commit_initial_round_fire(&mut self, role: AgentRole, record: &OrientationFireRecord) {
        self.initial_round_fired = true;
        self.sequence += 1;
        self.last_fire = Some(LastFire {
            role,
            sequence: self.sequence,
            injection_position: record.injection_position.clone(),
        });
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
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::Role;
    use crate::host::ToolResult;
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;

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

    // ── P0-0x S1 初始轮中立问询（ADR-0010 §14.66） ──────────────────────

    /// 状态层：初始轮 record 一次性、不触碰周期计数；commit 只置标志 +
    /// 推进 fire 序号（checkpoint_id 单调唯一），并随会话持久化。
    #[test]
    fn initial_round_record_is_one_shot_and_keeps_periodic_counter() {
        let mut s = OrientationSessionState::new_with_threshold("SESS-0X", 7);
        s.feed_round(AgentRole::Main);
        let rec = s
            .build_initial_round_fire_record(AgentRole::Main, "RUN-0X", "post_tool_batch_gap")
            .expect("first fire");
        assert_eq!(rec.trigger, TRIGGER_INITIAL_ROUND);
        assert!(rec.is_initial_round());
        assert_eq!(rec.completed_turns_since_orientation, 1);
        assert_eq!(rec.message_block, INITIAL_ROUND_INQUIRY_BLOCK);
        assert!(rec.message_block.starts_with("[INITIAL_ROUND_INQUIRY"));
        assert_eq!(rec.checkpoint_id, "ORIENT-RUN-0X-0000");

        // 构建不改状态：仍可再构建（事件先行、commit 在后）。
        assert!(
            s.build_initial_round_fire_record(AgentRole::Main, "RUN-0X", "post_tool_batch_gap")
                .is_some()
        );
        assert!(!s.initial_round_fired);

        s.commit_initial_round_fire(AgentRole::Main, &rec);
        assert!(s.initial_round_fired);
        assert_eq!(s.sequence, 1);
        // 周期计数不被重置（与阈值问询互不影响）。
        assert_eq!(s.main.completed_rounds, 1);
        assert_eq!(s.main.total_rounds, 1);
        // 二次构建 → None（会话内恰好一次）。
        assert!(
            s.build_initial_round_fire_record(AgentRole::Main, "RUN-0X", "post_tool_batch_gap")
                .is_none()
        );
        // 序号推进后周期问询的 checkpoint_id 不与之撞号。
        for _ in 0..6 {
            s.feed_round(AgentRole::Main);
        }
        let periodic = s
            .build_fire_record(AgentRole::Main, "RUN-0X", "post_tool_batch_gap")
            .expect("threshold due");
        assert_eq!(periodic.checkpoint_id, "ORIENT-RUN-0X-0001");
        assert!(!periodic.is_initial_round());

        // 会话持久化：标志随状态往返。
        let json = serde_json::to_string(&s).unwrap();
        let restored: OrientationSessionState = serde_json::from_str(&json).unwrap();
        assert!(restored.initial_round_fired);
    }

    /// 状态层：旧侧车（无 `initial_round_fired` 字段）反序列化为未触发，
    /// 升级不丢会话、不误判为「已问过」。
    #[test]
    fn initial_round_flag_defaults_false_for_legacy_sidecar() {
        let legacy = serde_json::json!({
            "main": { "completed_rounds": 3, "total_rounds": 9 },
            "internal": { "completed_rounds": 0, "total_rounds": 0 },
            "external": { "completed_rounds": 0, "total_rounds": 0 },
            "threshold": 50,
            "session_id": "SESS-LEGACY",
            "sequence": 1,
            "last_fire": null
        });
        let restored: OrientationSessionState = serde_json::from_value(legacy).unwrap();
        assert!(!restored.initial_round_fired);
        assert_eq!(restored.main.completed_rounds, 3);
    }

    /// 集成：首个**含工具调用**的动作批次结束时恰好注入一次初始轮问询；
    /// 首轮无工具调用（纯文本回复）时顺延到之后的动作批次（无工具批次
    /// 不会触发）。
    #[tokio::test]
    async fn initial_round_fires_once_at_first_tool_batch_and_defers_without_one() {
        // 两个 run 各自一份 journal（run_finished 后同一 journal 拒绝再起
        // run），但共享同一个会话级 orientation 状态——正是「顺延」要
        // 验证的：状态跨 run 延续，问询只在此后首个动作批次触发一次。
        let dir1 = test_dir();
        let host1 = TestHost {
            journal: JournalRecorder::new(dir1.clone()),
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let dir2 = test_dir();
        let host2 = TestHost {
            journal: JournalRecorder::new(dir2.clone()),
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
            // run 1：首轮纯文本（无工具批次）→ 反例门 → 终答。顺延。
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
            // run 2：首个动作批次 → 初始轮问询 → 回答被软消费 → 反例门 → 终答。
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-0")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ])));
        let mut orientation = OrientationSessionState::new_with_threshold("sess-initial-defer", 50);

        controller
            .run_turn(
                &host1,
                "任务",
                "RUN-0X-A",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();
        let after_run1 = events(&dir1);
        assert_eq!(
            after_run1
                .iter()
                .filter(|e| e.event_type == EventType::OrientationCheckpoint)
                .count(),
            0,
            "无工具批次的首轮不得触发初始轮问询: {:?}",
            event_types(&dir1)
        );
        assert_eq!(
            after_run1.last().unwrap().event_type,
            EventType::RunFinished
        );

        controller
            .run_turn(
                &host2,
                "任务",
                "RUN-0X-B",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();
        let fires: Vec<_> = events(&dir2)
            .into_iter()
            .filter(|e| e.event_type == EventType::OrientationCheckpoint)
            .collect();
        assert_eq!(fires.len(), 1, "{:?}", event_types(&dir2));
        assert_eq!(fires[0].payload["trigger"], TRIGGER_INITIAL_ROUND);
        assert_eq!(fires[0].payload["agent_role"], "main");
        assert_eq!(
            fires[0].payload["injection_position"],
            "post_tool_batch_gap"
        );
        assert!(orientation.initial_round_fired);
        // 阈值 50 未到——周期问询不因初始轮而触发。
        assert_eq!(orientation.sequence, 1);
        let _ = std::fs::remove_dir_all(&dir1);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    /// 集成（设计 §5-7「中断后恢复重触发」）：fire 已 journal、消费前即中断
    /// （生成失败）→ 一次性标志不提交 → **下次 run 在首个含工具调用的批次
    /// 结束再次触发一次**（§3.4 登记：初始轮注入位置固定为
    /// `post_tool_batch_gap`，`loop_top_gap` 只承担周期问询，故恢复形态是
    /// 「下次 run 首个动作批次」而非 loop-top）。
    ///
    /// 中断用「run1 的网关脚本只有动作批次一轮」复现：初始轮问询注入后的
    /// pending 轮一生成即耗尽（等价于注入后未消费即中断），run1 以错误终止；
    /// 两个 run 共用同一份会话级 orientation 状态（run2 用独立网关与 journal）。
    #[tokio::test]
    async fn initial_round_refires_next_run_when_the_fire_was_not_consumed() {
        let dir1 = test_dir();
        let host1 = TestHost {
            journal: JournalRecorder::new(dir1.clone()),
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let dir2 = test_dir();
        let host2 = TestHost {
            journal: JournalRecorder::new(dir2.clone()),
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let mut orientation =
            OrientationSessionState::new_with_threshold("sess-initial-recover", 50);

        // run1：首个动作批次 → 初始轮 fire（pending 未消费）→ 生成失败中断。
        let controller1 = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-0")]),
        ])));
        let run1 = controller1
            .run_turn(
                &host1,
                "任务",
                "RUN-0X-R1",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await;
        assert!(
            run1.is_err(),
            "run1 必须在 pending 轮生成失败处中断（脚本耗尽）"
        );
        let fires1: Vec<_> = events(&dir1)
            .into_iter()
            .filter(|e| e.event_type == EventType::OrientationCheckpoint)
            .collect();
        assert_eq!(fires1.len(), 1, "{:?}", event_types(&dir1));
        assert_eq!(fires1[0].payload["trigger"], TRIGGER_INITIAL_ROUND);
        // 关键：fire 已落 journal，但一次性标志**未提交**（消费前中断）。
        assert!(
            !orientation.initial_round_fired,
            "未消费的 fire 不得提交一次性标志（否则恢复即丢问询）"
        );
        assert_eq!(orientation.sequence, 0);

        // run2：首个动作批次结束再次触发（恢复重触发），回答被软消费后照常收尾。
        let controller2 = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-1")]),
            // 初始轮问询的回答（软门消费后续跑）。
            ScriptedResponse::text("完成"),
            // 反例门轮 + 终答轮。
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ])));
        controller2
            .run_turn(
                &host2,
                "任务",
                "RUN-0X-R2",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();
        let fires2: Vec<_> = events(&dir2)
            .into_iter()
            .filter(|e| e.event_type == EventType::OrientationCheckpoint)
            .collect();
        assert_eq!(fires2.len(), 1, "{:?}", event_types(&dir2));
        assert_eq!(fires2[0].payload["trigger"], TRIGGER_INITIAL_ROUND);
        assert_eq!(
            fires2[0].payload["injection_position"],
            "post_tool_batch_gap"
        );
        assert!(orientation.initial_round_fired);
        let _ = std::fs::remove_dir_all(&dir1);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    /// 集成：初始轮问询与阈值周期问询互不影响（初始轮 commit 不重置计数，
    /// 周期问询照常在阈值处触发），且触发轮保持常规工具面（软门不禁工具）。
    #[tokio::test]
    async fn initial_round_does_not_interfere_with_periodic_threshold() {
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
        };
        let mut script = Vec::new();
        for i in 0..4 {
            script.push(ScriptedResponse::tool_calls(vec![tool_call(
                "read_file",
                &format!("call-{i}"),
            )]));
        }
        script.push(ScriptedResponse::text("完成"));
        script.push(ScriptedResponse::text("完成"));
        let fake = Arc::new(FakeProvider::new(script));
        let controller = AgentLoopController::with_gateway(fake.clone());
        let mut orientation =
            OrientationSessionState::new_with_threshold("sess-initial-periodic", 3);
        controller
            .run_turn(
                &host,
                "任务",
                "RUN-0X-C",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();

        let fires: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::OrientationCheckpoint)
            .collect();
        assert_eq!(fires.len(), 2, "{:?}", event_types(&dir));
        assert_eq!(fires[0].payload["trigger"], TRIGGER_INITIAL_ROUND);
        assert_eq!(
            fires[0].payload["completed_turns_since_orientation"],
            serde_json::json!(1)
        );
        assert_eq!(
            fires[1].payload["trigger"],
            TRIGGER_COMPLETED_TURNS_INTERVAL
        );
        assert_eq!(
            fires[1].payload["completed_turns_since_orientation"],
            serde_json::json!(3),
            "初始轮问询不得吞掉周期计数"
        );
        // 初始轮触发轮（第 2 次请求）保持常规工具面——软门不禁工具。
        let requests = fake.received_requests();
        let initial_idx = requests
            .iter()
            .position(|req| {
                req.messages.iter().any(|m| {
                    m.role == Role::User && m.content.starts_with("[INITIAL_ROUND_INQUIRY")
                })
            })
            .expect("initial-round trigger request");
        assert_eq!(initial_idx, 1, "初始轮问询须在首个动作批次后注入");
        let trigger = &requests[initial_idx];
        let prior = &requests[initial_idx - 1];
        assert!(
            !trigger.tools.is_empty()
                && trigger.tools.len() == prior.tools.len()
                && trigger.tools.iter().all(|t| {
                    prior.tools.iter().any(|p| {
                        p.name == t.name
                            && p.description == t.description
                            && p.parameters == t.parameters
                    })
                }),
            "初始轮触发轮工具面必须与前一普通轮一致: prior={:?} trigger={:?}",
            prior.tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
            trigger.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        // 周期问询在阈值处提交后归零，只剩反例门轮 + 终答轮两轮。
        assert_eq!(orientation.main.completed_rounds, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 集成：终答前的机械审查报告与反例质询行为完全不变——两块照旧同轮
    /// 注入，且都不携带初始轮三问（审查依旧是结尾的事）。
    #[tokio::test]
    async fn final_answer_audit_blocks_unchanged_and_carry_no_inquiry() {
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
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-0")]),
            // 初始轮问询的回答（软门消费）。
            ScriptedResponse::text("完成"),
            // 终答候选 → 反例门 + 审查报告。
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(fake.clone());
        let mut orientation = OrientationSessionState::new_with_threshold("sess-0x-audit", 50);
        let mut conversation: Vec<crate::gateway::model::Message> = Vec::new();
        controller
            .run_turn(
                &host,
                "任务",
                "RUN-0X-D",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                Some(&mut conversation),
            )
            .await
            .unwrap();

        let gate_block = crate::prompt::COUNTEREXAMPLE_GATE_BLOCK;
        let events = events(&dir);
        let gate = events
            .iter()
            .find(|e| e.event_type == EventType::CounterexampleGate)
            .expect("counterexample gate fires once");
        assert_eq!(gate.payload["position"], "final_answer");
        assert_eq!(gate.payload["once_only"], serde_json::json!(true));
        assert_eq!(gate.payload["message_block"], gate_block);

        // 机械注入文本绝不持久化回会话（初始轮问询同待遇）。
        assert!(
            conversation
                .iter()
                .all(|m| !m.content.starts_with("[INITIAL_ROUND_INQUIRY")),
            "initial-round inquiry block must not persist into the conversation"
        );
        assert!(
            conversation
                .iter()
                .all(|m| m.content != gate_block && !m.content.starts_with("[MECHANICAL_AUDIT")),
            "audit report / gate block must not persist into the conversation"
        );

        // 终答前那一轮的请求：审查报告 + 反例门各自独立成块，且都不含三问。
        let requests = fake.received_requests();
        let final_idx = requests
            .iter()
            .rposition(|req| {
                req.messages
                    .iter()
                    .any(|m| m.role == Role::User && m.content == gate_block)
            })
            .expect("final-answer round carries the gate block");
        let final_req = &requests[final_idx];
        let audit_msg = final_req
            .messages
            .iter()
            .find(|m| {
                m.role == Role::User
                    && m.content
                        .starts_with(crate::mechanical_audit::MECHANICAL_AUDIT_PREFIX)
            })
            .expect("mechanical audit report rides the same round");
        for probe in [
            "[INITIAL_ROUND_INQUIRY",
            "本任务实际要交付什么",
            "大方向是什么",
            "当前做法优劣如何",
        ] {
            assert!(
                !audit_msg.content.contains(probe),
                "审查报告不得携带问询: {probe}"
            );
            assert!(!gate_block.contains(probe), "反例门块不得携带问询: {probe}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16) ─────

    /// THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29 软门): 7 轮跨越阈值时
    /// 注入简短 [ORIENTATION v0.4] 块并延迟 commit；触发后的纯文本回答被
    /// 消费（loop 明确续跑，不再被当终答），不产生 checkpoint_response、
    /// 不强制无工具轮；终答只由模型自发。
    #[tokio::test]
    async fn orientation_soft_gate_consumes_text_answer_and_continues() {
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
        };
        let mut script = Vec::new();
        for i in 0..7 {
            script.push(ScriptedResponse::tool_calls(vec![tool_call(
                "read_file",
                &format!("call-{i}"),
            )]));
        }
        // 触发后的第一轮：纯文本回答被软消费（续跑），不计为终答。
        script.push(ScriptedResponse::text("完成"));
        // 反例门轮 + 最终答案轮。
        script.push(ScriptedResponse::text("完成"));
        script.push(ScriptedResponse::text("完成"));
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(script)));
        let mut orientation =
            crate::orientation::OrientationSessionState::new_with_threshold("sess-template1", 7);
        controller
            .run_turn(
                &host,
                "任务",
                "RUN-TPL1",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let fires: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::OrientationCheckpoint)
            .collect();
        // 0x S1: 两条 fire —— 第 1 轮动作批次结束的一次性初始轮问询，
        // 与第 7 轮跨越阈值的周期问询。软门语义对两条都成立。
        assert_eq!(fires.len(), 2, "{:?}", event_types(&dir));
        let periodic = fires
            .iter()
            .find(|e| e.payload["trigger"].as_str() == Some("completed_turns_interval"))
            .expect("periodic fire present");
        assert_eq!(
            periodic.payload["injection_position"].as_str(),
            Some("post_tool_batch_gap")
        );
        assert_eq!(
            periodic
                .payload
                .get("completed_turns_since_orientation")
                .and_then(|v| v.as_u64()),
            Some(7)
        );
        // 无 checkpoint_response——不再有强制模板轮。
        let responses: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::CheckpointResponse)
            .collect();
        assert_eq!(responses.len(), 0, "{:?}", event_types(&dir));
        // 软门延迟 commit：第 8 轮（纯文本回答）feed 后 commit 归 0；
        // 反例门轮 + 最终答案轮再喂 2。
        assert_eq!(orientation.main.completed_rounds, 2);
        assert_eq!(
            events.last().unwrap().event_type,
            EventType::RunFinished,
            "the pure-text answer was consumed and the run continues"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29 软门): 触发后下一轮
    /// 仍是普通工具轮——orientation 不禁工具，read_file 调用照常执行。
    #[tokio::test]
    async fn orientation_fire_does_not_force_tool_free_round() {
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
        };
        let mut script = Vec::new();
        for i in 0..7 {
            script.push(ScriptedResponse::tool_calls(vec![tool_call(
                "read_file",
                &format!("call-{i}"),
            )]));
        }
        // Fire 后的下一轮：普通工具轮（read_file call-7）直接执行
        // （软门消费点 commit 后落入常规派发路径）。
        script.push(ScriptedResponse::tool_calls(vec![tool_call(
            "read_file",
            "call-7",
        )]));
        script.push(ScriptedResponse::text("完成"));
        script.push(ScriptedResponse::text("完成"));
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(script)));
        let mut orientation =
            crate::orientation::OrientationSessionState::new_with_threshold("sess-template2", 7);
        controller
            .run_turn(
                &host,
                "任务",
                "RUN-TPL2",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        // 无 checkpoint_response（软门不产生模板响应事件）。
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == EventType::CheckpointResponse)
                .count(),
            0,
            "{:?}",
            event_types(&dir)
        );
        // Fire 后的 read_file call-7 正常执行（ToolStarted + ToolCompleted）。
        let fire_idx = events
            .iter()
            .position(|e| e.event_type == EventType::OrientationCheckpoint)
            .expect("orientation fire");
        let started_idx = events
            .iter()
            .position(|e| {
                e.event_type == EventType::ToolStarted
                    && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-7")
            })
            .expect("post-fire tool round must execute");
        assert!(
            started_idx > fire_idx,
            "the tool round after the fire must execute normally"
        );
        // 延迟 commit：第 8 轮（call-7 工具轮）feed 后 commit 归 0；
        // 反例门轮 + 最终答案轮再喂 2。
        assert_eq!(orientation.main.completed_rounds, 2);
        assert_eq!(events.last().unwrap().event_type, EventType::RunFinished);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29 S5-1 修复 B 回归):
    /// orientation 软门触发轮的模型可见工具栏保持常规投影——实机
    /// make-doom / gcode-to-text / video-processing 触发轮工具面为空
    /// （`pending_checkpoint.is_some() → Vec::new()`），模型只能把 XML
    /// 工具调用写成纯文本，浪费一轮真实工作；修复后触发轮请求必须携带
    /// 完整工具列表（含 read_file），模型可回答后继续、也可直接动作。
    #[tokio::test]
    async fn orientation_trigger_round_keeps_tool_face_projected() {
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
        };
        let mut script = Vec::new();
        for i in 0..7 {
            script.push(ScriptedResponse::tool_calls(vec![tool_call(
                "read_file",
                &format!("call-{i}"),
            )]));
        }
        // 触发轮：工具调用照常执行（软门不禁工具）。
        script.push(ScriptedResponse::tool_calls(vec![tool_call(
            "read_file",
            "call-7",
        )]));
        script.push(ScriptedResponse::text("完成"));
        script.push(ScriptedResponse::text("完成"));
        let fake = Arc::new(FakeProvider::new(script));
        let controller = AgentLoopController::with_gateway(fake.clone());
        let mut orientation =
            crate::orientation::OrientationSessionState::new_with_threshold("sess-tplface", 7);
        controller
            .run_turn(
                &host,
                "任务",
                "RUN-TPLF",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();

        // 触发轮 = 消息含 [ORIENTATION 注入块的请求；其工具栏必须与
        // 前一普通工具轮完全一致（常规投影），不得因 pending checkpoint
        // 置空——实机工具面为空时模型只能把 XML 工具调用写成纯文本。
        let requests = fake.received_requests();
        let trigger_idx = requests
            .iter()
            .position(|req| {
                req.messages
                    .iter()
                    .any(|m| m.role == Role::User && m.content.starts_with("[ORIENTATION"))
            })
            .unwrap_or_else(|| panic!("no trigger-round request found: {requests:?}"));
        assert!(
            trigger_idx > 0,
            "a normal tool round must precede the trigger"
        );
        let trigger = &requests[trigger_idx];
        let prior = &requests[trigger_idx - 1];
        assert!(
            !trigger.tools.is_empty(),
            "orientation trigger round must keep the tool face projected: {:?}",
            trigger.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        assert!(
            trigger.tools.len() == prior.tools.len()
                && trigger.tools.iter().all(|t| {
                    prior.tools.iter().any(|p| {
                        p.name == t.name
                            && p.description == t.description
                            && p.parameters == t.parameters
                    })
                }),
            "trigger-round tool face must equal the prior normal round's projection: \
             prior={:?} trigger={:?}",
            prior.tools.iter().map(|t| &t.name).collect::<Vec<_>>(),
            trigger.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        assert_eq!(
            events(&dir).last().unwrap().event_type,
            EventType::RunFinished
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
