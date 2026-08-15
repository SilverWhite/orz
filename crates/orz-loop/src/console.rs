//! Classical Execution Assistant core — P0-C orz 内嵌集成 (slice 1).
//!
//! 操作台本体（v0.5 操作台模型的生产内嵌核心）：
//! - `ServiceRegistry`：动作名 → 输入契约 → 目标工具 → 响应契约的确定性路由表；
//! - `issue_action`：注册表路由 → 契约校验 → 目标解析 → 执行委托 → 响应验证，
//!   任一步失败返回 fail-closed 错误信封（step/code/message/upstream/trace_id）；
//! - `Trace` / `TraceStore`：有界、只读、可审计的执行日志（模型排障入口）。
//!
//! 切片边界（2026-08-15）：本切片只实现操作台核心与数据面，不改变模型可见
//! 工具投影、不接入轮末发放、不触碰 ACAF/权限门；执行委托经 `ActionExecutor`
//! 抽象，生产实现由 controller 复用 `run_host_tool`（权限 + ACAF + 事件链），
//! 禁止直接绕过既有门直接调 `host.call_tool`。
//!
//! 2026-08-15 全面检查修复（用户裁决）：
//! - 执行器返回 `ExecuteError`，区分执行失败（`step=execute`）与策略拒绝
//!   （`step=policy` / `code=policy_denied`；权限/ACAF/taint/模式门经适配层归一化）；
//! - 响应契约强制：`ActionSpec.response_schema` 必填，任何输出必须过机械验证
//!   （审计的一部分），注册时校验并缓存 JSON Schema；
//! - 成功退出码契约：`exit_code` 必须为 `Some(0)`；`None`/非零按执行失败
//!   （生产 host 成功输出已归一化为 `Some(0)`，拒绝由适配层归一化为 PolicyDenied）；
//! - `TraceStore` 提交语义：`new_trace` 返回工作副本，发放收口后 `commit`
//!   使事件在 store 可见；trace 满 200 后失败事件滚动保底（替换最旧）；
//! - 注册板块投影为最小参数提示（type/required/属性枚举/默认值），不复制完整 schema。

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::blackboard::ActionOrder;
use crate::host::ToolResult;

/// 单 trace 事件上限（POC 同构：200 条）。
pub const TRACE_MAX_EVENTS: usize = 200;
/// trace 存储保留的最近请求数（POC 同构：50）。
pub const TRACE_STORE_MAX_TRACES: usize = 50;

/// 错误信封 step 枚举（POC fail-closed 返回契约）。
pub const STEP_PROTOCOL: &str = "protocol";
pub const STEP_INTENT: &str = "intent";
pub const STEP_REGISTRY: &str = "registry";
pub const STEP_CONTRACT: &str = "contract";
pub const STEP_TARGET: &str = "target";
pub const STEP_EXECUTE: &str = "execute";
pub const STEP_VERIFY: &str = "verify";
pub const STEP_POLICY: &str = "policy";

/// 机器可读错误码（HA 同构，固定小写）。
pub const CODE_UNKNOWN_SERVICE: &str = "unknown_service";
pub const CODE_INVALID_ARGUMENTS: &str = "invalid_arguments";
pub const CODE_INVALID_RESPONSE: &str = "invalid_response";
pub const CODE_EXECUTION_FAILED: &str = "execution_failed";
pub const CODE_OUT_OF_SCOPE: &str = "out_of_scope";
pub const CODE_POLICY_DENIED: &str = "policy_denied";
pub const CODE_INTERNAL_ERROR: &str = "internal_error";
/// 订单过期/重放（round/plan_epoch 防重放与过期校验失败）——请求类错误，
/// 走 `step=protocol`（模型改订单后重写）。
pub const CODE_ORDER_STALE: &str = "order_stale";

/// 基础动作集响应契约：生产 host 工具当前统一返回文本输出
/// （`ToolResult.structured` 接缝仅 web_search 使用；console 收口为
/// `{"output": ...}` 信封并机械验证）。
fn text_output_response_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "output": {"type": "string"},
        },
        "required": ["output"],
    })
}

