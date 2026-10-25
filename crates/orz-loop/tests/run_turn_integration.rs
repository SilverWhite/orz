//! REV-083-05（2026-09-27，0bv 增量）：orz-loop **集成测试目录**——
//! 经 crate 公开 API（`AgentLoopController::run_turn` ∪ `host::LoopHost`
//! ∪ `gateway::fake::FakeProvider` ∪ `orz_assurance::JournalRecorder`）
//! 端到端驱动 `run_agent_loop` 主链，journal 断言从盘上 `events.jsonl`
//! 回读（与 crate 内单测的 `controller_test_support` 内部通道分离）。
//!
//! 背景：083 全面审查（REV-083-05）指出 run_agent_loop 测试全部住在
//! lib 内部、跨模块行为只有内测视野；拆分面已随 0bs ⑭ 定稿，本目录
//! 承接其「集成测试」增量——后续跨模块回归优先落此处（公开 API 视野）。

use std::path::PathBuf;
use std::sync::Arc;

use orz_assurance::journal::recorder::JournalRecorder;
use orz_loop::controller::AgentLoopController;
use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
use orz_loop::gateway::model::Message;
use orz_loop::host::{
    LoopHost, PermitDecision, PermitError, RiskClass, ToolDef, ToolError, ToolRegistry, ToolResult,
};

/// 最小集成宿主：零工具注册表＋脚本化放行（LoopHost 必需五件套）。
/// P0-1（0dq）：可选检查点记录器——`persist_conversation_checkpoint`
/// 的消息切片快照进 `checkpoints`（None ＝ 不记录）。
struct IntegrationHost {
    journal: JournalRecorder,
    checkpoints: Option<Arc<std::sync::Mutex<Vec<Vec<Message>>>>>,
}

impl IntegrationHost {
    fn new(dir: &std::path::Path) -> Self {
        Self {
            journal: JournalRecorder::new(dir.to_path_buf()),
            checkpoints: None,
        }
    }

    fn with_checkpoints(
        dir: &std::path::Path,
        checkpoints: Arc<std::sync::Mutex<Vec<Vec<Message>>>>,
    ) -> Self {
        Self {
            journal: JournalRecorder::new(dir.to_path_buf()),
            checkpoints: Some(checkpoints),
        }
    }
}

#[async_trait::async_trait]
impl LoopHost for IntegrationHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }

    fn tools_registry(&self) -> &dyn ToolRegistry {
        &EmptyTools
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
        Ok(ToolResult {
            output: "integration-ok".to_string(),
            exit_code: Some(0),
            ..Default::default()
        })
    }

    async fn persist_conversation_checkpoint(
        &self,
        messages: &[Message],
        _lif: &orz_assurance::lif::TemporalSessionSnapshot,
        _blackboard: &orz_loop::blackboard::Blackboard,
    ) {
        if let Some(sink) = &self.checkpoints {
            sink.lock().unwrap().push(messages.to_vec());
        }
    }
}

struct EmptyTools;

