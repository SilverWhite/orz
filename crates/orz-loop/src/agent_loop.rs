//! Shared model↔tool loop — the one loop all three agents run.
//!
//! GAP-SUBAGENT-RUNTIME (2026-08-10): ADR-0010 §3.1 isomorphism — the main
//! agent and both retrieval subagents run the SAME agent loop code; the
//! per-role semantics (which lane counts, whether the final-answer
//! counterexample gate applies, the budget) are carried by `LoopProfile`,
//! and the per-role surface (system prompt, tools, blackboard partition)
//! by the callers. The loop body here was extracted verbatim from
//! `AgentLoopController::run_turn_inner` (M1, 2026-08-10) — the main
//! profile must keep byte-identical event sequences with the pre-split
//! code (locked by `EXPECTED_SEQUENCES_V02` + conformance capture).
//!
//! The loop writes no run lifecycle events (`run_started` /
//! `run_finished` / …): those belong to the caller. Subagent loops run
//! inside the parent run's hash chain (M3, 2026-08-10) — the run
//! terminal uniqueness stays with the parent.

use std::path::Path;
use std::sync::{Arc, Mutex};

use orz_assurance::journal::chain::{payload_hash, sha256_hex};
use orz_assurance::{EventType, GateDecision};

use crate::agents::SubagentRole;
use crate::blackboard::BLACKBOARD_WRITE_TOOL_NAME;
use crate::blackboard::SharedBlackboard;
use crate::checkpoint::{self, PendingCheckpoint};
use crate::controller::{
    AgentLoopController, AgentLoopError, ContextCompactConfig, DENIAL_BREAKER_CONSECUTIVE,
    DenialKey, DenialState, EventWriter, PolicyFeedback, TEXT_DELTA_PACING,
    estimate_message_tokens, estimate_messages_tokens, format_edit_record,
};
use crate::gateway::model::{
    ActivityClock, FinishReason, GatewayError, Message, ModelGateway, ModelResponse, Role,
    ToolCall, TransportRetryKind,
};
use crate::host::{LoopHost, RiskClass, ToolDef, ToolResult};
use crate::orientation::{AgentRole, ORIENTATION_POST_TOOL_BATCH_GAP, OrientationSessionState};
use crate::prompt::COUNTEREXAMPLE_GATE_BLOCK;
use crate::relay::{DispatchTarget, route};
use crate::tool::ToolDispatcher;

/// The request-level max-tokens cap for ALL three agents (ADR-0010 §3.4.2:
/// the same default is injected into the main agent and both retrieval
/// subagents — no lane carries its own literal; review F3, 2026-08-10).
/// The transport's `ModelConfig::max_tokens` is the ceiling and this
/// request-level value is min-capped by it — equal today, so the full
/// budget is available (D-6: a thinking subagent with a 1024-token cap
/// would spend everything on reasoning and die before producing output).
/// OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35):
/// 输出预算恢复 32K → 256K（官方 maxTokens 默认值；回落档 128K，S4 实测
/// 不可接受才回落，编译期常量）。止损由输出健康哨兵承担——content 复读
/// 检测（P0-0d）+ reasoning 复读灵敏层（THIN-HARNESS-REDESIGN 2026-08-28
/// 用户裁决：官方 max 只等待不杀，reasoning-stall 预算兜底已物理删除，
/// 复读判定已足够），空流不原样重试（D-6 快速有界 ≤2 次 + 降级出口）。
pub const REQUEST_MAX_TOKENS: u32 = 256_000;

/// One model generation round — the uniform round entry every agent
/// implements (ADR-0010 §3.4: identical model request shape).
#[allow(clippy::too_many_arguments)] // mirrors MainAgent::run_round's contract
#[async_trait::async_trait]
pub(crate) trait RoundAgent: Send + Sync {
    /// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the gateway
    /// config digest — part of the model-request header fingerprint.
    fn config_fingerprint(&self) -> String {
        "unknown-config".to_string()
    }

    async fn run_round(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Vec<ToolDef>,
        max_tokens: u32,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError>;
}

/// Which system prompt the loop assembles — the per-role surface.
#[derive(Debug, Clone)]
pub(crate) enum SystemPromptKind {
    /// The main agent's `BASE_SYSTEM_PROMPT` + availability/budget/status
    /// blocks.
    Main,
    /// A retrieval task contract (ADR-0010 §3.2) — citation rules + the
    /// `[DOC]`/`[SOURCE]` delivery contract, NOT `BASE_SYSTEM_PROMPT`.
    Retrieval { role: SubagentRole, goal: String },
}

/// Retrieval-lane tool filtering (ADR-0010 §3.2 — deny-only write domain:
/// the subagent writes only its own blackboard partition, the task's
/// retrieval docs and the retrieval archive; file mutations and shell are
/// structurally refused before the host's permission bridge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolFilter {
    None,
    Retrieval,
}

impl ToolFilter {
    /// The write-domain gate — `Some(reason)` refuses the tool with a
    /// structured denial (IP2a "失败必显式"; the denial key feeds the
    /// shared 3-round breaker). The ACAF slice will resolve the write
    /// domains to absolute paths/object identities at this seam.
    ///
    /// Three refusal classes (review F6, 2026-08-10): file mutations
    /// (`modifies_files`), shell escape, and — distinctly — `run_tests`:
    /// §3.8.2 treats it as CONTROLLED CODE EXECUTION (the test process may
    /// write files / touch the network), so the lane refuses it with an
    /// execution-class reason, not a file-write one. A retrieval task
    /// contract never carries a test harness.
    fn write_gate(&self, tool: &str) -> Option<&'static str> {
        match self {
            ToolFilter::None => None,
            ToolFilter::Retrieval => {
                if tool == "run_tests" {
                    Some("retrieval_role_execution_denied")
                } else if tool == "blackboard_action_write" {
                    // P0-C S2 (2026-08-15): the console write button is
                    // main-lane only — subagents never write action orders.
                    Some("console_action_write_lane_denied")
                } else if tool == crate::planning::PLAN_WRITE_TOOL {
                    // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): the
                    // plan-gate write surface is main-lane only (P2-1
                    // 审查收口 — 投影剥除之外的机械兜底)。
                    Some("plan_write_lane_denied")
                } else if ToolDispatcher::modifies_files(tool) {
                    Some("retrieval_role_write_denied")
                } else if ToolDispatcher::risk_class(tool) == RiskClass::SandboxEscape {
                    Some("retrieval_role_shell_denied")
                } else {
                    None
                }
            }
        }
    }

    /// Nested retrieval dispatch is refused inside a retrieval lane —
    /// one seat per role, no recursion (ADR-0010 §11.3).
    fn denies_nested_dispatch(&self) -> bool {
        matches!(self, ToolFilter::Retrieval)
    }
}

/// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the model-request
/// header fingerprint — the provider prefix-cache key components the client
/// can observe (system + tools + config). Messages are deliberately NOT part
/// of the header: they change every round and a header event exists to
/// attribute cache misses caused by the STATIC prefix, not by new dialogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestHeader {
    pub header_sha256: String,
    pub system_sha256: String,
    pub tools_sha256: String,
    pub config_sha256: String,
    pub tools: Vec<String>,
}

/// Compute the request-header fingerprint from the assembled system prompt,
/// the projected tool list and the transport config digest.
pub(crate) fn compute_request_header(
    system: &str,
    tools: &[ToolDef],
    config_fingerprint: &str,
) -> RequestHeader {
    let system_sha256 = sha256_hex(system.as_bytes());
    // 2026-08-15 review boundary (audit §5): the JSON fallback strings are
    // theoretical only (serde_json cannot fail on these shapes) — if they
    // ever appeared, real changes would collapse onto a constant digest, so
    // they must be treated as a hard bug, not a silent degrade.
    let mut tool_rows: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            })
        })
        .collect();
    tool_rows.sort_by(|a, b| {
        a.get("name")
            .and_then(serde_json::Value::as_str)
            .cmp(&b.get("name").and_then(serde_json::Value::as_str))
    });
    let tools_sha256 = payload_hash(&tool_rows).unwrap_or_else(|_| "tools-hash-error".to_string());
    let config_sha256 = sha256_hex(config_fingerprint.as_bytes());
    let header_sha256 = payload_hash(&serde_json::json!({
        "system_sha256": system_sha256,
        "tools_sha256": tools_sha256,
        "config_sha256": config_sha256,
    }))
    .unwrap_or_else(|_| "header-hash-error".to_string());
    let tools = tools.iter().map(|t| t.name.clone()).collect();
    RequestHeader {
        header_sha256,
        system_sha256,
        tools_sha256,
        config_sha256,
        tools,
    }
}

/// ORZ-CACHE-CONTEXT-COST (2026-08-15 review fix, ADR-0010 §3.5 条6): which
/// header component changed between two requests — `system`, `tools`,
/// `config`, or `multiple` when more than one digest differs. This is the
/// mechanical「变化原因」attribution carried by `request_header_change` on
/// `reason=change`; probe-flip triggers are additionally attributable via
/// the flip↔header verifier cross-check (`_verify_v02_probe_accuracy`).
pub(crate) fn header_change_kind(prev: &RequestHeader, cur: &RequestHeader) -> &'static str {
    let mut changed = Vec::with_capacity(3);
    if prev.system_sha256 != cur.system_sha256 {
        changed.push("system");
    }
    if prev.tools_sha256 != cur.tools_sha256 {
        changed.push("tools");
    }
    if prev.config_sha256 != cur.config_sha256 {
        changed.push("config");
    }
    match changed.as_slice() {
        [single] => single,
        _ => "multiple",
    }
}

/// Build the `request_header_change` event payload. `reason` is `initial`
/// for the first request of a loop invocation, `change` when the fingerprint
/// differs from the previous request. `change_kind` is the mechanical
///「变化原因」(system/tools/config/multiple) and is present only on `change`.
pub(crate) fn request_header_payload(
    header: &RequestHeader,
    reason: &str,
    previous_header_sha256: Option<&str>,
    agent_role: &str,
    change_kind: Option<&str>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "reason": reason,
        "header_sha256": header.header_sha256,
        "system_sha256": header.system_sha256,
        "tools_sha256": header.tools_sha256,
        "config_sha256": header.config_sha256,
        "agent_role": agent_role,
        "tools": header.tools,
        "tool_count": header.tools.len(),
    });
    if let Some(prev) = previous_header_sha256 {
        payload["previous_header_sha256"] = serde_json::json!(prev);
    }
    if let Some(kind) = change_kind {
        payload["change_kind"] = serde_json::json!(kind);
    }
    payload
}

/// Role-specific loop semantics (ADR-0010 §3.2 — the ONLY difference surface
/// between the three agents' loop execution).
/// P2-4（2026-09-10）：SERP **引擎导航**预算（每个车道／每个检索 activation
/// 一个实例；主车道每个 run 一个实例）。
///
/// 单位是「引擎导航次数」而不是「search 调用次数」：一次 `auto` 调用可能
/// 依次打 Google→Bing→DDG，按调用计数会低估真实 SERP 流量。计费模型 =
/// 派发前预留 1（预算为 0 即拒绝，与候选门同族形态），调用返回后按信封里的
/// `engine_attempts` 实际导航数结算差额——多引擎调用最多只短暂超发 ≤2，
/// 由 host 侧物理兜底上限封住。
///
/// 为什么在 loop 层而不是 host：车道身份只在 loop 层可知（宿主工具接口不带
/// role），而预算必须按车道独立——否则主车道的探索会挤占检索车道的额度
/// （2026-09-10 复审 P2-4）。
#[derive(Debug)]
pub struct SerpSearchBudget {
    cap: u32,
    used: u32,
    /// P2-3（2026-09-10）：主车道／grill = `true`——本车道的会话头寸受
    /// [`SERP_SESSION_RETRIEVAL_FLOOR`] 约束（不得吃掉为检索车道保留的
    /// 那一段会话额度）；外部检索车道 = `false`（它就是被保留的一方）。
    reserves_session_floor: bool,
}

/// 主车道（含 grill）每个 run 的 SERP 导航预算。
pub const MAIN_SERP_NAVIGATION_BUDGET: u32 = 8;

/// P2-3（2026-09-10）：浏览器会话里**为检索车道保留**的引擎导航底线额度。
///
/// 会话上限是跨车道共享的物理面（每会话一个浏览器实例）；主车道每个 run
/// 都拿一张独立额度、且一个会话可以有多个 run，若不设底线，主车道的探索
/// 会把整会话额度吃光，检索车道随后只能收到宿主的
/// `browser_control_search_cap_exceeded`。本常量表达"检索至少拿得到一次
/// standard（8）或 extended（16）档激活的量"；deep（32/40）超出该底线，
/// 属尽力而为（设计 §3.2.5 已登记该边界）。
///
/// 判定口径与宿主会话上限一致，是**检查点式**：派发前读取会话头寸，头寸
/// ≤ 本常量即拒绝主车道的 search；单次多引擎调用最多再侵蚀 ≤2 次导航
/// （与会话上限的最坏超发同量级）。
pub const SERP_SESSION_RETRIEVAL_FLOOR: u32 = 16;

impl SerpSearchBudget {
    pub fn new(cap: u32) -> Self {
        Self::build(cap, false)
    }

    /// P2-3：主车道／grill 形态——会话头寸受检索底线约束。
    pub fn for_main_lane(cap: u32) -> Self {
        Self::build(cap, true)
    }

    /// P2-3/P2-2（2026-09-10）：以本激活**已消耗**的引擎导航数为初始用量
    /// 构造（`continue` 重入与 sidecar 恢复沿用同一额度，不重置）。档位
    /// 下调时把旧用量钳到新上限——已用尽的额度保持"用尽"语义。
    pub fn with_used(cap: u32, used: u32) -> Self {
        let mut budget = Self::new(cap);
        budget.used = used.min(budget.cap);
        budget
    }

    fn build(cap: u32, reserves_session_floor: bool) -> Self {
        Self {
            cap: cap.max(1),
            used: 0,
            reserves_session_floor,
        }
    }

    /// 本额度是否必须为检索车道保留会话底线（主车道／grill 为真）。
    pub fn reserves_session_floor(&self) -> bool {
        self.reserves_session_floor
    }

    /// 已消耗的引擎导航次数（激活写回用）。
    pub fn used(&self) -> u32 {
        self.used
    }

    pub fn remaining(&self) -> u32 {
        self.cap.saturating_sub(self.used)
    }

    pub fn usage(&self) -> (u32, u32) {
        (self.used, self.cap)
    }

    /// 派发前预留 1 次导航；预算为 0 时拒绝（用量经 `usage()` 读取）。
    pub fn reserve(&mut self) -> Result<(), ()> {
        if self.remaining() == 0 {
            return Err(());
        }
        self.used = self.used.saturating_add(1);
        Ok(())
    }

    /// 调用返回后按实际导航数结算（预留的 1 已计入，只补差额）。
    pub fn settle(&mut self, navigations: u32) {
        let extra = navigations.saturating_sub(1);
        self.used = self.used.saturating_add(extra).min(self.cap);
    }

    /// 调用被权限／模式门拒绝、从未执行时释放预留（候选门 rollback 同义）。
    pub fn rollback(&mut self) {
        self.used = self.used.saturating_sub(1);
    }
}

#[cfg(test)]
mod serp_budget_tests {
    use super::{
        LoopProfile, MAIN_SERP_NAVIGATION_BUDGET, SERP_SESSION_RETRIEVAL_FLOOR, SerpSearchBudget,
    };

    #[test]
    fn reserve_exhausts_and_reports_usage() {
        let mut budget = SerpSearchBudget::new(2);
        assert_eq!(budget.usage(), (0, 2));
        assert_eq!(budget.remaining(), 2);
        assert!(budget.reserve().is_ok());
        assert_eq!(budget.usage(), (1, 2));
        assert!(budget.reserve().is_ok());
        assert_eq!(budget.usage(), (2, 2));
        assert_eq!(budget.reserve(), Err(()));
    }

    #[test]
    fn settle_charges_only_the_extra_navigations_and_clamps() {
        let mut budget = SerpSearchBudget::new(4);
        assert!(budget.reserve().is_ok());
        // 一次三引擎 fallback：预留 1 + 结算补 2。
        budget.settle(3);
        assert_eq!(budget.usage(), (3, 4));
        // 单引擎成功：预留 1 已足够，不再补。
        assert!(budget.reserve().is_ok());
        budget.settle(1);
        assert_eq!(budget.usage(), (4, 4));
        // 越界结算钳制在 cap，不产生 used > cap 的伪状态。
        budget.settle(9);
        assert_eq!(budget.usage(), (4, 4));
    }

    #[test]
    fn rollback_releases_a_reservation_for_never_executed_calls() {
        let mut budget = SerpSearchBudget::new(3);
        assert!(budget.reserve().is_ok());
        assert_eq!(budget.usage(), (1, 3));
        budget.rollback();
        assert_eq!(budget.usage(), (0, 3));
        // 空预算上回滚不会下溢。
        budget.rollback();
        assert_eq!(budget.usage(), (0, 3));
    }

    /// P2-3/P2-2：激活已消耗的导航数作为初始用量承接（`continue` 重入与
    /// sidecar 恢复不重置额度）；档位下调时钳到新上限，保持"用尽"语义。
    #[test]
    fn with_used_carries_the_activation_usage_and_clamps_it() {
        let mut budget = SerpSearchBudget::with_used(8, 3);
        assert_eq!(budget.usage(), (3, 8));
        assert!(budget.reserve().is_ok());
        budget.settle(3);
        assert_eq!(budget.usage(), (6, 8));

        // 档位下调（extended → standard）时旧用量被钳到新上限。
        let mut exhausted = SerpSearchBudget::with_used(8, 16);
        assert_eq!(exhausted.usage(), (8, 8));
        assert_eq!(exhausted.reserve(), Err(()));

        // 承接用量的额度同样不承担会话底线保留（只有主车道形态承担）。
        assert!(!budget.reserves_session_floor());
    }

    #[test]
    fn zero_cap_still_admits_exactly_one_navigation() {
        let mut budget = SerpSearchBudget::new(0);
        assert_eq!(budget.usage(), (0, 1));
        assert!(budget.reserve().is_ok());
        assert_eq!(budget.reserve(), Err(()));
        assert_eq!(budget.usage(), (1, 1));
    }

    /// P2-4：主车道与 grill 各自持有独立预算实例——一个车道花掉的额度不会
    /// 影响另一个（同理，外部检索车道的额度与主车道互不挤占）。
    #[test]
    fn main_and_grill_lanes_get_independent_budgets() {
        let main = LoopProfile::main(5);
        let grill = LoopProfile::grill(5);
        let main_budget = main.serp_budget.clone().expect("main carries a budget");
        let grill_budget = grill.serp_budget.clone().expect("grill carries a budget");
        assert_eq!(
            main_budget.lock().unwrap().usage(),
            (0, MAIN_SERP_NAVIGATION_BUDGET)
        );
        assert!(main_budget.lock().unwrap().reserve().is_ok());
        assert_eq!(
            grill_budget.lock().unwrap().usage(),
            (0, MAIN_SERP_NAVIGATION_BUDGET),
            "grill budget must not observe the main lane's usage"
        );
        assert_eq!(
            main_budget.lock().unwrap().usage(),
            (1, MAIN_SERP_NAVIGATION_BUDGET)
        );
    }

    /// P2-3：主车道／grill 的额度带"会话底线保留"标记（派发前受
    /// [`SERP_SESSION_RETRIEVAL_FLOOR`] 约束）；检索车道形态不带该标记
    /// （它不是被保留的一方，会话物理上限仍由宿主把守）。
    #[test]
    fn session_floor_reservation_is_a_main_lane_attribute() {
        let main = LoopProfile::main(5);
        let grill = LoopProfile::grill(5);
        assert!(
            main.serp_budget
                .clone()
                .expect("main carries a budget")
                .lock()
                .unwrap()
                .reserves_session_floor()
        );
        assert!(
            grill
                .serp_budget
                .clone()
                .expect("grill carries a budget")
                .lock()
                .unwrap()
                .reserves_session_floor()
        );
        assert!(!SerpSearchBudget::new(8).reserves_session_floor());

        // 底线本身必须低于会话上限量级，并为检索留出 ≥ 一次 standard 档。
        assert_eq!(SERP_SESSION_RETRIEVAL_FLOOR, 16);
        assert!(
            SERP_SESSION_RETRIEVAL_FLOOR >= 8,
            "covers a standard activation"
        );

        // `used()` 是激活写回接口（与 usage() 同源）。
        let mut budget = SerpSearchBudget::new(4);
        assert_eq!(budget.used(), 0);
        assert!(budget.reserve().is_ok());
        budget.settle(3);
        assert_eq!(budget.used(), 3);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoopProfile {
    /// Whose loop this is — gates the main-only surfaces (plan status line,
    /// blackboard plan summary in the compaction marker, live text-delta
    /// forwarding).
    pub role: AgentRole,
    /// §4.5: the final-answer counterexample gate fires only in main runs
    /// (a retrieval result is not a run's formal answer). Grill turns fold
    /// this to `false` too (a grill question is not a run-semantic).
    pub counterexample_gate: bool,
    /// The orientation lane this loop's completed model rounds count
    /// toward (ADR-0010 §4.2; `None` counts nothing — grill / M3).
    pub orientation_role: Option<AgentRole>,
    /// Which system prompt to assemble per round.
    pub system_kind: SystemPromptKind,
    /// Retrieval-lane tool projection (deny-only write domain + nested
    /// dispatch guard).
    pub tool_filter: ToolFilter,
    /// Independent per-session tool-round budget (§3.4.6 — 120 default).
    pub max_tool_rounds: u32,
    /// Rounds already consumed by this session BEFORE this loop starts
    /// (user adjudication 2026-08-10, review F5): a `continue` re-entry is
    /// the SAME retrieval session — the budget accumulates across
    /// dispatches and resets only with a NEW activation (Closed → next
    /// creation starts at 0). The main agent keeps the inherited per-run
    /// semantic (`main`/`grill` = 0).
    pub initial_tool_rounds: u32,
    /// FUS-TOOL-PROBE P0-A/P0-A-2: whether this loop re-probes the work
    /// tools and re-projects the tool list before every model request.
    /// Main/grill turns yes; retrieval lanes never re-probe (their list is
    /// the caller's final projection and no second
    /// `tool_availability_check` event may fire inside a lane).
    pub probe_work_tools: bool,
    /// ACAF Slice 2 fail-closed D-13 (2026-08-13): the retrieval lane's
    /// real activation_id — bound on the lane's action tickets (web_fetch /
    /// browser_read). The main lane is `None` (main-lane actions stay
    /// activation-less).
    pub activation_id: Option<String>,
    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the current
    /// dispatch's candidate counter (per-activation shared domain — the
    /// subagent loop mutates it on every candidate gate: web_fetch family +
    /// browser_read; the caller writes it back into the activation on every
    /// path). The main/grill lanes pass `None` — candidate-counted tools
    /// never execute there, and a `None` domain fails the gate closed.
    pub fetch_candidates: Option<Arc<Mutex<Vec<String>>>>,
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
    /// 委托契约复杂度分档的同轮 `browser_read` 并发上限（standard 2 /
    /// extended 4 / deep None=不限）。主/grill lane 传 `None`。host tab
    /// 池是最终物理上限，本字段只约束 loop 批次内的并发启动。
    pub browser_read_concurrency: Option<usize>,
    /// P2-4（2026-09-10）：本车道的 SERP 引擎导航预算——主/grill = 每 run
    /// 一张（独立额度，不再与检索车道抢同一个会话计数器）；外部检索 =
    /// 每 activation 一张（按 effort 档位取额度）；内部检索 = `None`
    /// （检索声明面本就无 browser_control）。
    pub serp_budget: Option<Arc<Mutex<SerpSearchBudget>>>,
}

impl LoopProfile {
    pub(crate) fn main(max_tool_rounds: u32) -> Self {
        Self {
            role: AgentRole::Main,
            counterexample_gate: true,
            orientation_role: Some(AgentRole::Main),
            system_kind: SystemPromptKind::Main,
            tool_filter: ToolFilter::None,
            max_tool_rounds,
            initial_tool_rounds: 0,
            probe_work_tools: true,
            activation_id: None,
            fetch_candidates: None,
            browser_read_concurrency: None,
            serp_budget: Some(Arc::new(Mutex::new(SerpSearchBudget::for_main_lane(
                MAIN_SERP_NAVIGATION_BUDGET,
            )))),
        }
    }

    /// Grill-mode turn (2026-08-08): full loop, no final-answer gate
    /// (the grill answer is not a run-semantic; same reasoning as the
    /// counterexample skip comment in `run_turn_inner`).
    pub(crate) fn grill(max_tool_rounds: u32) -> Self {
        Self {
            role: AgentRole::Main,
            counterexample_gate: false,
            orientation_role: Some(AgentRole::Main),
            system_kind: SystemPromptKind::Main,
            tool_filter: ToolFilter::None,
            max_tool_rounds,
            initial_tool_rounds: 0,
            probe_work_tools: true,
            activation_id: None,
            fetch_candidates: None,
            browser_read_concurrency: None,
            serp_budget: Some(Arc::new(Mutex::new(SerpSearchBudget::for_main_lane(
                MAIN_SERP_NAVIGATION_BUDGET,
            )))),
        }
    }

    /// A retrieval subagent's loop (GAP-SUBAGENT-RUNTIME 2026-08-10):
    /// the internal/external lanes are fed — §4.2 counts every agent's
    /// completed logical model rounds. `initial_tool_rounds` carries the
    /// session's consumed budget across `continue` re-entries (user
    /// adjudication 2026-08-10, review F5 — the caller reads it back from
    /// the activation).
    /// 0k 第二批 (2026-08-30)：末尾追加 `browser_read_concurrency` 档位
    /// 参数——8 参数是已登记成本（同 write_close_record 先例）。
    /// P2-4 (2026-09-10)：再追加 `serp_budget`（每 activation 一张，按
    /// effort 档位取额度；内部检索恒 `None`）。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn retrieval(
        role: SubagentRole,
        goal: &str,
        max_tool_rounds: u32,
        initial_tool_rounds: u32,
        activation_id: &str,
        fetch_candidates: Option<Arc<Mutex<Vec<String>>>>,
        browser_read_concurrency: Option<usize>,
        serp_budget: Option<Arc<Mutex<SerpSearchBudget>>>,
    ) -> Self {
        let agent_role = match role {
            SubagentRole::InternalRetrieval => AgentRole::InternalRetrieval,
            SubagentRole::ExternalRetrieval => AgentRole::ExternalRetrieval,
        };
        Self {
            role: agent_role,
            counterexample_gate: false,
            orientation_role: Some(agent_role),
            system_kind: SystemPromptKind::Retrieval {
                role,
                goal: goal.to_string(),
            },
            tool_filter: ToolFilter::Retrieval,
            max_tool_rounds,
            initial_tool_rounds,
            probe_work_tools: false,
            activation_id: Some(activation_id.to_string()),
            fetch_candidates,
            browser_read_concurrency,
            serp_budget,
        }
    }
}

/// The shared `&self`-field subset the loop reads. The controller reference
/// rides separately — its methods (`maybe_fire_orientation`, tool dispatch,
/// status line) stay on the type.
pub(crate) struct SharedLoopServices<'a> {
    pub blackboard: &'a Arc<SharedBlackboard>,
    pub denial_state: &'a Mutex<DenialState>,
    pub pacing_rounds: &'a std::sync::atomic::AtomicU32,
    pub context_compact: &'a ContextCompactConfig,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval-lane tool-call evidence
    /// (ADR-0010 §3.7.4 — the mechanical source of the structured ledger).
    /// `Some` only on retrieval profiles; the main lane passes `None`.
    pub evidence: Option<&'a Mutex<Vec<crate::retrieval::evidence::EvidenceRecord>>>,
    /// GAP-DENIAL-POLICY-REVISION (2026-08-12): the live policy revision —
    /// feeds the role-gate denial key (a bump is a key change → the breaker
    /// resets, ADR-0010 §3.5.4).
    pub policy_revision: &'a std::sync::atomic::AtomicU64,
    /// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): the per-model-round
    /// tool-result injection budget in estimated tokens
    /// (`ORZ_MAX_INJECT_TOKENS_PER_ROUND`, default 50K). Read once at
    /// controller construction; the loop refuses later calls of a batch once
    /// the accumulated estimated tokens reach it.
    pub max_inject_tokens_per_round: u64,
    /// F5 (2026-08-15, BACKLOG 6e 复查遗留): the controller-configured
    /// epoch archive directory — the single source for the path-slot
    /// overflow pointer (never re-derived from the session cwd).
    pub blackboard_archive_dir: Option<&'a Path>,
    /// P2-13 B3（2026-09-03，ADR-0010 §14.52）：会话快照身份（ACP session
    /// id；CLI 单 run/测试控制器 = None）——压缩 marker 的「黑板会话」行。
    pub session_id: Option<&'a str>,
    /// 0k 审查处理 (P3-4, 2026-08-30)：子代理墙钟超时收口用的 in-flight
    /// 工具槽——串行路径工具执行前记录 `(tool, call_id)`、完成后移除；
    /// 超时 drop loop future 后 dispatch 据此为链上孤儿 ToolStarted 补
    /// ToolCompleted(error)（审计形态完整）。并行批次工具的事件在
    /// buffered writer 中、超时 drop 时整体丢弃，链上不产生孤儿，无需
    /// 入槽。主车道传 `None`（零开销）。
    pub in_flight_tools: Option<&'a Mutex<Vec<(String, String)>>>,
    /// 0ar S2（2026-09-19，检索批次回送设计 §3.6/§4.2）：本批已发起检索
    /// 调用计数——可见倒数行与 assessment `sufficiency_gap.retrieval_calls`
    /// 的机械来源。放 `SharedLoopServices` 而非 loop 局部量的原因：墙钟
    /// 到点路径 loop future 被丢弃、局部量随 drop 消失，而 dispatch 仍需
    /// 该读数（已得计数＋缺口）。检索车道传 `Some`（dispatch 持有
    /// AtomicU64），主车道传 `None`。
    pub retrieval_calls: Option<&'a std::sync::atomic::AtomicU64>,
}

/// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30, TODO P0-0k 第一批
/// 第 1 项)：同轮可并行读类工具集——只读/检索类可并发执行（Futures 并发、
/// 按声明序提交）；写类（search_replace 等）、命令类（run_terminal_cmd /
/// run_tests）、控制类（todo_write / plan 族 / submit）、生成类（image_*
/// / use_tool）与 web_search（生成式检索，信号量=1 + pacing 纪律）保持
/// 串行。`blackboard_read` 并发读安全（分区单写者纪律在提交阶段保持）。
pub(crate) const PARALLEL_READ_TOOLS: &[&str] = &[
    "read_file",
    "grep",
    "list_dir",
    "search_tool",
    "web_fetch",
    "browser_read",
    "blackboard_read",
    "memory_get",
    "memory_search",
    "lsp",
];

/// 0k：某调用是否可并入同轮读类并行批次——名称在集合内、实际执行目标是
/// Host（主车道 web 族走子代理派发，受单席位纪律约束不入批次；检索车道
/// lane 自执行经 Host 直跑可并入）、无写门拒绝、非计划轮。
pub(crate) fn parallel_read_eligible(
    tc: &ToolCall,
    profile: &LoopProfile,
    plan_round_active: bool,
) -> bool {
    if plan_round_active || !PARALLEL_READ_TOOLS.contains(&tc.name.as_str()) {
        return false;
    }
    let target = route(&tc.name);
    if target != DispatchTarget::Host {
        let lane_self_execute = target == DispatchTarget::ExternalRetrieval
            && profile.tool_filter.denies_nested_dispatch();
        if !lane_self_execute {
            return false;
        }
    }
    profile.tool_filter.write_gate(&tc.name).is_none()
}

/// What the loop produced — the caller maps it to its own terminal
/// semantics (main: run_finished/run_invalidated; subagent: result
/// formation / budget_exhausted / subagent_failed).
pub(crate) struct LoopOutcome {
    pub last_text: Option<String>,
    pub tool_rounds: u32,
    /// Rounds since the last compaction at loop exit — the session-end
    /// compaction reports it honestly (P0-D review fix 2026-08-14).
    pub rounds_since_compact: u32,
    /// Read by the subagent terminal mapping (M3) — the main caller's
    /// terminal event carries `tool_rounds` only.
    #[allow(dead_code)] // consumed by the subagent path (GAP-SUBAGENT-RUNTIME M3)
    pub budget_exhausted: bool,
    /// 0ar S2（2026-09-19，检索批次回送设计 §3/§4）：检索批次收尾成因——
    /// `Some` 仅在检索车道（阈值回送 / 机械护栏 / 墙钟到点）；主车道恒
    /// `None`。dispatch 据此映射 `retrieval_close_record.terminal_reason`
    /// 正常收尾臂（`evidence_threshold_met` / `dispatch_wallclock_bound`），
    /// 不复用 `RetrievalSubagentEarlyClose` 失败臂。
    pub retrieval_close: Option<crate::retrieval::batch_close::BatchCloseKind>,
    /// 本批（车道会话）已发起检索调用次数——可见倒数与 assessment
    /// `sufficiency_gap.retrieval_calls` 的机械来源（墙钟到点路径 loop
    /// future 已丢弃，计数器必须在 dispatch 侧存活 ⇒ 走
    /// `SharedLoopServices` 的 AtomicU64）。
    pub retrieval_calls: u64,
}

/// Outcome of one template-summary attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompactDecision {
    /// Nothing collapsible (or the trigger did not reach this path).
    NoOp,
    /// Collapsible content exists but the reduction guard is not
    /// satisfiable and the compaction was not forced.
    GuardBlocked,
    /// A compaction executed (机械模式：零模型调用，2026-08-18 B 定案，
    /// ADR-0010 §14.29——无 summary_incomplete 终止态).
    Executed,
}

/// 机械模板压缩的保留起点（**v8 起＝收尾／检索车道既有语义**）。
///
/// 主车道的**机械驱逐／推进**随滑块勘误退役（设计 v8 §10：`advance_fold`／
/// 驻留带预算 L／H−L 参数体系退役）——模型面改由投影层承载、`messages`
/// 不再被压缩 drain。本函数因此只保留 `collapsed_cut(tail)` 一条语义，
/// 供**检索/grill 车道**与**会话收尾**（模型已离场）的机械模板压缩使用。
fn compact_fallback_cut(messages: &[Message], tail: usize) -> Option<usize> {
    crate::action_ledger::collapsed_cut(messages, tail)
}

/// v7 原文定位指针（S1 修订批，设计 §3.5.1）：conversation sidecar 路径提示
/// ——`<session_cwd>/.gsa/conversations/<session8>.json`（会话 id 前 8 字符，
/// 与 host 侧 `conversation_sidecar_path` 同口径）。`session_id` 缺失
/// （CLI 单 run / 测试控制器）时返回 `None`，marker 如实渲染「（无）」。
fn conversation_sidecar_hint(host: &dyn LoopHost, session_id: Option<&str>) -> Option<String> {
    let session_id = session_id?;
    let suffix: String = session_id.chars().take(8).collect();
    Some(
        host.session_cwd()
            .join(".gsa")
            .join("conversations")
            .join(format!("{suffix}.json"))
            .display()
            .to_string(),
    )
}

