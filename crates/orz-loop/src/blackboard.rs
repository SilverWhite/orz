//! Blackboard — structured shared state for multi-agent communication.
//!
//! 5 sections, each with a single writer. All sections are readable by all agents.
//! Write rules enforce single-writer discipline.
//!
//! Single main agent + two retrieval subagents (fusion §4.5): the main agent
//! writes Plan/Exec, each retrieval subagent writes its own section, and the
//! controller writes GateLog. The console action board (P0-C S1) adds three
//! single-writer slots: the assistant layer refreshes the registration board
//! and appends results; the model writes one order per round.

use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// One action instance inside a plan step (design §5: `{"step_id", "do",
/// "with"}` — the step-id binds the action to its owning step; `with` is the
/// action's parameter object).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAction {
    #[serde(rename = "step_id")]
    pub step_id: String,
    #[serde(rename = "do")]
    pub do_action: String,
    #[serde(rename = "with")]
    pub with: serde_json::Value,
}

/// A single step in the execution plan (PLAN-FIRST 阶段 A structured plan:
/// id / goal / actions / acceptance / evidence; status is mechanically
/// managed by the assistant layer, not by the model).
/// 2026-08-16 审查收口：旧 epoch 归档（description-only 步骤）经
/// `alias="description"` + serde 默认值兼容读取——升级前归档仍可回查，
/// 不静默丢弃（P2 修复，测试锁定）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    #[serde(default, alias = "description")]
    pub goal: String,
    #[serde(default)]
    pub actions: Vec<PlanAction>,
    #[serde(default)]
    pub acceptance: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub status: StepStatus,
}

/// 步骤状态机（PLAN-FIRST 阶段 C，2026-08-16；设计 §2.5/§6）：
/// `pending → in_progress → done(receipt_id) | failed(receipt_id)`。
/// 旧（阶段 A 前）归档中的单位变体字符串（`Pending` / `InProgress` /
/// `Completed` / `Blocked`）经自定义反序列化兼容读取——升级前 epoch
/// 快照不静默丢弃（与 `description` alias 同一纪律）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    InProgress,
    /// done(receipt_id) — 由订单 receipt 或 direct 证据门（§7.5）置位。
    Done(DoneEvidence),
    Failed(FailedEvidence),
    Blocked,
}

/// done 证据：订单 receipt（console 订单发放成功）或 direct 有记录例外
/// （`console_step_done` 的 transition_id + trace_id 交叉校验通过）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoneEvidence {
    pub receipt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct: Option<DirectStepEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectStepEvidence {
    pub transition_id: String,
    pub trace_id: String,
}

/// failed(receipt_id) — 订单发放失败（fail-closed 信封 receipt）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedEvidence {
    pub receipt_id: String,
}

impl StepStatus {
    /// 是否已完成（done 计为完成；旧归档兼容的 `Completed` 映射为
    /// 空证据 Done）。
    pub fn is_done(&self) -> bool {
        matches!(self, StepStatus::Done(_))
    }
}

impl serde::Serialize for StepStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        match self {
            StepStatus::Pending => serializer.serialize_str("pending"),
            StepStatus::InProgress => serializer.serialize_str("in_progress"),
            StepStatus::Blocked => serializer.serialize_str("blocked"),
            StepStatus::Done(evidence) => {
                let mut s = serializer.serialize_struct("StepStatus", 3)?;
                s.serialize_field("status", "done")?;
                s.serialize_field("receipt_id", &evidence.receipt_id)?;
                s.serialize_field("direct", &evidence.direct)?;
                s.end()
            }
            StepStatus::Failed(evidence) => {
                let mut s = serializer.serialize_struct("StepStatus", 2)?;
                s.serialize_field("status", "failed")?;
                s.serialize_field("receipt_id", &evidence.receipt_id)?;
                s.end()
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for StepStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum StepStatusRepr {
            String(String),
            Object {
                status: String,
                #[serde(default)]
                receipt_id: Option<String>,
                #[serde(default)]
                direct: Option<DirectStepEvidence>,
            },
        }
        match StepStatusRepr::deserialize(deserializer)? {
            StepStatusRepr::String(s) => match s.as_str() {
                "Pending" | "pending" => Ok(StepStatus::Pending),
                "InProgress" | "in_progress" => Ok(StepStatus::InProgress),
                // 旧归档单位变体：Completed → 空证据 Done（完成事实保留）。
                "Completed" => Ok(StepStatus::Done(DoneEvidence {
                    receipt_id: String::new(),
                    direct: None,
                })),
                "Blocked" | "blocked" => Ok(StepStatus::Blocked),
                other => Err(serde::de::Error::custom(format!(
                    "unknown StepStatus variant {other:?}"
                ))),
            },
            StepStatusRepr::Object {
                status,
                receipt_id,
                direct,
            } => match status.as_str() {
                "pending" => Ok(StepStatus::Pending),
                "in_progress" => Ok(StepStatus::InProgress),
                "blocked" => Ok(StepStatus::Blocked),
                "done" => Ok(StepStatus::Done(DoneEvidence {
                    receipt_id: receipt_id.unwrap_or_default(),
                    direct,
                })),
                "failed" => Ok(StepStatus::Failed(FailedEvidence {
                    receipt_id: receipt_id.unwrap_or_default(),
                })),
                other => Err(serde::de::Error::custom(format!(
                    "unknown StepStatus object status {other:?}"
                ))),
            },
        }
    }
}

/// Main agent writes: goal, steps, analysis, decisions, auth_grants.
///
/// v1.15 / BLACKBOARD_PLAN_EPOCH_DESIGN (2026-08-14): the plan section is
/// the single-writer plan identity holder — every approved plan carries
/// `plan_id` + `plan_epoch`. Same-plan revisions keep the same epoch and do
/// not rotate the blackboard; a new plan epoch (new plan_id) rotates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanSection {
    /// The currently approved plan's stable identity (`PLAN-…`).
    pub plan_id: Option<String>,
    /// The plan epoch — 0 = no approved plan yet; >0 = current epoch.
    pub plan_epoch: u64,
    pub goal: Option<String>,
    pub steps: Vec<PlanStep>,
    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the mechanical delivery
    /// status line rendered by the `submit` request — harness-computed
    /// workspace change list (tool output, never model-authored; no
    /// repetition-detector exposure). Rendered as an extra plan-view line
    /// when present; cleared by plan rotation (the section is replaced).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_status: Option<String>,
    pub analysis: Vec<String>,
    pub decisions: Vec<String>,
    pub auth_grants: Vec<String>,
}

/// Main agent writes: results, observations, errors, auth_requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecSection {
    pub results: Vec<String>,
    pub observations: Vec<String>,
    pub errors: Vec<String>,
    pub auth_requests: Vec<String>,
}

/// Internal retrieval subagent writes: project docs, source ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalRetSection {
    pub project_docs: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
}

/// External retrieval subagent writes: web sources, source ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRetSection {
    pub web_sources: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
}

/// Assurance writes: gate decisions, orientation checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateLog {
    pub gate_decisions: Vec<String>,
    pub orientation_checks: Vec<String>,
}

/// 编辑动作区 (blackboard partition, 2026-08-08): one deterministic file-edit
/// record — the line-range delta of a successful file-edit tool call. Written
/// by the controller after execution ("实际变动" 才记); never model-written.
/// Timestamps mirror the journal event's timestamp (events carry one, the
/// in-memory blackboard does not — so the record carries its own).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditRecord {
    pub file: String,
    /// Line count of the tool's `old_string` arg (0 for new-file creation).
    pub old_lines: usize,
    /// Line count of the tool's `new_string` arg.
    pub new_lines: usize,
    /// ISO 8601 timestamp (journal event timestamp of the tool_completed).
    pub timestamp: String,
}

/// 工具动作区 (blackboard partition, 2026-08-08): one classified tool action
/// per executed tool call, folded by category (read / edit / terminal /
/// retrieval) and timestamped — the model looks back via blackboard_read.
/// Denied calls record nothing (the action did not happen).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolActionRecord {
    pub category: String,
    pub tool: String,
    pub timestamp: String,
}

/// 注册板块（v0.5 操作台模型，P0-C orz 内嵌集成）：助理层机械刷新、模型只读
/// 的“按钮”投影——动作名 + 最小参数提示（type/required/属性枚举/默认值；
/// 不做完整 schema 复制，防上下文膨胀）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionRegistration {
    pub name: String,
    pub description: String,
    /// 输入契约的最小参数提示投影（由 `console::ServiceRegistry` 生成）。
    pub parameters: serde_json::Value,
    /// R2 服务调用形态收敛：实体级 target 策略（None=全局动作省略 target；
    /// File/Process/Environment/AnyEntity=订单必须携带对应实体 id）。
    #[serde(default)]
    pub target_policy: crate::entities::TargetPolicy,
}

