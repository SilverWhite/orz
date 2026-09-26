//! LoopHost trait — the interface orz-host must implement.
//!
//! Defined in orz-loop per Codex discipline: orz-loop owns the agent loop contract;
//! orz-host bridges Grok providers to fulfill it.
//!
//! See: INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §3.4

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use orz_assurance::journal::JournalRecorder;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Risk classification for permission requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskClass {
    ReadOnly,
    LocalMutation,
    NetworkCall,
    SandboxEscape,
}

/// Session permission policy (IP2a, FIX_PLAN 2026-08-06 D-3) — the policy
/// the host's permission bridge enforces for this session. The loop uses it
/// for the NAME-LEVEL tool-availability projection: tools the policy refuses
/// outright (e.g. network/shell under Benchmark) are filtered from the
/// model-visible tool declarations, so the model never attempts them.
/// Mirrors orz-host's `PermissionPolicy` (defined here to avoid a
/// loop→host dependency; the host maps its policy onto this enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolPolicy {
    /// Normal prompt-decides behavior — all tools are declared; denial is
    /// scope/argument-level at permission time.
    #[default]
    Interactive,
    /// Read-only sandbox — only read-class tools are declared.
    ReadOnly,
    /// Headless benchmark — read + local file edits declared; network and
    /// shell-escape excluded by name.
    Benchmark,
    /// FUS-BENCHMARK-FULL-EXEC (2026-08-18): headless benchmark with the
    /// shell axis open (read + write + terminal declared). Network does not
    /// participate in the name-level projection — web tools are not on the
    /// work-tool/console face; the permission gate is their only gate.
    /// Mapped from `PermissionPolicy::Benchmark { allow_shell: true, .. }`.
    BenchmarkFull,
}

/// The refusing gate family behind a structured policy denial
/// (P0-C S3 前置, 2026-08-15, P1-2 定案).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDenialSource {
    /// The per-call permission bridge (user deny / headless defer).
    Permission,
    /// The ACAF ticket gate (Slice 2 fail-closed refusals).
    Acaf,
    /// The retrieval-mode gates (off / framework_fallback / local_browser).
    RetrievalMode,
    /// Reserved: taint action-combination policy (design item, not yet
    /// wired at runtime).
    Taint,
}

impl PolicyDenialSource {
    /// Machine-readable source key (`permission` | `acaf` |
    /// `retrieval_mode` | `taint`) — matches the serde snake_case names.
    pub fn as_str(&self) -> &'static str {
        match self {
            PolicyDenialSource::Permission => "permission",
            PolicyDenialSource::Acaf => "acaf",
            PolicyDenialSource::RetrievalMode => "retrieval_mode",
            PolicyDenialSource::Taint => "taint",
        }
    }
}

/// Structured policy denial (P0-C S3 前置, 2026-08-15, P1-2 定案):
/// `run_host_tool` refusals carry `{source, code, reason}` instead of
/// relying on stable output prefixes; the console adapter maps ONLY this
/// structured signal to `ExecuteError::PolicyDenied`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDenial {
    pub source: PolicyDenialSource,
    pub code: String,
    pub reason: String,
}

/// R2 半助理层：结构化工具错误类别（映射自 `ToolError`，供失败诊断
/// 签名词典消费——P5：结构化字段，不承载自由文本）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorKind {
    NotFound,
    Timeout,
    ExecutionFailed,
}

