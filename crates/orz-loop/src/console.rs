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
//!
//! P0-C S3 (2026-08-15)：`assistant.trace` 只读服务与 `workspace.run_script`
//! （PTC 线性脚本）作为控制台内部动作接线（`ActionKind::TraceRead` /
//! `ActionKind::RunScript`，不委托 host 工具）；注册板块升级为
//! 「Profile/Bundle 加载集 ∩ 探针完整集」投影（`registrations_for`）。

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::blackboard::ActionOrder;
use crate::host::{ToolPolicy, ToolResult};

/// 单 trace 事件上限（POC 同构：200 条）。
pub const TRACE_MAX_EVENTS: usize = 200;
/// trace 存储保留的最近请求数（POC 同构：50）。
pub const TRACE_STORE_MAX_TRACES: usize = 50;
/// `assistant.trace` 默认返回的最近事件数（POC 同构：20）。
pub const TRACE_TAIL_DEFAULT: usize = 20;

/// PTC 线性脚本服务名（小样 3 定档）。
pub const SCRIPT_SERVICE_NAME: &str = "workspace.run_script";
/// 只读 trace 服务名（v0.4 定档）。
pub const TRACE_SERVICE_NAME: &str = "assistant.trace";
/// PTC 脚本上限（小样 3 定档：20 步 / 30s 墙钟 / 4 MiB 累计响应）。
pub const MAX_SCRIPT_STEPS: usize = 20;
pub const MAX_SCRIPT_WALLCLOCK_SECONDS: f64 = 30.0;
pub const MAX_SCRIPT_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

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
/// P0-C S3：控制台内部服务错误码（POC `script_runner.py` 同构）。
pub const CODE_INVALID_SCRIPT: &str = "invalid_script";
pub const CODE_DUPLICATE_STEP_NAME: &str = "duplicate_step_name";
pub const CODE_INVALID_REFERENCE: &str = "invalid_reference";
pub const CODE_REFERENCE_SCOPE: &str = "reference_scope";
pub const CODE_REFERENCE_TYPE_MISMATCH: &str = "reference_type_mismatch";
pub const CODE_NESTED_SCRIPT: &str = "nested_script_not_allowed";
pub const CODE_SCRIPT_TIMEOUT: &str = "script_timeout";
pub const CODE_SCRIPT_RESPONSE_LIMIT: &str = "script_response_limit";
pub const CODE_TRACE_UNAVAILABLE: &str = "trace_unavailable";
pub const CODE_NOT_FOUND: &str = "not_found";

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
            target_tool: Some("read_file".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
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
            target_tool: Some("list_dir".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
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
            target_tool: Some("grep".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
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
            target_tool: Some("search_replace".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::READ_WRITE,
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
            target_tool: Some("run_tests".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::READ_WRITE,
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
            target_tool: Some("project_doc_index".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
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
        // P0-C S3：控制台内部只读服务——按 trace_id 取回有界执行日志
        // （POC `actions.py` 的 `assistant.trace` 同构；不委托 host 工具）。
        ActionSpec {
            name: TRACE_SERVICE_NAME.to_string(),
            description:
                "只读执行日志：按 trace_id 取回有界 trace（模型排障入口；读操作本身入审计）。"
                    .to_string(),
            target_tool: None,
            kind: ActionKind::TraceRead,
            bundle: ActionBundle::ALL,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "trace_id": {
                        "type": "string",
                        "description": "结果栏 receipt 或错误信封携带的 trace_id。",
                    },
                    "tail": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": TRACE_MAX_EVENTS,
                        "description": "返回的最近事件条数（默认 20）。",
                    },
                },
                "required": ["trace_id"],
                "additionalProperties": false,
            }),
            response_schema: json!({
                "type": "object",
                "properties": {
                    "trace_id": {"type": "string"},
                    "request_id": {"type": ["string", "null"]},
                    "events": {
                        "type": "array",
                        "items": {"type": "object"},
                    },
                    "truncated": {"type": "boolean"},
                },
                "required": ["trace_id", "request_id", "events", "truncated"],
                "additionalProperties": false,
            }),
        },
        // P0-C S3：PTC 线性脚本服务——步骤 = 注册动作实例 + `$ref` 数据引用；
        // 逐行契约校验 + trace；任一步 fail-closed（POC `script_runner.py`
        // 同构；上限 20 步 / 30s 墙钟 / 4 MiB 累计响应）。
        ActionSpec {
            name: SCRIPT_SERVICE_NAME.to_string(),
            description: "执行确定性线性动作脚本（PTC）：步骤 = 注册动作实例 + `$ref` 数据引用；\
                 逐行契约校验 + trace；任一步 fail-closed；上限 20 步/30s/4MiB。"
                .to_string(),
            target_tool: None,
            kind: ActionKind::RunScript,
            bundle: ActionBundle::ALL,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "script": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "do": {"type": "string"},
                                "with": {"type": "object"},
                                "as": {
                                    "type": "string",
                                    "pattern": "^[A-Za-z_][A-Za-z0-9_]*$",
                                },
                            },
                            "required": ["do"],
                            "additionalProperties": false,
                        },
                        "minItems": 1,
                        "maxItems": MAX_SCRIPT_STEPS,
                    }
                },
                "required": ["script"],
                "additionalProperties": false,
            }),
            response_schema: json!({
                "type": "object",
                "properties": {
                    "steps": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "index": {"type": "integer"},
                                "do": {"type": "string"},
                                "as": {"type": ["string", "null"]},
                                "ok": {"type": "boolean"},
                                "response": {"type": "object"},
                            },
                            "required": ["index", "do", "as", "ok", "response"],
                            "additionalProperties": false,
                        },
                    },
                    "result": {"type": ["object", "null"]},
                },
                "required": ["steps", "result"],
                "additionalProperties": false,
            }),
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

/// 动作类型（P0-C S3）：决定发放链如何执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// 委托既有 host 工具执行（生产注册动作集默认；目标解析/ACAF/权限/
    /// 模式门经 `ActionExecutor` 全链路复用）。
    #[default]
    Host,
    /// 控制台内部只读服务：按 `trace_id` 取回有界执行日志
    /// （POC `assistant.trace` 同构；读操作本身入 trace 与事件面）。
    TraceRead,
    /// 控制台内部 PTC 线性脚本：注册动作实例序列逐行执行
    /// （POC `workspace.run_script` 同构；`$ref` 数据引用、逐行契约 +
    /// trace、fail-closed）。
    RunScript,
}