/// 动作栏订单（v0.5 操作台模型）：模型写、写无副作用；发放后单槽清空。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionOrder {
    pub order_id: String,
    pub action: String,
    pub arguments: serde_json::Value,
    /// R2 服务调用形态收敛：动作作用对象实体 id（实体级 target；无作用
    /// 对象的全局动作省略）。由注册表 target_policy 机械校验。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): 订单绑定的计划
    /// 步骤 id（设计 §5：ActionOrder 增 step_id；Schema/事件/verifier 先行，
    /// 再接线 producer）。None = 无计划在案（直接订单不绑步骤）。
    #[serde(default)]
    pub step_id: Option<String>,
    /// 写单时的模型轮（round 防重放/过期）。
    pub round: u32,
    /// 写单时的 plan epoch（跨 epoch 语义隔离）。
    pub plan_epoch: u64,
    /// 写单时的 run id（P0-C S2：跨 run 防重放——run 结束遗留的订单在
    /// 下一 run 按 `order_stale` 显式拒绝，不会因轮号/epoch 重合被误发）。
    #[serde(default)]
    pub run_id: String,
}

/// 结果栏 receipt（v0.5 操作台模型）：助理层写；成功携带 response，
/// 失败携带 fail-closed 错误信封（step/code/message/upstream）+ trace_id。
/// `action` 为发放时订单的动作名（OUTPUT-DEGENERATION-GUARD 2026-08-19
/// 审查处理 P1：点读指针改向须按动作判定 run_terminal，响应信封形状
/// `{"output": string}` 被 read_file/grep/run_tests 等 text-output 动作
/// 共用，不能作为终端判据）；旧 epoch 归档缺该字段时 serde 回退 None
/// （点读走存档指针兜底，安全方向）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionResult {
    pub order_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
    pub trace_id: String,
    pub timestamp: String,
}

/// 黑板动作栏三板块（v0.5 用户提案，2026-08-13 定为生产协作形态）：
/// 注册板块（助理层维护）、动作栏（模型写订单，单轮一单）、结果栏
/// （助理层写 receipt）。本切片只落地数据面与单槽纪律；轮末发放接线下一切片。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ActionBoard {
    /// 当前轮可用动作投影（机械刷新、常驻按需读）。
    #[serde(default)]
    pub registration: Vec<ActionRegistration>,
    /// 未消费订单（单轮一单：已有 pending 时拒绝新单）。
    #[serde(default)]
    pub order: Option<ActionOrder>,
    /// 发放后的 receipts（有界，保留最近 50 条）。
    #[serde(default)]
    pub results: Vec<ActionResult>,
    /// PULL 自描述版本计数（2026-08-31；仅内存，不序列化）。
    #[serde(skip)]
    pub revision: u64,
}

impl ActionBoard {
    /// 结果栏保留上限（有界，防黑板无限膨胀）。
    pub const RESULTS_MAX: usize = 50;

    fn bump(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    /// 注册板块整块替换（助理层每轮机械刷新）。
    pub fn set_registration(&mut self, registration: Vec<ActionRegistration>) {
        // 2026-08-31 审查处理 M3：内容相同不计数（compare-and-set）——读取
        // actions 前的 `sync_console_registrations` 每读必刷，若无条件 bump
        // 会让每次读 actions 都带自触发的 `actions+1` 徽章，违背「自上次
        // 读取以来」语义（设计 §2：可见内容变化才计 1）。
        if self.registration == registration {
            return;
        }
        self.registration = registration;
        self.bump();
    }

    /// 模型写订单：单轮一单，已有 pending 订单时拒绝（fail-closed）。
    pub fn write_order(&mut self, order: ActionOrder) -> Result<(), ActionBoardError> {
        if self.order.is_some() {
            return Err(ActionBoardError::OrderSlotBusy);
        }
        self.order = Some(order);
        self.bump();
        Ok(())
    }

    /// 机械发放出口：取走唯一 pending 订单并清空单槽（消费一次）。
    pub fn take_order(&mut self) -> Option<ActionOrder> {
        let taken = self.order.take();
        if taken.is_some() {
            self.bump();
        }
        taken
    }

    /// 结果栏追加 receipt（有界：保留最近 `RESULTS_MAX` 条）。
    pub fn push_result(&mut self, result: ActionResult) {
        self.results.push(result);
        if self.results.len() > Self::RESULTS_MAX {
            let overflow = self.results.len() - Self::RESULTS_MAX;
            self.results.drain(..overflow);
        }
        self.bump();
    }
}

/// 动作栏单槽纪律错误（v0.5：本轮订单未发放完不进入下一轮写单）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionBoardError {
    OrderSlotBusy,
}

impl std::fmt::Display for ActionBoardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionBoardError::OrderSlotBusy => {
                write!(f, "action bar already holds a pending order")
            }
        }
    }
}

impl std::error::Error for ActionBoardError {}

/// The full blackboard with 5 sections + 2 controller-written partitions +
/// the console action board (assistant writes registration/results, the
/// model writes the single order slot).
///
/// Read rule: all sections are readable by all agents.
/// Write rule: each section has a single writer (enforced by convention).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Blackboard {
    pub plan: PlanSection,
    pub exec: ExecSection,
    pub internal_ret: InternalRetSection,
    pub external_ret: ExternalRetSection,
    pub gate_log: GateLog,
    /// 编辑动作区 — controller-written after successful file edits.
    pub edits: Vec<EditRecord>,
    /// 工具动作区 — controller-written per executed tool call.
    pub tool_actions: Vec<ToolActionRecord>,
    /// 操作台动作栏三板块（v0.5；P0-C orz 内嵌集成 S1）。
    #[serde(default)]
    pub actions: ActionBoard,
    /// R2 半助理层实体状态分区（process/file/environment；半助理层执行
    /// 工具时登记/更新，模型经 `blackboard_read section=entities` 按需
    /// 点读——不新增只读工具）。live-only：不进 epoch 快照（同检索分区
    /// 纪律，随 run 生命周期）。
    #[serde(default)]
    pub entities: crate::entities::EntityRegistry,
    /// 依赖图最小范围（P2-11 第 4 项 / 依赖图主线设计 2026-09-01）：
    /// 文件锚点链（read→write 锚点边 + 工具→实体变更边；D3 命令/检索
    /// 副作用不建图）。live-only：不进 epoch 快照、不持久化（同
    /// entities/temporal 纪律）；模型经 `blackboard_read section=deps`
    /// 按需 PULL。
    #[serde(default)]
    pub dep_graph: crate::dep_graph::DepGraph,
    /// PULL 自描述分区版本计数（2026-08-31，P2-11 第 1 项）——每个分区
    /// 可见内容变化计 1 次，供 `blackboard_read` 增量头读取；仅内存、
    /// 不进任何序列化面（`#[serde(skip)]`，epoch 快照/会话存档不携带）。
    #[serde(skip)]
    pub revisions: PartitionRevisions,
}

/// 分区版本计数（PULL 自描述 §2）。`actions` / `entities` 的自含计数
/// 由各自结构体持有（`ActionBoard.revision` / `EntityRegistry.revision`），
/// 不在本表重复；本表只记黑板直属分区。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartitionRevisions {
    pub plan: u64,
    pub exec: u64,
    pub edits: u64,
    pub tool_actions: u64,
    pub internal_ret: u64,
    pub external_ret: u64,
}

impl Blackboard {
    pub fn new() -> Self {
        Blackboard::default()
    }

    /// Capture the current plan-epoch-scoped work state as an archive
    /// snapshot — plan + edits + tool_actions + exec. Gate log, whitelist
    /// and the retrieval partitions are NOT part of an epoch snapshot
    /// (they survive rotation by design).
    pub fn epoch_snapshot(&self, persisted_at: &str) -> EpochSnapshot {
        EpochSnapshot {
            plan_id: self.plan.plan_id.clone(),
            plan_epoch: self.plan.plan_epoch,
            plan: self.plan.clone(),
            edits: self.edits.clone(),
            tool_actions: self.tool_actions.clone(),
            exec: self.exec.clone(),
            actions: self.actions.clone(),
            persisted_at: persisted_at.to_string(),
        }
    }

    /// Restore a persisted epoch snapshot into the live partitions. The
    /// retrieval partitions / gate log / whitelist are left untouched.
    pub fn restore_epoch_snapshot(&mut self, snapshot: &EpochSnapshot) {
        self.plan = snapshot.plan.clone();
        self.edits = snapshot.edits.clone();
        self.tool_actions = snapshot.tool_actions.clone();
        self.exec = snapshot.exec.clone();
        self.actions = snapshot.actions.clone();
        // PULL 自描述：恢复 = 可见内容整体替换，各分区计 1 次变化。
        self.revisions.plan = self.revisions.plan.saturating_add(1);
        self.revisions.edits = self.revisions.edits.saturating_add(1);
        self.revisions.tool_actions = self.revisions.tool_actions.saturating_add(1);
        self.revisions.exec = self.revisions.exec.saturating_add(1);
        self.actions.bump();
    }

    /// 编辑记录追加（单写者纪律 + 分区版本计数同一落点）。
    pub fn push_edit(&mut self, record: EditRecord) {
        self.edits.push(record);
        self.revisions.edits = self.revisions.edits.saturating_add(1);
    }

    /// 工具动作记录追加（单写者纪律 + 分区版本计数同一落点）。
    pub fn push_tool_action(&mut self, record: ToolActionRecord) {
        self.tool_actions.push(record);
        self.revisions.tool_actions = self.revisions.tool_actions.saturating_add(1);
    }