/// 生产注册动作集（S2 起接线；S3 由 Profile/Bundle 分区加载扩展）。
/// 目标工具名必须与 orz-host 注册表一致；输入契约镜像真实参数
/// （`target_file`/`target_directory`/`pattern`/`file_path` 等）。
pub fn default_service_registry() -> ServiceRegistry {
    let mut registry = ServiceRegistry::new();
    for spec in [
        ActionSpec {
            name: "workspace.read_file".to_string(),
            description: "读取工作区文件（行号锚点、可 offset/limit 分段续读）。".to_string(),
            target_tool: "read_file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target_file": {
                        "type": "string",
                        "description": "工作区相对路径或绝对路径（须在工作区内）。",
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "起始行号（文件过大时分段读取）。",
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "读取行数上限。",
                    },
                },
                "required": ["target_file"],
                "additionalProperties": false,
            }),
            response_schema: text_output_response_schema(),
        },
        ActionSpec {
            name: "workspace.list_dir".to_string(),
            description: "列出目录内容（工作区相对路径或绝对路径）。".to_string(),
            target_tool: "list_dir".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target_directory": {
                        "type": "string",
                        "description": "要列出的目录路径（相对工作区或绝对）。",
                    },
                },
                "required": ["target_directory"],
                "additionalProperties": false,
            }),
            response_schema: text_output_response_schema(),
        },
        ActionSpec {
            name: "workspace.grep".to_string(),
            description: "正则搜索文件内容（ripgrep；可限定路径/glob/类型/上下文）。".to_string(),
            target_tool: "grep".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "要搜索的正则表达式（rg --regexp）。",
                    },
                    "path": {
                        "type": "string",
                        "description": "搜索的文件或目录（默认工作区）。",
                    },
                    "glob": {
                        "type": "string",
                        "description": "文件过滤 glob（如 *.rs）。",
                    },
                    "type": {
                        "type": "string",
                        "description": "文件类型（rg --type，如 rust/py/js）。",
                    },
                    "head_limit": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "输出条数上限。",
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "大小写不敏感（rg -i）。",
                    },
                },
                "required": ["pattern"],
                "additionalProperties": false,
            }),
            response_schema: text_output_response_schema(),
        },
        ActionSpec {
            name: "workspace.search_replace".to_string(),
            description:
                "编辑执行器：唯一精确替换（old_string 必须唯一匹配，空 old_string 建新文件）。"
                    .to_string(),
            target_tool: "search_replace".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "目标文件（工作区相对路径）。",
                    },
                    "old_string": {
                        "type": "string",
                        "description": "要替换的原文（须唯一匹配；空=新建文件）。",
                    },
                    "new_string": {
                        "type": "string",
                        "description": "替换后的新文本。",
                    },
                },
                "required": ["file_path", "old_string", "new_string"],
                "additionalProperties": false,
            }),
            response_schema: text_output_response_schema(),
        },
        ActionSpec {
            name: "workspace.run_tests".to_string(),
            description: "运行会话固定的测试命令（模型不提供命令；host-owned）。".to_string(),
            target_tool: "run_tests".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            }),
            response_schema: text_output_response_schema(),
        },
        ActionSpec {
            name: "workspace.index".to_string(),
            description: "索引并检索工作区项目文档（query 空=全量列表）。".to_string(),
            target_tool: "project_doc_index".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "关键词（匹配路径/标题/标题行；空=列出全部）。",
                    },
                    "include_content": {
                        "type": "boolean",
                        "description": "返回截断全文。",
                    },
                    "max_results": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "结果上限（默认 10）。",
                    },
                },
                "additionalProperties": false,
            }),
            response_schema: text_output_response_schema(),
        },
    ] {
        // 基础动作集是静态契约——注册失败是编程错误（非法 schema 会在这里
        // 拒绝，符合「非法 schema 注册即拒绝」的登记语义）。
        registry
            .register(spec)
            .expect("static console registry valid");
    }
    registry
}

/// 一条有界执行日志事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceEvent {
    pub seq: usize,
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<Value>,
}

/// 一次请求的执行日志（有界、模型可经 `assistant.trace` 取回）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trace {
    pub trace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default)]
    pub events: Vec<TraceEvent>,
}

impl Trace {
    /// Append one bounded event; ordinary events past `TRACE_MAX_EVENTS` are
    /// dropped (POC parity). Failure events evict the oldest event instead,
    /// so the execute-failure tail contract always has the failure visible.
    pub fn add(
        &mut self,
        step: impl Into<String>,
        action: Option<&str>,
        ok: bool,
        code: Option<&str>,
        message: Option<String>,
        upstream: Option<Value>,
    ) {
        if self.events.len() >= TRACE_MAX_EVENTS {
            if !ok {
                self.events.remove(0);
            } else {
                return;
            }
        }
        let seq = self.events.last().map_or(1, |e| e.seq + 1);
        self.events.push(TraceEvent {
            seq,
            step: step.into(),
            action: action.map(str::to_string),
            ok,
            code: code.map(str::to_string),
            message,
            upstream,
        });
    }