impl ToolRegistry for EmptyTools {
    fn get(&self, _name: &str) -> Option<ToolDef> {
        None
    }
    fn list(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}

/// 从盘上回读 journal（集成视野：公开文件形态 `events.jsonl`）。
fn read_journal(dir: &std::path::Path) -> Vec<serde_json::Value> {
    let content = std::fs::read_to_string(dir.join("events.jsonl")).expect("events.jsonl");
    content
        .lines()
        .map(|l| serde_json::from_str(l).expect("journal line json"))
        .collect()
}

fn tempdir() -> PathBuf {
    // Windows SystemTime 粒度粗（同刻并发测试会撞名）——进程内原子计数器
    // 保证唯一（库内 controller_test_support 同形态）。
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("orz-loop-integration-{}-{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 单轮文本 run 的端到端生命周期：prompt → model → 终态 completed，
/// journal 链 first-sequence 无前像、终态收尾。
#[tokio::test]
async fn scripted_text_turn_completes_and_journals_lifecycle() {
    let dir = tempdir();
    let host = IntegrationHost::new(&dir);
    // ORZ-ENV-POLLUTION-001 纪律：狗粮启动 env（ORZ_ACAF_FAIL_CLOSED=1）
    // 会泄入测试进程 ⇒ 集成测试显式装配影子态，不依赖外部 env 干净。
    let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
        ScriptedResponse::text("集成测试回答"),
    ])))
    .with_acaf_fail_closed(false);
    let mut conversation: Vec<Message> = Vec::new();
    // run_turn 返回值＝(最终回答文本, 终态 sequence, previous)；status 在
    // journal 的 run_finished 载荷里（集成视野从盘上核对）。
    let (answer, sequence, _) = controller
        .run_turn(
            &host,
            "集成测试提问",
            "RUN-IT-1",
            &"0".repeat(64),
            0,
            None,
            None,
            Some(&mut conversation),
        )
        .await
        .expect("run_turn completes a scripted text turn");
    assert_eq!(answer, "集成测试回答", "最终回答文本随返回值透出");
    assert!(sequence >= 1);

    let events: Vec<serde_json::Value> = read_journal(&dir)
        .into_iter()
        .filter(|e| e["run_id"] == "RUN-IT-1")
        .collect();
    let types: Vec<&str> = events
        .iter()
        .map(|e| e["event_type"].as_str().expect("event_type"))
        .collect();
    assert_eq!(types.last(), Some(&"run_finished"), "{types:?}");
    assert!(types.contains(&"run_started"), "{types:?}");
    let started = types.iter().position(|t| *t == "run_started").unwrap();
    let prompted = types.iter().position(|t| *t == "prompt_submitted").unwrap();
    let output = types.iter().position(|t| *t == "model_output").unwrap();
    assert!(started < prompted && prompted < output, "{types:?}");
    // 首事件无前像（sequence 0 的链头契约；探针事件可先于 run_started）。
    assert_eq!(
        events[0]["previous_event_sha256"],
        serde_json::Value::Null,
        "链头无前像"
    );
    // 终态 completed。
    assert_eq!(events.last().unwrap()["payload"]["status"], "completed");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 工具调用轮：模型发起 tool_calls → host 执行 → 模型收尾——
/// ToolStarted/ToolCompleted 全序 + tool 结果回灌模型面（跨模块主链）。
#[tokio::test]
async fn scripted_tool_turn_journals_started_completed_pair() {
    use orz_loop::gateway::model::ToolCall;
    let dir = tempdir();
    let host = IntegrationHost::new(&dir);
    let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "grep".to_string(),
            arguments: serde_json::json!({"pattern": "x"}),
            call_id: "call-it-1".to_string(),
        }]),
        // 库内同形先例：工具轮脚本给足三条（工具结果回灌后的消费轮）。
        ScriptedResponse::text("工具结果已消化"),
        ScriptedResponse::text("工具结果已消化"),
    ])))
    .with_acaf_fail_closed(false);
    let mut conversation: Vec<Message> = Vec::new();
    let (answer, _, _) = controller
        .run_turn(
            &host,
            "查一下 x",
            "RUN-IT-2",
            &"0".repeat(64),
            0,
            None,
            None,
            Some(&mut conversation),
        )
        .await
        .expect("run_turn completes a scripted tool turn");
    assert_eq!(answer, "工具结果已消化");

    let events: Vec<serde_json::Value> = read_journal(&dir)
        .into_iter()
        .filter(|e| e["run_id"] == "RUN-IT-2")
        .collect();
    let find = |kind: &str| {
        events
            .iter()
            .position(|e| e["event_type"] == kind)
            .unwrap_or_else(|| panic!("{kind} not journaled"))
    };
    let started = find("tool_started");
    let completed = find("tool_completed");
    let finished = find("run_finished");
    assert!(started < completed, "ToolStarted 先于 ToolCompleted");
    assert!(completed < finished, "工具边界先于终态");
    assert_eq!(events[started]["payload"]["call_id"], "call-it-1");
    let _ = std::fs::remove_dir_all(&dir);
}

