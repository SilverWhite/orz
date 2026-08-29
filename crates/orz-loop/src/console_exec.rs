//! Console execution adapter + direct-mode switch — batch B6 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from controller.rs; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use orz_assurance::EventType;
use orz_assurance::sha256_hex;

use crate::blackboard::{ActionOrder, ActionRegistration, ActionResult};
use crate::console::ActionExecutor;
use crate::console::{
    ActionKind, CODE_BUDGET_INSUFFICIENT, CODE_CONTENT_ANCHOR_MISMATCH, CODE_ORDER_STALE,
    ConsoleError, MAX_SCRIPT_STEPS_PER_ORDER, STEP_CONTRACT, STEP_POLICY, STEP_PROTOCOL,
    STEP_REGISTRY, STEP_TARGET, failure_envelope, issue_action_inner, static_validate_script,
};
use crate::controller::{
    ActualAnchor, AgentLoopController, AgentLoopError, EventWriter, chrono_utc_now,
};
use crate::gateway::model::{Message, ToolCall};
use crate::host::{LoopHost, ToolResult};
use serde_json::{Value, json};

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

impl AgentLoopController {
    /// P0-C S2/S3 (2026-08-15)；PLAN-FIRST 阶段 B 审查收口 (2026-08-16):
    /// 注册板块投影 = Profile/Bundle 加载集 ∩ 探针完整集（最小参数提示，
    /// 不复制完整 schema）。会话场景键取 `host.tool_policy()`，探针快照取
    /// 本轮主车道工作工具探针。生产路径仅由 `sync_console_registrations`
    /// 以 `Some(probe)` 调用；`probe=None` 仅保留给既有单元测试，不再作为
    /// 生产刷新语义（无探针源时板块不改写、沿用上一轮内容）。
    pub(crate) fn console_registrations(
        &self,
        profile: crate::host::ToolPolicy,
        probe: Option<&crate::tool_probe::ToolProbeSnapshot>,
    ) -> Vec<ActionRegistration> {
        self.console_registry.registrations_for(profile, probe)
    }