    /// The most recent `n` events plus whether events were truncated.
    pub fn tail(&self, n: usize) -> (Vec<TraceEvent>, bool) {
        let start = self.events.len().saturating_sub(n);
        (self.events[start..].to_vec(), self.events.len() > n)
    }
}

/// Bounded in-memory trace store — keeps the most recent 50 traces.
#[derive(Debug, Clone, Default)]
pub struct TraceStore {
    traces: VecDeque<Trace>,
    seq: u64,
}

impl TraceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate the next trace id and return a detached working trace. The
    /// store keeps a placeholder until `commit` stores the completed events —
    /// callers must `commit` after issuing so `assistant.trace` can see them.
    pub fn new_trace(&mut self, request_id: Option<String>) -> Trace {
        self.seq += 1;
        let seq = self.seq;
        let trace = Trace {
            trace_id: format!("t{seq:06}"),
            request_id,
            events: Vec::new(),
        };
        if self.traces.len() >= TRACE_STORE_MAX_TRACES {
            self.traces.pop_front();
        }
        self.traces.push_back(trace.clone());
        trace
    }

    /// Commit a completed trace by id, replacing the placeholder so its
    /// events become visible to store lookups. No-op if the placeholder was
    /// already evicted by the bounded store.
    pub fn commit(&mut self, trace: &Trace) {
        if let Some(existing) = self
            .traces
            .iter_mut()
            .find(|t| t.trace_id == trace.trace_id)
        {
            *existing = trace.clone();
        }
    }

    pub fn get(&self, trace_id: &str) -> Option<&Trace> {
        self.traces.iter().find(|t| t.trace_id == trace_id)
    }
}

/// 动作契约（版本化 API）：名称 + 输入 schema + 目标工具 + 响应 schema。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionSpec {
    pub name: String,
    pub description: String,
    /// 解析后的真实执行目标（既有 host 工具名；操作台不建第二执行器）。
    pub target_tool: String,
    pub input_schema: Value,
    /// 响应契约（必填）：对 `ToolResult.structured`（无则 `{"output": ...}`）
    /// 做机械验证——任何输出必须过机械验证（审计的一部分），注册时校验并缓存。
    pub response_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    Duplicate(String),
    InvalidSchema {
        name: String,
        kind: &'static str,
        detail: String,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Duplicate(name) => write!(f, "duplicate service: {name}"),
            RegistryError::InvalidSchema { name, kind, detail } => {
                write!(f, "invalid {kind} schema for service {name}: {detail}")
            }
        }
    }
}

/// Compiled JSON Schema validator, cached at registration (runtime近零).
type CachedValidator = Arc<jsonschema::Validator>;

/// HA 式服务注册表：动作名 → 契约 + 目标工具，确定性、无模型参与。
#[derive(Debug, Clone, Default)]
pub struct ServiceRegistry {
    actions: BTreeMap<String, ActionSpec>,
    validators: BTreeMap<String, (CachedValidator, CachedValidator)>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, spec: ActionSpec) -> Result<(), RegistryError> {
        if self.actions.contains_key(&spec.name) {
            return Err(RegistryError::Duplicate(spec.name));
        }
        let input = jsonschema::validator_for(&spec.input_schema).map_err(|e| {
            RegistryError::InvalidSchema {
                name: spec.name.clone(),
                kind: "input",
                detail: e.to_string(),
            }
        })?;
        let response = jsonschema::validator_for(&spec.response_schema).map_err(|e| {
            RegistryError::InvalidSchema {
                name: spec.name.clone(),
                kind: "response",
                detail: e.to_string(),
            }
        })?;
        self.validators
            .insert(spec.name.clone(), (Arc::new(input), Arc::new(response)));
        self.actions.insert(spec.name.clone(), spec);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&ActionSpec> {
        self.actions.get(name)
    }

    /// Stable sorted names — the button panel order.
    pub fn names(&self) -> Vec<String> {
        self.actions.keys().cloned().collect()
    }

    /// The registration-board projection: 动作名 + 最小参数提示。
    pub fn registrations(&self) -> Vec<crate::blackboard::ActionRegistration> {
        self.actions
            .values()
            .map(|spec| crate::blackboard::ActionRegistration {
                name: spec.name.clone(),
                description: spec.description.clone(),
                parameters: minimal_parameter_hints(&spec.input_schema),
            })
            .collect()
    }
}

