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

use std::collections::{BTreeMap, VecDeque};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::blackboard::ActionOrder;
use crate::host::{ToolError, ToolResult};

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
pub const CODE_INTERNAL_ERROR: &str = "internal_error";

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
    /// Append one bounded event; events past `TRACE_MAX_EVENTS` are dropped.
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
            return;
        }
        self.events.push(TraceEvent {
            seq: self.events.len() + 1,
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

    /// Allocate the next trace id and retain the trace (bounded).
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
    /// 对 `ToolResult.structured`（无则 `{"output": ...}`）做机械验证。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    Duplicate(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Duplicate(name) => write!(f, "duplicate service: {name}"),
        }
    }
}

/// HA 式服务注册表：动作名 → 契约 + 目标工具，确定性、无模型参与。
#[derive(Debug, Clone, Default)]
pub struct ServiceRegistry {
    actions: BTreeMap<String, ActionSpec>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, spec: ActionSpec) -> Result<(), RegistryError> {
        if self.actions.contains_key(&spec.name) {
            return Err(RegistryError::Duplicate(spec.name));
        }
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
                parameters: spec.input_schema.clone(),
            })
            .collect()
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
    ) -> Result<ToolResult, ToolError>;
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

/// 发放一个动作订单：注册表路由 → 契约校验 → 目标解析 → 执行 → 响应验证。
///
/// 每一步成功后追加 trace 事件；失败返回 `ConsoleError`（调用方负责在 trace
/// 上追加失败事件并构造信封——与 POC `run_request` 的失败收口一致）。
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

    validate_arguments(&spec.input_schema, &order.arguments).map_err(|msg| ConsoleError {
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
        .map_err(|err| ConsoleError {
            step: STEP_EXECUTE,
            code: CODE_EXECUTION_FAILED,
            message: err.to_string(),
            upstream: Some(json!({
                "target_tool": spec.target_tool,
                "action": spec.name,
            })),
        })?;
    if result.exit_code != Some(0) {
        return Err(ConsoleError {
            step: STEP_EXECUTE,
            code: CODE_EXECUTION_FAILED,
            message: "target tool returned a non-zero exit code".to_string(),
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
    if let Some(schema) = &spec.response_schema {
        validate_response(schema, &response).map_err(|msg| ConsoleError {
            step: STEP_VERIFY,
            code: CODE_INVALID_RESPONSE,
            message: format!("{}: response failed verification: {msg}", spec.name),
            upstream: Some(json!({
                "action": spec.name,
                "target_tool": spec.target_tool,
            })),
        })?;
    }
    trace.add(STEP_VERIFY, Some(&order.action), true, None, None, None);

    Ok(response)
}

fn validate_arguments(schema: &Value, instance: &Value) -> Result<(), String> {
    let validator = jsonschema::validator_for(schema).map_err(|e| e.to_string())?;
    validator.validate(instance).map_err(|e| e.to_string())
}

fn validate_response(schema: &Value, instance: &Value) -> Result<(), String> {
    let validator = jsonschema::validator_for(schema).map_err(|e| e.to_string())?;
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
            response_schema: Some(json!({
                "type": "object",
                "properties": {
                    "output": {"type": "string"},
                },
                "required": ["output"],
            })),
        }
    }

    fn order(action: &str, arguments: Value) -> ActionOrder {
        ActionOrder {
            order_id: "ORD-1".to_string(),
            action: action.to_string(),
            arguments,
            round: 1,
            plan_epoch: 1,
        }
    }

    struct FakeExecutor {
        result: Result<ToolResult, String>,
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
                result: Err("boom".to_string()),
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
        ) -> Result<ToolResult, ToolError> {
            self.seen.lock().unwrap().push((
                target_tool.to_string(),
                arguments.clone(),
                call_id.to_string(),
            ));
            self.result.clone().map_err(ToolError::NotFound)
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
        trace.add(
            err.step,
            Some("workspace.read_file"),
            false,
            Some(err.code),
            Some(err.message.clone()),
            err.upstream.clone(),
        );
        let envelope = error_envelope(&err, &trace, 10);
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