    /// exec 结果追加（单写者纪律 + 分区版本计数同一落点）。
    pub fn push_exec_result(&mut self, text: String) {
        self.exec.results.push(text);
        self.revisions.exec = self.revisions.exec.saturating_add(1);
    }

    /// exec 错误追加（单写者纪律 + 分区版本计数同一落点）。
    pub fn push_exec_error(&mut self, text: String) {
        self.exec.errors.push(text);
        self.revisions.exec = self.revisions.exec.saturating_add(1);
    }

    /// plan 分区零散写点（mark_step_* / delivery_status 等）的计数入口。
    pub fn bump_plan(&mut self) {
        self.revisions.plan = self.revisions.plan.saturating_add(1);
    }

    /// 检索分区覆盖写（`agents::retrieval::write_section`）的计数入口。
    pub fn bump_retrieval(&mut self, section: &str) {
        match section {
            "internal_ret" => {
                self.revisions.internal_ret = self.revisions.internal_ret.saturating_add(1);
            }
            "external_ret" => {
                self.revisions.external_ret = self.revisions.external_ret.saturating_add(1);
            }
            _ => {}
        }
    }

    /// PULL 自描述 §2：固定分区序的黑板直属版本快照（live-only）。
    /// session / temporal 由控制器按 tool_rounds / LIF round 派生。
    pub fn partition_revisions(&self) -> Vec<(&'static str, u64)> {
        vec![
            ("plan", self.revisions.plan),
            ("exec", self.revisions.exec),
            ("edits", self.revisions.edits),
            ("tool_actions", self.revisions.tool_actions),
            ("actions", self.actions.revision),
            ("internal_ret", self.revisions.internal_ret),
            ("external_ret", self.revisions.external_ret),
            ("entities", self.entities.revision()),
            ("deps", self.dep_graph.revision()),
        ]
    }

    /// Plan-epoch rotation (ADR-0010 §14.15 / BLACKBOARD_PLAN_EPOCH_DESIGN).
    ///
    /// - Same `plan_id` → same-epoch revision: the plan goal/steps are
    ///   replaced, nothing is cleared, `Ok(None)` is returned (no archive).
    /// - New `plan_id` → new epoch: the old epoch-scoped state is captured
    ///   into an `EpochSnapshot` (`Ok(Some(..))` — the caller archives it),
    ///   the plan section is replaced and edits / tool_actions / exec are
    ///   cleared atomically under one write lock. Gate log, whitelist and
    ///   retrieval partitions survive.
    ///
    /// Identity invariants (v1.15⑧, 2026-08-15 — one-to-one, no misuse):
    /// - `plan_epoch` must be ≥ 1;
    /// - same `plan_id` must reuse the SAME `plan_epoch` (a revision never
    ///   advances the epoch — the caller passes the stored epoch back);
    /// - a new `plan_id` must advance to a STRICTLY GREATER `plan_epoch`
    ///   (timestamp-stamped monotonic numbering, `epoch::next_plan_epoch_
    ///   from_archive`).
    /// Violations return `Err` BEFORE any mutation — the board is untouched.
    pub fn rotate_to_plan(
        &mut self,
        plan_id: String,
        plan_epoch: u64,
        goal: String,
        steps: Vec<String>,
        persisted_at: &str,
    ) -> Result<Option<EpochSnapshot>, PlanEpochError> {
        self.rotate_impl(
            plan_id,
            plan_epoch,
            goal,
            Self::steps_from_descriptions(steps),
            persisted_at,
        )
    }

    /// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): same plan-epoch
    /// rotation semantics as [`Self::rotate_to_plan`], but accepts the
    /// structured steps from the first-round `plan_write` gate (design §5).
    /// Step statuses are taken as provided — the gate submits `pending`
    /// steps; the assistant layer owns status transitions later.
    pub fn rotate_to_structured_plan(
        &mut self,
        plan_id: String,
        plan_epoch: u64,
        goal: String,
        steps: Vec<PlanStep>,
        persisted_at: &str,
    ) -> Result<Option<EpochSnapshot>, PlanEpochError> {
        self.rotate_impl(plan_id, plan_epoch, goal, steps, persisted_at)
    }

    /// Legacy description-only steps (pre-PLAN-FIRST plan mode): first step
    /// in-progress, the rest pending — preserves the old status-line
    /// behavior for `with_plan` call sites.
    fn steps_from_descriptions(steps: Vec<String>) -> Vec<PlanStep> {
        use crate::blackboard::StepStatus;
        steps
            .into_iter()
            .enumerate()
            .map(|(i, description)| PlanStep {
                id: format!("step-{}", i + 1),
                goal: description,
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: if i == 0 {
                    StepStatus::InProgress
                } else {
                    StepStatus::Pending
                },
            })
            .collect()
    }

    fn rotate_impl(
        &mut self,
        plan_id: String,
        plan_epoch: u64,
        goal: String,
        steps: Vec<PlanStep>,
        persisted_at: &str,
    ) -> Result<Option<EpochSnapshot>, PlanEpochError> {
        if plan_epoch == 0 {
            return Err(PlanEpochError::ZeroEpoch);
        }
        if let Some(current_id) = self.plan.plan_id.as_deref() {
            if current_id == plan_id.as_str() {
                if plan_epoch != self.plan.plan_epoch {
                    return Err(PlanEpochError::SamePlanEpochMismatch {
                        plan_id,
                        expected: self.plan.plan_epoch,
                        got: plan_epoch,
                    });
                }
                // Same-epoch revision: no rotation, no clearing.
                self.plan.goal = Some(goal);
                self.plan.steps.clear();
                self.plan.steps.extend(steps);
                self.revisions.plan = self.revisions.plan.saturating_add(1);
                return Ok(None);
            }
            if plan_epoch <= self.plan.plan_epoch {
                return Err(PlanEpochError::NewPlanEpochNotGreater {
                    current_epoch: self.plan.plan_epoch,
                    got: plan_epoch,
                });
            }
        }
        let old = if self.plan.plan_epoch > 0 {
            Some(self.epoch_snapshot(persisted_at))
        } else {
            None
        };
        self.plan = PlanSection {
            plan_id: Some(plan_id),
            plan_epoch,
            goal: Some(goal),
            steps: Vec::new(),
            delivery_status: None,
            analysis: Vec::new(),
            decisions: Vec::new(),
            auth_grants: Vec::new(),
        };
        self.plan.steps.extend(steps);
        self.edits.clear();
        self.tool_actions.clear();
        self.exec = ExecSection::default();
        self.actions = ActionBoard::default();
        // PULL 自描述：轮换 = 计划替换 + 旧 epoch 工作分区清空，各计 1 次。
        self.revisions.plan = self.revisions.plan.saturating_add(1);
        self.revisions.edits = self.revisions.edits.saturating_add(1);
        self.revisions.tool_actions = self.revisions.tool_actions.saturating_add(1);
        self.revisions.exec = self.revisions.exec.saturating_add(1);
        self.actions.bump();
        Ok(old)
    }
}

/// Plan-epoch identity violation (v1.15⑧, 2026-08-15). The one-to-one
/// `plan_id ↔ plan_epoch` mapping is enforced at the rotation boundary;
/// callers that compute epochs via `epoch::next_plan_epoch_from_archive`
/// can only violate it through a programming error, so the API rejects it
/// instead of silently reinterpreting the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanEpochError {
    /// `plan_epoch == 0` — no approved plan has epoch 0.
    ZeroEpoch,
    /// Same `plan_id` passed with a different `plan_epoch`: revisions must
    /// reuse the epoch recorded with the plan_id.
    SamePlanEpochMismatch {
        plan_id: String,
        expected: u64,
        got: u64,
    },
    /// New `plan_id` passed without a strictly greater `plan_epoch`
    /// (timestamp-stamped monotonic numbering forbids reuse/regression).
    NewPlanEpochNotGreater { current_epoch: u64, got: u64 },
}

impl std::fmt::Display for PlanEpochError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanEpochError::ZeroEpoch => write!(f, "plan_epoch must be ≥ 1"),
            PlanEpochError::SamePlanEpochMismatch {
                plan_id,
                expected,
                got,
            } => write!(
                f,
                "plan_id {plan_id} already owns plan_epoch {expected}; revision passed {got} \
                 (same plan_id must reuse the same epoch)"
            ),
            PlanEpochError::NewPlanEpochNotGreater { current_epoch, got } => write!(
                f,
                "new plan_id must advance beyond current plan_epoch {current_epoch}; got {got}"
            ),
        }
    }
}

impl std::error::Error for PlanEpochError {}

/// Deterministic archive of one plan epoch (v1.15, 2026-08-14): the full
/// path/action records of an epoch at its rotation moment. Written to
/// `.gsa/blackboard/epoch-<plan_epoch>.json`; read back for cross-epoch
/// `blackboard_read` and restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochSnapshot {
    pub plan_id: Option<String>,
    pub plan_epoch: u64,
    pub plan: PlanSection,
    pub edits: Vec<EditRecord>,
    pub tool_actions: Vec<ToolActionRecord>,
    pub exec: ExecSection,
    /// 操作台动作栏（v0.5；P0-C S1）——随 epoch 归档/恢复。
    #[serde(default)]
    pub actions: ActionBoard,
    /// When this snapshot was PERSISTED (approval/revision refresh or
    /// rotation). F9 (2026-08-15, BACKLOG 6e 复查遗留): the old name
    /// `rotated_at` misleadingly implied rotation-only — the current-epoch
    /// persistence happens at every approval/revision. `alias` keeps old
    /// archives loadable (the JSON key is `persisted_at` for new writes).
    #[serde(alias = "rotated_at")]
    pub persisted_at: String,
}