/// Result returned by a tool invocation.
#[derive(Debug, Clone, Default)]
pub struct ToolResult {
    pub output: String,
    pub exit_code: Option<i32>,
    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.3): 本次调用发生的浏览器
    /// 启动/探活尝试事实——host 在 browser_read 调用期懒启动并注入句柄时
    /// 填充；loop 在 ToolStarted 与 ToolCompleted 之间落
    /// `browser_launch_result`（success/failure + 真实原因）。成功路径在
    /// 调用完成前消费；`None` = 本次调用未发生启动尝试。
    pub browser_launch_fact: Option<BrowserLaunchFact>,
    /// 0p S2 两段门（2026-09-07，ADR-0010 §14.61 设计 B4）：本次调用经
    /// 通知后放行了会话卷内部区读取——随 ToolCompleted 事件面落
    /// `session_volume_opened`（journal 记 `open_after_notice` 审计）。
    pub session_volume_opened: bool,
    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.3): workspace changes a
    /// mutation-capable host tool call caused (capped list; same semantics
    /// as the run_tests delta). Populated by orz-host for mutation tools;
    /// read-only tools and mock hosts leave it empty.
    pub workspace_delta: Vec<WorkspaceDeltaEntry>,
    /// AGENT-DELIVERY-FLOW: true when the per-call delta list was truncated
    /// at the host cap.
    pub workspace_delta_truncated: bool,
    /// P0-C S4 (2026-08-16): true when the host hit a per-call wall-clock
    /// bound and killed the tool's process tree before completion. The
    /// console adapter maps this structured signal to `tool_timeout` /
    /// `script_timeout` (never text prefix parsing).
    pub timed_out: bool,
    /// Decode stage that produced `output` (GAP-ENCODING-GATE,
    /// OPS-PROTOCOL §8): `utf-8` / `utf-8-sig` / `gb18030` / `utf-8-lossy:<p>%`
    /// (comma-joined for multi-chunk streams). `None` when the tool has no
    /// mechanical decode stage. The controller journals it as
    /// `tool_completed.output_encoding`. FUS-HOST-RESOURCE-SAFETY §4.4
    /// (2026-09-12, 0z S1) adds `utf-16le` / `utf-16be` for text-family files
    /// that `read_file`'s decode-first gate recovers instead of rejecting.
    pub output_encoding: Option<String>,
    /// TER T1.11 (W-F13b)：run_terminal_cmd 输出被截断时的结构化事实——
    /// 完整输出已落盘为检索对象（`output_object_id` 可 pattern/行区间/
    /// 尾部检索），模型无需 .gsa 摸黑补读。
    pub output_truncated: bool,
    pub output_object: Option<TerminalOutputObject>,
    /// FUS-RETRIEVAL-MECH B-1 (2026-08-13): optional structured tool
    /// metadata forwarded by the host across the loop seam. Today only
    /// `web_search` fills it — `{"citations": ["https://…", …]}` — the
    /// citation URL candidate pool the mechanical prefilter will consume.
    /// All other tools leave it `None`; it is never derived from the
    /// model-visible output text.
    pub structured: Option<serde_json::Value>,
    /// P0-C S3 前置 (2026-08-15, P1-2 定案): structured policy denial when
    /// the call was refused by a policy gate (permission / ACAF ticket /
    /// retrieval mode; taint reserved). `None` for every executed call and
    /// for non-policy refusals (candidate cap, inject budget, runner
    /// availability). The console adapter classifies policy refusals ONLY
    /// from this field — the old stable-output-prefix judgment is retired.
    pub policy_denial: Option<PolicyDenial>,
    /// R2 半助理层：调用物/工具结构化错误类别（`run_host_tool` 捕获
    /// `ToolError` 时填充；成功调用为 `None`）。失败诊断据此匹配
    /// `tool_not_found` 等签名，不做文本子串判定。
    pub tool_error_kind: Option<ToolErrorKind>,
    /// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): mid-run status for
    /// a terminal command auto-backgrounded at the 300s report point — the
    /// command is still running; the model-facing report and the
    /// `tool_running` journal event are built from these structured facts
    /// (never text parsing). `None` for every other call.
    pub mid_run: Option<ToolMidRunStatus>,
}

/// 0t (2026-09-09, ADR-0010 §14.65): 浏览器启动/探活尝试事实。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserLaunchFact {
    pub status: BrowserLaunchStatus,
    /// failure 时携带真实原因类别（browser_not_found/网络/超时等）；
    /// success 为 None。
    pub cause: Option<String>,
}

impl BrowserLaunchFact {
    pub fn success() -> Self {
        Self {
            status: BrowserLaunchStatus::Success,
            cause: None,
        }
    }

    pub fn failure(cause: impl Into<String>) -> Self {
        Self {
            status: BrowserLaunchStatus::Failure,
            cause: Some(cause.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserLaunchStatus {
    Success,
    Failure,
}

/// TER T1.11 (W-F13b)：截断输出的持久化对象事实。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalOutputObject {
    /// 截断前的真实单调输出字节。
    pub total_bytes: u64,
    /// 检索对象指针 = 落盘 log 路径（read_file/grep/offset/tail 语义
    /// 均以该对象为准）。
    pub output_object_id: String,
}

/// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): structured mid-run
/// facts for a terminal command that is still running after the report
/// point (auto-backgrounded). The controller journals `tool_running` from
/// these fields; the model-visible report text is composed by the tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolMidRunStatus {
    /// Background task id (the original tool call id for auto-background).
    pub task_id: String,
    /// Shell process pid, when the backend could surface it.
    pub pid: Option<u32>,
    /// Path of the output file the command keeps writing to.
    pub output_file: String,
    /// Total output bytes observed so far (before truncation).
    pub total_bytes: Option<u64>,
}

/// TER T1.6 (2026-09-04): 黑板 `section=processes` live 分区的结构化事实
/// ——host 把终端读取时现算快照映射为本类型（`status` 取值 running /
/// idle / completed / killed）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveProcessFact {
    pub task_id: String,
    pub command: String,
    pub display_command: Option<String>,
    pub pid: Option<u32>,
    pub elapsed_ms: u64,
    pub status: String,
    pub total_bytes: u64,
    pub cpu_micros: u64,
    pub killable: bool,
    pub owner_session_id: Option<String>,
    pub description: Option<String>,
}

/// TER 全面审查 P1-1（2026-09-04）：后台任务 idle-kill 生命周期事实——
/// host 在工具执行边界回收“自上次回收以来被 idle 机制 kill”的后台任务；
/// `task_id` = 原 auto-bg 调用 id（auto-background 语义下 task 以工具
/// call_id 注册，见 T0.2 §4 / T1.6）。loop 侧据此补记
/// `tool_running(status=idle_killed + reason)` 事件（链规则要求该调用先有
/// mid-run `tool_running` + `running:true` `tool_completed`）——见下方
/// `TerminalIdleKillFact`。
/// 0z S2 §4.2（2026-09-12，FUS-HOST-RESOURCE-SAFETY）：进程树扫除事实——
/// 宿主扫除器（装配期扫上轮孤儿 / 收尾扫本 run 泄漏）产出的 audit-first
/// 行，由 loop 落 `process_tree_reaped` 事件。drain 语义同 idle-kill 面。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessTreeReapFact {
    /// `parent_abort`（装配期扫上轮孤儿）| `run_shutdown`（收尾扫本 run）。
    pub reason: String,
    /// `planned`（杀前审计行）| `executed`（实杀结果行）。
    pub phase: String,
    pub pids: Vec<u32>,
    pub call_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalIdleKillFact {
    /// 原 auto-bg 工具调用 id（终端 task id）。
    pub task_id: String,
    /// Shell 进程 pid（终端能提供时）。
    pub pid: Option<u32>,
    /// kill 时刻前累计输出字节（截断前单调计数）。
    pub total_bytes: u64,
    /// 输出落盘路径（检索对象指针语义与 mid-run 一致）。
    pub output_file: String,
    /// 自调用 tool_started 起的墙钟毫秒（kill 时刻）。
    pub wall_ms: u64,
    /// kill 原因（按实际生效阈值渲染，非默认阈值不失真——TER 审查 S3）。
    pub reason: String,
}

/// 0ac S3①-b M2（2026-09-14, IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_
/// RETRIEVAL_DESIGN §2.1/§4.1「合法边界投递」）：**后台任务完成**的投递
/// 事实——host 在合法投递边界（B1 工具结果 / B2 轮结束）drain 自上次回收
/// 以来**新完成且尚未报告**的任务；loop 侧据此在边界注入模型可见事实并
/// 落 `result_delivered`（真投递面：`suppressed=false`）。
///
/// 与工具结果内联的 `TaskCompletionReminder` 互斥：两者共用同一「已报告」
/// 记账（mark_reported 语义），同一任务只经一个通道投给模型一次
/// （设计 §2.3「同一结果只投递一次」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTaskFact {
    /// 后台任务 id（auto-background 语义下即原工具调用 id）。
    pub task_id: String,
    /// 模型可见的机械事实文本（含输出指针；由来源侧单一源格式化——
    /// loop 只注入不重写）。
    pub report: String,
    /// 任务退出码（终端能提供时；完成即确定事实）。
    pub exit_code: Option<i32>,
}

