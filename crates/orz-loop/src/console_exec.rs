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
            if let Some(idx) = crate::planning::current_step_index(&w.plan.steps)
                && crate::planning::mark_step_in_progress(&mut w.plan.steps, idx)
            {
                w.bump_plan();
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
    /// 拒绝码 + 原因 + 订单机械盖章。step 只取拒绝门步（protocol /
    /// registry / contract / target / policy），永不取 execute/verify
    /// （已执行订单经 tool_started/tool_completed 留痕）。
    /// 判官侧 2026-09-06 起收窄为形状不变量（S2d 裁决一，ADR-0010
    /// §14.57）：written 前置与戳一致交叉核对已随写单链（§14.39）退役；
    /// 发射点运行时休眠（订单槽唯一生产写入面 blackboard_action_write
    /// 已被窄门拒发），形状子规则对写单复活后的 rejection 继续生效。
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
        // TER T1.7 (2026-09-04)：默认 `max_tool_rounds == 0` = 无硬限——
        // budget_insufficient 预检只在显式配置非零上限时挂载（escape
        // hatch；无上限时订单恒可发放，交由墙钟/其它闸兜底）。
        let remaining = (self.max_tool_rounds != 0).then(|| {
            self.max_tool_rounds
                .saturating_sub(tool_rounds.saturating_add(1))
        });
        if let Some(required) = required
            && let Some(remaining) = remaining
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
            if let Some(idx) = w.plan.steps.iter().position(|s| s.id == step_id)
                && crate::planning::mark_step_in_progress(&mut w.plan.steps, idx)
            {
                w.bump_plan();
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
                let mut error_value = serde_json::to_value(&envelope.error).unwrap_or_else(|_| {
                    serde_json::json!({
                        "step": err.step,
                        "code": err.code,
                        "message": err.message,
                    })
                });
                // 0q（ADR-0010 §14.63 裁决 ②）：订单失败收据同过漏斗——
                // action_target 第五族身份入聚合 + receipt 信封挂载。
                self.note_order_failure(&mut error_value, &order.order_id, &order.action, err.code);
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
        let mut error_value = serde_json::to_value(&envelope.error).unwrap_or_else(|_| {
            serde_json::json!({
                "step": err.step,
                "code": err.code,
                "message": err.message,
            })
        });
        // 0q（ADR-0010 §14.63 裁决 ②）：发放前拒绝（order_stale /
        // step_not_done / budget_insufficient 等）与执行失败同纪律——
        // 订单收据失败统一过漏斗。
        self.note_order_failure(&mut error_value, &order.order_id, &order.action, err.code);
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
        // B1 会话化基础（2026-09-03）：receipt 写时盖 (round, domain) 章
        // ——round/domain 与 exec/failure_agg 同源（LIF 会话相对决策轮 +
        // 当前域），供 B2 按域段/轮数折叠与展开。
        let (round, domain) = self.blackboard_stamp();
        self.blackboard.write().actions.push_result(ActionResult {
            order_id,
            action: Some(action),
            ok,
            response,
            error,
            trace_id,
            timestamp: chrono_utc_now(),
            round,
            domain: Some(domain),
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
                    if !is_terminal_step
                        && crate::planning::mark_step_done(
                            &mut w.plan.steps,
                            idx,
                            &order.order_id,
                            None,
                        )
                    {
                        w.bump_plan();
                    }
                } else if crate::planning::mark_step_failed(&mut w.plan.steps, idx, &order.order_id)
                {
                    w.bump_plan();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use crate::host::ToolDef;
    use orz_assurance::{EventTrack, JournalRecorder, RunEvent};
    use std::sync::Arc;

    /// 2026-08-18 (ADR-0010 §14.25 项 1): 状态行作为尾随用户消息、
    /// 仅在步骤状态变化时追加——订单 receipt ok → 步骤 done → 当前步
    /// 前进后追加新状态行，system 提示词保持字节稳定。
    /// 2026-08-18 (ADR-0010 §14.25 项 2) + MECHANICAL-AUDIT-LAYER 审查处理
    /// (2026-08-24): 调用面拒绝（现为退役工具窄门 retired_tool_denied）
    /// 不把绑定步骤标 failed——步骤保持 in_progress（可重试语义保留）。
    #[tokio::test]
    async fn policy_denied_order_does_not_fail_bound_step() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = DenyHost { journal };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_action_write".to_string(),
                arguments: serde_json::json!({
                    "action": "workspace.search_replace",
                    "arguments": {
                        "file_path": "a.txt",
                        "old_string": "x",
                        "new_string": "y",
                    },
                    "step_id": "step-1",
                }),
                call_id: "call-w1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_plan(
            "PLAN-DENY".to_string(),
            1,
            "改文件".to_string(),
            vec!["第一步".to_string()],
        );
        controller
            .run_turn(&host, "被拒订单", "RUN-DENY", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert_eq!(r.plan.steps.len(), 1);
        assert!(
            matches!(
                r.plan.steps[0].status,
                crate::blackboard::StepStatus::InProgress
            ),
            "policy-denied order must not fail the bound step: {:?}",
            r.plan.steps[0].status
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PLAN-FIRST 阶段 B (2026-08-16): 注册板块绑定黑板模型栏——读取
    /// actions 分区时由最近探针源派生并持久化，替代 loop-top 静态刷新
    /// 的陈旧内容（含 S2 静态基础集残留）。
    #[test]
    fn console_stage_b_actions_read_derives_registration_from_probe_source() {
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![])));
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
        // 陈旧板块：静态基础集残留——派生读取后不得再出现。
        controller
            .blackboard()
            .write()
            .actions
            .set_registration(vec![crate::blackboard::ActionRegistration {
                name: "workspace.run_tests".to_string(),
                description: "stale static base set".to_string(),
                parameters: serde_json::json!({}),
                target_policy: crate::entities::TargetPolicy::None,
            }]);
        controller.set_console_probe_source(crate::host::ToolPolicy::Interactive, snapshot);
        let text = controller.render_blackboard_section("actions", None, None, None);
        assert!(text.contains("workspace.read_file"), "{text}");
        assert!(text.contains("workspace.run_script"), "{text}");
        assert!(text.contains("assistant.trace"), "{text}");
        assert!(!text.contains("workspace.run_tests"), "{text}");
        // 派生结果持久化回板块（checkpoint/归档沿用）。
        let board = controller.blackboard().read();
        let names: Vec<&str> = board
            .actions
            .registration
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert!(names.contains(&"workspace.read_file"), "{names:?}");
        assert!(!names.contains(&"workspace.run_tests"), "{names:?}");
    }

    /// PLAN-FIRST 阶段 B (2026-08-16): 无探针源时读取沿用既有板块内容
    /// （checkpoint 轮/归档语义），不改写、不静默清空。
    #[test]
    fn console_stage_b_actions_read_retains_stored_registration_without_probe_source() {
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![])));
        controller
            .blackboard()
            .write()
            .actions
            .set_registration(vec![crate::blackboard::ActionRegistration {
                name: "assistant.trace".to_string(),
                description: "retained".to_string(),
                parameters: serde_json::json!({}),
                target_policy: crate::entities::TargetPolicy::None,
            }]);
        let text = controller.render_blackboard_section("actions", None, None, None);
        assert!(text.contains("assistant.trace"), "{text}");
        assert!(text.contains("(no pending order)"), "{text}");
        let board = controller.blackboard().read();
        assert_eq!(
            board.actions.registration.len(),
            1,
            "{:?}",
            board.actions.registration
        );
        assert_eq!(board.actions.registration[0].name, "assistant.trace");
    }

    /// PLAN-FIRST 阶段 B (2026-08-16): 工具栏与注册板块同源一致性——同一
    /// 探针源下，注册板块中工作工具目标（Host 动作）必须同时出现在模型
    /// 可见工具投影中；探针移除的工作工具不得出现在注册板块。
    #[test]
    fn console_stage_b_registration_matches_tool_projection() {
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
        let registry = crate::console::default_service_registry();
        let regs =
            registry.registrations_for(crate::host::ToolPolicy::Interactive, Some(&snapshot));
        let base = crate::tool_probe::WORK_TOOLS
            .iter()
            .map(|name| ToolDef {
                name: (*name).to_string(),
                description: format!("tool {name}"),
                parameters: serde_json::json!({}),
            })
            .collect::<Vec<_>>();
        let projected = AgentLoopController::project_main_agent_tool_defs(&base, &snapshot);
        let projected_names: std::collections::HashSet<&str> =
            projected.iter().map(|t| t.name.as_str()).collect();
        for reg in &regs {
            let target = reg.name.trim_start_matches("workspace.");
            // Host 动作目标若是工作工具，必须在工具栏投影内；内部动作
            // （assistant.trace/workspace.run_script）与非工作工具目标
            // （project_doc_index）不要求同名。
            // THIN-HARNESS-REDESIGN R1 (§4.1)：封存工具（list_dir /
            // run_tests / todo_write / …）已从投影面移除——console 注册
            // 板是休眠面，其封存动作不可达，不再要求同名投影。
            if crate::tool_probe::is_main_agent_work_tool(target)
                && !AgentLoopController::R1_SEALED_MAIN_TOOLS.contains(&target)
            {
                assert!(
                    projected_names.contains(target),
                    "registration {} missing from toolbar projection",
                    reg.name
                );
            }
        }
        assert!(
            regs.iter().all(|r| r.name != "workspace.run_tests"),
            "probe-incomplete tool must not be registered: {regs:?}"
        );
    }

    /// PLAN-FIRST 阶段 B 审查收口 (2026-08-16): 归档 epoch 读保持快照——
    /// 即使存在探针源，`render_blackboard_section("actions", None, Some(n))`
    /// 也不按探针源派生、不改写当前板块；live 读仍正常派生（对照）。
    #[test]
    fn console_stage_b_actions_read_archived_epoch_keeps_snapshot() {
        let dir = test_dir().join("gsa").join("stage_b_archive");
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![])))
            .with_blackboard_archive_dir(Some(dir.clone()))
            .with_plan(
                "PLAN-B-1".to_string(),
                1,
                "old".to_string(),
                vec!["旧步骤".to_string()],
            );
        controller
            .blackboard()
            .write()
            .actions
            .set_registration(vec![crate::blackboard::ActionRegistration {
                name: "workspace.archived_legacy".to_string(),
                description: "archived registration".to_string(),
                parameters: serde_json::json!({}),
                target_policy: crate::entities::TargetPolicy::None,
            }]);
        // 轮换：epoch 1（含旧注册板块）归档，当前板块重置。
        let controller = controller.with_plan(
            "PLAN-B-2".to_string(),
            2,
            "new".to_string(),
            vec!["新步骤".to_string()],
        );
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
        controller.set_console_probe_source(crate::host::ToolPolicy::Interactive, snapshot);
        let archived = controller.render_blackboard_section("actions", None, Some(1), None);
        assert!(archived.contains("workspace.archived_legacy"), "{archived}");
        assert!(!archived.contains("workspace.read_file"), "{archived}");
        let live = controller.render_blackboard_section("actions", None, None, None);
        assert!(live.contains("workspace.read_file"), "{live}");
        assert!(!live.contains("workspace.archived_legacy"), "{live}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PLAN-FIRST 阶段 B 审查收口 (2026-08-16): 探针源随 run 复位——复位后
    /// 无探针源，读取沿用既有板块内容、不按源派生（不跨 run 沿用）。
    #[test]
    fn console_stage_b_probe_source_resets_across_runs() {
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![])));
        controller
            .blackboard()
            .write()
            .actions
            .set_registration(vec![crate::blackboard::ActionRegistration {
                name: "assistant.trace".to_string(),
                description: "retained across reset".to_string(),
                parameters: serde_json::json!({}),
                target_policy: crate::entities::TargetPolicy::None,
            }]);
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
        controller.set_console_probe_source(crate::host::ToolPolicy::Interactive, snapshot);
        controller.reset_console_probe_source();
        let text = controller.render_blackboard_section("actions", None, None, None);
        assert!(text.contains("assistant.trace"), "{text}");
        assert!(!text.contains("workspace.read_file"), "{text}");
        let board = controller.blackboard().read();
        assert_eq!(board.actions.registration.len(), 1);
        assert_eq!(board.actions.registration[0].name, "assistant.trace");
    }

    /// P0-C S2 (2026-08-15): round/plan_epoch 防重放与过期——写单轮与发放轮
    /// 不一致的订单被消费（清槽）并写显式 `order_stale` receipt，不执行。
    #[tokio::test]
    async fn console_s2_stale_order_is_consumed_with_explicit_receipt() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "never".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.plan.plan_epoch = 1;
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-000009".to_string(),
                    action: "workspace.read_file".to_string(),
                    arguments: serde_json::json!({"target_file": "a.txt"}),
                    target: None,
                    step_id: None,
                    round: 0,
                    plan_epoch: 1,
                    run_id: "RUN-OLD".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-S2S",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "stale",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none(), "{:?}", r.actions.order);
        assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
        let receipt = &r.actions.results[0];
        assert!(!receipt.ok);
        assert_eq!(receipt.error.as_ref().unwrap()["step"], "protocol");
        assert_eq!(receipt.error.as_ref().unwrap()["code"], "order_stale");
        // 未执行任何目标工具。
        assert!(
            r.tool_actions.iter().all(|t| t.tool != "read_file"),
            "{:?}",
            r.tool_actions
        );
        // 过期订单的失败 trace 已 commit。
        let traces = controller.console_traces.lock().unwrap();
        let trace = traces
            .get(&receipt.trace_id)
            .expect("stale trace committed");
        assert_eq!(trace.events.last().unwrap().step, "protocol");
        assert_eq!(
            trace.events.last().unwrap().code.as_deref(),
            Some(crate::console::CODE_ORDER_STALE)
        );
        // P0-E 第 4 项 (2026-08-17): 发放前拒绝统一入事件面——stale 订单
        // 的 journal 必须携带结构化拒绝码（此前只进结果栏 receipt）。
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert_eq!(rejected[0].payload["order_id"], "ORD-000009");
        assert_eq!(rejected[0].payload["phase"], "pre_issue");
        assert_eq!(rejected[0].payload["step"], "protocol");
        assert_eq!(rejected[0].payload["code"], "order_stale");
        assert_eq!(rejected[0].payload["round"], 0);
        assert_eq!(rejected[0].payload["plan_epoch"], 1);
        assert_eq!(rejected[0].payload["run_id"], "RUN-OLD");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0q 第五族（2026-09-08，ADR-0010 §14.63 裁决 ②）：订单失败收据同过
    /// 漏斗——`action_target` 身份入 `failure_agg` 聚合 + receipt 错误
    /// 信封挂载。此前订单失败不进聚合，是 0q 治本清单上的已知缺口。
    #[tokio::test]
    async fn console_order_failure_stamps_action_target_into_agg_and_receipt() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "never".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.plan.plan_epoch = 1;
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-000077".to_string(),
                    action: "workspace.run_terminal".to_string(),
                    arguments: serde_json::json!({"command": "make test"}),
                    target: None,
                    step_id: None,
                    round: 0,
                    plan_epoch: 1,
                    run_id: "RUN-OLD".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-0Q-ORDER",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "stale",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        // receipt 错误信封挂 action_target 身份（同源同刻的订单面对账物）。
        let r = controller.blackboard().read();
        assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
        let receipt = &r.actions.results[0];
        assert!(!receipt.ok);
        assert_eq!(receipt.error.as_ref().unwrap()["code"], "order_stale");
        let ft = receipt
            .error
            .as_ref()
            .unwrap()
            .get("failure_target")
            .unwrap();
        assert_eq!(ft["kind"], "action_target");
        assert_eq!(ft["id"], orz_assurance::journal::sha256_hex(b"ORD-000077"));
        assert_eq!(ft["action"], "workspace.run_terminal");

        // 聚合面：action_target 一行，code = order_stale，渲染 preview =
        // action 名（failures_only 面可见此前缺失的订单失败行）。
        let agg = &r.failure_agg;
        assert_eq!(agg.rows.len(), 1, "{agg:?}");
        let row = &agg.rows[0];
        assert_eq!(row.kind, "action_target");
        assert_eq!(row.id, orz_assurance::journal::sha256_hex(b"ORD-000077"));
        assert_eq!(row.preview, "workspace.run_terminal");
        assert_eq!(row.codes[0].code, "order_stale");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-C S2 (2026-08-15): 跨 run 防重放——run_id 不匹配的遗留订单即使
    /// round/plan_epoch 重合也按 `order_stale` 显式拒绝（不误发）。
    #[tokio::test]
    async fn console_s2_cross_run_leftover_order_is_stale() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "never".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.plan.plan_epoch = 1;
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-000010".to_string(),
                    action: "workspace.read_file".to_string(),
                    arguments: serde_json::json!({"target_file": "a.txt"}),
                    target: None,
                    step_id: None,
                    round: 1,
                    plan_epoch: 1,
                    run_id: "RUN-PREVIOUS".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-S2X",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "cross-run",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none());
        let receipt = &r.actions.results[0];
        assert!(!receipt.ok);
        assert_eq!(receipt.error.as_ref().unwrap()["code"], "order_stale");
        assert_eq!(
            receipt.error.as_ref().unwrap()["upstream"]["order_run_id"],
            "RUN-PREVIOUS"
        );
        assert_eq!(
            receipt.error.as_ref().unwrap()["upstream"]["current_run_id"],
            "RUN-S2X"
        );
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert_eq!(rejected[0].payload["order_id"], "ORD-000010");
        assert_eq!(rejected[0].payload["phase"], "pre_issue");
        assert_eq!(rejected[0].payload["code"], "order_stale");
        assert_eq!(rejected[0].payload["run_id"], "RUN-PREVIOUS");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-READ-ANCHOR-WRITE-GUARD (ADR-0010 §14.38, 2026-08-21 审查收口):
    /// search_replace 订单携带 `expected_anchor` 且目标文件内容与锚点不符
    /// 时，发放前以 `content_anchor_mismatch` 拒单——清槽、写失败 receipt、
    /// 入 console_order_rejected 事件面、零编辑。
    #[tokio::test]
    async fn console_anchor_mismatch_rejects_before_issue_with_zero_edits() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "never".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        // 目标文件：实际内容为 "actual"（size=6）；期望锚点指向 "expected"
        // （size=8）——先触发 size 快筛，再以 sha256 权威拒单。
        let target = host.session_cwd().join("guard_target.txt");
        std::fs::write(&target, "actual").unwrap();
        let expected_sha256 = sha256_hex(b"expected");
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-GUARD-001".to_string(),
                    action: "workspace.search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "guard_target.txt",
                        "old_string": "old",
                        "new_string": "new",
                        "expected_anchor": {
                            "size": 8,
                            "mtime": 0,
                            "sha256": expected_sha256,
                        },
                    }),
                    target: None,
                    step_id: None,
                    round: 1,
                    plan_epoch: 0,
                    run_id: "RUN-GUARD".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-GUARD",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "guard",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none(), "{:?}", r.actions.order);
        assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
        let receipt = &r.actions.results[0];
        assert!(!receipt.ok);
        let error = receipt.error.as_ref().unwrap();
        assert_eq!(error["step"], "protocol");
        assert_eq!(error["code"], "content_anchor_mismatch");
        assert_eq!(error["upstream"]["expected"]["size"], 8);
        // 零编辑：未执行任何 search_replace 目标工具。
        assert!(
            r.tool_actions.iter().all(|t| t.tool != "search_replace"),
            "{:?}",
            r.tool_actions
        );
        // 目标文件未被改写。
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "actual");
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert_eq!(rejected[0].payload["order_id"], "ORD-GUARD-001");
        assert_eq!(rejected[0].payload["phase"], "pre_issue");
        assert_eq!(rejected[0].payload["step"], "protocol");
        assert_eq!(rejected[0].payload["code"], "content_anchor_mismatch");
        assert_eq!(rejected[0].payload["run_id"], "RUN-GUARD");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-READ-ANCHOR-WRITE-GUARD (ADR-0010 §14.38, 2026-08-21 审查收口):
    /// 核证期 I/O 错误 fail-closed——stat 成功但 read 失败（目标路径为目录）
    /// 时同样以 `content_anchor_mismatch` 拒单，绝不带未核证内容放行编辑。
    #[tokio::test]
    async fn console_anchor_io_error_rejects_fail_closed() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "never".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        // 目标路径是一个目录：metadata 成功（size/mtime 与期望一致——
        // mtime 取目录实际值以通过快筛）但 read 失败——必须拒单而不是放行。
        let target_dir = host.session_cwd().join("guard_dir_target");
        std::fs::create_dir_all(&target_dir).unwrap();
        let dir_mtime = std::fs::metadata(&target_dir)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-GUARD-002".to_string(),
                    action: "workspace.search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "guard_dir_target",
                        "old_string": "",
                        "new_string": "x",
                        "expected_anchor": {
                            "size": 0,
                            "mtime": dir_mtime,
                            "sha256": "a".repeat(64),
                        },
                    }),
                    target: None,
                    step_id: None,
                    round: 1,
                    plan_epoch: 0,
                    run_id: "RUN-GUARD2".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-GUARD2",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "guard",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none(), "{:?}", r.actions.order);
        assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
        let receipt = &r.actions.results[0];
        assert!(!receipt.ok);
        let error = receipt.error.as_ref().unwrap();
        assert_eq!(error["step"], "protocol");
        assert_eq!(error["code"], "content_anchor_mismatch");
        // 平台差异：Linux 目录 size 非 0 → 命中 size/mtime 预检；Windows
        // 目录 size 为 0 → 通过预检后 read 失败——两种消息共享前缀。
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("content anchor verification failed"),
            "{error:?}"
        );
        assert!(
            r.tool_actions.iter().all(|t| t.tool != "search_replace"),
            "{:?}",
            r.tool_actions
        );
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert_eq!(rejected[0].payload["code"], "content_anchor_mismatch");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-READ-ANCHOR-WRITE-GUARD (ADR-0010 §14.38, S2): expected_anchor
    /// 与目标当前内容一致时核证通过——订单正常发放执行（receipt ok、
    /// tool_action 留痕、无 console_order_rejected 事件）。mtime 为 null
    /// 时跳过快筛，sha256 仍是权威。
    #[tokio::test]
    async fn console_anchor_match_allows_issue_with_mtime_null() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "edited ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let target = host.session_cwd().join("match_target.txt");
        std::fs::write(&target, b"actual").unwrap();
        let expected_sha256 = sha256_hex(b"actual");
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-GUARD-MATCH".to_string(),
                    action: "workspace.search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "match_target.txt",
                        "old_string": "actual",
                        "new_string": "updated",
                        "expected_anchor": {
                            "size": 6,
                            "mtime": null,
                            "sha256": expected_sha256,
                        },
                    }),
                    target: Some("file:match_target.txt".to_string()),
                    step_id: None,
                    round: 1,
                    plan_epoch: 0,
                    run_id: "RUN-GUARDM".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-GUARDM",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "guard",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none(), "{:?}", r.actions.order);
        assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
        let receipt = &r.actions.results[0];
        assert!(receipt.ok, "{receipt:?}");
        assert!(
            r.tool_actions.iter().any(|t| t.tool == "search_replace"),
            "{:?}",
            r.tool_actions
        );
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert!(rejected.is_empty(), "{rejected:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-READ-ANCHOR-WRITE-GUARD (ADR-0010 §14.38, S2 fixture): 同 size
    /// 同 mtime、内容不同（git checkout / cp -p / touch -r 可保留时间戳）
    /// ——快筛通过但 sha256 权威兜底拒单；错误信封 / trace / 事件面完整
    /// 断言（expected vs actual、机械盖章、re-read 指引、零编辑）。
    #[tokio::test]
    async fn console_anchor_same_mtime_different_content_sha256_catches() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "never".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let target = host.session_cwd().join("same_mtime_target.txt");
        // fixture：内容从 "aaaaaa" 改为 "bbbbbb"（size 相同），mtime 被保留。
        std::fs::write(&target, b"aaaaaa").unwrap();
        let original_mtime = std::fs::metadata(&target).unwrap().modified().unwrap();
        let original_mtime_secs = original_mtime
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        std::fs::write(&target, b"bbbbbb").unwrap();
        {
            let f = std::fs::File::options().write(true).open(&target).unwrap();
            f.set_times(std::fs::FileTimes::new().set_modified(original_mtime))
                .unwrap();
        }
        let meta = std::fs::metadata(&target).unwrap();
        assert_eq!(meta.len(), 6);
        assert_eq!(
            meta.modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            original_mtime_secs,
            "fixture must preserve mtime"
        );
        let expected_sha256 = sha256_hex(b"aaaaaa");
        let actual_sha256 = sha256_hex(b"bbbbbb");
        assert_ne!(expected_sha256, actual_sha256);
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-GUARD-003".to_string(),
                    action: "workspace.search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "same_mtime_target.txt",
                        "old_string": "old",
                        "new_string": "new",
                        "expected_anchor": {
                            "size": 6,
                            "mtime": original_mtime_secs,
                            "sha256": expected_sha256,
                        },
                    }),
                    target: None,
                    step_id: None,
                    round: 1,
                    plan_epoch: 0,
                    run_id: "RUN-GUARD3".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-GUARD3",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "guard",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none(), "{:?}", r.actions.order);
        assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
        let receipt = &r.actions.results[0];
        assert!(!receipt.ok);
        let error = receipt.error.as_ref().unwrap();
        assert_eq!(error["step"], "protocol");
        assert_eq!(error["code"], "content_anchor_mismatch");
        let message = error["message"].as_str().unwrap();
        assert!(message.contains("sha256 mismatch"), "{message}");
        assert!(message.contains(&expected_sha256), "{message}");
        assert!(message.contains(&actual_sha256), "{message}");
        assert!(message.contains("re-read the file"), "{message}");
        assert_eq!(error["upstream"]["label"], "ORD-GUARD-003");
        assert_eq!(error["upstream"]["file_path"], "same_mtime_target.txt");
        assert_eq!(error["upstream"]["expected"]["size"], 6);
        assert_eq!(error["upstream"]["expected"]["mtime"], original_mtime_secs);
        assert_eq!(error["upstream"]["expected"]["sha256"], expected_sha256);
        assert_eq!(error["upstream"]["actual"]["size"], 6);
        assert_eq!(error["upstream"]["actual"]["mtime"], original_mtime_secs);
        assert_eq!(error["upstream"]["actual"]["sha256"], actual_sha256);
        // 零编辑：文件保持 bbbbbb，未执行任何 search_replace 目标工具。
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "bbbbbb");
        assert!(
            r.tool_actions.iter().all(|t| t.tool != "search_replace"),
            "{:?}",
            r.tool_actions
        );
        // trace：失败 trace 已 commit，末事件 protocol/content_anchor_mismatch。
        let traces = controller.console_traces.lock().unwrap();
        let trace = traces
            .get(&receipt.trace_id)
            .expect("anchor rejection trace committed");
        assert_eq!(trace.events.last().unwrap().step, "protocol");
        assert_eq!(
            trace.events.last().unwrap().code.as_deref(),
            Some(crate::console::CODE_CONTENT_ANCHOR_MISMATCH)
        );
        drop(traces);
        // 事件面：pre_issue / protocol / content_anchor_mismatch + 机械盖章。
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert_eq!(rejected[0].payload["order_id"], "ORD-GUARD-003");
        assert_eq!(rejected[0].payload["phase"], "pre_issue");
        assert_eq!(rejected[0].payload["step"], "protocol");
        assert_eq!(rejected[0].payload["code"], "content_anchor_mismatch");
        assert_eq!(rejected[0].payload["round"], 1);
        assert_eq!(rejected[0].payload["plan_epoch"], 0);
        assert_eq!(rejected[0].payload["run_id"], "RUN-GUARD3");
        assert!(
            rejected[0].payload["reason"]
                .as_str()
                .unwrap()
                .contains("sha256 mismatch"),
            "{rejected:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-READ-ANCHOR-WRITE-GUARD (ADR-0010 §14.38, S2): 陈旧写入场景——
    /// 读取 v1 后文件被别处改为 v2，v1 锚点下单被拒；主 agent 重读后以 v2
    /// 锚点重下成功。发放前拒绝零编辑，重下走正常执行链（S4 场景的单元级
    /// 预演）。
    #[tokio::test]
    async fn console_anchor_reject_then_reread_reissue_succeeds() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "edited ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let target = host.session_cwd().join("remedy.txt");
        // 读取快照 v1……
        std::fs::write(&target, b"v1").unwrap();
        let v1_sha = sha256_hex(b"v1");
        // ……窗口内文件被别处改为 v2（旧锚点不再匹配）。
        std::fs::write(&target, b"v2").unwrap();
        let v2_sha = sha256_hex(b"v2");
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        async fn issue_guard_order(
            controller: &AgentLoopController,
            host: &TestHost,
            journal: &JournalRecorder,
            run_id: &str,
        ) {
            let mut writer =
                EventWriter::new(Some(journal), EventTrack::V02, run_id, "", 0, None, None);
            controller
                .issue_pending_console_order(
                    host,
                    &mut writer,
                    "guard",
                    orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                    1,
                    None,
                )
                .await
                .unwrap();
        }
        // 第一次：旧锚点 → 发放前拒绝。
        {
            let mut w = controller.blackboard().write();
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-GUARD-RE1".to_string(),
                    action: "workspace.search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "remedy.txt",
                        "old_string": "old",
                        "new_string": "new",
                        "expected_anchor": {
                            "size": 2,
                            "mtime": null,
                            "sha256": v1_sha,
                        },
                    }),
                    target: Some("file:remedy.txt".to_string()),
                    step_id: None,
                    round: 1,
                    plan_epoch: 0,
                    run_id: "RUN-REMEDY1".to_string(),
                })
                .unwrap();
        }
        issue_guard_order(&controller, &host, &journal, "RUN-REMEDY1").await;
        {
            let r = controller.blackboard().read();
            assert!(r.actions.order.is_none(), "{:?}", r.actions.order);
            assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
            let receipt = &r.actions.results[0];
            assert!(!receipt.ok);
            assert_eq!(
                receipt.error.as_ref().unwrap()["code"],
                "content_anchor_mismatch"
            );
            assert!(
                r.tool_actions.iter().all(|t| t.tool != "search_replace"),
                "{:?}",
                r.tool_actions
            );
        }
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "v2");
        // 第二次：重读后以新锚点重下 → 正常发放执行。
        {
            let mut w = controller.blackboard().write();
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-GUARD-RE2".to_string(),
                    action: "workspace.search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "remedy.txt",
                        "old_string": "old",
                        "new_string": "new",
                        "expected_anchor": {
                            "size": 2,
                            "mtime": null,
                            "sha256": v2_sha,
                        },
                    }),
                    target: Some("file:remedy.txt".to_string()),
                    step_id: None,
                    round: 1,
                    plan_epoch: 0,
                    run_id: "RUN-REMEDY2".to_string(),
                })
                .unwrap();
        }
        issue_guard_order(&controller, &host, &journal, "RUN-REMEDY2").await;
        {
            let r = controller.blackboard().read();
            assert_eq!(r.actions.results.len(), 2, "{:?}", r.actions.results);
            assert!(r.actions.results[1].ok, "{:?}", r.actions.results[1]);
            assert!(
                r.tool_actions.iter().any(|t| t.tool == "search_replace"),
                "{:?}",
                r.tool_actions
            );
        }
        // 全程恰好一次发放前拒绝（第一次），第二次无拒绝事件。
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert_eq!(rejected.len(), 1, "{rejected:?}");
        assert_eq!(rejected[0].payload["order_id"], "ORD-GUARD-RE1");
        assert_eq!(rejected[0].payload["run_id"], "RUN-REMEDY1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-READ-ANCHOR-WRITE-GUARD (ADR-0010 §14.38, S2): expected_anchor
    /// 缺失时保持既有行为——不触发核证、正常发放执行、无拒绝事件（设计
    /// §3.2「缺失保持既有行为；必填加严为可选后续」）。
    #[tokio::test]
    async fn console_search_replace_without_anchor_skips_verification() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "edited ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        std::fs::write(host.session_cwd().join("legacy_target.txt"), b"actual").unwrap();
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-GUARD-NOANCHOR".to_string(),
                    action: "workspace.search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "legacy_target.txt",
                        "old_string": "actual",
                        "new_string": "updated",
                    }),
                    target: Some("file:legacy_target.txt".to_string()),
                    step_id: None,
                    round: 1,
                    plan_epoch: 0,
                    run_id: "RUN-GUARDN".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-GUARDN",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "guard",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none(), "{:?}", r.actions.order);
        assert_eq!(r.actions.results.len(), 1, "{:?}", r.actions.results);
        assert!(r.actions.results[0].ok, "{:?}", r.actions.results[0]);
        assert!(
            r.tool_actions.iter().any(|t| t.tool == "search_replace"),
            "{:?}",
            r.tool_actions
        );
        let rejected = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ConsoleOrderRejected)
            .collect::<Vec<_>>();
        assert!(rejected.is_empty(), "{rejected:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// R2 半助理层（THIN-HARNESS-REDESIGN V2 §4.2/§4.5）：执行失败自动
    /// 派发——失败对象实体登记（file 锚点）+ 结构化签名诊断附着 +
    /// 错误信封携带 `upstream.detail.diagnostic`（≤2KB 极简记录）。
    #[tokio::test]
    async fn r2_failure_auto_diagnosis_registers_entity_and_attaches_diagnostic() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "error: no such file\nat line 1\n".to_string(),
                exit_code: Some(1),
                output_encoding: Some("utf-8".to_string()),
                structured: None,
                ..Default::default()
            }),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        controller.set_console_probe_source(
            crate::host::ToolPolicy::Interactive,
            crate::tool_probe::ToolProbeSnapshot {
                complete: vec!["read_file".to_string()],
                incomplete: Vec::new(),
            },
        );
        {
            let mut w = controller.blackboard().write();
            w.plan.plan_epoch = 1;
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-R2-001".to_string(),
                    action: "workspace.read_file".to_string(),
                    arguments: serde_json::json!({"target_file": "missing_r2_target.txt"}),
                    target: Some("file:missing_r2_target.txt".to_string()),
                    step_id: None,
                    round: 1,
                    plan_epoch: 1,
                    run_id: "RUN-R2".to_string(),
                })
                .unwrap();
        }
        let mut writer =
            EventWriter::new(Some(&journal), EventTrack::V02, "RUN-R2", "", 0, None, None);
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "r2",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();
        {
            let bb = controller.blackboard().read();
            // 失败对象实体已登记（file 锚点：exists=false）。
            let entity = bb
                .entities
                .get("file:missing_r2_target.txt")
                .expect("file entity registered");
            assert_eq!(entity.kind, crate::entities::EntityKind::File);
            assert_eq!(entity.summary["exists"], serde_json::json!(false));
            // 结构化签名诊断已附着（target_missing；P5：非文本子串）。
            let diag = entity
                .last_diagnostic
                .as_ref()
                .expect("diagnostic attached");
            assert_eq!(diag.matched_signature.as_deref(), Some("target_missing"));
            assert_eq!(diag.log_pointer, "t000001");
            assert!(
                diag.key_fields
                    .iter()
                    .any(|f| f.key == "exists" && f.value == "false")
            );
            // 环境实体已登记（探针快照机械来源）。
            let env = bb
                .entities
                .get(&crate::entities::environment_entity_id())
                .expect("environment entity");
            assert_eq!(env.summary["tool_count"], serde_json::json!(1));
            // 错误信封携带极简诊断（≤2KB）。
            let receipt = bb.actions.results.last().expect("receipt");
            assert!(!receipt.ok);
            let error = receipt.error.as_ref().expect("error envelope");
            assert_eq!(error["code"], serde_json::json!("execution_failed"));
            assert_eq!(
                error["upstream"]["detail"]["diagnostic"]["matched_signature"],
                serde_json::json!("target_missing")
            );
        }
        // blackboard_read section=entities 有界渲染（摘要清单 + 总上限）。
        let text = controller.render_blackboard_section("entities", None, None, None);
        assert!(text.contains("file:missing_r2_target.txt"), "{text}");
        assert!(text.contains("total="), "{text}");
        assert!(text.contains("has_diagnostic=true"), "{text}");
        let epoch_reject = controller.render_blackboard_section("entities", None, Some(1), None);
        assert!(epoch_reject.contains("live-only"), "{epoch_reject}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// R2 半助理层：成功路径登记文件锚点（size/sha256/encoding）。
    #[tokio::test]
    async fn r2_success_registers_file_anchor_with_sha256() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let anchor_path = dir.join("anchor_target.txt");
        std::fs::write(&anchor_path, b"r2 anchor content").unwrap();
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "r2 anchor content".to_string(),
                exit_code: Some(0),
                output_encoding: Some("utf-8".to_string()),
                structured: None,
                ..Default::default()
            }),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.plan.plan_epoch = 1;
            w.actions
                .write_order(ActionOrder {
                    order_id: "ORD-R2-002".to_string(),
                    action: "workspace.read_file".to_string(),
                    arguments: serde_json::json!({
                        "target_file": anchor_path.to_string_lossy(),
                    }),
                    target: Some(crate::entities::file_entity_id(
                        &anchor_path.to_string_lossy(),
                    )),
                    step_id: None,
                    round: 1,
                    plan_epoch: 1,
                    run_id: "RUN-R2S".to_string(),
                })
                .unwrap();
        }
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-R2S",
            "",
            0,
            None,
            None,
        );
        controller
            .issue_pending_console_order(
                &host,
                &mut writer,
                "r2",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                1,
                None,
            )
            .await
            .unwrap();
        {
            let bb = controller.blackboard().read();
            let entity = bb
                .entities
                .get(&crate::entities::file_entity_id(
                    &anchor_path.to_string_lossy(),
                ))
                .expect("file entity registered");
            assert_eq!(entity.summary["exists"], serde_json::json!(true));
            assert_eq!(entity.summary["size"], serde_json::json!(17));
            assert_eq!(
                entity.summary["encoding"],
                serde_json::json!("utf-8"),
                "entity summary: {}",
                entity.summary
            );
            let expected_sha = sha256_hex(b"r2 anchor content");
            assert_eq!(entity.summary["sha256"], serde_json::json!(expected_sha));
            assert!(entity.last_diagnostic.is_none());
            let receipt = bb.actions.results.last().expect("receipt");
            assert!(receipt.ok, "{receipt:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-ENCODING-GATE 回归：run_host_tool 成功路径重建 ToolResult 时
    /// 必须透传 output_encoding（R2 实体登记/失败诊断的 encoding_lossy
    /// 签名依赖该结构化字段）。
    #[tokio::test]
    async fn r2_console_target_preserves_output_encoding() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "x".to_string(),
                exit_code: Some(0),
                output_encoding: Some("utf-8".to_string()),
                structured: None,
                ..Default::default()
            }),
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        let mut writer =
            EventWriter::new(Some(&journal), EventTrack::V02, "RUN-P", "", 0, None, None);
        let r = controller
            .run_console_target(
                &host,
                &mut writer,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::NotObserved,
                0,
                None,
                None,
                "read_file",
                &serde_json::json!({ "path": "x.txt" }),
                "call-p",
                "t000001",
            )
            .await
            .expect("run");
        assert_eq!(
            r.output_encoding.as_deref(),
            Some("utf-8"),
            "output_encoding preserved through console target"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-C S3 前置 (2026-08-15, P1-2 定案): 适配层只按结构化信号映射——
    /// permission / acaf / retrieval_mode / taint 全部落到
    /// `ExecuteError::PolicyDenied`，detail 携带 source/code/reason。
    #[tokio::test]
    async fn console_s2_structured_denial_sources_map_to_policy_step() {
        use crate::host::PolicyDenialSource;

        let controller = AgentLoopController::new();
        for (source, code) in [
            (PolicyDenialSource::Permission, "permission_deny"),
            (
                PolicyDenialSource::Acaf,
                "control_ticket_rejected:missing_goal_context",
            ),
            (PolicyDenialSource::RetrievalMode, "retrieval_mode_off"),
            (PolicyDenialSource::Taint, "taint_denied"),
        ] {
            let dir = test_dir();
            let journal = JournalRecorder::new(dir.clone());
            let host = TestHost {
                journal: journal.clone(),
                tool_result: Some(ToolResult {
                    output: format!("denied: {code}"),
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    timed_out: false,
                    policy_denial: Some(crate::host::PolicyDenial {
                        source,
                        code: code.to_string(),
                        reason: "policy reason".to_string(),
                    }),
                    ..Default::default()
                }),
            };
            let mut writer = EventWriter::new(
                Some(&journal),
                EventTrack::V02,
                "RUN-S3PD",
                "",
                0,
                None,
                None,
            );
            let err = controller
                .run_console_target(
                    &host,
                    &mut writer,
                    "",
                    orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                    0,
                    None,
                    None,
                    "read_file",
                    &serde_json::json!({ "path": "a.txt" }),
                    "call-pd",
                    "t000001",
                )
                .await
                .unwrap_err();
            match err {
                crate::console::ExecuteError::PolicyDenied { message, detail } => {
                    assert!(message.contains("denied:"), "{message}");
                    let detail = detail.expect("structured detail");
                    assert_eq!(detail["source"], serde_json::json!(source.as_str()));
                    assert_eq!(detail["code"], serde_json::json!(code));
                    assert_eq!(detail["reason"], serde_json::json!("policy reason"));
                }
                other => panic!("expected PolicyDenied for {code}, got {other:?}"),
            }
            // P0-C S3 前置审查修复 (F3): a host-level denial must ride the
            // ToolCompleted as a self-describing refusal completion —
            // exit_code=1 + status=error + error code + structured denial.
            let all_events = events(&dir);
            let completed: Vec<_> = all_events
                .iter()
                .filter(|e| e.event_type == EventType::ToolCompleted)
                .collect();
            assert_eq!(completed.len(), 1, "one refusal completion expected");
            assert_eq!(completed[0].payload["exit_code"], serde_json::json!(1));
            assert_eq!(completed[0].payload["status"], serde_json::json!("error"));
            assert_eq!(completed[0].payload["error"], serde_json::json!(code));
            assert_eq!(
                completed[0].payload["policy_denial"]["source"],
                serde_json::json!(source.as_str())
            );
            assert_eq!(
                completed[0].payload["policy_denial"]["code"],
                serde_json::json!(code)
            );
            assert_eq!(
                completed[0].payload["policy_denial"]["reason"],
                serde_json::json!("policy reason")
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// P0-C S3 前置 (2026-08-15, P1-2 定案): 内容碰撞回归——成功输出即使
    /// 包含旧拒绝前缀文案也必须判成功（策略判定只认结构化信号，不认文案）。
    #[tokio::test]
    async fn console_s2_old_refusal_prefix_in_success_output_is_not_policy() {
        let controller = AgentLoopController::new();
        for output in [
            "ACAF ticket refused for 'search_replace' — denied (missing_goal_context); \
             the action was not executed.",
            "retrieval 'project_doc_index' refused — retrieval mode is 'off' for this \
             session; no retrieval tools are available.",
            "retrieval 'browser_read' refused — retrieval mode is 'local_browser' for \
             this session; web tools require framework_fallback mode; no silent fallback.",
            "tool 'read_file' — 本次调用未获权限门禁放行",
        ] {
            let dir = test_dir();
            let journal = JournalRecorder::new(dir.clone());
            let host = TestHost {
                journal: journal.clone(),
                tool_result: Some(ToolResult {
                    output: output.to_string(),
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: None,
                    timed_out: false,
                    policy_denial: None,
                    ..Default::default()
                }),
            };
            let mut writer = EventWriter::new(
                Some(&journal),
                EventTrack::V02,
                "RUN-S3CO",
                "",
                0,
                None,
                None,
            );
            let result = controller
                .run_console_target(
                    &host,
                    &mut writer,
                    "",
                    orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                    0,
                    None,
                    None,
                    "read_file",
                    &serde_json::json!({ "path": "a.txt" }),
                    "call-co",
                    "t000001",
                )
                .await
                .expect("old-prefix success output must not be classified as policy");
            assert_eq!(result.exit_code, Some(0));
            assert!(result.policy_denial.is_none());
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// 设计 §5 验收 2：第 2 轮起 direct 执行面——脚本化运行只调工作工具；
    /// journal 中不得出现 blackboard_action_write / console_step_done /
    /// console_return_to_console（退役工具零调用残留）。
    #[tokio::test]
    async fn direct_surface_journal_never_contains_console_order_tools() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "edited".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "search_replace".to_string(),
                arguments: serde_json::json!({ "file_path": "a.txt", "old_string": "x", "new_string": "y" }),
                call_id: "call-edit-1".to_string(),
            }]),
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let controller = stage_c_controller(fake);
        controller
            .run_turn(&host, "改文件", "RUN-DIRECT", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let started_tools: Vec<&str> = events
            .iter()
            .filter(|e| e.event_type == EventType::ToolStarted)
            .filter_map(|e| e.payload.get("tool").and_then(serde_json::Value::as_str))
            .collect();
        for retired in [
            "blackboard_action_write",
            "console_step_done",
            "console_return_to_console",
        ] {
            assert!(
                !started_tools.contains(&retired),
                "retired console order tool {retired} must never appear in the journal: {started_tools:?}"
            );
        }
        assert!(
            started_tools.contains(&"search_replace"),
            "the direct work tool executes: {started_tools:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24, ADR-0010 §14.39):
    //    direct 面 read-anchor 写前核证 + 退役工具调用面窄门 ────────────

    /// P1-1 (审查处理): direct 面 `search_replace` 携带错误 `expected_anchor`
    /// → 执行前机械拒绝（content_anchor_mismatch、无 ToolStarted、零编辑），
    /// 审计层记录锚点拒单异常事实。
    #[tokio::test]
    async fn direct_search_replace_wrong_anchor_refuses_without_execution() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ok_result()),
        };
        let target = host.session_cwd().join("anchor_target.txt");
        std::fs::write(&target, "current").unwrap();
        let meta = std::fs::metadata(&target).unwrap();
        let wrong_sha = "0".repeat(64);
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "search_replace".to_string(),
                arguments: serde_json::json!({
                    "file_path": "anchor_target.txt",
                    "old_string": "current",
                    "new_string": "edited",
                    "expected_anchor": {
                        "size": meta.len(),
                        "mtime": meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs()),
                        "sha256": wrong_sha,
                    },
                }),
                call_id: "call-edit-1".to_string(),
            }]),
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let controller = stage_c_controller(fake);
        controller
            .run_turn(
                &host,
                "改文件",
                "RUN-ANCHOR-DIRECT",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 零编辑：文件保持原内容，host 工具未被调用。
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "current");
        let r = controller.blackboard().read();
        assert!(
            r.tool_actions.iter().all(|t| t.tool != "search_replace"),
            "{:?}",
            r.tool_actions
        );
        // 事件面：ToolCompleted 拒绝（content_anchor_mismatch），无 ToolStarted。
        let events = events(&dir);
        let refused: Vec<&RunEvent> = events
            .iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .filter(|e| e.payload.get("tool").and_then(|t| t.as_str()) == Some("search_replace"))
            .collect();
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert_eq!(refused[0].payload["exit_code"].as_u64(), Some(1));
        assert_eq!(
            refused[0].payload["error"].as_str(),
            Some("content_anchor_mismatch")
        );
        assert!(
            !events.iter().any(|e| e.event_type == EventType::ToolStarted
                && e.payload.get("tool").and_then(|t| t.as_str()) == Some("search_replace")),
            "no ToolStarted for the refused edit"
        );
        // 审计层：锚点拒单异常事实（结构化 error 透传）。
        let audit: Vec<&RunEvent> = events
            .iter()
            .filter(|e| e.event_type == EventType::MechanicalAuditUpdate)
            .filter(|e| e.payload["kind"] == "tool_result")
            .filter(|e| {
                e.payload["payload"]["key"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("file:anchor_target.txt")
            })
            .collect();
        assert!(
            !audit.is_empty(),
            "anchor refusal must be audited: {events:?}"
        );
        assert_eq!(
            audit[0].payload["payload"]["anomaly"].as_str(),
            Some("content_anchor_mismatch")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P1-1 (审查处理): direct 面 `search_replace` 携带正确锚点 / 不带锚点
    /// → 照常执行（核证通过或无核证要求）；目标不存在（新建路径）携带锚点
    /// → 跳过核证照常执行。
    #[tokio::test]
    async fn direct_search_replace_anchor_match_or_missing_target_executes() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ok_result()),
        };
        let target = host.session_cwd().join("anchor_match.txt");
        std::fs::write(&target, "current").unwrap();
        let bytes = std::fs::read(&target).unwrap();
        let meta = std::fs::metadata(&target).unwrap();
        let sha = sha256_hex(&bytes);
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            // 同批三次调用：正确锚点 / 无锚点 / 目标不存在（新建路径）。
            ScriptedResponse::tool_calls(vec![
                ToolCall {
                    name: "search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "anchor_match.txt",
                        "old_string": "current",
                        "new_string": "edited",
                        "expected_anchor": {
                            "size": meta.len(),
                            "mtime": meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs()),
                            "sha256": sha,
                        },
                    }),
                    call_id: "call-edit-match".to_string(),
                },
                ToolCall {
                    name: "search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "anchor_match.txt",
                        "old_string": "current",
                        "new_string": "edited",
                    }),
                    call_id: "call-edit-noanchor".to_string(),
                },
                ToolCall {
                    name: "search_replace".to_string(),
                    arguments: serde_json::json!({
                        "file_path": "brand_new.txt",
                        "old_string": "",
                        "new_string": "content",
                        "expected_anchor": {
                            "size": 0,
                            "sha256": "0".repeat(64),
                        },
                    }),
                    call_id: "call-edit-new".to_string(),
                },
            ]),
            ScriptedResponse::text("草稿"),
            // COUNTEREXAMPLE_GATE 终答前一次反例自查注入占一轮模型回答。
            ScriptedResponse::text("草稿（反例自查）。"),
            ScriptedResponse::text("终答"),
        ]));
        let controller = stage_c_controller(fake);
        controller
            .run_turn(
                &host,
                "改文件",
                "RUN-ANCHOR-OK",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let completed: Vec<&RunEvent> = events
            .iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .filter(|e| e.payload.get("tool").and_then(|t| t.as_str()) == Some("search_replace"))
            .collect();
        assert_eq!(completed.len(), 3, "{completed:?}");
        assert!(
            completed
                .iter()
                .all(|e| e.payload["exit_code"].as_u64() == Some(0)),
            "{completed:?}"
        );
        assert!(
            !events.iter().any(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("content_anchor_mismatch")
            }),
            "no anchor refusal on the matching path"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P1-2 (审查处理): 三个退役 console 订单工具（blackboard_action_write /
    /// console_step_done / console_return_to_console）在调用面被机械拒绝——
    /// 零 ToolStarted、零 console_order_written 事件、动作栏无订单残留、
    /// run 正常完成。
    #[tokio::test]
    async fn retired_console_tools_refused_with_zero_side_effects() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ok_result()),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::tool_calls(vec![
                ToolCall {
                    name: "blackboard_action_write".to_string(),
                    arguments: serde_json::json!({
                        "action": "workspace.search_replace",
                        "step_id": "s1",
                        "arguments": {"file_path": "a.txt", "old_string": "v1", "new_string": "v2"},
                    }),
                    call_id: "call-retired-1".to_string(),
                },
                ToolCall {
                    name: "console_step_done".to_string(),
                    arguments: serde_json::json!({
                        "step_id": "s1",
                        "transition_id": "t1",
                        "trace_id": "tr1",
                    }),
                    call_id: "call-retired-2".to_string(),
                },
                ToolCall {
                    name: "console_return_to_console".to_string(),
                    arguments: serde_json::json!({}),
                    call_id: "call-retired-3".to_string(),
                },
            ]),
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let controller = stage_c_controller(fake);
        controller
            .run_turn(
                &host,
                "幻觉调用退役工具",
                "RUN-RETIRED",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        for name in [
            "blackboard_action_write",
            "console_step_done",
            "console_return_to_console",
        ] {
            let completed: Vec<&RunEvent> = events
                .iter()
                .filter(|e| e.event_type == EventType::ToolCompleted)
                .filter(|e| e.payload.get("tool").and_then(|t| t.as_str()) == Some(name))
                .collect();
            assert_eq!(completed.len(), 1, "{name}: {completed:?}");
            assert_eq!(completed[0].payload["exit_code"].as_u64(), Some(1));
            assert_eq!(
                completed[0].payload["error"].as_str(),
                Some("retired_tool_denied"),
                "{name}"
            );
            assert!(
                !events.iter().any(|e| e.event_type == EventType::ToolStarted
                    && e.payload.get("tool").and_then(|t| t.as_str()) == Some(name)),
                "{name} must never reach ToolStarted"
            );
        }
        // 零订单残留：没有 console_order_written 事件，动作栏无 pending 订单。
        assert!(
            !events
                .iter()
                .any(|e| e.event_type == EventType::ConsoleOrderWritten),
            "retired tools must not produce console orders"
        );
        let r = controller.blackboard().read();
        assert!(r.actions.order.is_none(), "{:?}", r.actions.order);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.1 边界项): 边界三项
    /// `todo_write` / `update_goal` / `compaction_whitelist_add` 调用一律
    /// 结构化拒绝（sealed_tool_denied、无 ToolStarted、零副作用——
    /// whitelist 不落盘、goal 不变）。
    #[tokio::test]
    async fn sealed_boundary_tools_refused_with_zero_side_effects() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ok_result()),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![
                ToolCall {
                    name: "todo_write".to_string(),
                    arguments: serde_json::json!({"todos": [{"content": "x"}]}),
                    call_id: "call-sealed-1".to_string(),
                },
                ToolCall {
                    name: "update_goal".to_string(),
                    arguments: serde_json::json!({"goal": "g"}),
                    call_id: "call-sealed-2".to_string(),
                },
                ToolCall {
                    name: "compaction_whitelist_add".to_string(),
                    arguments: serde_json::json!({"content": "事实"}),
                    call_id: "call-sealed-3".to_string(),
                },
            ]),
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        // R1 审查处理：封存窄门与 console/plan 面无关——用普通控制器
        // （无 plan_first/console）保持测试聚焦于 sealed_tool_denied。
        let controller = AgentLoopController::with_gateway(fake);
        controller
            .run_turn(
                &host,
                "幻觉调用封存工具",
                "RUN-SEALED",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        for name in ["todo_write", "update_goal", "compaction_whitelist_add"] {
            let completed: Vec<&RunEvent> = events
                .iter()
                .filter(|e| e.event_type == EventType::ToolCompleted)
                .filter(|e| e.payload.get("tool").and_then(|t| t.as_str()) == Some(name))
                .collect();
            assert_eq!(completed.len(), 1, "{name}: {completed:?}");
            assert_eq!(completed[0].payload["exit_code"].as_u64(), Some(1));
            assert_eq!(
                completed[0].payload["error"].as_str(),
                Some("sealed_tool_denied"),
                "{name}"
            );
            assert!(
                !events.iter().any(|e| e.event_type == EventType::ToolStarted
                    && e.payload.get("tool").and_then(|t| t.as_str()) == Some(name)),
                "{name} must never reach ToolStarted"
            );
        }
        // 零副作用：whitelist 空、goal digest 未变（仍为任务 prompt 摘要）。
        let w = controller.whitelist.lock().unwrap();
        assert!(w.is_empty(), "sealed whitelist write must not land: {w:?}");
        drop(w);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / 设计 §2.1)：
    /// 第 2 轮起 direct 执行面——首轮仍只暴露 blackboard_read+plan_write；
    /// 第 2 轮起退役 console 订单控制工具（blackboard_action_write /
    /// console_step_done / console_return_to_console 不再声明），工作工具
    /// 按探针面直接暴露（本 TestHost 探针仅完整链工具可见，见投影单测）。
    #[tokio::test]
    async fn direct_surface_retires_console_order_tools() {
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = stage_c_controller(gateway);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-CSD",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let first_tools: Vec<String> = received[0].tools.iter().map(|t| t.name.clone()).collect();
        assert_eq!(
            first_tools,
            vec!["blackboard_read", "plan_write"],
            "first round must expose only the plan-round surface"
        );
        let direct_tools: Vec<String> = received[1].tools.iter().map(|t| t.name.clone()).collect();
        assert!(
            direct_tools.iter().any(|t| t == "submit"),
            "submit retained on the direct surface: {direct_tools:?}"
        );
        for tool in [
            "blackboard_action_write",
            "console_step_done",
            "console_return_to_console",
        ] {
            assert!(
                !direct_tools.iter().any(|t| t == tool),
                "retired console order tool {tool} must not be declared: {direct_tools:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 设计 §2.2 + 审查处理 O2：terminal 判定按末步 id（{deliver, submit}）
    /// ——旧/恢复计划（末步为普通工作 id）保持 S1 前语义：普通订单 receipt
    /// 仍自动置末步 done，不被隐式困在递交门。
    #[test]
    fn legacy_plan_last_step_keeps_auto_advance_on_order_receipt() {
        let controller = stage_c_controller(Arc::new(FakeProvider::new(Vec::new())));
        {
            let mut w = controller.blackboard().write();
            w.plan.plan_id = Some("plan-legacy".to_string());
            w.plan.plan_epoch = 1;
            w.plan.steps.push(crate::blackboard::PlanStep {
                id: "s1".to_string(),
                goal: "g".to_string(),
                actions: Vec::new(),
                acceptance: "a".to_string(),
                evidence: Vec::new(),
                status: crate::blackboard::StepStatus::Done(crate::blackboard::DoneEvidence {
                    receipt_id: "ORD-1".to_string(),
                    direct: None,
                }),
            });
            w.plan.steps.push(crate::blackboard::PlanStep {
                id: "s2".to_string(),
                goal: "g2".to_string(),
                actions: Vec::new(),
                acceptance: "a".to_string(),
                evidence: Vec::new(),
                status: crate::blackboard::StepStatus::InProgress,
            });
        }
        let order = crate::blackboard::ActionOrder {
            order_id: "ORD-LEGACY".to_string(),
            action: "workspace.read_file".to_string(),
            arguments: serde_json::json!({"target_file": "a.txt"}),
            target: None,
            step_id: Some("s2".to_string()),
            round: 0,
            plan_epoch: 1,
            run_id: "RUN-LEGACY".to_string(),
        };
        controller.record_console_receipt(&order, true, "s2", None, true);
        let r = controller.blackboard().read();
        assert!(
            r.plan.steps[1].status.is_done(),
            "legacy last step must auto-advance on an ordinary receipt: {:?}",
            r.plan.steps[1].status
        );
    }
}
