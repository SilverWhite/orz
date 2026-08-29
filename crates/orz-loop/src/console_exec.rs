//! Console execution adapter + direct-mode switch — batch B6 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from controller.rs; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use orz_assurance::sha256_hex;

use crate::console::ActionExecutor;
use crate::controller::{AgentLoopController, AgentLoopError, EventWriter, chrono_utc_now};
use crate::gateway::model::{Message, ToolCall};
use crate::host::{LoopHost, ToolResult};

impl AgentLoopController {
    /// 询问轮模型选择 switch（§7.3/§7.4）：写 `console_mode_transition`
    /// （from=console, to=direct, trigger=assistant_failure_streak）+
    /// gate_log，进入 direct（transition_id 生成），并将当前步骤（第一个
    /// 非 done）标记 in_progress——direct 为有记录的例外：执行不再经订单/
    /// 计划门，但每个直接动作带 transition_id 审计。
    pub(crate) async fn switch_console_to_direct(
        &self,
        writer: &mut EventWriter<'_>,
        round: u32,
        model_reason: Option<&str>,
    ) -> Result<String, AgentLoopError> {
        let plan_epoch = self.blackboard.read().plan.plan_epoch;
        let transition_id = format!("CONSMODE-{}-{:04}", writer.run_id(), writer.seq());
        let (streak, order_ids) = {
            let state = self.console_mode_state.lock().unwrap();
            (state.streak, state.streak_order_ids.clone())
        };
        self.record_console_transition(
            writer,
            &transition_id,
            crate::console_mode::ConsoleMode::Console,
            crate::console_mode::ConsoleMode::Direct,
            "assistant_failure_streak",
            Some(streak),
            &order_ids,
            "switch",
            model_reason,
            round,
            plan_epoch,
            None,
        )
        .await?;
        {
            let mut w = self.blackboard.write();
            if let Some(idx) = crate::planning::current_step_index(&w.plan.steps) {
                crate::planning::mark_step_in_progress(&mut w.plan.steps, idx);
            }
        }
        self.console_mode_state
            .lock()
            .unwrap()
            .switch_to_direct(transition_id.clone());
        Ok(transition_id)
    }