/// TER T1.12 (W-F11, 2026-09-04)：黑板 `section=env` 的机械层代码工具
/// 环境快照事实（PULL 白名单面）。`kind` 取值 tool / language / package /
/// input / connectivity（渲染层白名单登记；越权 kind 渲染层拒绝）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvSnapshotFact {
    pub kind: String,
    pub key: String,
    pub value: String,
}

/// P2-3（2026-09-10）：宿主侧浏览器会话的 SERP **物理事实**——本会话已
/// 消耗的引擎导航次数与会话上限（`SERP_MAX_NAVIGATIONS_PER_SESSION`）。
///
/// 宿主工具接口不带车道身份（只有 `call_tool(name, args, call_id)`），
/// 所以"检索车道底线额度"这类**车道策略**只能在 loop 层施加；本类型是
/// loop 施加策略前需要的唯一机器事实（与 `LiveProcessFact` /
/// `EnvSnapshotFact` 同族：宿主报事实、loop 做策略，宿主不猜车道）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerpSessionFacts {
    /// 本浏览器会话已发生的引擎导航次数（跨车道共享的同一计数器）。
    pub navigations: u32,
    /// 本浏览器会话的引擎导航上限（host 侧物理兜底常量）。
    pub ceiling: u32,
}

impl SerpSessionFacts {
    /// 剩余可用导航次数（会话头寸）；饱和减，不产生下溢。
    pub fn headroom(&self) -> u32 {
        self.ceiling.saturating_sub(self.navigations)
    }
}

/// Lightweight error from tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("tool execution failed: {0}")]
    ExecutionFailed(String),
    /// 0ac S3①（2026-09-13，设计稿 §10.1 / §10.3 第 1 项）：真实 cause 与人类
    /// 消息**同行**回传——工具侧 `xai_tool_runtime::ToolError.details.cause`
    /// 的结构化自报码（`retrieval_lane_busy` / `network_no_response` /
    /// `network_error` / `capability_unreachable` / `empty_result` /
    /// `no_progress` …）经宿主桥接保留，loop 在 `tool_completed.cause` 上原样
    /// 落真实原因，而不是只落壳码 `execution_failed`（F-003 验收样本）。
    /// 稳定码族不变：本变体仍归 `execution_failed` 系（不新增码）。
    #[error("tool execution failed ({cause}): {message}")]
    ExecutionFailedCaused { cause: String, message: String },
    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.3): 浏览器启动/探活尝试
    /// 失败——cause 是真实原因类别（browser_not_found/网络/超时等），按
    /// 普通 host 错误回传（§3.4），非模式拒绝。loop 在 ToolCompleted 前落
    /// `browser_launch_result` 事实事件。
    #[error("browser launch failed: {0}")]
    BrowserLaunchFailed(String),
    /// 0t P1-2a（2026-09-09, S2-R P2 / 设计 §3.2 场景 S3）：启动/探活尝试
    /// 成功但本次浏览器动作失败——启动事实独立于页面结果，success
    /// `launch_fact` 随 Err 携带（loop 在 ToolCompleted 前落
    /// `browser_launch_result`）；`reason` 是页面/导航层真实错误（FP-2
    /// 正常回传，无教学句），稳定码归 ExecutionFailed 系（不新增码）。
    #[error("browser step failed: {reason}")]
    BrowserStepFailed {
        reason: String,
        launch_fact: BrowserLaunchFact,
    },
    /// P0-1 (2026-08-08 stall guards): the tool exceeded the host's
    /// per-call wall-clock budget and its process tree was killed. The
    /// reason carries the budget so the journal (`tool_completed.error`)
    /// and the model-facing message are self-explanatory.
    #[error("tool timeout: {0}")]
    Timeout(String),
}

