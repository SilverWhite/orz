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

use orz_assurance::lif::Domain;

/// `blackboard_write` 工具名的唯一字面（0AE 收口去重）：controller
/// `run_turn_inner` 注册处与 agent_loop 压缩窗口过滤面共享，避免双副本漂移。
pub(crate) const BLACKBOARD_WRITE_TOOL_NAME: &str = "blackboard_write";

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
    /// 0ae D0（2026-09-15，用户裁决 DP-6）：模型写入面——`blackboard_write
    /// section=plan` 的落点（模型自有工作计划笔记，与机械单写者结构化
    /// plan 字段分立；折叠不灭、随 plan epoch 快照归档）。单次写入 ≤8K。
    #[serde(default)]
    pub model_notes: Vec<NoteEntry>,
}

/// 0ae D0：`blackboard_write` 的目标分区（DP-6：限 plan 与 notes 两域；
/// 机械单写者分区不开放写入，所有权不变）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelNoteSection {
    Plan,
    Notes,
}

impl ModelNoteSection {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelNoteSection::Plan => "plan",
            ModelNoteSection::Notes => "notes",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "plan" => Some(ModelNoteSection::Plan),
            "notes" => Some(ModelNoteSection::Notes),
            _ => None,
        }
    }
}

/// 0ae D0（2026-09-15，设计 §3）：模型写黑板的一条盖章记录——(round,
/// domain) 写时盖章 + ts 墙钟，同 EditRecord/ExecEntry 纪律；`content`
/// 单条 ≤8K（工具面 maxLength 机械钳制）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteEntry {
    pub round: u64,
    #[serde(default)]
    pub domain: Option<Domain>,
    pub timestamp: String,
    pub content: String,
}

/// 一条 exec 结果/错误行（B1 会话化基础，2026-09-03，P2-13 / 设计 §9.4
/// E8 结构化盖章）：写时盖 (round, domain) 章 + `ts` 墙钟——round 为
/// 会话相对决策轮（与 temporal 行/failure_agg 段同刻度）、domain 为写时
/// LIF 域机器当前域；旧无章行（round=0/domain=None，ts 空串）在 B2 渲染
/// 折叠中归 `pre-stamp` 段。
///
/// `exit_code`（0p S1 复审 F-A，2026-09-07）：命令真实退出码——仅
/// run_terminal_cmd/run_tests 等命令族工具在 Ok 臂写入（工具执行成功但
/// 命令退出码≠0 是命令级失败，与 host 级 ToolError 分开记账）；None =
/// 工具级成功且无命令退出语义（read_file 等）或 host 级错误行（errors
/// 臂）。`selfhistory::render_exec_search` 据此渲染 `exit=N`。
///
/// 兼容：旧 epoch 归档/侧车里的 exec 行是纯字符串（无字段对象），经
/// untagged 反序列化读回为无章行，不静默丢弃；缺字段对象按 serde(default)
/// 补 None，既有板零迁移。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecEntry {
    pub text: String,
    #[serde(default)]
    pub round: u64,
    #[serde(default)]
    pub domain: Option<Domain>,
    #[serde(default, alias = "timestamp")]
    pub ts: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
enum ExecEntryRepr {
    /// 旧归档字符串行（无章）。
    Legacy(String),
    Structured(StructuredExecEntry),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct StructuredExecEntry {
    pub text: String,
    #[serde(default)]
    pub round: u64,
    #[serde(default)]
    pub domain: Option<Domain>,
    #[serde(default, alias = "timestamp")]
    pub ts: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

impl<'de> Deserialize<'de> for ExecEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ExecEntryRepr::deserialize(deserializer)? {
            ExecEntryRepr::Legacy(text) => Ok(ExecEntry {
                text,
                round: 0,
                domain: None,
                ts: String::new(),
                exit_code: None,
            }),
            ExecEntryRepr::Structured(s) => Ok(ExecEntry {
                text: s.text,
                round: s.round,
                domain: s.domain,
                ts: s.ts,
                exit_code: s.exit_code,
            }),
        }
    }
}

impl ExecEntry {
    /// 写时盖章构造（round/domain 必填——生产写入点只走本入口；直接
    /// 字面量仅测试/legacy 使用）。`exit_code` 缺省 None，命令族 Ok 臂
    /// 写入点在构造后回填（`entry.exit_code = res.exit_code`）。
    pub fn stamped(text: String, round: u64, domain: Domain, ts: String) -> Self {
        ExecEntry {
            text,
            round,
            domain: Some(domain),
            ts,
            exit_code: None,
        }
    }

    /// 是否无章旧行（round=0/domain=None/ts 空 → B2 `pre-stamp` 段）。
    pub fn is_pre_stamp(&self) -> bool {
        self.round == 0 && self.domain.is_none()
    }
}

/// Legacy 便捷构造：`From<&str>/From<String>` 生成无章旧行（round=0 /
/// domain=None / ts 空）——测试与旧形状 fixture 的入口；生产写入点一律
/// 使用 [`ExecEntry::stamped`]（B1 纪律：不留无章行）。
impl From<&str> for ExecEntry {
    fn from(text: &str) -> Self {
        ExecEntry {
            text: text.to_string(),
            round: 0,
            domain: None,
            ts: String::new(),
            exit_code: None,
        }
    }
}

impl From<String> for ExecEntry {
    fn from(text: String) -> Self {
        ExecEntry {
            text,
            round: 0,
            domain: None,
            ts: String::new(),
            exit_code: None,
        }
    }
}

/// 文本比较便捷（`assert_eq!(results, vec!["ok"])` 等既有断言形态）。
impl PartialEq<&str> for ExecEntry {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl PartialEq<String> for ExecEntry {
    fn eq(&self, other: &String) -> bool {
        self.text == *other
    }
}

/// Main agent writes: results, observations, errors, auth_requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecSection {
    pub results: Vec<ExecEntry>,
    pub observations: Vec<String>,
    pub errors: Vec<ExecEntry>,
    pub auth_requests: Vec<String>,
}

/// 检索派发章（B1 会话化基础，2026-09-03，设计 §9.4/R1）：检索分区为
/// **派发级全量覆盖写**（每次派发替换整区），单次派发的所有行共享同一
/// (round, domain, ts)——round = 派发所属主决策轮（会话相对）、domain =
/// 写时 LIF 当前域。`None` = 旧数据/恢复路径尚无章（B2 pre-stamp）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchStamp {
    pub round: u64,
    pub domain: Domain,
    pub timestamp: String,
}

/// Internal retrieval subagent writes: project docs, source ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalRetSection {
    pub project_docs: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
    /// B1：本分区最后一次派发的 (round, domain) 章。
    #[serde(default)]
    pub stamp: Option<DispatchStamp>,
}