/// P0-1（0dq，2026-10-25）：检查点在消息面「provider 一致」边界触发——
/// ①写前：含 tool_calls 的声明入列后、任何工具执行前，首个切片以
/// **悬挂声明**收尾（无任何 Tool 消息）；②回写：结果入列后，切片含
/// 该调用的 tool 回应。跨模块主链：run_agent_loop → LoopHost trait。
#[tokio::test]
async fn conversation_checkpoints_fire_write_ahead_and_write_back() {
    use orz_loop::gateway::model::Role;
    use orz_loop::gateway::model::ToolCall;
    let dir = tempdir();
    let checkpoints: Arc<std::sync::Mutex<Vec<Vec<Message>>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let host = IntegrationHost::with_checkpoints(&dir, checkpoints.clone());
    let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
        ScriptedResponse::tool_calls(vec![ToolCall {
            name: "grep".to_string(),
            arguments: serde_json::json!({"pattern": "x"}),
            call_id: "call-it-ck1".to_string(),
        }]),
        ScriptedResponse::text("工具结果已消化"),
        ScriptedResponse::text("工具结果已消化"),
    ])))
    .with_acaf_fail_closed(false);
    let mut conversation: Vec<Message> = Vec::new();
    let (answer, _, _) = controller
        .run_turn(
            &host,
            "查一下 x",
            "RUN-IT-CK1",
            &"0".repeat(64),
            0,
            None,
            None,
            Some(&mut conversation),
        )
        .await
        .expect("run_turn completes a scripted tool turn");
    assert_eq!(answer, "工具结果已消化");

    let cps = checkpoints.lock().unwrap();
    assert!(
        cps.len() >= 2,
        "写前＋回写至少两次检查点，实际 {} 次",
        cps.len()
    );
    // 写前切片：末条＝悬挂声明（call-it-ck1），且没有任何 Tool 消息——
    // 证明检查点发生在工具执行**之前**（硬杀时该事实已在盘上）。
    let first = &cps[0];
    let last = first.last().expect("write-ahead snapshot non-empty");
    assert_eq!(last.role, Role::Assistant, "写前末条是 assistant 声明");
    assert!(
        last.tool_calls.iter().any(|c| c.call_id == "call-it-ck1"),
        "写前声明携带本轮 call_id: {last:?}"
    );
    assert!(
        !first.iter().any(|m| m.role == Role::Tool),
        "写前切片不含任何工具结果: {first:?}"
    );
    // 回写切片：call-it-ck1 的结果已在列（最后一个 tool_completed 不蒸发）。
    let write_back = cps
        .iter()
        .skip(1)
        .find(|snap| {
            snap.iter()
                .any(|m| m.role == Role::Tool && m.tool_call_id.as_deref() == Some("call-it-ck1"))
        })
        .expect("回写切片含 call-it-ck1 的 tool 结果");
    assert!(write_back.len() > first.len(), "回写切片严格长于写前切片");
    let _ = std::fs::remove_dir_all(&dir);
}

/// P0-1（0dq）车道把关：纯文本轮（无工具调用）**零检查点**——终答轴的
/// 持久化仍由 run 尾 SUCCESS-ONLY 写承担，检查点只在工具轮边界付账，
/// 不给无工具会话增加任何写放大。
#[tokio::test]
async fn text_only_turn_fires_no_conversation_checkpoint() {
    let dir = tempdir();
    let checkpoints: Arc<std::sync::Mutex<Vec<Vec<Message>>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let host = IntegrationHost::with_checkpoints(&dir, checkpoints.clone());
    let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
        ScriptedResponse::text("纯文本回答"),
    ])))
    .with_acaf_fail_closed(false);
    let mut conversation: Vec<Message> = Vec::new();
    let (answer, _, _) = controller
        .run_turn(
            &host,
            "打个招呼",
            "RUN-IT-CK2",
            &"0".repeat(64),
            0,
            None,
            None,
            Some(&mut conversation),
        )
        .await
        .expect("run_turn completes a scripted text turn");
    assert_eq!(answer, "纯文本回答");
    assert!(
        checkpoints.lock().unwrap().is_empty(),
        "纯文本轮不得触发检查点"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