/// Permission decision returned by the host's approval prompter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermitDecision {
    AllowOnce,
    AllowAlways,
    Deny,
    Defer,
}

/// 0bt④（2026-09-26；原 0bu「权限判定来源落账」并件）：权限判定**来源**
/// 观测面——封闭集，只加观测、**不改判定语义／阈值／fail-closed**。
///
/// 宿主在回传判定时可以一并自报来源；loop 将其原样落进
/// `permission_decision` 事件的 `source` 字段（payload schema 可选；
/// 旧 journal 回放零新增报错）。未分类宿主报 `None`，事件保持旧形状。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitSource {
    /// 策略/规则面判定（桥 ReadOnly・Benchmark 短路门；manager 内部读面
    /// 自动放行；manager 策略 deny 臂）。
    Policy,
    /// 桥读面 scope 校验拒绝。
    Scope,
    /// manager always-approve（yolo）快路自动放行。
    Yolo,
    /// 交互客户端裁决（实际收到放行/拒绝答复的两径）。
    User,
    /// 提示等待超时 → fail-closed。
    Timeout,
    /// 无客户端/未答复/取消 → fail-closed 默认拒。
    FailClosed,
    /// 保留：auto-mode 分类器家族（现休眠，生产不产出）。
    Classifier,
}

