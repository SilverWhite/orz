//! 宿主事实 → journal 事件：browser launch / 进程树回收 / 宿主资源事实 / idle kill 补账。
//! 0ai (2026-09-16) 拆分自 `host_exec.rs`（机械搬移，行为不变）。

use crate::controller::{AgentLoopController, AgentLoopError, EventWriter};
#[cfg(test)]
use crate::gateway::model::{Message, ToolCall};
use crate::host::LoopHost;
#[cfg(test)]
use crate::host::{PermitDecision, ToolError, ToolResult};
use orz_assurance::EventType;
use serde_json::Value;
#[cfg(test)]
use std::sync::Mutex;

/// 0z S2 §5（2026-09-12，FUS-HOST-RESOURCE-SAFETY）宿主资源事实 → journal 事件的
/// **唯一映射表**。生产者全集见 `orz-host` 的 `resource_facts` push 点
/// （run_start / tier_change 快照、回收、资源耗尽 planned+executed、pre-issue 拒绝）
/// 与设计文档 [`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`] §5 的事件表。
///
/// **GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP（2026-09-13）**：0.5.0 中本表缺
/// `host_resource_snapshot` 一项，导致 run_start 与跨档读数在 drain 时被丢弃
/// （`unknown host resource fact kind; dropped (audit-face loss)`；实测 6 run
/// 13 次 WARN、journal 0 事件）。新增宿主事实时**必须同批在本表落行**，并由
/// `host_resource_fact_table_covers_producer_kinds` 钉子守住覆盖性与族名一致性。
pub(crate) const HOST_RESOURCE_FACT_EVENT_TYPES: &[(&str, EventType)] = &[
    ("host_resource_snapshot", EventType::HostResourceSnapshot),
    ("reclaim_performed", EventType::ReclaimPerformed),
    ("resource_exhausted", EventType::ResourceExhausted),
    ("host_resource_denied", EventType::HostResourceDenied),
    ("resource_limit_hit", EventType::ResourceLimitHit),
];