/// The shared mechanical compaction flow (P0-D S3 + review fixes +
/// 2026-08-18 B 定案, ADR-0010 §14.29；v0.2 五段模板 marker).
///
/// **0ah 收口清理批（2026-09-16，v7→v8 收口）后的生产形态**：唯一生产调用方
/// ＝**检索/grill 车道的 session-end 压缩**（`retrieval::dispatch`；主车道的
/// loop-top 触发与收尾压缩已随 v8 勘误退役——模型面由投影层承载，本地面
/// 全程逐字全量）。保留起点一律按无状态 `collapsed_cut(messages, tail)` 重算。
///
/// Makes ZERO model calls.
/// Marker 双轨（2026-09-04 复审处理，车道范围裁决）：
/// - `fold_ctx = Some(主车道 LIF round/domain)` 且保留尾首条声明消息带轮章
///   时 → v0.3 压缩点冻结黑板折叠视图快照（A–E 块，r_keep 排除保留尾行；
///   P2-14 域，主会话压缩专用——v8 后生产恒走 None 路径，机制保留待 P2-14
///   S3/S4 裁决）；
/// - 其余（检索/grill 车道、旧会话消息无轮章）→ v0.2 五段模板（既有语义
///   原样保留：目的/计划/变动文件路径机械填充、注意事项 = HA 结构化事实
///   聚合、后续衔接 = 固定中性占位）。
/// 两条路径都：存档恒写入（审计副本 + digest，bounded retries，失败显式
/// 上报）、滚动单 marker 插入、`context_compressed` v0.2 事件（mode:
/// "mechanical"，事件面不变）。v1.15：压缩永不触碰黑板（快照只读）。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_template_compact(
    svc: &SharedLoopServices<'_>,
    writer: &mut EventWriter<'_>,
    host: &dyn LoopHost,
    messages: &mut Vec<Message>,
    measured: u64,
    reason: &str,
    force: bool,
    guard_failed: bool,
    rounds_since: u32,
    tail: usize,
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28 审查修复): the external-ledger path hint for the marker —
    // main lane only (检索车道不折叠); the hint is further filtered to
    // files that exist or a fold that happened, so a restored
    // conversation never gets a dangling pointer.
    ledger_path: Option<&std::path::Path>,
    // P2-14 S1（2026-09-04，ADR-0010 §14.54）：主会话折叠快照 LIF 上下文。
    // 非 Main 车道（检索/grill）传 None → 保持 v0.2 五段模板（车道范围
    // 裁决见模块注释）；调用方在压缩触发点取 `blackboard_stamp()`。
    fold_ctx: Option<crate::summary::FoldSnapshotCtx>,
    // v7（S1 修订批，2026-09-15，设计 §3.5.1）：原文定位指针四项——每条压缩
    // marker／摘要块必带（compaction 存档＋digest／台账 `[seq]` 区间／
    // journal run+sequence／conversation sidecar 路径）。
    locators: &crate::summary::LocatorPointers,
) -> Result<CompactDecision, AgentLoopError> {
    // FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27 / 处理文档
    // LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING_2026-08-18 §2.1): the rolling
    // single marker is NOT removed on the guard path — any in-place mutation
    // before execution is confirmed would leave a stale preamble
    // `[U0, A[...]]` (declaration without its tool reply) → provider 400
    // `insufficient tool messages`. The marker is removed only once
    // execution is confirmed; the cut is recomputed below.
    // 0ah 收口清理批（2026-09-16，v7→v8 收口）：**有状态折叠点退役**——
    // `LedgerFoldState`／`fold_cut` 随 v8 勘误整体下线后，本函数（唯一生产
    // 调用方＝检索/grill 车道 session-end）恒为未折叠态，保留起点一律按
    // 无状态 `collapsed_cut(messages, tail)` 重算。
    let cfg = svc.context_compact;
    // v8（2026-09-16 勘误批）：主车道的**机械驱逐／推进随勘误退役**——模型面
    // 由投影层（`model_face`）承载、`messages` 不再被压缩 drain，故本函数的
    // 主车道按块压缩路径不再存在；保留语义＝**检索/grill 车道**与**会话收尾**
    // （模型已离场）的机械模板压缩：`collapsed_cut(messages, tail)`。
    let Some(kept_start) = compact_fallback_cut(messages, tail) else {
        return Ok(CompactDecision::NoOp);
    };
    // Guard 口径：含旧 marker 的数组 + 冻结 kept_start（触发时不动数组）。
    // Guard 仍是「缩减比 ＋ 最小可压」，force 语义不变。
    // **v8（2026-09-16 勘误批）：语义轨的 drain 替换路径退役**——模型自压改走
    // `compress_blocks_now`（按块、本地面零覆盖），本函数只剩机械模板轨
    // （检索/grill 车道与会话收尾）。
    let marker_estimate = crate::summary::SUMMARY_MARKER_ESTIMATE_TOKENS;
    let after = estimate_messages_tokens(&messages[..kept_start])
        + estimate_messages_tokens(&messages[kept_start..])
        + marker_estimate;
    let removable = measured.saturating_sub(after);
    let reduction_ok = after as f64 <= measured as f64 * cfg.max_reduction_ratio;
    if !force && (removable < cfg.min_compactable || !reduction_ok) {
        return Ok(CompactDecision::GuardBlocked);
    }

    // 执行已确认：删除旧 marker 并重算 kept_start（无状态重算，marker 删除
    // 使消息索引整体左移一位）。事件估计在 drain + marker 插入后
    // 由实际数组重算（见下方 final_after），与执行后数组一致。
    let had_marker = messages.iter().any(|m| {
        m.content
            .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
    });
    if had_marker {
        messages.retain(|m| {
            !m.content
                .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
        });
    }
    let kept_start = compact_fallback_cut(messages, tail)
        .expect("guard path already resolved a kept_start for the same messages");

    let rounds_dropped = crate::action_ledger::rounds_before(messages, kept_start) as u32;
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28 审查修复): the marker carries the fixed external-ledger path
    // hint so a restored conversation points the model at the surviving
    // history (the file is append-only and NOT reset by compaction). The
    // hint is written only when the file exists — never a dangling pointer
    // for a conversation that never folded（0ah 收口清理批：`is_folded`
    // 折叠态条件随有状态折叠点退役）.
    let ledger_hint = ledger_path.filter(|p| p.exists());
    // The archive id rides the writer's CURRENT seq — no event is recorded
    // between here and the `context_compressed` journal, so the id is
    // stable and unique within the run.
    let id = format!("compaction-{}-{:04}", writer.run_id(), writer.seq());
    let archive_dir = host.session_cwd().join(".gsa").join("compaction");
    let archive_path = archive_dir.join(format!("{id}.md"));
    // v1.15 (2026-08-14) 起路径槽溢出指针指向当前 plan-epoch 快照；F5
    // (2026-08-15) 后路径取自定义归档目录（单一来源）。P2-13 B3
    // (2026-09-03)：生产面不再写 epoch 快照（plan_epoch 恒 0 → 指针回退
    // 摘要存档）；epoch 指针仅 --plan/测试域配置归档目录后产生；marker 的
    // plan_epoch 行已退役为「黑板会话」行。
    let plan_epoch = svc.blackboard.read().plan.plan_epoch;
    let epoch_archive = (plan_epoch > 0).then(|| {
        svc.blackboard_archive_dir
            .map(|dir| dir.join(format!("epoch-{plan_epoch}.json")))
    });
    let epoch_archive = epoch_archive.flatten();
    let session_snapshot = svc.session_id;
    let first_round_start = messages
        .iter()
        .position(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .unwrap_or(messages.len());
    // P2-14 S1：被压区间首轮（首条被 drain 声明消息的轮章）在 drain 前
    // 捕获（旧会话消息无章时为 None → 存档/marker 不虚构轮区间）。
    let drained_first_round = messages[first_round_start..kept_start]
        .iter()
        .find(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .and_then(|m| m.round);
    let dropped = rounds_dropped;
    let messages_dropped = messages.drain(first_round_start..kept_start).count();
    // v8 勘误（2026-09-16）：**fallback 第二段截断退役**（`reason == "fallback"`
    // 已不再由任何生产路径产生——它是 v7 的 256K 视图兜底，属「总窗口压缩」，
    // 随用户裁定整体退役）。收尾／检索车道的机械模板压缩只有一段 drain。
    let insert_at = first_round_start;
    // 机械模式（2026-08-18 B 定案）：存档恒写入（审计副本 + digest），
    // marker 恒携带真实 digest/路径——无 summary_incomplete 终止态。
    // 审查修复（2026-08-19）：存档/marker 在 drain + fallback 截断后定稿，
    // 「被压轮次」= 总轮数，与事件口径一致。
    // P2-14 S1：r_keep = 保留尾首条声明消息的轮章（drain 后取，fallback
    // 截断后仍以最终保留尾为准）；主会话传 ctx 且 r_keep 可得 → v0.3 折叠
    // 快照 marker；否则（非 Main 车道/旧会话无章）→ v0.2 五段模板回退。
    let r_keep = messages[insert_at..]
        .iter()
        .find(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .and_then(|m| m.round);
    let ledger_hint_text = ledger_hint.map(|p| p.display().to_string());
    // 两分支各自赋值（v0.3 折叠快照 / v0.2 五段模板），事件与 marker 共用。
    // **v8（2026-09-16 勘误批）：语义轨 drain 分支退役**——模型自压改走
    // `compress_blocks_now`（按块、本地面零覆盖、`v0.4-分块压缩` marker）。
    let archive_write_failed;
    let (markdown, marker) = if let (Some(ctx), Some(r_keep)) = (fold_ctx, r_keep) {
        // v0.3：先装配存档文本（不含 digest 自引用）→ 写盘 → 得
        // archive_write_failed → 再以真实失败态生成 marker（A 注记）。
        let budget = crate::summary::CompactionBudget::from_env();
        let make = |archive_write_failed: bool| {
            let bb = svc.blackboard.read();
            crate::summary::build_fold_snapshot_marker(
                &bb,
                &crate::summary::FoldSnapshotInput {
                    id: &id,
                    archive_path: &archive_path,
                    session: session_snapshot,
                    rounds_dropped: dropped,
                    round_from: drained_first_round,
                    r_keep,
                    ctx,
                    guard_failed,
                    archive_write_failed,
                    ledger_note: ledger_hint_text.as_deref(),
                    // 0ah 收口清理批：有状态折叠点退役 ⇒ 冻结台账指针恒缺席
                    //（折叠态才可能产出 `folded_ledger`）。
                    frozen_ledger: None,
                    locators,
                    budget,
                },
            )
        };
        let first = make(false);
        archive_write_failed =
            !crate::summary::write_archive_retry(&archive_dir, &archive_path, &first.archive);
        let final_out = make(archive_write_failed);
        (first.archive.clone(), final_out.marker)
    } else {
        // v0.2 五段模板回退（非 Main 车道 / 旧会话无轮章；语义不变）。
        let mut failure_annex: Option<Vec<String>> = None;
        let slots = {
            let bb = svc.blackboard.read();
            let (purpose, plan, paths) =
                crate::summary::mechanical_slots(&bb, &archive_path, epoch_archive.as_deref());
            let notes = crate::summary::render_facts_notes(&bb);
            if !notes.hidden_failure_rows.is_empty() {
                failure_annex = Some(notes.hidden_failure_rows);
            }
            crate::summary::SummarySlots {
                purpose,
                plan,
                paths,
                notes: notes.text,
                continuation: crate::summary::MECHANICAL_CONTINUATION_PLACEHOLDER.to_string(),
            }
        };
        let markdown = crate::summary::summary_archive_markdown(
            &id,
            &slots,
            dropped,
            guard_failed,
            failure_annex.as_deref(),
            // 0ah 收口清理批：有状态折叠点退役 ⇒ 恒 None（该槽渲染「（无）」）。
            None,
        );
        let digest = crate::summary::archive_digest(&markdown);
        archive_write_failed =
            !crate::summary::write_archive_retry(&archive_dir, &archive_path, &markdown);
        let marker = crate::summary::build_summary_marker(
            &id,
            &digest,
            &archive_path,
            &slots,
            dropped,
            guard_failed,
            archive_write_failed,
            session_snapshot,
            ledger_hint,
            locators,
        );
        (markdown, marker)
    };
    let digest = crate::summary::archive_digest(&markdown);
    messages.insert(
        insert_at,
        Message {
            role: Role::User,
            content: marker,
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        },
    );
    // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the conversation
    // was mutated (drain + marker) — 0ah 收口清理批（2026-09-16）：有状态
    // 折叠点已退役，无需再 reset；冻结索引/`fold_cut` 语义随 v8 下线。
    // 审查修复（2026-08-19）：事件估计在 marker 插入后重算（含 marker，
    // 与 schema「marker + preamble + recent tail」口径一致；fallback 的
    // 截断目标估算不再单独使用——真值含 marker）。
    let final_after = estimate_messages_tokens(messages);
    // `retained_rounds`＝压缩后 drain 尾轮数（机械模板轨既有口径，2）。
    let retained_rounds = tail as u32;
    writer
        .record(
            EventType::ContextCompressed,
            serde_json::json!({
                "trigger_tokens": measured,
                "target_tokens": final_after,
                "rounds_since_last_compaction": rounds_since,
                "rounds_dropped": dropped,
                "messages_dropped": messages_dropped,
                "messages_kept": messages.len(),
                "estimated_tokens_after": final_after,
                // 机械模板轨：mode=mechanical、reason 照旧（检索/grill 车道与
                // 会话收尾）。模型自压的按块压缩事件由 `compress_blocks_now`
                // 产出（mode=model_summary、v0.4-分块 marker）。
                "mode": "mechanical",
                "reason": reason,
                "summary_id": id,
                "summary_digest": digest,
                "summary_path": archive_path.display().to_string(),
                "summary_incomplete": false,
                "retained_rounds": retained_rounds,
                "guard_failed": guard_failed,
                "archive_write_failed": archive_write_failed,
            }),
        )
        .await?;
    Ok(CompactDecision::Executed)
}

/// 0ae D3 窗口收口的**统一出口**（2026-09-15 窄边沿修复）：凡携带未收口
/// `model_compression_close` 状态离开本迭代的路径——下一轮 loop-top、
/// budget 耗尽收尾 break、IPG block break、run 尾安全网——都必须先经本
/// 函数，保证收口审计事件要么落地、要么明确无窗口可收（`take` 幂等，
/// 多处调用安全）。按派发后真实黑板写数判定 `model_participated`、按 C2
/// 形状落 `model_compression` 审计事件。**v8（2026-09-16 勘误批）：不再置
/// 机械兜底旗标**——窗口收口只有两条去路：模型产出了语义摘要（⇒ 下一次
/// loop-top 按块压缩）或没产出（⇒ 如实落账 `model_participated=false`，
/// 机械层不做任何压缩，设计 §4「模型面总量只由模型自压与 H1/T1 管」）。
/// 边界：`?` 错误传播路径仍会丢弃状态（与 0AC-A6 Err 留痕同类，已挂账）。
async fn finalize_model_compression_close(
    state: &mut Option<(usize, u32)>,
    audit: &mut crate::mechanical_audit::MechanicalAuditState,
    svc: &SharedLoopServices<'_>,
    writer: &mut EventWriter<'_>,
    tool_rounds: u32,
) -> Result<(), AgentLoopError> {
    let Some((window_start_writes, rounds_used)) = state.take() else {
        return Ok(());
    };
    let writes_now = svc.blackboard.read().model_note_count();
    let participated = writes_now > window_start_writes;
    let payload = audit.record(
        "model_compression",
        tool_rounds,
        format!(
            "model_participated={participated} rounds_used={rounds_used} \
             window_start_writes={window_start_writes} writes_now={writes_now}"
        ),
        if participated {
            None
        } else {
            Some("model_not_participated".to_string())
        },
    );
    writer
        .record(
            EventType::MechanicalAuditUpdate,
            serde_json::json!({
                "kind": crate::mechanical_audit::KIND_MODEL_COMPRESSION,
                "payload": payload,
            }),
        )
        .await?;
    Ok(())
}

/// v8（2026-09-16 勘误批）分块压缩的执行结果。
#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockCompaction {
    pub(crate) blocks: usize,
    pub(crate) rounds: u32,
    pub(crate) freed_tokens: u64,
}

/// 黑板水位标记（与 `AgentLoopController::blackboard_watermark_label` 同口径；
/// 供不带 controller 的截断路径复用）。
fn blackboard_watermark_label(svc: &SharedLoopServices<'_>) -> String {
    let bytes = svc.blackboard.read().live_compact_bytes();
    let budget = crate::fatigue::live_budget_bytes();
    format!(
        "【{:.1}M/{}M】",
        bytes as f64 / 1_000_000.0,
        budget / 1_000_000
    )
}

/// 请求**静态开销**估算（chars/2 同尺）：系统提示词 ＋ 工具定义 JSON。
///
/// 2026-09-16 实现批（审查 R-9）：阶梯与守卫量的是「发往模型商的上下文」，
/// 只算投影层消息会系统性偏小（系统提示词与工具定义动辄数 K 估算，且工具面
/// 声明会随会话变化）⇒ 把它们计入读数，阶梯落点才与设计意图对齐。
fn estimate_static_overhead(system: &str, tool_defs: &[ToolDef]) -> u64 {
    let mut chars = system.chars().count() as u64;
    for def in tool_defs {
        // ToolDef 不是 Serialize ⇒ 按字段逐项量（名 ＋ 描述 ＋ 参数 schema）。
        chars += def.name.chars().count() as u64;
        chars += def.description.chars().count() as u64;
        chars += serde_json::to_string(&def.parameters)
            .map(|s| s.chars().count() as u64)
            .unwrap_or(0);
    }
    chars / 2
}

/// **按块压缩**（滑块上下文 v8，设计 §4 §6；取代 v7 的「摘要替换被压区」drain
/// 形态）——模型产出的语义摘要（可带 `压缩块: 1-4` 区间指令）⇒ 机械层把指定
/// 的**已闭合**分块从模型面下移：
///
/// ① **结构化轨**：被压块的工具／命令／结果调用取 `rows_for_round` 成台账
///    摘要行，追加外挂台账（`[seq]` 续号；**写失败 ⇒ 整体回滚不压**，同 v7
///    纪律）；
/// ② **回放面**（形态①）：每块逐字原文落盘
///    `.gsa/compaction/blocks/<tag>-block-<k>.md` ＋ 陈旧性标注（Z形指针的
///    回放口，零新工具）；
/// ③ 压缩存档（`.gsa/compaction/<id>.md`，摘要＋机械行＋四项定位指针，不复制
///    逐字正文）＋ `context_compressed` 事件（`mode=model_summary|mechanical`，
///    `reason=model_selected|context_scale_window`）；
/// ④ 往会话**追加**压缩 marker（`[前文上下文已压缩 v0.4-分块压缩]`，restore-
///    retained）——**本地面一条不删**（不变量 I3；块状态按块号幂等、恢复后由
///    marker 重建）。
///
/// 返回 `None` ＝ NoOp（无可压闭合块 / 区间无命中 / 台账写失败），**不虚构
/// 事件、不改模型面**。
#[allow(clippy::too_many_arguments)]
async fn compress_blocks_now(
    svc: &SharedLoopServices<'_>,
    writer: &mut EventWriter<'_>,
    host: &dyn LoopHost,
    messages: &mut Vec<Message>,
    params: &crate::model_face::ModelFaceParams,
    archive_tag: &str,
    selection: Option<Vec<u32>>,
    summary: Option<&str>,
    reason: &str,
    locators: &crate::summary::LocatorPointers,
) -> Result<Option<BlockCompaction>, AgentLoopError> {
    let blocks = crate::model_face::blocks_outside_slider(
        messages,
        params.slider_tokens,
        params.block_tokens,
    );
    let markers = crate::model_face::face_markers(messages);
    // 只压**已闭合**且仍为原文的块（末块仍在增长 ⇒ 摘要会与正文漂移而落空）；
    // 同一块在会话内不重复计数（去重键＝块号，设计 §12）。
    let mut selected: Vec<crate::model_face::ContextBlock> = blocks
        .iter()
        .filter(|b| b.closed && markers.state(b.number) == crate::model_face::BlockState::Live)
        .filter(|b| selection.as_ref().is_none_or(|sel| sel.contains(&b.number)))
        .cloned()
        .collect();
    // 未指定区间 ⇒ **最旧块优先**（设计 §4）：2026-09-16 实现批收窄为**只压
    // 最旧一个已闭合块**——此前会把全部已闭合块一次压掉（一个摘要吞掉最多
    // 数百 K 估算的历史），与「模型自己控制压多少」相反、也没有界。要一次压
    // 多块，模型在摘要块里写明 `压缩块: 1-4` 即可（该口径由钉子锁定）。
    if selection.is_none() && selected.len() > 1 {
        selected.truncate(1);
    }
    if selected.is_empty() {
        return Ok(None);
    }
    let ranges = crate::action_ledger::round_ranges(messages);
    // ① 结构化轨 → 外挂台账（写失败 ⇒ 回滚：行丢了、视图又不能改，宁可不压）。
    let mut rows = Vec::new();
    // 每块写了几行（用于把批次 `[seq]` 区间**拆成分块级三键**，设计 §2 §11）。
    let mut per_block_rows: Vec<(u32, usize)> = Vec::new();
    for block in &selected {
        let before_rows = rows.len();
        for round in block.first_round..=block.last_round {
            if let Some(&range) = ranges.get(round) {
                rows.extend(crate::action_ledger::rows_for_round(messages, range, round));
            }
        }
        per_block_rows.push((block.number, rows.len().saturating_sub(before_rows)));
    }
    let ledger_seq = match params.ledger_path.as_deref() {
        Some(path) => match crate::action_ledger::append_ledger_rows_range(path, &rows) {
            Ok(seq) => seq,
            Err(err) => {
                tracing::warn!(
                    ledger_path = %path.display(),
                    error = %err,
                    "v8 block compaction: external ledger append failed — compaction skipped"
                );
                return Ok(None);
            }
        },
        None => None,
    };
    // 分块级台账定位（批次区间按行数拆；无工具行的块不虚构键）。
    let ledger_locators: Vec<(u32, (u64, u64))> = match ledger_seq {
        Some((first, _)) => {
            let mut cursor = first;
            per_block_rows
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|(number, count)| {
                    let from = cursor;
                    let to = cursor + *count as u64 - 1;
                    cursor = to + 1;
                    (*number, (from, to))
                })
                .collect()
        }
        None => Vec::new(),
    };
    // ② 回放面：每块逐字原文落盘（失败如实标注，不阻塞压缩）。
    let mut replay = Vec::new();
    let mut block_archive_failed = false;
    for block in &selected {
        let path =
            crate::model_face::block_archive_path(&host.session_cwd(), archive_tag, block.number);
        let markdown = crate::model_face::block_archive_markdown(
            block,
            messages,
            params.run_id.as_str(),
            svc.session_id,
        );
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        if !crate::summary::write_archive_retry(dir.as_path(), &path, &markdown) {
            block_archive_failed = true;
        }
        replay.push(crate::model_face::replay_line(
            block,
            &crate::model_face::relative_block_archive(archive_tag, block.number),
        ));
    }
    // ③ 压缩存档 ＋ 事件 ＋ ④ marker（本地面零覆盖：只追加、不 drain）。
    let id = format!("compaction-{}-{:04}", writer.run_id(), writer.seq());
    let archive_dir = host.session_cwd().join(".gsa").join("compaction");
    let archive_path = archive_dir.join(format!("{id}.md"));
    let before = crate::model_face::estimate_model_face_tokens(messages, params);
    let numbers: Vec<u32> = selected.iter().map(|b| b.number).collect();
    let first_round = selected.first().map(|b| b.first_round).unwrap_or(0);
    let last_round = selected.last().map(|b| b.last_round).unwrap_or(0);
    let rounds: u32 = selected
        .iter()
        .map(|b| (b.last_round - b.first_round + 1) as u32)
        .sum();
    let freed_tokens: u64 = selected.iter().map(|b| b.estimate_tokens).sum();
    let messages_hidden: usize = selected
        .iter()
        .map(|b| b.msg_end.saturating_sub(b.msg_start))
        .sum();
    let mechanical_rows = crate::action_ledger::build_ledger_block(&rows);
    let locators = crate::summary::LocatorPointers {
        ledger_seq,
        // v8：按块压缩**不 drain** 会话本体 ⇒ sidecar 仍含逐字原文（审查 R-4）。
        local_face_full: true,
        ..locators.clone()
    };
    let markdown = format!(
        "# ORZ 会话压缩摘要（v8 分块压缩 {id}）\n\n\
         - 状态: complete（mode={mode}；按块压缩，主滑块以外的分块被替换为摘要）\n\
         - 事件 reason: {reason}\n- 处理分块: {numbers}\n\
         - 被处理轮次: 轮 {from}–{to}（{rounds} 轮）\n\
         - 模型面估算: 压缩前 {before}（释放 ≈{freed_tokens}tk token；压缩后读数见 marker 行）\n\
         - 黑板会话: {session}\n- 摘要存档: {path}\n\
         - 回放档案: {replay}\n\n\
         ## 原文定位（四项）\n{locator_lines}\n\n\
         ## 机械摘要行（结构化轨）\n{rows}\n\n\
         ## 语义摘要（模型产出）\n{summary}\n\n\
         ## 模型面声明\n{declaration}\n",
        mode = if summary.is_some() {
            "model_summary"
        } else {
            "mechanical"
        },
        numbers = crate::model_face::render_block_numbers(&numbers),
        from = first_round + 1,
        to = last_round + 1,
        session = svc.session_id.unwrap_or("（无）"),
        path = archive_path.display(),
        replay = replay.join("\n"),
        locator_lines = locators.render_marker_lines(&archive_path, "（见 marker 行）"),
        rows = mechanical_rows,
        summary = summary.unwrap_or("（无：纯结构化轨）"),
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
    );
    let archive_write_failed =
        !crate::summary::write_archive_retry(&archive_dir, &archive_path, &markdown);
    let digest = crate::summary::archive_digest(&markdown);
    // 0bh ③＋⑮（2026-09-22）：定位符行——折块与截断的**压缩回执**带
    // `r<轮>·b<块>·s<journal seq>` 指针（设计 §4.1 三生成点之回执面；
    // `journal_from` 取窗口级 journal 起点，s 口径与分块表一致）。
    let pointer_line = crate::model_face::render_pointer_line(
        &selected
            .iter()
            .map(|b| (b.number, b.first_round))
            .collect::<Vec<_>>(),
        &ledger_locators,
        locators.journal_seq.map(|(from, _)| from),
    );
    let marker = crate::model_face::compression_marker(
        &numbers,
        first_round,
        last_round,
        &id,
        &digest,
        &archive_path,
        svc.session_id,
        &mechanical_rows,
        summary,
        &replay.join("\n"),
        &locators,
        &ledger_locators,
        &pointer_line,
    );
    let insert_at = ranges
        .get(first_round)
        .map(|&(start, _)| start)
        .unwrap_or(messages.len());
    messages.insert(
        insert_at,
        Message {
            role: Role::User,
            content: marker,
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        },
    );
    let after = crate::model_face::estimate_model_face_tokens(messages, params);
    // 0bc S2④：只需条数——用不物化计数面（与装配逐条同口径），不再为取
    // `len` 物化整个模型面。
    let face_messages = crate::model_face::model_face_message_count(messages, params);
    writer
        .record(
            EventType::ContextCompressed,
            serde_json::json!({
                "trigger_tokens": before,
                "target_tokens": after,
                "rounds_since_last_compaction": 0,
                "rounds_dropped": rounds,
                "messages_dropped": messages_hidden,
                "messages_kept": face_messages,
                "estimated_tokens_after": after,
                "mode": if summary.is_some() { "model_summary" } else { "mechanical" },
                "reason": reason,
                "summary_id": id,
                "summary_digest": digest,
                "summary_path": archive_path.display().to_string(),
                "summary_incomplete": false,
                "retained_rounds": crate::action_ledger::rounds_before(messages, messages.len()) as u32,
                "guard_failed": false,
                "archive_write_failed": archive_write_failed || block_archive_failed,
            }),
        )
        .await?;
    Ok(Some(BlockCompaction {
        blocks: selected.len(),
        rounds,
        freed_tokens,
    }))
}

/// **T1 硬截断 / 1.10M 异常保险**（滑块上下文 v8，设计 §5）：把**主滑块以外
/// 的全部分块**移出模型面——本地面逐字一条不删（不变量 I3）、每块逐字原文
/// 落盘可按块回放、机械层注入告知块（①已截断 N 块／≈M token；②可按块回放；
/// ③任务无需中止），事实经 `mechanical_audit_update` 的 anomaly
/// `hard_context_truncated_blocks` 落账。
///
/// 返回 `None` ＝ 主滑块之外已无分块（无从截断、零事件）。
#[allow(clippy::too_many_arguments)]
async fn truncate_model_face_blocks(
    svc: &SharedLoopServices<'_>,
    writer: &mut EventWriter<'_>,
    host: &dyn LoopHost,
    messages: &mut Vec<Message>,
    params: &crate::model_face::ModelFaceParams,
    archive_tag: &str,
    guard_hit: bool,
    audit: &mut crate::mechanical_audit::MechanicalAuditState,
    tool_rounds: u32,
) -> Result<Option<(usize, u64)>, AgentLoopError> {
    let blocks = crate::model_face::blocks_outside_slider(
        messages,
        params.slider_tokens,
        params.block_tokens,
    );
    let markers = crate::model_face::face_markers(messages);
    // 2026-09-16 实现批（审查 R-1）：只截**已闭合**块。未闭合的**残段**
    // 留在模型面（它仍在增长；截断它＝隐藏工作现场，违反 I1）。
    let live: Vec<crate::model_face::ContextBlock> = blocks
        .iter()
        .filter(|b| b.closed && markers.state(b.number) == crate::model_face::BlockState::Live)
        .cloned()
        .collect();
    // 触发线：T1＝阶梯的硬截断档；守卫＝1.10M 异常保险。
    let target_line = if guard_hit {
        svc.context_compact.model_face_guard_tokens
    } else {
        crate::model_face::ladder_truncate_tokens(&svc.context_compact.context_scale_ladder)
    };
    let archive_dir = host.session_cwd().join(".gsa").join("compaction");
    // 指针化原文的落盘目录（审查 R-7：改前先落逐字原文）。
    let pointerized_dir = archive_dir.join("pointerized");
    if live.is_empty() {
        // 主滑块之外已无分块（溢出体量在**主滑块内**——例：单轮超大工具结果）
        // ⇒ 走**窗口内超大结果指针化**（0ah §10.10 用户裁定；先例
        // OUTPUT-DEGENERATION-GUARD）把正文换成「原文头部＋回读指针」，
        // 配对字段不动、幂等、大者优先。两者皆无 ⇒ 如实 NoOp（不改模型面、
        // 不虚构事件）。
        // 顺序（设计 §6/§12）：① **先裁回放块**（可再生——原文仍在该按块档案里）
        // ② 再把其余超大工具结果换成「头部 ＋ 落盘指针」（改前先把逐字原文落盘，
        // 审查 R-7：journal 不带结果正文，旧文案的「按 call_id 回读」对读类结果
        // 不成立）。
        let replay_reclaimed = crate::action_ledger::pointerize_replay_tool_results(
            messages,
            target_line,
            &host.session_cwd(),
        );
        let pointerized = crate::action_ledger::pointerize_oversized_tool_results(
            messages,
            target_line,
            crate::action_ledger::OVERSIZED_TOOL_RESULT_CAP_TOKENS,
            &pointerized_dir,
            archive_tag,
            Some(host.journal().events_path().as_path()),
            writer.run_id(),
        );
        if replay_reclaimed.replaced == 0 && pointerized.replaced == 0 {
            return Ok(None);
        }
        let freed = replay_reclaimed
            .freed_tokens
            .saturating_add(pointerized.freed_tokens);
        let payload = audit.record(
            "context_scale:hard_truncate",
            tool_rounds,
            format!(
                "truncated_blocks=0 pointerized_results={} replay_reclaimed={} freed_tokens={freed} \
                 guard_hit={guard_hit} archive_write_failed=false ledger_write_failed=false \
                 model_face_estimate_after={}",
                pointerized.replaced,
                replay_reclaimed.replaced,
                crate::model_face::estimate_model_face_tokens(messages, params),
            ),
            Some(if guard_hit {
                "hard_context_guard_result_pointerized".to_string()
            } else {
                "hard_context_result_pointerized".to_string()
            }),
        );
        writer
            .record(
                EventType::MechanicalAuditUpdate,
                serde_json::json!({
                    "kind": crate::mechanical_audit::KIND_CONTEXT_SCALE,
                    "payload": payload,
                }),
            )
            .await?;
        let notice = if guard_hit {
            crate::context_scale::guard_truncation_notice_block(
                0,
                freed,
                crate::model_face::estimate_model_face_tokens(messages, params),
                target_line,
                &format!(
                    "- （工作现场之外无**已闭合**分块；本轮把 {} 条回放块与 {} 个超大工具结果\
                     正文换成指针，逐字原文均已落盘）",
                    replay_reclaimed.replaced, pointerized.replaced
                ),
                false,
            )
        } else {
            crate::context_scale::truncation_notice_block(
                0,
                freed,
                crate::model_face::estimate_model_face_tokens(messages, params),
                &format!(
                    "- （工作现场之外无**已闭合**分块；本轮把 {} 条回放块与 {} 个超大工具结果\
                     正文换成指针，逐字原文均已落盘）",
                    replay_reclaimed.replaced, pointerized.replaced
                ),
                "- （无已闭合分块：溢出体量在工作现场内）",
                false,
            )
        };
        let watermark = blackboard_watermark_label(svc);
        messages.push(Message {
            role: Role::User,
            content: format!("{notice}\n{watermark}"),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        return Ok(Some((0, freed)));
    }
    let ranges = crate::action_ledger::round_ranges(messages);
    // ①a 分块台账行（三键之一；best-effort——截断是安全闸门，台账失败不阻断，
    // 但必须如实落账，不静默）。
    let mut rows = Vec::new();
    let mut per_block_rows: Vec<(u32, usize)> = Vec::new();
    for block in &live {
        let before_rows = rows.len();
        for round in block.first_round..=block.last_round {
            if let Some(&range) = ranges.get(round) {
                rows.extend(crate::action_ledger::rows_for_round(messages, range, round));
            }
        }
        per_block_rows.push((block.number, rows.len().saturating_sub(before_rows)));
    }
    let mut ledger_write_failed = false;
    let ledger_seq = match params.ledger_path.as_deref() {
        Some(path) => match crate::action_ledger::append_ledger_rows_range(path, &rows) {
            Ok(seq) => seq,
            Err(err) => {
                ledger_write_failed = true;
                tracing::warn!(
                    ledger_path = %path.display(),
                    error = %err,
                    "v8 truncation: external ledger append failed — truncation continues"
                );
                None
            }
        },
        None => None,
    };
    let ledger_locators: Vec<(u32, (u64, u64))> = match ledger_seq {
        Some((first, _)) => {
            let mut cursor = first;
            per_block_rows
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|(number, count)| {
                    let from = cursor;
                    let to = cursor + *count as u64 - 1;
                    cursor = to + 1;
                    (*number, (from, to))
                })
                .collect()
        }
        None => Vec::new(),
    };
    // ①b 回放面：逐字原文落盘（形态①，零新工具；失败如实落账，不再静默）。
    let mut replay = Vec::new();
    let mut archive_write_failed = false;
    for block in &live {
        let path =
            crate::model_face::block_archive_path(&host.session_cwd(), archive_tag, block.number);
        let markdown = crate::model_face::block_archive_markdown(
            block,
            messages,
            params.run_id.as_str(),
            svc.session_id,
        );
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        if !crate::summary::write_archive_retry(dir.as_path(), &path, &markdown) {
            archive_write_failed = true;
        }
        replay.push(crate::model_face::replay_line(
            block,
            &crate::model_face::relative_block_archive(archive_tag, block.number),
        ));
    }
    let numbers: Vec<u32> = live.iter().map(|b| b.number).collect();
    let first_round = live.first().map(|b| b.first_round).unwrap_or(0);
    let last_round = live.last().map(|b| b.last_round).unwrap_or(0);
    let freed_tokens: u64 = live.iter().map(|b| b.estimate_tokens).sum();
    // ② 会话内**追加**截断 marker（restore-retained ⇒ 恢复后块状态仍为已截断）。
    let id = format!("compaction-{}-{:04}", writer.run_id(), writer.seq());
    let archive_path = archive_dir.join(format!("{id}.md"));
    let locators = crate::summary::LocatorPointers {
        ledger_seq,
        ledger_path: params.ledger_path.as_ref().map(|p| p.display().to_string()),
        journal_run: Some(writer.run_id().to_string()),
        journal_seq: Some((0, writer.seq())),
        conversation_path: None,
        // v8：截断不 drain 本地面 ⇒ sidecar 仍含逐字原文（审查 R-4）。
        local_face_full: true,
    };
    let replay_text = replay.join("\n");
    let markdown = format!(
        "# ORZ 上下文分块截断记录（T1 {id}）\n\n\
         - 处理分块: {numbers}\n- 被截断: {count} 块 ≈{freed}tk token\n\
         - 被处理轮次: 轮 {from}–{to}\n- 触发: {trigger}\n\
         - 黑板会话: {session}\n- 记录存档: {path}\n\
         - 回放档案写入失败: {failed}\n- 台账写入失败: {ledger_failed}\n\
         - 回放档案:\n{replay_text}\n\n\
         ## 模型面声明\n{declaration}\n",
        numbers = crate::model_face::render_block_numbers(&numbers),
        count = live.len(),
        freed = freed_tokens,
        from = first_round + 1,
        to = last_round + 1,
        trigger = if guard_hit {
            "上限守卫（异常保险：单轮暴涨/换算漂移）"
        } else {
            "T1 硬截断（阶梯 500K 估算 ≈385K 真实；按越线重新武装）"
        },
        session = svc.session_id.unwrap_or("（无）"),
        path = archive_path.display(),
        failed = archive_write_failed,
        ledger_failed = ledger_write_failed,
        declaration = crate::model_face::MODEL_FACE_DECLARATION,
    );
    if !crate::summary::write_archive_retry(&archive_dir, &archive_path, &markdown) {
        archive_write_failed = true;
    }
    // 0bh ③＋⑮：截断回执同样带定位符行（与压缩回执同形态）。
    let pointer_line = crate::model_face::render_pointer_line(
        &live.iter().map(|b| (b.number, b.first_round)).collect::<Vec<_>>(),
        &ledger_locators,
        locators.journal_seq.map(|(from, _)| from),
    );
    let marker = crate::model_face::truncation_marker(
        &numbers,
        first_round,
        last_round,
        freed_tokens,
        &archive_path,
        &locators,
        &replay_text,
        &ledger_locators,
        &pointer_line,
    );
    let insert_at = ranges
        .get(first_round)
        .map(|&(start, _)| start)
        .unwrap_or(messages.len());
    messages.insert(
        insert_at,
        Message {
            role: Role::User,
            content: marker,
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        },
    );
    // ③ 二级消化（设计 §6/§12）：截断后**仍在线之上**时，先裁回放块（可再生），
    // 再把其余超大工具结果换成「头部 ＋ 落盘指针」。
    let mut replay_reclaimed = crate::action_ledger::PointerizeStats::default();
    let mut pointerized = crate::action_ledger::PointerizeStats::default();
    if crate::model_face::estimate_model_face_tokens(messages, params) > target_line {
        replay_reclaimed = crate::action_ledger::pointerize_replay_tool_results(
            messages,
            target_line,
            &host.session_cwd(),
        );
        if crate::model_face::estimate_model_face_tokens(messages, params) > target_line {
            pointerized = crate::action_ledger::pointerize_oversized_tool_results(
                messages,
                target_line,
                crate::action_ledger::OVERSIZED_TOOL_RESULT_CAP_TOKENS,
                &pointerized_dir,
                archive_tag,
                Some(host.journal().events_path().as_path()),
                writer.run_id(),
            );
        }
    }
    let extra_freed = replay_reclaimed
        .freed_tokens
        .saturating_add(pointerized.freed_tokens);
    let escalation = if replay_reclaimed.replaced == 0 && pointerized.replaced == 0 {
        String::new()
    } else {
        format!(
            "\n补充（截断后仍越线）：已按「先裁回放块（可再生）」移出 {replay} 条回放结果，\
             并把 {pointerized} 个超大工具结果换成指针——两者逐字原文均已落盘，可按指针重读。",
            replay = replay_reclaimed.replaced,
            pointerized = pointerized.replaced,
        )
    };
    // ④ 告知块（注入文本；截断事实 ＋ 回放口径 ＋ 任务无需中止）。
    let table = crate::model_face::render_block_table(
        &crate::model_face::blocks_outside_slider(
            messages,
            params.slider_tokens,
            params.block_tokens,
        ),
        &crate::model_face::face_markers(messages),
        params,
    );
    let notice = if guard_hit {
        format!(
            "{}{escalation}",
            crate::context_scale::guard_truncation_notice_block(
                live.len(),
                freed_tokens,
                crate::model_face::estimate_model_face_tokens(messages, params),
                svc.context_compact.model_face_guard_tokens,
                &replay_text,
                archive_write_failed || ledger_write_failed,
            )
        )
    } else {
        format!(
            "{}{escalation}",
            crate::context_scale::truncation_notice_block(
                live.len(),
                freed_tokens,
                crate::model_face::estimate_model_face_tokens(messages, params),
                &replay_text,
                &table,
                archive_write_failed || ledger_write_failed,
            )
        )
    };
    let watermark = blackboard_watermark_label(svc);
    messages.push(Message {
        role: Role::User,
        content: format!("{notice}\n{watermark}"),
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
        round: None,
    });
    // ⑤ 事实落账（设计 §5：`anomaly=hard_context_truncated_blocks`）。
    let payload = audit.record(
        "context_scale:hard_truncate",
        tool_rounds,
        format!(
            "truncated_blocks={} freed_tokens={} rounds={}-{} guard_hit={guard_hit} \
             replay_reclaimed={} pointerized_results={} freed_tokens_extra={extra_freed} \
             archive_write_failed={archive_write_failed} ledger_write_failed={ledger_write_failed} \
             model_face_estimate_after={}",
            live.len(),
            freed_tokens.saturating_add(extra_freed),
            first_round + 1,
            last_round + 1,
            replay_reclaimed.replaced,
            pointerized.replaced,
            crate::model_face::estimate_model_face_tokens(messages, params),
        ),
        Some(if guard_hit {
            "hard_context_guard_truncated_blocks".to_string()
        } else {
            "hard_context_truncated_blocks".to_string()
        }),
    );
    writer
        .record(
            EventType::MechanicalAuditUpdate,
            serde_json::json!({
                "kind": crate::mechanical_audit::KIND_CONTEXT_SCALE,
                "payload": payload,
            }),
        )
        .await?;
    Ok(Some((live.len(), freed_tokens.saturating_add(extra_freed))))
}

/// 语义压缩的事件归因（`context_compressed.reason`）：这次摘要由**哪个窗口**
/// 促成。v8 语义轨＝模型在回复文本里产出 `[SEMANTIC_SUMMARY]` 块，故归因在
/// **识别点随摘要一起固定**（写进 `pending_semantic`），不在落地时回看旗标：
/// 后者会把「工具窗口收口未产出摘要（旗标残留）之后由 H1 窗口产出的压缩」
/// 误记为 `model_selected`，也会被「落地前恰有 H1 开窗」抢走归因（0ap 复核
/// 批 P2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompressionWindowKind {
    /// H1 阶梯硬提醒打断式窗口 ⇒ `context_scale_window`。
    Ladder,
    /// 0ap `context_compress` 知情发起窗口 ⇒ `model_selected`（设计 §1）。
    ModelRequested,
}