impl PermitSource {
    /// Stable wire string for the journal `source` field (closed set).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Policy => "policy",
            Self::Scope => "scope",
            Self::Yolo => "yolo",
            Self::User => "user",
            Self::Timeout => "timeout",
            Self::FailClosed => "fail_closed",
            Self::Classifier => "classifier",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PermitError {
    #[error("permission denied by user")]
    Denied,
    #[error("permission timeout")]
    Timeout,
}

/// A record of a single turn (prompt + response + tool calls).
#[derive(Debug, Clone)]
pub struct TurnRecord {
    pub turn_id: String,
    pub prompt: String,
    pub response_text: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
}

#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub arguments: Value,
    pub result: Option<ToolResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("persist io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CompactionError {
    #[error("compaction not supported in this session")]
    NotSupported,
}

/// A hook event from the agent lifecycle.
#[derive(Debug, Clone)]
pub struct HookEvent {
    pub event_type: HookEventType,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub enum HookEventType {
    PreToolUse,
    PostToolUse,
    SessionStart,
    SessionStop,
    TurnStart,
    TurnEnd,
}

/// Result of running a hook.
#[derive(Debug, Clone)]
pub struct HookResult {
    pub hook_name: String,
    pub outcome: HookOutcome,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum HookOutcome {
    Allowed,
    Blocked,
    Modified,
}

/// Credentials bundle (e.g. API keys).
#[derive(Debug, Clone)]
pub struct Credentials {
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("credential not found for provider: {0}")]
    NotFound(String),
}

/// Tool definition (name + description + parameters schema).
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// F-09 (2026-08-07 review): default wall-clock cap for the fixed test
/// command (30min — user decision: context size is the priority, not wall
/// time; the cap only catches true hangs). Hosts may override per
/// `TestRunner::timeout`.
pub const RUN_TESTS_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// F-09: cap on the test output the host collects (tail kept) — bounds
/// session memory against pathological test output.
pub const RUN_TESTS_OUTPUT_CAP: usize = 1024 * 1024;

/// F-09: cap on the test output injected into the conversation (tail kept —
/// test frameworks put their summary at the end). The full (capped) output
/// stays on disk; the model fetches it via read_file when it wants more.
pub const RUN_TESTS_CONTEXT_CAP: usize = 32 * 1024;

/// A fixed test-runner command (D-9, FIX_PLAN 2026-08-06) — the Aider-model
/// feedback loop inside a single run. The host owns the command (the model
/// never sees the test files, only stdout/stderr/exit code via the
/// `run_tests` tool); the harness injects it.
#[derive(Debug, Clone)]
pub struct TestRunner {
    /// argv-style command (e.g. `["python", "-m", "pytest", "<hidden-tests>/x_test.py", "-q"]`).
    pub command: Vec<String>,
    /// Wall-clock cap for one execution (None = `RUN_TESTS_TIMEOUT`).
    pub timeout: Option<Duration>,
    /// RT-002 (2026-08-11): explicit environment allowlist entries — the
    /// harness's required test-environment variables (PYTHONPATH, venv
    /// activation, sitecustomize shim, …). The host runs the command with
    /// `env_clear()` plus a fixed minimal platform allowlist (PATH and the
    /// like) PLUS these entries; the host's full environment (secrets, host
    /// paths) is never inherited.
    pub env: Vec<(String, String)>,
}

/// One workspace change observed across a `run_tests` invocation
/// (RT-003, 2026-08-11) — tests may write files, populate caches, etc.;
/// the change list is the audit trace of that side effect.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WorkspaceDeltaEntry {
    /// Worktree-relative path (forward-slash normalized on Windows).
    pub path: String,
    pub kind: WorkspaceDeltaKind,
    /// Byte length of the file at the observed edge — after for
    /// added/modified, before for deleted (AGENT-DELIVERY-FLOW
    /// 2026-08-23: the console receipt delta carries the size so the
    /// model can gauge change magnitude without a separate stat).
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceDeltaKind {
    Added,
    Modified,
    Deleted,
}

/// RT-003 (2026-08-11): worktree metadata walk exclusions — VCS metadata,
/// the host's own `.gsa` tree, and the heavyweight dependency/cache
/// directories a tool would never legitimately write (recording
/// node_modules churn would drown the trace). `__pycache__` /
/// `.pytest_cache` / `.mypy_cache` / `.ruff_cache` / `.tox` are the Python
/// test-runner's own cache surface — every run rewrites them, so without
/// the exclusion the entry cap is consumed by cache noise and the real side
/// effects get truncated away. Single-sourced here; orz-host imports the
/// walk/diff helpers (AGENT-DELIVERY-FLOW 2026-08-23).
pub const DELTA_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".gsa",
    "node_modules",
    ".venv",
    "venv",
    "target",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
];

/// RT-003: cap on the workspace-delta entries recorded per run_tests call.
pub const RUN_TESTS_DELTA_MAX_ENTRIES: usize = 200;

/// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the mechanical delivery
/// status caps its rendered change list at 20 entries + a count line.
pub const DELIVERY_DELTA_MAX_ENTRIES: usize = 20;

/// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.3): per-call receipt delta cap
/// for host tools that can mutate the worktree (run_terminal_cmd etc.).
pub const TOOL_DELTA_MAX_ENTRIES: usize = 200;

/// RT-003: worktree metadata walk (zero content reads) — the workspace-delta
/// baseline. Symlinks are not followed (a target outside the worktree is not
/// a delta; a dangling link is not a file change). Windows junctions are
/// directory reparse points — `file_type().is_symlink()` is false for them,
/// so without an explicit reparse-point skip a junction pointing at an
/// ancestor (or at a huge tree) would be walked unboundedly. Skip every
/// reparse point, symlink or junction — both are linkage, not file content,
/// in the delta semantics.
pub fn workspace_delta_walk(cwd: &Path) -> HashMap<String, (u64, u64, u32)> {
    let mut map = HashMap::new();
    let mut stack = vec![cwd.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if is_reparse_or_symlink(&path) {
                continue;
            }
            if ft.is_dir() {
                if !DELTA_EXCLUDED_DIRS.contains(&name.as_str()) {
                    stack.push(path);
                }
            } else if ft.is_file()
                && let Ok(md) = std::fs::metadata(&path)
            {
                let (mtime_secs, mtime_nanos) = match md.modified() {
                    Ok(t) => {
                        let d = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                        (d.as_secs(), d.subsec_nanos())
                    }
                    // No mtime support: (0,0) — every file diffs as
                    // "modified" after a run (conservative; same
                    // registered trade-off as the doc-index walk).
                    Err(_) => (0, 0),
                };
                let rel = path
                    .strip_prefix(cwd)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                map.insert(rel, (md.len(), mtime_secs, mtime_nanos));
            }
        }
    }
    map
}

fn is_reparse_or_symlink(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        std::fs::symlink_metadata(path)
            .map(|md| {
                const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
                md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            })
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::fs::symlink_metadata(path)
            .map(|md| md.file_type().is_symlink())
            .unwrap_or(false)
    }
}