impl AgentLoopController {
    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.3): 落 `browser_launch_result`
    /// 事实事件——必须在 ToolStarted 之后、对应 ToolCompleted 之前调用。
    /// 事件不带 args、key 不落卷纪律不变。
    pub(crate) async fn journal_browser_launch(
        &self,
        writer: &mut EventWriter<'_>,
        fact: &crate::host::BrowserLaunchFact,
    ) -> Result<(), AgentLoopError> {
        let suffix = writer
            .run_id()
            .strip_prefix("RUN-")
            .unwrap_or(writer.run_id());
        let status = match fact.status {
            crate::host::BrowserLaunchStatus::Success => "success",
            crate::host::BrowserLaunchStatus::Failure => "failure",
        };
        writer
            .record(
                EventType::BrowserLaunchResult,
                serde_json::json!({
                    "attempt_id": format!("BLAUNCH-{}-{:04}", suffix, writer.seq()),
                    "status": status,
                    "cause": fact.cause,
                }),
            )
            .await
    }

    /// 0z S2 §4.2（2026-09-12，FUS-HOST-RESOURCE-SAFETY）：进程树扫除的
    /// journal 面——宿主扫除器的 planned/executed 行落 `process_tree_reaped`
    /// 事件（审计先行：planned 行先于任何 kill 落盘）。run 收尾 drain 一次。
    pub(crate) async fn journal_pending_process_tree_reaps(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
    ) -> Result<(), AgentLoopError> {
        let facts = host.drain_process_tree_reap_facts().await;
        for fact in facts {
            writer
                .record(
                    EventType::ProcessTreeReaped,
                    serde_json::json!({
                        "phase": fact.phase,
                        "reason": fact.reason,
                        "pids": fact.pids,
                        "call_ids": fact.call_ids,
                    }),
                )
                .await?;
        }
        Ok(())
    }

    /// 0z S2 §5（2026-09-12，FUS-HOST-RESOURCE-SAFETY）：宿主资源事实的
    /// journal 面——`host_resource_snapshot`（run_start 一次 + 跨档 tier_change）/
    /// `reclaim_performed`（审计先行由宿主保证）/
    /// `resource_exhausted`（hard 档 planned/executed）/ `host_resource_denied`
    /// （pre-issue 拒绝，含读数与动作分档）/ `resource_limit_hit`（Job 硬上限）。
    /// run 收尾 drain 一次。
    ///
    /// **纪律（GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP，2026-09-13）**：本表是
    /// `orz-host` → journal 的**唯一映射**；宿主每新增一种
    /// `resource_facts` 事件名，本表必须同批落行，否则该事实 drain 时被丢弃，
    /// 而 journal 只会留一行 `unknown host resource fact kind; dropped
    /// (audit-face loss)` —— 这正是 0.5.0 中 `host_resource_snapshot` 的实例
    /// （6 个 run 13 次 WARN、journal 0 事件）。覆盖性由
    /// `host_resource_fact_table_covers_producer_kinds` 钉子守住。
    pub(crate) async fn journal_pending_host_resource_facts(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
    ) -> Result<(), AgentLoopError> {
        use HOST_RESOURCE_FACT_EVENT_TYPES as EVENT_TYPE_BY_FACT;
        for fact in host.drain_host_resource_facts().await {
            let Some(kind) = fact.get("event").and_then(Value::as_str) else {
                // review F-EV-11: an unlabelled fact is an audit-face loss —
                // never fail-open silently.
                tracing::warn!(
                    fact = %fact,
                    "host resource fact without an `event` label; dropped"
                );
                continue;
            };
            let Some((_, event_type)) = EVENT_TYPE_BY_FACT.iter().find(|(k, _)| *k == kind) else {
                tracing::warn!(
                    kind,
                    "unknown host resource fact kind; dropped (audit-face loss)"
                );
                continue;
            };
            let mut payload = fact.clone();
            payload.as_object_mut().map(|o| o.remove("event"));
            writer.record(event_type.clone(), payload).await?;
        }
        Ok(())
    }

    /// TER 全面审查 P1-1 (2026-09-04)：idle-kill `tool_running` 事件生产者。
    /// host 在工具执行 / run 收尾边界 drain `LoopHost::drain_terminal_idle_kills`；
    /// loop 只对**本 run 内已记过 mid-run `tool_running`** 的 auto-bg 调用补记
    /// `tool_running(status=idle_killed + reason)`（T0.2 §4 链规则：晚于该调用
    /// 的 `running:true` `tool_completed`、每 call_id 至多一次、不引入第二个
    /// `tool_completed`）。跨 run 复用 call_id 不会串链（按 run_id 过滤）。
    pub(crate) async fn journal_pending_idle_kills(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
    ) -> Result<(), AgentLoopError> {
        let run_id = writer.run_id().to_string();
        let facts = host.drain_terminal_idle_kills().await;
        if facts.is_empty() {
            return Ok(());
        }
        let eligible = {
            let guard = self.mid_run_call_ids.lock().unwrap();
            facts
                .into_iter()
                .filter(|f| guard.get(&f.task_id).map(String::as_str) == Some(run_id.as_str()))
                .collect::<Vec<_>>()
        };
        for fact in eligible {
            let mut payload = serde_json::json!({
                "tool": "run_terminal_cmd",
                "call_id": fact.task_id,
                "task_id": fact.task_id,
                "total_bytes": fact.total_bytes,
                "output_file": fact.output_file,
                "wall_ms": fact.wall_ms,
                "status": "idle_killed",
                "reason": fact.reason,
            });
            if let Some(pid) = fact.pid {
                payload["pid"] = serde_json::json!(pid);
            }
            writer.record(EventType::ToolRunning, payload).await?;
            tracing::info!(
                run_id,
                task_id = %fact.task_id,
                "journaled idle-killed tool_running lifecycle event"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use crate::host::{BrowserLaunchFact, PermitError, RiskClass, ToolRegistry};
    use async_trait::async_trait;
    use orz_assurance::EventTrack;
    use orz_assurance::JournalRecorder;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP 钉子（2026-09-13）：宿主资源事实
    /// 判定表必须覆盖**生产侧全集**，且每个键名都必须与 `orz-assurance` 的族
    /// 注册表同名（防拼写漂移）。0.5.0 缺 `host_resource_snapshot` 一项 ⇒
    /// run_start 与跨档读数全被丢弃（6 run 13 次 WARN、journal 0 事件）。
    #[test]
    fn host_resource_fact_table_covers_producer_kinds() {
        // 生产侧全集 = `orz-host` 的 `resource_facts` push 点 ∪ 设计文档 §5 事件表。
        const PRODUCER_KINDS: &[&str] = &[
            "host_resource_snapshot",
            "reclaim_performed",
            "resource_exhausted",
            "host_resource_denied",
            "resource_limit_hit",
        ];
        for kind in PRODUCER_KINDS {
            assert!(
                HOST_RESOURCE_FACT_EVENT_TYPES
                    .iter()
                    .any(|(k, _)| k == kind),
                "宿主资源事实 `{kind}` 不在映射表内 ⇒ drain 时会被丢弃（audit-face loss）"
            );
        }
        for (kind, _event_type) in HOST_RESOURCE_FACT_EVENT_TYPES {
            assert!(
                orz_assurance::journal::ALL_FAMILIES.contains(kind),
                "映射表项 `{kind}` 不在 orz-assurance 族注册表内（拼写漂移）"
            );
        }
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：run_terminal_cmd
    /// 自动后台化（mid_run）→ 控制器在 ToolStarted 与 ToolCompleted 之间
    /// 记一条 `tool_running`（wall_ms/pid/total_bytes/output_file/task_id），
    /// 随后 ToolCompleted 带 `running: true` 且 exit_code=null；模型下一轮
    /// 收到中间状态文本。
    #[tokio::test]
    async fn mid_run_result_journals_tool_running_and_running_completed() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        struct MidRunOnceHost {
            journal: JournalRecorder,
            calls: AtomicU64,
        }
        #[async_trait::async_trait]
        impl LoopHost for MidRunOnceHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            async fn call_tool(
                &self,
                _name: &str,
                _arguments: serde_json::Value,
                _call_id: &str,
            ) -> Result<ToolResult, ToolError> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Ok(ToolResult {
                        output: "[Command still running after 300s] PID: 1234 ...".to_string(),
                        exit_code: None,
                        output_encoding: None,
                        structured: None,
                        mid_run: Some(crate::host::ToolMidRunStatus {
                            task_id: "call-t1".to_string(),
                            pid: Some(1234),
                            output_file: "/tmp/terminal/call-t1.log".to_string(),
                            total_bytes: Some(8192),
                        }),
                        ..Default::default()
                    })
                } else {
                    Ok(ToolResult {
                        output: "retry ok".to_string(),
                        exit_code: Some(0),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    })
                }
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
        let host = MidRunOnceHost {
            journal,
            calls: AtomicU64::new(0),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("run_terminal_cmd", "call-t1")]),
            ScriptedResponse::text("结果：完成"),
            ScriptedResponse::text("结果：完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(
                &host,
                "测试中间回报",
                "RUN-MIDRUN",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok(), "{result:?}");

        let events = events(&dir);
        let running: Vec<serde_json::Value> = events
            .iter()
            .filter(|e| e.event_type == EventType::ToolRunning)
            .map(|e| e.payload.clone())
            .collect();
        assert_eq!(running.len(), 1, "exactly one tool_running");
        assert_eq!(running[0]["tool"], "run_terminal_cmd");
        assert_eq!(running[0]["call_id"], "call-t1");
        assert_eq!(running[0]["task_id"], "call-t1");
        assert_eq!(running[0]["pid"], serde_json::json!(1234));
        assert_eq!(running[0]["total_bytes"], serde_json::json!(8192));
        assert_eq!(running[0]["output_file"], "/tmp/terminal/call-t1.log");
        assert!(
            running[0]["wall_ms"].as_u64().is_some(),
            "wall_ms present on tool_running: {}",
            running[0]
        );

        let completed: Vec<serde_json::Value> = events
            .iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload.clone())
            .collect();
        assert_eq!(completed.len(), 1, "exactly one ToolCompleted");
        assert_eq!(completed[0]["running"], serde_json::json!(true));
        assert!(completed[0]["exit_code"].is_null());
        assert!(
            completed[0]["wall_ms"].as_u64().is_some(),
            "wall_ms present: {}",
            completed[0]
        );

        // 模型下一轮收到中间状态文本（Tool 消息内容）。
        let received = fake.received_requests();
        assert!(received.len() >= 2, "{received:?}");
        let round2 = &received[1].messages;
        let tool_msg = round2
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-t1"));
        assert!(tool_msg.is_some(), "tool reply present: {round2:?}");
        assert!(
            tool_msg
                .unwrap()
                .content
                .contains("still running after 300s"),
            "mid-run report text: {}",
            tool_msg.unwrap().content
        );

        let replay = orz_assurance::replay_journal(
            &dir.join("events.jsonl"),
            Some("RUN-MIDRUN"),
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TER 全面审查 P1-1 (2026-09-04)：idle-kill `tool_running` 事件生产者
    /// 闭环——先 auto-bg（mid-run `tool_running` + `running:true`
    /// `tool_completed`），后续工具边界 host drain 返回同一任务 idle-kill
    /// 事实时，loop 补记 `tool_running(status=idle_killed + reason)`；事件
    /// 晚于原调用完成事件、每 call_id 至多一次。
    #[tokio::test]
    async fn idle_kill_after_mid_run_journals_lifecycle_tool_running() {
        use std::sync::atomic::{AtomicU64, Ordering};

        struct IdleKillHost {
            journal: JournalRecorder,
            calls: AtomicU64,
            idle_facts: Mutex<Vec<crate::host::TerminalIdleKillFact>>,
        }
        #[async_trait::async_trait]
        impl LoopHost for IdleKillHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            async fn call_tool(
                &self,
                name: &str,
                _arguments: serde_json::Value,
                _call_id: &str,
            ) -> Result<ToolResult, ToolError> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst);
                if n == 0 && name == "run_terminal_cmd" {
                    Ok(ToolResult {
                        output: "[Command still running after 180s] PID: 1234 ...".to_string(),
                        exit_code: None,
                        output_encoding: None,
                        structured: None,
                        mid_run: Some(crate::host::ToolMidRunStatus {
                            task_id: "call-t1".to_string(),
                            pid: Some(1234),
                            output_file: "/tmp/terminal/call-t1.log".to_string(),
                            total_bytes: Some(8192),
                        }),
                        ..Default::default()
                    })
                } else {
                    Ok(ToolResult {
                        output: "read ok".to_string(),
                        exit_code: Some(0),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    })
                }
            }
            async fn request_permission(
                &self,
                _risk: RiskClass,
                _tool: &str,
                _arguments: &serde_json::Value,
            ) -> Result<PermitDecision, PermitError> {
                Ok(PermitDecision::AllowOnce)
            }
            async fn drain_terminal_idle_kills(&self) -> Vec<crate::host::TerminalIdleKillFact> {
                std::mem::take(&mut *self.idle_facts.lock().unwrap())
            }
        }
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = IdleKillHost {
            journal,
            calls: AtomicU64::new(0),
            idle_facts: Mutex::new(vec![crate::host::TerminalIdleKillFact {
                task_id: "call-t1".to_string(),
                pid: Some(1234),
                total_bytes: 8192,
                output_file: "/tmp/terminal/call-t1.log".to_string(),
                wall_ms: 185_000,
                reason: "no output growth or CPU activity for 5s".to_string(),
            }]),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "run_terminal_cmd".to_string(),
                arguments: serde_json::json!({ "command": "sleep 300" }),
                call_id: "call-t1".to_string(),
            }]),
            ScriptedResponse::text("继续"),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({ "path": "a.txt" }),
                call_id: "call-t2".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(fake);
        controller
            .run_turn(
                &host,
                "测试 idle-kill 生命周期事件",
                "RUN-IDLEKILL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .expect("run succeeds");

        let events = events(&dir);
        let running: Vec<(usize, serde_json::Value)> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.event_type == EventType::ToolRunning)
            .map(|(i, e)| (i, e.payload.clone()))
            .collect();
        assert_eq!(running.len(), 2, "mid-run + idle-kill: {running:?}");
        let _mid = running
            .iter()
            .find(|(_, p)| p.get("status").is_none())
            .expect("mid-run tool_running");
        let idle = running
            .iter()
            .find(|(_, p)| p.get("status").and_then(|s| s.as_str()) == Some("idle_killed"))
            .expect("idle-kill tool_running");
        assert_eq!(idle.1["tool"], "run_terminal_cmd");
        assert_eq!(idle.1["call_id"], "call-t1");
        assert_eq!(idle.1["task_id"], "call-t1");
        assert_eq!(idle.1["pid"], serde_json::json!(1234));
        assert_eq!(idle.1["total_bytes"], serde_json::json!(8192));
        assert_eq!(idle.1["output_file"], "/tmp/terminal/call-t1.log");
        assert_eq!(idle.1["wall_ms"], serde_json::json!(185_000u64));
        assert_eq!(idle.1["reason"], "no output growth or CPU activity for 5s");

        let completed: Vec<(usize, serde_json::Value)> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.event_type == EventType::ToolCompleted)
            .map(|(i, e)| (i, e.payload.clone()))
            .collect();
        assert_eq!(
            completed.len(),
            2,
            "auto-bg call + follow-up call: {completed:?}"
        );
        let bg_completed = completed
            .iter()
            .find(|(_, p)| p["call_id"] == "call-t1")
            .expect("auto-bg completed");
        assert_eq!(bg_completed.1["running"], serde_json::json!(true));
        assert!(
            idle.0 > bg_completed.0,
            "idle-kill event must post-date the call's running:true completion \
             (idle idx {} vs completed idx {})",
            idle.0,
            bg_completed.0
        );

        // 同一任务只补记一次：再次 drain（空）+ 下一工具边界不产生重复事件。
        let replay = orz_assurance::replay_journal(
            &dir.join("events.jsonl"),
            Some("RUN-IDLEKILL"),
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.3): browser_read 调用期
    /// 懒启动结果经 ToolResult/`BrowserLaunchFailed` 接缝回传——loop 在
    /// ToolStarted 之后、ToolCompleted 之前落 `browser_launch_result`
    /// 事实事件。
    struct BrowserLaunchHost {
        journal: JournalRecorder,
        outcome: BrowserLaunchHostOutcome,
    }

    enum BrowserLaunchHostOutcome {
        OkWithLaunchFact,
        LaunchFailed(String),
        /// P1-2a 场景 S3：启动成功但动作失败——success fact 随 Err 携带。
        StepFailed(String),
    }

    #[async_trait]
    impl LoopHost for BrowserLaunchHost {
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
            match &self.outcome {
                BrowserLaunchHostOutcome::OkWithLaunchFact => Ok(ToolResult {
                    output: "page text".to_string(),
                    exit_code: Some(0),
                    browser_launch_fact: Some(BrowserLaunchFact::success()),
                    ..Default::default()
                }),
                BrowserLaunchHostOutcome::LaunchFailed(cause) => {
                    Err(ToolError::BrowserLaunchFailed(cause.clone()))
                }
                BrowserLaunchHostOutcome::StepFailed(reason) => Err(ToolError::BrowserStepFailed {
                    reason: reason.clone(),
                    launch_fact: BrowserLaunchFact::success(),
                }),
            }
        }
    }

    #[tokio::test]
    async fn browser_launch_success_fact_journaled_before_completion() {
        let dir = test_dir();
        let host = BrowserLaunchHost {
            journal: JournalRecorder::new(dir.clone()),
            outcome: BrowserLaunchHostOutcome::OkWithLaunchFact,
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_retrieval_enabled(true);
        let tc = ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "url": "https://example.com" }),
            call_id: "call-blaunch-ok".to_string(),
        };
        let mut messages: Vec<Message> = Vec::new();
        let candidates = Arc::new(Mutex::new(Vec::new()));
        let mut writer = EventWriter::new(
            Some(host.journal()),
            EventTrack::V02,
            "RUN-BOK",
            "",
            0,
            None,
            None,
        );
        controller
            .run_host_tool(
                &host,
                &mut writer,
                &tc,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                None,
                Some(&candidates),
                true,
                true,
            )
            .await
            .unwrap();
        let events = events(&dir);
        let started_index = events
            .iter()
            .position(|e| e.event_type == EventType::ToolStarted)
            .expect("tool_started precedes the launch fact");
        let fact_index = events
            .iter()
            .position(|e| e.event_type == EventType::BrowserLaunchResult)
            .expect("browser_launch_result success fact");
        let completed_index = events
            .iter()
            .position(|e| e.event_type == EventType::ToolCompleted)
            .expect("tool_completed");
        assert!(
            started_index < fact_index && fact_index < completed_index,
            "ToolStarted → browser_launch_result → ToolCompleted 全序: {events:?}"
        );
        assert_eq!(
            events[fact_index].payload["status"],
            serde_json::json!("success")
        );
        assert!(events[fact_index].payload["cause"].is_null());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn browser_launch_failure_fact_and_stable_code_journaled() {
        let dir = test_dir();
        let host = BrowserLaunchHost {
            journal: JournalRecorder::new(dir.clone()),
            outcome: BrowserLaunchHostOutcome::LaunchFailed(
                "browser_not_found: ORZ_BROWSER_PATH unset".to_string(),
            ),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_retrieval_enabled(true);
        let tc = ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "url": "https://example.com" }),
            call_id: "call-blaunch-fail".to_string(),
        };
        let mut messages: Vec<Message> = Vec::new();
        let candidates = Arc::new(Mutex::new(Vec::new()));
        let mut writer = EventWriter::new(
            Some(host.journal()),
            EventTrack::V02,
            "RUN-BFAIL",
            "",
            0,
            None,
            None,
        );
        let (result, _) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &tc,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                None,
                Some(&candidates),
                true,
                true,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(
            result.output.contains("browser launch failed")
                && result.output.contains("browser_not_found"),
            "real cause rides the model-visible message: {}",
            result.output
        );
        let events = events(&dir);
        let started_index = events
            .iter()
            .position(|e| e.event_type == EventType::ToolStarted)
            .expect("tool_started precedes the launch fact");
        let fact_index = events
            .iter()
            .position(|e| e.event_type == EventType::BrowserLaunchResult)
            .expect("browser_launch_result failure fact");
        let completed_index = events
            .iter()
            .position(|e| e.event_type == EventType::ToolCompleted)
            .expect("tool_completed");
        assert!(
            started_index < fact_index && fact_index < completed_index,
            "ToolStarted → browser_launch_result → ToolCompleted 全序: {events:?}"
        );
        assert_eq!(
            events[fact_index].payload["status"],
            serde_json::json!("failure")
        );
        assert_eq!(
            events[fact_index].payload["cause"],
            serde_json::json!("browser_not_found: ORZ_BROWSER_PATH unset")
        );
        let completed = &events[completed_index];
        assert_eq!(
            completed.payload["error"],
            serde_json::json!("browser_launch_failed")
        );
        assert_eq!(completed.payload["status"], serde_json::json!("error"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0t P1-2a（2026-09-09, S2-R P2 / 设计 §3.2 场景 S3）：启动成功但页面
    /// 动作失败——`BrowserStepFailed` 携带 success fact，loop 在 ToolCompleted
    /// 前落 `browser_launch_result(success)`；ToolCompleted 错误保留真实类别
    /// 文本（不覆盖稳定码，FP-2），失败漏斗归 ExecutionFailed 系。
    #[tokio::test]
    async fn browser_step_failed_success_fact_journaled_before_completion() {
        let dir = test_dir();
        let host = BrowserLaunchHost {
            journal: JournalRecorder::new(dir.clone()),
            outcome: BrowserLaunchHostOutcome::StepFailed(
                "browser_read failed [browser_read_empty_content]: page produced \
                 no readable text"
                    .to_string(),
            ),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_retrieval_enabled(true);
        let tc = ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "url": "https://example.com" }),
            call_id: "call-bstep-fail".to_string(),
        };
        let mut messages: Vec<Message> = Vec::new();
        let candidates = Arc::new(Mutex::new(Vec::new()));
        let mut writer = EventWriter::new(
            Some(host.journal()),
            EventTrack::V02,
            "RUN-BSTEP",
            "",
            0,
            None,
            None,
        );
        let (result, _feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &tc,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                None,
                Some(&candidates),
                true,
                true,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(
            result.tool_error_kind,
            Some(crate::host::ToolErrorKind::ExecutionFailed)
        );
        assert!(
            result.output.contains("browser step failed")
                && result.output.contains("browser_read_empty_content"),
            "real page-level cause rides the model-visible message: {}",
            result.output
        );
        let events = events(&dir);
        let started_index = events
            .iter()
            .position(|e| e.event_type == EventType::ToolStarted)
            .expect("tool_started precedes the launch fact");
        let fact_index = events
            .iter()
            .position(|e| e.event_type == EventType::BrowserLaunchResult)
            .expect("browser_launch_result success fact (S3)");
        let completed_index = events
            .iter()
            .position(|e| e.event_type == EventType::ToolCompleted)
            .expect("tool_completed");
        assert!(
            started_index < fact_index && fact_index < completed_index,
            "ToolStarted → browser_launch_result → ToolCompleted 全序: {events:?}"
        );
        assert_eq!(
            events[fact_index].payload["status"],
            serde_json::json!("success")
        );
        assert!(events[fact_index].payload["cause"].is_null());
        let completed = &events[completed_index];
        assert_eq!(completed.payload["status"], serde_json::json!("error"));
        assert!(
            completed.payload["error"]
                .as_str()
                .unwrap()
                .contains("browser_read_empty_content"),
            "{}",
            completed.payload["error"]
        );
        // 启动成功 + 动作失败归 ExecutionFailed 系（不新增稳定码），失败
        // 漏斗照常盖章（browser_read 有 URL 身份）。
        assert_eq!(
            completed.payload["failure_target"]["kind"],
            serde_json::json!("url_target")
        );
        assert_eq!(
            completed.payload["failure_target"]["canonical_url"],
            serde_json::json!("https://example.com/")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