/// 注册板块投影：最小参数提示（type/required/属性 type/description/enum/
/// const/default），不复制完整 schema——防上下文膨胀（v0.5 用户确认：常驻按需读）。
fn minimal_parameter_hints(schema: &Value) -> Value {
    const HINT_KEYS: [&str; 5] = ["type", "description", "enum", "const", "default"];
    let mut out = serde_json::Map::new();
    if let Some(t) = schema.get("type") {
        out.insert("type".to_string(), t.clone());
    }
    if let Some(required) = schema.get("required") {
        out.insert("required".to_string(), required.clone());
    }
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        let trimmed = props
            .iter()
            .map(|(name, prop)| {
                let mut hint = serde_json::Map::new();
                for key in HINT_KEYS {
                    if let Some(v) = prop.get(key) {
                        hint.insert(key.to_string(), v.clone());
                    }
                }
                (name.clone(), Value::Object(hint))
            })
            .collect();
        out.insert("properties".to_string(), Value::Object(trimmed));
    }
    Value::Object(out)
}

/// 执行器错误：区分「执行失败」与「策略拒绝」（权限/ACAF/taint/模式门）。
///
/// 2026-08-15 用户裁决：适配层返回丰富结果，`issue_action` 据此把策略拒绝
/// 映射为 `step=policy` / `code=policy_denied`，不落入 execute 失败。
#[derive(Debug, Clone)]
pub enum ExecuteError {
    ExecutionFailed {
        message: String,
        detail: Option<Value>,
    },
    PolicyDenied {
        message: String,
        detail: Option<Value>,
    },
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecuteError::ExecutionFailed { message, .. } => {
                write!(f, "execution failed: {message}")
            }
            ExecuteError::PolicyDenied { message, .. } => {
                write!(f, "policy denied: {message}")
            }
        }
    }
}

/// 执行委托抽象：生产实现由 controller 复用 `run_host_tool`
/// （权限桥 + ACAF 票据 + journal/事件链）；测试用 fake 实现。
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    async fn execute(
        &self,
        target_tool: &str,
        arguments: &Value,
        call_id: &str,
    ) -> Result<ToolResult, ExecuteError>;
}

/// 机械构造的失败点（fail-closed 返回契约；无模型参与）。
#[derive(Debug, Clone)]
pub struct ConsoleError {
    pub step: &'static str,
    pub code: &'static str,
    pub message: String,
    pub upstream: Option<Value>,
}

impl std::fmt::Display for ConsoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] {}", self.step, self.code, self.message)
    }
}

/// 成功信封：`{ok:true, response, trace_id}`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessEnvelope {
    pub ok: bool,
    pub response: Value,
    pub trace_id: String,
}

/// 失败信封：`{ok:false, error:{step,code,message,upstream,trace_id[,trace]}}`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorEnvelope {
    pub ok: bool,
    pub error: ErrorPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorPayload {
    pub step: String,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<Value>,
    pub trace_id: String,
    /// 仅 `step=execute` 失败携带：有界日志尾部（执行失败特殊反馈）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<TraceEvent>>,
}

pub fn success_envelope(response: Value, trace_id: String) -> SuccessEnvelope {
    SuccessEnvelope {
        ok: true,
        response,
        trace_id,
    }
}

pub fn error_envelope(err: &ConsoleError, trace: &Trace, tail: usize) -> ErrorEnvelope {
    let (events, _) = trace.tail(tail);
    ErrorEnvelope {
        ok: false,
        error: ErrorPayload {
            step: err.step.to_string(),
            code: err.code.to_string(),
            message: err.message.clone(),
            upstream: err.upstream.clone(),
            trace_id: trace.trace_id.clone(),
            trace: (err.step == STEP_EXECUTE).then_some(events),
        },
    }
}

/// 失败收口：先追加失败事件，再构造信封（POC `run_request` 的失败收口同构）。
/// 调用方不再手工拼接 trace 事件与信封。
pub fn failure_envelope(
    trace: &mut Trace,
    action: Option<&str>,
    err: &ConsoleError,
    tail: usize,
) -> ErrorEnvelope {
    trace.add(
        err.step,
        action,
        false,
        Some(err.code),
        Some(err.message.clone()),
        err.upstream.clone(),
    );
    error_envelope(err, trace, tail)
}

