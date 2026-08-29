//! Shared controller test scaffolding — N5-0 of the controller split
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29 §3.5): moved verbatim from
//! controller.rs mod tests; mechanical extraction only — behavior,
//! events and journal chain unchanged.

use orz_assurance::{EventType, JournalRecorder, RunEvent};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;

use crate::controller::{AgentLoopController, RetrievalCapability, RetrievalMode};
use crate::gateway::model::ToolCall;
use crate::host::{
    LoopHost, PermitDecision, PermitError, RiskClass, ToolDef, ToolError, ToolRegistry, ToolResult,
};

/// THIN-HARNESS-REDESIGN R2a 审查处理 (P2-1, 2026-08-27)：env 突变
/// 测试共享同一把锁——此前 `retrieval_dispatch_inline_channel_keeps_
/// full_text` 与 `retrieval_result_channel_env_parse_rules` 各自声明
/// 函数级 ENV_LOCK（互不排斥），且默认指针摘要测试无锁读取同一 env；
/// inline 测试持 env=inline 跨整个 async run_turn 期间，并行测试可能
/// 读到 inline 导致断言 flake。三个测试统一持本锁（读方也持锁排除
/// 突变窗口；std::sync::MutexGuard 跨 await 仅对 current_thread
/// 测试运行时成立，本文件 tokio::test 默认即此）。
pub(crate) static RETRIEVAL_CHANNEL_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Minimal LoopHost for testing the controller.
pub(crate) struct TestHost {
    pub(crate) journal: JournalRecorder,
    pub(crate) tool_result: Option<ToolResult>,
}

pub(crate) struct EmptyRegistry;
impl ToolRegistry for EmptyRegistry {
    fn get(&self, _name: &str) -> Option<ToolDef> {
        None
    }
    fn list(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}

#[async_trait]
impl LoopHost for TestHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }
    fn tools_registry(&self) -> &dyn ToolRegistry {
        &EmptyRegistry
    }
    // P0-D S3: summary archives are written under session_cwd/.gsa/ —
    // point the shared test host at the per-test temp journal dir so
    // successful summaries never pollute the crate workspace.
    fn session_cwd(&self) -> std::path::PathBuf {
        self.journal.journal_dir().to_path_buf()
    }
    // Explicit override — the trait default is fail-closed Deny; the tool
    // round-trip tests need an authorized host.
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
        self.tool_result
            .clone()
            .ok_or_else(|| ToolError::NotFound("test host has no tool result".into()))
    }
}

/// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2/§2.3): a test host with a
/// stateful workspace-snapshot queue — the first `workspace_snapshot`
/// call serves as the plan-approval baseline, later calls serve as the
/// live snapshots at `submit`; plus a fixed tool result for order
/// execution.
pub(crate) struct DeliveryHost {
    pub(crate) journal: JournalRecorder,
    pub(crate) tool_result: ToolResult,
    pub(crate) snapshots:
        std::sync::Mutex<VecDeque<std::collections::HashMap<String, (u64, u64, u32)>>>,
}

#[async_trait]
impl LoopHost for DeliveryHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }
    fn tools_registry(&self) -> &dyn ToolRegistry {
        &EmptyRegistry
    }
    fn session_cwd(&self) -> std::path::PathBuf {
        self.journal.journal_dir().to_path_buf()
    }
    fn workspace_snapshot(&self) -> Option<std::collections::HashMap<String, (u64, u64, u32)>> {
        self.snapshots.lock().unwrap().pop_front()
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
        Ok(self.tool_result.clone())
    }
}

/// P0-C S2 (2026-08-15): a host that denies every permission — used to
/// verify the console adapter maps permission denials to
/// `step=policy` / `code=policy_denied`.
pub(crate) struct DenyHost {
    pub(crate) journal: JournalRecorder,
}

#[async_trait]
impl LoopHost for DenyHost {
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
        risk: RiskClass,
        _tool: &str,
        _args: &serde_json::Value,
    ) -> Result<PermitDecision, PermitError> {
        // Real-host semantics: read-class tools auto-allow (console
        // action_write included); mutations are denied.
        if risk == RiskClass::ReadOnly {
            Ok(PermitDecision::AllowOnce)
        } else {
            Ok(PermitDecision::Deny)
        }
    }
    async fn call_tool(
        &self,
        _name: &str,
        _args: serde_json::Value,
        _call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::NotFound("deny host never executes".into()))
    }
}

