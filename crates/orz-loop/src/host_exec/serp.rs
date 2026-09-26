//! SERP 预算结算与取证：预算/会话保留码、导航计数、attempt 事实落盘、预算拒绝路径。
//! 0ai (2026-09-16) 拆分自 `host_exec.rs`（机械搬移，行为不变）。

use super::ToolFailureOutcome;
use crate::agent_loop::{SERP_SESSION_RETRIEVAL_FLOOR, SerpSearchBudget};
use crate::controller::{
    AgentLoopController, AgentLoopError, DenialKey, EventWriter, PolicyFeedback,
};
use crate::gateway::model::{Message, Role, ToolCall};
use crate::host::{LoopHost, SerpSessionFacts, ToolResult};
#[cfg(test)]
use crate::host::{PermitDecision, ToolError};
use orz_assurance::EventType;
use serde_json::Value;
use std::sync::Mutex;

/// P2-4 (2026-09-10)：SERP 车道预算耗尽的稳定拒绝码。
pub(crate) const SERP_BUDGET_EXCEEDED_CODE: &str = "browser_control_search_budget_exceeded";
/// P2-3 (2026-09-10)：主车道侵蚀"为检索车道保留的会话额度"时的稳定拒绝码。
pub(crate) const SERP_SESSION_FLOOR_CODE: &str = "browser_control_search_session_reserved";
/// P2-4：SERP 预算门的适用谓词——只对 `browser_control` 的 `search` 动作
/// 计数（navigate/back/forward/refresh/wait_load/snapshot 不消耗 SERP 预算）。
pub(crate) fn is_serp_search_call(name: &str, args: &Value) -> bool {
    name == "browser_control" && args.get("action").and_then(Value::as_str) == Some("search")
}
/// P2-4：从 search 信封读回实际发生的引擎导航数（`status` = ok/failed；
/// `not_attempted` 是本次未触及（0v 第二批软备忘退役 `skipped`）、`pending`
/// 是内部态，两者都不计）。信封不可解析时返回 1——保留派发前的预留（失败
/// 的调用同样占用了 SERP 机会）。
pub(crate) fn serp_navigations_from_output(output: &str) -> u32 {
    let Ok(value) = serde_json::from_str::<Value>(output) else {
        return 1;
    };
    let Some(attempts) = value.get("engine_attempts").and_then(Value::as_array) else {
        return 1;
    };
    let navigated = attempts
        .iter()
        .filter(|attempt| {
            matches!(
                attempt.get("status").and_then(Value::as_str),
                Some("ok") | Some("failed")
            )
        })
        .count() as u32;
    navigated.max(1)
}

/// 0bv（2026-09-26）：`web_search` 浏览器 SERP 车道的**结算读数**——从宿主
/// `ToolResult.structured` 的 `browser_serp.navigations` 读真实引擎导航数
/// （宿主 `structured_from_output` 填充；与 `browser_control search` 的信封
/// 读数是同一口径）。`None` = 本次未触及该车道（资源缺席/浏览器未就绪/零
/// 导航），不记账、不虚构。
pub(crate) fn browser_serp_navigations_from_structured(structured: &Option<Value>) -> Option<u32> {
    let serp = structured.as_ref()?.get("browser_serp")?;
    serp.get("navigations")
        .and_then(Value::as_u64)
        .map(|n| n as u32)
}