/// 发放一个动作订单：注册表路由 → 契约校验 → 目标解析 → 执行 → 响应验证。
///
/// 每一步成功后追加 trace 事件；失败返回 `ConsoleError`（调用方用
/// `failure_envelope` 收口——追加失败事件并构造信封）。
pub async fn issue_action<E: ActionExecutor + ?Sized>(
    registry: &ServiceRegistry,
    executor: &E,
    order: &ActionOrder,
    trace: &mut Trace,
    call_id: &str,
) -> Result<Value, ConsoleError> {
    let spec = registry.get(&order.action).ok_or_else(|| ConsoleError {
        step: STEP_REGISTRY,
        code: CODE_UNKNOWN_SERVICE,
        message: format!("unknown service: {}", order.action),
        upstream: Some(json!({ "action": order.action })),
    })?;
    trace.add(STEP_REGISTRY, Some(&order.action), true, None, None, None);

    let (input_validator, response_validator) = registry
        .validators
        .get(&order.action)
        .expect("registry invariant: every registered action has cached validators");
    validate_with(input_validator, &order.arguments).map_err(|msg| ConsoleError {
        step: STEP_CONTRACT,
        code: CODE_INVALID_ARGUMENTS,
        message: format!("{}: {msg}", spec.name),
        upstream: Some(json!({
            "action": spec.name,
            "arguments": order.arguments,
        })),
    })?;
    trace.add(STEP_CONTRACT, Some(&order.action), true, None, None, None);

    trace.add(
        STEP_TARGET,
        Some(&order.action),
        true,
        None,
        Some(format!("resolved target tool: {}", spec.target_tool)),
        Some(json!({ "target_tool": spec.target_tool })),
    );

    let result = executor
        .execute(&spec.target_tool, &order.arguments, call_id)
        .await
        .map_err(|err| match err {
            ExecuteError::ExecutionFailed { message, detail } => ConsoleError {
                step: STEP_EXECUTE,
                code: CODE_EXECUTION_FAILED,
                message,
                upstream: Some(execution_upstream(&spec.name, &spec.target_tool, detail)),
            },
            ExecuteError::PolicyDenied { message, detail } => ConsoleError {
                step: STEP_POLICY,
                code: CODE_POLICY_DENIED,
                message,
                upstream: Some(execution_upstream(&spec.name, &spec.target_tool, detail)),
            },
        })?;
    if result.exit_code != Some(0) {
        let exit_detail = match result.exit_code {
            None => "no exit code (actions require an explicit success exit code)".to_string(),
            Some(code) => format!("non-zero exit code {code}"),
        };
        return Err(ConsoleError {
            step: STEP_EXECUTE,
            code: CODE_EXECUTION_FAILED,
            message: format!("target tool {exit_detail}"),
            upstream: Some(json!({
                "target_tool": spec.target_tool,
                "exit_code": result.exit_code,
                "output": truncate(&result.output, 4000),
            })),
        });
    }
    trace.add(STEP_EXECUTE, Some(&order.action), true, None, None, None);

    let response = match result.structured {
        Some(value) => value,
        None => json!({ "output": result.output }),
    };
    // 响应契约强制（2026-08-15 用户裁决）：任何输出必须过机械验证，
    // 不仅是规整性与安全，也是审计的一部分。
    validate_with(response_validator, &response).map_err(|msg| ConsoleError {
        step: STEP_VERIFY,
        code: CODE_INVALID_RESPONSE,
        message: format!("{}: response failed verification: {msg}", spec.name),
        upstream: Some(json!({
            "action": spec.name,
            "target_tool": spec.target_tool,
        })),
    })?;
    trace.add(STEP_VERIFY, Some(&order.action), true, None, None, None);

    Ok(response)
}

fn execution_upstream(action: &str, target_tool: &str, detail: Option<Value>) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("action".to_string(), Value::String(action.to_string()));
    map.insert(
        "target_tool".to_string(),
        Value::String(target_tool.to_string()),
    );
    if let Some(detail) = detail {
        map.insert("detail".to_string(), detail);
    }
    Value::Object(map)
}