    /// P0-C S2 (2026-08-15): 生产执行器适配——`console::issue_action` 的
    /// 执行委托复用 `run_host_tool`（权限桥 + ACAF 票据 + 模式门 + 事件链），
    /// 禁止绕过既有门直接调 `host.call_tool`。工具回复消息写入 scratch
    /// 缓冲区后丢弃——console 的反馈闭环是结果栏 receipt，不是模型对话
    /// 中的 Tool 消息（发放发生在模型轮结束后，模型并不等待该回复）。
    /// 策略拒绝归一化（v0.6 用户裁决）：权限门（PolicyFeedback::Denied）与
    /// controller 侧无反馈标记的策略拒绝（ACAF/模式门）映射为
    /// `ExecuteError::PolicyDenied`，`issue_action` 据此产出
    /// `step=policy` / `code=policy_denied`。
    ///
    /// R2 半助理层（THIN-HARNESS-REDESIGN V2 §4.2/§4.5）：执行失败
    /// （非零退出 / 超时 / 目标缺失 / 工具不可用）自动派发失败诊断——
    /// 结构化签名词典匹配 → 调用物状态检查 → ≤2KB 极简记录 + 日志指针；
    /// 同时登记/更新 process / file / environment 实体（半助理层执行
    /// 工具时登记/更新；模型经 blackboard_read section=entities 点读）。
    #[allow(clippy::too_many_arguments)] // mirrors run_host_tool's shared-loop contract
    pub(crate) async fn run_console_target(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
        prompt: &str,
        workspace_trust: orz_assurance::gates::ipg::WorkspaceTrust,
        tool_rounds: u32,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        timeout: Option<std::time::Duration>,
        target_tool: &str,
        arguments: &serde_json::Value,
        call_id: &str,
        trace_id: &str,
    ) -> Result<ToolResult, crate::console::ExecuteError> {
        let tc = ToolCall {
            name: target_tool.to_string(),
            arguments: arguments.clone(),
            call_id: call_id.to_string(),
        };
        let mut scratch: Vec<Message> = Vec::new();
        let (result, _feedback) = match self
            .run_host_tool_with_timeout(
                host,
                writer,
                &tc,
                prompt,
                workspace_trust,
                &mut scratch,
                tool_rounds,
                heartbeat,
                None,
                None,
                true,
                true,
                // PLAN-FIRST 阶段 A (2026-08-16): console issuance runs
                // outside the first-round plan gate — never a gate attempt.
                None,
                // PLAN-FIRST 阶段 C (2026-08-16): console issuance is
                // assistant-layer execution — never a direct-mode stamp.
                None,
                timeout,
            )
            .await
        {
            Ok(pair) => pair,
            Err(e) => {
                // R2 失败诊断：执行委托错误（journal/model/session 等，
                // 非工具结果）→ 极简诊断（error_kind=ExecutionFailed），
                // 附着到环境实体。
                self.register_environment_entity();
                let record = crate::diagnostics::diagnose_failure(
                    crate::diagnostics::DiagnosticDomain::Environment,
                    arguments,
                    None,
                    false,
                    None,
                    Some(crate::diagnostics::ErrorKind::ExecutionFailed),
                    None,
                    None,
                    None,
                    &e.to_string(),
                    trace_id,
                );
                self.blackboard
                    .write()
                    .entities
                    .set_diagnostic(&crate::entities::environment_entity_id(), record.clone());
                return Err(crate::console::ExecuteError::ExecutionFailed {
                    message: e.to_string(),
                    detail: Some(serde_json::json!({ "diagnostic": record })),
                });
            }
        };
        // R2 半助理层：环境实体登记（shell 平台 + 工具可用性，探针快照
        // 机械来源）+ 执行目标实体登记（file 锚点 / process 状态）。
        self.register_environment_entity();
        let entity_id = self
            .register_entities_for_tool(target_tool, arguments, &result)
            .await;
        // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): 中间回报（命令
        // 仍在后台运行）不是失败——exit_code 为 None 时不得触发失败诊断。
        let failed = result.policy_denial.is_some()
            || result.timed_out
            || (result.exit_code != Some(0) && result.mid_run.is_none());
        if failed {
            // P0-C S3 前置 (2026-08-15, P1-2 定案): the console adapter
            // classifies policy refusals ONLY from the structured signal —
            // the old stable-output-prefix judgment is retired. R2 诊断域：
            // 策略拒绝 → environment（tool_unavailable）；其余按工具域。
            let diag_domain = if result.policy_denial.is_some() {
                crate::diagnostics::DiagnosticDomain::Environment
            } else {
                Self::tool_diagnostic_domain(target_tool)
            };
            let record = crate::diagnostics::diagnose_failure(
                diag_domain,
                arguments,
                result.exit_code,
                result.timed_out,
                result.output_encoding.as_deref(),
                result.tool_error_kind,
                result.policy_denial.as_ref().map(|d| d.source.as_str()),
                result.policy_denial.as_ref().map(|d| d.code.as_str()),
                result.policy_denial.as_ref().map(|d| d.reason.as_str()),
                &result.output,
                trace_id,
            );
            // 失败诊断附着到失败对象实体（file/process 优先；无目标实体
            // 时落到环境实体兜底）。
            let target_entity = entity_id
                .clone()
                .unwrap_or_else(crate::entities::environment_entity_id);
            self.blackboard
                .write()
                .entities
                .set_diagnostic(&target_entity, record.clone());
            if let Some(denial) = &result.policy_denial {
                return Err(crate::console::ExecuteError::PolicyDenied {
                    message: result.output.clone(),
                    detail: Some(serde_json::json!({
                        "source": denial.source.as_str(),
                        "code": denial.code,
                        "reason": denial.reason,
                        "diagnostic": record,
                    })),
                });
            }
            if result.timed_out {
                return Err(crate::console::ExecuteError::TimedOut {
                    message: format!(
                        "target tool {target_tool} timed out at the host — process tree killed"
                    ),
                    detail: Some(serde_json::json!({ "diagnostic": record })),
                });
            }
            let exit_detail = match result.exit_code {
                None => "no exit code (actions require an explicit success exit code)".to_string(),
                Some(code) => format!("non-zero exit code {code}"),
            };
            return Err(crate::console::ExecuteError::ExecutionFailed {
                message: format!("target tool {exit_detail}"),
                detail: Some(serde_json::json!({
                    "target_tool": target_tool,
                    "exit_code": result.exit_code,
                    "output": crate::console::truncate(&result.output, 4000),
                    "diagnostic": record,
                })),
            });
        }
        Ok(result)
    }

    /// R2：工具 → 诊断域映射（file 域 / terminal 域 / process 域 /
    /// environment 域；结构化签名词典分派）。
    fn tool_diagnostic_domain(tool: &str) -> crate::diagnostics::DiagnosticDomain {
        use crate::diagnostics::DiagnosticDomain;
        match tool {
            "read_file" | "list_dir" | "grep" | "search_replace" => DiagnosticDomain::File,
            "run_terminal_cmd" => DiagnosticDomain::Terminal,
            "run_tests" => DiagnosticDomain::Process,
            _ => DiagnosticDomain::Environment,
        }
    }