/// PLAN-FIRST 阶段 C (2026-08-16): per-call scripted host — a queue of
/// results so a single run can mix failing orders and succeeding direct
/// calls (the plain TestHost returns one result for every call).
pub(crate) struct QueueHost {
    pub(crate) journal: JournalRecorder,
    pub(crate) results: Mutex<VecDeque<Result<ToolResult, ToolError>>>,
}

#[async_trait]
impl LoopHost for QueueHost {
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
        self.results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(ToolError::NotFound("queue host exhausted".into())))
    }
}

pub(crate) fn fail_result() -> ToolResult {
    ToolResult {
        output: "host failed".to_string(),
        exit_code: None,
        output_encoding: None,
        structured: None,
        ..Default::default()
    }
}

pub(crate) fn ok_result() -> ToolResult {
    ToolResult {
        output: "ok".to_string(),
        exit_code: Some(0),
        output_encoding: None,
        structured: None,
        ..Default::default()
    }
}

pub(crate) static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn test_dir() -> PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("orz-controller-test-{}-{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub(crate) const MANIFEST: &str = "abcd-manifest-sha-64chars-long_____________________";

pub(crate) fn tool_call(name: &str, call_id: &str) -> ToolCall {
    ToolCall {
        name: name.to_string(),
        arguments: serde_json::json!({"query": "test"}),
        call_id: call_id.to_string(),
    }
}

/// FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): a web_fetch call with
/// a real URL argument (the count gate's identity).
pub(crate) fn web_fetch_call(call_id: &str, url: &str) -> ToolCall {
    ToolCall {
        name: "web_fetch".to_string(),
        arguments: serde_json::json!({ "url": url }),
        call_id: call_id.to_string(),
    }
}

/// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): a `plan_write` tool
/// call carrying the structured plan object (design §5).
pub(crate) fn plan_write_call(call_id: &str, plan: serde_json::Value) -> ToolCall {
    ToolCall {
        name: "plan_write".to_string(),
        arguments: serde_json::json!({ "plan": plan }),
        call_id: call_id.to_string(),
    }
}

pub(crate) fn valid_plan_json() -> serde_json::Value {
    serde_json::json!({
        "plan_id": "plan-1",
        "goal": "修复缓存回归",
        "steps": [
            {
                "id": "s1",
                "goal": "复现问题",
                "actions": [
                    {"step_id": "s1", "do": "workspace.read_file", "with": {"path": "src/cache.rs"}}
                ],
                "acceptance": "已定位回归点",
                "evidence": ["src/cache.rs"]
            },
            {
                "id": "deliver",
                "goal": "实施修复",
                "actions": [
                    {"step_id": "deliver", "do": "workspace.search_replace", "with": {"path": "src/cache.rs"}}
                ],
                "acceptance": "修复已落地",
                "evidence": ["src/cache.rs"]
            }
        ]
    })
}

pub(crate) fn invalid_plan_json() -> serde_json::Value {
    serde_json::json!({ "plan_id": "plan-bad", "goal": "", "steps": [] })
}

/// FUS-RETRIEVAL-MECH P0-B step 4 (2026-08-14): a browser_read call
/// with a real URL argument (the shared candidate count identity).
pub(crate) fn browser_read_call(call_id: &str, url: &str) -> ToolCall {
    ToolCall {
        name: "browser_read".to_string(),
        arguments: serde_json::json!({ "url": url }),
        call_id: call_id.to_string(),
    }
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): the retrieval tests run under an
/// explicit `framework_fallback` mode with an available capability —
/// the bare `with_gateway` default is mode=off (ADR-0010 §3.7.1).
pub(crate) fn with_retrieval_enabled(controller: AgentLoopController) -> AgentLoopController {
    controller.with_retrieval_mode(
        RetrievalMode::FrameworkFallback,
        RetrievalCapability::Available,
        false,
        None,
        None,
        None,
    )
}

/// FUS-RETRIEVAL-MECH P0-B step 4 (2026-08-14): the browser_read
/// tests run under an explicit `local_browser` mode with an available
/// capability (browser_read's mode gate requires it).
pub(crate) fn with_local_browser_enabled(controller: AgentLoopController) -> AgentLoopController {
    controller.with_retrieval_mode(
        RetrievalMode::LocalBrowser,
        RetrievalCapability::Available,
        false,
        None,
        None,
        None,
    )
}

pub(crate) fn events(dir: &Path) -> Vec<RunEvent> {
    let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
    content
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

pub(crate) fn event_types(dir: &Path) -> Vec<EventType> {
    events(dir).into_iter().map(|e| e.event_type).collect()
}