fn validate_with(validator: &jsonschema::Validator, instance: &Value) -> Result<(), String> {
    validator.validate(instance).map_err(|e| e.to_string())
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let head: String = s.chars().take(max_chars).collect();
        format!("{head}…[truncated]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::ActionOrder;

    fn spec(name: &str, target: &str) -> ActionSpec {
        ActionSpec {
            name: name.to_string(),
            description: format!("{name} test action"),
            target_tool: target.to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                },
                "required": ["path"],
                "additionalProperties": false,
            }),
            response_schema: json!({
                "type": "object",
                "properties": {
                    "output": {"type": "string"},
                },
                "required": ["output"],
            }),
        }
    }

    fn order(action: &str, arguments: Value) -> ActionOrder {
        ActionOrder {
            order_id: "ORD-1".to_string(),
            action: action.to_string(),
            arguments,
            round: 1,
            plan_epoch: 1,
            run_id: "RUN-1".to_string(),
        }
    }

    struct FakeExecutor {
        result: Result<ToolResult, ExecuteError>,
        seen: std::sync::Arc<std::sync::Mutex<Vec<(String, Value, String)>>>,
    }

    impl FakeExecutor {
        fn ok() -> (
            Self,
            std::sync::Arc<std::sync::Mutex<Vec<(String, Value, String)>>>,
        ) {
            let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            (
                Self {
                    result: Ok(ToolResult {
                        output: "hello".to_string(),
                        exit_code: Some(0),
                        output_encoding: None,
                        structured: Some(json!({"output": "hello"})),
                    }),
                    seen: seen.clone(),
                },
                seen,
            )
        }

        fn fail() -> Self {
            Self {
                result: Err(ExecuteError::ExecutionFailed {
                    message: "boom".to_string(),
                    detail: None,
                }),
                seen: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl ActionExecutor for FakeExecutor {
        async fn execute(
            &self,
            target_tool: &str,
            arguments: &Value,
            call_id: &str,
        ) -> Result<ToolResult, ExecuteError> {
            self.seen.lock().unwrap().push((
                target_tool.to_string(),
                arguments.clone(),
                call_id.to_string(),
            ));
            self.result.clone()
        }
    }

    #[test]
    fn registry_duplicate_rejected_and_names_sorted() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        assert_eq!(
            registry.register(spec("workspace.read_file", "read_file")),
            Err(RegistryError::Duplicate("workspace.read_file".to_string()))
        );
        registry
            .register(spec("workspace.list_dir", "list_dir"))
            .unwrap();
        assert_eq!(
            registry.names(),
            vec!["workspace.list_dir", "workspace.read_file"]
        );
    }

    #[test]
    fn default_registry_registers_base_action_set_with_valid_contracts() {
        let registry = default_service_registry();
        let names = registry.names();
        assert_eq!(
            names,
            vec![
                "workspace.grep",
                "workspace.index",
                "workspace.list_dir",
                "workspace.read_file",
                "workspace.run_tests",
                "workspace.search_replace",
            ]
        );
        for name in &names {
            let spec = registry.get(name).expect("registered action");
            // 响应契约必填且注册时已通过 JSON Schema 编译（校验器缓存）。
            assert!(registry.validators.contains_key(name));
            assert!(!spec.response_schema.is_null());
        }
    }

    #[test]
    fn registration_board_projects_names_and_contracts() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let board = registry.registrations();
        assert_eq!(board.len(), 1);
        assert_eq!(board[0].name, "workspace.read_file");
        assert!(board[0].parameters.get("required").is_some());
    }

    #[tokio::test]
    async fn issue_action_routes_contracts_executes_and_verifies() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let (executor, seen) = FakeExecutor::ok();
        let mut trace = Trace {
            trace_id: "t000001".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let response = issue_action(
            &registry,
            &executor,
            &order("workspace.read_file", json!({"path": "a.txt"})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap();
        assert_eq!(response, json!({"output": "hello"}));
        let calls = seen.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "read_file");
        assert_eq!(calls[0].2, "call-1");
        drop(calls);
        let steps: Vec<&str> = trace.events.iter().map(|e| e.step.as_str()).collect();
        assert_eq!(
            steps,
            vec!["registry", "contract", "target", "execute", "verify"]
        );
        assert!(trace.events.iter().all(|e| e.ok));
    }

    #[tokio::test]
    async fn unknown_service_fails_at_registry() {
        let registry = ServiceRegistry::new();
        let (executor, _) = FakeExecutor::ok();
        let mut trace = Trace {
            trace_id: "t000002".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            &order("nope.nope", json!({})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_REGISTRY);
        assert_eq!(err.code, CODE_UNKNOWN_SERVICE);
        assert!(trace.events.is_empty());
    }

    #[tokio::test]
    async fn contract_violation_fails_before_execution() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let (executor, seen) = FakeExecutor::ok();
        let mut trace = Trace {
            trace_id: "t000003".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            &order("workspace.read_file", json!({})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_CONTRACT);
        assert_eq!(err.code, CODE_INVALID_ARGUMENTS);
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events[0].step, STEP_REGISTRY);
    }

    #[tokio::test]
    async fn executor_failure_returns_execute_envelope_with_trace_tail() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let executor = FakeExecutor::fail();
        let mut trace = Trace {
            trace_id: "t000004".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            &order("workspace.read_file", json!({"path": "a.txt"})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_EXECUTE);
        assert_eq!(err.code, CODE_EXECUTION_FAILED);
        let envelope = failure_envelope(&mut trace, Some("workspace.read_file"), &err, 10);
        assert!(!envelope.ok);
        assert_eq!(envelope.error.trace_id, "t000004");
        let tail = envelope.error.trace.unwrap();
        assert!(!tail.is_empty());
        assert_eq!(tail.last().unwrap().step, STEP_EXECUTE);
    }

    #[tokio::test]
    async fn response_schema_violation_fails_at_verify() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let executor = FakeExecutor {
            result: Ok(ToolResult {
                output: "hello".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: Some(json!({"unexpected": true})),
            }),
            seen: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        };
        let mut trace = Trace {
            trace_id: "t000005".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            &order("workspace.read_file", json!({"path": "a.txt"})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_VERIFY);
        assert_eq!(err.code, CODE_INVALID_RESPONSE);
    }

    #[tokio::test]
    async fn non_zero_exit_is_execute_failure_with_output_bound() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let executor = FakeExecutor {
            result: Ok(ToolResult {
                output: "x".repeat(10_000),
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
            }),
            seen: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        };
        let mut trace = Trace {
            trace_id: "t000006".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            &order("workspace.read_file", json!({"path": "a.txt"})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_EXECUTE);
        let upstream = err.upstream.as_ref().unwrap();
        let output = upstream.get("output").and_then(Value::as_str).unwrap();
        assert!(output.contains("[truncated]"));
    }

    #[tokio::test]
    async fn policy_denial_maps_to_policy_step() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let executor = FakeExecutor {
            result: Err(ExecuteError::PolicyDenied {
                message: "denied by policy".to_string(),
                detail: Some(json!({"policy": "readonly"})),
            }),
            seen: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        };
        let mut trace = Trace {
            trace_id: "t000007".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            &order("workspace.read_file", json!({"path": "a.txt"})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_POLICY);
        assert_eq!(err.code, CODE_POLICY_DENIED);
        let upstream = err.upstream.as_ref().unwrap();
        assert_eq!(upstream["detail"]["policy"], "readonly");
        let envelope = failure_envelope(&mut trace, Some("workspace.read_file"), &err, 10);
        assert_eq!(envelope.error.step, STEP_POLICY);
        assert_eq!(envelope.error.code, CODE_POLICY_DENIED);
        assert!(
            envelope.error.trace.is_none(),
            "policy denial must not carry the execute trace tail"
        );
        assert_eq!(trace.events.last().unwrap().step, STEP_POLICY);
        assert!(!trace.events.last().unwrap().ok);
    }

    #[tokio::test]
    async fn missing_exit_code_is_execute_failure() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(spec("workspace.read_file", "read_file"))
            .unwrap();
        let executor = FakeExecutor {
            result: Ok(ToolResult {
                output: "ok".to_string(),
                exit_code: None,
                output_encoding: None,
                structured: None,
            }),
            seen: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        };
        let mut trace = Trace {
            trace_id: "t000008".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            &order("workspace.read_file", json!({"path": "a.txt"})),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_EXECUTE);
        assert!(err.message.contains("no exit code"));
    }

    #[test]
    fn invalid_schema_rejected_at_registration() {
        let mut registry = ServiceRegistry::new();
        let bad_input = ActionSpec {
            name: "bad.input".to_string(),
            description: "bad input schema".to_string(),
            target_tool: "read_file".to_string(),
            input_schema: json!({"type": 42}),
            response_schema: json!({"type": "object"}),
        };
        assert!(matches!(
            registry.register(bad_input),
            Err(RegistryError::InvalidSchema { kind: "input", .. })
        ));
        let bad_response = ActionSpec {
            name: "bad.response".to_string(),
            description: "bad response schema".to_string(),
            target_tool: "read_file".to_string(),
            input_schema: json!({"type": "object"}),
            response_schema: json!([]),
        };
        assert!(matches!(
            registry.register(bad_response),
            Err(RegistryError::InvalidSchema {
                kind: "response",
                ..
            })
        ));
    }

    #[test]
    fn registration_board_projects_minimal_hints() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ActionSpec {
                name: "workspace.read_file".to_string(),
                description: "read a file".to_string(),
                target_tool: "read_file".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "file path",
                            "format": "path",
                            "minLength": 1,
                            "enum": ["a.txt", "b.txt"],
                        }
                    },
                    "additionalProperties": false,
                }),
                response_schema: json!({"type": "object"}),
            })
            .unwrap();
        let board = registry.registrations();
        assert_eq!(board.len(), 1);
        let params = &board[0].parameters;
        assert_eq!(params["type"], "object");
        assert_eq!(params["required"], json!(["path"]));
        let path = &params["properties"]["path"];
        assert_eq!(path["type"], "string");
        assert_eq!(path["description"], "file path");
        assert_eq!(path["enum"], json!(["a.txt", "b.txt"]));
        assert!(
            path.get("format").is_none(),
            "format must not be copied into minimal hints"
        );
        assert!(path.get("minLength").is_none());
        assert!(params.get("additionalProperties").is_none());
    }

    #[test]
    fn trace_retains_failure_event_at_cap() {
        let mut store = TraceStore::new();
        let mut trace = store.new_trace(None);
        for i in 0..TRACE_MAX_EVENTS {
            trace.add("execute", None, true, None, Some(format!("ok {i}")), None);
        }
        assert_eq!(trace.events.len(), TRACE_MAX_EVENTS);
        trace.add(
            "execute",
            None,
            false,
            Some(CODE_EXECUTION_FAILED),
            Some("boom".to_string()),
            None,
        );
        assert_eq!(trace.events.len(), TRACE_MAX_EVENTS);
        assert_eq!(trace.events[0].seq, 2);
        assert_eq!(trace.events[TRACE_MAX_EVENTS - 1].seq, TRACE_MAX_EVENTS + 1);
        assert!(!trace.events[TRACE_MAX_EVENTS - 1].ok);
        assert_eq!(
            trace.events[TRACE_MAX_EVENTS - 1].code.as_deref(),
            Some(CODE_EXECUTION_FAILED)
        );
    }

    #[test]
    fn trace_store_commit_makes_events_visible() {
        let mut store = TraceStore::new();
        let mut trace = store.new_trace(Some("req-1".to_string()));
        trace.add(
            "registry",
            Some("workspace.read_file"),
            true,
            None,
            None,
            None,
        );
        assert!(
            store.get(&trace.trace_id).unwrap().events.is_empty(),
            "placeholder must be empty until commit"
        );
        store.commit(&trace);
        let stored = store.get(&trace.trace_id).expect("trace retained");
        assert_eq!(stored.events.len(), 1);
        assert_eq!(stored.events[0].step, "registry");
        assert_eq!(stored.request_id.as_deref(), Some("req-1"));
    }

    #[test]
    fn trace_bounds_events_and_store() {
        let mut store = TraceStore::new();
        let mut trace = store.new_trace(Some("req-1".to_string()));
        for i in 0..(TRACE_MAX_EVENTS + 10) {
            trace.add(
                "execute",
                None,
                true,
                None,
                Some(format!("event {i}")),
                None,
            );
        }
        assert_eq!(trace.events.len(), TRACE_MAX_EVENTS);
        assert_eq!(trace.events[0].seq, 1);
        assert_eq!(trace.events[TRACE_MAX_EVENTS - 1].seq, TRACE_MAX_EVENTS);
        let (tail, truncated) = trace.tail(10);
        assert!(truncated);
        assert_eq!(tail.len(), 10);

        for i in 0..(TRACE_STORE_MAX_TRACES + 10) {
            store.new_trace(Some(format!("req-{i}")));
        }
        assert!(store.get("t000001").is_none());
        assert!(
            store
                .get(&format!("t{:06}", TRACE_STORE_MAX_TRACES + 10))
                .is_some()
        );
    }
}