    /// P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): 发放前拒绝事件
    /// 的统一 payload——订单身份 + 信封 step + phase（pre_issue/issue）+
    /// 拒绝码 + 原因 + 订单机械盖章（round/plan_epoch/run_id 与
    /// `console_order_written` 记录一致，verifier 交叉核对）。step 只取
    /// 拒绝门步（protocol / registry / contract / target / policy），
    /// 永不取 execute/verify（已执行订单经 tool_started/tool_completed
    /// 留痕）。
    fn console_order_rejected_payload(
        order: &ActionOrder,
        step: &str,
        phase: &str,
        code: &str,
        reason: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "order_id": order.order_id,
            "step": step,
            "phase": phase,
            "code": code,
            "reason": reason,
            "round": order.round,
            "plan_epoch": order.plan_epoch,
            "run_id": order.run_id,
        })
    }

    /// P0-C S2 (2026-08-15): 轮末机械发放入口——动作栏有未消费订单时，
    /// round/plan_epoch 防重放与过期校验 → 取单（消费一次）→
    /// `console::issue_action`（注册表/契约/目标/执行/验证；执行委托复用
    /// `run_host_tool` 的权限/ACAF/模式门与事件链）→ 结果栏
    /// receipt + trace_id → `TraceStore.commit`。
    ///
    /// 调用方（`run_agent_loop`）保证只在 post-tool-batch 安全间隙调用，
    /// 且 pending checkpoint 优先（checkpoint 未完成时本入口不触发）。
    /// 过期订单被消费（清槽）并写显式错误 receipt（`step=protocol` /
    /// `code=order_stale`），模型下轮按结果栏反馈改写订单。
    pub(crate) async fn issue_pending_console_order<'a, 'b>(
        &self,
        host: &dyn LoopHost,
        writer: &'b mut EventWriter<'a>,
        prompt: &str,
        workspace_trust: orz_assurance::gates::ipg::WorkspaceTrust,
        tool_rounds: u32,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
    ) -> Result<u32, AgentLoopError> {
        // P0-C S4 (2026-08-16): 返回本次发放实际消耗的 tool-round 预算
        // 单位（无订单/拒绝 = 0；直接订单 1；脚本 = 实际执行步数）。
        let Some(order) = self.blackboard.read().actions.order.clone() else {
            return Ok(0);
        };
        let current_epoch = self.blackboard.read().plan.plan_epoch;
        let current_run = writer.run_id().to_string();
        if order.round != tool_rounds
            || order.plan_epoch != current_epoch
            || order.run_id != current_run
        {
            let err = ConsoleError {
                step: STEP_PROTOCOL,
                code: CODE_ORDER_STALE,
                message: format!(
                    "order {} is stale: written in round {} / plan_epoch {} / run {} but \
                     current round is {} / plan_epoch {} / run {} — rewrite the order",
                    order.order_id,
                    order.round,
                    order.plan_epoch,
                    order.run_id,
                    tool_rounds,
                    current_epoch,
                    current_run,
                ),
                upstream: Some(serde_json::json!({
                    "order_id": order.order_id,
                    "order_round": order.round,
                    "current_round": tool_rounds,
                    "order_plan_epoch": order.plan_epoch,
                    "current_plan_epoch": current_epoch,
                    "order_run_id": order.run_id,
                    "current_run_id": current_run,
                })),
            };
            // P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): 发放前
            // 拒绝统一入 v0.2 事件面（此前只进结果栏 receipt + TraceStore，
            // 事后核对看不到拒绝码）。
            writer
                .record(
                    EventType::ConsoleOrderRejected,
                    Self::console_order_rejected_payload(
                        &order,
                        STEP_PROTOCOL,
                        "pre_issue",
                        CODE_ORDER_STALE,
                        &err.message,
                    ),
                )
                .await?;
            self.blackboard.write().actions.take_order();
            self.consume_console_order(order, err);
            return Ok(0);
        }
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §6):
        // 步骤门（console 默认态 + 计划在案时）——订单必须绑定当前可执行
        // 步骤（第一个非 done），否则 `step_not_done` 显式拒绝、不执行。
        // direct 为有记录的例外：direct 直接调用不经过订单/计划门（§7.5）。
        if self.console_default_enabled
            && self.console_mode() == crate::console_mode::ConsoleMode::Console
        {
            let gate_result = {
                let guard = self.blackboard.read();
                if guard.plan.steps.is_empty() {
                    None
                } else {
                    Some(crate::planning::order_step_gate(
                        &guard.plan.steps,
                        order.step_id.as_deref(),
                    ))
                }
            };
            if let Some(Err(gate_err)) = gate_result {
                let err = ConsoleError {
                    step: STEP_PROTOCOL,
                    code: gate_err.code(),
                    message: gate_err.message(),
                    upstream: Some(serde_json::json!({
                        "order_id": order.order_id,
                        "step_id": order.step_id,
                    })),
                };
                writer
                    .record(
                        EventType::ConsoleOrderRejected,
                        Self::console_order_rejected_payload(
                            &order,
                            STEP_PROTOCOL,
                            "pre_issue",
                            err.code,
                            &err.message,
                        ),
                    )
                    .await?;
                self.blackboard.write().actions.take_order();
                self.consume_console_order(order, err);
                return Ok(0);
            }
        }
        // P0-C S4 (2026-08-16)：发放前预算预检——当前模型轮已消耗 1 单位
        // （本轮结束后 `tool_rounds += 1`），剩余 = max − (tool_rounds + 1)。
        // 脚本订单要求长度 ≤ 剩余（直接订单恒 1 单位）；不足零执行拒绝、
        // 显式错误码、不消耗预算。
        // P0-C S4 审查收口（2026-08-16）：预检不掩盖内层错误——脚本先做
        // 轻量静态校验（无执行），失败或超上限（>8 步）不预检，交注册表/
        // 契约校验产生对应错误码（unknown_service / invalid_script 等）。
        let required = match self.console_registry.get(&order.action).map(|s| s.kind) {
            Some(ActionKind::RunScript) => {
                match order.arguments.get("script").and_then(Value::as_array) {
                    Some(steps) if steps.len() > MAX_SCRIPT_STEPS_PER_ORDER => None,
                    Some(steps) => match static_validate_script(steps, &self.console_registry) {
                        Ok(_) => Some(steps.len() as u32),
                        Err(_) => None,
                    },
                    None => None,
                }
            }
            Some(ActionKind::TraceRead) | Some(ActionKind::Host) => Some(1),
            _ => None,
        };
        let remaining = self
            .max_tool_rounds
            .saturating_sub(tool_rounds.saturating_add(1));
        if let Some(required) = required
            && required > remaining
        {
            let err = ConsoleError {
                step: STEP_PROTOCOL,
                code: CODE_BUDGET_INSUFFICIENT,
                message: format!(
                    "order {} needs {required} tool-round unit(s) but only {remaining} \
                         remain (round {tool_rounds} / max {}) — rewrite with fewer steps",
                    order.order_id, self.max_tool_rounds
                ),
                upstream: Some(json!({
                    "action": order.action,
                    "required": required,
                    "remaining": remaining,
                    "tool_rounds": tool_rounds,
                    "max_tool_rounds": self.max_tool_rounds,
                })),
            };
            writer
                .record(
                    EventType::ConsoleOrderRejected,
                    Self::console_order_rejected_payload(
                        &order,
                        STEP_PROTOCOL,
                        "pre_issue",
                        CODE_BUDGET_INSUFFICIENT,
                        &err.message,
                    ),
                )
                .await?;
            self.blackboard.write().actions.take_order();
            self.consume_console_order(order, err);
            return Ok(0);
        }
        // FUS-READ-ANCHOR-WRITE-GUARD (2026-08-21, ADR-0010 §14.38): 写订单
        // 携带 `expected_anchor`（read_file 返回的 {size, mtime, sha256}）时，
        // 发放前机械核证目标文件内容锚点——stat 快筛 size/mtime + sha256 权威。
        // 不匹配拒单（复用 order_stale 信封形态：phase=pre_issue /
        // step=protocol / code=content_anchor_mismatch），不执行任何编辑；
        // 主 agent 重读后用新锚点重下。目标文件不存在（新建）时跳过。
        if self
            .console_registry
            .get(&order.action)
            .and_then(|s| s.target_tool.as_deref())
            == Some("search_replace")
            && let Some(anchor) = order.arguments.get("expected_anchor")
            && let Some(file_path) = order
                .arguments
                .get("file_path")
                .and_then(serde_json::Value::as_str)
            && let Some(err) = self
                .verify_content_anchor(host, file_path, anchor, &order.order_id)
                .await
        {
            writer
                .record(
                    EventType::ConsoleOrderRejected,
                    Self::console_order_rejected_payload(
                        &order,
                        STEP_PROTOCOL,
                        "pre_issue",
                        CODE_CONTENT_ANCHOR_MISMATCH,
                        &err.message,
                    ),
                )
                .await?;
            self.blackboard.write().actions.take_order();
            self.consume_console_order(order, err);
            return Ok(0);
        }
        let Some(order) = self.blackboard.write().actions.take_order() else {
            return Ok(0);
        };
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §6):
        // 发放时迁移 pending/failed → in_progress（订单绑定步骤）。
        if let Some(step_id) = order.step_id.as_deref() {
            let mut w = self.blackboard.write();
            if let Some(idx) = w.plan.steps.iter().position(|s| s.id == step_id) {
                crate::planning::mark_step_in_progress(&mut w.plan.steps, idx);
            }
        }
        let mut trace = self
            .console_traces
            .lock()
            .unwrap()
            .new_trace(Some(order.order_id.clone()));
        let call_id = order.order_id.to_lowercase();
        let executor = ControllerConsoleExecutor {
            controller: self,
            host,
            writer: tokio::sync::Mutex::new(writer),
            prompt,
            workspace_trust,
            tool_rounds,
            heartbeat,
            trace_id: trace.trace_id.clone(),
        };
        // P0-C S3: 控制台内部动作（assistant.trace / workspace.run_script）
        // 不经 run_host_tool——审计事件面显式补 ToolStarted/ToolCompleted
        // （读操作本身入 journal；run_script 的内层步骤仍经 run_host_tool
        // 逐行留痕，外层用合成 call_id 区分）。
        let is_internal = matches!(
            self.console_registry.get(&order.action).map(|s| s.kind),
            Some(ActionKind::TraceRead) | Some(ActionKind::RunScript)
        );
        if is_internal {
            executor
                .writer
                .lock()
                .await
                .record(
                    EventType::ToolStarted,
                    serde_json::json!({
                        "tool": order.action,
                        "call_id": call_id,
                    }),
                )
                .await?;
        }
        let mut consumed = 0u32;
        // R2 半助理层（§4.5）：实体注册表快照——短锁克隆、锁不跨 await，
        // 供 `diagnostics.diagnose` 内部动作点读失败对象实体的最近诊断。
        let entities_snapshot = self.blackboard.read().entities.clone();
        let result = issue_action_inner(
            &self.console_registry,
            &executor,
            Some(&self.console_traces),
            &order,
            &mut trace,
            &call_id,
            None,
            Some(&entities_snapshot),
            &mut consumed,
        )
        .await;
        if is_internal {
            let (exit_code, error_code) = match &result {
                Ok(_) => (0, None),
                Err(err) => (1, Some(err.code)),
            };
            let mut payload = serde_json::json!({
                "tool": order.action,
                "call_id": call_id,
                "exit_code": exit_code,
            });
            if let Some(code) = error_code {
                payload["status"] = serde_json::json!("error");
                payload["error"] = serde_json::json!(code);
            }
            executor
                .writer
                .lock()
                .await
                .record(EventType::ToolCompleted, payload)
                .await?;
        }
        match result {
            Ok(response) => {
                self.record_console_receipt(&order, true, "ok", None, true);
                self.push_console_result(
                    order.order_id.clone(),
                    order.action.clone(),
                    true,
                    Some(response),
                    None,
                    trace.trace_id.clone(),
                );
            }
            Err(err) => {
                // P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): 发放期
                // 门（registry / contract / target / ACAF / policy / mode）
                // 拒绝统一入事件面（phase=issue）；执行失败（execute/
                // verify）不入本事件——那些路径已经 tool_started/
                // tool_completed 留痕。
                if matches!(
                    err.step,
                    STEP_REGISTRY | STEP_CONTRACT | STEP_TARGET | STEP_POLICY
                ) {
                    executor
                        .writer
                        .lock()
                        .await
                        .record(
                            EventType::ConsoleOrderRejected,
                            Self::console_order_rejected_payload(
                                &order,
                                err.step,
                                "issue",
                                err.code,
                                &err.message,
                            ),
                        )
                        .await?;
                }
                let envelope = failure_envelope(&mut trace, Some(&order.action), &err, 10);
                self.record_console_receipt(
                    &order,
                    false,
                    &envelope.error.step,
                    envelope.error.upstream.as_ref(),
                    // 2026-08-18 (ADR-0010 §14.25 项 2): 步骤状态只随执行
                    // receipt 迁移——发放期拒绝（registry/contract/target/
                    // policy 等非执行步）不标 failed（未执行任何动作，步骤
                    // 保持发放时置的 in_progress，订单可重试）。
                    matches!(
                        err.step,
                        crate::console::STEP_EXECUTE | crate::console::STEP_VERIFY
                    ),
                );
                let error_value = serde_json::to_value(&envelope.error).unwrap_or_else(|_| {
                    serde_json::json!({
                        "step": err.step,
                        "code": err.code,
                        "message": err.message,
                    })
                });
                self.push_console_result(
                    order.order_id.clone(),
                    order.action.clone(),
                    false,
                    None,
                    Some(error_value),
                    envelope.error.trace_id.clone(),
                );
            }
        }
        self.commit_console_trace(&trace);
        Ok(consumed)
    }

    /// 过期/失败订单收口：追加失败事件 → 构造信封 → 结果栏 receipt →
    /// commit（`assistant.trace` 立即可见）。
    fn consume_console_order(&self, order: ActionOrder, err: ConsoleError) {
        // F4 (2026-08-16 审查收口): 拒绝路径（order_stale / step_not_done /
        // budget_insufficient）只做连败记账——这些步均非故障面（不递增、
        // 不清零），且未执行的订单不得把目标步骤置 failed；步骤状态只随
        // 执行 receipt 迁移（设计 §6：done/failed 来自发放后执行结果）。
        {
            let mut state = self.console_mode_state.lock().unwrap();
            state.record_receipt(false, err.step, err.upstream.as_ref());
        }
        let mut trace = self
            .console_traces
            .lock()
            .unwrap()
            .new_trace(Some(order.order_id.clone()));
        let envelope = failure_envelope(&mut trace, Some(&order.action), &err, 10);
        let error_value = serde_json::to_value(&envelope.error).unwrap_or_else(|_| {
            serde_json::json!({
                "step": err.step,
                "code": err.code,
                "message": err.message,
            })
        });
        self.push_console_result(
            order.order_id,
            order.action,
            false,
            None,
            Some(error_value),
            envelope.error.trace_id.clone(),
        );
        self.commit_console_trace(&trace);
    }

    /// FUS-READ-ANCHOR-WRITE-GUARD (ADR-0010 §14.38): 机械核证目标文件内容
    /// 锚点——stat 快筛 size/mtime + sha256 权威。返回 `Some(ConsoleError)`
    /// 表示不匹配或核证失败（发放前拒绝，复用 order_stale 信封形态）；
    /// `None` 表示核证通过或无需核证（目标文件不存在=新建路径）。核证期
    /// 间的 stat/read I/O 错误（NotFound 除外）fail-closed 拒单——宁可
    /// 重读重下，绝不带着未核证的内容放行编辑。
    pub(crate) async fn verify_content_anchor(
        &self,
        host: &dyn LoopHost,
        file_path: &str,
        anchor: &serde_json::Value,
        label: &str,
    ) -> Option<ConsoleError> {
        let expected_size = anchor.get("size").and_then(serde_json::Value::as_u64)?;
        let expected_sha256 = anchor.get("sha256").and_then(serde_json::Value::as_str)?;
        let expected_mtime = anchor.get("mtime").and_then(serde_json::Value::as_u64);
        let resolved = host.session_cwd().join(file_path);
        // 与 read_file 的解析近似：此处不做 canonicalize（clippy 禁用——
        // Windows verbatim 路径），stat/read 由 OS 解析相对成分；任何解析
        // 差异只会造成额外拒单重读，绝不会造成陈旧写入。
        let metadata = match tokio::fs::metadata(&resolved).await {
            Ok(metadata) => metadata,
            // 目标不存在=新建路径（与 search_replace 空 old_string 建文件
            // 的既有语义一致），跳过核证。
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
            // 其余 stat 错误（权限/瞬时 FS 等）fail-closed：无法核证即拒单。
            Err(err) => {
                return Some(Self::anchor_verify_error(
                    label,
                    file_path,
                    expected_size,
                    expected_mtime,
                    expected_sha256,
                    None,
                    &format!("failed to stat target: {err}"),
                ));
            }
        };
        let actual_size = metadata.len();
        let actual_mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        // 快筛：size/mtime 任一不匹配即拒（避免对大文件无谓哈希）；mtime
        // 仅作预检（可被保留/取整），权威仍是 sha256。
        if actual_size != expected_size
            || (expected_mtime.is_some()
                && actual_mtime.is_some()
                && expected_mtime != actual_mtime)
        {
            return Some(Self::anchor_verify_error(
                label,
                file_path,
                expected_size,
                expected_mtime,
                expected_sha256,
                Some(ActualAnchor {
                    size: Some(actual_size),
                    mtime: actual_mtime,
                    sha256: None,
                }),
                "size/mtime pre-check mismatch",
            ));
        }
        let bytes = match tokio::fs::read(&resolved).await {
            Ok(bytes) => bytes,
            // stat 已成功而 read 失败（权限变更/路径变为目录等）：无法
            // 重算权威哈希即 fail-closed 拒单，绝不带未核证内容放行。
            Err(err) => {
                return Some(Self::anchor_verify_error(
                    label,
                    file_path,
                    expected_size,
                    expected_mtime,
                    expected_sha256,
                    None,
                    &format!("failed to read target: {err}"),
                ));
            }
        };
        let actual_sha256 = sha256_hex(&bytes);
        if actual_sha256 != expected_sha256 {
            return Some(Self::anchor_verify_error(
                label,
                file_path,
                expected_size,
                expected_mtime,
                expected_sha256,
                Some(ActualAnchor {
                    size: Some(actual_size),
                    mtime: actual_mtime,
                    sha256: Some(actual_sha256),
                }),
                "sha256 mismatch",
            ));
        }
        None
    }

    /// 构建核证失败/不匹配的拒单信封（复用 order_stale 形态：
    /// phase=pre_issue / step=protocol / code=content_anchor_mismatch）。
    /// `actual=None` 表示核证期 I/O 失败（未能取得当前内容锚点）；
    /// `Some(actual)` 表示已取得并比较、与期望不符。
    fn anchor_verify_error(
        label: &str,
        file_path: &str,
        expected_size: u64,
        expected_mtime: Option<u64>,
        expected_sha256: &str,
        actual: Option<ActualAnchor>,
        reason: &str,
    ) -> ConsoleError {
        let actual = actual.unwrap_or(ActualAnchor {
            size: None,
            mtime: None,
            sha256: None,
        });
        let actual_sha256 = actual.sha256.as_deref().unwrap_or("<unverifiable>");
        ConsoleError {
            step: STEP_PROTOCOL,
            code: CODE_CONTENT_ANCHOR_MISMATCH,
            message: format!(
                "{label}: content anchor verification failed for {} ({reason}) — expected \
                 sha256={} size={} mtime={:?} but got sha256={} size={:?} mtime={:?}; re-read \
                 the file and rewrite the order with the new anchor",
                file_path,
                expected_sha256,
                expected_size,
                expected_mtime,
                actual_sha256,
                actual.size,
                actual.mtime,
            ),
            upstream: Some(serde_json::json!({
                "label": label,
                "file_path": file_path,
                "expected": {
                    "size": expected_size,
                    "mtime": expected_mtime,
                    "sha256": expected_sha256,
                },
                "actual": {
                    "size": actual.size,
                    "mtime": actual.mtime,
                    "sha256": actual.sha256,
                },
            })),
        }
    }

    fn push_console_result(
        &self,
        order_id: String,
        action: String,
        ok: bool,
        response: Option<serde_json::Value>,
        error: Option<serde_json::Value>,
        trace_id: String,
    ) {
        self.blackboard.write().actions.push_result(ActionResult {
            order_id,
            action: Some(action),
            ok,
            response,
            error,
            trace_id,
            timestamp: chrono_utc_now(),
        });
    }

    fn commit_console_trace(&self, trace: &crate::console::Trace) {
        self.console_traces.lock().unwrap().commit(trace);
    }

    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §6/§7.2):
    /// 订单 receipt 统一记账——① 双模式故障连败计数（ok=true 重置；
    /// 故障面递增；业务/policy/protocol 等不计）；② 步骤状态迁移
    /// （订单绑定步骤且 `mutate_step` 时：成功 → done(receipt_id)；
    /// 失败 → failed(receipt_id)）。2026-08-18（ADR-0010 §14.25 项 2）：
    /// `mutate_step=false`（发放期拒绝）只做故障记账、不迁移步骤状态。
    pub(crate) fn record_console_receipt(
        &self,
        order: &ActionOrder,
        ok: bool,
        step: &str,
        upstream: Option<&serde_json::Value>,
        mutate_step: bool,
    ) {
        let fault = !ok && crate::console_mode::counts_as_assistant_fault(step, upstream);
        {
            let mut state = self.console_mode_state.lock().unwrap();
            state.record_receipt(ok, step, upstream);
            if fault {
                state.push_streak_order(&order.order_id);
            }
        }
        if mutate_step && let Some(step_id) = order.step_id.as_deref() {
            let mut w = self.blackboard.write();
            if let Some(idx) = w.plan.steps.iter().position(|s| s.id == step_id) {
                if ok {
                    // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the final
                    // (terminal) step never auto-advances on an ordinary
                    // order receipt — only the explicit `submit` delivery
                    // path marks it done (杜绝 echo done 式虚假递交). The
                    // predicate is ID-keyed (last step id ∈ {deliver,
                    // submit}) — legacy/restored plans with a plain final
                    // step id keep the pre-S1 auto-advance semantics.
                    let is_terminal_step = crate::planning::is_terminal_step(&w.plan.steps, idx);
                    if !is_terminal_step {
                        crate::planning::mark_step_done(
                            &mut w.plan.steps,
                            idx,
                            &order.order_id,
                            None,
                        );
                    }
                } else {
                    crate::planning::mark_step_failed(&mut w.plan.steps, idx, &order.order_id);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // mirrors the v0.2 payload's closed field set
    pub(crate) async fn record_console_transition(
        &self,
        writer: &mut EventWriter<'_>,
        transition_id: &str,
        from: crate::console_mode::ConsoleMode,
        to: crate::console_mode::ConsoleMode,
        trigger: &str,
        streak: Option<u32>,
        order_ids: &[String],
        model_decision: &str,
        model_reason: Option<&str>,
        round: u32,
        plan_epoch: u64,
        related_transition_id: Option<&str>,
    ) -> Result<(), AgentLoopError> {
        writer
            .record(
                EventType::ConsoleModeTransition,
                crate::console_mode::transition_payload(
                    transition_id,
                    from,
                    to,
                    trigger,
                    streak,
                    order_ids,
                    model_decision,
                    model_reason,
                    writer.run_id(),
                    round,
                    plan_epoch,
                    related_transition_id,
                ),
            )
            .await?;
        self.blackboard
            .write()
            .gate_log
            .gate_decisions
            .push(format!(
                "console_mode: {} → {} (transition {transition_id}, decision {model_decision})",
                from.as_str(),
                to.as_str(),
            ));
        Ok(())
    }

    /// 询问轮模型选择 stay（§7.3）：写 `console_mode_transition`
    /// （from=console, to=console, trigger=assistant_failure_streak）+
    /// gate_log；连败清零、本 run 不再询问。
    pub(crate) async fn record_console_stay(
        &self,
        writer: &mut EventWriter<'_>,
        round: u32,
        model_reason: Option<&str>,
    ) -> Result<(), AgentLoopError> {
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
            crate::console_mode::ConsoleMode::Console,
            "assistant_failure_streak",
            Some(streak),
            &order_ids,
            "stay",
            model_reason,
            round,
            plan_epoch,
            None,
        )
        .await?;
        self.console_mode_state.lock().unwrap().stay_in_console();
        Ok(())
    }

    /// `console_return_to_console`（§7.1/§7.5）：单向返回——写
    /// `console_mode_transition`（from=direct, to=console,
    /// trigger=model_return, related=当前 direct transition）+ gate_log；
    /// 模式复位、本 run 不再询问。
    pub(crate) async fn return_console_to_console(
        &self,
        writer: &mut EventWriter<'_>,
        round: u32,
    ) -> Result<(), AgentLoopError> {
        let plan_epoch = self.blackboard.read().plan.plan_epoch;
        let (transition_id, direct_transition) = {
            let state = self.console_mode_state.lock().unwrap();
            (
                format!("CONSMODE-{}-{:04}", writer.run_id(), writer.seq()),
                state.transition_id.clone(),
            )
        };
        self.record_console_transition(
            writer,
            &transition_id,
            crate::console_mode::ConsoleMode::Direct,
            crate::console_mode::ConsoleMode::Console,
            "model_return",
            None,
            &[],
            "return_to_console",
            None,
            round,
            plan_epoch,
            direct_transition.as_deref(),
        )
        .await?;
        self.console_mode_state.lock().unwrap().return_to_console();
        Ok(())
    }
}