impl AgentLoopController {
    /// 0v-A 引擎级取证面（2026-09-12，0v 第二批 S1；设计 §8.6/§8.7）：
    /// 把一次 `browser_control search` 的**引擎级事实**落盘到
    /// `{journal_dir}/serp-attempts/{tool_round}.json`（与
    /// `retrieval-results/` 同形；同轮多次调用顺延 `-2`/`-3` 后缀），
    /// 判据 1/5/7/12 的可事后取证面——run 被墙钟杀死后已落盘文件仍在。
    ///
    /// 内容 = 模型实际收到的**完整信封**（`engine_attempts` 全量含
    /// `not_attempted`、每引擎 `error_class`/`wall_ms`、结果 tier 标注）
    /// 逐字内嵌 + 机械读数（结果计数、`low_quality` 计数、车道预算与会话
    /// 上限读数）。旁路纪律：本面不参与控制流，任何失败只 WARN，绝不影响
    /// 工具结果本身；与 `persist_result_artifact` 同漏斗——落盘前过
    /// orz-secrets 机械脱敏（key 不落卷不变量，0p S2 P1-2）。派发前拒绝
    /// （预算/底线）与宿主错误（浏览器未启动等）没有引擎级事实、不落
    /// 文件——journal 事件面已覆盖这些形态。
    pub(crate) async fn persist_serp_attempts(
        &self,
        writer: &EventWriter<'_>,
        host: &dyn LoopHost,
        serp_budget: Option<&Mutex<SerpSearchBudget>>,
        tool_round: u32,
        tc: &ToolCall,
        output: &str,
    ) {
        // 宿主错误/超时树杀路径的 output 是纯文本，没有信封——无引擎级
        // 事实可取证。
        let Ok(envelope) = serde_json::from_str::<Value>(output) else {
            return;
        };
        if envelope
            .get("engine_attempts")
            .and_then(Value::as_array)
            .is_none()
        {
            return;
        }
        let Some(journal_dir) = writer.journal_dir() else {
            return;
        };
        let results = envelope.get("results").and_then(Value::as_array);
        let results_count = results.map(|a| a.len());
        let low_quality_count = results.map(|a| {
            a.iter()
                .filter(|r| r.get("tier").and_then(Value::as_str) == Some("low_quality"))
                .count()
        });
        // 车道身份按 P2-3 语义从底线标记导出（reserves_session_floor=true
        // = 主车道/grill，false = 外部检索车道）；无预算面（测试/legacy
        // 形态）记 null。
        let lane = serp_budget.map(|b| {
            if b.lock().unwrap().reserves_session_floor() {
                "main"
            } else {
                "external"
            }
        });
        // 结算后读数（含本次调用消耗）。
        let lane_budget = serp_budget.map(|b| {
            let (used, cap) = b.lock().unwrap().usage();
            serde_json::json!({ "used": used, "cap": cap })
        });
        let session = host.serp_session_facts().await.map(|facts| {
            serde_json::json!({
                "navigations": facts.navigations,
                "ceiling": facts.ceiling,
                "floor_reserved": SERP_SESSION_RETRIEVAL_FLOOR,
            })
        });
        // 与宿主侧 SERP_MAX_SEARCH_QUERY_CHARS 同口径的防御性截断。
        let query: String = tc
            .arguments
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .take(500)
            .collect();

        let mut record = serde_json::Map::new();
        record.insert("tool_round".into(), serde_json::json!(tool_round));
        record.insert("lane".into(), serde_json::json!(lane));
        record.insert("query".into(), serde_json::json!(query));
        if let Some(lane_budget) = lane_budget {
            record.insert("lane_budget".into(), lane_budget);
        }
        if let Some(session) = session {
            record.insert("session".into(), session);
        }
        if let Some(n) = results_count {
            record.insert("results_count".into(), serde_json::json!(n));
        }
        if let Some(n) = low_quality_count {
            record.insert("low_quality_count".into(), serde_json::json!(n));
        }
        record.insert("envelope".into(), envelope);

        let dir = journal_dir.join("serp-attempts");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!("serp-attempts dir create failed ({}): {e}", dir.display());
            return;
        }
        let mut path = dir.join(format!("{tool_round:04}.json"));
        let mut suffix = 2;
        while path.exists() {
            path = dir.join(format!("{tool_round:04}-{suffix}.json"));
            suffix += 1;
        }
        match serde_json::to_string_pretty(&Value::Object(record)) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, orz_secrets::redact_secrets(&json).as_bytes())
                {
                    tracing::warn!("serp-attempts write failed ({}): {e}", path.display());
                }
            }
            Err(e) => tracing::warn!("serp-attempts serialize failed: {e}"),
        }
    }

    /// P2-4 (2026-09-10)：SERP 车道预算耗尽的派发前拒绝——与候选门同形
    /// （无 ToolStarted、中性陈述、Denied 反馈进连续拒绝断路器、LIF deny
    /// 通道 + 结构化错误码），单位是引擎导航次数。
    pub(crate) async fn refuse_serp_budget(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tc: &ToolCall,
        msg: &str,
        used: u32,
        cap: u32,
    ) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
        let code = SERP_BUDGET_EXCEEDED_CODE;
        let mut payload = serde_json::json!({
            "tool": tc.name,
            "call_id": tc.call_id,
            "status": "error",
            "error": code,
            "serp_budget_used": used,
            "serp_budget_cap": cap,
        });
        // 0q：拒绝完成同样过单一漏斗（该码不在聚合白名单 → 形状如实；
        // browser_control 非身份可及工具，不落 failure_agg_absent 标记）。
        self.stamp_failure(
            &mut payload,
            &tc.name,
            &tc.arguments,
            ToolFailureOutcome::Refused(code),
        );
        self.feed_lif_deny(None);
        writer.record(EventType::ToolCompleted, payload).await?;
        messages.push(Message {
            role: Role::Tool,
            content: msg.to_string(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        Ok((
            ToolResult {
                output: msg.to_string(),
                exit_code: Some(1),
                output_encoding: None,
                structured: Some(serde_json::json!({
                    "error": code,
                    "serp_budget_used": used,
                    "serp_budget_cap": cap,
                })),
                ..Default::default()
            },
            Some(PolicyFeedback::Denied(DenialKey {
                tool_name: tc.name.clone(),
                reason_code: code.to_string(),
                policy_revision: self.policy_revision(),
            })),
        ))
    }

    /// P2-3（2026-09-10）：主车道侵蚀检索车道会话底线额度的派发前拒绝——
    /// 与 [`Self::refuse_serp_budget`] 同形（无 ToolStarted、中性陈述、
    /// Denied 反馈、LIF deny 通道、结构化字段），但记的是**会话**头寸
    /// （跨车道共享的物理计数器），不是本车道额度。
    pub(crate) async fn refuse_serp_session_floor(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tc: &ToolCall,
        msg: &str,
        facts: SerpSessionFacts,
    ) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
        let code = SERP_SESSION_FLOOR_CODE;
        let mut payload = serde_json::json!({
            "tool": tc.name,
            "call_id": tc.call_id,
            "status": "error",
            "error": code,
            "serp_session_navigations": facts.navigations,
            "serp_session_ceiling": facts.ceiling,
        });
        // 0q：拒绝完成同样过单一漏斗（browser_control 非身份可及工具，
        // 不落 failure_agg_absent 标记）。
        self.stamp_failure(
            &mut payload,
            &tc.name,
            &tc.arguments,
            ToolFailureOutcome::Refused(code),
        );
        self.feed_lif_deny(None);
        writer.record(EventType::ToolCompleted, payload).await?;
        messages.push(Message {
            role: Role::Tool,
            content: msg.to_string(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        Ok((
            ToolResult {
                output: msg.to_string(),
                exit_code: Some(1),
                output_encoding: None,
                structured: Some(serde_json::json!({
                    "error": code,
                    "serp_session_navigations": facts.navigations,
                    "serp_session_ceiling": facts.ceiling,
                })),
                ..Default::default()
            },
            Some(PolicyFeedback::Denied(DenialKey {
                tool_name: tc.name.clone(),
                reason_code: code.to_string(),
                policy_revision: self.policy_revision(),
            })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::host::{PermitError, RiskClass, ToolRegistry};
    use async_trait::async_trait;
    use orz_assurance::EventTrack;
    use orz_assurance::JournalRecorder;
    use std::sync::Arc;

    /// 按调用序返回脚本化结果的宿主——取证面「逐条对应」要求每次 search 的
    /// 信封可不同；`serp_session_facts` 可注入以验证 `session` 机械读数块。
    struct ScriptedSerpHost {
        journal: JournalRecorder,
        results: std::sync::Mutex<std::collections::VecDeque<ToolResult>>,
        facts: Option<SerpSessionFacts>,
    }

    #[async_trait]
    impl LoopHost for ScriptedSerpHost {
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
                .ok_or_else(|| ToolError::NotFound("scripted serp results exhausted".into()))
        }
        async fn serp_session_facts(&self) -> Option<SerpSessionFacts> {
            self.facts
        }
    }

    /// 0v-A 取证面：落盘 JSON 与实际 search 调用**逐条对应**——每次带引擎级
    /// 信封的调用恰一份文件（同轮顺延 `-2` 后缀），`envelope` 与模型实际收到
    /// 的输出逐字同源，引擎名/类别/计数与信封一致；纯文本宿主错误（无信封）
    /// 不落文件。
    #[tokio::test]
    async fn serp_attempts_forensic_files_match_each_search_call() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let envelope_a = serde_json::json!({
            "action": "search",
            "action_status": "ok",
            "engine": "duckduckgo",
            "engine_attempts": [
                {"engine": "google", "status": "failed", "error_class": "network",
                 "reason": "other: connection refused", "wall_ms": 1200},
                {"engine": "bing", "status": "failed", "error_class": "captcha",
                 "reason": "search bing hit CAPTCHA/consent", "wall_ms": 45},
                {"engine": "duckduckgo", "status": "ok", "wall_ms": 30},
            ],
            "results": [
                {"title": "t1", "url": "https://a.example/", "tier": "default", "weight": 1.0},
                {"title": "t2", "url": "https://farm.example/", "tier": "low_quality", "weight": 0.7},
            ],
        });
        let envelope_b = serde_json::json!({
            "action": "search",
            "action_status": "ok",
            "engine": "google",
            "engine_attempts": [
                {"engine": "google", "status": "ok", "wall_ms": 900},
                {"engine": "bing", "status": "not_attempted",
                 "reason": "not attempted in this call: the chain succeeded on an earlier engine"},
                {"engine": "duckduckgo", "status": "not_attempted",
                 "reason": "not attempted in this call: the chain succeeded on an earlier engine"},
            ],
            "results": [],
        });
        let host = ScriptedSerpHost {
            journal: journal.clone(),
            results: std::sync::Mutex::new(std::collections::VecDeque::from(vec![
                ToolResult {
                    output: envelope_a.to_string(),
                    exit_code: Some(0),
                    ..Default::default()
                },
                ToolResult {
                    output: envelope_b.to_string(),
                    exit_code: Some(0),
                    ..Default::default()
                },
                // 宿主错误形态：纯文本、无引擎级事实 → 不落文件。
                ToolResult {
                    output: "browser not running".to_string(),
                    exit_code: Some(1),
                    ..Default::default()
                },
            ])),
            facts: Some(SerpSessionFacts {
                navigations: 9,
                ceiling: 40,
            }),
        };
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let budget = std::sync::Mutex::new(crate::agent_loop::SerpSearchBudget::new(8));
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-SERP-FORENSIC",
            "",
            0,
            None,
            None,
        );
        let call = |i: usize| ToolCall {
            name: "browser_control".to_string(),
            arguments: serde_json::json!({ "action": "search", "query": format!("q{i}") }),
            call_id: format!("serp-{i}"),
        };
        let serp_dir = dir.join("serp-attempts");

        // 调用 1（round 7）：三引擎回退信封 → 0007.json。
        let (result, feedback) = controller
            .run_host_tool_with_plan_gate(
                &host,
                &mut writer,
                &call(0),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                None,
                7,
                None,
                Some("act-1"),
                None,
                Some(&budget),
                false,
                false,
                None,
                None,
                // 0bl 审查修复（2026-09-24）：测试调用面无取消令牌。
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(
            matches!(feedback, Some(PolicyFeedback::Succeeded)),
            "search execution itself must succeed"
        );
        // 逐字同源：模型实际收到的输出 = 落盘内嵌的信封。
        assert_eq!(result.output, envelope_a.to_string());
        let record: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(serp_dir.join("0007.json")).unwrap())
                .unwrap();
        assert_eq!(record["tool_round"], serde_json::json!(7));
        assert_eq!(record["query"], serde_json::json!("q0"));
        assert_eq!(
            record["lane"],
            serde_json::json!("external"),
            "SerpSearchBudget::new 不保留会话底线 = 外部检索车道"
        );
        assert_eq!(record["envelope"], envelope_a);
        // 机械读数与信封逐条对应：结果计数、low_quality 计数、车道预算
        // （预留 1 + 三次导航结算 → used 3/cap 8）、会话事实块。
        assert_eq!(record["results_count"], serde_json::json!(2));
        assert_eq!(record["low_quality_count"], serde_json::json!(1));
        assert_eq!(record["lane_budget"]["used"], serde_json::json!(3));
        assert_eq!(record["lane_budget"]["cap"], serde_json::json!(8));
        assert_eq!(
            record["session"],
            serde_json::json!({"navigations": 9, "ceiling": 40, "floor_reserved": 16}),
        );
        let attempts = record["envelope"]["engine_attempts"].as_array().unwrap();
        assert_eq!(attempts.len(), 3);
        assert_eq!(attempts[0]["engine"], serde_json::json!("google"));
        assert_eq!(attempts[0]["status"], serde_json::json!("failed"));
        assert_eq!(attempts[0]["error_class"], serde_json::json!("network"));
        assert_eq!(attempts[2]["status"], serde_json::json!("ok"));

        // 调用 2（同 round 7）：同轮第二次 search → 顺延 0007-2.json。
        let (result, _) = controller
            .run_host_tool_with_plan_gate(
                &host,
                &mut writer,
                &call(1),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                None,
                7,
                None,
                Some("act-1"),
                None,
                Some(&budget),
                false,
                false,
                None,
                None,
                // 0bl 审查修复（2026-09-24）：测试调用面无取消令牌。
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.output, envelope_b.to_string());
        let record: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(serp_dir.join("0007-2.json")).unwrap())
                .unwrap();
        assert_eq!(record["envelope"], envelope_b);
        assert_eq!(record["results_count"], serde_json::json!(0));
        assert_eq!(record["low_quality_count"], serde_json::json!(0));
        // 结算读数推进：预留 1 + 单导航（成功首引擎）→ used 4/cap 8。
        assert_eq!(record["lane_budget"]["used"], serde_json::json!(4));
        let not_attempted = record["envelope"]["engine_attempts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["status"] == serde_json::json!("not_attempted"))
            .count();
        assert_eq!(not_attempted, 2);

        // 调用 3（round 8）：纯文本宿主错误 → 无信封不落文件。
        let (result, _) = controller
            .run_host_tool_with_plan_gate(
                &host,
                &mut writer,
                &call(2),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                None,
                8,
                None,
                Some("act-1"),
                None,
                Some(&budget),
                false,
                false,
                None,
                None,
                // 0bl 审查修复（2026-09-24）：测试调用面无取消令牌。
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.output, "browser not running");

        // 文件集恰为两次带信封的调用；journal 侧 browser_control 完成事件
        // 同为三次（调用↔事件↔文件的三面对应）。
        let mut files: Vec<String> = std::fs::read_dir(&serp_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(files, vec!["0007-2.json", "0007.json"]);
        let journaled = events(&dir)
            .into_iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("browser_control")
            })
            .count();
        assert_eq!(journaled, 3, "every call is journaled");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0v-A 取证面旁路纪律：落盘失败（目录被同名普通文件占位）只 WARN，
    /// **不影响工具结果本身**——信封逐字返回、退出码与反馈不变、占位文件
    /// 原样保留。
    #[tokio::test]
    async fn serp_attempts_write_failure_does_not_affect_tool_result() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let blocker = dir.join("serp-attempts");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let envelope = serde_json::json!({
            "action": "search",
            "action_status": "ok",
            "engine": "bing",
            "engine_attempts": [
                {"engine": "google", "status": "failed", "error_class": "network", "wall_ms": 10},
                {"engine": "bing", "status": "ok", "wall_ms": 20},
                {"engine": "duckduckgo", "status": "not_attempted",
                 "reason": "not attempted in this call: the chain succeeded on an earlier engine"},
            ],
            "results": [],
        });
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: envelope.to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(host.journal()),
            EventTrack::V02,
            "RUN-SERP-FORENSIC-BLOCKED",
            "",
            0,
            None,
            None,
        );
        let call = ToolCall {
            name: "browser_control".to_string(),
            arguments: serde_json::json!({ "action": "search", "query": "q" }),
            call_id: "serp-blocked".to_string(),
        };
        let (result, feedback) = controller
            .run_host_tool_with_plan_gate(
                &host,
                &mut writer,
                &call,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                None,
                3,
                None,
                Some("act-1"),
                None,
                None,
                false,
                false,
                None,
                None,
                // 0bl 审查修复（2026-09-24）：测试调用面无取消令牌。
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.output, envelope.to_string(), "envelope untouched");
        assert_eq!(result.exit_code, Some(0));
        assert!(matches!(feedback, Some(PolicyFeedback::Succeeded)));
        assert!(blocker.is_file(), "placeholder file must be untouched");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0v-A 取证面脱敏漏斗钉字（2026-09-12 复审 P2 项收口）：落盘前
    /// `orz_secrets::redact_secrets` 确实运行——信封内含敏感查询参数与
    /// `password =` 赋值形态时，**落盘文件脱敏、模型实际收到的输出原样**；
    /// 这同时把「内容逐字段同源、脱敏命中时不逐字」的精确口径钉进机械面。
    #[tokio::test]
    async fn serp_attempts_forensic_funnel_redacts_secrets_without_touching_model_output() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let secret_url = "https://farm.example/page?token=supersecretvalue&x=1";
        let envelope = serde_json::json!({
            "action": "search",
            "action_status": "ok",
            "engine": "bing",
            "engine_attempts": [
                {"engine": "google", "status": "failed", "error_class": "network", "wall_ms": 10},
                {"engine": "bing", "status": "ok", "wall_ms": 20},
                {"engine": "duckduckgo", "status": "not_attempted",
                 "reason": "not attempted in this call: the chain succeeded on an earlier engine"},
            ],
            "results": [
                {"title": "t", "url": secret_url,
                 "snippet": "docs mention password=hunter2secret in passing"},
            ],
        });
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: envelope.to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(host.journal()),
            EventTrack::V02,
            "RUN-SERP-FORENSIC-REDACT",
            "",
            0,
            None,
            None,
        );
        let call = ToolCall {
            name: "browser_control".to_string(),
            arguments: serde_json::json!({ "action": "search", "query": "q" }),
            call_id: "serp-redact".to_string(),
        };
        let (result, _) = controller
            .run_host_tool_with_plan_gate(
                &host,
                &mut writer,
                &call,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                None,
                4,
                None,
                Some("act-1"),
                None,
                None,
                false,
                false,
                None,
                None,
                // 0bl 审查修复（2026-09-24）：测试调用面无取消令牌。
                None,
            )
            .await
            .unwrap();

        // 模型实际收到的输出不脱敏。
        assert!(
            result.output.contains("supersecretvalue"),
            "model output must not be redacted: {}",
            result.output
        );
        // 落盘文件脱敏：原始秘密值不得存活，且至少出现一种脱敏标记
        // （URL 参数值 → redacted / [REDACTED_SECRET]；赋值形态 →
        // [REDACTED_SECRET]——两族 regex 的叠加次序可能让 URL 值最终落到
        // 任一形态，故按不变量断言而非钉死单一形态）。
        let record = std::fs::read_to_string(dir.join("serp-attempts").join("0004.json")).unwrap();
        assert!(
            !record.contains("supersecretvalue") && !record.contains("hunter2secret"),
            "raw secrets must not survive the forensic funnel: {record}"
        );
        assert!(
            record.contains("REDACTED"),
            "at least one redaction marker must be present on disk: {record}"
        );
        assert!(
            record.contains("farm.example") && record.contains("docs mention"),
            "non-secret content survives the funnel: {record}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0v-A 取证面无预算面形态（2026-09-12 复审 P3 项收口）：`serp_budget=None`
    /// 时 `lane` 记 null、不写 `lane_budget` 键——取证文件在 legacy/测试形态下
    /// 不虚构造数。
    #[tokio::test]
    async fn serp_attempts_lane_is_null_without_budget_surface() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let envelope = serde_json::json!({
            "action": "search",
            "action_status": "ok",
            "engine": "google",
            "engine_attempts": [
                {"engine": "google", "status": "ok", "wall_ms": 5},
                {"engine": "bing", "status": "not_attempted",
                 "reason": "not attempted in this call: the chain succeeded on an earlier engine"},
                {"engine": "duckduckgo", "status": "not_attempted",
                 "reason": "not attempted in this call: the chain succeeded on an earlier engine"},
            ],
            "results": [],
        });
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: envelope.to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(host.journal()),
            EventTrack::V02,
            "RUN-SERP-FORENSIC-NOBUDGET",
            "",
            0,
            None,
            None,
        );
        let call = ToolCall {
            name: "browser_control".to_string(),
            arguments: serde_json::json!({ "action": "search", "query": "q" }),
            call_id: "serp-nobudget".to_string(),
        };
        let (result, _) = controller
            .run_host_tool_with_plan_gate(
                &host,
                &mut writer,
                &call,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                None,
                5,
                None,
                Some("act-1"),
                None,
                None,
                false,
                false,
                None,
                None,
                // 0bl 审查修复（2026-09-24）：测试调用面无取消令牌。
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        let record: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("serp-attempts").join("0005.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(record["lane"], serde_json::Value::Null);
        assert!(
            record.get("lane_budget").is_none(),
            "no budget surface → no lane_budget key: {record}"
        );
        assert_eq!(record["envelope"], envelope);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