/// RT-003: diff two delta walks into the capped change list. Deterministic
/// order (same discipline as the doc-index sorted entries).
pub fn workspace_delta_diff(
    before: &HashMap<String, (u64, u64, u32)>,
    after: &HashMap<String, (u64, u64, u32)>,
    cap: usize,
) -> (Vec<WorkspaceDeltaEntry>, bool) {
    let mut entries: Vec<WorkspaceDeltaEntry> = Vec::new();
    for (path, after_stat) in after {
        match before.get(path) {
            None => entries.push(WorkspaceDeltaEntry {
                path: path.clone(),
                kind: WorkspaceDeltaKind::Added,
                size: after_stat.0,
            }),
            Some(before_stat) if before_stat != after_stat => entries.push(WorkspaceDeltaEntry {
                path: path.clone(),
                kind: WorkspaceDeltaKind::Modified,
                size: after_stat.0,
            }),
            _ => {}
        }
    }
    for (path, before_stat) in before {
        if !after.contains_key(path) {
            entries.push(WorkspaceDeltaEntry {
                path: path.clone(),
                kind: WorkspaceDeltaKind::Deleted,
                size: before_stat.0,
            });
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let truncated = entries.len() > cap;
    entries.truncate(cap);
    (entries, truncated)
}

/// Result of running the fixed test command.
#[derive(Debug, Clone, Default)]
pub struct TestRunResult {
    pub output: String,
    pub exit_code: Option<i32>,
    /// THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1 审查处理 P2-1):
    /// true when the host hit the F-09 wall-clock bound and killed the
    /// test process tree before completion — the run_tests counterpart of
    /// `ToolResult::timed_out` (structured signal; the controller journals
    /// `tool_completed.timed_out` and renders a definitive model message,
    /// never text-prefix judgment).
    pub timed_out: bool,
    /// Decoding stage(s) that produced `output` (GAP-ENCODING-GATE,
    /// OPS-PROTOCOL §8): `utf-8` / `utf-8-sig` / `gb18030` / `utf-8-lossy:<p>%`,
    /// comma-joined when stdout/stderr used different stages.
    pub output_encoding: Option<String>,
    /// Path of the full (capped) output written by the host — readable via
    /// `read_file`; `None` when the host could not write it (or timed out).
    pub full_output_path: Option<String>,
    /// RT-003: workspace changes the test run caused (capped list; the
    /// overall change count is recoverable via `workspace_delta_truncated`
    /// only as a binary signal — the cap is the only bound).
    pub workspace_delta: Vec<WorkspaceDeltaEntry>,
    /// RT-003: true when the delta list was truncated at the host cap.
    pub workspace_delta_truncated: bool,
}

/// Trait for looking up tools by name.
pub trait ToolRegistry: Send + Sync {
    fn get(&self, name: &str) -> Option<ToolDef>;
    fn list(&self) -> Vec<ToolDef>;
}

/// The contract orz-host implements to provide Grok-backed services
/// to the agent loop controller.
///
/// Phase 1: only `journal()` is fully functional; other methods
/// have default stub implementations that return errors.
#[async_trait]
pub trait LoopHost: Send + Sync {
    /// Access the shared journal recorder.
    fn journal(&self) -> &JournalRecorder;

    /// Look up available tools.
    fn tools_registry(&self) -> &dyn ToolRegistry;

    /// Workspace trust observation state, fed to the instruction provenance
    /// gate (`trusted_project` downgrades unless `observed_trusted`).
    /// Defaults to `not_observed` (fail-closed); orz-host overrides.
    fn workspace_trust(&self) -> orz_assurance::gates::ipg::WorkspaceTrust {
        orz_assurance::gates::ipg::WorkspaceTrust::NotObserved
    }

    /// Session permission policy (IP2a) — drives the name-level tool
    /// availability projection: tools the policy refuses outright are
    /// filtered from the model-visible declarations. Defaults to
    /// `Interactive` (all tools declared; scope-level denial at permission
    /// time); orz-host maps its `PermissionPolicy` onto this.
    fn tool_policy(&self) -> ToolPolicy {
        ToolPolicy::Interactive
    }

    /// D-9 (FIX_PLAN 2026-08-06): a fixed test-runner command the model can
    /// invoke via the `run_tests` tool (Aider `--test-cmd` semantics) —
    /// stdout/stderr/exit code are fed back inside the same run. The tests
    /// themselves stay hidden (outside the workspace; the model never reads
    /// them). `None` (default) omits the tool entirely.
    fn test_runner(&self) -> Option<TestRunner> {
        None
    }

    /// Session working directory — the workspace scope the work-tool
    /// probes check (read chain / write chain / storage chain). Defaults
    /// to the process cwd; hosts must override with the session cwd (which
    /// may differ).
    fn session_cwd(&self) -> PathBuf {
        std::env::current_dir().unwrap_or_default()
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): worktree metadata
    /// snapshot used as the delivery-status baseline. `None` (default) =
    /// the host does not provide a baseline — the mechanical delivery
    /// status reports the change list as unavailable rather than
    /// fabricating one. orz-host overrides with the real walk.
    fn workspace_snapshot(&self) -> Option<HashMap<String, (u64, u64, u32)>> {
        None
    }

    /// Whether the session is attached to an interactive user who can
    /// answer `ask_user_question` and approve plan mode (work-tool probe
    /// source). Defaults to `false` (fail-closed); hosts with a live
    /// client override.
    fn interactive_user(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2 (v0.2 single probe face, ADR-0010 §3.5 v1.8):
    /// whether the host carries a usable terminal backend
    /// (`run_terminal_cmd` probe source). Fail-closed default; hosts wire
    /// their real backend.
    fn terminal_available(&self) -> bool {
        false
    }

    /// TER T1.6 (2026-09-04): 黑板 `section=processes` 的 live 事实源
    /// （读取时现算，≤1s 新鲜度）。不支持的后端默认空列表（fail-closed，
    /// 不伪造状态）。
    async fn terminal_live_processes(&self) -> Vec<LiveProcessFact> {
        Vec::new()
    }

    /// TER 全面审查 F7 (2026-09-04)：宿主是否支持 `section=processes` 的
    /// live 读取。默认 `false`（fail-closed）——渲染层区分「不支持」与
    /// 「支持但当前无进程」，模型不会把不支持误读成“确实没有进程”。
    fn terminal_live_capable(&self) -> bool {
        false
    }

    /// TER T1.12 (W-F11)：黑板 `section=env` 的机械层环境快照事实源
    /// （工具/语言/包/版本、关键输入在场；连通性由 W-F12 快速判定闭环）。
    /// 不支持的后端默认空（fail-closed，不伪造）。
    async fn env_snapshot_facts(&self) -> Vec<EnvSnapshotFact> {
        Vec::new()
    }

    /// TER 全面审查 P1-1（2026-09-04）：idle-kill 生命周期事实源——host
    /// 在每次工具执行/run 收尾时被 loop 回收（drain 语义，自上次调用以来
    /// 新发生的 idle-kill；只报 `is_backgrounded` 且 signal=idle_killed 的
    /// 已完成任务）。不支持的后端默认空（fail-closed，不伪造）。
    async fn drain_terminal_idle_kills(&self) -> Vec<TerminalIdleKillFact> {
        Vec::new()
    }

    /// 0z S2 §4.2（2026-09-12）：进程树扫除事实源（drain 语义，同 idle-kill
    /// 面）。不支持的后端默认空（fail-closed，不伪造）。
    async fn drain_process_tree_reap_facts(&self) -> Vec<ProcessTreeReapFact> {
        Vec::new()
    }

    /// 0z S2 §4.6/§5（2026-09-12）：宿主资源事实源——`reclaim_performed` /
    /// `resource_exhausted` / `host_resource_denied` 的暂存行（drain 语义）。
    /// 不支持的后端默认空（fail-closed，不伪造）。
    async fn drain_host_resource_facts(&self) -> Vec<serde_json::Value> {
        Vec::new()
    }

    /// 0z S2 §4.2 / review F-EV-3 (2026-09-13)：run 收尾扫除——本 run 泄漏
    /// 的工具子进程（breakaway/attach 失败逃出两级 Job 者）在终态事件之前
    /// 由宿主回收，其 planned/executed 行随后经 drain 落链。默认无操作
    /// （不支持的后端零行为，不伪造）。
    async fn finalize_process_trees(&self) {}

    /// P2-3（2026-09-10）：本会话浏览器 SERP 的物理事实（已用导航 / 会话
    /// 上限），供 loop 层为检索车道保留底线额度（0v 设计 §3.2.5）。`None`
    /// （默认）= 该宿主没有浏览器会话或未接入该事实面——此时 loop 不施加
    /// 底线规则（浏览器缺席的调用按普通失败回传，不由额度面兜）。
    ///
    /// 语义边界：本方法只报事实，不做车道判定（宿主没有车道身份）；返回
    /// 的是**跨车道共享**的同一计数器，宿主不得按调用者分桶。
    async fn serp_session_facts(&self) -> Option<SerpSessionFacts> {
        None
    }

    /// FUS-TOOL-PROBE P0-A-2: whether a workspace language-service backend
    /// is configured (`lsp` probe source). Fail-closed default.
    /// 0ac S3①-b M2（2026-09-14）：合法边界投递的**来源面**——drain 自上次
    /// 调用以来新完成、且尚未经任何通道报告的后台任务（见
    /// [`CompletedTaskFact`]）。drain 语义同 idle-kill 面：同一任务只返回
    /// 一次；默认空 = 无来源（且零成本），子代理/测试宿主无需实现。
    async fn drain_completed_tasks(&self) -> Vec<CompletedTaskFact> {
        Vec::new()
    }

    fn lsp_configured(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether memory is explicitly opted in and a
    /// backend is present (`memory_get` / `memory_search` probe source).
    /// Fail-closed default.
    fn memory_enabled(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether an image-generation backend is
    /// configured (`image_gen` / `image_edit` probe source). Fail-closed
    /// default.
    fn image_backend_configured(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether a video-generation backend is
    /// configured (`image_to_video` / `reference_to_video` probe source).
    /// Fail-closed default.
    fn video_backend_configured(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether an MCP / capability registry is
    /// present in session scope (`use_tool` probe source). Fail-closed
    /// default.
    fn mcp_registry_available(&self) -> bool {
        false
    }

    /// D-9: execute the host's fixed test command. Only called when
    /// `test_runner()` returned `Some`. The host runs the command in the
    /// session cwd with the injected argv — never a model-supplied command.
    async fn run_tests(&self) -> Result<TestRunResult, ToolError> {
        Err(ToolError::NotFound("no test runner configured".into()))
    }

    /// Execute a tool call.
    async fn call_tool(
        &self,
        _name: &str,
        _args: Value,
        _call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::NotFound("not implemented".into()))
    }

    /// Execute a tool call under an explicit per-call wall-clock bound
    /// (P0-C S4, 2026-08-16: script step deadlines). `None` = host default
    /// (`call_tool` semantics). Hosts that honor the override MUST bound
    /// the whole call with the shorter of the override and their configured
    /// budget, and on expiry kill the tool's process tree and fail with
    /// `ToolError::Timeout` — a script layer timeout alone must never
    /// substitute for host-side process-tree reclamation. The default
    /// implementation ignores the override: hosts that do NOT override it
    /// do not provide per-step host deadlines (script steps then rely on
    /// step-boundary checks only, and the script runner enforces the total
    /// wall clock ex-post after each step). Production `OrzHost` honors
    /// the override.
    async fn call_tool_with_timeout(
        &self,
        name: &str,
        args: Value,
        call_id: &str,
        _timeout: Option<Duration>,
    ) -> Result<ToolResult, ToolError> {
        self.call_tool(name, args, call_id).await
    }

    /// Request user permission for a risky action.
    ///
    /// Default is fail-closed `Deny`. Hosts must implement their own
    /// permission bridge (e.g. `PermissionBridge` for ACP) to allow tools —
    /// an `AllowOnce` default would silently auto-allow `bash` whenever a
    /// host forgets to wire the bridge (review P1-1, 2026-08-04).
    async fn request_permission(
        &self,
        _risk: RiskClass,
        _tool: &str,
        _args: &Value,
    ) -> Result<PermitDecision, PermitError> {
        Ok(PermitDecision::Deny)
    }

    /// 0bt④（2026-09-26）：同 [`request_permission`](Self::request_permission)，
    /// 附带判定来源（[`PermitSource`]）供 journal 观测面落账。默认实现包装
    /// 旧方法、来源 `None`（宿主未分类 ⇒ 事件保持旧形状）。
    async fn request_permission_with_source(
        &self,
        risk: RiskClass,
        tool: &str,
        args: &Value,
    ) -> Result<(PermitDecision, Option<PermitSource>), PermitError> {
        Ok((self.request_permission(risk, tool, args).await?, None))
    }

    /// Live text-delta hook (streaming slice): the controller forwards each
    /// model text chunk in order, as the gateway produces it, before the
    /// round's `model_output` is recorded. Live-only — never journaled
    /// (Python `text_delta` precedent). Default no-op; hosts with a live
    /// client forward fire-and-forget. Synchronous because every delivery
    /// path is fire-and-forget (a dropped client must not stall the turn).
    fn on_text_delta(&self, _text: &str) {}

    /// Persist a turn record.
    async fn persist_turn(&self, _turn: &TurnRecord) -> Result<(), PersistError> {
        Ok(())
    }

    /// Trigger context compaction.
    async fn trigger_compaction(&self) -> Result<(), CompactionError> {
        Err(CompactionError::NotSupported)
    }

    /// Run lifecycle hooks.
    async fn run_hooks(&self, _event: &HookEvent) -> Vec<HookResult> {
        Vec::new()
    }

    /// Retrieve stored credentials.
    fn auth_credentials(&self) -> Result<Credentials, AuthError> {
        Err(AuthError::NotFound("no credentials configured".into()))
    }

    /// List MCP-provided tools.
    fn mcp_tools(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-host-delta-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §5/§6.3 审查处理 F1): the
    /// walk-exclusion rule is LOCKED by a unit test — `.gsa`/VCS/dependency
    /// cache trees never appear in the delta baseline, so a delivery status
    /// or run-tests delta cannot be drowned by host noise (the design
    /// explicitly requires the filter rule to be test-locked).
    #[test]
    fn workspace_delta_walk_excludes_host_noise_dirs() {
        let root = test_dir();
        let mut expected = std::collections::HashSet::new();
        // Real worktree files must be recorded.
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();
        expected.insert("src/main.rs".to_string());
        std::fs::write(root.join("README.md"), "readme").unwrap();
        expected.insert("README.md".to_string());
        // Host noise that must NEVER appear in the walk.
        for name in DELTA_EXCLUDED_DIRS {
            let sub = root.join(name);
            std::fs::create_dir_all(&sub).unwrap();
            std::fs::write(sub.join("noise.txt"), "noise").unwrap();
        }
        let map = workspace_delta_walk(&root);
        let paths: std::collections::HashSet<String> = map.keys().cloned().collect();
        assert_eq!(
            paths, expected,
            "excluded dirs must not enter the walk: {paths:?}"
        );
        // Sanity: the exclusion list itself must cover the host's own tree.
        assert!(DELTA_EXCLUDED_DIRS.contains(&".gsa"));
        assert!(DELTA_EXCLUDED_DIRS.contains(&".git"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 审查处理 O5): the walk is a
    /// zero-content metadata snapshot — added/modified edges (with the
    /// observed size) are detectable from the snapshot alone; no file
    /// content is ever read.
    #[test]
    fn workspace_delta_walk_skips_unreadable_entries_and_diff_is_metadata_only() {
        let root = test_dir();
        std::fs::write(root.join("a.txt"), "v1").unwrap();
        let before = workspace_delta_walk(&root);
        std::fs::write(root.join("a.txt"), "version two - longer").unwrap();
        std::fs::write(root.join("b.txt"), "new").unwrap();
        let after = workspace_delta_walk(&root);
        let (entries, truncated) = workspace_delta_diff(&before, &after, usize::MAX);
        assert!(!truncated);
        let kinds: Vec<(String, WorkspaceDeltaKind)> =
            entries.into_iter().map(|e| (e.path, e.kind)).collect();
        assert!(kinds.contains(&("a.txt".to_string(), WorkspaceDeltaKind::Modified)));
        assert!(kinds.contains(&("b.txt".to_string(), WorkspaceDeltaKind::Added)));
        assert_eq!(kinds.len(), 2);
        // Size is the observed edge (after for modified).
        let size = workspace_delta_diff(&before, &after, usize::MAX)
            .0
            .into_iter()
            .find(|e| e.path == "a.txt")
            .expect("a.txt entry")
            .size;
        assert!(size > 2, "modified size reflects the after edge: {size}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