/// Profile/Bundle 分区（P0-C S3；DeepSeek Harness 借鉴，只借设计）：
/// 按会话场景（Benchmark / ReadOnly / 标准）预打包动作集。
/// `ToolPolicy` 即场景键——Interactive=标准、ReadOnly=只读、Benchmark=基准。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ActionBundle {
    /// 标准交互会话（`ToolPolicy::Interactive`）。
    pub standard: bool,
    /// 只读会话（`ToolPolicy::ReadOnly`）。
    pub read_only: bool,
    /// 基准会话（`ToolPolicy::Benchmark`——读 + 本地文件编辑 + 测试）。
    pub benchmark: bool,
}

impl ActionBundle {
    /// 全部场景加载（只读服务 / 组合层 / 纯读动作）。
    pub const ALL: Self = Self {
        standard: true,
        read_only: true,
        benchmark: true,
    };
    /// 标准 + 基准（编辑/测试动作——ReadOnly 不加载写面按钮）。
    pub const READ_WRITE: Self = Self {
        standard: true,
        read_only: false,
        benchmark: true,
    };

    pub fn allows(&self, profile: ToolPolicy) -> bool {
        match profile {
            ToolPolicy::Interactive => self.standard,
            ToolPolicy::ReadOnly => self.read_only,
            ToolPolicy::Benchmark => self.benchmark,
        }
    }
}