/// Thread-safe wrapper around the Blackboard.
///
/// All agents share the same `SharedBlackboard` via `Arc<SharedBlackboard>`.
pub struct SharedBlackboard {
    inner: RwLock<Blackboard>,
}

impl SharedBlackboard {
    pub fn new() -> Self {
        SharedBlackboard {
            inner: RwLock::new(Blackboard::new()),
        }
    }

    /// Read the entire blackboard (shared access).
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Blackboard> {
        self.inner.read().unwrap()
    }

    /// Mutate the blackboard (exclusive access).
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, Blackboard> {
        self.inner.write().unwrap()
    }
}

impl Default for SharedBlackboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{ModelGateway, ToolCall};
    use crate::host::ToolResult;
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;

    #[test]
    fn shared_blackboard_read_write() {
        let bb = SharedBlackboard::new();

        // Write
        {
            let mut w = bb.write();
            w.plan.goal = Some("test goal".into());
            w.plan.steps.push(PlanStep {
                id: "step-1".into(),
                goal: "do something".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::Pending,
            });
        }

        // Read
        let r = bb.read();
        assert_eq!(r.plan.goal.as_deref(), Some("test goal"));
        assert_eq!(r.plan.steps.len(), 1);
    }

    /// 2026-08-16 审查收口：旧（description-only）PlanStep 归档仍可反序列化
    /// ——description 映射到 goal，新字段取默认值，状态保留（升级前 epoch
    /// 快照不静默丢弃）。
    #[test]
    fn legacy_description_plan_step_deserializes_with_compat() {
        let old: PlanStep =
            serde_json::from_str(r#"{"id":"step-1","description":"旧步骤","status":"InProgress"}"#)
                .expect("legacy plan step must deserialize");
        assert_eq!(old.id, "step-1");
        assert_eq!(old.goal, "旧步骤");
        assert!(old.actions.is_empty());
        assert!(old.acceptance.is_empty());
        assert!(old.evidence.is_empty());
        assert_eq!(old.status, StepStatus::InProgress);
    }

    #[test]
    fn gate_log_append() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.gate_log.gate_decisions.push("IPG: pass".into());
            w.gate_log
                .orientation_checks
                .push("checkpoint: no fire".into());
        }
        let r = bb.read();
        assert_eq!(r.gate_log.gate_decisions.len(), 1);
        assert_eq!(r.gate_log.orientation_checks.len(), 1);
    }

    #[test]
    fn action_board_single_order_slot_and_consume_once() {
        let bb = SharedBlackboard::new();
        let order = ActionOrder {
            order_id: "ORD-1".into(),
            action: "workspace.read_file".into(),
            arguments: serde_json::json!({"path": "a.txt"}),
            target: None,
            step_id: None,
            round: 1,
            plan_epoch: 1,
            run_id: "RUN-1".into(),
        };
        {
            let mut w = bb.write();
            assert!(w.actions.write_order(order.clone()).is_ok());
            assert_eq!(
                w.actions.write_order(order.clone()),
                Err(ActionBoardError::OrderSlotBusy)
            );
        }
        {
            let mut w = bb.write();
            let taken = w.actions.take_order();
            assert_eq!(taken, Some(order));
            assert!(w.actions.order.is_none());
            assert!(
                w.actions
                    .write_order(ActionOrder {
                        order_id: "ORD-2".into(),
                        action: "workspace.list_dir".into(),
                        arguments: serde_json::json!({"path": "."}),
                        target: None,
                        step_id: None,
                        round: 2,
                        plan_epoch: 1,
                        run_id: "RUN-1".into(),
                    })
                    .is_ok()
            );
        }
    }

    #[test]
    fn action_results_bounded_and_registration_replaceable() {
        let mut board = ActionBoard::default();
        board.set_registration(vec![ActionRegistration {
            name: "workspace.read_file".into(),
            description: "read a file".into(),
            parameters: serde_json::json!({"required": ["path"]}),
            target_policy: crate::entities::TargetPolicy::None,
        }]);
        for i in 0..(ActionBoard::RESULTS_MAX + 5) {
            board.push_result(ActionResult {
                order_id: format!("ORD-{i}"),
                action: Some("workspace.read_file".into()),
                ok: true,
                response: Some(serde_json::json!({})),
                error: None,
                trace_id: format!("t{i:06}"),
                timestamp: "2026-08-15T00:00:00Z".into(),
            });
        }
        assert_eq!(board.results.len(), ActionBoard::RESULTS_MAX);
        assert!(board.results[0].order_id.starts_with("ORD-5"));
        board.set_registration(Vec::new());
        assert!(board.registration.is_empty());
    }

    #[test]
    fn action_board_rotates_with_plan_epoch_and_snapshot_round_trips() {
        let mut bb = Blackboard::new();
        bb.rotate_to_plan(
            "PLAN-1".into(),
            1,
            "first".into(),
            vec!["step".into()],
            "2026-08-15T00:00:00Z",
        )
        .unwrap();
        bb.actions.set_registration(vec![ActionRegistration {
            name: "workspace.read_file".into(),
            description: "read a file".into(),
            parameters: serde_json::json!({}),
            target_policy: crate::entities::TargetPolicy::None,
        }]);
        bb.actions
            .write_order(ActionOrder {
                order_id: "ORD-1".into(),
                action: "workspace.read_file".into(),
                arguments: serde_json::json!({"path": "a.txt"}),
                target: None,
                step_id: None,
                round: 1,
                plan_epoch: 1,
                run_id: "RUN-1".into(),
            })
            .unwrap();
        let snap = bb.epoch_snapshot("2026-08-15T00:00:00Z");
        assert!(snap.actions.order.is_some());

        let mut restored = Blackboard::new();
        restored.restore_epoch_snapshot(&snap);
        assert_eq!(
            restored.actions.order.unwrap().action,
            "workspace.read_file"
        );

        let old = bb
            .rotate_to_plan(
                "PLAN-2".into(),
                2,
                "next".into(),
                vec!["step".into()],
                "2026-08-15T00:00:01Z",
            )
            .unwrap();
        assert!(old.is_some());
        assert!(bb.actions.order.is_none());
        assert!(bb.actions.registration.is_empty());
        assert!(bb.actions.results.is_empty());
    }

    /// v1.15 (2026-08-14): `blackboard_read` with `epoch` reads the archived
    /// snapshot; a missing epoch / unconfigured archive is explicit — never
    /// a silent fallback to the live board.
    #[tokio::test]
    async fn blackboard_read_cross_epoch_and_restore() {
        let dir = test_dir().join("gsa").join("blackboard");
        let controller =
            AgentLoopController::with_gateway(Arc::new(FakeProvider::from_texts(vec!["ok", "ok"])))
                .with_blackboard_archive_dir(Some(dir.clone()))
                .with_plan(
                    "PLAN-RESTORE-A".to_string(),
                    1,
                    "旧任务".to_string(),
                    vec!["旧步骤".to_string()],
                );
        {
            let mut bb = controller.blackboard().write();
            bb.edits.push(EditRecord {
                file: "old.py".into(),
                old_lines: 1,
                new_lines: 2,
                timestamp: "2026-08-14T00:00:00Z".into(),
            });
        }
        // Rotate: epoch-1 archived with the old plan + edit record.
        let controller = controller.with_plan(
            "PLAN-RESTORE-B".to_string(),
            2,
            "新任务".to_string(),
            vec!["新步骤".to_string()],
        );
        let live = controller.render_blackboard_section("plan", None, None, None);
        assert!(live.contains("新任务"));
        assert!(live.contains("plan_epoch: 2"));
        // P0-E 计划视图补渲染步骤 ID (2026-08-17, ADR-0010 §14.21 项 2):
        // both the live view and the archived epoch read carry each step's
        // id as the leading token — the console step gate's exact `step_id`
        // binding is visible without guessing.
        assert!(
            live.contains("- [in-progress] step-1: 新步骤 (actions: 0; evidence: 0)"),
            "live plan view must render step id: {live}"
        );
        let archived = controller.render_blackboard_section("edits", None, Some(1), None);
        assert!(archived.contains("old.py"), "cross-epoch read: {archived}");
        let archived_plan = controller.render_blackboard_section("plan", None, Some(1), None);
        assert!(
            archived_plan.contains("- [in-progress] step-1: 旧步骤 (actions: 0; evidence: 0)"),
            "archived plan view must render step id: {archived_plan}"
        );
        let missing = controller.render_blackboard_section("plan", None, Some(99), None);
        assert!(missing.contains("epoch snapshot 99 not found"));

        // A new controller with the same archive dir restores the latest
        // epoch (the restore entry).
        let restored = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_blackboard_archive_dir(Some(dir.clone()));
        let r = restored.blackboard().read();
        assert_eq!(r.plan.plan_id.as_deref(), Some("PLAN-RESTORE-B"));
        assert_eq!(r.plan.plan_epoch, 2);
        assert_eq!(r.plan.goal.as_deref(), Some("新任务"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-E 计划视图补渲染步骤 ID (2026-08-17, ADR-0010 §14.21 项 2): the
    /// actual `blackboard_read section=plan` tool reply reaches the model
    /// with each step's id — the console step gate's exact `step_id`
    /// binding is visible, no guessing.
    #[tokio::test]
    async fn blackboard_read_serves_plan_partition_with_step_ids() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "plan"}),
                call_id: "call-pv1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_plan(
            "PLAN-VIEW-TEST".to_string(),
            1,
            "构建 ELF".to_string(),
            vec!["侦查源码".to_string(), "构建并验证".to_string()],
        );
        controller
            .run_turn(
                &host,
                "读计划视图",
                "RUN-PV1",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-pv1"))
            })
            .expect("round carrying blackboard_read plan reply");
        assert!(
            round
                .messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-pv1")
                    && m.content
                        .contains("- [in-progress] step-1: 侦查源码 (actions: 0; evidence: 0)")),
            "plan reply must render the current step id: {:?}",
            round.messages
        );
        assert!(
            round
                .messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-pv1")
                    && m.content
                        .contains("- [pending] step-2: 构建并验证 (actions: 0; evidence: 0)")),
            "plan reply must render every step id: {:?}",
            round.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PUSH→PULL (2026-08-21, CONTEXT_SCAFFOLDING_PULL_REDESIGN §4 方案 A):
    /// `blackboard_read section=session` 经真实工具链回达模型——live 预算面
    /// （已用/剩余 + 状态行）；同时校验工具定义已声明 session 分区
    /// （工具定义增量扩展）。
    #[tokio::test]
    async fn blackboard_read_serves_session_section() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "session"}),
                call_id: "call-se1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_max_tool_rounds(120)
            .with_plan(
                "PLAN-SES-T".to_string(),
                1,
                "构建".to_string(),
                vec!["侦查".to_string()],
            );
        controller
            .run_turn(&host, "读会话面", "RUN-SES1", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-se1"))
            })
            .expect("round carrying blackboard_read session reply");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-se1"))
            .expect("session tool result message");
        assert!(
            reply.content.contains("TOOL_ROUNDS_USED: 0")
                && reply.content.contains("TOOL_ROUNDS_REMAINING: 120")
                && reply
                    .content
                    .contains("TOOL_ROUND_BUDGET: 120 tool rounds per turn")
                && reply.content.contains("[任务状态 v0.1]"),
            "session reply: {:?}",
            round.messages
        );
        // 工具定义增量扩展：blackboard_read 的 section 枚举含 session。
        let bb_def = round
            .tools
            .iter()
            .find(|t| t.name == "blackboard_read")
            .expect("blackboard_read declared in request tools");
        let sections = bb_def
            .parameters
            .get("properties")
            .and_then(|p| p.get("section"))
            .and_then(|s| s.get("enum"))
            .and_then(|e| e.as_array())
            .expect("section enum declared");
        assert!(
            sections.iter().any(|v| v.as_str() == Some("session")),
            "session must be declared in the section enum: {sections:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-10 F2 §3.3 (I3, 2026-08-30): `blackboard_read section=temporal`
    /// reaches the model through the real tool chain — the LIF engine is
    /// fed on the decision round (model_output with tool_calls) and the
    /// completed tool event, and the render returns the Now row without
    /// any fires.
    #[tokio::test]
    async fn blackboard_read_serves_temporal_section() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "temporal", "selector": "now"}),
                call_id: "call-tp1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_max_tool_rounds(120);
        controller
            .run_turn(&host, "读时间面", "RUN-TP1", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-tp1"))
            })
            .expect("round carrying blackboard_read temporal reply");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-tp1"))
            .expect("temporal tool result message");
        assert!(
            reply.content.contains("temporal.now"),
            "temporal reply: {:?}",
            round.messages
        );
        // fires never render — no fire line in the board.
        assert!(
            !reply.content.contains("fire"),
            "fires must not render: {:?}",
            reply.content
        );
        // P2-10 F1 §2.2 ⑦ / F2 §3.3 (I5): the temporal board is bounded
        // ≤ 1 KiB even for the widest query (Recent(k=20)).
        let board = controller
            .render_temporal_section(Some("recent"), Some(20), None)
            .unwrap();
        assert!(
            board.len() <= 1024,
            "temporal board must be ≤ 1 KiB (got {} bytes)",
            board.len()
        );
        // 工具定义增量扩展：blackboard_read 的 section 枚举含 temporal。
        let bb_def = round
            .tools
            .iter()
            .find(|t| t.name == "blackboard_read")
            .expect("blackboard_read declared in request tools");
        let sections = bb_def
            .parameters
            .get("properties")
            .and_then(|p| p.get("section"))
            .and_then(|s| s.get("enum"))
            .and_then(|e| e.as_array())
            .expect("section enum declared");
        assert!(
            sections.iter().any(|v| v.as_str() == Some("temporal")),
            "temporal must be declared in the section enum: {sections:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-10 阶段 3 V1 §6.6 (2026-08-31): 模型熟悉度近零提示——temporal 分区
    /// 渲染必须自描述：字段标签/值/语义域字符串直接内联，模型无需外部词汇表
    /// 即可单轮正确消费（P3 验收原话；无需实机）。四个查询面（now/recent/
    /// history/feature）都验证标签存在且 fires 永不渲染（§9.7）。
    #[tokio::test]
    async fn temporal_render_is_self_describing_for_near_zero_prompt() {
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
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "temporal", "selector": "now"}),
                call_id: "call-v1b-1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.rs"}),
                call_id: "call-v1b-2".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "grep".to_string(),
                arguments: serde_json::json!({"pattern": "fn"}),
                call_id: "call-v1b-3".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_max_tool_rounds(120);
        controller
            .run_turn(&host, "读时间面", "RUN-V1B", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let now = controller
            .render_temporal_section(Some("now"), None, None)
            .unwrap();
        for label in [
            "temporal.now",
            "u_prog=",
            "u_err=",
            "u_stuck=",
            "err10=",
            "succ10=",
            "入域",
            "驻留",
        ] {
            assert!(now.contains(label), "now render missing {label}: {now}");
        }
        assert!(
            ["start", "normal", "pressure", "low_progress", "stuck"]
                .iter()
                .any(|d| now.contains(d)),
            "now render must carry the semantic domain string: {now}"
        );

        let recent = controller
            .render_temporal_section(Some("recent"), Some(20), None)
            .unwrap();
        assert!(
            recent.contains("temporal.recent"),
            "recent header: {recent}"
        );
        assert!(
            recent.contains("u_prog="),
            "recent rows carry labels: {recent}"
        );

        let history = controller
            .render_temporal_section(Some("history"), None, None)
            .unwrap();
        assert!(
            history.contains("temporal.history"),
            "history header: {history}"
        );

        let feature = controller
            .render_temporal_section(Some("feature"), Some(5), Some("u_prog"))
            .unwrap();
        assert!(
            feature.contains("temporal.feature"),
            "feature header: {feature}"
        );

        for board in [&now, &recent, &history, &feature] {
            assert!(
                !board.contains("fire"),
                "fires must never render (§9.7): {board}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-10 F2 §3.3 (I3): an invalid temporal selector fails loud (exit
    /// code 1 + explicit error), never a silent fallback.
    #[tokio::test]
    async fn blackboard_read_temporal_invalid_selector_fails_loud() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "temporal", "selector": "bogus"}),
                call_id: "call-tp2".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_max_tool_rounds(120);
        controller
            .run_turn(&host, "读时间面", "RUN-TP2", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-tp2"))
            })
            .expect("round carrying temporal reply");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-tp2"))
            .expect("temporal tool result message");
        assert!(
            reply.content.contains("invalid temporal selector"),
            "invalid selector must fail loud: {:?}",
            reply.content
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PUSH→PULL (2026-08-21) 全面审查处理（O4/O5）：session 面的越权组合
    /// （epoch / receipt_id）走参数级显式报错——事件 ToolCompleted exit_code
    /// 1 + error 字段、工具结果 exit_code 1、错误文本回达模型（与非法
    /// epoch/receipt_id 同纪律，绝不静默回退）。
    #[tokio::test]
    async fn blackboard_read_session_combination_errors_are_explicit() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "session", "epoch": 1}),
                call_id: "call-se-e".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "session", "receipt_id": "ORD-1"}),
                call_id: "call-se-r".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "越权组合",
                "RUN-SESCOMBO",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 事件面：两个 ToolCompleted 均 exit_code 1 且带 error 字段。
        let payloads: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        assert_eq!(payloads.len(), 2, "{payloads:?}");
        let epoch_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-se-e")
            .expect("epoch combo completed");
        assert_eq!(epoch_payload["exit_code"], 1, "{epoch_payload:?}");
        assert!(
            epoch_payload["error"]
                .as_str()
                .unwrap()
                .contains("session 面是 live 会话状态"),
            "{epoch_payload:?}"
        );
        let receipt_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-se-r")
            .expect("receipt combo completed");
        assert_eq!(receipt_payload["exit_code"], 1, "{receipt_payload:?}");
        assert!(
            receipt_payload["error"]
                .as_str()
                .unwrap()
                .contains("receipt_id 仅与 section=actions"),
            "{receipt_payload:?}"
        );

        // 模型面：错误文本回达模型。
        let received = fake.received_requests();
        let epoch_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-se-e"))
            })
            .expect("epoch combo round");
        assert!(
            epoch_round.messages.iter().any(|m| {
                m.tool_call_id.as_deref() == Some("call-se-e")
                    && m.content.contains("session 面是 live 会话状态")
            }),
            "{:?}",
            epoch_round.messages
        );
        let receipt_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-se-r"))
            })
            .expect("receipt combo round");
        assert!(
            receipt_round.messages.iter().any(|m| {
                m.tool_call_id.as_deref() == Some("call-se-r")
                    && m.content.contains("receipt_id 仅与 section=actions")
            }),
            "{:?}",
            receipt_round.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / 设计 §4.1+§4.2，S2
    /// 工具级断言）：`blackboard_read` 经真实工具链回达模型的 actions/exec
    /// 分区载荷已瘦身——结果板不再携带 response JSON / 完整 error 对象
    /// （固定形态行，缺失回退 `?`）；exec 行截断 200 字符、段总长受 4K
    /// 字符上限约束（超限仅头行 + 计数行）。
    #[tokio::test]
    async fn blackboard_read_serves_slimmed_actions_and_exec_sections() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "actions"}),
                call_id: "call-slim-a".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "exec"}),
                call_id: "call-slim-e".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        {
            let mut bb = controller.blackboard().write();
            bb.actions.push_result(ActionResult {
                order_id: "ORD-SLIM-1".into(),
                action: Some("workspace.run_terminal".into()),
                ok: true,
                response: Some(serde_json::json!({"output": "x".repeat(5_000)})),
                error: None,
                trace_id: "t-slim-1".into(),
                timestamp: "2026-08-19T00:00:00Z".into(),
            });
            bb.actions.push_result(ActionResult {
                order_id: "ORD-SLIM-2".into(),
                action: Some("workspace.run_tests".into()),
                ok: false,
                response: None,
                error: Some(serde_json::json!({
                    "step": "execute",
                    "code": "boom",
                    "message": "y".repeat(4_000),
                })),
                trace_id: "t-slim-2".into(),
                timestamp: "2026-08-19T00:00:01Z".into(),
            });
            // 40 条超长 exec 行：逐行截断后段总长仍超 4K → 头行 + 计数行。
            for _ in 0..40 {
                bb.exec.results.push("z".repeat(500));
            }
        }
        controller
            .run_turn(&host, "看黑板", "RUN-SLIM", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let actions_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-slim-a"))
            })
            .expect("round carrying blackboard_read actions reply");
        let actions_reply = actions_round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-slim-a"))
            .expect("actions tool result message");
        assert!(
            actions_reply
                .content
                .contains("ORD-SLIM-1 ok=true step=? code=? trace_id=t-slim-1"),
            "{:?}",
            actions_round.messages
        );
        assert!(
            actions_reply
                .content
                .contains("ORD-SLIM-2 ok=false step=execute code=boom trace_id=t-slim-2"),
            "{:?}",
            actions_round.messages
        );
        assert!(
            !actions_reply.content.contains("xxxx"),
            "response JSON must not reach the model: {:?}",
            actions_round.messages
        );
        assert!(
            !actions_reply.content.contains("response="),
            "{:?}",
            actions_round.messages
        );

        let exec_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-slim-e"))
            })
            .expect("round carrying blackboard_read exec reply");
        let exec_reply = exec_round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-slim-e"))
            .expect("exec tool result message");
        assert!(
            exec_reply
                .content
                .contains("[exec: 共 40 条，未省略；明细超 4K 字符上限]"),
            "{:?}",
            exec_round.messages
        );
        assert!(
            exec_reply
                .content
                .contains("完整内容见 blackboard_read 分区 exec 与存档"),
            "{:?}",
            exec_round.messages
        );
        // PULL 自描述（2026-08-31）：成功 live 读取响应首行为「自上次读取
        // 以来」增量头，正文仍为头行 + 计数行（共 3 行）。
        assert!(
            exec_reply
                .content
                .lines()
                .next()
                .unwrap()
                .starts_with("[黑板增量]"),
            "{:?}",
            exec_round.messages
        );
        assert_eq!(
            exec_reply.content.lines().count(),
            3,
            "{:?}",
            exec_round.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B，S2
    /// 工具级断言）：`blackboard_read` 携带 `receipt_id` 经真实工具链回达——
    /// 单条 receipt 点读的完整 response 全文到达模型（瘦身整段不注入大载荷）。
    #[tokio::test]
    async fn blackboard_read_receipt_point_read_reaches_model() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({
                    "section": "actions",
                    "receipt_id": "ORD-PR-1",
                }),
                call_id: "call-pr1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        {
            let mut bb = controller.blackboard().write();
            bb.actions.push_result(ActionResult {
                order_id: "ORD-PR-1".into(),
                action: Some("workspace.read_file".into()),
                ok: true,
                response: Some(serde_json::json!({
                    "output": "完整成功输出",
                    "nested": {"key": "value"},
                })),
                error: None,
                trace_id: "t-pr-1".into(),
                timestamp: "2026-08-19T04:00:00Z".into(),
            });
        }
        controller
            .run_turn(
                &host,
                "点读 receipt",
                "RUN-PR1",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-pr1"))
            })
            .expect("round carrying blackboard_read point-read reply");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-pr1"))
            .expect("point-read tool result message");
        // PULL 自描述（2026-08-31）：响应首行为增量头，receipt 正文紧随其后。
        assert!(
            reply
                .content
                .lines()
                .next()
                .unwrap()
                .starts_with("[黑板增量]"),
            "{:?}",
            round.messages
        );
        assert!(
            reply
                .content
                .contains("ORD-PR-1 ok=true step=? code=? trace_id=t-pr-1\nresponse={"),
            "{:?}",
            round.messages
        );
        assert!(
            reply.content.contains("\"output\":\"完整成功输出\""),
            "{:?}",
            round.messages
        );
        assert!(
            reply.content.contains("\"nested\":{\"key\":\"value\"}"),
            "{:?}",
            round.messages
        );
        // 点读回达不携带整段注册/订单/结果板。
        assert!(
            !reply.content.contains("== registration =="),
            "{:?}",
            round.messages
        );
        // 工具定义增量扩展：blackboard_read 声明了可选 receipt_id 参数。
        let bb_def = round
            .tools
            .iter()
            .find(|t| t.name == "blackboard_read")
            .expect("blackboard_read declared in request tools");
        assert!(
            bb_def
                .parameters
                .get("properties")
                .and_then(|p| p.get("receipt_id"))
                .is_some(),
            "receipt_id must be declared: {bb_def:?}"
        );
        assert_eq!(
            bb_def.parameters["properties"]["receipt_id"]["minLength"],
            1
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B，S2
    /// 工具级断言）：非法 `receipt_id`（非字符串/空串）= 显式报错
    /// （exit_code 1），绝不静默回退整段读取。
    #[tokio::test]
    async fn blackboard_read_invalid_receipt_id_is_explicit_error() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({
                    "section": "actions",
                    "receipt_id": 123,
                }),
                call_id: "call-inv".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "非法点读",
                "RUN-PRINV",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-inv"))
            })
            .expect("round carrying invalid receipt_id error");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-inv"))
            .expect("invalid receipt_id tool result message");
        assert!(
            reply.content.contains("invalid blackboard_read receipt_id"),
            "{:?}",
            round.messages
        );
        // 事件面不变：ToolCompleted 仍只带 section（receipt_id 不进事件面）。
        let completed: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        let inv_payload = completed
            .iter()
            .find(|p| p["tool"] == "blackboard_read" && p["exit_code"] == 1)
            .expect("invalid read completed");
        assert_eq!(inv_payload["section"], "actions", "{inv_payload:?}");
        assert!(inv_payload.get("receipt_id").is_none(), "{inv_payload:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-19 方案B 全面审查处理（N3）：`section` 非字符串 = 显式报错
    /// （绝不静默回退到 "plan" 默认值）——单独传非法 section 与 receipt_id
    /// 组合两条路径均回达显式错误，且不落到误导性的「section=plan 不支持
    /// receipt_id」守卫消息。
    #[tokio::test]
    async fn blackboard_read_non_string_section_is_explicit_error() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({
                    "section": 123,
                    "receipt_id": "ORD-1",
                }),
                call_id: "call-secnum".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "非法分区类型",
                "RUN-PRSECNUM",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-secnum"))
            })
            .expect("round carrying invalid section error");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-secnum"))
            .expect("invalid section tool result message");
        assert!(
            reply.content.contains("invalid blackboard_read section"),
            "{:?}",
            round.messages
        );
        assert!(reply.content.contains("123"), "{:?}", round.messages);
        assert!(
            !reply.content.contains("receipt_id 仅与 section=actions"),
            "{:?}",
            round.messages
        );
        // 事件面：section 以哨兵字符串呈现（保持事件字段类型稳定）。
        let completed: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        let sec_payload = completed
            .iter()
            .find(|p| p["tool"] == "blackboard_read" && p["exit_code"] == 1)
            .expect("invalid section completed");
        assert_eq!(sec_payload["section"], "<invalid>", "{sec_payload:?}");
        assert!(sec_payload.get("receipt_id").is_none(), "{sec_payload:?}");

        // 单独传非字符串 section（无 receipt_id）同样显式报错，不回退 plan。
        let dir2 = test_dir();
        let journal2 = JournalRecorder::new(dir2.clone());
        let host2 = TestHost {
            journal: journal2,
            tool_result: None,
        };
        let fake2 = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": 456}),
                call_id: "call-secnum2".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway2: Arc<dyn ModelGateway> = fake2.clone();
        let controller2 = AgentLoopController::with_gateway(gateway2);
        controller2
            .run_turn(
                &host2,
                "非法分区类型（无点读）",
                "RUN-PRSECNUM2",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let received2 = fake2.received_requests();
        let round2 = received2
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-secnum2"))
            })
            .expect("round carrying second invalid section error");
        let reply2 = round2
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-secnum2"))
            .expect("second invalid section tool result message");
        assert!(
            reply2.content.contains("invalid blackboard_read section"),
            "{:?}",
            round2.messages
        );
        assert!(reply2.content.contains("456"), "{:?}", round2.messages);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B，S2
    /// 工具级断言）：非 actions 分区携带 `receipt_id` = 显式报错（fail
    /// loud，同未知分区风格），工具结果回达模型。
    #[tokio::test]
    async fn blackboard_read_receipt_id_with_non_actions_section_errors() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({
                    "section": "plan",
                    "receipt_id": "ORD-1",
                }),
                call_id: "call-sec".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "错误组合",
                "RUN-PRSEC",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-sec"))
            })
            .expect("round carrying receipt_id section error");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-sec"))
            .expect("section error tool result message");
        assert!(
            reply
                .content
                .contains("receipt_id 仅与 section=actions 组合有效"),
            "{:?}",
            round.messages
        );
        assert!(
            reply.content.contains("section=plan"),
            "{:?}",
            round.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B，S2
    /// 跨 epoch 断言）：`receipt_id` 与 `epoch` 组合 = 归档快照点读——轮转
    /// 后 live 板点读旧 receipt = 显式 not found（提示旧 epoch 归档），
    /// epoch-1 快照点读 = 完整 response 回达；当前 epoch 点读正常。
    #[test]
    fn blackboard_read_receipt_point_read_live_and_archived() {
        let dir = test_dir().join("gsa").join("blackboard");
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_blackboard_archive_dir(Some(dir.clone()))
            .with_plan(
                "PLAN-PR-A".to_string(),
                1,
                "旧任务".to_string(),
                vec!["旧步骤".to_string()],
            );
        {
            let mut bb = controller.blackboard().write();
            bb.actions.push_result(ActionResult {
                order_id: "ORD-OLD-1".into(),
                action: Some("workspace.read_file".into()),
                ok: true,
                response: Some(serde_json::json!({"output": "archived payload"})),
                error: None,
                trace_id: "t-old-1".into(),
                timestamp: "2026-08-19T03:00:00Z".into(),
            });
        }
        // Rotate: epoch-1 archived WITH the receipt; the live board clears.
        let controller = controller.with_plan(
            "PLAN-PR-B".to_string(),
            2,
            "新任务".to_string(),
            vec!["新步骤".to_string()],
        );
        // Live point-read of the rotated-away receipt = explicit not-found.
        let live_missing =
            controller.render_blackboard_section("actions", None, None, Some("ORD-OLD-1"));
        assert!(
            live_missing.contains("ORD-OLD-1 not found"),
            "{live_missing}"
        );
        assert!(live_missing.contains("旧 epoch 归档"), "{live_missing}");
        // Archived point-read = full response.
        let archived =
            controller.render_blackboard_section("actions", None, Some(1), Some("ORD-OLD-1"));
        assert!(
            archived.starts_with("ORD-OLD-1 ok=true step=? code=? trace_id=t-old-1\nresponse="),
            "{archived}"
        );
        assert!(archived.contains("archived payload"), "{archived}");
        // Current-epoch point-read works on the live board.
        {
            let mut bb = controller.blackboard().write();
            bb.actions.push_result(ActionResult {
                order_id: "ORD-NEW-1".into(),
                action: Some("workspace.run_tests".into()),
                ok: false,
                response: None,
                error: Some(serde_json::json!({
                    "step": "execute",
                    "code": "boom",
                    "message": "m",
                    "upstream": {"x": 1},
                })),
                trace_id: "t-new-1".into(),
                timestamp: "2026-08-19T03:00:01Z".into(),
            });
        }
        let live = controller.render_blackboard_section("actions", None, None, Some("ORD-NEW-1"));
        assert!(
            live.starts_with("ORD-NEW-1 ok=false step=execute code=boom trace_id=t-new-1\nerror="),
            "{live}"
        );
        assert!(live.contains("\"upstream\":{\"x\":1}"), "{live}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-08 blackboard partition (A3): `blackboard_read` serves the
    /// requested partition from the controller's blackboard — an edits
    /// section after a real edit round returns the record the model needs
    /// for look-back.
    #[tokio::test]
    async fn blackboard_read_serves_edits_partition() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "edited ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "search_replace".to_string(),
                arguments: serde_json::json!({
                    "file_path": "1.py",
                    "old_string": "old\nold2",
                    "new_string": "new\nnew2\nnew3",
                }),
                call_id: "call-e3".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "edits"}),
                call_id: "call-b1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "改文件再看黑板",
                "RUN-BB",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // ToolCompleted for blackboard_read carries the section it served.
        let payloads: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        assert_eq!(payloads.len(), 2, "{payloads:?}");
        let read_payload = payloads
            .iter()
            .find(|p| p["tool"] == "blackboard_read")
            .expect("blackboard_read completed");
        assert_eq!(read_payload["section"], "edits", "{read_payload:?}");
        assert_eq!(read_payload["exit_code"], 0);

        // The served content — the edit record line (timestamp + "1.py
        // 2→3行变动") — reaches the model as the tool's reply.
        let received = fake.received_requests();
        let round3 = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-b1"))
            })
            .expect("round carrying blackboard_read reply");
        assert!(
            round3
                .messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-b1")
                    && m.content.contains("1.py 2→3行变动")),
            "{:?}",
            round3.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-08 review closure (P2-2): `since_timestamp` filtering is
    /// format-robust — a since in 'Z' or truncated precision must not
    /// silently drop records (bare string compare would: 'Z' > '+' means
    /// every "+00:00" record sorts BELOW a "…Z" since). Parsed RFC 3339
    /// comparison: a since in the far future filters everything, a since in
    /// the far past keeps everything.
    #[tokio::test]
    async fn blackboard_read_since_filters_with_any_rfc3339_form() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "edited ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "search_replace".to_string(),
                arguments: serde_json::json!({
                    "file_path": "1.py",
                    "old_string": "old",
                    "new_string": "new\nnew2",
                }),
                call_id: "call-e4".to_string(),
            }]),
            // since in Z form, far future — must filter everything out.
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({
                    "section": "edits",
                    "since_timestamp": "2999-01-01T00:00:00Z",
                }),
                call_id: "call-b2".to_string(),
            }]),
            // since in Z form, far past — must keep everything.
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({
                    "section": "edits",
                    "since_timestamp": "2000-01-01T00:00:00Z",
                }),
                call_id: "call-b3".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "改文件并回看",
                "RUN-BBS",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let far_future = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-b2"))
            })
            .expect("call-b2 round");
        assert!(
            far_future
                .messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-b2")
                    && m.content.contains("(no edit records)")),
            "Z-form future since must filter everything: {:?}",
            far_future.messages
        );

        let far_past = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-b3"))
            })
            .expect("call-b3 round");
        assert!(
            far_past
                .messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-b3")
                    && m.content.contains("1.py 1→2行变动")),
            "Z-form past since must keep the record: {:?}",
            far_past.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-08: unknown sections are surfaced to the model — fail
    /// loud, not silent (an invalid section must not read like an empty
    /// partition).
    #[tokio::test]
    async fn blackboard_read_unknown_section_surfaces_error() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "bogus"}),
                call_id: "call-b4".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "读错误分区",
                "RUN-BBE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-b4"))
            })
            .expect("call-b4 round");
        assert!(
            round
                .messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some("call-b4")
                    && m.content.contains("unknown blackboard section")),
            "{:?}",
            round.messages
        );
        // PULL 自描述（2026-08-31 审查处理 M2）：失败形状不挂增量头——
        // 模型拿到的是错误文本，不算读过该分区（设计 §3）。
        assert!(
            round
                .messages
                .iter()
                .filter(|m| m.tool_call_id.as_deref() == Some("call-b4"))
                .all(|m| !m.content.starts_with("[黑板增量]")),
            "error shape must not carry the delta header: {:?}",
            round.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R2a (§4.4): `blackboard_read` 服务 live
    /// 检索分区——internal_ret / external_ret 各自渲染 response / entries /
    /// ledger，工具结果回达模型，事件面记录对应 section。
    #[tokio::test]
    async fn blackboard_read_serves_retrieval_partitions() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "internal_ret"}),
                call_id: "call-ir".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        // 预写内部检索分区（模拟子代理落盘）。
        {
            let mut w = controller.blackboard().write();
            w.internal_ret.response = Some("检索完成\n[DOC] design.md".to_string());
            w.internal_ret.project_docs = vec!["design.md".to_string()];
            w.internal_ret.source_ledger = vec!["docs/index".to_string()];
        }
        controller
            .run_turn(
                &host,
                "读检索分区",
                "RUN-RIR",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-ir"))
            })
            .expect("call-ir round");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-ir"))
            .expect("internal_ret tool result message");
        assert!(
            reply.content.contains("== internal_ret ==")
                && reply.content.contains("检索完成")
                && reply.content.contains("- design.md")
                && reply.content.contains("- docs/index"),
            "internal_ret reply: {:?}",
            round.messages
        );
        // 事件面记录 section。
        let completed = events(&dir)
            .into_iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("blackboard_read")
                    && e.payload.get("section").and_then(|v| v.as_str()) == Some("internal_ret")
            })
            .expect("blackboard_read internal_ret completed");
        assert_eq!(completed.payload["exit_code"], serde_json::json!(0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R2a (§4.4): 检索分区带 `epoch` = 显式报错
    /// （live-only，结果不进 epoch 快照；fail loud 同 session 面纪律），
    /// 绝不静默回退 live 视图或空分区。
    #[tokio::test]
    async fn blackboard_read_retrieval_section_with_epoch_errors_explicitly() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "external_ret", "epoch": 1}),
                call_id: "call-re".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "读归档检索分区",
                "RUN-RRE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-re"))
            })
            .expect("call-re round");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-re"))
            .expect("epoch-error tool result message");
        assert!(
            reply.content.contains("with epoch is not supported")
                && reply.content.contains("live-only"),
            "epoch error reply: {:?}",
            round.messages
        );
        // 事件面 exit_code 0（渲染层文本错误，与 receipt_id+非 actions 先例
        // 同纪律——正文显式说明不支持，事件带 section 标注）。
        let completed = events(&dir)
            .into_iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("blackboard_read")
                    && e.payload.get("section").and_then(|v| v.as_str()) == Some("external_ret")
            })
            .expect("blackboard_read external_ret completed");
        assert_eq!(completed.payload["exit_code"], serde_json::json!(0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F6 (2026-08-15, BACKLOG 6e 复查遗留): an epoch parameter that is
    /// present but invalid (0 / negative / float) is an EXPLICIT error —
    /// never a silent fallback to the live view.
    #[tokio::test]
    async fn blackboard_read_invalid_epoch_is_explicit_error() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "irrelevant".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![
                ToolCall {
                    name: "blackboard_read".to_string(),
                    arguments: serde_json::json!({"section": "plan", "epoch": 0}),
                    call_id: "call-b5".to_string(),
                },
                ToolCall {
                    name: "blackboard_read".to_string(),
                    arguments: serde_json::json!({"section": "plan", "epoch": -1}),
                    call_id: "call-b6".to_string(),
                },
                ToolCall {
                    name: "blackboard_read".to_string(),
                    arguments: serde_json::json!({"section": "plan", "epoch": 1.5}),
                    call_id: "call-b7".to_string(),
                },
            ]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "非法 epoch",
                "RUN-BEI",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let payloads: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .filter(|e| e.payload.get("tool").and_then(|t| t.as_str()) == Some("blackboard_read"))
            .map(|e| e.payload)
            .collect();
        let invalid: Vec<_> = payloads
            .iter()
            .filter(|p| {
                p["call_id"] == "call-b5" || p["call_id"] == "call-b6" || p["call_id"] == "call-b7"
            })
            .collect();
        assert_eq!(invalid.len(), 3, "{invalid:?}");
        for payload in invalid {
            assert_eq!(payload["exit_code"], 1, "{payload:?}");
            assert!(
                payload["error"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("invalid blackboard_read epoch"),
                "{payload:?}"
            );
        }

        // The explicit error must not leak the live plan (no silent
        // fallback): every reply line mentions the invalid epoch value.
        let received = fake.received_requests();
        for call_id in ["call-b5", "call-b6", "call-b7"] {
            let round = received
                .iter()
                .find(|r| {
                    r.messages
                        .iter()
                        .any(|m| m.tool_call_id.as_deref() == Some(call_id))
                })
                .unwrap_or_else(|| panic!("{call_id} round"));
            let reply = round
                .messages
                .iter()
                .find(|m| m.tool_call_id.as_deref() == Some(call_id))
                .expect("reply message");
            assert!(
                reply.content.contains("invalid blackboard_read epoch"),
                "{call_id}: {}",
                reply.content
            );
            assert!(
                !reply.content.contains("goal:"),
                "{call_id} silently fell back to the live view: {}",
                reply.content
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PULL 自描述（2026-08-31，P2-11 第 1 项 / 设计 §2）：追加类分区计数
    /// 在 push_* 方法内自增；计划轮换/同 epoch 修订/快照恢复计 1 次变化。
    #[test]
    fn partition_revisions_bump_on_push_rotate_and_restore() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.push_edit(EditRecord {
                file: "a.txt".into(),
                old_lines: 1,
                new_lines: 2,
                timestamp: "t".into(),
            });
            w.push_tool_action(ToolActionRecord {
                category: "read".into(),
                tool: "read_file".into(),
                timestamp: "t".into(),
            });
            w.push_exec_result("r".into());
            w.push_exec_error("e".into());
        }
        let rev = bb.read().partition_revisions();
        let get = |name: &str| rev.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(get("plan"), 0);
        assert_eq!(get("exec"), 2);
        assert_eq!(get("edits"), 1);
        assert_eq!(get("tool_actions"), 1);
        assert_eq!(get("actions"), 0);

        // 新 epoch 轮换：plan 替换 + exec/edits/tool_actions/actions 清空。
        {
            let mut w = bb.write();
            w.rotate_to_structured_plan(
                "PLAN-A".into(),
                1,
                "goal".into(),
                Vec::new(),
                "2026-08-31T00:00:00Z",
            )
            .unwrap();
        }
        let rev = bb.read().partition_revisions();
        let get = |name: &str| rev.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(get("plan"), 1);
        assert_eq!(get("exec"), 3);
        assert_eq!(get("edits"), 2);
        assert_eq!(get("tool_actions"), 2);
        assert_eq!(get("actions"), 1);

        // 同 epoch 修订：只计 plan。
        {
            let mut w = bb.write();
            w.rotate_to_structured_plan(
                "PLAN-A".into(),
                1,
                "goal2".into(),
                Vec::new(),
                "2026-08-31T00:00:01Z",
            )
            .unwrap();
        }
        let rev = bb.read().partition_revisions();
        let get = |name: &str| rev.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(get("plan"), 2);
        assert_eq!(get("exec"), 3);
        assert_eq!(get("tool_actions"), 2);

        // 快照恢复：恢复的分区各计 1。
        let snapshot = bb.read().epoch_snapshot("2026-08-31T00:00:02Z");
        {
            let mut w = bb.write();
            w.restore_epoch_snapshot(&snapshot);
        }
        let rev = bb.read().partition_revisions();
        let get = |name: &str| rev.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(get("plan"), 3);
        assert_eq!(get("exec"), 4);
        assert_eq!(get("edits"), 3);
        assert_eq!(get("tool_actions"), 3);
        assert_eq!(get("actions"), 2);
    }

    /// PULL 自描述 §2：ActionBoard 四类变化（注册替换 / 订单写入 / 消费 /
    /// receipt 追加）各计 1 次。
    #[test]
    fn action_board_revision_bumps_on_mutations() {
        let mut board = ActionBoard::default();
        assert_eq!(board.revision, 0);
        let reg = vec![ActionRegistration {
            name: "workspace.read_file".into(),
            description: "读文件".into(),
            parameters: serde_json::json!({}),
            target_policy: crate::entities::TargetPolicy::File,
        }];
        board.set_registration(reg.clone());
        assert_eq!(board.revision, 1);
        // 内容相同不计数（审查处理 M3：读 actions 的机械刷新不再自触发徽章）。
        board.set_registration(reg.clone());
        assert_eq!(board.revision, 1);
        // 内容变化才计数。
        board.set_registration(Vec::new());
        assert_eq!(board.revision, 2);
        board
            .write_order(ActionOrder {
                order_id: "ORD-1".into(),
                action: "workspace.read_file".into(),
                arguments: serde_json::json!({}),
                target: None,
                step_id: None,
                round: 1,
                plan_epoch: 1,
                run_id: "RUN-1".into(),
            })
            .unwrap();
        assert_eq!(board.revision, 3);
        assert!(board.take_order().is_some());
        assert_eq!(board.revision, 4);
        board.push_result(ActionResult {
            order_id: "ORD-1".into(),
            action: Some("workspace.read_file".into()),
            ok: true,
            response: None,
            error: None,
            trace_id: "t".into(),
            timestamp: "2026-08-31T00:00:00Z".into(),
        });
        assert_eq!(board.revision, 5);
        // 空槽消费不计数。
        assert!(board.take_order().is_none());
        assert_eq!(board.revision, 5);
    }
}