/// External retrieval subagent writes: web sources, source ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRetSection {
    pub web_sources: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
    /// B1：本分区最后一次派发的 (round, domain) 章。
    #[serde(default)]
    pub stamp: Option<DispatchStamp>,
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
    /// B1 会话化基础（2026-09-03）：写时盖会话相对决策轮章（0 = 旧无章
    /// 行，pre-stamp）。
    #[serde(default)]
    pub round: u64,
    /// B1：写时 LIF 域章（None = 旧无章行，pre-stamp）。
    #[serde(default)]
    pub domain: Option<Domain>,
    /// 0AE-C11 处置（2026-09-17，盘点 FR-C04 采②分 run 标注）：写时
    /// run 章（journal run id）。空 = 旧无章行（pre-stamp）——D4 渲染面
    /// 按此把编辑分「本 run / 会话历史」两组，多 run 会话不再被「本 run」
    /// 标签夸大归属。
    #[serde(default)]
    pub run: String,
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
    /// B1 会话化基础（2026-09-03）：写时盖会话相对决策轮章（0 = 旧无章
    /// 行，pre-stamp）。
    #[serde(default)]
    pub round: u64,
    /// B1：写时 LIF 域章（None = 旧无章行，pre-stamp）。
    #[serde(default)]
    pub domain: Option<Domain>,
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
    /// B1 会话化基础（2026-09-03）：写时盖会话相对决策轮章（0 = 旧无章
    /// 行，pre-stamp）。
    #[serde(default)]
    pub round: u64,
    /// B1：写时 LIF 域章（None = 旧无章行，pre-stamp）。
    #[serde(default)]
    pub domain: Option<Domain>,
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

    /// B1：恢复会话快照时归零版本计数（run 级徽章不跨 prompt 延续）。
    pub(crate) fn reset_revision(&mut self) {
        self.revision = 0;
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

/// 0ae D0：单次写入字符上限（设计 §3「单次写入 ≤8K」）。
pub const MODEL_NOTE_MAX_CHARS: usize = 8192;

/// 0ae D0：NoteEntry 列表渲染（逐条 (r轮/域) 盖章头 + 内容；时间正序）。
pub fn render_note_entries(notes: &[NoteEntry]) -> String {
    if notes.is_empty() {
        return "（无）".to_string();
    }
    notes
        .iter()
        .enumerate()
        .map(|(index, note)| {
            let domain = note
                .domain
                .map(|d| d.as_str().to_string())
                .unwrap_or_else(|| "-".to_string());
            format!(
                "[笔记 {}] r{}@{} {}\n{}",
                index + 1,
                note.round,
                domain,
                note.timestamp,
                note.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The full blackboard with 5 sections + 2 controller-written partitions +
/// the console action board (assistant writes registration/results, the
/// model writes the single order slot).
///
/// Read rule: all sections are readable by all agents.
/// Write rule: each section has a single writer (enforced by convention).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// B1（2026-09-03）：会话级 live 侧车也不携带——恢复后为全新空图
    /// （run 内派生态，跨 prompt 不延续）。
    #[serde(default, skip_serializing)]
    pub dep_graph: crate::dep_graph::DepGraph,
    /// F4 失败目标聚合分区（P2-12 COMPRESSION-LINGUISTIC-FORMAL-LAYER，
    /// 2026-09-02 方案 A）：epoch 作用域、随黑板轮换重置、随 EpochSnapshot
    /// 归档/恢复（同 exec/actions 纪律）；只被压缩「注意事项」槽渲染消费，
    /// 不是 `blackboard_read` 的查询分区（PULL 面不变）。
    #[serde(default)]
    pub failure_agg: crate::failure_agg::FailureAgg,
    /// 0ae D0（2026-09-15，设计 §3）：模型自有工作笔记分区——
    /// `blackboard_write section=notes` 的落点（折叠不灭、随会话延续；
    /// 机械单写者分区所有权不变，本分区唯一写者是模型写入面）。
    #[serde(default)]
    pub notes: Vec<NoteEntry>,
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
    pub notes: u64,
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
    /// snapshot — plan + notes + edits + tool_actions + exec. Gate log, whitelist
    /// and the retrieval partitions are NOT part of an epoch snapshot
    /// (they survive rotation by design).
    pub fn epoch_snapshot(&self, persisted_at: &str) -> EpochSnapshot {
        EpochSnapshot {
            plan_id: self.plan.plan_id.clone(),
            plan_epoch: self.plan.plan_epoch,
            plan: self.plan.clone(),
            notes: self.notes.clone(),
            edits: self.edits.clone(),
            tool_actions: self.tool_actions.clone(),
            exec: self.exec.clone(),
            actions: self.actions.clone(),
            failure_agg: self.failure_agg.clone(),
            persisted_at: persisted_at.to_string(),
        }
    }

    /// Restore a persisted epoch snapshot into the live partitions. The
    /// retrieval partitions / gate log / whitelist are left untouched.
    pub fn restore_epoch_snapshot(&mut self, snapshot: &EpochSnapshot) {
        self.plan = snapshot.plan.clone();
        self.notes = snapshot.notes.clone();
        self.revisions.notes = self.revisions.notes.saturating_add(1);
        self.edits = snapshot.edits.clone();
        self.tool_actions = snapshot.tool_actions.clone();
        self.exec = snapshot.exec.clone();
        self.actions = snapshot.actions.clone();
        self.failure_agg = snapshot.failure_agg.clone();
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

    /// 0ae D0：模型写入面追加（section=notes → `notes` 分区；
    /// section=plan → `plan.model_notes`）。分区版本计数同一落点。
    pub fn push_model_note(&mut self, section: ModelNoteSection, entry: NoteEntry) {
        match section {
            ModelNoteSection::Plan => {
                self.plan.model_notes.push(entry);
                self.revisions.plan = self.revisions.plan.saturating_add(1);
            }
            ModelNoteSection::Notes => {
                self.notes.push(entry);
                self.revisions.notes = self.revisions.notes.saturating_add(1);
            }
        }
    }

    /// 0ae D1 补救规则读数：模型写入面累计条数（notes + plan.model_notes）。
    pub fn model_note_count(&self) -> usize {
        self.notes.len() + self.plan.model_notes.len()
    }

    /// 0ae D0：`blackboard_read section=notes` 渲染（时间正序、逐条盖章头；
    /// 空分区 = 「（无）」同空槽纪律）。
    pub fn render_notes_section(&self) -> String {
        render_note_entries(&self.notes)
    }

    /// 0ae D0：`blackboard_read section=plan` 的模型笔记尾段（plan 视图
    /// 主体由既有 render_section 渲染，本段追加其后）。
    pub fn render_plan_model_notes_tail(&self) -> Option<String> {
        if self.plan.model_notes.is_empty() {
            return None;
        }
        Some(render_note_entries(&self.plan.model_notes))
    }

    /// 工具动作记录追加（单写者纪律 + 分区版本计数同一落点）。
    pub fn push_tool_action(&mut self, record: ToolActionRecord) {
        self.tool_actions.push(record);
        self.revisions.tool_actions = self.revisions.tool_actions.saturating_add(1);
    }

    /// exec 结果追加（单写者纪律 + 分区版本计数同一落点）。B1：调用方
    /// 构造带 (round, domain, ts) 章的 [`ExecEntry`]——生产写入点不落
    /// 无章行。
    pub fn push_exec_result(&mut self, entry: ExecEntry) {
        self.exec.results.push(entry);
        self.revisions.exec = self.revisions.exec.saturating_add(1);
    }

    /// exec 错误追加（单写者纪律 + 分区版本计数同一落点）。B1：同
    /// [`Self::push_exec_result`] 的盖章纪律。
    pub fn push_exec_error(&mut self, entry: ExecEntry) {
        self.exec.errors.push(entry);
        self.revisions.exec = self.revisions.exec.saturating_add(1);
    }

    /// B1 会话化基础（2026-09-03）：跨 prompt 持久化/恢复用的黑板 live
    /// 快照——整板克隆后做会话边界清理：动作栏注册板块与 pending 订单
    /// 槽是 run 级机械面（恢复后由下一 run 的探针/写单重建，跨 prompt
    /// 陈旧内容不延续）；依赖图 live-only 不随会话延续（置空，恢复后
    /// 由下一 run 从零建图）。entities / 检索分区随会话延续（设计
    /// §11.1 W 计量含 entities 当前内容；检索分区另有 activation 侧车
    /// 同源副本）。
    pub fn conversation_snapshot(&self) -> Blackboard {
        let mut snap = self.clone();
        snap.actions.registration.clear();
        snap.actions.order = None;
        snap.dep_graph = crate::dep_graph::DepGraph::default();
        snap
    }

    /// B1：用会话快照整体替换 live 黑板（ACP 每 prompt 续载入口）。
    /// 快照已含会话边界清理（None = 全新会话）。
    pub fn restore_conversation_snapshot(&mut self, snapshot: Blackboard) {
        *self = snapshot;
        // PULL 自描述：分区版本计数只随 run 存活（跨 prompt 侧车不携带）。
        // 恢复 = 可见内容整体替换——各分区统一归位为「1 次变化」，首次
        // blackboard_read 恰好各挂 +1 徽章（与 restore_epoch_snapshot 的
        // bump 语义对齐；不延续上 run 的累积计数，防跨 prompt 徽章噪声）。
        // actions/entities 自含计数同样归位 1（reset 后 bump）；依赖图已随
        // 会话快照置空（空图保持版本 0，直到 run 内重新建图）。
        self.revisions = PartitionRevisions {
            plan: 1,
            notes: 1,
            exec: 1,
            edits: 1,
            tool_actions: 1,
            internal_ret: 1,
            external_ret: 1,
        };
        self.actions.reset_revision();
        self.actions.bump();
        self.entities.reset_revision();
        self.entities.bump_revision();
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

    /// B2 折叠触发 / B3 疲劳度共用（§11.1 v0.7 存储字节口径）：整份黑板
    /// live 内容的紧凑 JSON 字节——revision / dep_graph 等 `skip` 字段不
    /// 参与；actions 注册/订单槽按 live 面现状计入（与 run 末持久化的
    /// 会话快照口径差一个注册/订单槽清理，量级可忽略，登记于 B2 审计）。
    /// 序列化失败回退 0（渲染层安全方向：视为未达 W，不误折叠）。
    pub fn live_compact_bytes(&self) -> usize {
        serde_json::to_vec(self).map(|v| v.len()).unwrap_or(0)
    }

    /// PULL 自描述 §2：固定分区序的黑板直属版本快照（live-only）。
    /// session / temporal 由控制器按 tool_rounds / LIF round 派生。
    pub fn partition_revisions(&self) -> Vec<(&'static str, u64)> {
        vec![
            ("plan", self.revisions.plan),
            ("notes", self.revisions.notes),
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
            model_notes: Vec::new(),
        };
        self.plan.steps.extend(steps);
        self.edits.clear();
        self.tool_actions.clear();
        self.exec = ExecSection::default();
        self.actions = ActionBoard::default();
        self.failure_agg = crate::failure_agg::FailureAgg::default();
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
    /// 0ae D0（2026-09-15）：模型自有工作笔记随 epoch 归档/恢复；
    /// 旧归档（无该字段）经 serde default 兼容读取。
    #[serde(default)]
    pub notes: Vec<NoteEntry>,
    pub edits: Vec<EditRecord>,
    pub tool_actions: Vec<ToolActionRecord>,
    pub exec: ExecSection,
    /// 操作台动作栏（v0.5；P0-C S1）——随 epoch 归档/恢复。
    #[serde(default)]
    pub actions: ActionBoard,
    /// F4 失败目标聚合（P2-12，2026-09-02）——epoch 作用域、随归档/恢复；
    /// 旧归档（无该字段）经 serde default 兼容读取（与 actions 同纪律）。
    #[serde(default)]
    pub failure_agg: crate::failure_agg::FailureAgg,
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
                round: 0,
                domain: None,
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

    /// P2-12（2026-09-02 方案 A）：F4 失败目标聚合分区与 exec/actions 同
    /// 纪律——epoch 作用域：随 EpochSnapshot 归档/恢复、随黑板轮换清空；
    /// 旧归档（无该字段）经 serde default 兼容读取。
    #[test]
    fn failure_agg_rotates_and_snapshots_with_epoch() {
        let mut bb = Blackboard::new();
        bb.rotate_to_plan(
            "PLAN-1".into(),
            1,
            "first".into(),
            vec!["step".into()],
            "2026-08-15T00:00:00Z",
        )
        .unwrap();
        bb.failure_agg.record(
            "cmd_target",
            "id-1",
            "make -j8",
            "tool_timeout",
            5.0,
            1,
            orz_assurance::lif::Domain::Start,
        );
        bb.failure_agg.record(
            "cmd_target",
            "id-1",
            "make -j8",
            "execution_failed",
            20.0,
            3,
            orz_assurance::lif::Domain::Normal,
        );

        let snap = bb.epoch_snapshot("2026-08-15T00:00:00Z");
        assert_eq!(snap.failure_agg.rows.len(), 1);
        assert_eq!(snap.failure_agg.rows[0].count, 2);

        // New plan epoch rotates: old epoch archived (rows preserved),
        // live board cleared.
        let old = bb
            .rotate_to_plan(
                "PLAN-2".into(),
                2,
                "next".into(),
                vec!["step".into()],
                "2026-08-15T00:00:01Z",
            )
            .unwrap()
            .expect("epoch 1 archived");
        assert!(bb.failure_agg.is_empty());
        assert_eq!(old.failure_agg.rows.len(), 1);

        // Restore roundtrip brings the aggregate back.
        let mut restored = Blackboard::new();
        restored.restore_epoch_snapshot(&snap);
        assert_eq!(restored.failure_agg.rows.len(), 1);
        assert_eq!(restored.failure_agg.rows[0].count, 2);

        // Old archive JSON (no failure_agg key) still parses via serde
        // default — upgrades never poison cross-epoch restore.
        let mut json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&snap).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("failure_agg");
        let parsed: EpochSnapshot = serde_json::from_value(json).expect("old archive parses");
        assert!(parsed.failure_agg.is_empty());
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
                round: 0,
                domain: None,
                run: String::new(),
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
        // F-012（2026-09-14）：宿主环境可常驻 ORZ_MAX_WALLCLOCK（本机实测
        // 3600）——会话面的限额字段随环境取值；本测试钉「会话面渲染口径」，
        // 不承担「环境无预算」的断言义务（并行测试下改进程 env 不可靠）。
        // 期望值走与渲染同一解析入口，环境事实不再误报成产品回归。
        let expected_wallclock_limit = match crate::controller::main_wallclock_limit_secs_override()
        {
            Some(secs) => format!("WALLCLOCK_LIMIT: {secs}s"),
            None => "WALLCLOCK_LIMIT: none".to_string(),
        };
        assert!(
            reply.content.contains("TOOL_ROUNDS_USED: 0")
                && reply.content.contains("TOOL_ROUNDS_REMAINING: 120")
                && reply
                    .content
                    .contains("TOOL_ROUND_BUDGET: 120 tool rounds per turn")
                && reply.content.contains("WALLCLOCK_ELAPSED:")
                && reply.content.contains(&expected_wallclock_limit)
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
                round: 0,
                domain: None,
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
                round: 0,
                domain: None,
            });
            // 40 条超长 exec 行：逐行截断后段总长仍超 4K → 头行 + 计数行。
            for _ in 0..40 {
                bb.exec.results.push("z".repeat(500).into());
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
                .starts_with("[黑板 增量 水位"),
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
                round: 0,
                domain: None,
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
                .starts_with("[黑板 增量 水位"),
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

    /// P2-13 B2（2026-09-03，设计 §9.3/R2 + §12）：blackboard_read 工具声明
    /// 增量扩展——可选 `domain`/`round_from`/`round_to` 展开参数（含边界、
    /// 相等 = 单轮）随请求工具定义回达模型。
    #[tokio::test]
    async fn blackboard_read_declares_fold_expand_params() {
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
                arguments: serde_json::json!({"section": "exec"}),
                call_id: "call-fold-decl".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "看 exec 声明",
                "RUN-FOLD-DECL",
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
                    .any(|m| m.tool_call_id.as_deref() == Some("call-fold-decl"))
            })
            .expect("round carrying blackboard_read");
        let bb_def = round
            .tools
            .iter()
            .find(|t| t.name == "blackboard_read")
            .expect("blackboard_read declared in request tools");
        let props = bb_def
            .parameters
            .get("properties")
            .expect("tool parameters properties");
        for key in ["domain", "round_from", "round_to"] {
            assert!(
                props.get(key).is_some(),
                "{key} must be declared: {bb_def:?}"
            );
        }
        assert_eq!(
            props["domain"]["enum"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0),
            5,
            "{bb_def:?}"
        );
        assert_eq!(props["round_from"]["minimum"], 1);
        assert_eq!(props["round_to"]["minimum"], 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0p S1（2026-09-07，ADR-0010 §14.61 设计 A1/A2）：blackboard_read 工具
    /// 声明增量扩展——可选 `failures_only`（boolean）与 `search`（string，
    /// minLength 1）自历史查询参数随请求工具定义回达；描述含教学句
    /// （failures_only 失败总览 / search 字面检索）。
    #[tokio::test]
    async fn blackboard_read_declares_selfhistory_params() {
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
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "看声明",
                "RUN-SH-DECL",
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
            .first()
            .expect("first request declares the tool surface");
        let bb_def = round
            .tools
            .iter()
            .find(|t| t.name == "blackboard_read")
            .expect("blackboard_read declared in request tools");
        let props = bb_def
            .parameters
            .get("properties")
            .expect("tool parameters properties");
        assert_eq!(props["failures_only"]["type"], "boolean", "{bb_def:?}");
        assert_eq!(props["search"]["type"], "string", "{bb_def:?}");
        assert_eq!(props["search"]["minLength"], 1, "{bb_def:?}");
        // A3 教学句：描述里 failures_only / search 用法可见。
        assert!(
            bb_def.description.contains("failures_only") && bb_def.description.contains("search"),
            "{bb_def:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0p S1（2026-09-07，ADR-0010 §14.61 设计 A1/A2）：自信息面经真实工具
    /// 链回达——`failures_only=true` 返回 failure_agg 聚合行集（P2-12 行
    /// 语义 + 回查指针）；`search=<literal>` 大小写不敏感命中早期失败行与
    /// actions receipt（行含 order_id 指针）；两响应均挂 [黑板增量] 头
    /// （live PULL 面一致纪律）。
    #[tokio::test]
    async fn blackboard_read_serves_failures_only_and_search_faces() {
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
                arguments: serde_json::json!({"section": "exec", "failures_only": true}),
                call_id: "call-sh-fo".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "exec", "search": "MEMORYERROR"}),
                call_id: "call-sh-search".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        {
            let mut bb = controller.blackboard().write();
            // 早期轮失败聚合（F4 身份 + 错误码）。
            bb.failure_agg.record(
                "cmd_target",
                "id-a",
                "python train.py",
                "tool_timeout",
                10.0,
                3,
                orz_assurance::lif::Domain::Normal,
            );
            // 早期轮 exec 失败行（折叠视图默认不含的行）。
            bb.exec.errors.push(crate::blackboard::ExecEntry::stamped(
                "[workspace.run_terminal] pip install fasttext → MemoryError: bad allocation"
                    .to_string(),
                2,
                orz_assurance::lif::Domain::Normal,
                "2026-09-07T00:00:00Z".to_string(),
            ));
            // receipt 失败（message 是检索面的一部分；行带 order_id 指针）。
            bb.actions.push_result(crate::blackboard::ActionResult {
                order_id: "ORD-SH-1".into(),
                action: Some("workspace.run_terminal".into()),
                ok: false,
                response: None,
                error: Some(serde_json::json!({
                    "step": "execute",
                    "code": "execution_failed",
                    "message": "g++: fatal error: MemoryError at fasttext.o",
                })),
                trace_id: "t-sh-1".into(),
                timestamp: "2026-09-07T00:00:01Z".into(),
                round: 4,
                domain: Some(orz_assurance::lif::Domain::Normal),
            });
        }
        controller
            .run_turn(&host, "看自历史", "RUN-SH", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();

        // —— failures_only 面：P2-12 行语义 + 指针行 + 增量头。
        let fo_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-sh-fo"))
            })
            .expect("round carrying failures_only reply");
        let fo_reply = fo_round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-sh-fo"))
            .expect("failures_only tool result message");
        assert!(
            fo_reply
                .content
                .contains("== exec · 失败聚合（failures_only=true）=="),
            "{:?}",
            fo_round.messages
        );
        assert!(
            fo_reply
                .content
                .contains("[失败目标 cmd_target] python train.py ×1"),
            "{:?}",
            fo_round.messages
        );
        assert!(
            fo_reply.content.contains("domain/round_from/round_to"),
            "{:?}",
            fo_round.messages
        );
        assert!(
            fo_reply
                .content
                .lines()
                .next()
                .unwrap()
                .starts_with("[黑板 增量 水位"),
            "{:?}",
            fo_round.messages
        );

        // —— search 面：早期失败行 + receipt（大小写不敏感）。
        let search_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-sh-search"))
            })
            .expect("round carrying search reply");
        let search_reply = search_round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-sh-search"))
            .expect("search tool result message");
        assert!(
            search_reply
                .content
                .contains("== exec · search \"MEMORYERROR\"（命中 2 条）=="),
            "{:?}",
            search_round.messages
        );
        assert!(
            search_reply.content.contains("r2 | exit=err |"),
            "{:?}",
            search_round.messages
        );
        assert!(
            search_reply.content.contains("ORD-SH-1"),
            "{:?}",
            search_round.messages
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0p S1 组合守卫：六类非法组合全部显式报错（exit 1 + error 文案回达
    /// 模型，绝不静默忽略——同非法 epoch/receipt_id 纪律；F-H 补全
    /// ×since/×expand/×epoch 三例）。
    #[tokio::test]
    async fn blackboard_read_selfhistory_combo_guards_error_explicitly() {
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
                arguments: serde_json::json!(
                    {"section": "exec", "failures_only": true, "search": "x"}
                ),
                call_id: "call-g1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "plan", "failures_only": true}),
                call_id: "call-g2".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!(
                    {"section": "exec", "search": "x", "receipt_id": "ORD-1"}
                ),
                call_id: "call-g3".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!(
                    {"section": "exec", "search": "x", "since_timestamp": "2026-01-01T00:00:00Z"}
                ),
                call_id: "call-g4".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!(
                    {"section": "exec", "search": "x", "domain": "normal",
                     "round_from": 1, "round_to": 2}
                ),
                call_id: "call-g5".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!(
                    {"section": "exec", "failures_only": true, "epoch": 1}
                ),
                call_id: "call-g6".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "守卫", "RUN-SH-GUARD", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();
        let reply_of = |call_id: &str| {
            received
                .iter()
                .find_map(|r| {
                    r.messages
                        .iter()
                        .find(|m| m.tool_call_id.as_deref() == Some(call_id))
                })
                .unwrap_or_else(|| panic!("reply for {call_id}"))
        };
        assert!(
            reply_of("call-g1")
                .content
                .contains("failures_only 与 search 互斥"),
            "{:?}",
            reply_of("call-g1").content
        );
        assert!(
            reply_of("call-g2")
                .content
                .contains("仅与 section=exec 组合有效"),
            "{:?}",
            reply_of("call-g2").content
        );
        assert!(
            reply_of("call-g3")
                .content
                .contains("与 failures_only/search 互斥"),
            "{:?}",
            reply_of("call-g3").content
        );
        assert!(
            reply_of("call-g4")
                .content
                .contains("since_timestamp 与 failures_only/search 互斥"),
            "{:?}",
            reply_of("call-g4").content
        );
        assert!(
            reply_of("call-g5")
                .content
                .contains("expand（domain/round_from/round_to）与 failures_only/search 互斥"),
            "{:?}",
            reply_of("call-g5").content
        );
        assert!(
            reply_of("call-g6")
                .content
                .contains("failures_only/search 是 live 面"),
            "{:?}",
            reply_of("call-g6").content
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0p S1 复审 F-C 最小闭合（2026-09-07）：命令级失败（工具 Ok 臂 +
    /// exit≠0）补盖章 failure_agg——run_terminal_cmd → cmd_target、
    /// code=exit_1；exec 行回填真实退出码，search 面渲染 exit=1（而非
    /// 误导性 exit=ok）。聚合行随后被 failures_only 面回达模型。
    #[tokio::test]
    async fn command_exit_failure_stamps_failure_agg_and_search_shows_exit() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "pip install fasttext\r\nMemoryError: bad allocation".to_string(),
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "run_terminal_cmd".to_string(),
                arguments: serde_json::json!({"command": "pip install fasttext"}),
                call_id: "call-fc-1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "exec", "failures_only": true}),
                call_id: "call-fc-2".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "exec", "search": "pip install"}),
                call_id: "call-fc-3".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "跑命令", "RUN-FC", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        // 聚合面：cmd_target ×1，code=exit_1；exec 行携带真实退出码。
        {
            let bb = controller.blackboard().read();
            assert_eq!(bb.failure_agg.rows.len(), 1, "命令级失败必须补盖章");
            let row = &bb.failure_agg.rows[0];
            assert_eq!(row.kind, "cmd_target");
            assert_eq!(row.count, 1);
            assert_eq!(row.codes.len(), 1);
            assert_eq!(row.codes[0].code, "exit_1");
            assert_eq!(bb.exec.results[0].exit_code, Some(1));
        }
        let received = fake.received_requests();
        let fo_reply = received
            .iter()
            .find_map(|r| {
                r.messages
                    .iter()
                    .find(|m| m.tool_call_id.as_deref() == Some("call-fc-2"))
            })
            .expect("failures_only reply");
        assert!(
            fo_reply.content.contains("[失败目标 cmd_target]")
                && fo_reply.content.contains("exit_1"),
            "{:?}",
            fo_reply.content
        );
        let search_reply = received
            .iter()
            .find_map(|r| {
                r.messages
                    .iter()
                    .find(|m| m.tool_call_id.as_deref() == Some("call-fc-3"))
            })
            .expect("search reply");
        assert!(
            search_reply.content.contains("exit=1 | "),
            "search row must carry the real command exit: {:?}",
            search_reply.content
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-13 B3（2026-09-03，ADR-0010 §14.52 / 设计 §12 R4）：`epoch` 参数
    /// 生产面退役——未配置归档目录的控制器（生产面）模型工具声明不含
    /// `epoch`（live 参数 receipt_id/domain 等仍在）；配置归档目录的
    /// `--plan`/测试域恢复声明（cross-epoch 归档读入口）。
    #[tokio::test]
    async fn blackboard_read_epoch_param_retired_unless_archive_dir_configured() {
        async fn declared_epoch(archive_dir: Option<std::path::PathBuf>) -> bool {
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
                    arguments: serde_json::json!({"section": "exec"}),
                    call_id: "call-epoch-decl".to_string(),
                }]),
                ScriptedResponse::text("完成"),
                ScriptedResponse::text("完成"),
            ]));
            let gateway: Arc<dyn ModelGateway> = fake.clone();
            let controller =
                AgentLoopController::with_gateway(gateway).with_blackboard_archive_dir(archive_dir);
            controller
                .run_turn(
                    &host,
                    "看 blackboard_read 声明",
                    "RUN-EPOCH-DECL",
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
                        .any(|m| m.tool_call_id.as_deref() == Some("call-epoch-decl"))
                })
                .expect("round carrying blackboard_read");
            let bb_def = round
                .tools
                .iter()
                .find(|t| t.name == "blackboard_read")
                .expect("blackboard_read declared in request tools");
            let epoch_declared = bb_def
                .parameters
                .get("properties")
                .and_then(|p| p.get("epoch"))
                .is_some();
            let _ = std::fs::remove_dir_all(&dir);
            epoch_declared
        }

        assert!(
            !declared_epoch(None).await,
            "production tool def must NOT declare the retired epoch parameter"
        );
        let plan_dir = test_dir();
        assert!(
            declared_epoch(Some(plan_dir.clone())).await,
            "archive-configured (--plan/test) tool def must declare epoch for cross-epoch reads"
        );
        let _ = std::fs::remove_dir_all(&plan_dir);
    }

    /// P2-13 B2（2026-09-03，设计 §9.3/R2 + §12）：展开参数组合守卫全部
    /// fail loud（exit_code 1 + error 字段，事件面记录；模型面收到显式错误）
    /// ——all-or-none、非法域名、round 范围倒置、与 receipt_id 互斥、
    /// 非可折叠分区拒绝。
    #[tokio::test]
    async fn blackboard_read_fold_expand_guards_fail_loud() {
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
        let calls = vec![
            serde_json::json!({"section": "exec", "domain": "normal", "round_from": 1}),
            serde_json::json!({"section": "exec", "domain": "bogus", "round_from": 1, "round_to": 2}),
            serde_json::json!({"section": "edits", "domain": "normal", "round_from": 2, "round_to": 1}),
            serde_json::json!({"section": "exec", "receipt_id": "ORD-1", "domain": "normal", "round_from": 1, "round_to": 2}),
            serde_json::json!({"section": "plan", "domain": "normal", "round_from": 1, "round_to": 2}),
        ];
        let mut script = Vec::new();
        for (i, args) in calls.iter().enumerate() {
            script.push(ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: args.clone(),
                call_id: format!("call-fold-guard-{i}"),
            }]));
        }
        script.push(ScriptedResponse::text("完成"));
        script.push(ScriptedResponse::text("完成"));
        let fake = Arc::new(FakeProvider::new(script));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "守卫测试",
                "RUN-FOLD-GUARD",
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
            .map(|e| e.payload)
            .collect();
        assert_eq!(payloads.len(), 5, "{payloads:?}");
        let hints = [
            "必须同时给出",
            "invalid blackboard_read domain",
            "round_to=1 < round_from=2",
            "与 receipt_id 互斥",
            "仅与 exec|edits|tool_actions",
        ];
        for (i, hint) in hints.iter().enumerate() {
            let p = payloads
                .iter()
                .find(|p| p["call_id"] == format!("call-fold-guard-{i}"))
                .unwrap_or_else(|| panic!("missing payload {i}: {payloads:?}"));
            assert_eq!(p["exit_code"], 1, "{p:?}");
            assert!(
                p["error"].as_str().unwrap_or_default().contains(hint),
                "{i}: {p:?}"
            );
        }
        // 模型面：错误文本回达（取任一守卫的代表性消息）。
        let received = fake.received_requests();
        assert!(received.iter().any(|r| {
            r.messages.iter().any(|m| {
                m.tool_call_id.as_deref() == Some("call-fold-guard-0")
                    && m.content.contains("必须同时给出")
            })
        }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-13 B2（2026-09-03，设计 §9.2/§9.3 + §12 R5）：exec 分区 live
    /// 字符 ≥ T 时默认读取进入折叠态（更早域段折叠为标注、当前域段 +
    /// 最近 K 轮 + 最近 20% 行展开）；携带 domain+round 显式展开可精读目标
    /// 轮段（跨会话恢复的 LIF 轮号/域为轴）。
    #[tokio::test]
    async fn blackboard_read_folded_exec_default_view_and_expand() {
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
                arguments: serde_json::json!({"section": "exec"}),
                call_id: "call-fold-1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({
                    "section": "exec",
                    "domain": "normal",
                    "round_from": 1,
                    "round_to": 2,
                }),
                call_id: "call-fold-2".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        // 会话恢复续接：当前轮 r400 / pressure（B1 conversation-relative 轴）。
        controller.restore_lif_session(
            &orz_assurance::lif::TemporalSessionSnapshot {
                round: 400,
                has_success: true,
                current_domain: orz_assurance::lif::Domain::Pressure,
                entry_round: 301,
                spikes: Vec::new(),
            },
            Some(AgentLoopController::now_epoch_secs()),
        );
        {
            let mut bb = controller.blackboard().write();
            // normal r1–r300：每行 >220 字符 → 分区 live 字符 >64K（T 触发）。
            for r in 1..=300 {
                bb.exec.results.push(crate::blackboard::ExecEntry::stamped(
                    format!("n{r}-{}", "x".repeat(220)),
                    r,
                    orz_assurance::lif::Domain::Normal,
                    format!("2026-09-03T00:{:02}:00Z", r % 60),
                ));
            }
            // pressure r301–r400（当前域段，短行）。
            for r in 301..=400 {
                bb.exec.results.push(crate::blackboard::ExecEntry::stamped(
                    format!("p{r}"),
                    r,
                    orz_assurance::lif::Domain::Pressure,
                    format!("2026-09-03T01:{:02}:00Z", r % 60),
                ));
            }
        }
        controller
            .run_turn(
                &host,
                "看折叠 exec",
                "RUN-FOLD-EXEC",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let fold_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-fold-1"))
            })
            .expect("round carrying folded exec reply");
        let fold_reply = fold_round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-fold-1"))
            .expect("folded exec tool result message");
        // 旧 normal 段折叠为标注（计数 + 轮区间）；当前 pressure 段行可见；
        // 折叠段明细行不注入默认视图。
        assert!(
            fold_reply.content.contains("[域段 normal r1–r300 · 300 条"),
            "{:?}",
            fold_round.messages
        );
        assert!(
            fold_reply.content.contains("p301"),
            "{:?}",
            fold_round.messages
        );
        assert!(
            fold_reply.content.contains("p400"),
            "{:?}",
            fold_round.messages
        );
        assert!(
            !fold_reply.content.contains("n1-"),
            "{:?}",
            fold_round.messages
        );

        let expand_round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-fold-2"))
            })
            .expect("round carrying expand reply");
        let expand_reply = expand_round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-fold-2"))
            .expect("expand tool result message");
        assert!(
            expand_reply.content.contains("n1-"),
            "{:?}",
            expand_round.messages
        );
        assert!(
            expand_reply.content.contains("n2-"),
            "{:?}",
            expand_round.messages
        );
        assert!(
            expand_reply
                .content
                .contains("[域段 normal r3–r300 · 298 条"),
            "{:?}",
            expand_round.messages
        );
        assert!(
            expand_reply.content.contains("p301"),
            "{:?}",
            expand_round.messages
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

    /// P2-13 B2 复审（2026-09-03，P2-1 e2e）：exec 分区已触发折叠态时携带
    /// `receipt_id`（无展开参数）仍返回显式“receipt_id 仅 actions”文本错误
    /// （render-error 形状 → 不挂增量头、不推进游标），绝不静默忽略点读
    /// 参数并回折叠内容。
    #[tokio::test]
    async fn blackboard_read_receipt_id_on_fold_triggered_exec_errors_not_fold() {
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
                arguments: serde_json::json!({"section": "exec", "receipt_id": "ORD-1"}),
                call_id: "call-fold-rid".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        {
            let mut bb = controller.blackboard().write();
            for r in 1..=300 {
                bb.exec.results.push(crate::blackboard::ExecEntry::stamped(
                    format!("r{r}-{}", "x".repeat(220)),
                    r,
                    orz_assurance::lif::Domain::Normal,
                    format!("2026-09-03T00:{:02}:00Z", r % 60),
                ));
            }
        }
        controller
            .run_turn(
                &host,
                "折叠态点读守卫",
                "RUN-FOLD-RID",
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
                    .any(|m| m.tool_call_id.as_deref() == Some("call-fold-rid"))
            })
            .expect("round carrying fold-state receipt_id read");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-fold-rid"))
            .expect("receipt_id error tool result message");
        assert!(
            reply
                .content
                .contains("receipt_id 仅与 section=actions 组合有效"),
            "{:?}",
            round.messages
        );
        assert!(
            !reply.content.contains("[exec: 折叠视图"),
            "{:?}",
            round.messages
        );
        assert!(!reply.content.contains("黑板 增量"), "{:?}", round.messages);
        let _ = std::fs::remove_dir_all(&dir);
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
                round: 0,
                domain: None,
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
                round: 0,
                domain: None,
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
                .all(|m| !m.content.starts_with("[黑板 增量")),
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
                round: 0,
                domain: None,
                run: String::new(),
            });
            w.push_tool_action(ToolActionRecord {
                category: "read".into(),
                tool: "read_file".into(),
                timestamp: "t".into(),
                round: 0,
                domain: None,
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
        assert_eq!(get("notes"), 1);
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
            round: 0,
            domain: None,
        });
        assert_eq!(board.revision, 5);
        // 空槽消费不计数。
        assert!(board.take_order().is_none());
        assert_eq!(board.revision, 5);
    }

    /// B1 会话化基础：exec 旧归档字符串（无章）可解析为 pre-stamp
    /// `ExecEntry`（round=0/domain=None/ts 空）；新写章行带
    /// (round, domain, ts) 且 JSON 往返不丢章。
    #[test]
    fn exec_legacy_strings_parse_as_pre_stamp_and_stamped_roundtrips() {
        let json =
            r#"{"results":["ok 一","ok 二"],"observations":[],"errors":["错"],"auth_requests":[]}"#;
        let section: ExecSection = serde_json::from_str(json).unwrap();
        assert_eq!(section.results.len(), 2);
        assert!(section.results[0].is_pre_stamp());
        assert_eq!(section.results[0].text, "ok 一");
        assert_eq!(section.results[1].text, "ok 二");
        assert!(section.errors[0].is_pre_stamp());

        let stamped = ExecEntry::stamped(
            "跑批完成".into(),
            7,
            Domain::Normal,
            "2026-09-03T00:00:00Z".into(),
        );
        let mut section2 = ExecSection::default();
        section2.results.push(stamped.clone());
        let json2 = serde_json::to_string(&section2).unwrap();
        let back: ExecSection = serde_json::from_str(&json2).unwrap();
        assert_eq!(back.results.len(), 1);
        assert_eq!(back.results[0].round, 7);
        assert_eq!(back.results[0].domain, Some(Domain::Normal));
        assert_eq!(back.results[0].ts, "2026-09-03T00:00:00Z");
    }

    /// B1 会话化基础：`conversation_snapshot` 剥离 run 级机械面（动作栏
    /// 注册板块 / pending 订单槽 / 依赖图），保留分区内容与章；JSON 序列
    /// 化不携带 dep_graph（live-only）；`restore_conversation_snapshot`
    /// 后各分区版本计数归位为 1（恢复 = 一次整体变化，跨 prompt 徽章
    /// 不延续上 run 累积计数；entities 自含计数同此，dep_graph 空图
    /// 保持 0）。
    #[test]
    fn conversation_snapshot_and_restore_keep_board_but_strip_run_faces() {
        let mut bb = Blackboard::default();
        bb.actions.set_registration(vec![ActionRegistration {
            name: "workspace.read_file".into(),
            description: "读文件".into(),
            parameters: serde_json::json!({}),
            target_policy: crate::entities::TargetPolicy::File,
        }]);
        bb.actions
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
        bb.push_edit(EditRecord {
            file: "a.py".into(),
            old_lines: 1,
            new_lines: 2,
            timestamp: "2026-09-03T00:00:00Z".into(),
            round: 3,
            domain: Some(Domain::Normal),
            run: String::new(),
        });
        bb.push_tool_action(ToolActionRecord {
            category: "read".into(),
            tool: "read_file".into(),
            timestamp: "2026-09-03T00:00:00Z".into(),
            round: 3,
            domain: Some(Domain::Normal),
        });
        bb.push_exec_result(ExecEntry::stamped(
            "[read_file] 内容".into(),
            3,
            Domain::Normal,
            "2026-09-03T00:00:00Z".into(),
        ));
        bb.actions.push_result(ActionResult {
            order_id: "ORD-1".into(),
            action: Some("workspace.read_file".into()),
            ok: true,
            response: None,
            error: None,
            trace_id: "t".into(),
            timestamp: "2026-09-03T00:00:00Z".into(),
            round: 3,
            domain: Some(Domain::Normal),
        });

        // 侧车 JSON 不携带 dep_graph（live-only）。
        let serialized = serde_json::to_string(&bb).unwrap();
        assert!(
            !serialized.contains("\"dep_graph\""),
            "dep graph not persisted"
        );

        let snap = bb.conversation_snapshot();
        assert!(
            snap.actions.registration.is_empty(),
            "registration is run-local"
        );
        assert!(snap.actions.order.is_none(), "pending order is run-local");
        assert_eq!(snap.actions.results.len(), 1, "receipts persist");
        assert_eq!(snap.edits.len(), 1);
        assert_eq!(snap.tool_actions.len(), 1);
        assert_eq!(snap.exec.results.len(), 1);

        let mut restored = Blackboard::new();
        restored.restore_conversation_snapshot(snap);
        assert_eq!(restored.edits[0].round, 3);
        assert_eq!(restored.tool_actions[0].round, 3);
        assert_eq!(restored.exec.results[0].round, 3);
        let rev = restored.partition_revisions();
        for (name, value) in rev {
            match name {
                "plan" | "notes" | "exec" | "edits" | "tool_actions" | "actions"
                | "internal_ret" | "external_ret" | "entities" => {
                    assert_eq!(value, 1, "{name} shows one restore change")
                }
                "deps" => assert_eq!(value, 0, "deps stays silent (empty graph until touched)"),
                other => panic!("unexpected partition {other}"),
            }
        }
    }

    /// TER T1.6 (2026-09-04): `blackboard_read section=processes` 经真实
    /// 工具链回达模型——live 分区渲染（无事实时「（无）」）；工具定义
    /// section 枚举已声明 processes。
    #[tokio::test]
    async fn blackboard_read_serves_processes_section() {
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
                arguments: serde_json::json!({"section": "processes"}),
                call_id: "call-p1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_plan(
            "PLAN-PROC".to_string(),
            1,
            "构建".to_string(),
            vec!["侦查".to_string()],
        );
        controller
            .run_turn(
                &host,
                "读进程面",
                "RUN-PROC1",
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
                    .any(|m| m.tool_call_id.as_deref() == Some("call-p1"))
            })
            .expect("round carrying blackboard_read processes reply");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-p1"))
            .expect("processes tool result message");
        assert!(
            reply.content.contains("== processes (live) ==") && reply.content.contains("（无）"),
            "processes reply: {:?}",
            round.messages
        );
        // 工具定义增量扩展：blackboard_read 的 section 枚举含 processes。
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
            sections.iter().any(|v| v.as_str() == Some("processes")),
            "processes must be declared in the section enum: {sections:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TER T1.6 (2026-09-04): processes live 分区的越权组合（epoch /
    /// receipt_id）显式报错——事件面 ToolCompleted exit_code 1 + error，
    /// 模型面收到错误文本；不静默回退。
    #[tokio::test]
    async fn blackboard_read_processes_combination_errors_are_explicit() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "processes", "epoch": 1}),
                call_id: "call-p-e".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "processes", "receipt_id": "ORD-1"}),
                call_id: "call-p-r".to_string(),
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
                "RUN-PROCCOMBO",
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
            .map(|e| e.payload)
            .collect();
        assert_eq!(payloads.len(), 2, "{payloads:?}");
        let epoch_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-p-e")
            .expect("epoch combo completed");
        assert_eq!(epoch_payload["exit_code"], 1, "{epoch_payload:?}");
        assert!(
            epoch_payload["error"]
                .as_str()
                .unwrap()
                .contains("processes is a live-only partition"),
            "{epoch_payload:?}"
        );
        let receipt_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-p-r")
            .expect("receipt combo completed");
        assert_eq!(receipt_payload["exit_code"], 1, "{receipt_payload:?}");
        assert!(
            receipt_payload["error"]
                .as_str()
                .unwrap()
                .contains("当前 section=processes 不支持 receipt_id"),
            "{receipt_payload:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TER T1.12 (W-F11)：`blackboard_read section=env` 经真实工具链回达
    /// 模型——live 分区渲染（host 无快照时「（无）」）；section 枚举声明
    /// env。
    #[tokio::test]
    async fn blackboard_read_serves_env_section() {
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
                arguments: serde_json::json!({"section": "env"}),
                call_id: "call-env1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_plan(
            "PLAN-ENV".to_string(),
            1,
            "构建".to_string(),
            vec!["侦查".to_string()],
        );
        controller
            .run_turn(&host, "读环境面", "RUN-ENV1", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-env1"))
            })
            .expect("round carrying blackboard_read env reply");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-env1"))
            .expect("env tool result message");
        assert!(
            reply.content.contains("== env (live) ==") && reply.content.contains("（无）"),
            "env reply: {:?}",
            round.messages
        );
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
            sections.iter().any(|v| v.as_str() == Some("env")),
            "env must be declared in the section enum: {sections:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TER T1.12 (W-F11)：env live 分区越权组合（epoch / receipt_id）
    /// 显式报错——事件面 exit_code 1 + error，模型面收到文本；不静默回退。
    #[tokio::test]
    async fn blackboard_read_env_combination_errors_are_explicit() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "env", "epoch": 1}),
                call_id: "call-env-e".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({"section": "env", "receipt_id": "ORD-1"}),
                call_id: "call-env-r".to_string(),
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
                "RUN-ENVCOMBO",
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
            .map(|e| e.payload)
            .collect();
        assert_eq!(payloads.len(), 2, "{payloads:?}");
        let epoch_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-env-e")
            .expect("epoch combo completed");
        assert_eq!(epoch_payload["exit_code"], 1, "{epoch_payload:?}");
        assert!(
            epoch_payload["error"]
                .as_str()
                .unwrap()
                .contains("env is a live-only partition"),
            "{epoch_payload:?}"
        );
        let receipt_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-env-r")
            .expect("receipt combo completed");
        assert_eq!(receipt_payload["exit_code"], 1, "{receipt_payload:?}");
        assert!(
            receipt_payload["error"]
                .as_str()
                .unwrap()
                .contains("当前 section=env 不支持 receipt_id"),
            "{receipt_payload:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