/// 动作契约（版本化 API）：名称 + 输入 schema + 目标工具 + 响应 schema。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionSpec {
    pub name: String,
    pub description: String,
    /// 解析后的真实执行目标（既有 host 工具名；操作台不建第二执行器）。
    /// 控制台内部动作（`ActionKind::TraceRead` / `RunScript`）为 `None`。
    pub target_tool: Option<String>,
    pub input_schema: Value,
    /// 响应契约（必填）：对 `ToolResult.structured`（无则 `{"output": ...}`）
    /// 做机械验证——任何输出必须过机械验证（审计的一部分），注册时校验并缓存。
    pub response_schema: Value,
    /// P0-C S3：动作类型（默认 Host；注册时校验 Host 必须有目标工具）。
    #[serde(default)]
    pub kind: ActionKind,
    /// P0-C S3：Profile/Bundle 分区（按会话场景加载的按钮组）。
    #[serde(default)]
    pub bundle: ActionBundle,
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
        // P0-C S3 不变式：Host 动作必须携带真实目标工具；内部动作不得
        // 伪装成 host 执行（fail-fast，注册即拒绝，绝不运行时兜底）。
        if spec.kind == ActionKind::Host && spec.target_tool.is_none() {
            return Err(RegistryError::InvalidSchema {
                name: spec.name.clone(),
                kind: "target",
                detail: "host actions require a target tool".to_string(),
            });
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

    /// P0-C S3：注册板块投影 = Profile/Bundle 加载集 ∩ 探针完整集。
    ///
    /// - `bundle` 按会话场景（Interactive/ReadOnly/Benchmark）过滤按钮组；
    /// - 探针过滤只作用于 Host 动作的工作工具目标：工作工具必须出现在
    ///   本轮 `ToolProbeSnapshot.complete` 中；非工作工具（如
    ///   `project_doc_index`）不探不标、按注册表声明恒显示（与模型可见
    ///   工具投影的「完整集 ∩ 声明集 + 非工作工具」语义一致）；控制台
    ///   内部动作（trace/run_script）无 host 目标，不做探针过滤；
    /// - `probe = None`（checkpoint 轮等无探针间隙）时只做 bundle 过滤，
    ///   板块保留上一轮内容由调用方决定是否刷新。
    pub fn registrations_for(
        &self,
        profile: ToolPolicy,
        probe: Option<&crate::tool_probe::ToolProbeSnapshot>,
    ) -> Vec<crate::blackboard::ActionRegistration> {
        self.actions
            .values()
            .filter(|spec| spec.bundle.allows(profile))
            .filter(|spec| match (spec.kind, probe) {
                (ActionKind::Host, Some(snapshot)) => {
                    let Some(tool) = spec.target_tool.as_deref() else {
                        return false;
                    };
                    !crate::tool_probe::is_main_agent_work_tool(tool)
                        || snapshot.complete.iter().any(|t| t == tool)
                }
                _ => true,
            })
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
///
/// P0-C S3：`traces` 为控制台 TraceStore（`assistant.trace` 只读服务与
/// PTC 脚本内嵌的 trace 步骤需要）；Host 动作不需要但可传 `None`。
/// 内部动作（TraceRead/RunScript）不委托 `ActionExecutor`，走控制台内部
/// 确定性实现；Host 动作保持注册表 → 契约 → 目标解析 → 执行委托 →
/// 响应验证五步链。
pub async fn issue_action<E: ActionExecutor + ?Sized>(
    registry: &ServiceRegistry,
    executor: &E,
    traces: Option<&std::sync::Mutex<TraceStore>>,
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

    match spec.kind {
        ActionKind::Host => {
            let target_tool = spec.target_tool.as_deref().ok_or_else(|| ConsoleError {
                step: STEP_REGISTRY,
                code: CODE_INTERNAL_ERROR,
                message: format!("{}: host action without target tool", spec.name),
                upstream: Some(json!({ "action": spec.name })),
            })?;
            trace.add(
                STEP_TARGET,
                Some(&order.action),
                true,
                None,
                Some(format!("resolved target tool: {target_tool}")),
                Some(json!({ "target_tool": target_tool })),
            );

            let result = executor
                .execute(target_tool, &order.arguments, call_id)
                .await
                .map_err(|err| match err {
                    ExecuteError::ExecutionFailed { message, detail } => ConsoleError {
                        step: STEP_EXECUTE,
                        code: CODE_EXECUTION_FAILED,
                        message,
                        upstream: Some(execution_upstream(&spec.name, target_tool, detail)),
                    },
                    ExecuteError::PolicyDenied { message, detail } => ConsoleError {
                        step: STEP_POLICY,
                        code: CODE_POLICY_DENIED,
                        message,
                        upstream: Some(execution_upstream(&spec.name, target_tool, detail)),
                    },
                })?;
            if result.exit_code != Some(0) {
                let exit_detail = match result.exit_code {
                    None => {
                        "no exit code (actions require an explicit success exit code)".to_string()
                    }
                    Some(code) => format!("non-zero exit code {code}"),
                };
                return Err(ConsoleError {
                    step: STEP_EXECUTE,
                    code: CODE_EXECUTION_FAILED,
                    message: format!("target tool {exit_detail}"),
                    upstream: Some(json!({
                        "target_tool": target_tool,
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
                    "target_tool": target_tool,
                })),
            })?;
            trace.add(STEP_VERIFY, Some(&order.action), true, None, None, None);

            Ok(response)
        }
        ActionKind::TraceRead => {
            let store = traces.ok_or_else(|| ConsoleError {
                step: STEP_REGISTRY,
                code: CODE_TRACE_UNAVAILABLE,
                message: format!("{}: trace store unavailable", spec.name),
                upstream: Some(json!({ "action": spec.name })),
            })?;
            let trace_id = order
                .arguments
                .get("trace_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let tail = order
                .arguments
                .get("tail")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .unwrap_or(TRACE_TAIL_DEFAULT);
            // 同步短锁取回快照，锁不跨 await（发放链可保持 Send）。
            let (request_id, events, truncated) = {
                let guard = store.lock().unwrap();
                let found = guard.get(trace_id).ok_or_else(|| ConsoleError {
                    step: STEP_REGISTRY,
                    code: CODE_NOT_FOUND,
                    message: format!("trace not found: {trace_id}"),
                    upstream: Some(json!({ "trace_id": trace_id })),
                })?;
                let (events, truncated) = found.tail(tail);
                (found.request_id.clone(), events, truncated)
            };
            let response = json!({
                "trace_id": trace_id,
                "request_id": request_id,
                "events": events,
                "truncated": truncated,
            });
            trace.add(STEP_EXECUTE, Some(&order.action), true, None, None, None);
            validate_with(response_validator, &response).map_err(|msg| ConsoleError {
                step: STEP_VERIFY,
                code: CODE_INVALID_RESPONSE,
                message: format!("{}: response failed verification: {msg}", spec.name),
                upstream: Some(json!({ "action": spec.name })),
            })?;
            trace.add(STEP_VERIFY, Some(&order.action), true, None, None, None);
            Ok(response)
        }
        ActionKind::RunScript => {
            let response =
                run_script(registry, executor, traces, &order.arguments, trace, call_id).await?;
            trace.add(STEP_EXECUTE, Some(&order.action), true, None, None, None);
            validate_with(response_validator, &response).map_err(|msg| ConsoleError {
                step: STEP_VERIFY,
                code: CODE_INVALID_RESPONSE,
                message: format!("{}: response failed verification: {msg}", spec.name),
                upstream: Some(json!({ "action": spec.name })),
            })?;
            trace.add(STEP_VERIFY, Some(&order.action), true, None, None, None);
            Ok(response)
        }
    }
}

/// 脚本步骤的静态元数据（POC `script_runner.py::_static_validate` 同构）。
struct ScriptStepMeta {
    index: usize,
    action: String,
    arguments: Value,
    name: Option<String>,
}

/// JSON Schema 的 `type` 关键字 → 类型名列表（string 或 array）。
fn schema_types(schema: Option<&Value>) -> Option<Vec<String>> {
    let ty = schema?.get("type")?;
    match ty {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(a) => {
            let types: Vec<String> = a
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            (!types.is_empty()).then_some(types)
        }
        _ => None,
    }
}

/// `integer`/`number` 互认（POC `_types_compatible` 同构）。
fn types_compatible(expected: Option<&[String]>, actual: Option<&[String]>) -> bool {
    let (Some(expected), Some(actual)) = (expected, actual) else {
        return true;
    };
    expected.iter().any(|e| {
        actual.iter().any(|a| {
            e == a || (e == "integer" && a == "number") || (e == "number" && a == "integer")
        })
    })
}

/// 收集 `{"$ref": ...}` 叶节点及该路径在输入 schema 中的期望类型
/// （POC `_collect_refs` 同构）。
fn collect_refs(
    value: &Value,
    schema: Option<&Value>,
    path: &str,
    out: &mut Vec<(String, String, Option<Vec<String>>)>,
) -> Result<(), ConsoleError> {
    match value {
        Value::Object(map) => {
            if map.len() == 1
                && let Some(Value::String(reference)) = map.get("$ref")
            {
                if reference.trim().is_empty() {
                    return Err(ConsoleError {
                        step: STEP_CONTRACT,
                        code: CODE_INVALID_REFERENCE,
                        message: format!("{path}: $ref must be a non-empty string"),
                        upstream: Some(json!({ "path": path })),
                    });
                }
                out.push((path.to_string(), reference.clone(), schema_types(schema)));
                return Ok(());
            }
            let props = schema
                .and_then(|s| s.get("properties"))
                .and_then(Value::as_object);
            let additional = schema.and_then(|s| s.get("additionalProperties"));
            for (key, val) in map {
                let child = props
                    .and_then(|p| p.get(key))
                    .or_else(|| additional.filter(|a| a.is_object()));
                collect_refs(val, child, &format!("{path}.{key}"), out)?;
            }
        }
        Value::Array(items) => {
            let item_schema = schema.and_then(|s| s.get("items"));
            for (idx, val) in items.iter().enumerate() {
                collect_refs(val, item_schema, &format!("{path}[{idx}]"), out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// 解析响应 schema 中点分字段路径的叶子类型（POC `_resolve_leaf_type` 同构）。
fn resolve_leaf_type(response_schema: &Value, parts: &[&str]) -> Option<Vec<String>> {
    let mut schema = Some(response_schema);
    for (idx, part) in parts.iter().enumerate() {
        let cur = schema?;
        let is_last = idx == parts.len() - 1;
        if let Some(prop) = cur
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|props| props.get(*part))
        {
            if is_last {
                return schema_types(Some(prop));
            }
            schema = Some(prop);
            continue;
        }
        if let Some(items) = cur
            .get("items")
            .filter(|_| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        {
            if is_last {
                return schema_types(Some(items));
            }
            schema = Some(items);
            continue;
        }
        return None;
    }
    None
}

/// 整个脚本执行前的静态校验（fail-closed：任一错误先于任何执行返回）：
/// 名称唯一、服务已知、禁嵌套脚本、`$ref` 形状/作用域/类型（POC 同构）。
fn static_validate_script(
    script: &[Value],
    registry: &ServiceRegistry,
) -> Result<Vec<ScriptStepMeta>, ConsoleError> {
    let mut available: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut named_response_schemas: BTreeMap<String, Value> = BTreeMap::new();
    let mut steps = Vec::with_capacity(script.len());
    for (idx, raw) in script.iter().enumerate() {
        let index = idx + 1;
        let Some(step) = raw.as_object() else {
            return Err(ConsoleError {
                step: STEP_CONTRACT,
                code: CODE_INVALID_SCRIPT,
                message: format!("script[{index}]: step must be an object"),
                upstream: Some(json!({ "script_step": index })),
            });
        };
        let action = step
            .get("do")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .ok_or_else(|| ConsoleError {
                step: STEP_CONTRACT,
                code: CODE_INVALID_SCRIPT,
                message: format!("script[{index}].do must be a non-empty string"),
                upstream: Some(json!({ "script_step": index })),
            })?;
        let name = step.get("as").and_then(Value::as_str).map(str::to_string);
        let arguments = step.get("with").cloned().unwrap_or_else(|| json!({}));
        if let Some(n) = &name
            && available.contains(n)
        {
            return Err(ConsoleError {
                step: STEP_CONTRACT,
                code: CODE_DUPLICATE_STEP_NAME,
                message: format!("script[{index}].as duplicates an earlier step name: {n}"),
                upstream: Some(json!({ "name": n, "script_step": index })),
            });
        }
        let spec = registry.get(&action).ok_or_else(|| ConsoleError {
            step: STEP_REGISTRY,
            code: CODE_UNKNOWN_SERVICE,
            message: format!("script[{index}].do: unknown service: {action}"),
            upstream: Some(json!({ "script_step": index, "action": action })),
        })?;
        if action == SCRIPT_SERVICE_NAME {
            return Err(ConsoleError {
                step: STEP_CONTRACT,
                code: CODE_NESTED_SCRIPT,
                message: format!("script[{index}].do: nested scripts are not allowed"),
                upstream: Some(json!({ "script_step": index })),
            });
        }
        let mut refs = Vec::new();
        collect_refs(
            &arguments,
            Some(&spec.input_schema),
            &format!("script[{index}].with"),
            &mut refs,
        )?;
        for (path, reference, expected) in refs {
            let parts: Vec<&str> = reference.split('.').collect();
            if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
                return Err(ConsoleError {
                    step: STEP_CONTRACT,
                    code: CODE_INVALID_REFERENCE,
                    message: format!("{path}: invalid $ref shape: {reference}"),
                    upstream: Some(json!({ "script_step": index, "ref": reference })),
                });
            }
            let ref_name = parts[0];
            if !available.contains(ref_name) {
                return Err(ConsoleError {
                    step: STEP_CONTRACT,
                    code: CODE_REFERENCE_SCOPE,
                    message: format!("{path}: $ref must name an earlier step output: {reference}"),
                    upstream: Some(json!({ "script_step": index, "ref": reference })),
                });
            }
            let leaf_type = resolve_leaf_type(
                named_response_schemas
                    .get(ref_name)
                    .expect("named earlier step has a response schema"),
                &parts[1..],
            );
            let Some(leaf_type) = leaf_type else {
                return Err(ConsoleError {
                    step: STEP_CONTRACT,
                    code: CODE_INVALID_REFERENCE,
                    message: format!(
                        "{path}: referenced field not in {ref_name} response schema: {reference}"
                    ),
                    upstream: Some(json!({ "script_step": index, "ref": reference })),
                });
            };
            if !types_compatible(expected.as_deref(), Some(&leaf_type)) {
                return Err(ConsoleError {
                    step: STEP_CONTRACT,
                    code: CODE_REFERENCE_TYPE_MISMATCH,
                    message: format!(
                        "{path}: $ref type mismatch: expected {expected:?}, \
                         referenced {leaf_type:?} ({reference})"
                    ),
                    upstream: Some(json!({
                        "script_step": index,
                        "ref": reference,
                        "expected": expected,
                        "actual": leaf_type,
                    })),
                });
            }
        }
        if let Some(n) = &name {
            available.insert(n.clone());
            named_response_schemas.insert(n.clone(), spec.response_schema.clone());
        }
        steps.push(ScriptStepMeta {
            index,
            action,
            arguments,
            name,
        });
    }
    Ok(steps)
}

/// 运行时 `$ref` 解析失败（POC `_substitute` 同构）——只发生在静态校验
/// 已通过之后（理论上不可达，fail-closed 仍显式结构化）。
fn runtime_ref_error(reference: &str) -> ConsoleError {
    ConsoleError {
        step: STEP_EXECUTE,
        code: CODE_INVALID_REFERENCE,
        message: format!("runtime $ref resolution failed: {reference}"),
        upstream: Some(json!({ "ref": reference })),
    }
}

/// 递归替换参数中的 `$ref` 叶节点（POC `_substitute` 同构）。
fn substitute(value: &Value, outputs: &BTreeMap<String, Value>) -> Result<Value, ConsoleError> {
    match value {
        Value::Object(map) => {
            if map.len() == 1
                && let Some(Value::String(reference)) = map.get("$ref")
            {
                let parts: Vec<&str> = reference.split('.').collect();
                let mut current = outputs
                    .get(parts[0])
                    .cloned()
                    .ok_or_else(|| runtime_ref_error(reference))?;
                for part in &parts[1..] {
                    current = match current {
                        Value::Object(m) => m
                            .get(*part)
                            .cloned()
                            .ok_or_else(|| runtime_ref_error(reference))?,
                        Value::Array(a) => part
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| a.get(i).cloned())
                            .ok_or_else(|| runtime_ref_error(reference))?,
                        _ => return Err(runtime_ref_error(reference)),
                    };
                }
                return Ok(current);
            }
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                out.insert(key.clone(), substitute(val, outputs)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => items
            .iter()
            .map(|item| substitute(item, outputs))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        other => Ok(other.clone()),
    }
}

/// 运行 PTC 线性脚本（POC `script_runner.py::run_script` 同构；默认上限）。
pub async fn run_script<E: ActionExecutor + ?Sized>(
    registry: &ServiceRegistry,
    executor: &E,
    traces: Option<&std::sync::Mutex<TraceStore>>,
    arguments: &Value,
    trace: &mut Trace,
    call_id: &str,
) -> Result<Value, ConsoleError> {
    run_script_with_limits(
        registry,
        executor,
        traces,
        arguments,
        trace,
        call_id,
        MAX_SCRIPT_STEPS,
        MAX_SCRIPT_WALLCLOCK_SECONDS,
        MAX_SCRIPT_RESPONSE_BYTES,
    )
    .await
}

/// 带可注入上限的脚本运行器（测试用更小上限验证 timeout/字节门）。
#[allow(clippy::too_many_arguments)] // mirrors run_host_tool's shared-loop contract
async fn run_script_with_limits<E: ActionExecutor + ?Sized>(
    registry: &ServiceRegistry,
    executor: &E,
    traces: Option<&std::sync::Mutex<TraceStore>>,
    arguments: &Value,
    trace: &mut Trace,
    call_id: &str,
    max_steps: usize,
    max_wallclock: f64,
    max_bytes: usize,
) -> Result<Value, ConsoleError> {
    let script = arguments
        .get("script")
        .and_then(Value::as_array)
        .ok_or_else(|| ConsoleError {
            step: STEP_CONTRACT,
            code: CODE_INVALID_SCRIPT,
            message: "run_script: `script` must be an array".to_string(),
            upstream: Some(json!({ "action": SCRIPT_SERVICE_NAME })),
        })?;
    if script.is_empty() || script.len() > max_steps {
        return Err(ConsoleError {
            step: STEP_CONTRACT,
            code: CODE_INVALID_SCRIPT,
            message: format!(
                "run_script: script length {} outside 1..={max_steps}",
                script.len()
            ),
            upstream: Some(json!({ "length": script.len(), "max_steps": max_steps })),
        });
    }
    let metas = static_validate_script(script, registry)?;
    let started = Instant::now();
    let mut outputs: BTreeMap<String, Value> = BTreeMap::new();
    let mut executed: Vec<Value> = Vec::with_capacity(metas.len());
    let mut response_bytes = 0usize;
    let mut final_response: Option<Value> = None;

    for meta in &metas {
        if started.elapsed().as_secs_f64() > max_wallclock {
            return Err(ConsoleError {
                step: STEP_EXECUTE,
                code: CODE_SCRIPT_TIMEOUT,
                message: format!("script exceeded {max_wallclock:.0}s wall clock"),
                upstream: Some(json!({
                    "script_step": meta.index,
                    "action": meta.action,
                })),
            });
        }
        let arguments = substitute(&meta.arguments, &outputs)?;
        let step_call_id = format!("{call_id}.s{}", meta.index);
        let step_order = ActionOrder {
            order_id: step_call_id.clone(),
            action: meta.action.clone(),
            arguments,
            round: 0,
            plan_epoch: 0,
            run_id: String::new(),
        };
        // 递归（run_script → issue_action → run_script）需要指针间接层，
        // 避免 async future 无限尺寸；嵌套脚本已在静态校验阶段拒绝。
        let response = match Box::pin(issue_action(
            registry,
            executor,
            traces,
            &step_order,
            trace,
            &step_call_id,
        ))
        .await
        {
            Ok(response) => response,
            Err(inner) => {
                trace.add(
                    "script",
                    Some(&meta.action),
                    false,
                    Some(inner.code),
                    Some(inner.message.clone()),
                    Some(json!({
                        "script_step": meta.index,
                        "code": inner.code,
                        "upstream": inner.upstream,
                    })),
                );
                return Err(ConsoleError {
                    step: inner.step,
                    code: inner.code,
                    message: format!(
                        "script step {} ({}) failed: {}",
                        meta.index, meta.action, inner.message
                    ),
                    upstream: Some(json!({
                        "script_step": meta.index,
                        "action": meta.action,
                        "code": inner.code,
                        "message": inner.message,
                        "upstream": inner.upstream,
                    })),
                });
            }
        };
        let entry = json!({
            "index": meta.index,
            "do": meta.action,
            "as": meta.name,
            "ok": true,
            "response": response,
        });
        let entry_bytes = serde_json::to_vec(&entry)
            .map_err(|e| ConsoleError {
                step: STEP_EXECUTE,
                code: CODE_INTERNAL_ERROR,
                message: format!("script step serialization failed: {e}"),
                upstream: Some(json!({ "script_step": meta.index })),
            })?
            .len();
        if response_bytes.saturating_add(entry_bytes) > max_bytes {
            return Err(ConsoleError {
                step: STEP_EXECUTE,
                code: CODE_SCRIPT_RESPONSE_LIMIT,
                message: format!("script response exceeded {max_bytes} bytes"),
                upstream: Some(json!({
                    "script_step": meta.index,
                    "action": meta.action,
                    "response_bytes": response_bytes + entry_bytes,
                    "limit": max_bytes,
                })),
            });
        }
        response_bytes += entry_bytes;
        if let Some(name) = &meta.name {
            outputs.insert(name.clone(), response.clone());
        }
        trace.add(
            "script",
            Some(&meta.action),
            true,
            None,
            None,
            Some(json!({ "script_step": meta.index, "as": meta.name })),
        );
        executed.push(entry);
        final_response = Some(response);
    }

    let result = json!({ "steps": executed, "result": final_response });
    let total_bytes = serde_json::to_vec(&result)
        .map_err(|e| ConsoleError {
            step: STEP_EXECUTE,
            code: CODE_INTERNAL_ERROR,
            message: format!("script result serialization failed: {e}"),
            upstream: None,
        })?
        .len();
    if total_bytes > max_bytes {
        return Err(ConsoleError {
            step: STEP_EXECUTE,
            code: CODE_SCRIPT_RESPONSE_LIMIT,
            message: format!(
                "script response exceeded {max_bytes} bytes (includes final result duplication)"
            ),
            upstream: Some(json!({
                "response_bytes": total_bytes,
                "limit": max_bytes,
            })),
        });
    }
    Ok(result)
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
            target_tool: Some(target.to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
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
                        ..Default::default()
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
                "assistant.trace",
                "workspace.grep",
                "workspace.index",
                "workspace.list_dir",
                "workspace.read_file",
                "workspace.run_script",
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
        // P0-C S3：内部服务不携带 host 目标、走内部动作类型。
        assert_eq!(
            registry.get("assistant.trace").unwrap().kind,
            ActionKind::TraceRead
        );
        assert_eq!(
            registry.get("workspace.run_script").unwrap().kind,
            ActionKind::RunScript
        );
        assert!(
            registry
                .get("assistant.trace")
                .unwrap()
                .target_tool
                .is_none()
        );
        assert!(
            registry
                .get("workspace.run_script")
                .unwrap()
                .target_tool
                .is_none()
        );
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
            None,
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
            None,
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
            None,
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
            None,
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
                ..Default::default()
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
            None,
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
                ..Default::default()
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
            None,
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
            None,
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
                ..Default::default()
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
            None,
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
            target_tool: Some("read_file".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
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
            target_tool: Some("read_file".to_string()),
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
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
                target_tool: Some("read_file".to_string()),
                kind: ActionKind::Host,
                bundle: ActionBundle::ALL,
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

    /// P0-C S3：Host 动作必须携带真实目标工具（注册即拒绝，不运行时兜底）。
    #[test]
    fn host_action_without_target_tool_rejected_at_registration() {
        let mut registry = ServiceRegistry::new();
        let err = registry.register(ActionSpec {
            name: "bad.host".to_string(),
            description: "host action without target".to_string(),
            target_tool: None,
            kind: ActionKind::Host,
            bundle: ActionBundle::ALL,
            input_schema: json!({"type": "object"}),
            response_schema: json!({"type": "object"}),
        });
        assert!(matches!(
            err,
            Err(RegistryError::InvalidSchema { kind: "target", .. })
        ));
    }

    /// P0-C S3：注册板块投影 = Profile/Bundle ∩ 探针完整集。
    #[test]
    fn registrations_for_filters_by_bundle_and_probe() {
        let registry = default_service_registry();
        let names = |profile: ToolPolicy, probe: Option<&crate::tool_probe::ToolProbeSnapshot>| {
            registry
                .registrations_for(profile, probe)
                .into_iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
        };
        // ReadOnly：无编辑/测试按钮，保留读面 + 内部服务。
        let ro = names(ToolPolicy::ReadOnly, None);
        assert!(ro.contains(&"workspace.read_file".to_string()));
        assert!(ro.contains(&"assistant.trace".to_string()));
        assert!(ro.contains(&"workspace.run_script".to_string()));
        assert!(!ro.contains(&"workspace.search_replace".to_string()));
        assert!(!ro.contains(&"workspace.run_tests".to_string()));
        // Benchmark：编辑/测试动作加载。
        let bm = names(ToolPolicy::Benchmark, None);
        assert!(bm.contains(&"workspace.search_replace".to_string()));
        assert!(bm.contains(&"workspace.run_tests".to_string()));
        // 探针过滤：run_tests 不完整 → 移除；非工作工具目标
        // （project_doc_index）不探不标、按注册表声明保留。
        let snapshot = crate::tool_probe::ToolProbeSnapshot {
            complete: vec![
                "read_file".to_string(),
                "grep".to_string(),
                "list_dir".to_string(),
                "search_replace".to_string(),
            ],
            incomplete: vec![crate::tool_probe::ProbeFailure {
                tool: "run_tests".to_string(),
                reason: crate::tool_probe::REASON_MISSING_TEST_RUNNER,
            }],
        };
        let proj = names(ToolPolicy::Interactive, Some(&snapshot));
        assert!(!proj.contains(&"workspace.run_tests".to_string()));
        assert!(proj.contains(&"workspace.index".to_string()));
        assert!(proj.contains(&"workspace.run_script".to_string()));
        assert!(proj.contains(&"workspace.read_file".to_string()));
    }

    /// P0-C S3：`assistant.trace` 只读服务——按 trace_id 取回有界事件。
    #[tokio::test]
    async fn trace_read_service_returns_bounded_events() {
        let registry = default_service_registry();
        let (executor, _) = FakeExecutor::ok();
        let store = std::sync::Mutex::new(TraceStore::new());
        {
            let mut guard = store.lock().unwrap();
            let mut target = guard.new_trace(Some("ORD-000001".to_string()));
            for i in 0..30 {
                target.add(
                    "execute",
                    Some("workspace.read_file"),
                    true,
                    None,
                    Some(format!("event {i}")),
                    None,
                );
            }
            guard.commit(&target);
        }
        let mut trace = Trace {
            trace_id: "t000099".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let response = issue_action(
            &registry,
            &executor,
            Some(&store),
            &order(
                "assistant.trace",
                json!({"trace_id": "t000001", "tail": 10}),
            ),
            &mut trace,
            "call-t",
        )
        .await
        .unwrap();
        assert_eq!(response["trace_id"], "t000001");
        assert_eq!(response["request_id"], "ORD-000001");
        assert_eq!(response["events"].as_array().unwrap().len(), 10);
        assert!(response["truncated"].as_bool().unwrap());
        // 读操作本身进入本次 trace（审计）。
        assert!(trace.events.iter().any(|e| e.step == STEP_EXECUTE));
        assert!(trace.events.iter().any(|e| e.step == STEP_VERIFY));
    }

    /// P0-C S3：trace 缺失按 `not_found` 结构化失败（fail-closed）。
    #[tokio::test]
    async fn trace_read_missing_trace_fails_not_found() {
        let registry = default_service_registry();
        let (executor, _) = FakeExecutor::ok();
        let store = std::sync::Mutex::new(TraceStore::new());
        let mut trace = Trace {
            trace_id: "t000098".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = issue_action(
            &registry,
            &executor,
            Some(&store),
            &order("assistant.trace", json!({"trace_id": "t999999"})),
            &mut trace,
            "call-t",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_REGISTRY);
        assert_eq!(err.code, CODE_NOT_FOUND);
        assert_eq!(err.upstream.as_ref().unwrap()["trace_id"], "t999999");
    }

    /// P0-C S3：PTC 脚本成功路径——`$ref` 数据引用、逐行执行、响应验证。
    #[tokio::test]
    async fn run_script_executes_steps_with_refs() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ActionSpec {
                name: "workspace.read_file".to_string(),
                description: "ref test action".to_string(),
                target_tool: Some("read_file".to_string()),
                kind: ActionKind::Host,
                bundle: ActionBundle::ALL,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "nested": {
                            "type": "object",
                            "properties": {"count": {"type": "integer"}},
                        },
                    },
                    "required": ["path"],
                    "additionalProperties": false,
                }),
                response_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"},
                        "count": {"type": "integer"},
                    },
                    "required": ["path", "content"],
                    "additionalProperties": false,
                }),
            })
            .unwrap();
        registry
            .register(ActionSpec {
                name: SCRIPT_SERVICE_NAME.to_string(),
                description: "script action".to_string(),
                target_tool: None,
                kind: ActionKind::RunScript,
                bundle: ActionBundle::ALL,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "script": {
                            "type": "array",
                            "items": {"type": "object"},
                            "minItems": 1,
                            "maxItems": MAX_SCRIPT_STEPS,
                        }
                    },
                    "required": ["script"],
                    "additionalProperties": false,
                }),
                response_schema: json!({
                    "type": "object",
                    "properties": {
                        "steps": {"type": "array", "items": {"type": "object"}},
                        "result": {"type": ["object", "null"]},
                    },
                    "required": ["steps", "result"],
                    "additionalProperties": false,
                }),
            })
            .unwrap();
        let executor = SequenceExecutor::ok_ref();
        let mut trace = Trace {
            trace_id: "t000097".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let response = issue_action(
            &registry,
            &executor,
            None,
            &order(
                SCRIPT_SERVICE_NAME,
                json!({"script": [
                    {"do": "workspace.read_file", "with": {"path": "a.txt"}, "as": "a"},
                    {"do": "workspace.read_file", "with": {"path": {"$ref": "a.path"}}, "as": "b"},
                    {"do": "workspace.read_file", "with": {"path": {"$ref": "b.path"}}},
                ]}),
            ),
            &mut trace,
            "call-1",
        )
        .await
        .unwrap();
        assert_eq!(response["steps"].as_array().unwrap().len(), 3);
        assert_eq!(response["result"]["content"], "hello");
        let calls = executor.seen.lock().unwrap();
        assert_eq!(calls.len(), 3);
        // 第二/三步的 $ref 已被替换为先序输出字段。
        assert_eq!(calls[1].1, json!({"path": "a.txt"}));
        assert_eq!(calls[2].1, json!({"path": "a.txt"}));
        assert_eq!(calls[2].2, "call-1.s3");
        drop(calls);
        // trace 含逐行 script 事件（3 成功）。
        let script_events: Vec<&TraceEvent> =
            trace.events.iter().filter(|e| e.step == "script").collect();
        assert_eq!(script_events.len(), 3);
        assert!(script_events.iter().all(|e| e.ok));
        assert!(trace.events.iter().any(|e| e.step == STEP_VERIFY));
    }

    /// P0-C S3：静态校验先于任何执行（名称重复/作用域/类型/嵌套/未知服务）。
    #[tokio::test]
    async fn run_script_static_validation_rejects_bad_scripts() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ActionSpec {
                name: "workspace.read_file".to_string(),
                description: "ref test action".to_string(),
                target_tool: Some("read_file".to_string()),
                kind: ActionKind::Host,
                bundle: ActionBundle::ALL,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "nested": {
                            "type": "object",
                            "properties": {"count": {"type": "integer"}},
                        },
                    },
                    "required": ["path"],
                    "additionalProperties": false,
                }),
                response_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"},
                        "count": {"type": "integer"},
                    },
                    "required": ["path", "content"],
                    "additionalProperties": false,
                }),
            })
            .unwrap();
        registry
            .register(ActionSpec {
                name: SCRIPT_SERVICE_NAME.to_string(),
                description: "script action".to_string(),
                target_tool: None,
                kind: ActionKind::RunScript,
                bundle: ActionBundle::ALL,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "script": {
                            "type": "array",
                            "items": {"type": "object"},
                            "minItems": 1,
                            "maxItems": MAX_SCRIPT_STEPS,
                        }
                    },
                    "required": ["script"],
                    "additionalProperties": false,
                }),
                response_schema: json!({
                    "type": "object",
                    "properties": {
                        "steps": {"type": "array", "items": {"type": "object"}},
                        "result": {"type": ["object", "null"]},
                    },
                    "required": ["steps", "result"],
                    "additionalProperties": false,
                }),
            })
            .unwrap();
        let executor = SequenceExecutor::ok_ref();
        let cases: Vec<(Value, &str, &str)> = vec![
            (
                json!({"script": [
                    {"do": "workspace.read_file", "with": {"path": "a.txt"}, "as": "a"},
                    {"do": "workspace.read_file", "with": {"path": "b.txt"}, "as": "a"},
                ]}),
                STEP_CONTRACT,
                CODE_DUPLICATE_STEP_NAME,
            ),
            (
                json!({"script": [
                    {"do": "workspace.read_file", "with": {"path": {"$ref": "z.path"}}},
                ]}),
                STEP_CONTRACT,
                CODE_REFERENCE_SCOPE,
            ),
            (
                json!({"script": [
                    {"do": "workspace.read_file", "with": {"path": "a.txt"}, "as": "a"},
                    {"do": "workspace.read_file", "with": {"path": {"$ref": "a.count"}}},
                ]}),
                STEP_CONTRACT,
                CODE_REFERENCE_TYPE_MISMATCH,
            ),
            (
                json!({"script": [
                    {"do": "workspace.run_script", "with": {"script": []}},
                ]}),
                STEP_CONTRACT,
                CODE_NESTED_SCRIPT,
            ),
            (
                json!({"script": [
                    {"do": "nope.nope", "with": {}},
                ]}),
                STEP_REGISTRY,
                CODE_UNKNOWN_SERVICE,
            ),
        ];
        for (arguments, expected_step, expected_code) in cases {
            let mut trace = Trace {
                trace_id: "t000096".to_string(),
                request_id: None,
                events: Vec::new(),
            };
            let err = run_script(&registry, &executor, None, &arguments, &mut trace, "call-s")
                .await
                .unwrap_err();
            assert_eq!(err.step, expected_step, "{arguments}");
            assert_eq!(err.code, expected_code, "{arguments}");
        }
        assert!(
            executor.seen.lock().unwrap().is_empty(),
            "static validation must fail before any execution"
        );
    }

    /// P0-C S3：脚本任一步 fail-closed——保留内层 step/code、带 script_step，
    /// 后续步骤不执行。
    #[tokio::test]
    async fn run_script_step_failure_stops_and_preserves_inner_error() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ActionSpec {
                name: "workspace.read_file".to_string(),
                description: "ref test action".to_string(),
                target_tool: Some("read_file".to_string()),
                kind: ActionKind::Host,
                bundle: ActionBundle::ALL,
                input_schema: json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"],
                    "additionalProperties": false,
                }),
                response_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"},
                    },
                    "required": ["path", "content"],
                    "additionalProperties": false,
                }),
            })
            .unwrap();
        let executor = SequenceExecutor::new(vec![
            Ok(ref_result()),
            Err(ExecuteError::ExecutionFailed {
                message: "boom".to_string(),
                detail: None,
            }),
        ]);
        let mut trace = Trace {
            trace_id: "t000095".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = run_script(
            &registry,
            &executor,
            None,
            &json!({"script": [
                {"do": "workspace.read_file", "with": {"path": "a.txt"}, "as": "a"},
                {"do": "workspace.read_file", "with": {"path": "b.txt"}, "as": "b"},
                {"do": "workspace.read_file", "with": {"path": "c.txt"}, "as": "c"},
            ]}),
            &mut trace,
            "call-s",
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_EXECUTE);
        assert_eq!(err.code, CODE_EXECUTION_FAILED);
        assert_eq!(err.upstream.as_ref().unwrap()["script_step"], 2);
        assert_eq!(
            err.upstream.as_ref().unwrap()["action"],
            "workspace.read_file"
        );
        // 第三步未执行。
        assert_eq!(executor.seen.lock().unwrap().len(), 2);
        // trace 含 script 失败事件（code 保留内层）。
        let failed: Vec<&TraceEvent> = trace
            .events
            .iter()
            .filter(|e| e.step == "script" && !e.ok)
            .collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].code.as_deref(), Some(CODE_EXECUTION_FAILED));
        assert_eq!(failed[0].upstream.as_ref().unwrap()["script_step"], 2);
    }

    /// P0-C S3：墙钟与累计响应字节上限（测试用小上限注入）。
    #[tokio::test]
    async fn run_script_enforces_wallclock_and_byte_limits() {
        let mut registry = ServiceRegistry::new();
        registry
            .register(ActionSpec {
                name: "workspace.read_file".to_string(),
                description: "ref test action".to_string(),
                target_tool: Some("read_file".to_string()),
                kind: ActionKind::Host,
                bundle: ActionBundle::ALL,
                input_schema: json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"],
                    "additionalProperties": false,
                }),
                response_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"},
                    },
                    "required": ["path", "content"],
                    "additionalProperties": false,
                }),
            })
            .unwrap();
        let executor = SequenceExecutor::ok_ref();
        let arguments = json!({"script": [
            {"do": "workspace.read_file", "with": {"path": "a.txt"}},
        ]});
        let mut trace = Trace {
            trace_id: "t000094".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = run_script_with_limits(
            &registry,
            &executor,
            None,
            &arguments,
            &mut trace,
            "call-s",
            MAX_SCRIPT_STEPS,
            0.0,
            MAX_SCRIPT_RESPONSE_BYTES,
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_EXECUTE);
        assert_eq!(err.code, CODE_SCRIPT_TIMEOUT);
        assert!(executor.seen.lock().unwrap().is_empty());

        let mut trace = Trace {
            trace_id: "t000093".to_string(),
            request_id: None,
            events: Vec::new(),
        };
        let err = run_script_with_limits(
            &registry,
            &executor,
            None,
            &arguments,
            &mut trace,
            "call-s",
            MAX_SCRIPT_STEPS,
            MAX_SCRIPT_WALLCLOCK_SECONDS,
            1,
        )
        .await
        .unwrap_err();
        assert_eq!(err.step, STEP_EXECUTE);
        assert_eq!(err.code, CODE_SCRIPT_RESPONSE_LIMIT);
        assert_eq!(executor.seen.lock().unwrap().len(), 1);
    }

    /// 固定响应序列的执行器（脚本测试用）。
    struct SequenceExecutor {
        results: std::sync::Mutex<std::collections::VecDeque<Result<ToolResult, ExecuteError>>>,
        seen: std::sync::Arc<std::sync::Mutex<Vec<(String, Value, String)>>>,
    }

    impl SequenceExecutor {
        fn new(results: Vec<Result<ToolResult, ExecuteError>>) -> Self {
            Self {
                results: std::sync::Mutex::new(results.into()),
                seen: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        fn ok_ref() -> Self {
            Self::new(vec![Ok(ref_result())])
        }
    }

    #[async_trait]
    impl ActionExecutor for SequenceExecutor {
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
            self.results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(ref_result()))
        }
    }

    fn ref_result() -> ToolResult {
        ToolResult {
            output: "hello".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: Some(json!({"path": "a.txt", "content": "hello"})),
            ..Default::default()
        }
    }
}