    /// R2：环境实体登记/更新——shell 平台（结构化，非宿主具体 shell
    /// 二进制）+ 工具可用性（探针快照机械来源）。每次执行前刷新。
    fn register_environment_entity(&self) {
        let shell = if cfg!(windows) { "windows" } else { "unix" };
        let tools: Vec<String> = self
            .console_probe_source
            .lock()
            .unwrap()
            .as_ref()
            .map(|(_, snapshot)| snapshot.complete.clone())
            .unwrap_or_default();
        self.blackboard
            .write()
            .entities
            .register_environment(shell, tools, &chrono_utc_now());
    }

    /// R2：执行目标实体登记——file 域（路径 stat + 锚点 size/mtime/
    /// sha256 + encoding）与 process 域（exit_code/timed_out）。返回
    /// 实体 id（无目标实体的工具为 None）。
    async fn register_entities_for_tool(
        &self,
        target_tool: &str,
        arguments: &serde_json::Value,
        result: &ToolResult,
    ) -> Option<String> {
        match target_tool {
            "read_file" | "list_dir" | "grep" | "search_replace" => {
                let path = ["target_file", "file_path", "path", "target_directory"]
                    .iter()
                    .find_map(|key| arguments.get(*key).and_then(serde_json::Value::as_str))?;
                let metadata = tokio::fs::metadata(path).await.ok();
                let (exists, size, mtime, is_file) = match metadata {
                    Some(meta) => {
                        let mtime = meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs());
                        (true, Some(meta.len()), mtime, meta.is_file())
                    }
                    None => (false, None, None, false),
                };
                // 锚点 sha256：仅对文件（目录跳过，避免无谓 read）且
                // ≤16MB 计算（确定性、有界；超限记 size/mtime 不记哈希）。
                let sha256 = if exists
                    && is_file
                    && size.is_some_and(|s| s <= crate::entities::FILE_ANCHOR_HASH_MAX)
                {
                    tokio::fs::read(path)
                        .await
                        .ok()
                        .map(|bytes| sha256_hex(&bytes))
                } else {
                    None
                };
                let id = self.blackboard.write().entities.register_file(
                    path,
                    exists,
                    size,
                    mtime,
                    sha256,
                    result.output_encoding.clone(),
                    &chrono_utc_now(),
                );
                Some(id)
            }
            "run_terminal_cmd" | "run_tests" => {
                let command = arguments
                    .get("command")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(target_tool);
                let id = self.blackboard.write().entities.register_process(
                    command,
                    result.exit_code,
                    result.timed_out,
                    &chrono_utc_now(),
                );
                Some(id)
            }
            _ => None,
        }
    }
}

/// P0-C S2 (2026-08-15): `console::ActionExecutor` 的生产实现——委托
/// `AgentLoopController::run_console_target`（即 `run_host_tool` 全链路）。
/// `writer` 经互斥量包裹以配合 `ActionExecutor::execute(&self)` 的签名；
/// 发放为单线程顺序调用，锁不竞争。
pub(crate) struct ControllerConsoleExecutor<'a, 'b, 'c, 'd, 'e> {
    pub(crate) controller: &'c AgentLoopController,
    pub(crate) host: &'e dyn LoopHost,
    /// tokio mutex: the guard must stay Send across the `run_host_tool`
    /// await (async_trait futures require Send).
    pub(crate) writer: tokio::sync::Mutex<&'b mut EventWriter<'a>>,
    pub(crate) prompt: &'d str,
    pub(crate) workspace_trust: orz_assurance::gates::ipg::WorkspaceTrust,
    pub(crate) tool_rounds: u32,
    pub(crate) heartbeat: Option<&'d crate::gateway::model::ActivityClock>,
    /// R2：本次发放的执行 trace id——失败诊断的日志指针
    /// （`assistant.trace` 续读）。
    pub(crate) trace_id: String,
}

#[async_trait::async_trait]
impl<'a, 'b, 'c, 'd, 'e> ActionExecutor for ControllerConsoleExecutor<'a, 'b, 'c, 'd, 'e> {
    async fn execute(
        &self,
        target_tool: &str,
        arguments: &serde_json::Value,
        call_id: &str,
        timeout: Option<std::time::Duration>,
    ) -> Result<ToolResult, crate::console::ExecuteError> {
        let mut writer = self.writer.lock().await;
        self.controller
            .run_console_target(
                self.host,
                &mut writer,
                self.prompt,
                self.workspace_trust,
                self.tool_rounds,
                self.heartbeat,
                timeout,
                target_tool,
                arguments,
                call_id,
                &self.trace_id,
            )
            .await
    }
}