impl CompressionWindowKind {
    fn semantic_reason(self) -> &'static str {
        match self {
            Self::Ladder => "context_scale_window",
            Self::ModelRequested => "model_selected",
        }
    }
}

/// 归因终值：有窗口 ⇒ 按窗口种类；无窗口（模型自选摘要块，不经任何窗口）
/// ＝ `model_selected`（既有口径不变）。
fn semantic_compression_reason(kind: Option<CompressionWindowKind>) -> &'static str {
    kind.map_or("model_selected", CompressionWindowKind::semantic_reason)
}

/// The shared model↔tool loop (M1 extraction, 2026-08-10).
///
/// Semantics preserved verbatim from `run_turn_inner`'s loop body:
/// pacing → compaction → orientation loop-top gap → system assembly →
/// model round → model_output journal → orientation feed → budget
/// exhaustion / final answer / counterexample gate / IPG → tool dispatch
/// (role-gated) → batch injections (denial breaker / edit push /
/// orientation post-tool-batch gap) → budget exhaustion check (the live
/// remaining count is read on demand via `blackboard_read section=session`,
/// PUSH→PULL 2026-08-21).
///
/// `tool_defs` is the BASE list for main/grill turns (registry + main-only
/// additions + mode projection) — the loop re-probes the work tools before
/// EVERY model request and re-projects it (探针完整集 ∩ 会话声明集 + 非工作
/// 工具), emitting `tool_availability_check` only on flips against
/// the minimal previous-round map (P0-A step 5 / P0-A-2). Retrieval lanes pass
/// their FINAL list and never re-probe (no second
/// `tool_availability_check` event inside a lane). The call-time permission
/// gate remains the final backstop. `messages` is in/out: the conversation
/// continues across rounds; the caller owns the seed and the post-loop use
/// (grill history writeback).
#[allow(clippy::too_many_arguments)] // the shared loop's full contract
pub(crate) async fn run_agent_loop(
    svc: &SharedLoopServices<'_>,
    controller: &AgentLoopController,
    writer: &mut EventWriter<'_>,
    host: &dyn LoopHost,
    agent: &dyn RoundAgent,
    profile: &LoopProfile,
    prompt: &str,
    tool_defs: &[ToolDef],
    messages: &mut Vec<Message>,
    mut orientation: Option<&mut OrientationSessionState>,
    cancel: Option<&tokio_util::sync::CancellationToken>,
    heartbeat: Option<&ActivityClock>,
    // STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37 / 设计
    // §2.2)：本 run 的 per-run 隔离 gateway——主 agent 与检索子代理
    // 共享同一 run 实例（run 内退化计数/档位一致）；子代理派发时传给
    // `run_retrieval_subagent`。
    run_gateway: &Arc<dyn ModelGateway>,
) -> Result<LoopOutcome, AgentLoopError> {
    let workspace_trust = host.workspace_trust();
    // The session's budget counter (user adjudication 2026-08-10, review
    // F5): a `continue` re-entry is the same retrieval session — the
    // counter starts from the activation's consumed rounds and resets only
    // with a NEW activation (Closed → next creation starts at 0). The main
    // agent keeps the inherited per-run semantic (`initial_tool_rounds` 0).
    let mut tool_rounds = profile.initial_tool_rounds;
    // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): run 级墙钟
    // 起点——审计报告的预算（轮数/墙钟用量）与运行内审查表（run 结束即弃）。
    let run_started_at = std::time::Instant::now();
    let mut mechanical_audit = crate::mechanical_audit::MechanicalAuditState::new();
    // The `None` seed is required by Rust's initialization rules (the
    // value is overwritten on every break path before the read at the
    // end — clippy's unused_assignments is a false positive here).
    #[allow(unused_assignments)]
    let mut last_text: Option<String> = None;
    // D-8: after the budget is exhausted the model gets ONE final
    // no-tools round to report a partial result; if it still requests
    // tools, the run ends there (no execution of post-budget calls).
    let mut budget_exhausted = false;
    // 0ar S2-D1（2026-09-19，检索批次回送设计 §3.4 定案 β）：检索车道
    // 阈值/护栏收尾状态——`batch_close` 记成因（进 `LoopOutcome` 供
    // dispatch 映射 terminal_reason），`close_round_armed` 置位后下一轮
    // 工具面收空、且该轮响应不再派发（唯一收尾回合）。主车道恒不置位。
    let mut batch_close: Option<crate::retrieval::batch_close::BatchCloseKind> = None;
    let mut close_round_armed = false;
    // 0ar S2-D3（§5.5）：同轮溢出检索调用的未派发登记——本轮 post-batch
    // 间隙注入一次性重述（每轮至多一条），防模型漏看丢覆盖。
    let mut deferred_retrievals: Vec<String> = Vec::new();
    // 0au（2026-09-20 立项，S3 摩擦 N2）：因 run 墙钟余量不足而保留的检索
    // 登记——post-batch 间隙一次性重述（cause 自描述），只报事实不邀请
    // 重派（保留窗内重派会被同一判定再拦）。
    let mut reserved_retrievals: Vec<String> = Vec::new();
    // 0ar S3 前去噪（设计 §5.6）：检索车道（子代理）在本激活内的 query
    // 去重键 → 首次 call_id；重复 query 走指针回踩（不重复检索）。只对
    // 检索车道跨轮持久；主车道改用每轮局部表（同轮重复去重、跨激活重派
    // 仍按「每次派发新激活」语义执行）。
    let mut lane_dispatched_queries: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // §4.6 wiring state: the final-answer counterexample gate fires once
    // per run. The old mixed inquiry counters are gone (GAP-INQUIRY-SPLIT
    // 2026-08-09) — orientation counts live in the session-level
    // `OrientationSessionState` threaded through the turn chain; output
    // repetition is handled by the generation-time output-health guard.
    let mut counterexample_fired = false;
    // ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16): a
    // checkpoint fire whose block was injected but whose forced-template
    // round has not completed yet (main lane only — retrieval lanes commit
    // at fire time and never set this). While pending, the loop skips
    // compaction and further fires, offers NO tools, and runs the template
    // round at the next loop-top.
    let mut pending_checkpoint: Option<PendingCheckpoint> = None;
    // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 首轮计划轮硬门 —
    // 主车道且黑板尚无已批准计划（新会话/新 plan epoch）时触发；已有计划
    // （恢复会话或本会话后续 run）不重复触发；检索车道不触发。计划轮计入
    // 已完成逻辑模型轮（feed_round 照常），通过后计划落黑板 plan epoch。
    let mut plan_gate: Option<crate::planning::PlanGateState> = None;
    if controller.plan_first_enabled()
        && !controller.plan_first_session_done()
        && profile.role == AgentRole::Main
        && svc.blackboard.read().plan.plan_id.is_none()
    {
        plan_gate = Some(crate::planning::PlanGateState::start());
    }
    // P0-D (2026-08-14, ADR-0010 v1.10 / v1.14): template-summary state —
    // the rounds since the last compaction（事件读数 + `LoopOutcome` 出口）。
    // **v8（2026-09-16 勘误批）：provider 实测 prompt 量尺随勘误退役**——
    // rhythm／fallback 两档「视图刻度」触发先后被 v7-S1 与 v8 取消，模型面
    // 阶梯改吃投影层估算，故上一轮实测 token 不再参与任何判定（字段一并移除，
    // 避免留一个「看着还在量、实际没人读」的死量尺）。
    let mut rounds_since_compact: u32 = 0;
    // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the previous
    // model-request header fingerprint of THIS loop invocation — `None`
    // until the first request, so the first request journals `initial` and
    // only real prefix changes journal `change`.
    let mut last_request_header: Option<RequestHeader> = None;
    // 0ac S3①-b M2（2026-09-15，IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_
    // RETRIEVAL_DESIGN §2.1/§4.1「合法边界投递」）：per-run 投递队列——
    // run 生命周期 = 队列生命周期（不跨 run 存活）；B1/B2 边界投递与
    // M1 收尾注入共用。主开关关时队列恒空（admit 只在开关下被调用），
    // 零事件、零行为变化。
    let mut delivery_queue = crate::immediate_delivery::DeliveryQueue::from_env();
    let mut m1_used = false;
    // 0ac S3①-b ⑥（2026-09-15，同稿 §7 风险 6）：检索子代理提前收口——
    // 确定性不可达（`capability_unreachable`）立即收口；连续确定失联
    // （network_no_response，成功即清零；`empty_result` 是通道判活的
    // 合法 I1 信息，不计入——0AC-A2）达到阈值收口；墙钟只作最后兜底。
    // 仅检索车道启用（Main 车道检索失败照常回传，由模型自行决策）。
    let mut retrieval_early_close: Option<String> = None;
    let mut retrieval_failure_streak: u32 = 0;
    // 滑块上下文 v8（2026-09-16 勘误批，设计 §3）：**注意力阶梯**（192/224/
    // 256/288K 软提醒 → 320K 硬提醒 → 500K 硬截断），量尺＝**模型面估算**。
    // 水位**会话级**——已提醒键由会话侧车经 controller 注入 loop、fire 时回写
    // （与黑板同族：跨 prompt 延续、新会话独立、恢复不重发）；每档一次、
    // 不 rearm。仅主车道启用（检索车道不提醒——同 0ae 车道范围裁决）。
    let mut context_scale = crate::context_scale::ContextScaleState::from_notified_keys(
        &controller.context_scale_notified_keys(),
    );
    // v8 压缩分工（设计 §4）：**语义轨**——模型在回复里产出的语义摘要块
    // （机械可识别前缀，`response.text` 上取）在下一个 loop-top 安全间隙执行
    // 一次**按块压缩**（可带 `压缩块: 1-4` 区间指令）。工具轮的文本不进
    // `messages`（只留 model_output 事件），故识别点在**响应处理**而非会话扫描。
    // 0ap 复核批（P2）：归因**随摘要一起记录**——`(摘要文本, 促成它的窗口)`。
    // 落地时只用记录值，不再回看「本 epoch 开过哪个窗口」的旗标：旗标残留会
    // 让 H1 窗口产出的压缩被记成 `model_selected`（工具窗口未产出摘要即收口
    // 的场景），落地前恰有 H1 开窗也会抢走已产出摘要的归因。
    let mut pending_semantic: Option<(String, Option<CompressionWindowKind>)> = None;
    // 在程压缩窗口的种类（`None` ＝ 无窗口在程）。开窗时置位、窗口不在程时
    // 清空；识别语义摘要那一刻的值随摘要进 `pending_semantic`（归因固定点）。
    let mut window_kind: Option<CompressionWindowKind> = None;
    // loop 迭代计数（每迭代至多一个模型轮）。
    let mut loop_rounds: u32 = 0;
    // v8 原文定位指针（设计 §4 §6）：journal 窗口 epoch 起点 / 台账 `[seq]`
    // 区间（本 epoch 已压缩块写入的行）/ conversation sidecar 路径。
    let mut journal_epoch_start_seq: u64 = writer.seq();
    let mut ledger_seq_epoch: Option<(u64, u64)> = None;
    let conversation_sidecar_path = conversation_sidecar_hint(host, svc.session_id);
    // v8 模型面投影的固定入参（不随轮次变化）：外挂台账路径 ＋ 分块档案标签。
    let face_ledger_path = (profile.role == AgentRole::Main)
        .then(|| crate::action_ledger::ledger_file_path(&host.session_cwd()));
    let face_archive_tag = crate::model_face::archive_tag(svc.session_id, writer.run_id());
    // A4（v8 触发改锚「首个分块形成」）：一次性固化提醒——2026-09-16 实现批
    // 由 **per-run 变量**改为**会话级水位**（键 `first_block`，随侧车持久化；
    // 「一次性」指的是一次会话，不是一次 run）。
    //
    // v8（2026-09-16 实现批，审查 R-9）：阶梯/守卫的量尺＝模型面估算 ＋
    // **静态开销**（系统提示词 ＋ 工具定义；取**上一轮请求**的实测值，首轮为 0）。
    // 不含静态开销的估算会系统性偏小、把阶梯落点整体推后。
    let mut face_static_overhead: u64 = 0;
    // v8 D4 机械段（基线 ＋ 自编辑清单 ＋ 编辑指纹）：按 epoch 冻结，压缩/
    // 截断后重渲一次（前缀字节稳定，不变量 I6）。
    let mut face_d4_block: Option<String> = None;
    // 0AE-C1 修复（2026-09-15 深审）：压缩窗口轮携带 blackboard_write 落穿
    // 正常派发路径时的**延迟收口**状态——(窗口开始写入面计数, 窗口已用
    // 轮数)。写入在消费分支落穿后派发完成，下一 loop-top 读数真实，据实
    // 落账 `model_compression` 审计事件（participated 由此可达）。
    let mut model_compression_close: Option<(usize, u32)> = None;
    // 0ae D1（2026-09-15，设计 §4，用户裁决 DP-2）：补救规则——至第
    // N=20 轮仍无任何 blackboard_write ⇒ 再提醒一次；此后不再提醒、
    // 不设硬门。
    const PLAN_WRITE_REMINDER_ROUND: u32 = 20;
    let mut plan_reminder_done = false;
    // 0ae D4（2026-09-15，设计 §7，用户裁决 DP-4）：run 起始基线
    // （HEAD + worktree 干净与否；git 失败/非 git 工作区 = None）。
    let run_baseline = if profile.role == AgentRole::Main {
        crate::action_ledger::capture_run_baseline(&host.session_cwd())
    } else {
        None
    };

    loop {
        loop_rounds = loop_rounds.saturating_add(1);
        // Cooperative cancellation checkpoint (Phase 3 slice #7): polled
        // BEFORE the pacing sleep so a cancel never waits on
        // TEXT_DELTA_PACING. Returning here terminates the run with a
        // `run_cancelled` journal event (recorded by `run_turn`).
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(AgentLoopError::Cancelled);
        }

        // Streaming ordering guard (Phase 3 slice #6): this round's text
        // deltas reach a live client at arrival rate, but the previous
        // round's `model_output` lands via the TUI journal tail — the
        // tail thread polls the file every 50ms AND the runner drains
        // its channel on a separate 50ms tick, so worst-case projection
        // is ~100ms after the fsync ack; TEXT_DELTA_PACING (2 ticks)
        // covers that alignment. Starting a new round's deltas before
        // that journal event is projected would append them to the
        // previous round's card (the dedup only clears
        // `current_model_index` on a matching model_output). The counter
        // lives on the controller (not per-turn) so a turn ≥ 2's FIRST
        // round is paced too — a programmatic client may chain prompts
        // faster than the tail projects the previous turn's terminal
        // events (review P3-5); user-paced TUIs are naturally safe.
        // Residual boundary (2026-08-05 review): a render stall beyond
        // ~10ms could still race — accepted. Note the sleep applies per
        // extra round even headless (deltas go nowhere): ~120ms × rounds
        // is the recorded cost of keeping the guard universal.
        if svc
            .pacing_rounds
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            > 0
        {
            tokio::time::sleep(TEXT_DELTA_PACING).await;
        }

        // P0-D (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §2-§4):
        // template-summary trigger at any safe loop-top gap (never inside a
        // batch). Measured = the previous round's provider prompt tokens on
        // the FOLDED request view. Two triggers:
        //   - RHYTHM: measured > trigger_tokens (192K, 2026-08-18
        //     adjudication ADR-0010 §14.26) with a ≥ min_rounds
        //     (2) model-round cooldown (v1.14 review fix).
        //   - FALLBACK (window guard): measured > safety_tokens (256K,
        //     2026-08-18 adjudication ADR-0010 §14.26), bypassing the
        //     cooldown — the emergency path must not stay over the line
        //     even when the summary fails (mechanical truncation
        //     fallback, D2-2 semantics).
        // Reduction guards: removable content ≥ min_compactable and kept ≤
        // max_reduction_ratio of before. P0-D review fix (2026-08-14): a
        // guard that cannot be satisfied is NOT skipped — it retries on the
        // next trigger rounds (no content interruption, no raw truncation)
        // and after GUARD_RETRY_LIMIT consecutive failures forces one
        // compaction and reports `guard_failed` for explicit handling.
        // A pending checkpoint round has priority over compaction — the
        // injected template block must reach the model before any window
        // collapse (§14.16: 触发点下一安全动作间隙暂停).
        // 滑块上下文 v8（2026-09-16 勘误批，设计 §1 §2 §3 §5）：loop-top 安全
        // 间隙（绝不在工具批内；pending checkpoint 优先）做四件事——
        // ① 装配**模型面投影**（前置 ＋ 固定指针 ＋ 分块表 ＋ D4 机械段 ＋
        //    各分块 ＋ 主滑块）并取**模型面估算**当阶梯量尺（设计 §3.1：不是
        //    本地全量、不是 provider 实测——v7 量错对象正是本次勘误的实质处）；
        // ② 阶梯（每档每会话一次）：192/224/256/288K 软提醒 → **320K 硬提醒**
        //    （打断 ＋ 开压缩窗口）→ **500K 硬截断**（把主滑块以外的全部分块
        //    移出模型面，本地面不动）；
        // ③ 首个分块形成时一次性固化提醒（A4 文案语义保留、触发改锚）；
        // ④ **1.10M 估算异常保险**：越线＝强制截断到线上（v7 的「不开窗降级」
        //    随勘误作废）。
        // **减少模型面的动作只有两个**：模型自压（`[SEMANTIC_SUMMARY]` ⇒ 按块
        // 压缩）与 T1 硬截断——机械层不再以「总量超线」为由压缩或截断模型面。
        let face_slider_tokens = svc.context_compact.slider_window_tokens;
        let face_block_tokens = svc.context_compact.model_face_block_tokens;
        let face_blocks = crate::model_face::blocks_outside_slider(
            messages,
            face_slider_tokens,
            face_block_tokens,
        );
        // D4 机械段（run 基线 ＋ 自编辑清单 ＋ 最近编辑指纹）按 **epoch 冻结**：
        // 下一次压缩/截断后重渲一次，两次之间模型面前缀字节稳定（不变量 I6）。
        // 0AE-C5（2026-09-15 深审修复）：无条件渲染——baseline=None（非 git
        // 工作区/git 失败）只省略基线段，自编辑清单与编辑指纹两段照常。
        // 0AE-C11 处置（2026-09-17，盘点 FR-C04 采②）：传入当前 run id，
        // 清单按 run 章分「本 run / 会话历史」两组。
        if !face_blocks.is_empty() && face_d4_block.is_none() {
            face_d4_block = Some(crate::action_ledger::render_run_context_block(
                run_baseline.as_deref(),
                &svc.blackboard.read().edits,
                Some(writer.run_id()),
            ));
        }
        let face_params = crate::model_face::ModelFaceParams {
            slider_tokens: face_slider_tokens,
            block_tokens: face_block_tokens,
            ledger_path: face_ledger_path.clone(),
            archive_tag: Some(face_archive_tag.clone()),
            run_id: writer.run_id().to_string(),
            d4_block: face_d4_block.clone(),
            // 静态开销＝**上一轮请求**的实测值（首轮 0，保守侧偏差）。
            static_overhead_tokens: face_static_overhead,
        };
        let mut model_face_tokens =
            crate::model_face::estimate_model_face_tokens(messages, &face_params);
        // A4（2026-09-15，设计 §3.2，用户裁定 R3；v8 触发改锚「首个分块形成」）：
        // **首次出现主滑块之外的分块**后发射一次固化提醒（**每会话**恰一次）。
        if profile.role == AgentRole::Main
            && !face_blocks.is_empty()
            && !context_scale.has_flag(crate::context_scale::FLAG_FIRST_BLOCK)
        {
            context_scale.mark_flag(crate::context_scale::FLAG_FIRST_BLOCK);
            controller.mark_context_scale_notified(crate::context_scale::FLAG_FIRST_BLOCK);
            let watermark = controller.blackboard_watermark_label();
            messages.push(Message {
                role: Role::User,
                content: format!(
                    "{}\n{watermark}",
                    crate::context_scale::first_block_reminder_block()
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            let payload = mechanical_audit.record(
                "context_scale:first_block",
                tool_rounds,
                format!(
                    "form=standalone_block model_face_estimate_tokens={model_face_tokens} \
                     blocks={} watermark={watermark}",
                    face_blocks.len()
                ),
                None,
            );
            writer
                .record(
                    EventType::MechanicalAuditUpdate,
                    serde_json::json!({
                        "kind": crate::mechanical_audit::KIND_CONTEXT_SCALE,
                        "payload": payload,
                    }),
                )
                .await?;
        }
        // 阶梯（量尺＝**模型面估算**；每档每会话一次、不 rearm）。
        let mut truncate_now = false;
        if pending_checkpoint.is_none() && profile.role == AgentRole::Main {
            let ladder = svc.context_compact.context_scale_ladder;
            let truncate_tokens = crate::model_face::ladder_truncate_tokens(&ladder);
            let fires = context_scale.due(model_face_tokens, &ladder);
            let has_truncate = fires
                .iter()
                .any(|f| f.tier == crate::context_scale::LadderTier::HardTruncate);
            // **一轮内只注入最高档**（只读审查 R-12③ 处置，2026-09-16）：单轮
            // 暴涨会让 `due()` 一次返回 4–5 档；逐档注入等于把同一段文案（只有
            // 刻度字样不同）连发数遍——最高档的文案已含当前读数、分块表、压缩
            // 方法与下一档承诺，低档无独立信息。**水位与事件照记**（journal 保留
            // 「本轮越过了哪几档」的完整事实，payload 以 `form=` 注明是否真的注入）。
            // T1 本轮生效时同理只发**截断告知块**（内容即将被截断，软／硬提醒都
            // 已无意义）；告知块由 `truncate_model_face_blocks` 在截断**之后**渲染，
            // 并如实携带截断后的当前读数（故压掉低档提醒是无损的）。
            let highest_tokens = fires.iter().map(|f| f.milestone_tokens).max();
            let watermark = controller.blackboard_watermark_label();
            for fire in &fires {
                let injected = !has_truncate && Some(fire.milestone_tokens) == highest_tokens;
                if injected {
                    let block = match fire.tier {
                        crate::context_scale::LadderTier::Soft => {
                            Some(crate::context_scale::soft_reminder_block(
                                fire.milestone_tokens,
                                model_face_tokens,
                            ))
                        }
                        crate::context_scale::LadderTier::HardReminder => {
                            let table = crate::model_face::render_block_table(
                                &face_blocks,
                                &crate::model_face::face_markers(messages),
                                &face_params,
                            );
                            Some(format!(
                                "{}\n{}",
                                crate::context_scale::hard_reminder_block(
                                    fire.milestone_tokens,
                                    truncate_tokens,
                                    model_face_tokens,
                                    &table,
                                ),
                                crate::context_scale::compression_window_block(
                                    fire.milestone_tokens
                                ),
                            ))
                        }
                        // 硬截断的告知块在截断**之后**渲染（需要截断事实读数）。
                        crate::context_scale::LadderTier::HardTruncate => None,
                    };
                    if let Some(block) = block {
                        messages.push(Message {
                            role: Role::User,
                            content: format!("{block}\n{watermark}"),
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                            round: None,
                        });
                        if fire.tier == crate::context_scale::LadderTier::HardReminder {
                            // H1 硬提醒＝**打断式提醒**（FR-3，0bc 长杂轮：
                            // 不锁工具面——窗口轮工具面＝常规面、动作照常
                            // 派发）：≤3 轮压缩窗口；窗口结束仍未产出摘要 ⇒
                            // 如实落账 `model_participated=false`，**不再机械
                            // 兜底压缩**。
                            pending_checkpoint = Some(PendingCheckpoint::ModelCompression {
                                rounds_left: crate::context_scale::COMPRESSION_WINDOW_ROUNDS,
                                window_start_writes: svc.blackboard.read().model_note_count(),
                            });
                            window_kind = Some(CompressionWindowKind::Ladder);
                        }
                    }
                }
                // 会话级水位：fire 即回写（随侧车持久化；新会话独立、恢复不重发）。
                controller.mark_context_scale_notified(&fire.key);
                let form = if injected {
                    "standalone_block"
                } else if has_truncate {
                    "deferred_to_truncation_notice"
                } else {
                    "suppressed_superseded_by_higher_tier"
                };
                let payload = mechanical_audit.record(
                    format!("context_scale:{}", fire.key),
                    tool_rounds,
                    format!(
                        "milestone_tokens={} model_face_estimate_tokens={} form={form} \
                         tier={} truncate_tokens={} blocks={} watermark={watermark}",
                        fire.milestone_tokens,
                        model_face_tokens,
                        match fire.tier {
                            crate::context_scale::LadderTier::Soft => "soft",
                            crate::context_scale::LadderTier::HardReminder => "hard_reminder",
                            crate::context_scale::LadderTier::HardTruncate => "hard_truncate",
                        },
                        truncate_tokens,
                        face_blocks.len(),
                    ),
                    None,
                );
                writer
                    .record(
                        EventType::MechanicalAuditUpdate,
                        serde_json::json!({
                            "kind": crate::mechanical_audit::KIND_CONTEXT_SCALE,
                            "payload": payload,
                        }),
                    )
                    .await?;
            }
            truncate_now = has_truncate;
        }
        // T1 硬截断 ＋ 1.10M 异常保险：把**主滑块以外的全部分块**移出模型面
        // （设计 §5）：本地面逐字不动、可按块回放、任务无需中止。T1 一次/会话
        // （会话级水位）；守卫线可重复触发（截断后新累积的分块才会再次越线）。
        model_face_tokens = crate::model_face::estimate_model_face_tokens(messages, &face_params);
        let guard_hit = model_face_tokens >= svc.context_compact.model_face_guard_tokens;
        if pending_checkpoint.is_none()
            && profile.role == AgentRole::Main
            && (truncate_now || guard_hit)
        {
            let cut_outcome = truncate_model_face_blocks(
                svc,
                writer,
                host,
                messages,
                &face_params,
                &face_archive_tag,
                guard_hit && !truncate_now,
                &mut mechanical_audit,
                tool_rounds,
            )
            .await?;
            if cut_outcome.is_none() && truncate_now {
                // 越线为真但本轮**无可执行动作**（无已闭合分块／无回放块／无超大
                // 结果）⇒ 不消费 T1 闩位：下一轮继续试（内容一旦可动即压回线上）。
                context_scale.rearm(crate::model_face::ladder_truncate_tokens(
                    &svc.context_compact.context_scale_ladder,
                ));
            }
            // 截断后本 epoch 的 D4 机械段与分块表随之重渲（前缀重写＝一次
            // 压缩级事件，设计 §9「重写税」口径）。
            face_d4_block = None;
            journal_epoch_start_seq = writer.seq();
            ledger_seq_epoch = None;
        }
        // 0ae D3 延迟收口（0AE-C1 修复，2026-09-15 深审）：上一迭代的窗口
        // 轮携带 blackboard_write 落穿了正常派发路径——写入此刻已派发完成，
        // 黑板读数真实。按 C2 形状落账 `model_compression` 并置机械折叠
        // 兜底旗标；本迭代稍后的 summary_now 块即执行机械折叠
        // （reason=context_scale_window；S1 前该值写作 `attention_920k_window`
        // ——不在 `context_compressed` reason 闭枚举内，S1 批按契约同步为
        // `context_scale_window`）。收口逻辑统一在
        // `finalize_model_compression_close`（budget 收尾 / IPG block /
        // run 尾三条异常出口同走此函数，窄边沿修复 2026-09-15）。
        finalize_model_compression_close(
            &mut model_compression_close,
            &mut mechanical_audit,
            svc,
            writer,
            tool_rounds,
        )
        .await?;
        // v8 模型自压（设计 §4；取代 v7 的「摘要替换被压区」drain 形态）：
        // 模型在回复里产出**语义摘要块** ⇒ 下一个安全间隙按**块**压缩——
        // 机械层把「机械摘要行（结构化轨）＋语义摘要（模型产出）」写到 marker
        // 里，模型面随之下移；**本地面一条不删**（不变量 I3）。reason：窗口
        // 收口＝`context_scale_window`，模型自选＝`model_selected`。
        if let Some((summary, summary_window)) = pending_semantic.take() {
            // 0ap 复核批（P2）：归因取**摘要产出时**记录的窗口种类——工具
            // 知情发起 ⇒ `model_selected`（设计 §1）；H1 阶梯窗口 ⇒
            // `context_scale_window`；无窗口（模型自选）⇒ `model_selected`。
            let semantic_reason = semantic_compression_reason(summary_window);
            let selection = crate::context_scale::extract_block_selection(&summary);
            let locators = crate::summary::LocatorPointers {
                ledger_seq: ledger_seq_epoch,
                ledger_path: face_ledger_path.as_ref().map(|p| p.display().to_string()),
                journal_run: Some(writer.run_id().to_string()),
                journal_seq: Some((journal_epoch_start_seq, writer.seq())),
                conversation_path: conversation_sidecar_path.clone(),
                // 按块压缩不 drain 本地面（`compress_blocks_now` 内部会再置真）。
                local_face_full: false,
            };
            if let Some(outcome) = compress_blocks_now(
                svc,
                writer,
                host,
                messages,
                &face_params,
                &face_archive_tag,
                selection,
                Some(&summary),
                semantic_reason,
                &locators,
            )
            .await?
            {
                tracing::debug!(
                    blocks = outcome.blocks,
                    rounds = outcome.rounds,
                    freed_tokens = outcome.freed_tokens,
                    reason = semantic_reason,
                    "v8 block compaction executed"
                );
                rounds_since_compact = 0;
                // 压缩已落地 ⇒ 本 epoch 收口：D4 机械段重渲、定位指针跨度与
                // 台账 `[seq]` 游标重启（下一次压缩从新 epoch 起算）。
                face_d4_block = None;
                journal_epoch_start_seq = writer.seq();
                ledger_seq_epoch = None;
            }
        }
        // 0ap（2026-09-18，设计 §1/§4-1）：`context_compress` **知情发起**
        // ——工具执行点只置请求位（幂等防连点；读数不可得即中性返回、不置
        // 位），此处（下一个 loop-top 安全边界）统一消费开窗：D3 既有机制
        // 照旧（`PendingCheckpoint::ModelCompression` ≤3 轮 +
        // `finalize_model_compression_close` 统一出口）。软门/其他 pending
        // 在程时请求位锁存顺延（不丢不绕）；窗口在程中 ⇒ 工具侧已 no-op。
        if profile.role == AgentRole::Main
            && controller.compression_window_requested()
            && pending_checkpoint.is_none()
        {
            controller.take_compression_window_request();
            pending_checkpoint = Some(PendingCheckpoint::ModelCompression {
                rounds_left: crate::context_scale::COMPRESSION_WINDOW_ROUNDS,
                window_start_writes: svc.blackboard.read().model_note_count(),
            });
            window_kind = Some(CompressionWindowKind::ModelRequested);
        }
        // GAP-INQUIRY-SPLIT (2026-08-09) — FALLBACK orientation injection
        // point (loop-top): a post-tool-batch gap exists only on tool
        // rounds. When the threshold round is a FINAL round (no tool
        // batch — the turn breaks right after), the crossing is carried
        // here on the next loop-top: pacing + compaction done, before
        // the system prompt is built. (A deny round DOES have a tool
        // batch — its crossing fires in the post-tool-batch gap like any
        // tool round; review D3-1.) This is also the recovery resume
        // point (a restored session's persisted count crosses here).
        // Fires at most once per loop iteration, so the two injection
        // points never double-fire.
        // One pending checkpoint at a time — while an orientation pending
        // round is outstanding (THIN-HARNESS-REDESIGN-V2 §9.2 软门),
        // no further fire may happen (the count is not committed yet, so
        // the gate is the only thing preventing a double-fire at the next
        // loop-top).
        if pending_checkpoint.is_none()
            && let Some(role) = profile.orientation_role
            && let Some(record) = controller
                // THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29 软门):
                // 主车道 fire 延迟 commit 并经 pending 轮软消费；检索
                // 车道维持 fire-and-continue（controller 内按 role 分派）。
                .maybe_fire_orientation(
                    writer,
                    messages,
                    orientation.as_deref_mut(),
                    role,
                    "loop_top_gap",
                )
                .await?
        {
            pending_checkpoint = Some(PendingCheckpoint::Orientation { record });
        }

        // FUS-TOOL-PROBE P0-A-2 (design §4/§5 v0.2): per-round work-tool
        // refresh before EVERY model request. The minimal previous-round
        // map (seeded by the pre-run_started event in `run_turn_inner`)
        // decides flip-only events: an unchanged partition emits nothing;
        // a flip journals the fresh snapshot and re-projects the visible
        // list (探针完整集 ∩ 会话声明集 + 非工作工具, names only). Retrieval
        // lanes never re-probe — their list is the caller's final
        // projection (no second `tool_availability_check` inside a lane).
        // The call-time permission gate remains the final backstop (design
        // invariant 2).
        // P0-C S3 (2026-08-15): 探针快照提升到循环顶部作用域——同一快照
        // 同时驱动模型可见工具投影与注册板块投影（Profile/Bundle ∩ 探针
        // 完整集），避免两处各探一次导致投影不一致。
        // THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29 S5-1 修复 B) ／ FR-3
        // （2026-09-21 0bc 长杂轮）：orientation 软门触发轮与**压缩窗口轮**
        // 不禁工具——探针与工具栏投影按常规轮处理；仅 console 询问轮保持
        // 无工具暂停（§14.17⑱）。
        let pending_keeps_tools = pending_checkpoint.as_ref().is_some_and(|p| {
            matches!(
                p,
                PendingCheckpoint::Orientation { .. } | PendingCheckpoint::ModelCompression { .. }
            )
        });
        let probe_snapshot: Option<crate::tool_probe::ToolProbeSnapshot> =
            if pending_checkpoint.is_some() && !pending_keeps_tools {
                // console-inquiry round is a tool-free pause — no registry
                // projection and no probe.
                None
            } else if profile.probe_work_tools {
                let probe_context = crate::tool_probe::ProbeContext {
                    cwd: host.session_cwd(),
                    policy: host.tool_policy(),
                    test_runner_present: host.test_runner().is_some(),
                    interactive_user: host.interactive_user(),
                    goal_context_present: controller.goal_context_present(),
                    pending_retrieval_activation: controller.has_live_activation(),
                    terminal_available: host.terminal_available(),
                    lsp_configured: host.lsp_configured(),
                    memory_enabled: host.memory_enabled(),
                    image_backend_configured: host.image_backend_configured(),
                    video_backend_configured: host.video_backend_configured(),
                    mcp_registry_available: host.mcp_registry_available(),
                };
                let snapshot = crate::tool_probe::narrow_to_declared_face(
                    crate::tool_probe::probe_work_tools(&probe_context),
                    tool_defs,
                );
                if controller.probe_flip(&snapshot) {
                    writer
                        .record(
                            EventType::ToolAvailabilityCheck,
                            AgentLoopController::tool_availability_payload(&snapshot),
                        )
                        .await?;
                }
                // PLAN-FIRST 阶段 B (2026-08-16): 记录主车道本轮探针源——
                // 工具栏投影与注册板块（黑板模型栏）共用的单一事实源；
                // 检索车道无操作台，不记录。
                if profile.role == AgentRole::Main {
                    controller.set_console_probe_source(host.tool_policy(), snapshot.clone());
                }
                Some(snapshot)
            } else {
                None
            };
        // 0ap：窗口在程位对齐（工具执行点防抖读取）。值＝本迭代 loop-top
        // 的 pending 快照——工具执行只发生在 loop-top 之间的派发段，故该
        // 值在整个派发段内恒真（H1 窗口轮与工具发起窗口轮同语义）。
        let window_in_progress = matches!(
            pending_checkpoint.as_ref(),
            Some(PendingCheckpoint::ModelCompression { .. })
        );
        controller.set_compression_window_active(window_in_progress);
        // 0ap 复核批（P2）：窗口不在程 ⇒ 种类作废（开窗点重新置位）。语义
        // 摘要的归因已在识别点随摘要固定，本处清空不回改已记录的归因。
        if !window_in_progress {
            window_kind = None;
        }
        let current_tool_defs: Vec<ToolDef> = if close_round_armed {
            // 0ar S2-D1（β 收尾回合，设计 §3.4 定案）：工具面机械收空——
            // 收尾回合不再烧检索（机械保证），模型只产出结果总结。
            Vec::new()
        } else if pending_checkpoint.is_some() && !pending_keeps_tools {
            // §14.16: DC / console-inquiry checkpoint rounds expose no
            // tools. The orientation soft gate (§9.2) keeps the normal
            // projection — the model may answer and continue, or call
            // tools directly on the trigger round.
            Vec::new()
        } else if plan_gate.is_some() {
            // 首轮计划轮面：只暴露黑板读取 + plan_write（设计 §2.3/§3）。
            tool_defs
                .iter()
                .filter(|t| {
                    t.name == crate::planning::PLAN_WRITE_TOOL
                        || t.name == crate::planning::BLACKBOARD_READ_TOOL
                })
                .cloned()
                .collect()
        } else if let Some(snapshot) = probe_snapshot.as_ref() {
            let projected = AgentLoopController::project_main_agent_tool_defs(tool_defs, snapshot);
            // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / 设计
            // §2.1)：第 2 轮起 direct 执行面——模型直接调用工作工具（一次
            // 调用一个往返）；console 订单面退役，不再收敛到只读+写单面。
            // 半助理层背板（run_terminal_cmd/search_replace/run_tests/检索
            // 派发）与 ACAF/权限/预算/候选计数硬门由调用面照常执行。
            projected
        } else {
            tool_defs.to_vec()
        };
        // P0-C orz 内嵌集成 S2 (2026-08-15): 注册板块每轮机械刷新（主车道；
        // 检索车道无操作台）。内容 = 动作名 + 最小参数提示（最小提示由
        // `console::ServiceRegistry` 生成，不复制完整 schema）；板块常驻、
        // 内容按需读（模型用 blackboard_read section=actions 取回）。
        // PLAN-FIRST 阶段 B (2026-08-16): 刷新收敛为「记录探针源 → 派生
        // 注册板块」单一路径——与模型可见工具投影共用同一探针源（工具栏
        // 绑定黑板模型栏）；checkpoint 轮/无探针轮次不改写板块，保留上
        // 一轮探针过滤后的内容（替代 bundle-only 静态刷新中间态）。
        if profile.role == AgentRole::Main && (pending_checkpoint.is_none() || pending_keeps_tools)
        {
            controller.sync_console_registrations();
        }

        // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.3): SESSION 常驻轮次
        // 预算块删除——预算保留为静默硬门（耗尽时仅提示一次）；主代理系统
        // 提示 = 近零中间态文本（build_system_prompt(None)），检索子代理
        // 提示也不再携带预算块。live 预算仍可经 blackboard_read
        // section=session 按需读取（PUSH→PULL 2026-08-21 方案 A）。
        let system = match &profile.system_kind {
            SystemPromptKind::Main => controller
                .main_agent
                .prompt_builder
                .build_system_prompt(None),
            SystemPromptKind::Retrieval { role, goal } => {
                // ADR-0010 §3.2 task contract — the subagent's own system
                // (citation rules + [DOC]/[SOURCE] delivery contract), with
                // the shared budget declaration (retired in R1 — mechanical
                // hard gate only).
                crate::prompt::build_retrieval_system_prompt(*role, goal, "")
            }
        };
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 计划型执行框架
        // 在系统提示词层无条件注入（plan_first 会话；主/检索子代理同一
        // 入口）——不依赖 AGENTS.md 是否存在（D2 全覆盖，审查收口
        // 2026-08-16）。常量跨轮稳定，不影响前缀缓存。
        let system = if controller.plan_first_enabled() {
            format!(
                "{}\n\n{}",
                crate::planning::PLAN_FIRST_FRAMEWORK_BLOCK,
                system
            )
        } else {
            system
        };
        // v8（2026-09-16 实现批，审查 R-9）：阶梯/守卫量尺＝模型面估算 ＋
        // **静态开销**（系统提示词 ＋ 工具定义）。此处与请求装配同刻刷新，
        // 供下一轮 loop-top 的阶梯判定与守卫使用（首轮为 0，属保守侧偏差）。
        if profile.role == AgentRole::Main {
            // 测试缝隙（`with_model_face_static_overhead`）优先：pin 0 ＝ 只量
            // 会话面（既有小刻度钉子语义不变）。
            face_static_overhead = controller
                .model_face_static_overhead_pin()
                .unwrap_or_else(|| estimate_static_overhead(&system, &current_tool_defs));
        }
        // D-6 (FIX_PLAN 2026-08-06): subagents get the full 256K budget too
        // (the main agent's request-level cap — §3.4.2 same defaults;
        // OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20, ADR-0010
        // §14.35: 32K → 256K).
        // Review F3 (2026-08-10): ONE constant for both lanes — the three
        // agents must never carry their own literals.
        let max_tokens = match &profile.system_kind {
            SystemPromptKind::Main => controller.main_agent_max_tokens(),
            SystemPromptKind::Retrieval { .. } => REQUEST_MAX_TOKENS,
        };

        // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6/§14.9):
        // request-header留痕 — journal the fingerprint (system + tools +
        // config digests) only when it changes from the previous request of
        // this loop (first request = `initial`). The event precedes the
        // model round so a prefix-cache miss is attributable and probe
        // flips (which re-projected `current_tool_defs` above) are
        // cross-checkable against the actual request shape.
        // Boundary (2026-08-15 review, ADR-0010 §14.9/审计 §5): auxiliary
        // model requests OUTSIDE this loop — compaction summary calls
        // (`run_template_compact`) and fast-preflight gates — do not emit
        // header events by design: their header (fixed system prompt +
        // empty tools + config) is constant and never interacts with probe
        // flips, so they are deliberately excluded from the留痕 chain.
        let current_header =
            compute_request_header(&system, &current_tool_defs, &agent.config_fingerprint());
        if last_request_header
            .as_ref()
            .is_none_or(|prev| prev.header_sha256 != current_header.header_sha256)
        {
            let reason = if last_request_header.is_none() {
                "initial"
            } else {
                "change"
            };
            let previous_sha = last_request_header
                .as_ref()
                .map(|prev| prev.header_sha256.clone());
            let change_kind = last_request_header
                .as_ref()
                .map(|prev| header_change_kind(prev, &current_header));
            writer
                .record(
                    EventType::RequestHeaderChange,
                    request_header_payload(
                        &current_header,
                        reason,
                        previous_sha.as_deref(),
                        profile.role.as_str(),
                        change_kind,
                    ),
                )
                .await?;
            last_request_header = Some(current_header);
        }

        let mut partial_text: Vec<String> = Vec::new();
        // P0-D S2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN
        // §3) + 动态上下文滑块 S1 (2026-09-15, 设计 §3.1/§3.2): the
        // MODEL-VISIBLE view 驱逐最旧的连续完整轮（成为台账摘要行），
        // **驻留带**（预算 L，默认 64K）＋其后逐字尾部原样保留 —
        // statefully: the fold point advances only at the mechanical
        // trigger (loop-top, view estimate ≥ H), and between advances the
        // view prefix is byte-stable (pure append, restoring the v1.9
        // prefix-cache discipline). `messages` itself stays full for
        // journal/sidecar audit, so the persisted conversation keeps the
        // complete records.
        // 2026-08-18 (ADR-0010 §14.25 项 1): 常驻状态行移出系统提示词——
        // 每轮请求前把 `[任务状态]` 作为尾随用户消息、仅在变化时追加
        // （尾随消息纪律；退役前的 `[TOOL_ROUND_BUDGET] REMAINING` 同此
        // 纪律），system 提示词保持完全静态，前缀缓存不被步骤推进打断。
        controller.sync_status_line_message(messages);
        // TER T1.9 (2026-09-04)：F6 push 档（默认 off）——剩余评测墙钟
        // 跨 <600/300/120s 阈值时注入中性事实并记 budget_cue_injected
        // （仅主车道；开关/上限未配置 = 零注入）。
        controller
            .maybe_push_f6_budget_cue(
                &mut *writer,
                messages,
                tool_rounds,
                profile.role == AgentRole::Main,
            )
            .await?;
        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26) × 动态上下文
        // 滑块 S1（2026-09-15，设计 §3.1）：the request view comes from the
        // stateful fold point — `messages` verbatim until the first
        // mechanical advance, then preamble ＋ 固定指针 ＋ 驻留带
        // `[fold_cut..bridge_end)` ＋ 逐字尾部（byte-stable prefix, pure
        // 滑块上下文 v8（2026-09-16 勘误批，设计 §1）：**模型面＝投影层装配**。
        // 视图＝前置 ＋ 固定指针 ＋ 分块表 ＋ D4 机械段 ＋ 各分块（原文，或被
        // 压缩/截断块的 marker 摘要行）＋ 主滑块（最近 x K 连续完整轮，逐字）
        // ＋ 尾部。`messages`（本地面）逐字全量、不受影响；**没有机械驱逐**，
        // 故两次压缩之间前缀只追加（不变量 I2/I6）。压缩窗口轮与普通轮装配
        // 同形——v8 的模型面本来就包含全部携带内容（不存在「窗口绕过折叠」）。
        // 0bc S2④：模型面装配＝可失败分配路径——失败以 `AllocFailure` 上抛、
        // run 走终态收口（不 abort；链有效、留终态，与 journal 降级链同形）。
        let request_messages = crate::model_face::build_model_face(messages, &face_params)
            .map_err(|e| AgentLoopError::AllocFailure(e.to_string()))?;
        // FUS-LEDGER-FOLD-STATE 复验取证 (2026-08-18)：`ORZ_DEBUG_VIEW=1`
        // 时在请求失败路径 dump 实际发送的视图角色序列（定位折叠/压缩
        // 交互下的消息配对破坏点；正常路径零成本）。
        let view_debug = std::env::var("ORZ_DEBUG_VIEW").is_ok().then(|| {
            request_messages
                .iter()
                .map(|m| match m.role {
                    Role::Assistant => format!(
                        "A[{}]",
                        m.tool_calls
                            .iter()
                            .map(|t| t.call_id.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    Role::Tool => {
                        format!("T[{}]", m.tool_call_id.as_deref().unwrap_or("?"))
                    }
                    _ => "U".to_string(),
                })
                .collect::<Vec<_>>()
        });
        let response = match agent
            .run_round(
                &system,
                request_messages,
                current_tool_defs.clone(),
                max_tokens,
                cancel,
                heartbeat,
                &mut |chunk| {
                    // F-06 (2026-08-07 review): accumulate the streamed
                    // content deltas — on an abort (watchdog/timeout)
                    // the partial output must still reach the journal.
                    partial_text.push(chunk.to_string());
                    // Subagent text has no live consumer — deltas are
                    // dropped (F-03, matching the pre-split one-shot pass).
                    if profile.role == AgentRole::Main {
                        host.on_text_delta(chunk);
                    }
                },
            )
            .await
        {
            Ok(r) => {
                // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 /
                // 设计 §2.3)：成功路径 transport 重试计数入事件面——
                // 重试后恢复（recovered），journal 可观测重试频率。
                if r.transport_retry.retries > 0 {
                    writer
                        .record(
                            EventType::TransportRetry,
                            serde_json::json!({
                                "agent_role": profile.role.as_str(),
                                "outcome": "recovered",
                                "kind": match r.transport_retry.kind {
                                    Some(TransportRetryKind::ZeroChunk) => {
                                        serde_json::Value::String("zero_chunk".into())
                                    }
                                    Some(TransportRetryKind::Midstream) => {
                                        serde_json::Value::String("midstream".into())
                                    }
                                    None => serde_json::Value::Null,
                                },
                                "retries": r.transport_retry.retries,
                                "reason": r.transport_retry.reason,
                            }),
                        )
                        .await?;
                }
                r
            }
            // Phase 3 slice #11 (P3-7): a cancellation observed mid-stream
            // is a cancel, not a model failure — it must end the run with
            // `run_cancelled`, not a spurious `run_failed`.
            Err(GatewayError::Cancelled) => {
                return Err(AgentLoopError::Cancelled);
            }
            Err(other) => {
                if let Some(roles) = view_debug.as_ref() {
                    tracing::error!(
                        "model request failed ({other}); view roles: {}",
                        roles.join(" ")
                    );
                }
                // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 /
                // 设计 §2.3)：失败路径 transport 重试耗尽事件——耗尽后
                // 显式失败（exhausted），带类别与原因；哨兵中断
                // （attempts=0）不在此列。记录在 run_failed/terminal
                // 之前，journal 链序完整。
                if let GatewayError::StreamInterrupted {
                    attempts,
                    saw_chunk,
                    detail,
                    ..
                } = &other
                    && *attempts > 0
                {
                    writer
                        .record(
                            EventType::TransportRetry,
                            serde_json::json!({
                                "agent_role": profile.role.as_str(),
                                "outcome": "exhausted",
                                "kind": if *saw_chunk { "midstream" } else { "zero_chunk" },
                                "retries": attempts,
                                "reason": detail,
                            }),
                        )
                        .await?;
                }
                // F-06 (D-7 "保留输出 + incomplete 标记 + 明确终止原因"): a
                // stream that aborted after producing partial content
                // must not lose it from the audit trail — journal it as
                // an incomplete model output BEFORE the terminal event
                // records the failure. Previously the partial text went
                // only to live deltas; the journal had a run_failed with
                // no trace of what was produced (2026-08-07 review).
                if !partial_text.is_empty() {
                    writer
                        .record(
                            EventType::ModelOutput,
                            serde_json::json!({
                                "text": partial_text.concat(),
                                "tool_calls": [],
                                // No natural finish reached — closest
                                // enum value; the terminal run_failed
                                // carries the real abort reason.
                                "finish_reason": "length",
                                "reasoning_tokens": null,
                                "completion_tokens": null,
                                "cache_hit_tokens": null,
                                "cache_miss_tokens": null,
                                "incomplete": true,
                            }),
                        )
                        .await?;
                }
                // OUTPUT-DEGENERATION-GUARD (2026-08-19) + OUTPUT-BUDGET-
                // RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35):
                // 输出健康哨兵中断分流——达到 DEGENERATION_LIMIT（会话级
                // 连续，两族共享）→ `AgentLoopError::Degeneration`（run 层
                // 记 run_invalidated）；未达上限 → run_failed 同路径。审计
                // 留痕（tracing）携带触发族与 detail，不改终止语义
                // （设计 §3.3 journal/审计）。
                if let GatewayError::StreamInterrupted { detail, .. } = &other
                    && (crate::gateway::transport::is_degeneration_detail(detail)
                        || crate::gateway::transport::is_reasoning_guard_detail(detail))
                {
                    tracing::warn!(
                        guard_family = crate::gateway::transport::guard_family_label(detail),
                        detail = %detail,
                        "output-health guard interrupted the model round (audit-only)"
                    );
                    if detail.starts_with(crate::gateway::transport::DEGENERATION_LIMIT_PREFIX) {
                        // STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010
                        // §14.37 / 设计 §2.2.4)：显式终止原因带触发轮次
                        // （当前正在进行的模型请求轮，1 基；tool_rounds 在
                        // 工具发放后才 +1）——journal/TUI 可追溯。
                        return Err(AgentLoopError::Degeneration(format!(
                            "{detail} round={}",
                            tool_rounds + 1
                        )));
                    }
                    return Err(AgentLoopError::Model(format!(
                        "stream degeneration guard: {detail}"
                    )));
                }
                return Err(AgentLoopError::Model(other.to_string()));
            }
        };

        // Cooperative cancellation checkpoint (Phase 3 slice #7): a
        // cancelled run may omit this round's `model_output` — the chain
        // stays valid (the terminal event follows).
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(AgentLoopError::Cancelled);
        }

        writer
            .record(
                EventType::ModelOutput,
                serde_json::json!({
                    "text": response.text,
                    "tool_calls": response.tool_calls.iter().map(|tc| {
                        serde_json::json!({
                            "name": tc.name,
                            "arguments": tc.arguments,
                            "call_id": tc.call_id,
                        })
                    }).collect::<Vec<_>>(),
                    "finish_reason": match response.finish_reason {
                        FinishReason::Stop => "stop",
                        FinishReason::ToolCalls => "tool_calls",
                        FinishReason::Length => "length",
                    },
                    // D-6 usage observation — reasoning tokens per round
                    // calibrate the single-round budget decision (data →
                    // whether the 256K cap needs calibration; 32K→256K per
                    // OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20,
                    // ADR-0010 §14.35).
                    "reasoning_tokens": response.reasoning_tokens,
                    "completion_tokens": response.completion_tokens,
                    // Cache-hit observation (2026-08-07 fix): per-round
                    // hit/miss tokens verify the prefix-cache fix — the
                    // hit rate jumped from ~17% (per-round REMAINING in
                    // the rebuilt system prompt) to 98%+ steady-state /
                    // ~80% incl. cold start (live-verified 2026-08-07).
                    "cache_hit_tokens": response.cache_hit_tokens,
                    "cache_miss_tokens": response.cache_miss_tokens,
                }),
            )
            .await?;

        // P2-10 F3 §4.5 (I3): a decision round = model_output with non-empty
        // tool_calls. Feed the LIF engine (T̂ interval + channel decay +
        // temporal record) — pure observation, zero injection (§3.4).
        // v7 语义轨识别（S1 修订批，设计 §3.4.1，DP-14）：模型在本轮回复文本
        // 里产出语义摘要块 ⇒ 记录待执行（下一个 loop-top 安全间隙执行
        // 「摘要替换被压区」，mode=model_summary）。工具轮的文本不进
        // `messages`（只留 model_output 事件），故识别点必须在这里；仅主
        // 车道（检索车道不折叠、不压缩——车道范围裁决）。
        if profile.role == AgentRole::Main
            && pending_semantic.is_none()
            && let Some(summary) = response
                .text
                .as_deref()
                .and_then(crate::context_scale::extract_model_summary)
        {
            // 0ap 复核批（P2）：归因随摘要**在产出时固定**——此刻在程窗口的
            // 种类即促成者；无窗口在程（模型自选摘要块）记 `None`。
            pending_semantic = Some((summary, window_kind));
        }
        if !response.tool_calls.is_empty() {
            controller
                .lif
                .lock()
                .unwrap()
                .on_decision_round(AgentLoopController::now_epoch_secs());
        }
        // P2-14 S1（2026-09-04，ADR-0010 §14.54）：决策轮 = model_output
        // 带工具调用。捕获决策后 LIF 轮章（声明消息与黑板行共用同一轴），
        // 主车道同时置「执行窗主轮章」pin——本轮工具段（含嵌套检索子
        // 车道）写共享折叠分区一律盖本主轮章（派发主轮口径）；守卫随
        // 循环体结束/break/return Drop 清除，不跨模型请求残留。
        // 车道范围裁决（2026-09-04 复审处理，ADR-0010 §14.54 补注）：v0.3
        // 折叠视图快照 marker 只用于主会话压缩（主会话消息轮轴 = 主决策
        // 轮轴，行章由 pin 对齐）；检索子车道会话按自己的消息轮次 drain，
        // 其消息不盖主决策轮章（保持 None）——子车道压缩继续走 v0.2 五段
        // 模板，避免「子消息实时 LIF 轮号 vs 行主轮章」双轴错配（若给子
        // 消息盖实时轮章，将来按「保留尾首条声明轮章」推 r_keep 会把保留
        // 尾消息对应的行全部误收进 marker）。
        let decision_round: Option<u64> = if response.tool_calls.is_empty() {
            None
        } else {
            Some(controller.blackboard_stamp().0)
        };
        // 检索/grill 等非 Main 车道不盖消息轮章（见上车道范围裁决）。
        let decision_round = if profile.role == AgentRole::Main {
            decision_round
        } else {
            None
        };
        let _board_stamp_pin = if response.tool_calls.is_empty() || profile.role != AgentRole::Main
        {
            None
        } else {
            Some(controller.pin_main_board_stamp())
        };

        // P0-D: track the round — 自上一次压缩以来的模型轮数（事件读数）；
        // **v8 起实测 prompt token 不再参与触发**（阶梯吃模型面估算）。
        rounds_since_compact += 1;

        // GAP-INQUIRY-SPLIT (2026-08-09): the model round just COMPLETED —
        // count it against the session-level orientation counter (ADR-0010
        // §4.2: completed logical model rounds; this point is the only
        // completion point that every round passes — tool rounds, deny
        // rounds and gate-answer rounds alike — while transport retries
        // never reach it, so they never count).
        if let (Some(o), Some(role)) = (orientation.as_deref_mut(), profile.orientation_role) {
            o.feed_round(role);
        }

        // THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29 用户裁决): orientation
        // 软门——pending 轮的模型回答不再校验 JSON 模板、不再禁止工具：
        // 纯文本回答被"消费"（保留进会话）后 loop 明确续跑；工具调用照常
        // 执行（commit 后落入下方常规派发路径）。fire 在消费点提交
        // （延迟 commit）。软门不产生 `checkpoint_response` 事件（无模板
        // 可验证；fire 事件 + 后续 model_output/tool 事件构成审计链）。
        // 回答轮本身照常计入已完成逻辑模型轮（上方 feed）并保留在会话中。
        // P2-11 DC 清理（2026-08-31）：强制模板轮机制已整体删除——剩余
        // pending 轮仅 orientation 软门与 console 询问轮。
        // 边界（2026-08-29 审查收口）：若 run 在 fire 与消费之间硬中断，
        // 计数保持未提交、journal 留孤儿 fire 事件，下次 run 首轮重触发
        // ——见 controller `maybe_fire_orientation` 延迟 commit 注释。
        if let Some(pending) = pending_checkpoint.take() {
            // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.3):
            // console 双模式询问轮——模板不同（decision/reason），无工具轮
            // 语义同 checkpoint；一次重填、仍失败默认 stay；每 run 至多一次。
            if let PendingCheckpoint::ConsoleModeInquiry {
                attempt,
                streak: _,
                order_ids: _,
            } = &pending
            {
                let text = response.text.as_deref().unwrap_or_default();
                let parsed = crate::console_mode::parse_and_validate_inquiry(text);
                let answer = match parsed {
                    Ok(answer) => Some(answer),
                    Err(errors) if *attempt < crate::console_mode::MAX_INQUIRY_ATTEMPTS => {
                        messages.push(Message {
                            role: Role::User,
                            content: crate::console_mode::inquiry_refill_feedback(&errors),
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                            round: None,
                        });
                        pending_checkpoint = Some(pending.with_attempt(*attempt + 1));
                        continue;
                    }
                    Err(_) => None,
                };
                match answer {
                    Some(answer) if answer.decision == "switch" => {
                        controller
                            .switch_console_to_direct(writer, tool_rounds, answer.reason.as_deref())
                            .await?;
                    }
                    _ => {
                        // stay（含降级默认 stay）：写 transition 事件 + gate_log。
                        controller
                            .record_console_stay(
                                writer,
                                tool_rounds,
                                answer
                                    .as_ref()
                                    .and_then(|a| a.reason.as_deref())
                                    .or(Some("degraded: invalid template after refill")),
                            )
                            .await?;
                    }
                }
                // 询问轮的回答是模型输出——保留在会话中。
                if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: text,
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: response.reasoning_content.clone(),
                        round: None,
                    });
                }
                continue;
            }
            // 0ae D3：模型参与压缩窗口轮（2026-09-15，设计 §6，DP-7）——
            // 模型固化（blackboard_write）＋标注；≤3 轮
            // （COMPRESSION_WINDOW_ROUNDS）。FR-3（2026-09-21 0bc 长杂轮，
            // 用户裁决「硬提醒仅是打断式提醒、不锁工具面」）：撤出 0AE-C1
            // 的窗口工具面收窄——窗口轮工具面＝常规面（见上方
            // current_tool_defs 装配），本轮声明的动作**全部落穿正常派发
            // 路径照常执行**（不过滤、不丢弃、无机械提示）。窗口收口延迟到
            // 下一 loop-top（写入派发完成后读数真实，
            // `model_compression_close`；窗口内 context_compress 调用由执行
            // 点按 in_progress no-op 应答，不改变收口判定位）。
            if let PendingCheckpoint::ModelCompression {
                rounds_left,
                window_start_writes,
            } = &pending
            {
                if !response.tool_calls.is_empty() {
                    // 不记审计、不 continue——窗口收口所需状态存入延迟收口
                    // 变量（下一 loop-top 写入已派发完成，读数真实），落穿
                    // 下方正常派发路径派发本轮全部声明调用。窗口轮文本随
                    // model_output 事件留痕（与 orientation 工具轮消费同
                    // 口径：工具轮不单独回放文本）。
                    model_compression_close = Some((
                        *window_start_writes,
                        crate::context_scale::COMPRESSION_WINDOW_ROUNDS
                            .saturating_sub(*rounds_left)
                            + 1,
                    ));
                } else {
                    // 纯文本轮：窗口轮回答保留在
                    // 会话中（v7：语义摘要块由 loop-top 扫描消费）。
                    if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                        messages.push(Message {
                            role: Role::Assistant,
                            content: text,
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: response.reasoning_content.clone(),
                            round: None,
                        });
                    }
                    let writes_now = svc.blackboard.read().model_note_count();
                    // v7 V3（S1 修订批，设计 §3.4.1）：窗口内的「参与」＝
                    // 产出**语义摘要块**或固化到黑板——语义轨是 v7 的主面
                    // （S1 只认黑板写入 ⇒ 模型产出摘要仍被判未参与并连开
                    // 三轮窗口）。
                    let produced_summary = response
                        .text
                        .as_deref()
                        .and_then(crate::context_scale::extract_model_summary)
                        .is_some();
                    let participated = writes_now > *window_start_writes || produced_summary;
                    if *rounds_left > 1 && !participated {
                        // 未产出摘要也未固化：再给一轮（有限窗口，不挂死）。
                        pending_checkpoint = Some(PendingCheckpoint::ModelCompression {
                            rounds_left: rounds_left.saturating_sub(1),
                            window_start_writes: *window_start_writes,
                        });
                        messages.push(Message {
                            role: Role::User,
                            content: crate::context_scale::window_remaining_notice(
                                rounds_left.saturating_sub(1),
                            ),
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                            round: None,
                        });
                        continue;
                    }
                    // 窗口收口（此时无在途写入）：原地按 C2 形状落账 +
                    // 机械折叠兜底旗标（下一 loop-top summary_now 执行）。
                    let rounds_used = crate::context_scale::COMPRESSION_WINDOW_ROUNDS
                        .saturating_sub(*rounds_left)
                        + 1;
                    let payload = mechanical_audit.record(
                        "model_compression",
                        tool_rounds,
                        format!(
                            "model_participated={participated} rounds_used={rounds_used} \
                             window_start_writes={window_start_writes} writes_now={writes_now} \
                             semantic_summary={produced_summary}"
                        ),
                        if participated {
                            None
                        } else {
                            Some("model_not_participated".to_string())
                        },
                    );
                    writer
                        .record(
                            EventType::MechanicalAuditUpdate,
                            serde_json::json!({
                                "kind": crate::mechanical_audit::KIND_MODEL_COMPRESSION,
                                "payload": payload,
                            }),
                        )
                        .await?;
                    // v8（2026-09-16 勘误批）：窗口收口**不再**置机械兜底旗标
                    // ——机械层不替模型决定模型面收缩（设计 §4）。窗口内模型产出
                    // 的语义摘要已由 `pending_semantic` 承载，在下一次 loop-top
                    // 按块压缩；未产出则如实落账 `model_participated=false`。
                    continue;
                }
            } else {
                // Orientation soft gate — 消费并续跑（详见分支上方注释）。
                debug_assert!(matches!(pending, PendingCheckpoint::Orientation { .. }));
                checkpoint::commit_pending(pending, orientation.as_deref_mut());
                if response.tool_calls.is_empty() {
                    // 纯文本回答被消费：保留进会话，loop 明确续跑。
                    if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                        messages.push(Message {
                            role: Role::Assistant,
                            content: text,
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: response.reasoning_content.clone(),
                            round: None,
                        });
                    }
                    continue;
                }
                // 工具轮：commit 已完成，落入下方常规工具派发路径（不
                // continue——触发轮不禁工具）。
            }
        }

        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 首轮计划轮——
        // 无工具回复（未写 plan_write）不算最终答案：继续计划面；连续
        // MAX_PLAN_ROUNDS_WITHOUT_SUBMISSION 轮未提交则机械降级
        // （plan_not_submitted 留痕）并放行，绝不挂死。
        if plan_gate.is_some() && response.tool_calls.is_empty() {
            let gate = plan_gate.as_mut().expect("checked above");
            gate.rounds_without_submission += 1;
            if gate.rounds_without_submission >= crate::planning::MAX_PLAN_ROUNDS_WITHOUT_SUBMISSION
            {
                let g = plan_gate.take().expect("checked above");
                writer
                    .record(
                        EventType::PlanWrite,
                        crate::planning::plan_write_payload(
                            "",
                            "",
                            0,
                            "degraded",
                            g.attempt,
                            &crate::planning::PlanVerdict::default(),
                            Some("plan_not_submitted"),
                        ),
                    )
                    .await?;
                // 降级后按普通最终答案路径继续（下面的 final-answer 分支处理）。
            } else {
                if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: text,
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: response.reasoning_content.clone(),
                        round: None,
                    });
                }
                continue;
            }
        }

        // D-8: the post-exhaustion final round may only produce TEXT — a
        // tool request there is refused (no execution after the budget is
        // gone) and the run ends with the partial result. Checked BEFORE
        // the final-answer path so the exhaustion round never triggers
        // the counterexample gate's extra model round.
        if budget_exhausted {
            if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                messages.push(Message {
                    role: Role::Assistant,
                    content: text,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: response.reasoning_content.clone(),
                    round: None,
                });
            }
            last_text = response.text;
            // 0ae D3 窄边沿修复（2026-09-15）：耗尽前最后一轮恰为窗口轮
            // 落穿时，保留的 blackboard_write 声明因 D-8 不再派发——收口
            // 审计事件必须在本 break 前落地（model_not_participated 如实），
            // 否则状态随循环终止丢弃（审查登记观察）。
            finalize_model_compression_close(
                &mut model_compression_close,
                &mut mechanical_audit,
                svc,
                writer,
                tool_rounds,
            )
            .await?;
            break;
        }

        // 0ar S2-D1（2026-09-19，β 收尾回合出口）：置位后的第一轮响应即为
        // 收尾产物——工具面已收空，本轮不再派发任何调用（模型仍声明工具
        // 属幻觉面，一律不执行，D-8 同纪律）；文本入会话，`last_text` 交
        // dispatch 形成结果并按 `batch_close` 映射 terminal_reason。
        if close_round_armed {
            if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                messages.push(Message {
                    role: Role::Assistant,
                    content: text,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: response.reasoning_content.clone(),
                    round: None,
                });
            }
            last_text = response.text;
            finalize_model_compression_close(
                &mut model_compression_close,
                &mut mechanical_audit,
                svc,
                writer,
                tool_rounds,
            )
            .await?;
            break;
        }

        if response.tool_calls.is_empty() {
            // §4.6.1/4.6.2: the first no-tool-call response is a
            // final-answer candidate — before committing it, the
            // counterexample gate fires ONCE (the block explicitly tells
            // the model it appears only once). The candidate is journaled
            // as model_output (evidence) but not committed to the
            // conversation; the post-gate response is the final answer
            // (D6). A post-gate round that returns tool calls continues
            // the loop normally — the gate never fires again this run.
            // Grill mode (2026-08-08): the counterexample gate is a
            // run-semantic (final answers); a grill question is not one
            // — skipped.
            if !counterexample_fired && profile.counterexample_gate {
                writer
                    .record(
                        EventType::CounterexampleGate,
                        serde_json::json!({
                            "position": "final_answer",
                            "message_block": COUNTEREXAMPLE_GATE_BLOCK,
                            "once_only": true,
                        }),
                    )
                    .await?;
                // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 /
                // 设计 §2.4)：审计报告随最终答案前中立问询轮注入——与
                // [COUNTEREXAMPLE_GATE] 同轮独立块；收敛为执行事实摘要
                // （动作/文件 delta/预算/异常事实），无建议、无引导；
                // 报告块不进归档（注册进 injected-block filter）。
                let audit_report = mechanical_audit.report(Some(run_started_at.elapsed()));
                messages.push(Message {
                    role: Role::User,
                    content: audit_report,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                messages.push(Message {
                    role: Role::User,
                    content: COUNTEREXAMPLE_GATE_BLOCK.to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                counterexample_fired = true;
                continue;
            }
            // 0ac S3①-b M1（2026-09-15，设计 §4.1「句号边界分段续写」的
            // 收尾注入形态）：本轮已是纯文本终答候选而队列仍有未投递事实
            // 时，保留该轮 assistant 文本（含 reasoning_content）、注入
            // 一次机械事实消息后 `continue` 续跑——把这一轮切成一段；
            // 每 run 至多一次，用尽即按原路径收尾（绝不挂死、绝不重复
            // 注入）。子开关从属主开关（A/B 面）；boundary 取闭枚举
            // B2_turn_end（轮末边界的最近语义位）。
            // 0AC-A1 修复（2026-09-15 深审）：B2 drain——终答候选处补收
            // 完成事实（交接件 §4-⑤-B 的缺失半边）。B1 间隙 drain+due
            // 背靠背 ⇒ 队列到终答处恒空，终答（无工具轮）期间完成的后台
            // 任务本 run 零投递（admit 唯一调用点在 B1 块内的接线级断点）。
            // gate 用 `m1_enabled()`（蕴含主开关）而非仅主开关——M1 关时
            // 不 drain，避免只进不投；已注入过（m1_used）同样不 drain
            // （再入队只会落 close_drop 留痕后丢弃）。
            if !m1_used && crate::immediate_delivery::m1_enabled() {
                let facts = host.drain_completed_tasks().await;
                if !facts.is_empty() {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    for f in facts {
                        delivery_queue.admit(
                            crate::immediate_delivery::PendingFact::background_task(
                                &f.task_id,
                                f.report,
                                now_ms,
                                u64::from(tool_rounds),
                            ),
                        );
                    }
                }
            }
            if !m1_used && crate::immediate_delivery::m1_enabled() && !delivery_queue.is_empty() {
                if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: text,
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: response.reasoning_content.clone(),
                        round: None,
                    });
                }
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let deliveries = delivery_queue.due(u64::from(tool_rounds), now_ms);
                for d in &deliveries {
                    writer
                        .record(
                            EventType::ResultDelivered,
                            crate::immediate_delivery::delivered_fact_payload(
                                d,
                                crate::immediate_delivery::BOUNDARY_B2,
                            ),
                        )
                        .await?;
                }
                messages.push(Message {
                    role: Role::User,
                    content: format!(
                        "[结果投递] {}",
                        crate::immediate_delivery::render_delivery_message(&deliveries)
                    ),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                m1_used = true;
                continue;
            }
            // Include the final assistant message in the conversation so
            // the rebuilt dialogue matches what a real transport would have
            // received
            // (2026-08-04 review P2-2).
            if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                messages.push(Message {
                    role: Role::Assistant,
                    content: text,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: response.reasoning_content.clone(),
                    round: None,
                });
            }
            last_text = response.text;
            break;
        }

        // IP3a: instruction provenance gate — evaluated once per tool
        // phase. A block stops every tool and ends the phase without
        // further model calls (the instruction stream is poisoned).
        let ipg = ToolDispatcher::evaluate_ipg(prompt, workspace_trust);
        if matches!(ipg, GateDecision::Block { .. }) {
            writer
                .record(
                    EventType::InstructionProvenanceGate,
                    serde_json::json!({
                        "decision": ipg.decision_str(),
                        "entries": 1,
                    }),
                )
                .await?;
            writer
                .record(
                    EventType::GateDecision,
                    serde_json::json!({
                        "gate": "instruction_provenance_gate",
                        "decision": "block",
                        "tools": response.tool_calls.iter().map(|tc| tc.name.clone()).collect::<Vec<_>>(),
                    }),
                )
                .await?;
            {
                let mut w = svc.blackboard.write();
                w.gate_log
                    .gate_decisions
                    .push("IPG: block (tool phase)".to_string());
            }
            last_text = response.text;
            // 0ae D3 窄边沿修复（2026-09-15）：IPG block 在派发前终止本轮
            // ——窗口落穿保留的 blackboard_write 声明不会派发，收口审计
            // 事件必须在本 break 前落地（model_not_participated 如实）。
            finalize_model_compression_close(
                &mut model_compression_close,
                &mut mechanical_audit,
                svc,
                writer,
                tool_rounds,
            )
            .await?;
            break;
        }

        // Execute tool calls, feeding results back into the conversation.
        // Cooperative cancellation checkpoints (Phase 3 slice #7): the
        // loop-top check before dispatch plus a per-tool re-check inside
        // — a cancel landing while tool #k runs must not start tools
        // #k+1..N of the same round (2026-08-05 review P2-2; the comment
        // "never starts a new tool" holds per tool, not per round).
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(AgentLoopError::Cancelled);
        }
        // Replay the assistant's call declarations BEFORE their results:
        // the provider protocol requires each tool message's
        // `tool_call_id` to match a declaration in the history, and
        // DeepSeek rejects unmatched ids (2026-08-06 design review D2-1).
        // The reasoning content rides the same declaration message —
        // DeepSeek expects it replayed with the assistant turn
        // (alpha-test 2026-08-06 closure).
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: response.tool_calls.clone(),
            reasoning_content: response.reasoning_content.clone(),
            // P2-14 S1：声明消息盖所属决策轮 LIF 轮章——压缩点据此取保留
            // 尾首条声明的轮章作 r_keep（跨恢复/多 prompt 精确）。
            round: decision_round,
        });
        // Pending policy messages (denial breaker) — appended AFTER the
        // tool batch completes so no user message lands between the
        // assistant declaration and its tool replies (provider protocol;
        // 2026-08-07 wordy 400 + review P1).
        let mut pending_policy: Vec<Message> = Vec::new();
        // ADR-0010 §3.5.4 round-level denial aggregation: the breaker
        // counts ROUNDS (a round with N denied calls and no success
        // counts 1), keyed by (tool, reason_code, policy_revision).
        let mut round_denials: Vec<crate::controller::DenialKey> = Vec::new();
        let mut round_had_success = false;
        // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): the accumulated
        // estimated tokens of THIS model round's injected tool results
        // (chars/2 — the same estimator as compaction). Once at/over the
        // budget, the remaining calls of the batch are refused without
        // execution and told to continue with offset/grep-first.
        let mut round_inject_tokens: u64 = 0;
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 本轮是否提交过
        // plan_write —— 无提交的工具轮计入 plan_not_submitted 降级计数。
        let mut plan_write_called = false;
        // 2026-08-16 审查收口：计划轮不消耗 tool-round 预算（用户裁决）——
        // 在批处理前固定本轮的“计划轮身份”，即使 plan_write 中途 accepted
        // 解除门，本轮仍按计划轮处理（不 +1）；whitelist 的 tool_rounds==0
        // 窗口因此顺延到计划落板后的首个工具轮（P2-2 顺延裁决）。
        let plan_round_active = plan_gate.is_some();
        // 2026-08-08 blackboard partition (A2): snapshot the edit-action
        // length BEFORE this round's tools — the incremental push after
        // the batch reports exactly the records this round added.
        let round_edit_count = svc.blackboard.read().edits.len();
        // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30, TODO P0-0k
        // 第一批第 1 项)：同轮读类并行——批首 ≥2 个连续只读 Host 调用并发
        // 执行、按声明序提交（事件链/消息/审计/预算序与串行等价）。写类/
        // 命令/派发类与计划轮保持串行。
        // 0k 审查处理 (2026-08-30) 登记边界：
        // - P2-1：黑板 `tool_actions`/`exec` 分区在工具完成时写入
        //   （host_exec 内 push），并行批次下该分区为完成序、事件链为
        //   声明序——记录均带真实 timestamp，审计权威以事件链为准，本
        //   分区展示序差异接受（行为不变，不补写）。
        // - P2-3：预算超限的后续调用在提交阶段按串行语义拒绝（消息面
        //   注入拒绝消息、结果不计入注入预算），但调用已真实执行——事件
        //   仍按声明序重放留痕、后处理（direct trace/evidence/DC/机械
        //   审查）照常，审计面与事实一致。
        // - P3-3：批次内某调用出错（Err）时，其余已执行的调用仍按声明序
        //   提交（执行无留痕是更大的审计缺口），首个错误在批次提交完毕
        //   后传播（串行语义为遇错即停；并行批次无法中途停止已启动的
        //   调用）。
        let run_id = writer.run_id().to_string();
        let mut parallel_skip_until = 0usize;
        if !plan_round_active {
            // P3-1：批次启动前检查取消令牌（串行路径每工具前检查；并行
            // 批次内调用已启动无法中断，取消在批次提交后由串行循环接续）。
            if cancel.is_some_and(|c| c.is_cancelled()) {
                return Err(AgentLoopError::Cancelled);
            }
            let eligible: Vec<bool> = response
                .tool_calls
                .iter()
                .map(|tc| parallel_read_eligible(tc, profile, plan_round_active))
                .collect();
            let mut run_len = 0;
            while run_len < eligible.len() && eligible[run_len] {
                run_len += 1;
            }
            if run_len >= 2 {
                let batch: Vec<&ToolCall> = response.tool_calls[..run_len].iter().collect();
                // 0ap 复核批（P1）：并行批的**只读会话视图**＝批首会话——并发
                // 语义即「各调用看到同一份批首上下文」。批内注入槽 `local_msgs`
                // 批首为空 `Vec`、只承载本次调用的结果注入（按声明序并回主
                // 会话），当会话读会得出假读数（「滑块外可压缩 0 块」）。
                let batch_conversation: &[Message] = messages;
                // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
                // 委托契约复杂度分档——同轮 `browser_read` 并发上限（档位
                // 机械映射；deep=不限）。host tab 池是最终物理上限，这里
                // 只约束批次内的并发启动（simple 任务不把 tab 池打满）。
                let browser_sem = profile
                    .browser_read_concurrency
                    .filter(|n| *n > 0)
                    .map(|n| Arc::new(tokio::sync::Semaphore::new(n)));
                let mut futures = Vec::with_capacity(batch.len());
                for tc in batch {
                    let tc_owned = tc.clone();
                    let browser_sem_fut = browser_sem.clone();
                    let mut w = EventWriter::buffered(&run_id);
                    let mut local_msgs: Vec<Message> = Vec::new();
                    let direct_ctx = if controller.console_default_enabled()
                        && profile.role == AgentRole::Main
                    {
                        controller.console_direct_begin(&tc_owned.call_id)
                    } else {
                        None
                    };
                    let lane_self_execute = route(&tc_owned.name)
                        == DispatchTarget::ExternalRetrieval
                        && profile.tool_filter.denies_nested_dispatch();
                    let permission_gated = !lane_self_execute;
                    let fetch_candidates = profile.fetch_candidates.clone();
                    let serp_budget = profile.serp_budget.clone();
                    let activation_id = profile.activation_id.clone();
                    futures.push(async move {
                        // 0ar S2：检索族调用发起即计数（发起≠成功——失败
                        // 也计入「已发起检索调用」；主车道计数槽为 None）。
                        if crate::retrieval::batch_close::is_lane_retrieval_tool(&tc_owned.name)
                            && let Some(counter) = svc.retrieval_calls
                        {
                            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        // 档位并发上限：browser_read 先取 permit 再执行
                        // （permit 随 future drop 释放；信号量本批内不
                        // close，acquire 失败实际不可达——fail-closed 兜底）。
                        let _browser_permit = if tc_owned.name == "browser_read" {
                            match browser_sem_fut.as_ref() {
                                Some(sem) => {
                                    Some(sem.clone().acquire_owned().await.map_err(|_| {
                                        AgentLoopError::Assurance(
                                            "browser_read concurrency semaphore closed".into(),
                                        )
                                    })?)
                                }
                                None => None,
                            }
                        } else {
                            None
                        };
                        let direct_stamp = direct_ctx.as_ref().map(|(s, _)| s);
                        let (result, feedback) = controller
                            .run_host_tool_with_plan_gate(
                                host,
                                &mut w,
                                &tc_owned,
                                prompt,
                                workspace_trust,
                                &mut local_msgs,
                                Some(batch_conversation),
                                tool_rounds,
                                heartbeat,
                                activation_id.as_deref(),
                                fetch_candidates.as_deref(),
                                serp_budget.as_deref(),
                                permission_gated,
                                profile.probe_work_tools,
                                None,
                                direct_stamp,
                            )
                            .await?;
                        Ok::<_, AgentLoopError>((
                            tc_owned, result, feedback, w, local_msgs, direct_ctx,
                        ))
                    });
                }
                let outcomes = futures::future::join_all(futures).await;
                // P3-3：错误路径延迟传播——先提交全部已执行调用的结果
                // （事件/后处理留痕），首个错误在批次提交完毕后返回。
                let mut first_err: Option<AgentLoopError> = None;
                for outcome in outcomes {
                    let (tc, result, feedback, mut w, local_msgs, direct_ctx) = match outcome {
                        Ok(x) => x,
                        Err(e) => {
                            if first_err.is_none() {
                                first_err = Some(e);
                            }
                            continue;
                        }
                    };
                    // P2-3：预算超限按串行语义拒绝（消息面注入拒绝消息、
                    // 结果不计入注入预算），但调用已真实执行——事件仍重放
                    // 留痕、后处理照常，审计面与事实一致（并行批次无法像
                    // 串行那样在执行前预检预算）。
                    let budget_refused = round_inject_tokens >= svc.max_inject_tokens_per_round;
                    // 事件按声明序重放（hash 链续接，与串行事件序一致）。
                    for (event_type, payload) in w.drain() {
                        writer.record(event_type, payload).await?;
                    }
                    if budget_refused {
                        let (_, f) = refuse_inject_budget(
                            controller,
                            writer,
                            messages,
                            &tc,
                            round_inject_tokens,
                            svc.max_inject_tokens_per_round,
                            svc.policy_revision
                                .load(std::sync::atomic::Ordering::SeqCst),
                            // 并行批次：调用已真实执行（或已被 gate 拒绝并
                            // 留痕），buffered 事件已含完成事件——预算拒绝
                            // 只注入消息面 + deny，不再写第二条
                            // tool_completed（F11 receipt 同一 call_id 至多
                            // 一条完成事件；S4 实机复验 2026-08-31 发现）。
                            false,
                        )
                        .await?;
                        // P2-10 R2 (2026-08-31): inject-budget refusal = deny.
                        controller.feed_lif_deny(None);
                        match f {
                            Some(PolicyFeedback::Denied(key)) => round_denials.push(key),
                            Some(PolicyFeedback::Succeeded) => round_had_success = true,
                            None => {}
                        }
                    } else {
                        messages.extend(local_msgs);
                    }
                    // direct 动作 trace 收口（与串行 Host 分支同构）。
                    if let Some((stamp, trace)) = direct_ctx {
                        let detail = if result.exit_code == Some(0) {
                            None
                        } else {
                            Some(result.output.as_str())
                        };
                        controller.console_direct_end(
                            &stamp,
                            trace,
                            &tc.name,
                            result.exit_code == Some(0),
                            detail,
                        );
                    }
                    // 检索车道证据收集（与串行 Host 分支同构）。
                    if let Some(evidence) = svc.evidence
                        && let Some(record) = crate::retrieval::evidence::build_evidence_record(
                            &tc.name, &tc, &result,
                        )
                    {
                        evidence.lock().unwrap().push(record);
                    }
                    // 0ar S2：可见倒数行（设计 §3.6）——检索族结果尾部机械
                    // 追加（本批证据快照＋已发起调用数，条目/调用分开报）。
                    if svc.retrieval_calls.is_some()
                        && crate::retrieval::batch_close::is_lane_retrieval_tool(&tc.name)
                    {
                        let snapshot = svc
                            .evidence
                            .map(|e| e.lock().unwrap().clone())
                            .unwrap_or_default();
                        let calls = svc
                            .retrieval_calls
                            .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                            .unwrap_or(0);
                        crate::retrieval::batch_close::append_countdown_to_tool_message(
                            messages,
                            &tc.call_id,
                            &snapshot,
                            calls,
                        );
                    }
                    // 静默机械审查（主车道专属，与串行同构）。
                    if profile.role == AgentRole::Main {
                        crate::mechanical_audit::record_tool_result(
                            &mut mechanical_audit,
                            writer,
                            &tc,
                            &result,
                            tool_rounds,
                            controller.retrieval_candidate_count(),
                        )
                        .await?;
                    }
                    match feedback {
                        Some(PolicyFeedback::Denied(key)) => round_denials.push(key),
                        Some(PolicyFeedback::Succeeded) => round_had_success = true,
                        None => {}
                    }
                    if !budget_refused {
                        round_inject_tokens =
                            round_inject_tokens.saturating_add(estimate_message_tokens(&Message {
                                role: Role::Tool,
                                content: format!("[{}] {}", tc.name, result.output),
                                tool_call_id: None,
                                tool_calls: Vec::new(),
                                reasoning_content: None,
                                round: None,
                            }));
                    }
                }
                if let Some(e) = first_err {
                    return Err(e);
                }
                parallel_skip_until = run_len;
            }
        }
        // 0ar S2-D3（2026-09-19，检索批次回送设计 §5.4 定案「合并优先＋
        // 溢出拆轮」）：派发前预扫描本轮剩余调用中的检索派发调用（可派发
        // 车道——检索车道 nested dispatch 已被单席位纪律拒绝，不入扫描）。
        // 前 MERGE_MAX_QUERIES(3) 个调用**合并优先**：由首个调用承载单激
        // 活多 query 任务，其余被合并调用以合并回执交回；溢出调用按 R-1
        // 形态**未派发拒绝**（无 ToolStarted 的 gate 拒绝，§5.5 模板）。
        // 二者互补：合并让子代理承担更重任务、消除 N 倍往返；溢出底座保
        // 住「一轮发 N 个搜索也不会让主代理失去回合」。
        let dispatch_bound: Vec<usize> = (parallel_skip_until..response.tool_calls.len())
            .filter(|&i| {
                let target = route(&response.tool_calls[i].name);
                matches!(
                    target,
                    DispatchTarget::InternalRetrieval | DispatchTarget::ExternalRetrieval
                ) && !profile.tool_filter.denies_nested_dispatch()
            })
            .collect();
        // 0ar S3 前去噪（§5.6）：先按 query 去重——检索车道跨轮（同一
        // 激活）持久、主车道仅同轮（跨激活重派仍新开）；重复 query 不再
        // 进入合并/派发面，登记为指针回踩（原 call_id）。
        let mut duplicate_positions: std::collections::HashMap<usize, String> =
            std::collections::HashMap::new();
        let mut dispatchable: Vec<usize> = Vec::new();
        let mut round_dispatched_queries: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let dedupe_map = if svc.retrieval_calls.is_some() {
            &mut lane_dispatched_queries
        } else {
            &mut round_dispatched_queries
        };
        for &i in &dispatch_bound {
            let tc = &response.tool_calls[i];
            match crate::retrieval::batch_close::query_key(&tc.arguments) {
                Some(key) => {
                    if let Some(original) = dedupe_map.get(&key) {
                        duplicate_positions.insert(i, original.clone());
                    } else {
                        dedupe_map.insert(key, tc.call_id.clone());
                        dispatchable.push(i);
                    }
                }
                None => dispatchable.push(i),
            }
        }
        let merged_positions: std::collections::BTreeSet<usize> = dispatchable
            .iter()
            .take(crate::retrieval::batch_close::MERGE_MAX_QUERIES)
            .copied()
            .collect();
        let merged_extra_queries: Vec<(usize, String)> = dispatchable
            .iter()
            .take(crate::retrieval::batch_close::MERGE_MAX_QUERIES)
            .skip(1)
            .map(|&i| {
                let tc = &response.tool_calls[i];
                let q = tc
                    .arguments
                    .get("query")
                    .or_else(|| tc.arguments.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(tc.name.as_str())
                    .to_string();
                (i, q)
            })
            .collect();
        let deferred_positions: std::collections::BTreeSet<usize> = dispatchable
            .iter()
            .skip(crate::retrieval::batch_close::MERGE_MAX_QUERIES)
            .copied()
            .collect();
        // 0au（2026-09-20 立项，S3 摩擦 N2）：派发前 run 级墙钟余量判定——
        // 原预扫描只看「同轮合并上限」，run 还剩多少无人过问（S3 五次
        // trailing：余量 26–191s 的批被 run 墙钟直接截断）。以**首个可派发
        // 调用**的档位墙钟为本批批墙钟（合并激活共用同一批预算；档位表
        // 180/300/450 单一来源），`remaining = 上限 − 已耗`；余量不足 ⇒
        // 本批全部可派发位（含合并位与溢出位）保留不派发。上限解析序＝
        // env（生产读源）> controller seam（测试）> None（不判定、不保留）。
        let run_limit_secs = crate::controller::main_wallclock_limit_secs_override()
            .or(controller.run_wallclock_limit_secs);
        let remaining_run_wallclock = run_limit_secs
            .map(|limit| limit.saturating_sub(run_started_at.elapsed().as_secs()))
            .map(std::time::Duration::from_secs);
        let batch_wallclock = dispatchable
            .first()
            .map(|&i| {
                let tc = &response.tool_calls[i];
                let (query, scope, max_results) =
                    crate::retrieval::effort::effort_inputs_from_args(&tc.arguments);
                let external = matches!(route(&tc.name), DispatchTarget::ExternalRetrieval);
                crate::retrieval::effort::classify_retrieval_effort(
                    &query,
                    scope.as_deref(),
                    max_results,
                    external,
                )
                .wallclock_default()
            })
            .unwrap_or_default();
        let wallclock_reserved = !dispatchable.is_empty()
            && crate::retrieval::batch_close::wallclock_reserved(
                remaining_run_wallclock,
                batch_wallclock,
            );
        let mut tool_idx = 0usize;
        while tool_idx < response.tool_calls.len() {
            let tc = &response.tool_calls[tool_idx];
            if tool_idx < parallel_skip_until {
                tool_idx += 1;
                continue;
            }
            if cancel.is_some_and(|c| c.is_cancelled()) {
                return Err(AgentLoopError::Cancelled);
            }
            if round_inject_tokens >= svc.max_inject_tokens_per_round {
                let (_, feedback) = refuse_inject_budget(
                    controller,
                    writer,
                    messages,
                    tc,
                    round_inject_tokens,
                    svc.max_inject_tokens_per_round,
                    svc.policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                    // 串行预检：调用未执行、无 buffered 事件——写一条无
                    // ToolStarted 的 gate 拒绝完成事件（F11 gate 段允许）。
                    true,
                )
                .await?;
                // P2-10 R2 (2026-08-31): inject-budget refusal = deny.
                controller.feed_lif_deny(None);
                match feedback {
                    Some(PolicyFeedback::Denied(key)) => round_denials.push(key),
                    Some(PolicyFeedback::Succeeded) => round_had_success = true,
                    None => {}
                }
                tool_idx += 1;
                continue;
            }
            // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 计划轮只允许
            // blackboard_read + plan_write；其余工具即使被声明也机械拒绝
            // （首轮禁止 执行/变更/shell/子代理/检索/action_write）。
            if plan_round_active && tc.name == crate::planning::PLAN_WRITE_TOOL && plan_write_called
            {
                // P3-5 (2026-08-16): 同一计划轮最多一次 plan_write ——
                // 重填反馈在下一轮注入，同轮第二次提交无意义且会绕过
                // “错误反馈后重填”的交互语义。
                let (_, f) = plan_round_denied(
                    controller,
                    writer,
                    messages,
                    tc,
                    "plan_write_already_submitted",
                    svc.policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                )
                .await?;
                // P2-10 R2 (2026-08-31): plan-round refusal = deny.
                controller.feed_lif_deny(None);
                match f {
                    PolicyFeedback::Denied(key) => round_denials.push(key),
                    PolicyFeedback::Succeeded => round_had_success = true,
                }
                tool_idx += 1;
                continue;
            }
            if plan_gate.is_some()
                && tc.name != crate::planning::PLAN_WRITE_TOOL
                && tc.name != crate::planning::BLACKBOARD_READ_TOOL
            {
                let (_, f) = plan_round_denied(
                    controller,
                    writer,
                    messages,
                    tc,
                    "plan_round_tool_denied",
                    svc.policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                )
                .await?;
                // P2-10 R2 (2026-08-31): plan-round refusal = deny.
                controller.feed_lif_deny(None);
                match f {
                    PolicyFeedback::Denied(key) => round_denials.push(key),
                    PolicyFeedback::Succeeded => round_had_success = true,
                }
                tool_idx += 1;
                continue;
            }
            // 0au：run 墙钟余量保留（预派发拒绝，模板同 D3——无 ToolStarted
            // 的 gate 拒绝；stamp_failure(Refused)；不喂 deny 断路器——保留
            // 不是失败）。本批全部可派发位（合并 leader/被合并/溢出）与
            // **同轮重复位**一并保留（2026-09-20 审查修复批：保留轮的重复
            // 位若照旧走指针回踩，会指向一条未派发回执，构成假指针——保留
            // 判定整批同质，重复位的原调用必被保留，故回执同形）。文案只
            // 报事实（批墙钟/run 剩余/保留额），不重述邀请——同轮重派会被
            // 同一判定再拦。post-batch 间隙的 cause 自述重述见
            // reserved_retrievals。
            if wallclock_reserved
                && (dispatchable.contains(&tool_idx) || duplicate_positions.contains_key(&tool_idx))
            {
                let q = tc
                    .arguments
                    .get("query")
                    .or_else(|| tc.arguments.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(tc.name.as_str())
                    .to_string();
                // 保留＝本调用从未派发，不得占用 query 去重槽（2026-09-20
                // 审查修复批）——槽位若被保留调用认领，后续重发同 query
                // 会得到「已派发过」的指针回踩、指向一条未派发回执。仅在
                // 槽位仍指向本 call_id 时摘除（重复位与更早真实派发的槽位
                // 不在摘除面内）。
                if let Some(key) = crate::retrieval::batch_close::query_key(&tc.arguments) {
                    let owned_by_this_call = if svc.retrieval_calls.is_some() {
                        lane_dispatched_queries
                            .get(&key)
                            .map(|id| id == &tc.call_id)
                    } else {
                        round_dispatched_queries
                            .get(&key)
                            .map(|id| id == &tc.call_id)
                    };
                    if owned_by_this_call == Some(true) {
                        if svc.retrieval_calls.is_some() {
                            lane_dispatched_queries.remove(&key);
                        } else {
                            round_dispatched_queries.remove(&key);
                        }
                    }
                }
                reserved_retrievals.push(q.clone());
                let remaining_secs = remaining_run_wallclock
                    .map(|d| d.as_secs())
                    .unwrap_or_default();
                let output = format!(
                    "[{}] 本调用未派发（cause: {}）：run 剩余墙钟 {}s，不足以容纳本批 \
                     检索墙钟 {}s ＋收尾回合（保留 {}s）与落盘窗口（保留 {}s）。已发起部分 \
                     不受影响。",
                    tc.name,
                    crate::retrieval::batch_close::WALLCLOCK_RESERVED_CAUSE,
                    remaining_secs,
                    batch_wallclock.as_secs(),
                    crate::retrieval::batch_close::CLOSE_ROUND_MARGIN_SECS,
                    crate::retrieval::batch_close::RUN_TAIL_RESERVE_SECS,
                );
                let mut payload = serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "exit_code": 1,
                    "status": "error",
                    "error": crate::retrieval::batch_close::WALLCLOCK_RESERVED_CAUSE,
                });
                // 0q：gate 拒绝形状统一过漏斗（Refused 码，法官对账物齐备）。
                controller.stamp_failure(
                    &mut payload,
                    &tc.name,
                    &tc.arguments,
                    crate::host_exec::ToolFailureOutcome::Refused(
                        crate::retrieval::batch_close::WALLCLOCK_RESERVED_CAUSE,
                    ),
                );
                writer.record(EventType::ToolCompleted, payload).await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: output,
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                tool_idx += 1;
                continue;
            }
            // 0ar S3 前去噪（§5.6）：重复 query 指针回踩——不发起新检索，
            // 以中性回执把已有结果（原 call_id）交回模型；事件面成对
            // ToolStarted+ToolCompleted（exit 0，真实被服务）。
            if let Some(original_call_id) = duplicate_positions.get(&tool_idx) {
                let q = crate::retrieval::batch_close::query_key(&tc.arguments).unwrap_or_default();
                let target_name = match route(&tc.name) {
                    DispatchTarget::InternalRetrieval => "internal_retrieval",
                    _ => "external_retrieval",
                };
                writer
                    .record(
                        EventType::ToolStarted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "target": target_name,
                        }),
                    )
                    .await?;
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "target": target_name,
                            "exit_code": 0,
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: crate::retrieval::batch_close::duplicate_query_note(
                        &q,
                        original_call_id,
                    ),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                tool_idx += 1;
                continue;
            }
            // 0ar S2-D3：被合并调用回执（先于执行分支——本调用不再单独
            // 派发；leader 已按声明序在前位以单激活多 query 执行完毕）。
            // 事件面 ToolStarted+ToolCompleted 沿既有形状（真实被服务），
            // 合并事实在结果文本与合并激活的 query_summary（判据 6）。
            if merged_positions.contains(&tool_idx)
                && dispatchable.first().copied() != Some(tool_idx)
            {
                let q = merged_extra_queries
                    .iter()
                    .find(|(i, _)| *i == tool_idx)
                    .map(|(_, q)| q.clone())
                    .unwrap_or_default();
                let target_name = match route(&tc.name) {
                    DispatchTarget::InternalRetrieval => "internal_retrieval",
                    _ => "external_retrieval",
                };
                let note = format!(
                    "[{}] 本调用已与本轮首次检索调用合并为同一检索激活（合并上限 {}）；\
                     本 query（\"{}\"）已由该激活统一执行，结果见其合并返回与 \
                     query_summary 对应条目。",
                    tc.name,
                    crate::retrieval::batch_close::MERGE_MAX_QUERIES,
                    q
                );
                writer
                    .record(
                        EventType::ToolStarted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "target": target_name,
                        }),
                    )
                    .await?;
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "target": target_name,
                            "exit_code": 0,
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: note.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                tool_idx += 1;
                continue;
            }
            // 0ar S2-D3：溢出未派发拒绝（§5.4 R-1 兜底／§5.5 中性文案）。
            // 无 ToolStarted 的 gate 拒绝（模板同族 refuse_inject_budget——
            // F11 gate 段允许无起点的拒绝完成事件）；不喂 deny 断路器
            // （推迟不是失败，一次性重述在 post-batch 间隙注入）。
            if deferred_positions.contains(&tool_idx) {
                let q = tc
                    .arguments
                    .get("query")
                    .or_else(|| tc.arguments.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(tc.name.as_str())
                    .to_string();
                deferred_retrievals.push(q.clone());
                let leader_q = dispatchable
                    .first()
                    .and_then(|&i| {
                        response.tool_calls[i]
                            .arguments
                            .get("query")
                            .or_else(|| response.tool_calls[i].arguments.get("url"))
                            .and_then(|v| v.as_str())
                    })
                    .unwrap_or_default();
                let output = format!(
                    "[{}] 本轮已派发 1 个检索激活（query: \"{}\"，合并上限 {}）；\
                     本调用未派发（cause: {}，deferred_call_id: {}）。",
                    tc.name,
                    leader_q,
                    crate::retrieval::batch_close::MERGE_MAX_QUERIES,
                    crate::retrieval::batch_close::DEFERRED_CAUSE,
                    tc.call_id
                );
                let mut payload = serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "exit_code": 1,
                    "status": "error",
                    "error": crate::retrieval::batch_close::DEFERRED_CAUSE,
                });
                // 0q：gate 拒绝形状统一过漏斗（Refused 码，法官对账物齐备）。
                controller.stamp_failure(
                    &mut payload,
                    &tc.name,
                    &tc.arguments,
                    crate::host_exec::ToolFailureOutcome::Refused(
                        crate::retrieval::batch_close::DEFERRED_CAUSE,
                    ),
                );
                writer.record(EventType::ToolCompleted, payload).await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: output,
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                tool_idx += 1;
                continue;
            }
            let target = route(&tc.name);
            // C2-1 (2026-08-11, ADR-0006 web-search slice): lane
            // self-execution — inside a retrieval lane, web tools (routed
            // ExternalRetrieval by relay) execute through the host instead
            // of dispatching a nested retrieval: the retrieval lane IS the
            // web lane (ADR-0010 §3.7.8 — the delegated search must
            // actually run). The nested-dispatch refusal keeps its
            // anti-recursion meaning for `retrieve_project_*`. The host
            // path preserves the write gate, the semaphore, the evidence
            // ledger and the enable gate; the permission bridge is skipped
            // — the explicit retrieval enable gate (ADR-0010 §14.65) is the
            // authorization chain (2026-08-11 user adjudication; 0t 后三值
            // 模式状态机退役，仅独立启用门；the main lane's delegated path
            // has no per-call gate either).
            let lane_self_execute = target == DispatchTarget::ExternalRetrieval
                && profile.tool_filter.denies_nested_dispatch();
            let effective = if lane_self_execute {
                DispatchTarget::Host
            } else {
                target.clone()
            };
            let mut round_feedback: Option<PolicyFeedback> = None;
            let result = match effective {
                DispatchTarget::InternalRetrieval | DispatchTarget::ExternalRetrieval => {
                    if profile.tool_filter.denies_nested_dispatch() {
                        // One seat per role — a retrieval lane never
                        // dispatches another retrieval (ADR-0010 §11.3).
                        let (r, f) = role_gate_denied(
                            controller,
                            writer,
                            messages,
                            tc,
                            profile.role.as_str(),
                            "nested_subagent_dispatch_refused",
                            svc.policy_revision
                                .load(std::sync::atomic::Ordering::SeqCst),
                        )
                        .await?;
                        // P2-10 R2 (2026-08-31): role-gate refusal = deny.
                        controller.feed_lif_deny(None);
                        round_feedback = Some(f);
                        r
                    } else {
                        // 0ar S2-D3 合并优先：leader 携带同轮其余被合并
                        // query（单激活多任务；结果 query_summary 逐 query
                        // 一条）。非 leader 位不会进入本分支（上方已回执）。
                        let merged_queries: Vec<String> =
                            if dispatch_bound.first() == Some(&tool_idx) {
                                merged_extra_queries
                                    .iter()
                                    .map(|(_, q)| q.clone())
                                    .collect()
                            } else {
                                Vec::new()
                            };
                        controller
                            .run_retrieval_subagent(
                                host,
                                writer,
                                run_gateway,
                                target.clone(),
                                tc,
                                messages,
                                prompt,
                                &current_tool_defs,
                                orientation.as_deref_mut(),
                                cancel,
                                heartbeat,
                                &merged_queries,
                            )
                            .await?
                    }
                }
                DispatchTarget::ParentDisposition => {
                    // M4: the parent's structured disposition control tool.
                    // The subagent's projection strips it; the gate below is
                    // belt-and-braces for scripted lanes.
                    if profile.tool_filter.denies_nested_dispatch() {
                        let (r, f) = role_gate_denied(
                            controller,
                            writer,
                            messages,
                            tc,
                            profile.role.as_str(),
                            "control_tool_lane_denied",
                            svc.policy_revision
                                .load(std::sync::atomic::Ordering::SeqCst),
                        )
                        .await?;
                        // P2-10 R2 (2026-08-31): role-gate refusal = deny.
                        controller.feed_lif_deny(None);
                        round_feedback = Some(f);
                        r
                    } else {
                        controller
                            .handle_parent_disposition(writer, tc, messages)
                            .await?
                    }
                }
                DispatchTarget::Host => {
                    if let Some(reason) = profile.tool_filter.write_gate(&tc.name) {
                        // ADR-0010 §3.2 deny-only write domain: refused
                        // BEFORE the host's permission bridge (a policy
                        // refusal, not a user choice — the permission
                        // dialog is never consulted).
                        let (r, f) = role_gate_denied(
                            controller,
                            writer,
                            messages,
                            tc,
                            profile.role.as_str(),
                            reason,
                            svc.policy_revision
                                .load(std::sync::atomic::Ordering::SeqCst),
                        )
                        .await?;
                        // P2-10 R2 (2026-08-31): role-gate refusal = deny.
                        controller.feed_lif_deny(None);
                        round_feedback = Some(f);
                        r
                    } else {
                        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱):
                        // direct 模式直接动作——创建 console trace（trace_id
                        // 供事件盖章与 step_done 证据），盖章传入事件链。
                        let direct_ctx = if controller.console_default_enabled()
                            && profile.role == AgentRole::Main
                        {
                            controller.console_direct_begin(&tc.call_id)
                        } else {
                            None
                        };
                        // 0k 审查处理 (P3-4)：in-flight 工具入槽——子代理
                        // 墙钟超时 drop loop future 时，dispatch 据此为链上
                        // 孤儿 ToolStarted 补 ToolCompleted(error)。工具
                        // 结果返回后立即移除；中途 Err 传播时残留（该调用
                        // 确实处于 in-flight 中断态，超时收口补事件是正确
                        // 语义；不超时时派发结束槽即丢弃）。
                        if let Some(slot) = svc.in_flight_tools {
                            slot.lock()
                                .unwrap()
                                .push((tc.name.clone(), tc.call_id.clone()));
                        }
                        // 0ar S2：检索族调用发起即计数（发起≠成功）。
                        if crate::retrieval::batch_close::is_lane_retrieval_tool(&tc.name)
                            && let Some(counter) = svc.retrieval_calls
                        {
                            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        let (result, feedback) = controller
                            .run_host_tool_with_plan_gate(
                                host,
                                writer,
                                tc,
                                prompt,
                                workspace_trust,
                                messages,
                                None,
                                tool_rounds,
                                heartbeat,
                                // ACAF Slice 2 D-13 (2026-08-13): the lane's
                                // real activation — bound on web_fetch etc.
                                profile.activation_id.as_deref(),
                                // FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14):
                                // the dispatch's web_fetch candidate counter
                                // (None on main/grill — fails the gate closed).
                                profile.fetch_candidates.as_deref(),
                                // P2-4 (2026-09-10): the lane's SERP
                                // engine-navigation budget (per run on
                                // main/grill, per activation on the external
                                // retrieval lane).
                                profile.serp_budget.as_deref(),
                                // C2-1 (2026-08-11): lane self-execution skips
                                // the per-call permission bridge — the mode
                                // gate is the authorization chain (see the
                                // lane_self_execute comment above).
                                !lane_self_execute,
                                // P0-A step 5 review fix: only lanes that
                                // probe work tools (main/grill) write call
                                // failures back into the probe map.
                                profile.probe_work_tools,
                                // PLAN-FIRST 阶段 A (2026-08-16): the plan
                                // gate's current submission attempt — the
                                // plan_write handler decides refill vs
                                // degrade from it.
                                plan_gate.as_ref().map(|g| g.attempt),
                                direct_ctx.as_ref().map(|(stamp, _)| stamp),
                            )
                            .await?;
                        if let Some(slot) = svc.in_flight_tools {
                            let mut guard = slot.lock().unwrap();
                            if let Some(pos) = guard.iter().position(|(_, id)| id == &tc.call_id) {
                                guard.remove(pos);
                            }
                        }
                        // direct 动作 trace 收口（commit + 证据面登记）。
                        if let Some((stamp, trace)) = direct_ctx {
                            let detail = if result.exit_code == Some(0) {
                                None
                            } else {
                                Some(result.output.as_str())
                            };
                            controller.console_direct_end(
                                &stamp,
                                trace,
                                &tc.name,
                                result.exit_code == Some(0),
                                detail,
                            );
                        }
                        // GAP-RETRIEVAL-TOOLS (2026-08-10): evidence
                        // collection for the retrieval lanes — the
                        // mechanical source of the structured result's
                        // ledger (§3.7.4). Main lane: `None`.
                        if let Some(evidence) = svc.evidence
                            && let Some(record) = crate::retrieval::evidence::build_evidence_record(
                                &tc.name, tc, &result,
                            )
                        {
                            evidence.lock().unwrap().push(record);
                        }
                        // 0ar S2：可见倒数行（设计 §3.6）——检索族结果尾部
                        // 机械追加（本批证据快照＋已发起调用数）。
                        if svc.retrieval_calls.is_some()
                            && crate::retrieval::batch_close::is_lane_retrieval_tool(&tc.name)
                        {
                            let snapshot = svc
                                .evidence
                                .map(|e| e.lock().unwrap().clone())
                                .unwrap_or_default();
                            let calls = svc
                                .retrieval_calls
                                .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                                .unwrap_or(0);
                            crate::retrieval::batch_close::append_countdown_to_tool_message(
                                messages,
                                &tc.call_id,
                                &snapshot,
                                calls,
                            );
                        }
                        round_feedback = feedback;
                        result
                    }
                }
            };
            // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 更新计划门
            // 状态——accepted/degraded 解除门，refill_requested 进入第二次
            // （最后一次）提交机会。
            if tc.name == crate::planning::PLAN_WRITE_TOOL {
                plan_write_called = true;
                if let Some(outcome) = result
                    .structured
                    .as_ref()
                    .and_then(|s| s.get("outcome"))
                    .and_then(serde_json::Value::as_str)
                {
                    match outcome {
                        "accepted" | "degraded" => plan_gate = None,
                        "refill_requested" => {
                            if let Some(g) = plan_gate.as_mut() {
                                g.attempt += 1;
                                g.rounds_without_submission = 0;
                            }
                        }
                        _ => {}
                    }
                }
            }
            // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): 静默
            // 审查记录——只记机械事实、不给建议；每对象键仅最后一轮结果
            // 覆盖写；覆盖写动作以轻量 `mechanical_audit_update` 事件留痕。
            // 检索派发带激活候选计数（候选/上限）。
            // 主车道专属：检索子代理车道不记（事件面留痕仅主面，检索面
            // 事件序列保持既有形态）。
            if profile.role == AgentRole::Main {
                crate::mechanical_audit::record_tool_result(
                    &mut mechanical_audit,
                    writer,
                    tc,
                    &result,
                    tool_rounds,
                    controller.retrieval_candidate_count(),
                )
                .await?;
            }
            // plan 对象键：首轮计划门结果（accepted/degraded、步骤数）。
            if tc.name == crate::planning::PLAN_WRITE_TOOL
                && let Some(outcome) = result
                    .structured
                    .as_ref()
                    .and_then(|s| s.get("outcome"))
                    .and_then(serde_json::Value::as_str)
                && matches!(outcome, "accepted" | "degraded")
            {
                let step_count = svc.blackboard.read().plan.steps.len();
                let epoch = svc.blackboard.read().plan.plan_epoch;
                let payload = mechanical_audit.record(
                    "plan",
                    tool_rounds,
                    format!("plan {outcome}（{step_count} 步，epoch {epoch}）"),
                    None,
                );
                writer
                    .record(
                        EventType::MechanicalAuditUpdate,
                        serde_json::json!({
                            "kind": crate::mechanical_audit::KIND_PLAN_GATE,
                            "payload": payload
                        }),
                    )
                    .await?;
            }
            match round_feedback {
                Some(PolicyFeedback::Denied(key)) => round_denials.push(key),
                // A successful call resets the breaker; None
                // (timeout / tool error) is neutral — it neither
                // resets nor counts (ADR-0010 §3.5.4 分开记账).
                Some(PolicyFeedback::Succeeded) => round_had_success = true,
                None => {}
            }
            // 0ac S3①-b ⑥（2026-09-15，设计 §7 风险 6）：检索子代理提前
            // 收口计数——仅检索车道。`capability_unreachable` = 确定性
            // 不可达，一次即收口；`network_no_response` 连续达到阈值
            // （ORZ_RETRIEVAL_EARLY_CLOSE_FAILURES，默认 3，0=禁用）收口。
            // 0AC-A2（2026-09-15 深审修复）：`empty_result` 不再计入连续
            // 失联——它是通道判活的合法 I1 信息（设计 §3.4：引擎链合法
            // 空转即产出该码），计入会把「查无可得」过早杀成
            // `subagent_failed`（2×失联＋1×合法空也达阈值）。成功清零；
            // 其他失败码中性（不计不清，与 §3.5.4 分开记账同纪律）。
            // 中止判定在 post-tool-batch 间隙（本批全部 tool replies 已
            // 回传后，协议形态完整）。
            if profile.role != AgentRole::Main && crate::relay::is_web_retrieval_tool(&tc.name) {
                if result.exit_code == Some(0) {
                    retrieval_failure_streak = 0;
                } else if let Some(code) =
                    crate::immediate_delivery::stable_code_from_error(&result.output)
                {
                    if code == "capability_unreachable" {
                        retrieval_early_close =
                            Some(format!("deterministic retrieval failure: {code}"));
                    } else if code == "network_no_response" {
                        retrieval_failure_streak += 1;
                        if crate::immediate_delivery::early_close_failure_limit() > 0
                            && u64::from(retrieval_failure_streak)
                                >= crate::immediate_delivery::early_close_failure_limit()
                        {
                            retrieval_early_close = Some(format!(
                                "consecutive deterministic retrieval failures: \
                                 {retrieval_failure_streak} (last: {code})"
                            ));
                        }
                    }
                }
            }
            // Count this result against the per-round injection budget (the
            // same `[tool] output` text the model receives).
            round_inject_tokens =
                round_inject_tokens.saturating_add(estimate_message_tokens(&Message {
                    role: Role::Tool,
                    content: format!("[{}] {}", tc.name, result.output),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                }));

            // GAP-INQUIRY-SPLIT (2026-08-09): the old per-tool-call
            // counter feeds are deleted — `tool_calls` / `tool_variety`
            // are not orientation 判定点 (§4.2) and subagent output
            // repetition is handled by the generation-time output-health
            // guard. The main lane's orientation round count happens at
            // the model-round completion point (one completed logical
            // model round counts 1 regardless of tool-call count).
            tool_idx += 1;
        }
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 一轮工具轮未写
        // plan_write（仅 blackboard_read 等）→ 无提交计数；达到上限机械
        // 降级放行（plan_not_submitted 留痕，不挂死）。
        if let Some(gate) = plan_gate.as_mut()
            && !plan_write_called
        {
            gate.rounds_without_submission += 1;
            if gate.rounds_without_submission >= crate::planning::MAX_PLAN_ROUNDS_WITHOUT_SUBMISSION
            {
                let g = plan_gate.take().expect("checked above");
                writer
                    .record(
                        EventType::PlanWrite,
                        crate::planning::plan_write_payload(
                            "",
                            "",
                            0,
                            "degraded",
                            g.attempt,
                            &crate::planning::PlanVerdict::default(),
                            Some("plan_not_submitted"),
                        ),
                    )
                    .await?;
            }
        }
        // Post-tool-batch injections — AFTER every tool reply of this
        // round, so no user message breaks the assistant-declaration →
        // tool-replies sequence (provider protocol; 2026-08-07 review
        // P1/P2). Semantics are unchanged: the neutral inquiry fires at
        // most once per round (counters reset on trigger), so hoisting
        // it out of the per-tool loop is equivalent.
        // ADR-0010 §3.5.4 round-level denial aggregation — extracted to
        // [`aggregate_denial_round`] (2026-08-12, behaviour unchanged): a
        // round with any success or with denials that do NOT all share one
        // normalized key resets the count; otherwise the round counts, and
        // at 3 consecutive same-key rounds the breaker message fires once
        // (count restarts).
        {
            let mut denial = svc.denial_state.lock().unwrap();
            if let Some(tool_name) =
                aggregate_denial_round(&mut denial, &round_denials, round_had_success)
            {
                pending_policy.push(Message {
                    role: Role::User,
                    content: crate::prompt::tool_policy_breaker_block(
                        &tool_name,
                        DENIAL_BREAKER_CONSECUTIVE,
                    ),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
            }
        }
        for pm in pending_policy {
            messages.push(pm);
        }
        // 2026-08-08 blackboard partition (A2): incremental push — the
        // edits this round actually made, replayed as ONE compact user
        // message right after the tool batch (mechanical, deterministic;
        // the model sees "本轮发生了什么" — the delta, never the whole
        // blackboard). Entries before this round were pushed on their
        // own rounds and are already in history; the blackboard keeps
        // the full list for blackboard_read look-backs.
        let round_edits = svc.blackboard.read().edits.clone();
        // `round_edit_count` was the section length BEFORE this round's
        // tools — it IS the start index of this round's records.
        let edits_start = round_edit_count.min(round_edits.len());
        let round_summary: Vec<String> = round_edits[edits_start..]
            .iter()
            .map(format_edit_record)
            .collect();
        if !round_summary.is_empty() {
            messages.push(Message {
                role: Role::User,
                content: format!("[本轮编辑] {}", round_summary.join("；")),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
        }
        // 0ar S2-D3（§5.5）：一次性重述——本轮有溢出未派发检索时，在
        // post-batch 间隙注入一条中性事实（每轮至多一条，不跨轮累积），
        // 防模型漏看丢覆盖；不教学（只报事实，不指导拆分）。
        if !deferred_retrievals.is_empty() {
            let note = format!(
                "[上轮检索未派发] 上一轮有 {} 次检索未派发（cause: {}）：{}。",
                deferred_retrievals.len(),
                crate::retrieval::batch_close::DEFERRED_CAUSE,
                deferred_retrievals
                    .iter()
                    .map(|q| format!("\"{q}\""))
                    .collect::<Vec<_>>()
                    .join("、")
            );
            messages.push(Message {
                role: Role::User,
                content: note,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            deferred_retrievals.clear();
        }
        // 0au：run 墙钟保留的一次性重述（cause 自描述；只报事实——尾部
        // 保留窗内的重派会被同一判定再拦，文案不做重派邀请）。
        if !reserved_retrievals.is_empty() {
            let note = format!(
                "[上轮检索未派发] 上一轮有 {} 次检索因 run 墙钟余量不足未派发 \
                （cause: {}）：{}。run 剩余墙钟不足一个完整检索批，尾部保留给落盘。",
                reserved_retrievals.len(),
                crate::retrieval::batch_close::WALLCLOCK_RESERVED_CAUSE,
                reserved_retrievals
                    .iter()
                    .map(|q| format!("\"{q}\""))
                    .collect::<Vec<_>>()
                    .join("、")
            );
            messages.push(Message {
                role: Role::User,
                content: note,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            reserved_retrievals.clear();
        }
        // 0ar S2-D1（2026-09-19，设计 §3.1/§3.2/§3.4）：阈值回送判定——
        // post-tool-batch gap（本批全部 tool replies 已回传的安全间隙）。
        // 宽口径计数（§3.3 共用尺）达标即进入 β 收尾：下一轮工具面机械
        // 收空、唯一收尾回合产出结果总结。护栏（10）优先于阈值（5）——
        // 同一 β 形态、成因不同（terminal_reason 同为
        // evidence_threshold_met，assessment reason_codes 区分）。
        if !close_round_armed
            && batch_close.is_none()
            && profile.role != AgentRole::Main
            && svc.retrieval_calls.is_some()
        {
            let snapshot = svc
                .evidence
                .map(|e| e.lock().unwrap().clone())
                .unwrap_or_default();
            let usable = crate::retrieval::batch_close::usable_source_count(&snapshot);
            let kind = crate::retrieval::batch_close::should_arm_close(usable);
            if let Some(kind) = kind {
                messages.push(Message {
                    role: Role::User,
                    content: crate::retrieval::batch_close::close_round_block(kind, usable),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                batch_close = Some(kind);
                close_round_armed = true;
            }
        }
        // 0ac S3①-b ⑥（2026-09-15）：提前收口中止判定——本批 tool
        // replies 已全部回传、协议形态完整处退出；父侧 dispatch 以
        // `subagent_failed` 收口，cause 随错误文本自描述（wallclock
        // 只作最后兜底）。
        // 0ar S2：阈值/护栏已武装时不再走失败臂——本批按正常收尾交回
        //（确定性失败事实已在链上工具事件留痕，不因之丢掉已达标证据）。
        if let Some(cause) = retrieval_early_close.take() {
            if !close_round_armed {
                return Err(AgentLoopError::RetrievalSubagentEarlyClose(cause));
            }
            tracing::warn!(cause, "retrieval early close suppressed by batch close");
        }
        // 0ac S3①-b M2 B1（2026-09-15，设计 §2.1/§4.1「合法边界投递」）：
        // 全部 tool replies 之后的合法间隙 drain 宿侧后台任务完成事实，
        // 到达即投——真投递落 `result_delivered`（suppressed=false、
        // boundary=B1_tool_result）+ 一条中性事实消息（与 pending_policy /
        // [本轮编辑] 同间隙，不插进 assistant 声明与 tool replies 之间）。
        // 主开关关 ⇒ 不 drain、零事件、零行为变化。
        if crate::immediate_delivery::switch_enabled() {
            let facts = host.drain_completed_tasks().await;
            if !facts.is_empty() {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                for f in facts {
                    delivery_queue.admit(crate::immediate_delivery::PendingFact::background_task(
                        &f.task_id,
                        f.report,
                        now_ms,
                        u64::from(tool_rounds),
                    ));
                }
            }
            if !delivery_queue.is_empty() {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let deliveries = delivery_queue.due(u64::from(tool_rounds), now_ms);
                for d in &deliveries {
                    writer
                        .record(
                            EventType::ResultDelivered,
                            crate::immediate_delivery::delivered_fact_payload(
                                d,
                                crate::immediate_delivery::BOUNDARY_B1,
                            ),
                        )
                        .await?;
                }
                messages.push(Message {
                    role: Role::User,
                    content: format!(
                        "[结果投递] {}",
                        crate::immediate_delivery::render_delivery_message(&deliveries)
                    ),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
            }
        }
        // GAP-INQUIRY-SPLIT (2026-08-09) — MAIN orientation injection
        // point: the post-tool-batch gap (a safe action gap: the tool
        // results are in, the next generate has not started). The fired
        // block rides into the next generate, which is a soft-gate round
        // (THIN-HARNESS-REDESIGN-V2 §9.2): 纯文本回答被消费、loop 续跑；
        // 工具调用照常执行。When this is the budget-exhausting round,
        // the checkpoint round runs first and the post-budget final round
        // reports the partial result after it (§14.16).
        // P0-0x S1（ADR-0010 §14.66）：本间隙同时是**初始轮中立问询**的
        // 触发点——主车道首个含工具调用的动作批次结束时，`maybe_fire_orientation`
        // 优先派发一次性初始轮 record（会话内恰好一次）。
        let mut fired_initial_round = false;
        if pending_checkpoint.is_none()
            && let Some(role) = profile.orientation_role
            && let Some(record) = controller
                // THIN-HARNESS-REDESIGN-V2 §9.2: 同 loop-top——软门模式
                // （fire 延迟 commit + pending 软消费）。
                .maybe_fire_orientation(
                    writer,
                    messages,
                    orientation.as_deref_mut(),
                    role,
                    ORIENTATION_POST_TOOL_BATCH_GAP,
                )
                .await?
        {
            // 0ae D1（2026-09-15，设计 §4，用户已同意方向）：初始轮问询
            // 追加一问——引导把工作计划/关键中间结论写入黑板（复用 0x
            // 注入机制与间隙，0x 模板块本身不改——签名模板 sha256 面不动）。
            fired_initial_round = record.is_initial_round();
            pending_checkpoint = Some(PendingCheckpoint::Orientation { record });
        }
        if fired_initial_round {
            messages.push(Message {
                role: Role::User,
                content: format!(
                    "[工作台] 请将本任务的工作计划与关键中间结论写入黑板（{BLACKBOARD_WRITE_TOOL_NAME} section=plan|notes）；黑板不受上下文折叠与机械压缩影响，硬截断（500K 估算）后仍可经 blackboard_read 找回。"
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            // 0AE-C2：payload 收敛为审查表条目形状（key/round/summary/anomaly）。
            let payload = mechanical_audit.record(
                "plan_write_guidance",
                tool_rounds,
                "trigger=initial_round",
                None,
            );
            writer
                .record(
                    EventType::MechanicalAuditUpdate,
                    serde_json::json!({
                        "kind": crate::mechanical_audit::KIND_PLAN_WRITE_GUIDANCE,
                        "payload": payload,
                    }),
                )
                .await?;
        }
        // 0ae D1 补救规则（DP-2）：至第 N=20 轮仍无任何 blackboard_write
        // ⇒ 再提醒一次；此后不再提醒、不设硬门。
        if pending_checkpoint.is_none()
            && profile.role == AgentRole::Main
            && !plan_reminder_done
            && tool_rounds >= PLAN_WRITE_REMINDER_ROUND
            && svc.blackboard.read().model_note_count() == 0
        {
            plan_reminder_done = true;
            messages.push(Message {
                role: Role::User,
                content: format!(
                    "[工作台] 已 {} 轮未写入黑板：请把工作计划与关键中间结论固化到黑板（{BLACKBOARD_WRITE_TOOL_NAME} section=plan|notes）。此为唯一一次提醒。",
                    PLAN_WRITE_REMINDER_ROUND
                ),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            // 0AE-C2：payload 收敛为审查表条目形状；键区分引导与提醒
            // （schema 描述的键面：plan_write_guidance / plan_write_reminder）。
            let payload = mechanical_audit.record(
                "plan_write_reminder",
                tool_rounds,
                format!(
                    "trigger=n_round_reminder round_threshold={PLAN_WRITE_REMINDER_ROUND} note_count=0"
                ),
                None,
            );
            writer
                .record(
                    EventType::MechanicalAuditUpdate,
                    serde_json::json!({
                        "kind": crate::mechanical_audit::KIND_PLAN_WRITE_GUIDANCE,
                        "payload": payload,
                    }),
                )
                .await?;
        }
        // P0-C orz 内嵌集成 S2 (2026-08-15): 轮末机械发放——动作栏有未消费
        // 订单时在 post-tool-batch 安全间隙发放（副作用只发生在单一出口；
        // 模型面只有读板块 + 写订单）。pending checkpoint 优先级：本间隙
        // 已有 checkpoint 待轮时跳过发放，订单留在槽中；下一工具轮发放时
        // round 不匹配会按 `order_stale` 显式拒绝（防重放/过期，fail-closed）。
        let console_consumed = if pending_checkpoint.is_none() && profile.role == AgentRole::Main {
            controller
                .issue_pending_console_order(
                    host,
                    writer,
                    prompt,
                    workspace_trust,
                    tool_rounds,
                    heartbeat,
                )
                .await?
        } else {
            0
        };
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.3):
        // 双模式显式询问轮触发——发放后若 console 态连续故障 ≥ 阈值且本
        // run 未问过，设置询问 checkpoint（无工具轮；优先级低于
        // orientation——上方 fire 先占位）。询问回答（switch/stay）在
        // 下一轮 checkpoint 分支处理。
        if pending_checkpoint.is_none()
            && profile.role == AgentRole::Main
            && controller.console_inquiry_due()
        {
            let (streak, order_ids) = controller.console_streak_snapshot();
            pending_checkpoint = Some(PendingCheckpoint::ConsoleModeInquiry {
                attempt: 1,
                streak,
                order_ids,
            });
            // 询问轮为无工具轮——注入模板块（模型只回答 decision/reason）。
            messages.push(Message {
                role: Role::User,
                content: crate::console_mode::inquiry_template_block(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
        }

        // P0-C S4 (2026-08-16): 发放的 console 动作按实际执行单位计入同一
        // tool-round 预算（直接订单 1、脚本每步 1）——剩余预算按需经
        // `blackboard_read section=session` 读取（PUSH→PULL 2026-08-21，
        // 每轮 REMAINING 尾随注入已退役）；耗尽后同样进入最后无工具轮
        // 并结束。
        tool_rounds = tool_rounds.saturating_add(console_consumed);
        if !plan_round_active {
            tool_rounds += 1;
        }
        // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39)：预算键
        // 每轮末覆盖写（轮数；墙钟在报告时取 elapsed）。
        if profile.role == AgentRole::Main {
            let budget_payload =
                mechanical_audit.record_budget(tool_rounds, tool_rounds, profile.max_tool_rounds);
            writer
                .record(
                    EventType::MechanicalAuditUpdate,
                    serde_json::json!({
                        "kind": crate::mechanical_audit::KIND_BUDGET,
                        "payload": budget_payload
                    }),
                )
                .await?;
            // 0bg S2（2026-09-22，双迁移定案「连带记录」）：LIF 域迁移事实
            // ——模型面「域迁移+n」徽章撤除后，机械层连带留痕（只记不发
            // 模型；每键一条覆盖写，逐次历史由 journal 事件流可离线复算）。
            for (from, to, at_round, cumulative) in controller.take_new_lif_migrations() {
                let payload = mechanical_audit.record(
                    "lif.domain_migration",
                    at_round.min(u32::MAX as u64) as u32,
                    format!(
                        "{}→{}@r{at_round}；累计 {cumulative} 次",
                        from.as_str(),
                        to.as_str()
                    ),
                    None,
                );
                writer
                    .record(
                        EventType::MechanicalAuditUpdate,
                        serde_json::json!({
                            "kind": crate::mechanical_audit::KIND_LIF_DOMAIN,
                            "payload": payload
                        }),
                    )
                    .await?;
            }
        }
        // TER T1.7 (2026-09-04)：`max_tool_rounds == 0` = 默认无硬限——
        // 只有显式配置非零上限时才挂载轮数闸（escape hatch 语义）。
        if profile.max_tool_rounds > 0 && tool_rounds >= profile.max_tool_rounds {
            // Anti-runaway backstop — mark the truncation so the journal
            // records why pending tool calls were dropped. D-8: the cap
            // is a backstop, not a target — the run gets ONE final
            // no-tools round to report its partial result, then ends.
            writer
                .record(
                    EventType::GateDecision,
                    serde_json::json!({
                        "gate": "tool_rounds_limit",
                        "decision": "stop",
                        "reason": "max_tool_rounds_reached",
                        "tool_rounds": tool_rounds,
                        "max_tool_rounds": profile.max_tool_rounds,
                    }),
                )
                .await?;
            messages.push(Message {
                role: Role::User,
                content: crate::prompt::tool_round_budget_exhaustion_block(profile.max_tool_rounds),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            budget_exhausted = true;
        }
    }

    // 0ae D3 窄边沿修复（2026-09-15）：run 尾安全网——前两条 break 路径
    // 之外的未知正常出口若仍携带未收口窗口状态，在此兜底落账（take 幂等，
    // 已收口时为空操作）。
    finalize_model_compression_close(
        &mut model_compression_close,
        &mut mechanical_audit,
        svc,
        writer,
        tool_rounds,
    )
    .await?;
    // v7 修正批（2026-09-15，审查 P1②）× v8（2026-09-16 勘误批）：
    // **终止轮的语义摘要仍必须在此消费**——run 尾是安全间隙（模型已离场、无
    // 工具在途、无后续请求）。v8 下消费形态改为**按块压缩**（不再 drain 会话
    // 本体）：压得动 ⇒ 落 `mode=model_summary` 事件、被压块从模型面下移
    // （本地面逐字保留）；压不动（无可压闭合块 / 块区间非法）⇒ 如实 NoOp，
    // 不虚构事件。
    // 边界：`?` 错误传播路径仍会丢弃该状态（与 0AC-A6 Err 留痕同类，挂账）。
    if let Some((summary, summary_window)) = pending_semantic.take() {
        // 0ap：归因口径与 loop-top 主消费点一致——取摘要产出时记录的窗口种类
        // （复核批 P2：不再回看旗标）。
        let semantic_reason = semantic_compression_reason(summary_window);
        // run 尾重建投影入参（loop 内的 `face_params` 已出作用域；D4 机械段
        // 允许按 epoch 重渲——此处已无后续请求，代价为零）。
        let face_params = crate::model_face::ModelFaceParams {
            slider_tokens: svc.context_compact.slider_window_tokens,
            block_tokens: svc.context_compact.model_face_block_tokens,
            ledger_path: face_ledger_path.clone(),
            archive_tag: Some(face_archive_tag.clone()),
            run_id: writer.run_id().to_string(),
            d4_block: face_d4_block.clone(),
            static_overhead_tokens: face_static_overhead,
        };
        let selection = crate::context_scale::extract_block_selection(&summary);
        let locators = crate::summary::LocatorPointers {
            ledger_seq: ledger_seq_epoch,
            ledger_path: face_ledger_path.as_ref().map(|p| p.display().to_string()),
            journal_run: Some(writer.run_id().to_string()),
            journal_seq: Some((journal_epoch_start_seq, writer.seq())),
            conversation_path: conversation_sidecar_path.clone(),
            local_face_full: false,
        };
        if let Some(outcome) = compress_blocks_now(
            svc,
            writer,
            host,
            messages,
            &face_params,
            &face_archive_tag,
            selection,
            Some(&summary),
            semantic_reason,
            &locators,
        )
        .await?
        {
            tracing::debug!(
                blocks = outcome.blocks,
                freed_tokens = outcome.freed_tokens,
                "v8 block compaction executed at the run tail"
            );
            rounds_since_compact = 0;
        }
    }
    // 0ac S3①-b（2026-09-15，设计 §4.2「不跨 run 存活」）：run 尾关闭
    // 队列——B1 投递在同一间隙 drain+due，正常路径恒空；此处的 close_drop
    // 是安全网（非空 = 异常残留，warn 留痕后丢弃，绝不跨 run 泄漏）。
    let dropped = delivery_queue.close_drop();
    if !dropped.is_empty() {
        tracing::warn!(
            ids = ?dropped.iter().map(|f| f.source_id.clone()).collect::<Vec<_>>(),
            "delivery queue non-empty at run end — dropped (per-run lifecycle)"
        );
    }

    Ok(LoopOutcome {
        last_text,
        tool_rounds,
        rounds_since_compact,
        budget_exhausted,
        // 0ar S2：检索批次收尾成因（主车道恒 None）＋已发起检索调用数。
        retrieval_close: batch_close,
        retrieval_calls: svc
            .retrieval_calls
            .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0),
    })
}

/// ADR-0010 §3.5.4 round-level denial aggregation (extracted from the
/// post-tool-batch injection point, 2026-08-12 — behaviour unchanged): a
/// round with any success, or whose denials do NOT all share one normalized
/// key, resets the count; otherwise the round counts against `last_key`,
/// and at `DENIAL_BREAKER_CONSECUTIVE` consecutive same-key rounds returns
/// the offending tool name (the caller injects the strategy-switch message
/// once) with the count restarted. The key includes `policy_revision` — a
/// policy bump is a key change, so the reset path is structurally
/// reachable (GAP-DENIAL-POLICY-REVISION, 2026-08-12).
pub(crate) fn aggregate_denial_round(
    denial: &mut DenialState,
    round_denials: &[DenialKey],
    round_had_success: bool,
) -> Option<String> {
    let all_same_key = round_denials
        .first()
        .is_some_and(|k0| round_denials.iter().all(|k| k == k0));
    if round_had_success || !all_same_key {
        denial.consecutive_rounds = 0;
        denial.last_key = None;
        None
    } else if let Some(key) = round_denials.first() {
        if denial.last_key.as_ref() == Some(key) {
            denial.consecutive_rounds += 1;
        } else {
            denial.consecutive_rounds = 1;
            denial.last_key = Some(key.clone());
        }
        if denial.consecutive_rounds >= DENIAL_BREAKER_CONSECUTIVE {
            denial.consecutive_rounds = 0; // injected once per burst
            Some(key.tool_name.clone())
        } else {
            None
        }
    } else {
        None
    }
}

/// Structured refusal for a role-gated tool (ADR-0010 §3.2 deny-only write
/// domain / §11.3 nested dispatch guard): the call is journaled as
/// ToolStarted → ToolCompleted(status=error) — a refused call is visible in
/// the audit chain — and replayed as a Tool message (provider protocol:
/// every declared call is answered; 2026-08-06 polyglot probe). The denial
/// key feeds the shared 3-round breaker via the round-level aggregation.
async fn role_gate_denied(
    controller: &AgentLoopController,
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
    tc: &ToolCall,
    target: &str,
    reason: &str,
    policy_revision: u64,
) -> Result<(ToolResult, PolicyFeedback), AgentLoopError> {
    writer
        .record(
            EventType::ToolStarted,
            serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "target": target,
            }),
        )
        .await?;
    let output = format!(
        "tool '{}' denied — {}; this retrieval lane refuses this tool \
         (write-domain deny-only gate, GAP-SUBAGENT-RUNTIME).",
        tc.name, reason,
    );
    let mut payload = serde_json::json!({
        "tool": tc.name,
        "call_id": tc.call_id,
        "target": target,
        "exit_code": 1,
        "status": "error",
        "error": reason,
    });
    // 0q：error 形状完成统一过漏斗（角色门拒绝不在四写点白名单 →
    // 「有意不聚合」标记，法官对账物齐备）。
    controller.stamp_failure(
        &mut payload,
        &tc.name,
        &tc.arguments,
        crate::host_exec::ToolFailureOutcome::Refused(reason),
    );
    writer.record(EventType::ToolCompleted, payload).await?;
    messages.push(Message {
        role: Role::Tool,
        content: output.clone(),
        tool_call_id: Some(tc.call_id.clone()),
        tool_calls: Vec::new(),
        reasoning_content: None,
        round: None,
    });
    Ok((
        ToolResult {
            output,
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        },
        PolicyFeedback::Denied(DenialKey {
            tool_name: tc.name.clone(),
            reason_code: reason.to_string(),
            // GAP-DENIAL-POLICY-REVISION (2026-08-12): live value — a bump is
            // a key change, resetting the breaker (ADR-0010 §3.5.4).
            policy_revision,
        }),
    ))
}

/// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 计划轮的机械拒绝 —
/// 首轮只允许 `blackboard_read` + `plan_write`；任何其他工具调用（即使被
/// 声明）都在派发前拒绝并留痕（ToolStarted → ToolCompleted(status=error)），
/// 与角色门的审计形状一致。
async fn plan_round_denied(
    controller: &AgentLoopController,
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
    tc: &ToolCall,
    reason: &'static str,
    policy_revision: u64,
) -> Result<(ToolResult, PolicyFeedback), AgentLoopError> {
    writer
        .record(
            EventType::ToolStarted,
            serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
            }),
        )
        .await?;
    let output = match reason {
        "plan_round_tool_denied" => format!(
            "tool '{}' denied — 首轮计划轮只允许 blackboard_read 与 plan_write；\
             执行/变更/检索/子代理等工具在计划落板前不可用（PLAN-FIRST 阶段 A）。",
            tc.name,
        ),
        "plan_write_already_submitted" => {
            "plan_write denied — 本轮已提交过 plan_write；请等待校验反馈\
             （若需重填，反馈将在下一轮注入）。"
                .to_string()
        }
        _ => format!("tool '{}' denied — {reason}", tc.name),
    };
    let mut payload = serde_json::json!({
        "tool": tc.name,
        "call_id": tc.call_id,
        "exit_code": 1,
        "status": "error",
        "error": reason,
    });
    // 0q：error 形状完成统一过漏斗（计划轮拒绝不在四写点白名单 →
    // 「有意不聚合」标记，法官对账物齐备）。
    controller.stamp_failure(
        &mut payload,
        &tc.name,
        &tc.arguments,
        crate::host_exec::ToolFailureOutcome::Refused(reason),
    );
    writer.record(EventType::ToolCompleted, payload).await?;
    messages.push(Message {
        role: Role::Tool,
        content: output.clone(),
        tool_call_id: Some(tc.call_id.clone()),
        tool_calls: Vec::new(),
        reasoning_content: None,
        round: None,
    });
    Ok((
        ToolResult {
            output,
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        },
        PolicyFeedback::Denied(DenialKey {
            tool_name: tc.name.clone(),
            reason_code: reason.to_string(),
            policy_revision,
        }),
    ))
}

/// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): per-round tool-result
/// injection budget refusal — the batch is at/over
/// `ORZ_MAX_INJECT_TOKENS_PER_ROUND` (default 50K), so this call is refused
/// WITHOUT ToolStarted (nothing executed). The journal carries the used
/// budget and the model receives an explicit offset/grep-first hint. The
/// refusal feeds the consecutive-denial breaker (same normalized key →
/// after 3 rounds the strategy-switch message fires, ADR-0010 §3.5.4).
#[allow(clippy::too_many_arguments)] // 0q: controller threaded for the failure funnel
async fn refuse_inject_budget(
    controller: &AgentLoopController,
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
    tc: &ToolCall,
    used: u64,
    budget: u64,
    policy_revision: u64,
    write_completed: bool,
) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
    let code = "round_inject_budget_exceeded";
    let output = format!(
        "tool '{}' — 本轮工具结果注入预算已满（已用 {} 估计 tokens / 上限 {}）；\
         请改用 grep/结构提取优先，或对 read_file 使用 offset 分段续读。",
        tc.name, used, budget,
    );
    if write_completed {
        let mut payload = serde_json::json!({
            "tool": tc.name,
            "call_id": tc.call_id,
            "exit_code": 1,
            "status": "error",
            "error": code,
            "inject_tokens_used": used,
            "inject_tokens_budget": budget,
        });
        // 0q：error 形状完成统一过漏斗（预算拒绝不在四写点白名单 →
        // 「有意不聚合」标记，法官对账物齐备）。
        controller.stamp_failure(
            &mut payload,
            &tc.name,
            &tc.arguments,
            crate::host_exec::ToolFailureOutcome::Refused(code),
        );
        writer.record(EventType::ToolCompleted, payload).await?;
    }
    messages.push(Message {
        role: Role::Tool,
        content: output.clone(),
        tool_call_id: Some(tc.call_id.clone()),
        tool_calls: Vec::new(),
        reasoning_content: None,
        round: None,
    });
    Ok((
        ToolResult {
            output,
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        },
        Some(PolicyFeedback::Denied(DenialKey {
            tool_name: tc.name.clone(),
            reason_code: code.to_string(),
            policy_revision,
        })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::host::{PermitDecision, PermitError, ToolError};
    use async_trait::async_trait;

    /// The retrieval-lane write-domain gate classifies the three refusal
    /// classes distinctly (ADR-0010 §3.2 deny-only domain; review F6,
    /// 2026-08-10 — `run_tests` is §3.8.2 controlled code execution, not a
    /// file write, and gets its own reason).
    #[test]
    fn retrieval_write_gate_classifies_denial_classes() {
        // File mutations → write-domain denial.
        assert_eq!(
            ToolFilter::Retrieval.write_gate("search_replace"),
            Some("retrieval_role_write_denied")
        );
        // Controlled code execution → execution denial (the test process
        // may write files / touch the network; a retrieval task contract
        // never carries a test harness).
        assert_eq!(
            ToolFilter::Retrieval.write_gate("run_tests"),
            Some("retrieval_role_execution_denied")
        );
        // Shell escape → shell denial.
        assert_eq!(
            ToolFilter::Retrieval.write_gate("bash"),
            Some("retrieval_role_shell_denied")
        );
        // Reads pass the gate — the same registry stays visible, refusal
        // is scope-level at call time (§3.5.2).
        assert_eq!(ToolFilter::Retrieval.write_gate("read_file"), None);
        assert_eq!(ToolFilter::Retrieval.write_gate("web_search"), None);
        assert_eq!(ToolFilter::Retrieval.write_gate("browser_control"), None);
        // The main lane's gate refuses nothing.
        assert_eq!(ToolFilter::None.write_gate("search_replace"), None);
        assert_eq!(ToolFilter::None.write_gate("bash"), None);
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15): the request-header fingerprint
    /// is stable for identical inputs and changes when ANY header component
    /// (system / tools / config) changes.
    #[test]
    fn request_header_fingerprint_is_stable_and_component_sensitive() {
        let tools = vec![
            ToolDef {
                name: "read_file".to_string(),
                description: "reads a file".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
            ToolDef {
                name: "grep".to_string(),
                description: "searches text".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let a = compute_request_header("system v1", &tools, "config-v1");
        let b = compute_request_header("system v1", &tools, "config-v1");
        assert_eq!(a, b);
        assert_eq!(a.header_sha256.len(), 64);
        assert_eq!(a.tools, vec!["read_file", "grep"]);

        // Tool order must not change the digest (the canonical rows are
        // sorted by name before hashing).
        let swapped = vec![tools[1].clone(), tools[0].clone()];
        let c = compute_request_header("system v1", &swapped, "config-v1");
        assert_eq!(a.tools_sha256, c.tools_sha256);
        assert_eq!(a.header_sha256, c.header_sha256);

        // Each component change breaks the header digest.
        assert_ne!(
            a.header_sha256,
            compute_request_header("system v2", &tools, "config-v1").header_sha256
        );
        assert_ne!(
            a.header_sha256,
            compute_request_header("system v1", &tools[..1], "config-v1").header_sha256
        );
        assert_ne!(
            a.header_sha256,
            compute_request_header("system v1", &tools, "config-v2").header_sha256
        );
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15): the payload carries the reason,
    /// the three component digests, the tool list and the change-kind
    /// attribution; `change` includes the previous header digest.
    #[test]
    fn request_header_payload_shapes_initial_and_change() {
        let header = compute_request_header("system", &[], "config");
        let initial = request_header_payload(&header, "initial", None, "main", None);
        assert_eq!(initial["reason"], "initial");
        assert_eq!(initial["header_sha256"], header.header_sha256);
        assert_eq!(initial["tool_count"], 0);
        assert_eq!(initial["agent_role"], "main");
        assert!(initial.get("previous_header_sha256").is_none());
        assert!(initial.get("change_kind").is_none());

        let changed =
            request_header_payload(&header, "change", Some("prev"), "main", Some("system"));
        assert_eq!(changed["reason"], "change");
        assert_eq!(changed["previous_header_sha256"], "prev");
        assert_eq!(changed["change_kind"], "system");
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15 review fix): the change-kind
    /// attribution is exact — one component change names that component,
    /// two or three name `multiple`.
    #[test]
    fn header_change_kind_attributes_component_changes() {
        let tool = vec![ToolDef {
            name: "read_file".to_string(),
            description: "reads a file".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let base = compute_request_header("system v1", &[], "config-v1");
        let system = compute_request_header("system v2", &[], "config-v1");
        let tools = compute_request_header("system v1", &tool, "config-v1");
        let config = compute_request_header("system v1", &[], "config-v2");
        let all = compute_request_header("system v2", &tool, "config-v2");
        assert_eq!(header_change_kind(&base, &system), "system");
        assert_eq!(header_change_kind(&base, &tools), "tools");
        assert_eq!(header_change_kind(&base, &config), "config");
        assert_eq!(header_change_kind(&base, &all), "multiple");
    }

    fn denial_key(tool: &str, reason: &str, policy_revision: u64) -> DenialKey {
        DenialKey {
            tool_name: tool.to_string(),
            reason_code: reason.to_string(),
            policy_revision,
        }
    }

    /// Same normalized key across three consecutive rounds fires the breaker
    /// exactly once (count restarts — injected once per burst).
    #[test]
    fn aggregate_denial_round_same_key_fires_at_three() {
        let mut denial = DenialState::default();
        let key = denial_key("read_file", "permission_denied", 0);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 1);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 2);
        // Third round → fires, count restarts (burst-once semantics).
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            Some("read_file".to_string())
        );
        assert_eq!(denial.consecutive_rounds, 0);
    }

    /// GAP-DENIAL-POLICY-REVISION (2026-08-12): a policy revision change is
    /// a DenialKey change — the consecutive count resets, so a
    /// `0,0,1`-sequence never fires (ADR-0010 §3.5.4 reset path now
    /// structurally reachable).
    #[test]
    fn aggregate_denial_round_policy_revision_change_resets() {
        let mut denial = DenialState::default();
        let rev0 = denial_key("read_file", "permission_denied", 0);
        let rev1 = denial_key("read_file", "permission_denied", 1);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&rev0), false),
            None
        );
        // Two rounds at revision 0…
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&rev0), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 2);
        // …then the policy bumps: the third round's key differs → reset.
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&rev1), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 1);
        // 0,0,1 then 1,1 — a fresh 3-round run at the new revision would
        // still fire; the bump itself never does.
        assert_eq!(denial.last_key, Some(rev1));
    }

    /// A round with any success resets the consecutive count.
    #[test]
    fn aggregate_denial_round_success_resets() {
        let mut denial = DenialState::default();
        let key = denial_key("read_file", "permission_denied", 0);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 1);
        // A mixed round (denial + success) is a key change → reset.
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), true),
            None
        );
        assert_eq!(denial.consecutive_rounds, 0);
        // A pure-success round also resets.
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), true),
            None
        );
        assert_eq!(denial.consecutive_rounds, 0);
        // Empty round (no denials) resets too (conservative baseline).
        assert_eq!(aggregate_denial_round(&mut denial, &[], false), None);
        assert_eq!(denial.consecutive_rounds, 0);
    }

    // ---- FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27) ----
    // 0ah 收口清理批（2026-09-16）：`LedgerFoldState` 导入随有状态折叠点退役
    // 移除（压缩直调测试改走无状态 `collapsed_cut` 语义）。

    use crate::host::ToolRegistry;
    use orz_assurance::journal::JournalRecorder;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, AtomicU64};

    struct CompactTestHost {
        journal: JournalRecorder,
    }

    struct CompactEmptyRegistry;

    impl ToolRegistry for CompactEmptyRegistry {
        fn get(&self, _name: &str) -> Option<ToolDef> {
            None
        }
        fn list(&self) -> Vec<ToolDef> {
            Vec::new()
        }
    }

    #[async_trait::async_trait]
    impl LoopHost for CompactTestHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &CompactEmptyRegistry
        }
        // Summary archives land under session_cwd/.gsa/compaction — point
        // the mock at the per-test journal dir (same pattern as the
        // controller test hosts).
        fn session_cwd(&self) -> PathBuf {
            self.journal.journal_dir().to_path_buf()
        }
    }

    fn compact_test_svc<'a>(
        cfg: &'a ContextCompactConfig,
        blackboard: &'a Arc<SharedBlackboard>,
        denial_state: &'a Mutex<DenialState>,
        pacing: &'a AtomicU32,
        policy: &'a AtomicU64,
    ) -> SharedLoopServices<'a> {
        SharedLoopServices {
            blackboard,
            denial_state,
            pacing_rounds: pacing,
            context_compact: cfg,
            evidence: None,
            policy_revision: policy,
            max_inject_tokens_per_round: 50_000,
            blackboard_archive_dir: None,
            session_id: None,
            in_flight_tools: None,
            retrieval_calls: None,
        }
    }

    /// 取证形态：0=user, 1=marker, 2=assistant(c1), 3=tool(c1),
    /// 4=assistant(c2), 5=tool(c2)——0ah 收口清理批后有状态折叠点退役，
    /// 保留起点一律按无状态 `collapsed_cut(messages, tail)` 重算。
    fn compact_test_messages() -> Vec<Message> {
        let mut messages = vec![Message {
            role: Role::User,
            content: "任务".to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }];
        messages.push(Message {
            role: Role::User,
            content: format!("{} v0.2] 摘要", crate::prompt::CONTEXT_COMPRESSED_PREFIX),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        for (id, target, result) in [("c1", "a.py", "A"), ("c2", "b.rs", "B")] {
            messages.push(Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": target}),
                    call_id: id.to_string(),
                }],
                reasoning_content: None,
                round: None,
            });
            messages.push(Message {
                role: Role::Tool,
                content: result.to_string(),
                tool_call_id: Some(id.to_string()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
        }
        messages
    }

    /// 压缩触发但缩减守卫不满足 → `GuardBlocked`：messages 不变（旧代码在此
    /// 路径删除了 marker → 后续保留起点重算裸露声明 → provider 400——
    /// marker 仅在执行确认后删除的纪律，0ah 收口后仍以无状态重算保持）。
    #[tokio::test]
    async fn guard_blocked_leaves_messages_and_fold_untouched() {
        let dir = std::env::temp_dir().join(format!("orz-compact-guard-{}", std::process::id()));
        let host = CompactTestHost {
            journal: JournalRecorder::new(dir),
        };
        let cfg = ContextCompactConfig::default();
        let blackboard = Arc::new(SharedBlackboard::new());
        let denial_state = Mutex::new(DenialState::default());
        let pacing = AtomicU32::new(0);
        let policy = AtomicU64::new(0);
        let svc = compact_test_svc(&cfg, &blackboard, &denial_state, &pacing, &policy);
        let mut writer = crate::controller::discard_event_writer("test-run");
        let mut messages = compact_test_messages();
        let before_messages = messages.clone();
        let decision = run_template_compact(
            &svc,
            &mut writer,
            &host,
            &mut messages,
            0, // measured=0 → removable=0 < min_compactable → 守卫必不满足
            "rhythm",
            false,
            false,
            0,
            // 0ah 收口清理批：tail=1 ⇒ 两轮里压掉 1 轮，kept_start 可解。
            1,
            None,
            // P2-14 S1 测试直调：消息构造无轮章 → 保持 v0.2 模板路径。
            None,
            // v7（S1 修订批）：直调测试的定位指针（None 项如实渲染「（无）」）。
            &crate::summary::LocatorPointers::default(),
        )
        .await
        .expect("guard path returns a decision");
        assert_eq!(decision, CompactDecision::GuardBlocked);
        assert_eq!(
            messages, before_messages,
            "guard path must not mutate messages (marker stays)"
        );
    }

    /// 执行路径：确认执行后删除旧 marker 并按无状态 `collapsed_cut` 重算
    /// kept_start——drain 数量与事件口径一致（0ah 收口清理批后有状态折叠点
    /// 退役，语义不变）。
    #[tokio::test]
    async fn compaction_execution_recomputes_kept_start_after_marker_removal() {
        let dir = std::env::temp_dir().join(format!("orz-compact-exec-{}", std::process::id()));
        let host = CompactTestHost {
            journal: JournalRecorder::new(dir.clone()),
        };
        let cfg = ContextCompactConfig::default();
        let blackboard = Arc::new(SharedBlackboard::new());
        let denial_state = Mutex::new(DenialState::default());
        let pacing = AtomicU32::new(0);
        let policy = AtomicU64::new(0);
        let svc = compact_test_svc(&cfg, &blackboard, &denial_state, &pacing, &policy);
        let mut writer = crate::controller::discard_event_writer("test-run");
        let mut messages = compact_test_messages();
        let decision = run_template_compact(
            &svc,
            &mut writer,
            &host,
            &mut messages,
            100_000, // force=true 跳过缩减守卫，measured 仅用于事件口径
            "rhythm",
            true,
            false,
            0,
            // tail=1 ⇒ marker 删除后重算保留起点（c2 轮起点），压掉 c1 轮。
            1,
            None,
            // P2-14 S1 测试直调：消息构造无轮章 → 保持 v0.2 模板路径。
            None,
            // v7（S1 修订批）：直调测试的定位指针（None 项如实渲染「（无）」）。
            &crate::summary::LocatorPointers::default(),
        )
        .await
        .expect("execution path returns a decision");
        assert_eq!(decision, CompactDecision::Executed);
        // marker 删除后 collapsed_cut(messages,1)=c2 轮起点（原索引 4→3）；
        // drain [first_round_start=1..3) 丢弃 c1 轮（声明+回复 2 条）；
        // 新 marker 插入索引 1 → U0+marker+c2 轮。
        assert_eq!(messages.len(), 4, "U0 + marker + c2 轮");
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::User);
        assert!(
            messages[1]
                .content
                .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX),
            "new rolling marker inserted at index 1"
        );
        assert_eq!(messages[2].tool_calls.len(), 1, "c2 声明保留");
        assert_eq!(messages[2].tool_calls[0].call_id, "c2");
        assert_eq!(messages[3].tool_call_id.as_deref(), Some("c2"));
        // 阶段 (c)（ADR-0010 §14.30 / 设计 §4.4.1）+ P2-13 D4：空黑板 →
        // 注意事项槽显示「（无）」；marker 携带回查入口与后续衔接占位。
        assert!(
            messages[1]
                .content
                .contains(crate::summary::NOTES_FACTS_EMPTY),
            "marker must carry the empty-notes text: {}",
            messages[1].content
        );
        assert!(
            messages[1]
                .content
                .contains(crate::summary::MECHANICAL_CONTINUATION_PLACEHOLDER)
        );
        let archive = dir
            .join(".gsa")
            .join("compaction")
            .join("compaction-test-run-0000.md");
        assert!(
            archive.exists(),
            "summary archive written under session_cwd"
        );
    }

    /// 阶段 (c) e2e（2026-08-19，ADR-0010 §14.30 / 设计 §4.4.1）+ P2-12
    /// 方案 A（2026-09-02）：黑板上已有结构化失败事实时，压缩 marker 与
    /// 存档的「注意事项」槽均为 HA 结构化事实聚合（计划面失败/阻塞 →
    /// F4 失败目标聚合 → 动作失败；exec 原文窗口被聚合行替换），零模型调用。
    #[tokio::test]
    async fn compaction_marker_and_archive_carry_facts_notes() {
        let dir = std::env::temp_dir().join(format!("orz-compact-facts-{}", std::process::id()));
        let host = CompactTestHost {
            journal: JournalRecorder::new(dir.clone()),
        };
        let cfg = ContextCompactConfig::default();
        let blackboard = Arc::new(SharedBlackboard::new());
        {
            let mut w = blackboard.write();
            w.plan.steps.push(crate::blackboard::PlanStep {
                id: "s1".into(),
                goal: "编译 MIPS 镜像".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: crate::blackboard::StepStatus::Failed(crate::blackboard::FailedEvidence {
                    receipt_id: "ORD-000001".into(),
                }),
            });
            w.plan.steps.push(crate::blackboard::PlanStep {
                id: "s2".into(),
                goal: "链接符号表".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: crate::blackboard::StepStatus::Blocked,
            });
            w.failure_agg.record(
                "file_target",
                "id-anchor1",
                "src/main.c",
                "tool_timeout",
                10.0,
                10,
                orz_assurance::lif::Domain::Normal,
            );
            w.failure_agg.record(
                "file_target",
                "id-anchor1",
                "src/main.c",
                "execution_failed",
                90.0,
                13,
                orz_assurance::lif::Domain::Pressure,
            );
            w.actions.results.push(crate::blackboard::ActionResult {
                order_id: "ORD-000002".into(),
                action: Some("workspace.run_tests".into()),
                ok: false,
                response: None,
                round: 0,
                domain: None,
                error: Some(serde_json::json!({
                    "step": "policy",
                    "code": "policy_denied",
                    "message": "denied",
                })),
                trace_id: "t000002".into(),
                timestamp: "2026-08-19T00:00:00Z".into(),
            });
        }
        let denial_state = Mutex::new(DenialState::default());
        let pacing = AtomicU32::new(0);
        let policy = AtomicU64::new(0);
        let svc = compact_test_svc(&cfg, &blackboard, &denial_state, &pacing, &policy);
        let mut writer = crate::controller::discard_event_writer("test-run");
        let mut messages = compact_test_messages();
        // 与既有执行路径测试同构：tail=1 ⇒ 压掉 c1 轮。
        let decision = run_template_compact(
            &svc,
            &mut writer,
            &host,
            &mut messages,
            100_000,
            "rhythm",
            true,
            false,
            0,
            1,
            None,
            // P2-14 S1 测试直调：消息构造无轮章 → 保持 v0.2 模板路径。
            None,
            // v7（S1 修订批）：直调测试的定位指针（None 项如实渲染「（无）」）。
            &crate::summary::LocatorPointers::default(),
        )
        .await
        .expect("execution path returns a decision");
        assert_eq!(decision, CompactDecision::Executed);
        let marker = &messages[1].content;
        // 排序=计划面 → 失败目标聚合 → 动作失败；marker 与存档同源同序。
        let plan_pos = marker.find("[步骤 s1]").expect("failed step in marker");
        let block_pos = marker.find("[步骤 s2]").expect("blocked step in marker");
        let fail_pos = marker
            .find("[失败目标 file_target] src/main.c ×2")
            .expect("failure-target aggregation row in marker");
        let action_pos = marker
            .find("[动作失败] ORD-000002 step=policy code=policy_denied trace_id=t000002")
            .expect("failed receipt in marker");
        assert!(plan_pos < block_pos && block_pos < fail_pos && fail_pos < action_pos);
        assert!(marker.contains("codes=[tool_timeout×1, execution_failed×1]"));
        assert!(marker.contains("首末 10s–90s"));
        assert!(marker.contains("域 normal(r10)→pressure(r13)"));
        assert!(
            !marker.contains("[执行错误]"),
            "exec raw-text window no longer rides the notes slot: {marker}"
        );
        assert!(marker.contains("receipt: ORD-000001"));
        assert!(marker.contains("（受阻）"));

        let archive = dir
            .join(".gsa")
            .join("compaction")
            .join("compaction-test-run-0000.md");
        let archive_text = std::fs::read_to_string(&archive).unwrap();
        let a_plan = archive_text.find("[步骤 s1]").unwrap();
        let a_block = archive_text.find("[步骤 s2]").unwrap();
        let a_fail = archive_text
            .find("[失败目标 file_target] src/main.c ×2")
            .unwrap();
        let a_action = archive_text.find("[动作失败] ORD-000002").unwrap();
        assert!(a_plan < a_block && a_block < a_fail && a_fail < a_action);
        // P2-13 D4（2026-09-03）：空槽统一渲染「（无）」——本测试的意图是
        // 注意事项槽有真实事实（非空态回退），改为按段头断言而非哨兵常量。
        assert!(
            !archive_text.contains("## 注意事项\n（无）"),
            "notes slot must not fall back to the empty marker: {archive_text}"
        );
        assert!(
            archive_text.contains("## 注意事项\n[步骤 s1]"),
            "notes slot must carry the first failed step: {archive_text}"
        );
        assert!(!archive_text.contains("[执行错误]"));
    }

    /// **用户 2026-09-15 裁定第二条**（窗口内溢出 ⇒ 超大工具结果指针化）：
    /// 溢出体量位于**当前上下文窗口内**（单结果过大、窗口外无可截留轮次）时，
    /// 正文换成「原文头部＋回读指针」⇒ 也能降到硬线之下；anomaly 记
    /// `..._result_pointerized`，工具配对字段不动（先例＝OUTPUT-DEGENERATION-GUARD）。
    #[tokio::test]
    async fn in_window_oversized_result_is_pointerized_until_under_the_line() {
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // v8（2026-09-16 勘误批）：T1 硬截断线钉在 5K **模型面估算**；分块关掉
        // （x 巨大）⇒ 20_000 字符（≈10K 估计）的单轮工具结果落在**主滑块内**
        // ⇒ 主滑块之外无可截断分块，只能靠**窗口内超大结果指针化**降线
        // （0ah §10.10 用户裁定，先例 OUTPUT-DEGENERATION-GUARD）。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, 5_000));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: "B".repeat(20_000),
                ..ok_result()
            }),
        };
        controller
            .run_turn(
                &host,
                "窗口内指针化测试",
                "RUN-PTR",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let audit = audit_events(&dir);
        let event = audit
            .iter()
            .find(|e| e.payload["payload"]["key"] == "context_scale:hard_truncate")
            .expect("截断/指针化落账");
        assert_eq!(
            event.payload["payload"]["anomaly"],
            "hard_context_result_pointerized"
        );
        let summary = event.payload["payload"]["summary"]
            .as_str()
            .unwrap_or_default();
        assert!(summary.contains("truncated_blocks=0"), "{summary}");
        assert!(summary.contains("pointerized_results=1"), "{summary}");
        let after: u64 = summary
            .split("model_face_estimate_after=")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse().ok())
            .expect("干预后模型面读数");
        assert!(after < 5_000, "主滑块内指针化须真降线: {after}");

        // 模型看到的是「头部＋回读指针」，且工具配对字段未动。
        let requests = fake.received_requests();
        let tool_msg = requests
            .iter()
            .flat_map(|r| r.messages.iter())
            .find(|m| m.tool_call_id.as_deref() == Some("call-r1"))
            .expect("工具结果消息");
        assert!(
            tool_msg
                .content
                .starts_with(crate::action_ledger::TOOL_RESULT_POINTERIZED_PREFIX),
            "{}",
            tool_msg.content
        );
        assert!(
            tool_msg.content.contains("call_id=call-r1"),
            "{}",
            tool_msg.content
        );
        assert!(
            tool_msg.content.contains("events.jsonl"),
            "{}",
            tool_msg.content
        );
        // 告知块：指针化事实（本例无分块截断）。
        let told = requests
            .iter()
            .flat_map(|r| r.messages.iter())
            .find(|m| m.content.contains("硬截断"))
            .expect("告知块");
        assert!(told.content.contains("个超大工具结果"), "{}", told.content);
        assert!(told.content.contains("工作现场"), "{}", told.content);
        assert!(
            told.content
                .contains(crate::model_face::MODEL_FACE_DECLARATION),
            "{}",
            told.content
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0k 并行批次专用 host——`call_tool` 带 60ms 固定延迟并记录并发峰值，
    /// 证明同轮只读调用确实并发执行（串行路径峰值恒为 1）。
    struct ConcurrentReadHost {
        journal: JournalRecorder,
        active: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        max_active: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    /// 0ap 复核批（P1）钉专用 host：按 `call_id` 分档返回体量并记录并发峰值
    /// ——`call-fat` 肥胖结果（第 1 轮形成闭合分块，读数需非零），其余小结果
    /// （并行批内两条工具消息都保真入会话）；峰值断言证明批内确实并发。
    struct SizedReadHost {
        journal: JournalRecorder,
        active: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        max_active: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl LoopHost for SizedReadHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &EmptyRegistry
        }
        fn session_cwd(&self) -> std::path::PathBuf {
            self.journal.journal_dir().to_path_buf()
        }
        async fn request_permission(
            &self,
            _risk: RiskClass,
            _tool: &str,
            _args: &serde_json::Value,
        ) -> Result<PermitDecision, PermitError> {
            Ok(PermitDecision::AllowOnce)
        }
        async fn call_tool(
            &self,
            _name: &str,
            _args: serde_json::Value,
            call_id: &str,
        ) -> Result<ToolResult, ToolError> {
            let now = self
                .active
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            self.max_active
                .fetch_max(now, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
            self.active
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            if call_id == "call-fat" {
                Ok(s1_fat_result())
            } else {
                Ok(ok_result())
            }
        }
    }

    #[async_trait]
    impl LoopHost for ConcurrentReadHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &EmptyRegistry
        }
        fn session_cwd(&self) -> std::path::PathBuf {
            self.journal.journal_dir().to_path_buf()
        }
        async fn request_permission(
            &self,
            _risk: RiskClass,
            _tool: &str,
            _args: &serde_json::Value,
        ) -> Result<PermitDecision, PermitError> {
            Ok(PermitDecision::AllowOnce)
        }
        async fn call_tool(
            &self,
            _name: &str,
            _args: serde_json::Value,
            _call_id: &str,
        ) -> Result<ToolResult, ToolError> {
            let now = self
                .active
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            self.max_active
                .fetch_max(now, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            self.active
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            Ok(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            })
        }
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30, TODO P0-0k 第一批
    /// 第 1 项)：同轮 ≥2 个连续只读 Host 调用走并行批次——并发峰值 ≥2、
    /// 事件与消息按声明序提交、journal 完整。
    #[tokio::test]
    async fn parallel_read_batch_runs_concurrently_and_commits_in_order() {
        let dir = test_dir();
        let active = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let max_active = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let host = ConcurrentReadHost {
            journal: JournalRecorder::new(dir.clone()),
            active,
            max_active,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![
                tool_call("read_file", "call-1"),
                tool_call("grep", "call-2"),
            ]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        AgentLoopController::with_gateway(gateway)
            .run_turn(
                &host,
                "并行读测试",
                "RUN-PAR",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        assert!(
            host.max_active.load(std::sync::atomic::Ordering::SeqCst) >= 2,
            "parallel batch must overlap host calls"
        );
        let events = events(&dir);
        let started: Vec<&str> = events
            .iter()
            .filter(|e| e.event_type == EventType::ToolStarted)
            .filter_map(|e| e.payload.get("tool").and_then(|v| v.as_str()))
            .collect();
        assert_eq!(started, ["read_file", "grep"], "{:?}", event_types(&dir));
        let completed: Vec<&str> = events
            .iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .filter_map(|e| e.payload.get("tool").and_then(|v| v.as_str()))
            .collect();
        assert_eq!(completed, ["read_file", "grep"], "{:?}", event_types(&dir));
        assert!(
            events
                .iter()
                .any(|e| e.event_type == EventType::RunFinished),
            "{:?}",
            event_types(&dir)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// S4 实机复验（2026-08-31）：并行批次中同一 call 只允许一条
    /// tool_completed（F11 receipt 段不重复）。并行批次提交阶段预算
    /// 超限时调用已真实执行（或已被 gate 拒绝）并留痕——`refuse_inject_budget`
    /// 只注入消息面拒绝 + deny，不再写第二条完成事件。
    #[tokio::test]
    async fn inject_budget_parallel_refusal_does_not_duplicate_completed_event() {
        let dir = test_dir();
        let host = ConcurrentReadHost {
            journal: JournalRecorder::new(dir.clone()),
            active: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            max_active: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        let mut writer = crate::controller::journal_event_writer(&host.journal, "RUN-BUDGET");
        let tc = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "x"}),
            call_id: "call-1".to_string(),
        };
        let mut messages: Vec<Message> = Vec::new();
        // 并行路径语义：write_completed=false —— 事件已在 buffered writer
        // 中留痕，此处只注入消息 + deny。
        let (_, feedback) = refuse_inject_budget(
            &AgentLoopController::default(),
            &mut writer,
            &mut messages,
            &tc,
            60_000,
            50_000,
            0,
            false,
        )
        .await
        .unwrap();
        assert!(matches!(feedback, Some(PolicyFeedback::Denied(_))));
        assert_eq!(messages.len(), 1);
        let events_path = dir.join("events.jsonl");
        if events_path.exists() {
            let events = std::fs::read_to_string(&events_path).unwrap();
            assert!(
                !events.contains("\"event_type\":\"tool_completed\""),
                "parallel-path budget refusal must not journal a duplicate completion: {events}"
            );
        }

        // 串行预检路径：write_completed=true —— 无 ToolStarted 的 gate
        // 拒绝完成事件是 F11 gate 段允许的形态。
        let dir2 = test_dir();
        let host2 = ConcurrentReadHost {
            journal: JournalRecorder::new(dir2.clone()),
            active: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            max_active: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        let mut writer2 = crate::controller::journal_event_writer(&host2.journal, "RUN-BUDGET2");
        let mut messages2: Vec<Message> = Vec::new();
        let _ = refuse_inject_budget(
            &AgentLoopController::default(),
            &mut writer2,
            &mut messages2,
            &tc,
            60_000,
            50_000,
            0,
            true,
        )
        .await
        .unwrap();
        let events2 = std::fs::read_to_string(dir2.join("events.jsonl")).unwrap();
        assert!(
            events2.contains("\"event_type\":\"tool_completed\""),
            "serial precheck refusal must journal one gate completion: {events2}"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    // ── 0ae D3 / 0AE-C1 修复接线测试（2026-09-15 深审）─────────────────
    //
    // 消费分支修复前零覆盖：窗口轮工具面 Vec::new() ＋ tool_calls 无条件
    // 丢弃 ⇒ model_participated 结构上恒 false。以下端到端钉子覆盖三条
    // 路径：参与（落穿派发 blackboard_write ＋ 延迟收口）、混合声明（FR-3：
    // 不锁工具面——非写入调用照常派发、无机械提示）、纯文本窗口耗尽（3 轮
    // 后原地收口 model_not_participated）。
    //
    // **动态上下文滑块 S1（2026-09-15，设计 §3.4 A7）**：开窗量尺已从
    // 「上一轮实测 prompt tokens 视图刻度 920K」改为「**实际上下文估算**
    // 500K/900K（全量会话 chars/2）＋ 模型自选」——旧钉子的
    // `with_prompt_tokens(950 * 1024)` 直驱法不再成立（实测 token 与
    // 实际上下文是两个量尺）。新驱动件 `s1_actual_context_prompt()` ＋
    // `s1_fat_result()` 把**实际上下文**推过 500K 而不碰视图刻度。
    //
    // **滑块上下文 v8（2026-09-16 勘误批，设计 §3）**：开窗量尺＝**模型面
    // 估算**（投影层装配出的请求视图 chars/2），阶梯改为 192/224/256/288K
    // 软提醒 → **320K 硬提醒（打断 ＋ 开窗）** → **500K 硬截断**。旧刻度缝隙
    // `with_context_scale_milestones` 退役，改由 `v8_test_ladder` 把 H1／T1
    // 钉到任意刻度。

    /// v8 阶梯测试缝隙：软提醒抬到不可达刻度，**H1 硬打断**与
    /// **T1 硬截断**分别钉在 `hard`／`truncate`（模型面估算刻度）。
    /// 0bh ④（2026-09-22）：软档改 192/256 双档 ⇒ 测试梯同缩为 4 档。
    fn v8_test_ladder(hard: u64, truncate: u64) -> [crate::context_scale::LadderStep; 4] {
        use crate::context_scale::{LadderStep, LadderTier};
        [
            LadderStep {
                tokens: u64::MAX - 1,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: u64::MAX,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: hard,
                tier: LadderTier::HardReminder,
            },
            LadderStep {
                tokens: truncate,
                tier: LadderTier::HardTruncate,
            },
        ]
    }

    /// 把初始 prompt 推大到 800K 字符（估算 ≈400K < 500K ⇒ 首轮 loop-top
    /// **不**越线），配合 `s1_fat_result()` 的首轮工具结果 ⇒ 次轮 loop-top
    /// 估算 ≈550K ≥ 500K ⇒ 500K 刻度越线并开窗（与旧流程「第 2 个 loop-top
    /// 开窗」同形，便于保持各钉子的轮序）。
    fn s1_actual_context_prompt() -> String {
        "x".repeat(800_000)
    }

    /// 首轮工具结果 300K 字符（估算 +150K）——把实际上下文推过 500K 刻度。
    fn s1_fat_result() -> ToolResult {
        ToolResult {
            output: "y".repeat(300_000),
            ..ok_result()
        }
    }

    /// 只读审查 R-12③（2026-09-16 处置）：**一轮内只注入最高档**。
    ///
    /// 单轮暴涨（例：会话冷启就带着长上下文）会让 `due()` 在同一次 loop-top
    /// 返回 4–5 档；逐档注入等于把「只有刻度字样不同」的同一段文案连发数遍。
    /// 处置＝**只注入最高档**，低档**水位与事件照记**（journal 保留「本轮越过了
    /// 哪几档」的完整事实，`form=` 注明未注入）。
    #[tokio::test]
    async fn a_single_round_surge_injects_only_the_highest_tier() {
        use crate::context_scale::{LadderStep, LadderTier};
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 初始 prompt 估算 ≈400K（800K 字符）⇒ 两个软档在**同一次 loop-top**
        // 一起越线；硬档抬到不可达（本钉只测软档合并）。0bh ④：软档双档。
        let ladder = [
            LadderStep {
                tokens: 100_000,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: 250_000,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: u64::MAX,
                tier: LadderTier::HardReminder,
            },
            LadderStep {
                tokens: u64::MAX,
                tier: LadderTier::HardTruncate,
            },
        ];
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_context_scale_ladder(ladder);
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: None,
        };
        controller
            .run_turn(
                &host,
                &s1_actual_context_prompt(),
                "RUN-SCALE-SURGE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 注入面：恰一块提醒，且是**最高档**（250K）。
        let requests = fake.received_requests();
        let injected: Vec<_> = requests[0]
            .messages
            .iter()
            .filter(|m| {
                m.content
                    .starts_with(crate::context_scale::REMINDER_INJECTED_PREFIX)
            })
            .collect();
        assert_eq!(injected.len(), 1, "一轮暴涨只注入最高档: {injected:?}");
        assert!(
            injected[0].content.starts_with("[CONTEXT_SCALE 250K]"),
            "最高档＝250K: {}",
            injected[0].content
        );

        // 水位面：两个软档照记（会话级事实不留缺口）。0bh ④：软档 192／256 双档
        // ⇒ 本钉的测试梯同缩为两个软刻度（2026-09-23 重建批同步：断言随梯改）。
        assert_eq!(
            controller.context_scale_notified_keys(),
            vec!["100k".to_string(), "250k".to_string()]
        );

        // 事件面：两行照落，只有最高档 form=standalone_block。
        let scale: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_CONTEXT_SCALE)
            .collect();
        assert_eq!(scale.len(), 2, "两档事件照落: {scale:?}");
        let summary_of = |key: &str| -> String {
            scale
                .iter()
                .find(|e| e.payload["payload"]["key"] == key)
                .map(|e| {
                    e.payload["payload"]["summary"]
                        .as_str()
                        .unwrap_or("")
                        .to_string()
                })
                .unwrap_or_else(|| panic!("{key} 事件缺失: {scale:?}"))
        };
        assert!(
            summary_of("context_scale:250k").contains("form=standalone_block"),
            "最高档须实际注入: {}",
            summary_of("context_scale:250k")
        );
        for key in ["context_scale:100k"] {
            assert!(
                summary_of(key).contains("form=suppressed_superseded_by_higher_tier"),
                "{key} 须记为「被更高档吞掉」: {}",
                summary_of(key)
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 只读审查 R-12③ 的另一面：**T1 与本轮提醒同轮**时只发**截断告知块**
    /// （内容即将被截断，软／硬提醒已无意义），且告知块自带**截断后读数**
    /// ——压掉低档提醒因此是无损的。
    #[tokio::test]
    async fn a_truncation_round_injects_only_the_notice_with_the_post_cut_reading() {
        use crate::context_scale::{LadderStep, LadderTier};
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 首轮工具结果 20K 字符（估算 +10K）⇒ 软档（3K）与 T1（4K）在**同一次
        // loop-top** 一起越线；H1 与另一软档抬到不可达。0bh ④：软档双档。
        let ladder = [
            LadderStep {
                tokens: 3_000,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: u64::MAX,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: u64::MAX,
                tier: LadderTier::HardReminder,
            },
            LadderStep {
                tokens: 4_000,
                tier: LadderTier::HardTruncate,
            },
        ];
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_context_scale_ladder(ladder)
            .with_session_id(Some("SESSION-T1-SAME-ROUND".to_string()));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: "T".repeat(20_000),
                ..ok_result()
            }),
        };
        controller
            .run_turn(
                &host,
                "截断同轮测试",
                "RUN-T1-SAME-ROUND",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 事件面：软档与 T1 档都如实记为「让位给截断告知块」（水位与事实不留缺口）。
        let scale: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_CONTEXT_SCALE)
            .collect();
        let summary_of = |key: &str| -> String {
            scale
                .iter()
                .find(|e| e.payload["payload"]["key"] == key)
                .map(|e| {
                    e.payload["payload"]["summary"]
                        .as_str()
                        .unwrap_or("")
                        .to_string()
                })
                .unwrap_or_else(|| panic!("{key} 事件缺失: {scale:?}"))
        };
        assert!(
            summary_of("context_scale:3k").contains("form=deferred_to_truncation_notice"),
            "{}",
            summary_of("context_scale:3k")
        );
        assert!(
            summary_of("context_scale:4k").contains("form=deferred_to_truncation_notice"),
            "{}",
            summary_of("context_scale:4k")
        );

        // 注入面：本轮无软／硬提醒（`压缩交给你自选` 只见于软提醒文案），
        // 只有截断告知块，且它带着截断后的当前读数。
        let requests = fake.received_requests();
        let last = requests.last().expect("至少一轮请求");
        assert!(
            !last
                .messages
                .iter()
                .any(|m| m.content.contains("压缩交给你自选")),
            "T1 同轮不得注入软／硬提醒: {last:?}"
        );
        let notice = last
            .messages
            .iter()
            .find(|m| m.content.contains("硬截断] 已把"))
            .expect("截断告知块");
        assert!(
            notice.content.contains("截断后当前读数"),
            "告知块须带截断后读数: {}",
            notice.content
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn context_compress_call(call_id: &str) -> ToolCall {
        ToolCall {
            name: "context_compress".to_string(),
            arguments: serde_json::json!({}),
            call_id: call_id.to_string(),
        }
    }

    fn window_write_call(call_id: &str) -> ToolCall {
        ToolCall {
            name: "blackboard_write".to_string(),
            arguments: serde_json::json!({
                "section": "notes",
                "content": "关键接线结论：压缩窗口参与测试的固化内容",
            }),
            call_id: call_id.to_string(),
        }
    }

    fn audit_events(dir: &std::path::Path) -> Vec<orz_assurance::RunEvent> {
        events(dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::MechanicalAuditUpdate)
            .collect()
    }

    /// 0ap S1 钉③（2026-09-18，设计 §6）：端到端链——`context_compress`
    /// 调用 → 请求位消费开窗（下一个 loop-top 安全边界，D3 既有机制）→
    /// 窗口轮工具面＝常规面（FR-3）→ 窗口内再调用＝in-progress no-op（防抖）
    /// → 出窗后语义摘要 → 按块压缩落地 →
    /// `context_compressed{mode=model_summary, reason=model_selected}`。
    /// 窗口由工具知情发起（非 H1 阶梯：阶梯抬到不可达）⇒ 归因按设计 §1
    /// 明文取 `model_selected`（与 H1 窗口的 `context_scale_window` 分流）。
    #[tokio::test]
    async fn context_compress_end_to_end_request_window_summary_and_selected_reason() {
        let semantic_block = "[SEMANTIC_SUMMARY]\n目标: 完成接线\n已完成: 台账接线\n\
                              关键决策: 分块压缩\n[/SEMANTIC_SUMMARY]";
        let fake = Arc::new(FakeProvider::new(vec![
            // 第 1 轮：正常工作——肥胖结果在 slider=1K/block=1K 下形成闭合分块。
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")]),
            // 第 2 轮：模型知情发起（响应 = Requested 形态＋滑块读数）。
            ScriptedResponse::tool_calls(vec![context_compress_call("call-c1")]),
            // 第 3 轮（窗口轮）：窗口内再调用 = in-progress no-op；本轮有
            // 窗口调用 ⇒ 延迟收口（无写入/无摘要 ⇒ model_participated=false
            // 如实落账——防抖调用不构成参与）。
            ScriptedResponse::tool_calls(vec![context_compress_call("call-c2")]),
            // 第 4 轮（已出窗，普通面）：模型产出语义摘要块。
            ScriptedResponse::text(semantic_block),
            // 第 5 轮 loop-top 按块压缩落地后的收尾。
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(1_000)
            .with_context_scale_ladder(v8_test_ladder(1_000_000, 1_000_000_000));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(s1_fat_result()),
        };
        let (answer, _, _) = controller
            .run_turn(
                &host,
                "0ap 端到端",
                "RUN-CC-E2E",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(answer, "终答");

        // ① 两次调用都 exit 0（fail-soft 三态信封）。
        let cc: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted && e.payload["tool"] == "context_compress"
            })
            .collect();
        assert_eq!(cc.len(), 2, "{:?}", cc.len());
        assert!(cc.iter().all(|e| e.payload["exit_code"] == 0));

        // ② 窗口轮（第 3 轮请求）工具面 = **常规面**（FR-3，0bc 长杂轮：
        // 硬提醒不锁工具面——不再收窄为两件窗口工具；与相邻常规轮一致）。
        let requests = fake.received_requests();
        assert_eq!(
            requests[2]
                .tools
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            requests[1]
                .tools
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            "窗口轮工具面应＝常规面（FR-3）"
        );
        // ③ 第 2 轮工具结果 = Requested（含读数表）；第 3 轮 = in-progress。
        assert!(
            requests[2]
                .messages
                .iter()
                .any(|m| m.content.contains("压缩窗口已请求") && m.content.contains("滑块读数："))
        );
        assert!(
            requests[3]
                .messages
                .iter()
                .any(|m| m.content.contains("压缩窗口已在程中（in_progress）"))
        );

        // ④ 窗口收口审计恰一条（防抖轮有窗口调用 ⇒ 延迟收口、未参与如实）。
        let mc: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_MODEL_COMPRESSION)
            .collect();
        assert_eq!(mc.len(), 1, "{mc:?}");
        let summary = mc[0].payload["payload"]["summary"].as_str().unwrap();
        assert!(summary.contains("model_participated=false"), "{summary}");
        assert!(summary.contains("rounds_used=1"), "{summary}");

        // ⑤ 语义压缩落地：reason=model_selected（工具发起窗口；设计 §1）。
        let compact: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact.len(), 1, "语义轨恰压一次: {compact:?}");
        assert_eq!(compact[0]["mode"], "model_summary");
        assert_eq!(compact[0]["reason"], "model_selected");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0ap 复核批（P2）钉：语义压缩归因映射——H1 阶梯窗口 ⇒
    /// `context_scale_window`；工具知情发起窗口与无窗口（模型自选摘要块）
    /// ⇒ `model_selected`。归因在**摘要识别点**取值（见
    /// `CompressionWindowKind`），故「工具窗口收口未产出摘要」之后的 H1
    /// 窗口压缩不再被残留旗标误记为 `model_selected`。
    #[test]
    fn semantic_compression_reason_maps_window_kinds() {
        assert_eq!(
            semantic_compression_reason(Some(CompressionWindowKind::Ladder)),
            "context_scale_window"
        );
        assert_eq!(
            semantic_compression_reason(Some(CompressionWindowKind::ModelRequested)),
            "model_selected"
        );
        assert_eq!(semantic_compression_reason(None), "model_selected");
    }

    /// 0ap 复核批（P1）钉：同轮读类并行批次里 `blackboard_read` 的滑块读数段
    /// 必须读**批首会话**——该路径的 `messages` 只是本调用的注入槽（批首为空
    /// `Vec`），修复前据此现算会恒定渲染「滑块外可压缩 0 块 ≈ est 0」。
    /// 峰值并发断言证明本批确实走并行路径（否则钉子不成立）。
    #[tokio::test]
    async fn blackboard_read_slider_readout_reads_the_parallel_batch_conversation() {
        let blackboard_read_call = ToolCall {
            name: "blackboard_read".to_string(),
            arguments: serde_json::json!({"section": "session"}),
            call_id: "call-b2".to_string(),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // 第 1 轮：肥胖结果在 slider=1K/block=1K 下形成闭合分块（单调用 ⇒ 串行）。
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-fat")]),
            // 第 2 轮：批首三个连续只读调用 ⇒ 并行批次（run_len = 3）。
            ScriptedResponse::tool_calls(vec![
                // 两件 Host 读调用证明真并发（并发峰值只看 Host 调用；
                // `blackboard_read` 在控制器内执行，不经过宿主）。
                tool_call("read_file", "call-r2a"),
                tool_call("read_file", "call-r2b"),
                blackboard_read_call,
            ]),
            ScriptedResponse::text("终答"),
            // 反例门（一次性）消费后仍需一轮收尾。
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(1_000)
            // 并行批内两个结果都保真入会话（不被注入预算拒绝替换）。
            .with_max_inject_tokens_per_round(10_000_000)
            .with_context_scale_ladder(v8_test_ladder(1_000_000, 1_000_000_000));
        let dir = test_dir();
        let host = SizedReadHost {
            journal: JournalRecorder::new(dir.clone()),
            active: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            max_active: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        controller
            .run_turn(
                &host,
                "并行批读数",
                "RUN-PAR-READOUT",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert!(
            host.max_active.load(std::sync::atomic::Ordering::SeqCst) >= 2,
            "并行批必须真并发（否则本钉不成立）"
        );

        let requests = fake.received_requests();
        let header = requests[2]
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-b2"))
            .map(|m| m.content.lines().next().unwrap_or_default().to_string())
            .expect("blackboard_read 工具消息带增量头");
        let blocks: usize = header
            .split("滑块外可压缩 ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("增量头缺滑块读数段: {header}"));
        assert!(
            blocks >= 1,
            "并行批读数段必须读批首会话（修复前为 0）: {header}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 参与路径：窗口轮声明 blackboard_write → 落穿正常派发执行 → 下一
    /// loop-top 延迟收口（model_participated=true，无异常事实）。
    #[tokio::test]
    async fn compression_window_participates_via_blackboard_write_and_closes_deferred() {
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![window_write_call("call-w1")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("已固化；可弃范围：第 1 轮读数"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 分块关掉（x 巨大）＋阶梯缝隙：本钉只管**窗口语义**——H1 钉在
        // 1K（模型面估算）即刻打断并开窗；T1 抬到不可达（另钉覆盖截断）。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_context_scale_ladder(v8_test_ladder(1_000, 1_000_000_000));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(s1_fat_result()),
        };
        let (answer, _, _) = controller
            .run_turn(
                &host,
                "窗口开启量尺测试任务",
                "RUN-WIN-A",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(answer, "终答");

        // 窗口轮请求面：**常规面**（FR-3，0bc 长杂轮：硬提醒不锁工具面——
        // 与上一常规轮工具面一致），任务块注入且携带水位（0AE-C6）。
        let requests = fake.received_requests();
        let window_round = &requests[1];
        assert_eq!(
            window_round
                .tools
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            requests[0]
                .tools
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            "窗口轮工具面应＝常规面（FR-3）"
        );
        let window_block = window_round
            .messages
            .iter()
            .find(|m| m.content.contains("模型参与压缩"))
            .expect("compression window block injected");
        assert!(
            window_block.content.contains("【"),
            "watermark rides the window block: {window_block:?}"
        );
        // v8（设计 §5）：注入块＝**H1 硬提醒**（模型面读数 ＋ 宣告 T1 硬截断
        // 口径 ＋ 分块表 ＋「不压缩也可以」）＋ 压缩窗口任务（同块注入）。
        assert!(
            window_block.content.starts_with("[CONTEXT_SCALE")
                && window_block.content.contains("硬提醒")
                && window_block.content.contains("当前上下文窗口")
                && window_block.content.contains("硬性截断")
                && window_block.content.contains("模型参与压缩 · 窗口"),
            "硬提醒须携带模型面读数并与窗口任务同块注入: {window_block:?}"
        );

        // journal：恰一条 model_compression 收口事件（延迟收口，参与为真，
        // 0AE-C2 条目形状 {key, round, summary, anomaly}）。
        let mc: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_MODEL_COMPRESSION)
            .collect();
        assert_eq!(mc.len(), 1, "exactly one close event: {mc:?}");
        let payload = &mc[0].payload["payload"];
        assert_eq!(payload["key"], "model_compression");
        let summary = payload["summary"].as_str().unwrap();
        assert!(summary.contains("model_participated=true"), "{summary}");
        assert!(summary.contains("rounds_used=1"), "{summary}");
        assert!(summary.contains("window_start_writes=0"), "{summary}");
        assert!(summary.contains("writes_now=1"), "{summary}");
        assert!(payload["anomaly"].is_null());
        assert_eq!(payload["round"], 2, "close recorded at the deferred round");

        // 阶梯事件形状：key=context_scale:<刻度>（C2 形状 {key, round, summary,
        // anomaly}，summary 必含**模型面读数**——v8 量尺＝投影层估算）。
        // D2 阶梯 kind 已退役 ⇒ 本 run 不得出现 attention_ladder 事件。
        let scale: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_CONTEXT_SCALE)
            .collect();
        let fire_h1 = scale
            .iter()
            .find(|e| e.payload["payload"]["key"] == "context_scale:1k")
            .unwrap_or_else(|| panic!("H1 硬提醒事件缺失: {scale:?}"));
        let summary_h1 = fire_h1.payload["payload"]["summary"].as_str().unwrap();
        assert!(
            summary_h1.contains("milestone_tokens=1000")
                && summary_h1.contains("model_face_estimate_tokens=")
                && summary_h1.contains("tier=hard_reminder")
                && summary_h1.contains("watermark=【"),
            "阶梯事件须含模型面读数与水位的 C2 形状: {summary_h1}"
        );
        assert!(
            !scale
                .iter()
                .any(|e| e.payload["payload"]["key"] == "context_scale:500k"),
            "T1 已抬到不可达刻度，本 run 不得出现截断档: {scale:?}"
        );
        assert!(
            audit_events(&dir)
                .into_iter()
                .all(|e| e.payload["kind"] != crate::mechanical_audit::KIND_ATTENTION_LADDER),
            "D2 阶梯已整体退役：本 run 不得再有 attention_ladder 事件"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FR-3（2026-09-21，0bc 长杂轮）：窗口轮**不锁工具面**——混合声明
    /// （read_file ＋ blackboard_write）全部照常派发、无「已跳过」提示；
    /// blackboard_write 执行后延迟收口为参与。
    #[tokio::test]
    async fn compression_window_dispatches_non_write_calls_without_notice() {
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![
                tool_call("read_file", "call-r2-window"),
                window_write_call("call-w1"),
            ])
            .with_prompt_tokens(100),
            ScriptedResponse::text("固化完成"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // v8：H1 打断档钉在 1K（模型面估算），T1 抬到不可达。
        // FR-3 勘定（2026-09-21）：窗口轮的 fat read 照常派发后，其大结果
        // 会计入轮内注入预算（默认 50K，ADR-0010 §3.6）；默认预算下同轮
        // **后续**调用会被该既有门拒绝（实测 write 被拒 ⇒ 参与判定位失真）
        // ——与本测试命题（FR-3 不过滤/不丢弃/无提示）正交，此处抬高预算
        // 隔离该门；注入门自身行为由 host_exec 侧测试覆盖。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_max_inject_tokens_per_round(1_000_000)
            .with_context_scale_ladder(v8_test_ladder(1_000, 1_000_000_000));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(s1_fat_result()),
        };
        let (answer, _, _) = controller
            .run_turn(
                &host,
                "窗口开启量尺测试任务",
                "RUN-WIN-B",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(answer, "终答");

        // 无丢弃提示：窗口轮不再注入「窗口内仅 … 可执行」话术（FR-3）。
        let requests = fake.received_requests();
        let round_after_window = &requests[2].messages;
        assert!(
            !round_after_window
                .iter()
                .any(|m| m.content.contains("窗口内仅")),
            "no drop notice: {round_after_window:?}"
        );

        // 窗口轮声明的 read_file（call-r2-window）照常派发：全程执行两次
        // （第 1 轮 ＋ 窗口轮）。
        let read_file_completions = events(&dir)
            .into_iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted && e.payload["tool"] == "read_file"
            })
            .count();
        assert_eq!(
            read_file_completions, 2,
            "window-round call must dispatch (FR-3)"
        );
        // blackboard_write 照常执行；收口事件如实记参与。
        let mc: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_MODEL_COMPRESSION)
            .collect();
        assert_eq!(mc.len(), 1, "{mc:?}");
        assert!(
            mc[0].payload["payload"]["summary"]
                .as_str()
                .unwrap()
                .contains("model_participated=true"),
            "{mc:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 纯文本窗口耗尽路径：3 轮窗口全文本 → 原地收口
    /// （model_not_participated 异常事实；rounds_used=3）＋ 机械折叠
    /// 兜底旗标。提醒文案为单空格（0AE-C13）。
    #[tokio::test]
    async fn compression_window_text_only_rounds_close_as_not_participated() {
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("我先标注一下可弃范围").with_prompt_tokens(100),
            ScriptedResponse::text("再确认一遍").with_prompt_tokens(100),
            ScriptedResponse::text("仍未写入黑板").with_prompt_tokens(100),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // v8：H1 打断档钉在 1K（模型面估算），T1 抬到不可达。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_context_scale_ladder(v8_test_ladder(1_000, 1_000_000_000));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(s1_fat_result()),
        };
        let (answer, _, _) = controller
            .run_turn(
                &host,
                "窗口开启量尺测试任务",
                "RUN-WIN-C",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(answer, "终答");

        // 窗口恰耗 3 轮：3 个窗口轮请求工具面＝常规面（FR-3：不锁工具面）。
        let requests = fake.received_requests();
        for request in &requests[1..4] {
            assert_eq!(
                request
                    .tools
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>(),
                requests[0]
                    .tools
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>(),
                "窗口轮工具面应＝常规面（FR-3）"
            );
            assert!(
                request
                    .messages
                    .iter()
                    .any(|m| m.content.contains("模型参与压缩"))
            );
        }
        // 提醒文案（v7 重写）：窗口剩余轮提示改指**语义摘要块**，仍守
        // 「无成串空格」纪律（0AE-C13）。
        let second_window_round = &requests[2].messages;
        assert!(
            second_window_round.iter().any(|m| m
                .content
                .contains("窗口剩余 2 轮：尚未检测到语义摘要块。请输出")),
            "reminder copy uses a single space: {second_window_round:?}"
        );
        assert!(
            !second_window_round
                .iter()
                .any(|m| m.content.contains("语义摘要块。  ")),
            "no run-on spaces in the reminder: {second_window_round:?}"
        );

        // 收口事件：未参与 → anomaly=model_not_participated、rounds_used=3。
        let mc: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_MODEL_COMPRESSION)
            .collect();
        assert_eq!(mc.len(), 1, "{mc:?}");
        let payload = &mc[0].payload["payload"];
        let summary = payload["summary"].as_str().unwrap();
        assert!(summary.contains("model_participated=false"), "{summary}");
        assert!(summary.contains("rounds_used=3"), "{summary}");
        assert_eq!(payload["anomaly"], "model_not_participated");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 窄边沿修复钉（2026-09-15 审查登记观察）：窗口轮落穿的同迭代若被
    /// budget 耗尽收尾 break 截停（D-8：耗尽后不再派发任何工具），收口
    /// 状态不得随循环终止丢弃——收口审计事件必须在 break 前落地，且如实
    /// 记 model_not_participated（blackboard_write 因 D-8 未派发）。
    #[tokio::test]
    async fn compression_window_close_survives_budget_exhaustion_break() {
        let fake = Arc::new(FakeProvider::new(vec![
            // 第 1 轮：普通工具轮（大工具结果把**实际上下文**推过 500K
            // 刻度）⇒ 第 2 轮 loop-top 越线开窗；
            // 轮末 tool_rounds(1) ≥ max_tool_rounds(1) ⇒ budget_exhausted。
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            // 第 2 轮（窗口轮）：模型声明 blackboard_write → 落穿 →
            // D-8 耗尽收尾 break（不派发）→ break 前收口。
            ScriptedResponse::tool_calls(vec![window_write_call("call-w1")])
                .with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            // v8：H1 打断档钉在 1K（模型面估算），T1 抬到不可达。
            .with_context_scale_ladder(v8_test_ladder(1_000, 1_000_000_000))
            .with_max_tool_rounds(1);
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(s1_fat_result()),
        };
        let _ = controller
            .run_turn(
                &host,
                "窗口开启量尺测试任务",
                "RUN-WIN-D",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 恰两轮请求：第 2 轮窗口面＝常规面（FR-3：不锁工具面）；无第 3 轮。
        let requests = fake.received_requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[1]
                .tools
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            requests[0]
                .tools
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            "窗口轮工具面应＝常规面（FR-3）"
        );

        // 收口事件在 break 前落地：未参与如实落账（写入未派发）。
        let mc: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_MODEL_COMPRESSION)
            .collect();
        assert_eq!(
            mc.len(),
            1,
            "close event must land before the break: {mc:?}"
        );
        let payload = &mc[0].payload["payload"];
        let summary = payload["summary"].as_str().unwrap();
        assert!(summary.contains("model_participated=false"), "{summary}");
        assert!(summary.contains("writes_now=0"), "{summary}");
        assert_eq!(payload["anomaly"], "model_not_participated");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 滑块上下文 v8（2026-09-16 勘误批，设计 §2/§3/§6）────────────────

    /// A4（设计 §3.2；**v8 触发改锚「首个分块形成」**——滑窗驱逐随勘误退役）：
    /// 主滑块之外首次出现分块后发射一次固化提醒——每 run 恰一次、指向黑板
    /// 写入面；同时模型面投影层开始带**分块表**（设计 §2）。
    #[tokio::test]
    async fn first_block_reminder_fires_exactly_once_after_the_first_block() {
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r3")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 主滑块 x 很小 ⇒ 首轮之后即出现主滑块之外的分块（投影层据此挂分块表
        // 与指针）；阶梯抬到不可达 ⇒ 本钉只测分块面与 A4 提醒。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(2_000)
            .with_model_face_block_tokens(1_000)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, u64::MAX));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: "z".repeat(6_000),
                ..ok_result()
            }),
        };
        let (answer, _, _) = controller
            .run_turn(
                &host,
                "首个分块提醒测试",
                "RUN-SLIDE-A4",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(answer, "终答");

        // 提醒恰一次（审计）＋ 注入面恰一次（跨全部请求计数）。
        let reminders: Vec<_> = audit_events(&dir)
            .into_iter()
            .filter(|e| e.payload["kind"] == crate::mechanical_audit::KIND_CONTEXT_SCALE)
            .collect();
        assert_eq!(
            reminders.len(),
            1,
            "首个分块提醒每 run 恰一次: {reminders:?}"
        );
        let payload = &reminders[0].payload["payload"];
        assert_eq!(payload["key"], "context_scale:first_block");
        let summary = payload["summary"].as_str().unwrap();
        assert!(
            summary.contains("form=standalone_block")
                && summary.contains("model_face_estimate_tokens=")
                && summary.contains("blocks=")
                && summary.contains("watermark=【"),
            "提醒事件须带模型面读数、分块数与水位的 C2 形状: {summary}"
        );
        let requests = fake.received_requests();
        let injected = requests
            .iter()
            .flat_map(|r| r.messages.iter())
            .filter(|m| m.content.starts_with("[CONTEXT_SCALE 首个分块]"))
            .count();
        // 注入面：提醒块进入 `messages` 后随会话延续（故跨请求计数 ≥1）；
        // 「恰一次」由上面的审计事件计数钉住。
        assert!(injected >= 1, "提醒块须已注入: {injected}");
        assert!(
            requests.iter().any(|r| r
                .messages
                .iter()
                .any(|m| m.content.contains("blackboard_write section=plan|notes"))),
            "提醒文案保留 A4 的固化语义（指向黑板写入面）"
        );
        // 模型面带**固定指针**与**分块表**（投影层结构；设计 §2「块表挂在
        // 固定指针之后」）。
        let pointer_rounds = requests
            .iter()
            .filter(|r| {
                r.messages.iter().any(|m| {
                    m.content
                        .starts_with(crate::action_ledger::LEDGER_FOLD_POINTER_PREFIX)
                })
            })
            .count();
        assert!(
            pointer_rounds >= 1,
            "分块出现后模型面须带固定指针消息（{requests:?}）"
        );
        assert!(
            requests.iter().any(|r| r
                .messages
                .iter()
                .any(|m| m.content.starts_with(crate::model_face::BLOCK_TABLE_PREFIX))),
            "模型面须带分块表"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── v7（S1 修订批）档位分工 / 压缩分工 / 会话级水位 / 停手纪律 ──────

    /// v8 DP-16（设计 §3）＋ 2026-09-16 实现批：
    ///
    /// ① **软档＝会话级水位**——已提醒刻度随会话状态注入 loop ⇒ 同一刻度跨
    ///    prompt 不重发（新会话独立由 host 侧侧车保证）；四个软刻度全部预置 ⇒
    ///    本 prompt 零提醒、零阶梯事件、水位原样。
    /// ② 硬档（H1／T1）改为**按越线重新武装**（用户 2026-09-16 裁定），故它们的
    ///    抑制不走水位——见姊妹钉 `context_scale_hard_tiers_rearm_above_the_line`。
    #[tokio::test]
    async fn context_scale_watermark_is_session_level() {
        use crate::context_scale::{LadderStep, LadderTier};
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 两个软刻度钉在会话面量级（静态开销由阶梯缝隙 pin 0）；硬档抬到不可达。
        // 0bh ④：软档双档（192/256 步距语义）。
        let ladder = [
            LadderStep {
                tokens: 100_000,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: 400_000,
                tier: LadderTier::Soft,
            },
            LadderStep {
                tokens: u64::MAX,
                tier: LadderTier::HardReminder,
            },
            LadderStep {
                tokens: u64::MAX,
                tier: LadderTier::HardTruncate,
            },
        ];
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_context_scale_ladder(ladder)
            .with_context_scale_notified(
                ["100k", "200k", "300k", "400k"]
                    .iter()
                    .map(|k| (*k).to_string())
                    .collect(),
            );
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(s1_fat_result()),
        };
        controller
            .run_turn(
                &host,
                &s1_actual_context_prompt(),
                "RUN-SCALE-SESSION",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let requests = fake.received_requests();
        assert!(
            !requests.iter().any(|r| r.messages.iter().any(|m| m
                .content
                .starts_with(crate::context_scale::REMINDER_INJECTED_PREFIX))),
            "软刻度已提醒过 ⇒ 本 prompt 不再注入提醒块"
        );
        assert!(
            !audit_events(&dir)
                .iter()
                .any(|e| { e.payload["kind"] == crate::mechanical_audit::KIND_CONTEXT_SCALE }),
            "会话级水位已全部预置 ⇒ 不得再落任何阶梯事件"
        );
        assert_eq!(
            controller.context_scale_notified_keys(),
            vec![
                "100k".to_string(),
                "200k".to_string(),
                "300k".to_string(),
                "400k".to_string()
            ],
            "水位键原样保留（顺序＝升序）"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// v8 实现批（用户 2026-09-16 裁定「守卫降值 ＋ 重新武装」）：**硬档按越线
    /// 重新武装**——预置水位**不**抑制 T1：恢复会话若仍在线上，必须先按读数把
    /// 模型面压回线上（这里走「工作现场之外无已闭合分块」⇒ 窗口内超大结果指针化
    /// 分支），水位键照旧保留（审计事实）。
    #[tokio::test]
    async fn context_scale_hard_tiers_rearm_above_the_line() {
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(100_000_000)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, 100))
            .with_context_scale_notified(
                ["192k", "224k", "256k", "288k", "320k", "500k"]
                    .iter()
                    .map(|k| (*k).to_string())
                    .collect(),
            );
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(s1_fat_result()),
        };
        controller
            .run_turn(
                &host,
                &s1_actual_context_prompt(),
                "RUN-SCALE-REARM",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let cut = audit_events(&dir)
            .into_iter()
            .find(|e| e.payload["payload"]["key"] == "context_scale:hard_truncate")
            .expect("预置水位不得抑制 T1（重新武装）");
        assert_eq!(
            cut.payload["payload"]["anomaly"],
            "hard_context_result_pointerized"
        );
        assert!(
            controller
                .context_scale_notified_keys()
                .contains(&"500k".to_string()),
            "水位键为审计事实、不因重新武装被清除: {:?}",
            controller.context_scale_notified_keys()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// v8（2026-09-16 勘误批，设计 §4 §6）：模型在回复里产出语义摘要块 ⇒
    /// 机械层执行一次**按块压缩**（`mode=model_summary`、reason=
    /// `model_selected`）——被压块移出**模型面**、marker（v0.4-分块压缩）携带
    /// 摘要正文＋机械摘要行＋四项定位指针＋**按块回放路径**；**本地面零覆盖**
    /// （`messages` 一条不删，逐字原文另落 `.gsa/compaction/blocks/`）。
    #[tokio::test]
    async fn model_summary_block_replaces_the_region() {
        let semantic_block = "[SEMANTIC_SUMMARY]\n目标: 完成接线\n已完成: 台账接线\n\
                             关键决策: 保留滑块\n未决问题: 无\n下一步: 跑 A/B\n关键文件: compact.rs\n\
                             [/SEMANTIC_SUMMARY]";
        // 被压区必须**显著大于** marker 固定开销（定位指针＋分工声明 ≈1.2K
        // 估计）——否则语义轨按设计判「替换无收益」⇒ NoOp（不替换）。
        let old_round = "O".repeat(10_000);
        let outputs = std::sync::Mutex::new(vec![
            ToolResult {
                output: old_round.clone(),
                ..ok_result()
            },
            ToolResult {
                output: "N".repeat(600),
                ..ok_result()
            },
        ]);
        struct SeqHost {
            journal: JournalRecorder,
            outputs: std::sync::Mutex<Vec<ToolResult>>,
        }
        #[async_trait::async_trait]
        impl LoopHost for SeqHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            fn session_cwd(&self) -> std::path::PathBuf {
                self.journal.journal_dir().to_path_buf()
            }
            async fn call_tool(
                &self,
                _name: &str,
                _arguments: serde_json::Value,
                _call_id: &str,
            ) -> Result<ToolResult, ToolError> {
                let mut outputs = self.outputs.lock().unwrap();
                if outputs.is_empty() {
                    return Ok(ok_result());
                }
                Ok(outputs.remove(0))
            }
            async fn request_permission(
                &self,
                _risk: RiskClass,
                _tool: &str,
                _arguments: &serde_json::Value,
            ) -> Result<PermitDecision, PermitError> {
                Ok(PermitDecision::AllowOnce)
            }
        }
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            // 第 2 轮：工具轮文本携带语义摘要块（工具轮文本不进 messages，
            // 识别点在响应处理）⇒ 下一个 loop-top 执行语义压缩。
            ScriptedResponse {
                text: Some(semantic_block.to_string()),
                ..ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
            }
            .with_prompt_tokens(100),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 主滑块 x 与分块 y 都取小值 ⇒ 首轮之后即有「主滑块之外的闭合分块」
        // 可压；阶梯抬到不可达（本钉只测压缩面）。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, u64::MAX))
            .with_session_id(Some("SESSION-SEM-1".to_string()));
        let dir = test_dir();
        let host = SeqHost {
            journal: JournalRecorder::new(dir.clone()),
            outputs,
        };
        controller
            .run_turn(
                &host,
                "语义轨测试",
                "RUN-SEMANTIC",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact.len(), 1, "语义轨恰压一次: {compact:?}");
        assert_eq!(compact[0]["mode"], "model_summary");
        assert_eq!(compact[0]["reason"], "model_selected");
        assert_eq!(compact[0]["summary_incomplete"], false);
        assert!(compact[0]["summary_path"].as_str().is_some());

        let requests = fake.received_requests();
        let after = requests.last().expect("至少一轮请求");
        let marker = after
            .messages
            .iter()
            .find(|m| {
                m.content
                    .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
            })
            .expect("语义轨 marker");
        assert!(
            marker.content.contains("v0.4-分块压缩"),
            "{}",
            marker.content
        );
        assert!(
            marker.content.contains("已处理分块: 1"),
            "{}",
            marker.content
        );
        assert!(
            marker.content.contains("关键决策: 保留滑块"),
            "{}",
            marker.content
        );
        for locator in [
            "原文定位（四项）",
            "journal: run=RUN-SEMANTIC",
            "会话档案（sidecar）:",
        ] {
            assert!(
                marker.content.contains(locator),
                "语义 marker 缺 {locator:?}: {}",
                marker.content
            );
        }
        // 按块回放面（设计 §6 形态①）：marker 带 read_file 分页指针，且该
        // 逐字档案确实落盘。
        assert!(
            marker.content.contains(".gsa/compaction/blocks/")
                && marker.content.contains("read_file offset/limit"),
            "{}",
            marker.content
        );
        let replay_dir = dir.join(".gsa").join("compaction").join("blocks");
        let replay_files = std::fs::read_dir(&replay_dir)
            .map(|d| d.flatten().count())
            .unwrap_or(0);
        assert!(replay_files >= 1, "分块逐字回放档案须落盘: {replay_dir:?}");
        assert!(
            !after
                .messages
                .iter()
                .any(|m| m.content.contains(&old_round)),
            "被压块原文必须移出**模型面**（摘要替换；本地面仍留档）"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 审查修正批（2026-09-15，审查 P1②）：**终止轮的语义摘要必须在 run 尾
    /// 被消费**。此前 `pending_semantic` 只有 loop-top 一个消费点 ⇒ 模型把
    /// `[SEMANTIC_SUMMARY]` 放在**最后一次终答**（反例门已用过、无工具轮）
    /// 时，run 直接 break、摘要静默丢弃（无事件、无 marker）。本钉把摘要块
    /// 放在「反例门触发后的那次终答」⇒ 断言 run 尾补齐一次语义压缩
    /// （`mode=model_summary`／被压区移出／存档落盘）。
    #[tokio::test]
    async fn final_answer_semantic_summary_is_consumed_at_the_run_tail() {
        let semantic_block = "[SEMANTIC_SUMMARY]\n目标: 收尾压缩\n已完成: 台账接线\n\
                             关键决策: 保留滑块\n未决问题: 无\n下一步: 跑 A/B\n关键文件: compact.rs\n\
                             [/SEMANTIC_SUMMARY]";
        // 被压区必须显著大于语义 marker 的固定开销（定位指针＋分工声明）。
        let old_round = "O".repeat(10_000);
        let outputs = std::sync::Mutex::new(vec![
            ToolResult {
                output: old_round.clone(),
                ..ok_result()
            },
            ToolResult {
                output: "N".repeat(600),
                ..ok_result()
            },
        ]);
        struct SeqHost {
            journal: JournalRecorder,
            outputs: std::sync::Mutex<Vec<ToolResult>>,
        }
        #[async_trait::async_trait]
        impl LoopHost for SeqHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            fn session_cwd(&self) -> std::path::PathBuf {
                self.journal.journal_dir().to_path_buf()
            }
            async fn call_tool(
                &self,
                _name: &str,
                _arguments: serde_json::Value,
                _call_id: &str,
            ) -> Result<ToolResult, ToolError> {
                let mut outputs = self.outputs.lock().unwrap();
                if outputs.is_empty() {
                    return Ok(ok_result());
                }
                Ok(outputs.remove(0))
            }
            async fn request_permission(
                &self,
                _risk: RiskClass,
                _tool: &str,
                _arguments: &serde_json::Value,
            ) -> Result<PermitDecision, PermitError> {
                Ok(PermitDecision::AllowOnce)
            }
        }
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            // 首次终答候选（无工具轮）⇒ 反例门 fire + continue（该轮文本不携带
            // 摘要块，避免被 loop-top 提前消费）。两个工具轮是必需的：驻留带
            // 恒含最新一轮，只有 ≥2 个完整轮时「最旧轮」才可能被移出。
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答初稿"),
            // 门之后的终答：**摘要块就在这里** ⇒ 本 run 只有 run 尾能消费它。
            ScriptedResponse::text(semantic_block),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, u64::MAX))
            .with_session_id(Some("SESSION-SEM-TAIL".to_string()));
        let dir = test_dir();
        let host = SeqHost {
            journal: JournalRecorder::new(dir.clone()),
            outputs,
        };
        controller
            .run_turn(
                &host,
                "终答语义摘要测试",
                "RUN-SEM-TAIL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(
            compact.len(),
            1,
            "终止轮的语义摘要必须在 run 尾被消费（恰一次）: {compact:?}"
        );
        assert_eq!(compact[0]["mode"], "model_summary");
        assert_eq!(compact[0]["reason"], "model_selected");
        assert!(
            compact[0]["rounds_dropped"].as_u64().unwrap_or(0) >= 1,
            "被压区须真实移出: {compact:?}"
        );
        // 语义存档与 marker 落盘（run 尾路径与 loop-top 同形）。
        let archive = compact[0]["summary_path"].as_str().expect("summary_path");
        assert!(
            std::path::Path::new(archive).exists(),
            "语义压缩存档须落盘: {archive}"
        );
        // 数值刻度水位未被本路径污染（本测试未开窗口／无刻度越线）；`first_block`
        // 属字面水位（首个分块固化提醒，2026-09-16 实现批由 per-run 改会话级），
        // 本路径出现分块时理应置位。
        assert!(
            controller
                .context_scale_notified_keys()
                .iter()
                .all(|k| k == crate::context_scale::FLAG_FIRST_BLOCK),
            "本路径不得写入数值刻度水位: {:?}",
            controller.context_scale_notified_keys()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **v8 T1 硬截断**（设计 §5）：模型面估算越过 T1 ⇒ 机械层把**主滑块以外
    /// 的全部分块**移出模型面——① 逐字原文落盘可按块回放（形态①）；② 注入
    /// 告知块（已截断 N 块／可按块回放／任务无需中止／模型面声明）；③ 事实经
    /// `anomaly=hard_context_truncated_blocks` 落账；④ **本地面零覆盖**：被截断
    /// 轮的逐字原文仍在持久化会话里（不变量 I3）。
    #[tokio::test]
    async fn t1_truncation_moves_blocks_out_of_the_model_face_and_keeps_the_local_face() {
        let old_round = "T".repeat(6_000);
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // x 小 ⇒ 首轮之后即有分块；T1 钉在 4K 模型面估算（首轮 6K 字符 ≈3K
        // 估计 ＋ 次轮 ⇒ 越线）；H1 抬到不可达（本钉只测截断）。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, 4_000))
            .with_session_id(Some("SESSION-T1".to_string()));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: old_round.clone(),
                ..ok_result()
            }),
        };
        let mut conversation = vec![conv_message(Role::User, "截断测试任务")];
        controller
            .run_turn(
                &host,
                "截断测试任务",
                "RUN-T1",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();

        // ① 事实落账（设计 §5：anomaly=hard_context_truncated_blocks）。
        let cut = audit_events(&dir)
            .into_iter()
            .find(|e| e.payload["payload"]["key"] == "context_scale:hard_truncate")
            .expect("截断事实落账");
        assert_eq!(
            cut.payload["payload"]["anomaly"],
            "hard_context_truncated_blocks"
        );
        let summary = cut.payload["payload"]["summary"].as_str().unwrap_or("");
        assert!(summary.contains("truncated_blocks="), "{summary}");
        assert!(summary.contains("freed_tokens="), "{summary}");

        // ② 告知块（注入文本）＋ ③ 回放档案落盘。
        let requests = fake.received_requests();
        let notice = requests
            .iter()
            .flat_map(|r| r.messages.iter())
            .find(|m| m.content.contains("已把**工作现场以外**"))
            .expect("T1 告知块");
        // 0bh ⑯ 子项（2026-09-23 重建批同步）：替模型下判断的「任务无需中止」句已删
        // ⇒ 本钉改钉**状况陈述**（删句后仍须如实告知现场未动）。
        assert!(
            notice.content.contains("工作现场") && notice.content.contains("逐字未动"),
            "{}",
            notice.content
        );
        assert!(notice.content.contains("可按块回放"), "{}", notice.content);
        assert!(
            notice
                .content
                .contains(crate::model_face::MODEL_FACE_DECLARATION),
            "{}",
            notice.content
        );
        let replay_dir = dir.join(".gsa").join("compaction").join("blocks");
        let replay_files: Vec<std::path::PathBuf> = std::fs::read_dir(&replay_dir)
            .map(|d| d.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        assert!(
            !replay_files.is_empty(),
            "分块逐字回放档案须落盘: {replay_dir:?}"
        );
        let replay_text = std::fs::read_to_string(&replay_files[0]).unwrap();
        assert!(replay_text.contains("陈旧性标注"), "{replay_text}");

        // ④ 模型面：被截断块的工具结果不在请求面。
        let last = requests.last().expect("至少一轮请求");
        assert!(
            !last
                .messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-r1")),
            "被截断块必须移出模型面"
        );
        // ④b **主滑块逐字必须在场**（P0 修复钉子，审查 R-1）：截断只动
        // 「已闭合分块」，工作现场（最新完整轮）与其后的新轮次一条不动。
        assert!(
            last.messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-r2")),
            "主滑块（工作现场）逐字不得被截断：{last:?}"
        );

        // ⑤ 本地面零覆盖：逐字原文仍在持久化会话（I3）＋ 截断 marker 在场。
        assert!(
            conversation.iter().any(|m| m.content.contains(&old_round)),
            "本地面不得因截断丢失逐字原文"
        );
        assert!(
            conversation.iter().any(|m| {
                crate::model_face::parse_marker_blocks(&m.content)
                    .is_some_and(|(state, _)| state == crate::model_face::BlockState::Truncated)
            }),
            "截断 marker 须随会话留存（恢复后块状态由此重建）"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// v8 实现批（审查 R-5）：**T1 档案写失败不得静默**——`.gsa` 落点被占用时，
    /// 截断照常执行（安全闸门优先），但事实必须①落账（`archive_write_failed=true`）
    /// ②如实告知模型（告知块写明回放档案未落盘，并指出逐字原文仍在会话档案与
    /// journal），**不得向模型承诺一个不存在的回放档案**。
    #[tokio::test]
    async fn t1_archive_write_failure_is_reported_to_the_model_and_the_journal() {
        let old_round = "T".repeat(6_000);
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, 4_000))
            .with_session_id(Some("SESSION-T1-FAIL".to_string()));
        let dir = test_dir();
        let blocked_cwd = test_dir();
        // `.gsa` 落成一个**文件** ⇒ `create_dir_all(.gsa/compaction/…)` 必失败。
        std::fs::write(blocked_cwd.join(".gsa"), "occupied").unwrap();
        let host = BlockedArchiveHost {
            inner: TestHost {
                journal: JournalRecorder::new(dir.clone()),
                tool_result: Some(ToolResult {
                    output: old_round.clone(),
                    ..ok_result()
                }),
            },
            blocked_cwd: blocked_cwd.clone(),
        };
        let mut conversation = vec![conv_message(Role::User, "截断落盘失败测试")];
        controller
            .run_turn(
                &host,
                "截断落盘失败测试",
                "RUN-T1-FAIL",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();
        // ① 截断照常执行（被截块移出模型面）且事实带失败读数。
        let cut = audit_events(&dir)
            .into_iter()
            .find(|e| e.payload["payload"]["key"] == "context_scale:hard_truncate")
            .expect("截断事实落账（不因落盘失败而停手）");
        let summary = cut.payload["payload"]["summary"].as_str().unwrap_or("");
        assert!(summary.contains("archive_write_failed=true"), "{summary}");
        assert!(summary.contains("truncated_blocks="), "{summary}");
        // ② 告知块如实说明「回放档案写入失败」，并指出逐字原文仍在别处。
        let requests = fake.received_requests();
        let notice = requests
            .iter()
            .flat_map(|r| r.messages.iter())
            .find(|m| m.content.contains("已把**工作现场以外**"))
            .expect("T1 告知块");
        assert!(
            notice.content.contains("回放档案写入失败"),
            "{}",
            notice.content
        );
        assert!(
            notice.content.contains("会话档案") && notice.content.contains("journal"),
            "失败时须给出真实载体: {}",
            notice.content
        );
        // ③ 本地面零覆盖不变（逐字原文仍在持久化会话）。
        assert!(
            conversation.iter().any(|m| m.content.contains(&old_round)),
            "本地面不得因落盘失败丢失逐字原文"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&blocked_cwd);
    }

    /// **v8 块号幂等**（设计 §12「块表状态幂等；同一块不重复计数」）：同一块被
    /// 压缩一次后再次被摘要点名 ⇒ NoOp（不产生第二条 `context_compressed`、
    /// 不重复计数），分块表状态保持「已压缩」。
    #[tokio::test]
    async fn block_compaction_is_idempotent_by_block_number() {
        let semantic_block = "[SEMANTIC_SUMMARY]\n压缩块: 1\n目标: 接线\n已完成: 一遍\n\
                              [/SEMANTIC_SUMMARY]";
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse {
                text: Some(semantic_block.to_string()),
                ..ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
            }
            .with_prompt_tokens(100),
            // 第二次点同一块：loop-top 压缩应 NoOp。
            ScriptedResponse {
                text: Some(semantic_block.to_string()),
                ..ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r3")])
            }
            .with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, u64::MAX))
            .with_session_id(Some("SESSION-IDEM".to_string()));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: "I".repeat(6_000),
                ..ok_result()
            }),
        };
        controller
            .run_turn(
                &host,
                "块号幂等测试",
                "RUN-IDEM",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(
            compact.len(),
            1,
            "同一块只压一次（去重键＝块号）: {compact:?}"
        );
        assert_eq!(compact[0]["mode"], "model_summary");
        // 分块表状态：块 1 已压缩（模型面里带状态标注）。
        let requests = fake.received_requests();
        let table = requests
            .iter()
            .flat_map(|r| r.messages.iter())
            .filter(|m| m.content.starts_with(crate::model_face::BLOCK_TABLE_PREFIX))
            .last()
            .expect("分块表");
        assert!(table.content.contains("已压缩"), "{}", table.content);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **v8 上限守卫（异常保险）**（设计 §3 §8）：模型面估算越过 1.10M 估算
    /// 守卫时**强制截断到线上**（v7 的「不开窗降级」随勘误作废），anomaly 记
    /// 守卫口径。
    #[tokio::test]
    async fn model_face_guard_forces_truncation_above_the_safety_line() {
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
                .with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 守卫钉在 4K（异常线测试化）；阶梯抬到不可达（不与 T1 混）。
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_model_face_guard_tokens(4_000)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, u64::MAX))
            .with_session_id(Some("SESSION-GUARD".to_string()));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: "G".repeat(6_000),
                ..ok_result()
            }),
        };
        controller
            .run_turn(
                &host,
                "守卫测试",
                "RUN-GUARD",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let cut = audit_events(&dir)
            .into_iter()
            .find(|e| e.payload["payload"]["key"] == "context_scale:hard_truncate")
            .expect("守卫截断落账");
        assert_eq!(
            cut.payload["payload"]["anomaly"],
            "hard_context_guard_truncated_blocks"
        );
        let summary = cut.payload["payload"]["summary"].as_str().unwrap_or("");
        assert!(summary.contains("guard_hit=true"), "{summary}");
        let requests = fake.received_requests();
        // 同上：判据由「任务无需中止」改为状况陈述（⑯ 子项已删该句）。
        assert!(
            requests.iter().flat_map(|r| r.messages.iter()).any(|m| {
                m.content.contains("上限守卫") && m.content.contains("逐字未动")
            }),
            "守卫告知块须注入"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **v8 存档写入失败必须显式上报**（P0-D review fix 纪律 × v8 按块压缩）：
    /// `.gsa/compaction` 落点被占（目录创建失败）⇒ 压缩照常执行（模型面已下移、
    /// marker 落地），但事件的 `archive_write_failed=true`，**绝不静默吞掉**。
    /// 台账落点（`.gsa/ledger/`）不受影响，故压缩不会被回滚。
    #[tokio::test]
    async fn block_compaction_reports_archive_write_failure_explicitly() {
        let semantic_block = "[SEMANTIC_SUMMARY]\n压缩块: 1\n目标: 接线\n已完成: 一遍\n\
                              [/SEMANTIC_SUMMARY]";
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")])
                .with_prompt_tokens(100),
            ScriptedResponse {
                text: Some(semantic_block.to_string()),
                ..ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r2")])
            }
            .with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
            ScriptedResponse::text("终答").with_prompt_tokens(100),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_slider_window_tokens(1_000)
            .with_model_face_block_tokens(500)
            .with_context_scale_ladder(v8_test_ladder(u64::MAX, u64::MAX))
            .with_session_id(Some("SESSION-ARCHFAIL".to_string()));
        let dir = test_dir();
        // `.gsa/compaction` 占成**文件** ⇒ create_dir_all 失败 ⇒ 存档写不进去；
        // `.gsa` 本身仍是目录 ⇒ 台账（`.gsa/ledger/`）照常可写。
        std::fs::create_dir_all(dir.join(".gsa")).unwrap();
        std::fs::write(dir.join(".gsa").join("compaction"), "occupied").unwrap();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: "A".repeat(6_000),
                ..ok_result()
            }),
        };
        controller
            .run_turn(
                &host,
                "存档失败测试",
                "RUN-ARCHFAIL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        assert_eq!(compact.len(), 1, "压缩仍须执行: {compact:?}");
        assert_eq!(
            compact[0]["archive_write_failed"], true,
            "存档写入失败必须显式上报: {compact:?}"
        );
        assert_eq!(compact[0]["summary_incomplete"], false);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 0au：派发前 run 墙钟余量保留（2026-09-20 立项，S3 摩擦 N2） ──────

    /// 0au 钉子（判据 ②③）：run 余量不足时检索派发被保留——
    /// ① 无 `ToolStarted` 配对（预派发拒绝，模板同 D3），`ToolCompleted`
    ///   携带 cause `retrieval_dispatch_wallclock_reserved`；
    /// ② 模型面文案只报事实（批墙钟／run 剩余／保留额）；
    /// ③ 下一轮 post-batch 间隙一次性重述（cause 自描述、不邀请重派）。
    /// 上限经 controller 测试 seam 注入（`with_run_wallclock_limit_secs`）
    /// ——不触碰进程 env，无并行污染面；env 优先序由解析式
    /// `.or(seam)` 结构保证（生产 env 在位时 seam 永不生效）。
    #[tokio::test]
    async fn retrieval_dispatch_reserved_when_run_wallclock_is_short() {
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: None,
        };
        // 回应链：保留轮 → 重述后的续答轮 → 终答前的草稿/终答轮。
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-0au")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_retrieval_enabled(
            AgentLoopController::with_gateway(gateway).with_run_wallclock_limit_secs(10),
        );
        controller
            .run_turn(
                &host,
                "查文档",
                "RUN-0AU-RESERVE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .expect("the run itself completes (reserve is not a failure)");

        // ① journal：无 ToolStarted 配对；ToolCompleted.error = cause。
        let journal_events = events(&dir);
        let started: Vec<&orz_assurance::RunEvent> = journal_events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolStarted && e.payload["call_id"] == "call-0au"
            })
            .collect();
        assert!(
            started.is_empty(),
            "reserved dispatch must not start: {started:?}"
        );
        let completed = journal_events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted && e.payload["call_id"] == "call-0au"
            })
            .expect("reserved dispatch still completes (audit face)");
        assert_eq!(
            completed.payload["error"],
            crate::retrieval::batch_close::WALLCLOCK_RESERVED_CAUSE,
            "{:?}",
            completed.payload
        );

        // ②③ 下一轮请求：post-batch 间隙的一次性重述（cause 自描述）。
        let requests = fake.received_requests();
        let restated = requests.iter().any(|r| {
            r.messages
                .iter()
                .any(|m| m.content.contains("retrieval_dispatch_wallclock_reserved"))
        });
        assert!(
            restated,
            "the one-shot restatement must name the reserve cause"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0au 钉子（2026-09-20 审查修复批）：保留轮的同轮重复位也发保留回执
    /// ——保留判定整批同质，重复位的指针回踩若照旧发出，会指向一条未派发
    /// 回执（原调用同样被保留），构成「已派发过」的假指针。本钉：同轮两次
    /// 同 query 检索在保留轮各得一条 cause 回执，指针回踩文案不出现。
    #[tokio::test]
    async fn reserved_round_holds_duplicates_with_the_reserve_receipt() {
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: None,
        };
        // 回应链：保留轮（同 query 两次）→ 完成轮 → 草稿/终答轮。
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![
                tool_call("web_search", "call-0au-a"),
                tool_call("web_search", "call-0au-b"),
            ]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_retrieval_enabled(
            AgentLoopController::with_gateway(gateway).with_run_wallclock_limit_secs(10),
        );
        controller
            .run_turn(
                &host,
                "查文档",
                "RUN-0AU-RESERVE-DUP",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .expect("the run itself completes");

        // 两个调用位都走保留臂：无 ToolStarted，ToolCompleted.error = cause。
        let journal_events = events(&dir);
        for call_id in ["call-0au-a", "call-0au-b"] {
            let started = journal_events
                .iter()
                .filter(|e| {
                    e.event_type == EventType::ToolStarted && e.payload["call_id"] == call_id
                })
                .count();
            assert_eq!(started, 0, "{call_id} must not start");
            let completed = journal_events
                .iter()
                .find(|e| {
                    e.event_type == EventType::ToolCompleted && e.payload["call_id"] == call_id
                })
                .unwrap_or_else(|| panic!("{call_id} must complete"));
            assert_eq!(
                completed.payload["error"],
                crate::retrieval::batch_close::WALLCLOCK_RESERVED_CAUSE,
                "the duplicate of a reserved original must hold too: {:?}",
                completed.payload
            );
        }
        // 指针回踩文案（「已于本 run 派发过」）不得出现于任何模型面请求。
        let requests = fake.received_requests();
        let pointered_back = requests.iter().any(|r| {
            r.messages
                .iter()
                .any(|m| m.content.contains("已于本 run 派发过"))
        });
        assert!(
            !pointered_back,
            "a reserved round must not yield the duplicate pointer note"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
