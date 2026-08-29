//! Host tool execution family — run_host_tool 三件套 + candidate gate —
//! batch N2 of the controller split second round
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29 §3.5).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use orz_assurance::EventType;

use serde_json::Value;

use std::sync::Mutex;

use crate::blackboard::{ActionOrder, EditRecord, ToolActionRecord};
use crate::console::CODE_CONTENT_ANCHOR_MISMATCH;
use crate::controller::{
    AgentLoopController, AgentLoopError, CandidateGateDecision, DenialKey, EventWriter,
    PolicyFeedback, RetrievalMode, TicketGate, candidate_tool_prefix, chrono_utc_now,
    commit_candidate, compose_test_output_message,
};
use crate::gateway::model::{Message, Role, ToolCall};
use crate::host::{
    LoopHost, PermitDecision, PolicyDenial, PolicyDenialSource, ToolError, ToolResult,
};
use crate::tool::ToolDispatcher;

impl AgentLoopController {
    /// Run a host tool call through the permission and execution gates.
    /// (IP3a IPG evaluation is hoisted to the controller's tool phase — a
    /// block ends the whole phase without further model calls.)
    #[allow(clippy::too_many_arguments)] // mirrors run_turn_with_cancel + the retrieval dispatch lane contract
    pub(crate) async fn run_host_tool(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
        tc: &ToolCall,
        _prompt: &str,
        _workspace_trust: orz_assurance::gates::ipg::WorkspaceTrust,
        messages: &mut Vec<Message>,
        tool_rounds: u32,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        // ACAF Slice 2 fail-closed D-13 (2026-08-13): the current lane's
        // activation (retrieval lanes bind web_fetch etc.; the main lane is
        // None). Threaded from the loop profile.
        activation_id: Option<&str>,
        // FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the current
        // dispatch's candidate counter (per-activation shared domain,
        // threaded from the loop profile — web_fetch family + browser_read).
        // None on main/grill — the candidate gate fails closed without a
        // count domain.
        fetch_candidates: Option<&Mutex<Vec<String>>>,
        // C2-1 (2026-08-11): whether the per-call permission bridge is
        // consulted. The main lane passes `true`; retrieval-lane
        // self-execution (web tools inside a retrieval lane) passes
        // `false` — the explicit retrieval-mode gate (§3.7.1) is its
        // authorization chain (2026-08-11 user adjudication).
        permission_gated: bool,
        // P0-A step 5 review fix (2026-08-13): whether this lane owns the
        // work-tool probe map. Main/grill lanes pass `true` — a real call
        // failure writes back (调用即探针). Retrieval lanes pass `false`:
        // they never re-probe and must NOT pollute the main probe map with
        // lane-local failures (review: cross-lane write-back would surface
        // as spurious recovery-flip events in the main audit stream).
        probe_writeback: bool,
    ) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
        self.run_host_tool_with_plan_gate(
            host,
            writer,
            tc,
            _prompt,
            _workspace_trust,
            messages,
            tool_rounds,
            heartbeat,
            activation_id,
            fetch_candidates,
            permission_gated,
            probe_writeback,
            None,
            None,
        )
        .await
    }

    /// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): `run_host_tool`
    /// with the first-round plan gate's current submission attempt (1 =
    /// first, 2 = refill). `None` = plan_write outside the gate (plan
    /// revision) — invalid plans are rejected without a forced refill
    /// round. Only the main loop calls this; test and console call sites
    /// keep the plain 12-argument form.
    #[allow(clippy::too_many_arguments)] // mirrors run_host_tool's contract
    pub(crate) async fn run_host_tool_with_plan_gate(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
        tc: &ToolCall,
        _prompt: &str,
        _workspace_trust: orz_assurance::gates::ipg::WorkspaceTrust,
        messages: &mut Vec<Message>,
        tool_rounds: u32,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        activation_id: Option<&str>,
        fetch_candidates: Option<&Mutex<Vec<String>>>,
        permission_gated: bool,
        probe_writeback: bool,
        plan_gate_attempt: Option<u32>,
        console_direct: Option<&crate::console_mode::DirectStamp>,
    ) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
        self.run_host_tool_with_timeout(
            host,
            writer,
            tc,
            _prompt,
            _workspace_trust,
            messages,
            tool_rounds,
            heartbeat,
            activation_id,
            fetch_candidates,
            permission_gated,
            probe_writeback,
            plan_gate_attempt,
            console_direct,
            None,
        )
        .await
    }

    /// P0-C S4 (2026-08-16): same gate chain as `run_host_tool` with a
    /// per-call host timeout override (script step deadlines). `None`
    /// behaves exactly like `run_host_tool`.
    #[allow(clippy::too_many_arguments)] // mirrors run_turn_with_cancel + the retrieval dispatch lane contract
    pub(crate) async fn run_host_tool_with_timeout(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
        tc: &ToolCall,
        _prompt: &str,
        _workspace_trust: orz_assurance::gates::ipg::WorkspaceTrust,
        messages: &mut Vec<Message>,
        tool_rounds: u32,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        activation_id: Option<&str>,
        fetch_candidates: Option<&Mutex<Vec<String>>>,
        permission_gated: bool,
        probe_writeback: bool,
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): the first-round
        // plan gate's current submission attempt (1 = first, 2 = refill);
        // `None` = plan revision outside the gate.
        plan_gate_attempt: Option<u32>,
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): direct 模式
        // 直接动作盖章——ToolStarted/ToolCompleted 携带 console_mode:
        // "direct" + transition_id + trace_id（事件链关联；§7.4）。
        // console 发放链（assistant 层）与普通路径传 None。
        console_direct: Option<&crate::console_mode::DirectStamp>,
        timeout: Option<std::time::Duration>,
    ) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): direct 模式
        // 直接动作事件盖章——所有 ToolStarted/ToolCompleted 携带
        // console_mode:"direct" + transition_id + trace_id（§7.4）。
        let stamp_direct = |payload: &mut Value| {
            if let Some(direct) = console_direct {
                direct.apply(payload);
            }
        };
        // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24, ADR-0010 §14.39 /
        // 设计 §2.5)：console 订单面退役工具的调用面窄门——
        // `blackboard_action_write` / `console_step_done` /
        // `console_return_to_console` 不再声明且不再可调用（休眠 handler
        // 不可达），模型幻觉调用一律机械拒绝（无 ToolStarted、零副作用、
        // 零 console_order_written/rejected 事件）；拒绝计入连败熔断
        // （与旧 console belt-and-braces 门同反馈形态）。
        if matches!(
            tc.name.as_str(),
            "blackboard_action_write" | "console_step_done" | "console_return_to_console"
        ) {
            let msg = format!(
                "tool '{}' — 已退役，不再可用（console 订单面已由 direct 执行面取代；\
                 直接调用工作工具即可）",
                tc.name,
            );
            let mut completed = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "status": "error",
                "error": "retired_tool_denied",
            });
            stamp_direct(&mut completed);
            writer.record(EventType::ToolCompleted, completed).await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: Some(serde_json::json!({
                        "error": "retired_tool_denied",
                    })),
                    ..Default::default()
                },
                Some(PolicyFeedback::Denied(DenialKey {
                    tool_name: tc.name.clone(),
                    reason_code: "retired_tool_denied".to_string(),
                    policy_revision: self
                        .policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                })),
            ));
        }
        // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.1 边界项/删除项;
        // 审查 P2-2 收口 2026-08-27): 封存工具调用面窄门——边界三项
        // `todo_write` / `update_goal` / `compaction_whitelist_add` 与
        // host 层执行特例 `project_doc_index` / `pdf_read` 均不再向模型
        // 声明，调用一律结构化拒绝（无 ToolStarted、零副作用；拒绝计入
        // 连败熔断，与退役工具窄门同反馈形态）。代码与独立模块保留，
        // 可经配置恢复（A/B 观察后裁决）。`list_dir` / `run_tests` /
        // `search_tool` 走既有名字路由拒绝（registry 未声明 → unknown
        // tool；run_tests 休眠执行路径保留，R3 统一裁决）。
        if matches!(
            tc.name.as_str(),
            "todo_write"
                | "update_goal"
                | "compaction_whitelist_add"
                | "project_doc_index"
                | "pdf_read"
        ) {
            let msg = format!(
                "tool '{}' — 已封存，不再可用（THIN-HARNESS-REDESIGN R1；\
                 直接调用工作工具即可；代码保留为休眠模块，可经配置恢复）",
                tc.name,
            );
            let mut completed = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "status": "error",
                "error": "sealed_tool_denied",
            });
            stamp_direct(&mut completed);
            writer.record(EventType::ToolCompleted, completed).await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: Some(serde_json::json!({
                        "error": "sealed_tool_denied",
                    })),
                    ..Default::default()
                },
                Some(PolicyFeedback::Denied(DenialKey {
                    tool_name: tc.name.clone(),
                    reason_code: "sealed_tool_denied".to_string(),
                    policy_revision: self
                        .policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                })),
            ));
        }
        // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24, ADR-0010 §14.39 /
        // FUS-READ-ANCHOR-WRITE-GUARD)：read-anchor 写前核证的 direct 面
        // 落点——`search_replace` 直接调用携带 `expected_anchor`（read_file
        // 返回的 {size, mtime, sha256}）时，执行前机械核证目标文件内容锚点
        // （复用订单链 `verify_content_anchor`：stat 快筛 size/mtime +
        // sha256 权威；目标不存在=新建路径跳过；其余 I/O 错误 fail-closed）。
        // 不匹配返回结构化 `content_anchor_mismatch` 拒绝、不执行、无
        // ToolStarted（与既有发放前拒绝同形）；审计层按该结构化字段记录
        // 锚点拒单异常事实。
        if tc.name == "search_replace"
            && let Some(anchor) = tc.arguments.get("expected_anchor")
            && let Some(file_path) = tc
                .arguments
                .get("file_path")
                .and_then(serde_json::Value::as_str)
            && let Some(err) = self
                .verify_content_anchor(host, file_path, anchor, &tc.call_id)
                .await
        {
            let msg = err.message.clone();
            let mut completed = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "status": "error",
                "error": CODE_CONTENT_ANCHOR_MISMATCH,
                "file_path": file_path,
                "reason": err.upstream,
            });
            stamp_direct(&mut completed);
            writer.record(EventType::ToolCompleted, completed).await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: Some(serde_json::json!({
                        "error": CODE_CONTENT_ANCHOR_MISMATCH,
                        "file_path": file_path,
                    })),
                    ..Default::default()
                },
                None,
            ));
        }
        // The second tuple element is a pending policy feedback (a denial
        // key) that the caller aggregates at the END of the whole tool round
        // — the breaker user message must be injected after the tool batch
        // (a Role::User message inserted between the assistant declaration
        // and the tool replies violates the provider protocol (400,
        // 2026-08-07 wordy); ADR-0010 §3.5.4 counts rounds, not calls, so
        // the aggregation belongs at round granularity anyway.
        // H1 (review 2026-08-10): the host-routed retrieval tool is gated by
        // the mode=off refusal — same no-ToolStarted shape as the subagent
        // dispatch gate (the verifier's mode rule forbids retrieval dispatch
        // after a transition to off; the refusal is the ToolCompleted(error)
        // alone).
        if self.retrieval_mode == RetrievalMode::Off
            && (crate::relay::is_retrieval_mode_gated_host_tool(&tc.name)
                // C2-1 (2026-08-11): the web family joins the off gate —
                // lane self-execution routes web tools through the host
                // path, so "off means no retrieval tools" must cover them
                // here too (belt and braces over the dispatch gate).
                || crate::relay::is_web_retrieval_tool(&tc.name))
        {
            let msg = format!(
                "retrieval '{}' refused — retrieval mode is 'off' for this \
                 session (ADR-0010 §3.7.1); no retrieval tools are available.",
                tc.name,
            );
            let target = if crate::relay::is_web_retrieval_tool(&tc.name) {
                "external_retrieval"
            } else {
                "internal_retrieval"
            };
            let code = "retrieval_mode_off";
            let policy_denial = PolicyDenial {
                source: PolicyDenialSource::RetrievalMode,
                code: code.to_string(),
                reason: msg.clone(),
            };
            let mut payload = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "target": target,
                "status": "error",
                "error": code,
                "policy_denial": {
                    "source": "retrieval_mode",
                    "code": code,
                    "reason": msg,
                },
            });
            stamp_direct(&mut payload);
            writer.record(EventType::ToolCompleted, payload).await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    policy_denial: Some(policy_denial),
                    timed_out: false,
                    ..Default::default()
                },
                None,
            ));
        }
        // C2-1 (2026-08-11): the mirror lane gate for the web family —
        // web tools are only reachable in framework_fallback mode (the
        // web-tool lane). Without this gate, lane self-execution would let
        // web tools run under local_browser (cross-lane), violating the
        // explicit mode semantics (ADR-0010 §3.7.1). No ToolStarted — same
        // refusal shape as the off gate and the browser_read gate.
        if crate::relay::is_web_retrieval_tool(&tc.name)
            && self.retrieval_mode != RetrievalMode::FrameworkFallback
        {
            let msg = format!(
                "retrieval '{}' refused — retrieval mode is '{}' for this \
                 session; web tools require framework_fallback mode \
                 (ADR-0010 §3.7.1); no silent fallback to the browser lane.",
                tc.name,
                self.retrieval_mode.as_str(),
            );
            let code = "retrieval_mode_requires_framework_fallback";
            let policy_denial = PolicyDenial {
                source: PolicyDenialSource::RetrievalMode,
                code: code.to_string(),
                reason: msg.clone(),
            };
            let mut payload = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "target": "external_retrieval",
                "status": "error",
                "error": code,
                "policy_denial": {
                    "source": "retrieval_mode",
                    "code": code,
                    "reason": msg,
                },
            });
            stamp_direct(&mut payload);
            writer.record(EventType::ToolCompleted, payload).await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    policy_denial: Some(policy_denial),
                    timed_out: false,
                    ..Default::default()
                },
                None,
            ));
        }
        // local_browser (2026-08-10): `browser_read` is only reachable in
        // local_browser mode — framework_fallback is the web-tool lane and
        // the model must not cross lanes (ADR-0010 §3.7.1; the off case was
        // already refused above via the gated-host-tool family). No
        // ToolStarted — same refusal shape as the off gate.
        if tc.name == "browser_read" && self.retrieval_mode != RetrievalMode::LocalBrowser {
            let msg = format!(
                "retrieval '{}' refused — retrieval mode is '{}' for this \
                 session; browser_read requires local_browser mode \
                 (ADR-0010 §3.7.1); no silent fallback to web tools.",
                tc.name,
                self.retrieval_mode.as_str(),
            );
            let code = "retrieval_mode_requires_local_browser";
            let policy_denial = PolicyDenial {
                source: PolicyDenialSource::RetrievalMode,
                code: code.to_string(),
                reason: msg.clone(),
            };
            let mut payload = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "target": "external_retrieval",
                "status": "error",
                "error": code,
                "policy_denial": {
                    "source": "retrieval_mode",
                    "code": code,
                    "reason": msg,
                },
            });
            stamp_direct(&mut payload);
            writer.record(EventType::ToolCompleted, payload).await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    policy_denial: Some(policy_denial),
                    timed_out: false,
                    ..Default::default()
                },
                None,
            ));
        }
        // FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): candidate
        // mechanical count gate (design §1) — the prompt's soft "候选 ≤5"
        // becomes a hard per-activation cap (ORZ_WEB_FETCH_CANDIDATE_CAP,
        // default 8, user adjudication 2026-08-14), shared by web_fetch
        // (framework_fallback) and browser_read (local_browser second
        // segment, design §1.3). The gate DECIDES before any fetch/read
        // action: per-activation accumulation, exact-string URL dedup
        // (same page re-read consumes no new candidate; canonical /
        // host-level dedup is step 3), no reset on continue re-entry
        // (only activation close). Refusals are no-ToolStarted (same shape
        // as the mode gates) and feed the consecutive-denial breaker (no
        // retry space, ADR-0010 §3.5.4). Consumption is committed later —
        // after the permission/ACAF gates pass and immediately before
        // ToolStarted — so a permission/ticket-blocked call consumes no
        // budget (review fix 2026-08-14).
        let mut candidate_counts: Option<(usize, usize)> = None;
        let mut candidate_commit: Option<(String, usize)> = None;
        if crate::relay::is_candidate_counted_tool(&tc.name) {
            match self
                .candidate_gate(writer, messages, tc, fetch_candidates)
                .await?
            {
                CandidateGateDecision::Refused(result, feedback) => {
                    return Ok((result, feedback));
                }
                CandidateGateDecision::Allowed { url, cap } => {
                    candidate_commit = Some((url, cap));
                }
            }
        }
        // Permission gate. C2-1 (2026-08-11): lane self-execution skips
        // the bridge entirely (no PermissionRequested/PermissionDecision
        // events) — the explicit retrieval-mode gate above is its
        // authorization chain; the main lane keeps the per-call bridge.
        let decision = if permission_gated {
            let risk = ToolDispatcher::risk_class(&tc.name);
            writer
                .record(
                    EventType::PermissionRequested,
                    serde_json::json!({
                        "tool": tc.name,
                        "risk": format!("{risk:?}"),
                        "call_id": tc.call_id,
                    }),
                )
                .await?;
            let d = host
                .request_permission(risk, &tc.name, &tc.arguments)
                .await
                .map_err(|e| AgentLoopError::Session(e.to_string()))?;
            writer
                .record(
                    EventType::PermissionDecision,
                    serde_json::json!({
                        "tool": tc.name,
                        "decision": match d {
                            PermitDecision::AllowOnce => "allow_once",
                            PermitDecision::AllowAlways => "allow_always",
                            PermitDecision::Deny => "deny",
                            PermitDecision::Defer => "defer",
                        },
                    }),
                )
                .await?;
            d
        } else {
            PermitDecision::AllowOnce
        };

        if matches!(decision, PermitDecision::Deny | PermitDecision::Defer) {
            // Deny and Defer both refuse execution — the headless host has no
            // pending user to resolve a deferred decision (fail-closed).
            {
                let mut w = self.blackboard.write();
                w.gate_log
                    .gate_decisions
                    .push(format!("permission: deny (tool {})", tc.name));
            }
            // IP2a circuit breaker (D-3; ADR-0010 §3.5.4): NO per-call
            // counting here — the denial key is handed to the caller, which
            // aggregates at round granularity (a round with N denied calls
            // and no success counts as ONE consecutive round; success or key
            // change resets; the old 10-total ceiling is deleted). The
            // breaker user message is injected by the caller AFTER the whole
            // tool batch (provider protocol: tool messages must immediately
            // follow the assistant tool_calls declaration — 400 otherwise,
            // 2026-08-07 wordy fix).
            let reason_code = match decision {
                PermitDecision::Deny => "permission_deny".to_string(),
                PermitDecision::Defer => "permission_defer".to_string(),
                _ => unreachable!("decision narrowed to Deny|Defer above"),
            };
            let output = format!(
                "tool '{tool_name}' — 本次调用未获权限门禁放行",
                tool_name = tc.name,
            );
            let result = ToolResult {
                // P0-A 步骤 6（2026-08-13）：兜底消息中性化——只陈述本次
                // 调用事实（未获放行），不使用 可用/不可用/成功/失败/
                // available/unavailable 等判定词，也不承诺策略级不可用
                // （旧措辞 "NOT available in the current policy" 是静态
                // 声明残留，与逐次判定语义矛盾）。烧轮防护由 §3.5.4 连续
                // 拒绝熔断承担。
                output: output.clone(),
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                // P0-C S3 前置 (2026-08-15, P1-2 定案): structured signal —
                // permission denials stay event-less by the existing audit
                // contract (no ToolCompleted), so only the ToolResult
                // carries the denial for the console adapter.
                policy_denial: Some(PolicyDenial {
                    source: PolicyDenialSource::Permission,
                    code: reason_code.clone(),
                    reason: output,
                }),
                timed_out: false,
                ..Default::default()
            };
            // Replay the denial as a tool message — the provider protocol
            // requires a tool message answering each declared call, even a
            // refused one. Skipping it breaks the next round with a 400
            // (2026-08-06 polyglot probe: denied search_replace left the
            // assistant declaration unanswered → invalid_request_error).
            messages.push(Message {
                role: Role::Tool,
                content: result.output.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            // The breaker user message is NOT constructed or pushed here —
            // the denial key is returned to the caller, which aggregates the
            // round's denials and appends the breaker only after the WHOLE
            // tool batch (a user message between tool replies would violate
            // the provider protocol; 2026-08-07 wordy 400 + review P1).
            return Ok((
                result,
                Some(PolicyFeedback::Denied(DenialKey {
                    tool_name: tc.name.clone(),
                    reason_code,
                    // GAP-DENIAL-POLICY-REVISION (2026-08-12): live value — a
                    // bump is a key change, resetting the breaker
                    // (ADR-0010 §3.5.4).
                    policy_revision: self.policy_revision(),
                })),
            ));
        }

        // D-9 (FIX_PLAN 2026-08-06) + RT-001 (2026-08-11): `run_tests`
        // executes the host's FIXED command — the model supplies no argv
        // (the command itself is host-owned and hidden; the tool is only
        // declared when the work-tool probe finds a test runner), but it IS
        // controlled code execution (ADR-0010 §3.8.2: the test process can
        // write files, hit the network, spawn children), so it passes the
        // SAME permission gate as any LocalMutation tool above: Interactive
        // prompts the user, Benchmark (harness — ORZ_ALLOW_WRITE) auto-allows
        // via the host bridge (permission.rs), the retrieval lane never
        // reaches this point (its write-domain gate refuses run_tests with
        // `retrieval_role_execution_denied` first). The execution leaves a
        // full audit trail: PermissionRequested/PermissionDecision (denials
        // are no-ToolStarted, same shape as every other tool) then
        // ToolStarted/ToolCompleted (D-5; 2026-08-07 review F-02: this path
        // previously recorded zero journal events).
        if tc.name == "run_tests" {
            // P0-A review cleanup (design §4/§6): without a host runner the
            // work-tool probe keeps `run_tests` out of the model-visible list;
            // a race call is refused HERE with the neutral statement and NO
            // ToolStarted (same no-ToolStarted shape as the mode/ACAF
            // refusals) instead of executing the default NotFound error
            // after a ToolStarted.
            let Some(runner) = host.test_runner() else {
                let msg = "tool 'run_tests' — 缺少测试运行器".to_string();
                let mut completed = serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "status": "error",
                    "error": "missing_test_runner",
                });
                // F3 (2026-08-16 审查收口): run_tests 拒绝路径同样盖章
                // （direct 模式事件链关联，§7.4）。
                stamp_direct(&mut completed);
                writer.record(EventType::ToolCompleted, completed).await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                // P0-A step 5 (design §5): 调用即探针 — the refused work-tool
                // call writes back into the minimal previous-round map.
                self.maybe_note_probe_call_failure(probe_writeback, &tc.name);
                return Ok((
                    ToolResult {
                        output: msg,
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    },
                    None,
                ));
            };
            // ACAF Slice 2 (2026-08-12): command_exec_v1 for the host-owned
            // fixed command — issued/verified BEFORE ToolStarted (same
            // ordering discipline as every other action ticket). Shadow
            // mode: rejections are journaled and the run proceeds;
            // fail-closed (2026-08-13): a Blocked gate refuses the run
            // (no ToolStarted).
            let gate = self
                .acaf_command_exec_event(
                    writer,
                    &tc.name,
                    &runner,
                    activation_id.map(str::to_string),
                )
                .await?;
            if let TicketGate::Blocked { .. } = &gate {
                return self
                    .refuse_ticketed_tool(writer, messages, tc, &gate, probe_writeback)
                    .await;
            }
            let fixed_command: Option<String> = Some(runner.command.join(" "));
            // THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1): 事件面
            // tool_completed 补 wall_ms（ToolStarted → ToolCompleted 墙钟）。
            let wall_started = std::time::Instant::now();
            let mut started = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "fixed_command": fixed_command,
            });
            stamp_direct(&mut started);
            writer.record(EventType::ToolStarted, started).await?;
            // P1-1 (2026-08-08 stall guards): a legit long test run (up to
            // the 30min F-09 cap) journals nothing between ToolStarted and
            // ToolCompleted — without periodic stamps the stall watchdog
            // would fire mid-run (2026-08-08 review P1-2/D1-2: the original
            // before/after stamps only reset the idle counter at the
            // boundaries; a 30min run idles past the 6min window).
            if let Some(h) = heartbeat {
                h.stamp();
            }
            let result = {
                let fut = host.run_tests();
                tokio::pin!(fut);
                let r = loop {
                    tokio::select! {
                        r = &mut fut => break r,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                            if let Some(h) = heartbeat {
                                h.stamp();
                            }
                        }
                    }
                };
                // Tool-level failure (spawn/wait/pipe — e.g. the fixed test
                // command's interpreter missing from the environment's PATH)
                // feeds back to the model as an ORDINARY tool failure instead
                // of terminating the session; the model can pivot (bash,
                // different approach) and the run continues. Exposed by the
                // 2026-08-11 TB B 组重跑: a python-less task container called
                // run_tests → spawn failed → the old `map_err(Session)`
                // killed the whole session.
                match r {
                    Ok(r) => r,
                    Err(e) => {
                        let msg = format!("run_tests failed: {e}");
                        let mut payload = serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "status": "error",
                            "error": msg,
                            "wall_ms": wall_started.elapsed().as_millis() as u64,
                        });
                        stamp_direct(&mut payload);
                        writer.record(EventType::ToolCompleted, payload).await?;
                        messages.push(Message {
                            role: Role::Tool,
                            content: msg.clone(),
                            tool_call_id: Some(tc.call_id.clone()),
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                        });
                        // P0-A step 5 (design §5): 调用即探针 — the failed
                        // work-tool call writes back into the minimal map.
                        self.maybe_note_probe_call_failure(probe_writeback, &tc.name);
                        // None = neutral for the denial streak (only actual
                        // success resets — ADR-0010 §3.5.4).
                        return Ok((
                            ToolResult {
                                output: msg,
                                exit_code: None,
                                output_encoding: None,
                                structured: None,
                                ..Default::default()
                            },
                            None,
                        ));
                    }
                }
            };
            if let Some(h) = heartbeat {
                h.stamp();
            }
            let mut completed_payload = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": result.exit_code,
                "wall_ms": wall_started.elapsed().as_millis() as u64,
                "full_output_path": result.full_output_path,
                // RT-003 (2026-08-11): workspace changes the test
                // run caused (capped list; schema extended in
                // `tool-completed-event-payload-v0.1.schema.json`
                // — Schema first, ADR-0010 §5.3).
                "workspace_delta": result.workspace_delta,
                "workspace_delta_truncated": result.workspace_delta_truncated,
            });
            // GAP-ENCODING-GATE (OPS-PROTOCOL §8): record the decode stage
            // that produced the test output when the host observed one.
            if let Some(enc) = &result.output_encoding {
                completed_payload["output_encoding"] = serde_json::json!(enc);
            }
            // THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1 审查处理
            // P2-1): run_tests 的 F-09 墙钟掐杀经 TestRunResult.timed_out
            // 结构化透传——与通用工具路径一致，事件面可按超时事实画像
            // （schema 描述「host's per-call wall-clock kill」）。
            if result.timed_out {
                completed_payload["timed_out"] = serde_json::json!(true);
            }
            stamp_direct(&mut completed_payload);
            writer
                .record(EventType::ToolCompleted, completed_payload)
                .await?;
            // 2026-08-08 blackboard partition: fold the executed call into
            // the tool-action section (terminal — a fixed command run).
            self.blackboard.write().tool_actions.push(ToolActionRecord {
                category: ToolDispatcher::action_category(&tc.name).to_string(),
                tool: tc.name.clone(),
                timestamp: chrono_utc_now(),
            });
            let tool_result = ToolResult {
                // F-09 (2026-08-07 review): mechanical context gate — only
                // the completion reminder + the final output (tail-capped
                // and secret/path-scrubbed, RT-002 2026-08-11) enter the
                // conversation; the full (capped) output is on disk and the
                // model reads it via read_file when it wants more than the
                // tail.
                output: compose_test_output_message(&result),
                exit_code: result.exit_code,
                // 2026-08-28 全面审查处理：run_tests 特例路径同样透传解码
                // 阶段（此前只进 journal、结果面丢失；与通用路径对齐）。
                output_encoding: result.output_encoding.clone(),
                structured: None,
                // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.3): the run_tests
                // receipt carries the host-measured workspace delta (the
                // event payload already carried it; now the console receipt
                // can attach it too).
                workspace_delta: result.workspace_delta.clone(),
                workspace_delta_truncated: result.workspace_delta_truncated,
                ..Default::default()
            };
            messages.push(Message {
                role: Role::Tool,
                content: tool_result.output.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            // run_tests executed the host's fixed command — a success for
            // the denial streak (ADR-0010 §3.5.4: only actual success resets).
            return Ok((tool_result, Some(PolicyFeedback::Succeeded)));
        }

        // ACAF Slice 2 first phase (2026-08-12): action tickets for
        // external-effect tools. The ticket is issued and verified AFTER the
        // permission gate allowed the call (denied calls need no ticket —
        // two-layer gate, ADR-0011 §2.4/§4.5: permission decides policy, the
        // ticket decides this-call authorization) and BEFORE the ToolStarted
        // evidence. Shadow mode: failures journal `control_ticket_rejected`
        // and the tool proceeds.
        if crate::acaf::action_kind_for_tool(&tc.name).is_some() {
            let gate = self
                .acaf_action_event(
                    writer,
                    &tc.name,
                    &tc.arguments,
                    activation_id.map(str::to_string),
                )
                .await?;
            if let TicketGate::Blocked { .. } = &gate {
                return self
                    .refuse_ticketed_tool(writer, messages, tc, &gate, probe_writeback)
                    .await;
            }
        }

        // IP5: pre-mutation snapshot — record the pre-tool worktree state of
        // the mutation tool's targets (ToolDispatcher wrapper, v0.2 §4 IP5).
        // Evidence layer, not a gate: a snapshot failure is journaled
        // (`snapshot_error`) and does not block the tool. Tools without
        // statically knowable targets (e.g. bash) produce no snapshot.
        if let Some(store) = &self.snapshot_store
            && ToolDispatcher::modifies_files(&tc.name)
        {
            let targets =
                ToolDispatcher::snapshot_targets(store.worktree(), &tc.name, &tc.arguments);
            if !targets.is_empty() {
                let target_strs: Vec<String> = targets
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                match store.track(&targets).await {
                    Ok(record) => {
                        writer
                            .record(
                                EventType::SnapshotCreated,
                                serde_json::json!({
                                    "tool": tc.name,
                                    "targets": target_strs,
                                    "snapshot_hash": record.snapshot_hash,
                                }),
                            )
                            .await?;
                    }
                    Err(e) => {
                        writer
                            .record(
                                EventType::SnapshotCreated,
                                serde_json::json!({
                                    "tool": tc.name,
                                    "targets": target_strs,
                                    "snapshot_error": e.to_string(),
                                }),
                            )
                            .await?;
                    }
                }
            }
        }

        // FUS-RETRIEVAL-MECH P0-B step 2/4 review fix (2026-08-14): commit
        // the candidate consumption NOW — after the permission and ACAF
        // ticket gates passed, immediately before ToolStarted. A call
        // blocked by a later gate (permission deny / ticket reject) never
        // reaches this point, so it consumes no budget and its refusal
        // carries no candidate counts.
        if let Some((url, cap)) = candidate_commit.take() {
            if let Some(counter) = fetch_candidates {
                candidate_counts = Some(commit_candidate(counter, &url, cap));
            }
        }

        // Execute.
        // THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1): 事件面
        // tool_completed 补 wall_ms（ToolStarted → ToolCompleted 墙钟）
        // 与 timed_out 标记——S4 失败画像据此直接读 per-call 时长/超时
        // 事实（web_search 单次最高 1365s 的时间黑洞可审计）。
        let wall_started = std::time::Instant::now();
        let mut started = serde_json::json!({
            "tool": tc.name,
            "call_id": tc.call_id,
        });
        stamp_direct(&mut started);
        writer.record(EventType::ToolStarted, started).await?;
        // A6 §8 C.2 (2026-08-08): `compaction_whitelist_add` — served from
        // the controller's own whitelist (in-memory + .gsa archive), no
        // host dispatch. The permission gate already ran (ReadOnly class
        // auto-allows under every policy); the event chain is complete.
        // Window (user decision): only the FIRST tool batch may write —
        // `tool_rounds == 0` while this batch is executing. Cap (user
        // decision): cumulative 16K chars, configurable.
        if tc.name == "compaction_whitelist_add" {
            let content = tc
                .arguments
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let window_ok = tool_rounds == 0;
            let cap_ok = {
                let w = self.whitelist.lock().unwrap();
                let used: usize = w.iter().map(|e| e.chars().count()).sum();
                used + content.chars().count() <= self.whitelist_cap
            };
            let refused = if !window_ok {
                Some(
                    "compaction whitelist is only writable during the first tool batch (first round)",
                )
            } else if content.trim().is_empty() {
                Some("compaction whitelist entry must not be empty")
            } else if !cap_ok {
                Some("compaction whitelist cumulative size cap exceeded")
            } else {
                None
            };
            if let Some(reason) = refused {
                let mut completed = serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "status": "error",
                    "error": reason,
                });
                // F3 (2026-08-16 审查收口): controller 内建工具在 direct 模式
                // 的 ToolCompleted 同样盖章（与 ToolStarted 对称，§7.4）。
                stamp_direct(&mut completed);
                writer.record(EventType::ToolCompleted, completed).await?;
                let output = format!("whitelist write refused: {reason}");
                messages.push(Message {
                    role: Role::Tool,
                    content: output.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                return Ok((
                    ToolResult {
                        output,
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    },
                    None,
                ));
            }
            let entry_index = {
                let mut w = self.whitelist.lock().unwrap();
                w.push(content.clone());
                w.len()
            };
            let total_chars: usize = self
                .whitelist
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.chars().count())
                .sum();
            // Archive first (mechanical best-effort), then make the entry
            // resident in the preamble zone — the compaction mechanism
            // skips the preamble, so the whitelist survives compaction.
            self.archive_whitelist_entry(host, &content);
            self.upsert_whitelist_message(messages);
            let mut completed = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 0,
            });
            // F3 (2026-08-16 审查收口): direct 盖章对称。
            stamp_direct(&mut completed);
            writer.record(EventType::ToolCompleted, completed).await?;
            self.blackboard.write().tool_actions.push(ToolActionRecord {
                category: ToolDispatcher::action_category(&tc.name).to_string(),
                tool: tc.name.clone(),
                timestamp: chrono_utc_now(),
            });
            let output = format!(
                "whitelist entry #{entry_index} written (cumulative {total_chars} chars) — \
                 it will NOT be compressed away and is archived to .gsa"
            );
            messages.push(Message {
                role: Role::Tool,
                content: output.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output,
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                },
                None,
            ));
        }
        // 2026-08-08 blackboard partition (A3): `blackboard_read` is served
        // from the controller's own blackboard — no host dispatch. The
        // permission gate already ran (ReadOnly class auto-allows under
        // every policy); the event chain is complete (PermissionRequested/
        // PermissionDecision/ToolStarted above, ToolCompleted below).
        if tc.name == "blackboard_read" {
            // 2026-08-19 方案B 全面审查处理（N3）：section 非字符串 = 显式报错
            // （同非法 epoch/receipt_id 纪律——绝不静默回退到 "plan" 默认值，
            // 否则与 receipt_id 组合时守卫报错会显示误导性的 section=plan）。
            let section = match tc.arguments.get("section") {
                Some(raw) => match raw.as_str() {
                    Some(s) => s.to_string(),
                    None => {
                        let content = format!(
                            "invalid blackboard_read section: {raw} — section 必须 \
                             是字符串（plan|edits|tool_actions|exec|actions|session|\
                             internal_ret|external_ret|entities）"
                        );
                        let mut completed = serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "section": "<invalid>",
                            "error": content,
                        });
                        // F3 (2026-08-16 审查收口): direct 盖章对称。
                        stamp_direct(&mut completed);
                        writer.record(EventType::ToolCompleted, completed).await?;
                        self.blackboard.write().tool_actions.push(ToolActionRecord {
                            category: ToolDispatcher::action_category(&tc.name).to_string(),
                            tool: tc.name.clone(),
                            timestamp: chrono_utc_now(),
                        });
                        let result = ToolResult {
                            output: content,
                            exit_code: Some(1),
                            output_encoding: None,
                            structured: None,
                            ..Default::default()
                        };
                        messages.push(Message {
                            role: Role::Tool,
                            content: result.output.clone(),
                            tool_call_id: Some(tc.call_id.clone()),
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                        });
                        return Ok((result, None));
                    }
                },
                None => "plan".to_string(),
            };
            let since = tc.arguments.get("since_timestamp").and_then(|s| s.as_str());
            // F6 (2026-08-15, BACKLOG 6e 复查遗留): distinguish "epoch
            // omitted" (live view) from "epoch present but invalid" (0,
            // negative, float, string, …) — an invalid value is an explicit
            // error, never a silent fallback to the live board.
            let epoch = match tc.arguments.get("epoch") {
                Some(raw) => match raw.as_u64() {
                    Some(n) if n >= 1 => Some(n),
                    _ => {
                        let content = format!(
                            "invalid blackboard_read epoch: {raw} — epoch must be a \
                             positive integer (≥1); omit the parameter to read the \
                             live view"
                        );
                        let mut completed = serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "section": section,
                            "error": content,
                        });
                        // F3 (2026-08-16 审查收口): direct 盖章对称。
                        stamp_direct(&mut completed);
                        writer.record(EventType::ToolCompleted, completed).await?;
                        self.blackboard.write().tool_actions.push(ToolActionRecord {
                            category: ToolDispatcher::action_category(&tc.name).to_string(),
                            tool: tc.name.clone(),
                            timestamp: chrono_utc_now(),
                        });
                        let result = ToolResult {
                            output: content,
                            exit_code: Some(1),
                            output_encoding: None,
                            structured: None,
                            ..Default::default()
                        };
                        messages.push(Message {
                            role: Role::Tool,
                            content: result.output.clone(),
                            tool_call_id: Some(tc.call_id.clone()),
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                        });
                        return Ok((result, None));
                    }
                },
                None => None,
            };
            // 方案 B（2026-08-19，ADR-0010 §14.31 / 设计 §4.5）：可选
            // `receipt_id` 点读——值 = 结果栏 receipt 的 order_id（如
            // ORD-000012），仅与 section=actions 组合有效（非 actions 由
            // render_section 显式报错）；格式非法（非字符串/空串）= 显式
            // 报错，绝不静默回退整段。
            let receipt_id = match tc.arguments.get("receipt_id") {
                Some(raw) => match raw.as_str() {
                    Some(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
                    _ => {
                        let content = format!(
                            "invalid blackboard_read receipt_id: {raw} — receipt_id \
                             必须是非空字符串（结果栏 receipt 的 order_id，如 \
                             ORD-000012）；省略该参数读取整个 actions 分区"
                        );
                        let mut completed = serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "section": section,
                            "error": content,
                        });
                        // F3 (2026-08-16 审查收口): direct 盖章对称。
                        stamp_direct(&mut completed);
                        writer.record(EventType::ToolCompleted, completed).await?;
                        self.blackboard.write().tool_actions.push(ToolActionRecord {
                            category: ToolDispatcher::action_category(&tc.name).to_string(),
                            tool: tc.name.clone(),
                            timestamp: chrono_utc_now(),
                        });
                        let result = ToolResult {
                            output: content,
                            exit_code: Some(1),
                            output_encoding: None,
                            structured: None,
                            ..Default::default()
                        };
                        messages.push(Message {
                            role: Role::Tool,
                            content: result.output.clone(),
                            tool_call_id: Some(tc.call_id.clone()),
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                        });
                        return Ok((result, None));
                    }
                },
                None => None,
            };
            // PUSH→PULL (2026-08-21, CONTEXT_SCAFFOLDING_PULL_REDESIGN §4
            // 方案 A): `section=session` 是 live 会话面（预算剩余 + 状态行），
            // 由 controller 直接渲染、不进 epoch 归档；其余分区走黑板渲染。
            // 2026-08-21 全面审查处理（O4）：session 组合错误（epoch /
            // receipt_id）走参数级显式报错——exit_code 1 + error 字段，
            // 绝不静默回退（同非法 epoch/receipt_id 纪律）。
            let content = if section == "session" {
                match self.render_session_section(epoch, receipt_id.as_deref(), tool_rounds) {
                    Ok(text) => text,
                    Err(error) => {
                        let mut completed = serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "section": section,
                            "error": error,
                        });
                        // F3 (2026-08-16 审查收口): direct 盖章对称。
                        stamp_direct(&mut completed);
                        writer.record(EventType::ToolCompleted, completed).await?;
                        self.blackboard.write().tool_actions.push(ToolActionRecord {
                            category: ToolDispatcher::action_category(&tc.name).to_string(),
                            tool: tc.name.clone(),
                            timestamp: chrono_utc_now(),
                        });
                        let result = ToolResult {
                            output: error,
                            exit_code: Some(1),
                            output_encoding: None,
                            structured: None,
                            ..Default::default()
                        };
                        messages.push(Message {
                            role: Role::Tool,
                            content: result.output.clone(),
                            tool_call_id: Some(tc.call_id.clone()),
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                        });
                        return Ok((result, None));
                    }
                }
            } else {
                self.render_blackboard_section(&section, since, epoch, receipt_id.as_deref())
            };
            let mut completed = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 0,
                "section": section,
            });
            if let Some(epoch) = epoch {
                completed["epoch"] = serde_json::json!(epoch);
            }
            // F3 (2026-08-16 审查收口): direct 盖章对称（ToolStarted 已在
            // 上方盖章，ToolCompleted 必须一致，§7.4）。
            stamp_direct(&mut completed);
            writer.record(EventType::ToolCompleted, completed).await?;
            self.blackboard.write().tool_actions.push(ToolActionRecord {
                category: ToolDispatcher::action_category(&tc.name).to_string(),
                tool: tc.name.clone(),
                timestamp: chrono_utc_now(),
            });
            let result = ToolResult {
                output: content,
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            };
            messages.push(Message {
                role: Role::Tool,
                content: result.output.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((result, None));
        }
        // P0-C orz 内嵌集成 S2 (2026-08-15): `blackboard_action_write` —
        // 模型写订单（写无副作用；副作用只在轮末单一发放出口）。单轮一单：
        // 已有 pending 订单机械拒绝（`order_slot_busy`）。round/plan_epoch
        // 由机械层盖章（模型不提供——防重放信任锚）；动作名/参数合法性由
        // 发放链的注册表/契约校验负责。main lane only（检索车道由投影 +
        // ToolFilter write gate + 此处 activation 守卫三重拒绝）。
        if tc.name == "blackboard_action_write" {
            if activation_id.is_some() {
                let msg = "console action write refused — the action board is main-lane only";
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "status": "error",
                            "error": "console_action_write_lane_denied",
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.to_string(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                return Ok((
                    ToolResult {
                        output: msg.to_string(),
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    },
                    None,
                ));
            }
            let action = tc
                .arguments
                .get("action")
                .and_then(|a| a.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): 可选
            // step_id —— 绑定计划步骤（ActionOrder.step_id）。步骤门在
            // 发放时机械校验（§6）：console 默认态下订单必须绑定当前步骤，
            // 否则 step_not_done 拒绝。
            let step_id = tc
                .arguments
                .get("step_id")
                .and_then(|s| s.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let arguments = tc
                .arguments
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let Some(action) = action else {
                let content =
                    "invalid blackboard_action_write call: `action` must be a non-empty string"
                        .to_string();
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "status": "error",
                            "error": content,
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: content.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                return Ok((
                    ToolResult {
                        output: content,
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    },
                    None,
                ));
            };
            let seq = self
                .console_order_seq
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            let order = ActionOrder {
                order_id: format!("ORD-{seq:06}"),
                action,
                arguments,
                target: None,
                step_id,
                round: tool_rounds,
                plan_epoch: self.blackboard.read().plan.plan_epoch,
                run_id: writer.run_id().to_string(),
            };
            let write_result = {
                let mut w = self.blackboard.write();
                w.actions.write_order(order.clone())
            };
            match write_result {
                Ok(()) => {
                    // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱):
                    // action_write ToolCompleted 收敛到通用契约形状
                    // （成功只带 exit_code）；订单身份/step 绑定/机械盖章
                    // 由 `console_order_written` 事件承载（阶段 A 审计 §7.4
                    // 的 S2 payload-shape 债务随本次收口）。
                    writer
                        .record(
                            EventType::ToolCompleted,
                            serde_json::json!({
                                "tool": tc.name,
                                "call_id": tc.call_id,
                                "exit_code": 0,
                            }),
                        )
                        .await?;
                    writer
                        .record(
                            EventType::ConsoleOrderWritten,
                            serde_json::json!({
                                // F1 (2026-08-16 审查收口): verifier 按
                                // write_call_id 与 action_write ToolCompleted
                                // 对拍——order_id 是内部 ORD-xxxxx，与模型
                                // 工具调用 id 不同。
                                "order_id": order.order_id,
                                "write_call_id": tc.call_id.clone(),
                                "action": order.action,
                                "step_id": order.step_id,
                                "round": order.round,
                                "plan_epoch": order.plan_epoch,
                                "run_id": order.run_id,
                            }),
                        )
                        .await?;
                    self.blackboard.write().tool_actions.push(ToolActionRecord {
                        category: ToolDispatcher::action_category(&tc.name).to_string(),
                        tool: tc.name.clone(),
                        timestamp: chrono_utc_now(),
                    });
                    let output = format!(
                        "order {} written (action={}, round={}, plan_epoch={}) — 本轮轮末机械发放",
                        order.order_id, order.action, order.round, order.plan_epoch,
                    );
                    messages.push(Message {
                        role: Role::Tool,
                        content: output.clone(),
                        tool_call_id: Some(tc.call_id.clone()),
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                    return Ok((
                        ToolResult {
                            output,
                            exit_code: Some(0),
                            output_encoding: None,
                            structured: None,
                            ..Default::default()
                        },
                        None,
                    ));
                }
                Err(crate::blackboard::ActionBoardError::OrderSlotBusy) => {
                    let content =
                        "action bar already holds a pending order — 本轮订单未发放完不进入下一轮写单（单轮一单）。\
                         请先查看结果栏/等待轮末发放"
                            .to_string();
                    writer
                        .record(
                            EventType::ToolCompleted,
                            serde_json::json!({
                                "tool": tc.name,
                                "call_id": tc.call_id,
                                "exit_code": 1,
                                "status": "error",
                                "error": "order_slot_busy",
                            }),
                        )
                        .await?;
                    messages.push(Message {
                        role: Role::Tool,
                        content: content.clone(),
                        tool_call_id: Some(tc.call_id.clone()),
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                    return Ok((
                        ToolResult {
                            output: content,
                            exit_code: Some(1),
                            output_encoding: None,
                            structured: None,
                            ..Default::default()
                        },
                        None,
                    ));
                }
            }
        }
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD
        // _DESIGN §3-§5): `plan_write` — 首轮计划轮唯一写面（计划修订时也可
        // 使用）。机械校验 + 一次重填 + 降级留痕；通过后结构化计划落黑板
        // plan epoch。main lane only（检索车道由投影 + ToolFilter 写门 +
        // 此处 activation 守卫三重拒绝）。ReadOnly 类 → 权限门自动放行；
        // ToolStarted 已在上方记录，ToolCompleted 在此收口。
        if tc.name == crate::planning::PLAN_WRITE_TOOL {
            // 2026-08-16 审查收口（P3-4）：声明面随开关收敛后，调用面同样
            // fail-closed —— 关闭态/grill 的 plan_write 一律拒绝，不落板。
            if !self.plan_first_enabled {
                let msg = "plan_write refused — the plan gate is disabled in this session";
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "status": "error",
                            "error": "plan_write_disabled",
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.to_string(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                return Ok((
                    ToolResult {
                        output: msg.to_string(),
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    },
                    None,
                ));
            }
            if activation_id.is_some() {
                let msg = "plan_write refused — the plan gate is main-lane only";
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 1,
                            "status": "error",
                            "error": "plan_write_lane_denied",
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.to_string(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                return Ok((
                    ToolResult {
                        output: msg.to_string(),
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    },
                    None,
                ));
            }
            let verdict = crate::planning::parse_and_validate_plan(&tc.arguments);
            let attempt = plan_gate_attempt.unwrap_or(1);
            let outcome = if verdict.errors.is_empty() {
                crate::planning::PlanWriteOutcome::Accepted
            } else if plan_gate_attempt.is_some() {
                crate::planning::decide_outcome(attempt, &verdict.errors)
            } else {
                // 计划修订（首轮门之外）：失败即拒绝，不强制重填轮。
                crate::planning::PlanWriteOutcome::Degraded {
                    reason: "validation_failed",
                }
            };
            let (outcome_str, degrade_reason) = match outcome {
                crate::planning::PlanWriteOutcome::Accepted => ("accepted", None),
                crate::planning::PlanWriteOutcome::RefillRequested => ("refill_requested", None),
                crate::planning::PlanWriteOutcome::Degraded { reason } => {
                    ("degraded", Some(reason))
                }
            };
            let mut plan_id = verdict.plan_id.clone().unwrap_or_default();
            let mut goal = verdict.goal.clone().unwrap_or_default();
            let step_count = verdict.steps.len();
            let mut plan_epoch = self.blackboard.read().plan.plan_epoch;
            let mut final_outcome = outcome_str;
            let mut final_degrade = degrade_reason;
            if matches!(outcome, crate::planning::PlanWriteOutcome::Accepted) {
                let current_id = self.blackboard.read().plan.plan_id.clone();
                // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the delivery
                // baseline is captured when a NEW plan epoch is approved —
                // the `submit` status diffs the live worktree against it.
                // Same-plan revisions keep the original baseline (the work
                // of the current delivery did not restart).
                let is_new_epoch = current_id.as_deref() != Some(plan_id.as_str());
                let epoch = if current_id.as_deref() == Some(plan_id.as_str()) {
                    plan_epoch
                } else {
                    match &self.blackboard_archive_dir {
                        Some(dir) => crate::epoch::next_plan_epoch_from_archive(dir),
                        None => plan_epoch.saturating_add(1).max(1),
                    }
                };
                match self.apply_structured_plan(
                    plan_id.clone(),
                    epoch,
                    goal.clone(),
                    verdict.steps.clone(),
                ) {
                    Ok(()) => {
                        plan_epoch = epoch;
                        if is_new_epoch {
                            *self.delivery_baseline.lock().unwrap() = host.workspace_snapshot();
                            *self.delivery_pending.lock().unwrap() = (epoch, false);
                        }
                    }
                    Err(_) => {
                        // 落板失败（epoch 身份/归档异常）——机械降级，不挂死。
                        final_outcome = "degraded";
                        final_degrade = Some("plan_rotate_failed");
                        plan_id = String::new();
                        goal = String::new();
                    }
                }
            }
            let output = match (final_outcome, final_degrade) {
                ("accepted", _) => format!(
                    "plan accepted: {plan_id} ({step_count} steps) — landed in \
                     blackboard plan_epoch {plan_epoch}"
                ),
                ("refill_requested", _) => format!(
                    "[PLAN_REFILL v0.1] 计划校验未通过：{}；\
                     请只重填 plan_write（唯一一次重填机会，之后机械降级）。",
                    verdict.errors.join("；")
                ),
                _ => format!(
                    "plan rejected: {} — 本次运行无已批准计划，继续执行。",
                    final_degrade.unwrap_or("validation_failed")
                ),
            };
            writer
                .record(
                    EventType::PlanWrite,
                    crate::planning::plan_write_payload(
                        &plan_id,
                        &goal,
                        step_count,
                        final_outcome,
                        attempt,
                        &verdict,
                        final_degrade,
                    ),
                )
                .await?;
            let exit_code = if final_outcome == "accepted" { 0 } else { 1 };
            // 2026-08-16 审查收口（P1）：ToolCompleted 收敛到通用契约形状——
            // 成功只带 exit_code（status/extras 移除，避免 additionalProperties
            // 与 status=const("error") 冲突）；失败带 status=error + error。
            // 计划的 outcome/attempt/plan_epoch 由 PlanWrite 事件承载。
            let mut completed = serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": exit_code,
            });
            if exit_code != 0 {
                completed["status"] = serde_json::json!("error");
                completed["error"] = serde_json::json!(final_degrade.unwrap_or(final_outcome));
            }
            writer.record(EventType::ToolCompleted, completed).await?;
            messages.push(Message {
                role: Role::Tool,
                content: output.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output,
                    exit_code: Some(exit_code),
                    output_encoding: None,
                    structured: Some(serde_json::json!({
                        "outcome": final_outcome,
                        "attempt": attempt,
                        "plan_id": plan_id,
                        "plan_epoch": plan_epoch,
                    })),
                    ..Default::default()
                },
                None,
            ));
        }
        // P1-1 (2026-08-08 stall guards): mirror the run_tests stamp — a
        // tool that journals nothing between ToolStarted/ToolCompleted must
        // not trip the stall watchdog (the tool itself is bounded by the
        // P0-1 per-call timeout).
        // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): `submit` —— 末步
        // 「递交/完成」的显式递交/状态展示路径（无参、两阶段）。
        // THIN-HARNESS-REDESIGN-V2 §9.3 (2026-08-29)：无 plan 会话同样
        // 放行——降级为纯状态展示（不再 `no plan in force` 拒绝），只渲染
        // 交付状态、不推进任何计划步骤。有 plan 会话：第一次调用机械计算
        // 交付状态渲染进黑板 plan 末步状态行（pending 确认）；第二次调用
        // 确认并置末步 done（进入最终回答流程）。普通订单绑定末步不产生
        // done；console_step_done 对末步同样拒绝（见下），杜绝绕过递交门。
        if tc.name == "submit" {
            if !self.console_default_enabled {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "submit_disabled",
                        "submit refused — the console dual-mode is disabled",
                    )
                    .await?,
                    None,
                ));
            }
            if activation_id.is_some() {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "submit_lane_denied",
                        "submit refused — main-lane only",
                    )
                    .await?,
                    None,
                ));
            }
            let (terminal_idx, terminal_id) = {
                let w = self.blackboard.read();
                if w.plan.steps.is_empty() {
                    (None, None)
                } else {
                    let idx = w.plan.steps.len() - 1;
                    (Some(idx), Some(w.plan.steps[idx].id.clone()))
                }
            };
            // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / 设计
            // §2.1/§3)：submit 为信息展示、非硬门——订单层退役后无机械
            // 步骤推进机制（step 绑定/顺序转事件留痕），终答前的反例自查
            // 轮 + 审计报告承接「计划完成声明」核对；不再要求前序步骤
            // done（step 状态退化为方向与状态展示）；无 plan 会话更只是
            // 纯状态展示（§9.3：放行、不拒绝）。
            let epoch = self.blackboard.read().plan.plan_epoch;
            let pending = {
                let p = self.delivery_pending.lock().unwrap();
                p.0 == epoch && p.1
            };
            let status = self.compute_delivery_status(host);
            let (msg, structured, phase) = if !pending {
                {
                    let mut w = self.blackboard.write();
                    w.plan.delivery_status = Some(status.clone());
                }
                *self.delivery_pending.lock().unwrap() = (epoch, true);
                (
                    format!(
                        "submit: 交付状态已渲染进黑板 plan 视图；核查后同动作再触发一次确认递交。\n{status}"
                    ),
                    serde_json::json!({ "phase": "requested", "status": status }),
                    "requested",
                )
            } else {
                {
                    let mut w = self.blackboard.write();
                    w.plan.delivery_status = Some(status.clone());
                    if let Some(terminal_idx) = terminal_idx {
                        crate::planning::mark_step_done(
                            &mut w.plan.steps,
                            terminal_idx,
                            &tc.call_id,
                            None,
                        );
                    }
                }
                *self.delivery_pending.lock().unwrap() = (epoch, false);
                (
                    Self::submit_confirm_message(terminal_id.as_deref(), &status),
                    serde_json::json!({
                        "phase": "confirmed",
                        "status": status,
                        "step_id": terminal_id,
                    }),
                    "confirmed",
                )
            };
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "exit_code": 0,
                        "delivery_phase": phase,
                    }),
                )
                .await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: Some(structured),
                    ..Default::default()
                },
                None,
            ));
        }
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.5):
        // `console_step_done` —— direct 模式的有记录例外收口：模型提交
        // {step_id, transition_id, trace_id}，机械层校验（步骤为当前
        // in_progress、transition_id 属于本 run 的 direct 切换、trace_id
        // 对应已发生的 direct ToolCompleted）后置步骤 done(direct 证据)；
        // 证据不匹配拒绝（不自我认证）。
        if tc.name == "console_step_done" {
            if !self.console_default_enabled {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_disabled",
                        "console_step_done refused — the console dual-mode is disabled",
                    )
                    .await?,
                    None,
                ));
            }
            if activation_id.is_some() {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_lane_denied",
                        "console_step_done refused — main-lane only",
                    )
                    .await?,
                    None,
                ));
            }
            let step_id = tc
                .arguments
                .get("step_id")
                .and_then(|s| s.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let transition_id = tc
                .arguments
                .get("transition_id")
                .and_then(|s| s.as_str())
                .map(str::to_string);
            let trace_id = tc
                .arguments
                .get("trace_id")
                .and_then(|s| s.as_str())
                .map(str::to_string);
            let (Some(step_id), Some(transition_id), Some(trace_id)) =
                (step_id, transition_id, trace_id)
            else {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_missing_arguments",
                        "invalid console_step_done call: step_id / transition_id / \
                         trace_id must be non-empty strings",
                    )
                    .await?,
                    None,
                ));
            };
            // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the final step is
            // the fixed 递交/完成 step — it advances ONLY via the `submit`
            // delivery path, never via the direct-mode evidence exception
            // (否则模型可绕过机械交付状态直接"完成"末步). ID-keyed —
            // legacy/restored plans with a plain final step id are not the
            // fixed 递交/完成 step and keep the direct evidence path.
            let step_is_terminal = {
                let w = self.blackboard.read();
                w.plan
                    .steps
                    .iter()
                    .position(|s| s.id == step_id)
                    .map(|idx| crate::planning::is_terminal_step(&w.plan.steps, idx))
                    .unwrap_or(false)
            };
            if step_is_terminal {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_terminal_step",
                        &format!(
                            "console_step_done refused — step {step_id} is the fixed \
                             递交/完成 step; advance it only via the submit delivery \
                             action (call `submit` to render the mechanical delivery \
                             status, then call it again to confirm)"
                        ),
                    )
                    .await?,
                    None,
                ));
            }
            let (mode_ok, current_transition, trace_ok) = {
                let state = self.console_mode_state.lock().unwrap();
                (
                    state.is_direct(),
                    state.transition_id.clone(),
                    state.has_direct_trace(&trace_id),
                )
            };
            let step_in_progress = {
                let w = self.blackboard.read();
                w.plan
                    .steps
                    .iter()
                    .position(|s| s.id == step_id)
                    .map(|idx| {
                        matches!(
                            w.plan.steps[idx].status,
                            crate::blackboard::StepStatus::InProgress
                        )
                    })
                    .unwrap_or(false)
            };
            if !mode_ok {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_not_direct",
                        "console_step_done refused — the run is not in direct mode",
                    )
                    .await?,
                    None,
                ));
            }
            if current_transition.as_deref() != Some(transition_id.as_str()) {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_bad_transition",
                        &format!(
                            "console_step_done refused — transition_id {transition_id} \
                             does not match the current direct transition {current:?}",
                            current = current_transition,
                        ),
                    )
                    .await?,
                    None,
                ));
            }
            if !step_in_progress {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_not_in_progress",
                        &format!(
                            "console_step_done refused — step {step_id} is not the current \
                             in-progress step (evidence cannot self-certify)"
                        ),
                    )
                    .await?,
                    None,
                ));
            }
            if !trace_ok {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_step_done_bad_trace",
                        &format!(
                            "console_step_done refused — trace_id {trace_id} does not \
                             correspond to an already-occurred direct-mode ToolCompleted"
                        ),
                    )
                    .await?,
                    None,
                ));
            }
            // 证据通过：步骤 → done(direct, transition_id, trace_id)。
            {
                let mut w = self.blackboard.write();
                if let Some(idx) = w.plan.steps.iter().position(|s| s.id == step_id) {
                    crate::planning::mark_step_done(
                        &mut w.plan.steps,
                        idx,
                        &transition_id,
                        Some(crate::blackboard::DirectStepEvidence {
                            transition_id: transition_id.clone(),
                            trace_id,
                        }),
                    );
                }
            }
            let msg = format!("step {step_id} marked done (direct evidence)");
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "exit_code": 0,
                    }),
                )
                .await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                },
                None,
            ));
        }
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.1):
        // `console_return_to_console` —— direct → console 单向返回
        // （写 transition 事件 + gate_log，模式复位，本 run 不再询问）。
        if tc.name == "console_return_to_console" {
            if !self.console_default_enabled || activation_id.is_some() {
                return Ok((
                    self.refuse_console_tool(
                        writer,
                        messages,
                        &tc.name,
                        &tc.call_id,
                        "console_return_lane_denied",
                        "console_return_to_console refused — main lane / dual-mode only",
                    )
                    .await?,
                    None,
                ));
            }
            let is_direct = self.console_mode_state.lock().unwrap().is_direct();
            if !is_direct {
                let msg = "console_return_to_console ignored — the run is already in console mode"
                    .to_string();
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "exit_code": 0,
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                return Ok((
                    ToolResult {
                        output: msg,
                        exit_code: Some(0),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    },
                    None,
                ));
            }
            self.return_console_to_console(writer, tool_rounds).await?;
            let msg =
                "returned to console mode — plan gate re-engages for console orders".to_string();
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "exit_code": 0,
                    }),
                )
                .await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok((
                ToolResult {
                    output: msg,
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                },
                None,
            ));
        }
        if let Some(h) = heartbeat {
            h.stamp();
        }
        // FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14, review fix): the
        // mechanical count feedback is computed ONCE so the conversation
        // message and the blackboard exec mirror stay consistent (a failed
        // fetch/read still consumed its candidate).
        let count_note = candidate_counts.map(|(count, cap)| {
            format!("\n候选 {count}/{cap}，剩余 {}", cap.saturating_sub(count))
        });
        // The bool tracks execution success vs timeout/tool error: only a
        // successful call resets the denial streak (ADR-0010 §3.5.4);
        // timeout/error are neutral (分开记账 — neither reset nor count).
        // THIN-HARNESS-REDESIGN §4.6 审查处理 (2026-08-27, 用户裁定)：通用
        // 工具执行路径周期心跳打点（run_tests 的 60s 先例）——ORZ_STALL_
        // TIMEOUT 回落后（默认 360s），合法长命令（编译/训练，工具配置层
        // 上限已放开到 900s）在工具执行期间每 60s 打点保活；stall 看门狗
        // 只收模型侧静默挂死（权限等待/重试背压/轮间代码），不再误杀长
        // 工具。挂死工具仍由工具超时树杀（模型继续），stall 不与工具超时
        // 等窗竞态（2026-08-08 review P2-1/D2-1 纪律）。
        let call =
            host.call_tool_with_timeout(&tc.name, tc.arguments.clone(), &tc.call_id, timeout);
        tokio::pin!(call);
        let call_result = loop {
            tokio::select! {
                r = &mut call => break r,
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                    if let Some(h) = heartbeat {
                        h.stamp();
                    }
                }
            }
        };
        let (mut result, succeeded) = match call_result {
            Ok(res) => {
                // 2026-08-08 blackboard partition: a SUCCESSFUL file-edit
                // tool records its line-range delta — old/new line counts
                // from the call's old_string/new_string args ("行范围从
                // old_str/new_str 换行计数计算"; empty old_string = new-file
                // creation, 0 lines). The record lands in the edit-action
                // section AND the journal payload (tool_completed.edits);
                // the event-level timestamp is the time. Non-edit tools
                // record nothing.
                let mut edits_payload: Vec<serde_json::Value> = Vec::new();
                if res.exit_code == Some(0) && ToolDispatcher::is_file_edit(&tc.name) {
                    let old_lines = tc
                        .arguments
                        .get("old_string")
                        .and_then(|v| v.as_str())
                        .map(|s| s.lines().count())
                        .unwrap_or(0);
                    let new_lines = tc
                        .arguments
                        .get("new_string")
                        .and_then(|v| v.as_str())
                        .map(|s| s.lines().count())
                        .unwrap_or(0);
                    let file = tc
                        .arguments
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !file.is_empty() {
                        let timestamp = chrono_utc_now();
                        self.blackboard.write().edits.push(EditRecord {
                            file: file.clone(),
                            old_lines,
                            new_lines,
                            timestamp,
                        });
                        edits_payload.push(serde_json::json!({
                            "file": file,
                            "old_lines": old_lines,
                            "new_lines": new_lines,
                        }));
                    }
                }
                let mut completed_payload = serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "exit_code": res.exit_code,
                    "wall_ms": wall_started.elapsed().as_millis() as u64,
                });
                if res.timed_out {
                    completed_payload["timed_out"] = serde_json::json!(true);
                }
                // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): 中间回报
                // ——run_terminal_cmd 满 300s 自动后台化时，先记一条
                // `tool_running`（运行时长/进程状态/输出活跃度/落盘指针，
                // 单次仅一次），该调用随后照常收 `tool_completed` 并带
                // `running: true` 标记（命令仍在后台运行，exit_code=null）。
                if let Some(mid) = &res.mid_run {
                    let mut running_payload = serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "wall_ms": wall_started.elapsed().as_millis() as u64,
                        "task_id": mid.task_id,
                        "output_file": mid.output_file,
                    });
                    if let Some(pid) = mid.pid {
                        running_payload["pid"] = serde_json::json!(pid);
                    }
                    if let Some(total) = mid.total_bytes {
                        running_payload["total_bytes"] = serde_json::json!(total);
                    }
                    stamp_direct(&mut running_payload);
                    writer
                        .record(EventType::ToolRunning, running_payload)
                        .await?;
                    completed_payload["running"] = serde_json::json!(true);
                }
                if !edits_payload.is_empty() {
                    completed_payload["edits"] = serde_json::Value::Array(edits_payload);
                }
                // FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): journal
                // the mechanical candidate count/cap on candidate-counted
                // completions (Schema-first; the verifier cross-checks
                // the shape).
                if let Some((count, cap)) = &candidate_counts {
                    completed_payload["candidate_count"] = serde_json::json!(count);
                    completed_payload["candidate_cap"] = serde_json::json!(cap);
                }
                // GAP-ENCODING-GATE (OPS-PROTOCOL §8): record the decode
                // stage that produced the tool output (run_terminal_cmd /
                // read_file / run_tests) when the host observed one.
                if let Some(enc) = &res.output_encoding {
                    completed_payload["output_encoding"] = serde_json::json!(enc);
                }
                // P0-C S3 前置 (2026-08-15, P1-2 定案): a host-level
                // structured denial rides the ToolCompleted event too — it
                // must stay a self-describing refusal completion
                // (status=error + error code + non-zero exit_code), the same
                // shape as the controller-side no-ToolStarted refusals.
                if let Some(pd) = &res.policy_denial {
                    completed_payload["status"] = serde_json::json!("error");
                    completed_payload["error"] = serde_json::json!(pd.code);
                    completed_payload["policy_denial"] = serde_json::json!({
                        "source": pd.source.as_str(),
                        "code": pd.code,
                        "reason": pd.reason,
                    });
                }
                stamp_direct(&mut completed_payload);
                writer
                    .record(EventType::ToolCompleted, completed_payload)
                    .await?;
                // 2026-08-08 blackboard partition: fold the executed call
                // into the tool-action section (category from the dispatcher).
                self.blackboard.write().tool_actions.push(ToolActionRecord {
                    category: ToolDispatcher::action_category(&tc.name).to_string(),
                    tool: tc.name.clone(),
                    timestamp: chrono_utc_now(),
                });
                {
                    let mut w = self.blackboard.write();
                    w.exec.results.push(format!(
                        "[{}] {}{}",
                        tc.name,
                        res.output,
                        count_note.as_deref().unwrap_or("")
                    ));
                }
                // IP2a (D-3): 失败必显式 — a tool result must NEVER be blank
                // in the conversation (blank tool messages give the model
                // nothing to react to; a host that returns empty output is
                // surfaced as an explicit completion marker instead).
                let output = if res.output.trim().is_empty() {
                    format!(
                        "tool '{tool_name}' completed with no output (exit_code={exit:?})",
                        tool_name = tc.name,
                        exit = res.exit_code,
                    )
                } else {
                    res.output
                };
                (
                    ToolResult {
                        output,
                        exit_code: res.exit_code,
                        // GAP-ENCODING-GATE (OPS-PROTOCOL §8): 重建时透传
                        // 解码阶段——此前只进 journal（tool_completed.
                        // output_encoding）却从返回结果丢失；R2 半助理层
                        // 失败诊断/实体登记的 encoding_lossy 签名需要它。
                        output_encoding: res.output_encoding.clone(),
                        structured: None,
                        policy_denial: res.policy_denial.clone(),
                        timed_out: res.timed_out,
                        tool_error_kind: None,
                        // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.3): 透传
                        // host 计算的工作区 delta——终端/运行类订单 receipt
                        // 据此挂变更清单（run_tests 特殊路径已在上面透传）。
                        workspace_delta: res.workspace_delta.clone(),
                        workspace_delta_truncated: res.workspace_delta_truncated,
                        // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2):
                        // 中间回报结构化透传给 console 适配层（仍在运行 ≠
                        // 失败，不触发失败诊断）。
                        mid_run: res.mid_run.clone(),
                    },
                    true,
                )
            }
            Err(e) => {
                let timed_out = matches!(e, ToolError::Timeout(_));
                let mut err_payload = {
                    let mut payload = serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "status": "error",
                        "error": e.to_string(),
                        "wall_ms": wall_started.elapsed().as_millis() as u64,
                    });
                    if timed_out {
                        payload["timed_out"] = serde_json::json!(true);
                    }
                    // FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14):
                    // a candidate-counted host error still consumed
                    // its candidate — carry the count/cap for audit.
                    if let Some((count, cap)) = &candidate_counts {
                        payload["candidate_count"] = serde_json::json!(count);
                        payload["candidate_cap"] = serde_json::json!(cap);
                    }
                    payload
                };
                stamp_direct(&mut err_payload);
                writer.record(EventType::ToolCompleted, err_payload).await?;
                // P0-A step 5 (design §5): 调用即探针 — a real work-tool call
                // failure (ToolCompleted status=error) corrects the minimal
                // previous-round map; the next probe compares against it.
                self.maybe_note_probe_call_failure(probe_writeback, &tc.name);
                // 2026-08-08 blackboard partition: a failed execution still
                // HAPPENED — fold it into the tool-action section (the
                // "实际变动" rule applies to edit records, not to the action
                // ledger).
                self.blackboard.write().tool_actions.push(ToolActionRecord {
                    category: ToolDispatcher::action_category(&tc.name).to_string(),
                    tool: tc.name.clone(),
                    timestamp: chrono_utc_now(),
                });
                {
                    let mut w = self.blackboard.write();
                    w.exec.errors.push(format!(
                        "[{}] {e}{}",
                        tc.name,
                        count_note.as_deref().unwrap_or("")
                    ));
                }
                // P0-1 (2026-08-08 stall guards): a host-level timeout means
                // the tool was KILLED — the model must not read it as a
                // regular failure it can retry the same way (the reason
                // carries the budget; the journal records the same text in
                // `tool_completed.error`).
                (
                    ToolResult {
                        output: match &e {
                            ToolError::Timeout(reason) => {
                                format!("tool TIMED OUT — it did not complete: {reason}")
                            }
                            _ => format!("tool error: {e}"),
                        },
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        timed_out: matches!(e, ToolError::Timeout(_)),
                        // R2 半助理层：结构化工具错误类别透传（失败诊断
                        // 签名词典据此匹配 tool_not_found 等；不做文本判定）。
                        tool_error_kind: Some(match &e {
                            ToolError::NotFound(_) => crate::host::ToolErrorKind::NotFound,
                            ToolError::Timeout(_) => crate::host::ToolErrorKind::Timeout,
                            ToolError::ExecutionFailed(_) => {
                                crate::host::ToolErrorKind::ExecutionFailed
                            }
                        }),
                        ..Default::default()
                    },
                    false,
                )
            }
        };

        // FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): mechanical count
        // feedback rides the web_fetch tool result (design §1.2) — the
        // model decides full vs keyword fetch under a known budget. It is
        // appended to success AND host-error outputs (a failed fetch still
        // consumed its candidate).
        if let Some(note) = &count_note {
            result.output.push_str(note);
        }
        // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24, ADR-0010 §14.39)：
        // 候选计数/上限以结构化字段透传到 ToolResult——机械审查层
        // `retrieval:<n>` 分类据此读取真实 per-call 计数（而非仅靠激活
        // 池求和回退）；成功与失败（已消耗候选）都携带。
        if let Some((count, cap)) = candidate_counts {
            let mut structured = result
                .structured
                .take()
                .unwrap_or_else(|| serde_json::json!({}));
            structured["candidate_count"] = serde_json::json!(count);
            structured["candidate_cap"] = serde_json::json!(cap);
            result.structured = Some(structured);
        }

        // Replay the tool result into the conversation — the provider
        // protocol requires a tool message answering each declared call
        // (D2-1; the retrieval subagent path already did this, the host path
        // only mirrored the result into the blackboard — a real transport
        // would have seen `[user, decl, summary]` with no tool message and
        // rejected the round).
        messages.push(Message {
            role: Role::Tool,
            content: result.output.clone(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        let feedback = if succeeded {
            Some(PolicyFeedback::Succeeded)
        } else {
            None // timeout / tool error — neutral for the denial streak
        };
        Ok((result, feedback))
    }

    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): candidate count gate
    /// — count domain lookup, exact-string URL dedup and cap check (design
    /// §1), shared by the web_fetch family and `browser_read` (local_browser
    /// second segment). Runs BEFORE any fetch/read action and BEFORE
    /// ToolStarted / ACAF ticketing (a refused call needs no ticket). The
    /// decision is consumption-free: the caller commits the URL at the
    /// execution boundary after the permission/ACAF gates pass (review fix
    /// 2026-08-14).
    ///
    /// Fail-closed arms (per tool family, stable `{family}_candidate_*`
    /// codes):
    /// - no count domain (main/grill lane — retrieval tools never execute
    ///   there; belt-and-braces): `{family}_candidate_count_unbound`;
    /// - missing `url` argument (no count identity):
    ///   `{family}_candidate_url_missing`;
    /// - new URL at/over the cap: `{family}_candidate_cap_exceeded` —
    ///   no ToolStarted, neutral statement, Denied feedback (the
    ///   consecutive-denial breaker gives no retry space, ADR-0010
    ///   §3.5.4).
    pub(crate) async fn candidate_gate(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tc: &ToolCall,
        fetch_candidates: Option<&Mutex<Vec<String>>>,
    ) -> Result<CandidateGateDecision, AgentLoopError> {
        let prefix = candidate_tool_prefix(&tc.name);
        let Some(counter) = fetch_candidates else {
            return self
                .refuse_candidate(
                    writer,
                    messages,
                    tc,
                    &format!("{prefix}_candidate_count_unbound"),
                    &format!("{prefix} 已拒绝 — 候选核验计数域不可用"),
                    None,
                    false,
                )
                .await;
        };
        let Some(url) = tc
            .arguments
            .get("url")
            .and_then(|u| u.as_str())
            .map(str::to_string)
        else {
            return self
                .refuse_candidate(
                    writer,
                    messages,
                    tc,
                    &format!("{prefix}_candidate_url_missing"),
                    &format!("{prefix} 已拒绝 — 缺少 url 参数，候选核验无法计数"),
                    None,
                    true,
                )
                .await;
        };
        let cap = self.candidate_cap as usize;
        // Decision only — no mutation here; consumption commits at the
        // execution boundary (review fix 2026-08-14). The std MutexGuard
        // must not cross the async refusal below (Send).
        let outcome = {
            let seen = counter.lock().unwrap();
            let count = seen.len();
            let is_new = !seen.iter().any(|u| u == &url);
            if is_new && count >= cap {
                Err((count, cap))
            } else {
                Ok(())
            }
        };
        match outcome {
            Ok(()) => Ok(CandidateGateDecision::Allowed { url, cap }),
            Err((count, cap)) => {
                self.refuse_candidate(
                    writer,
                    messages,
                    tc,
                    &format!("{prefix}_candidate_cap_exceeded"),
                    &format!("{prefix} 已拒绝 — 候选核验数量已达上限 {cap}（当前 {count}/{cap}）"),
                    Some((count, cap)),
                    true,
                )
                .await
            }
        }
    }

    /// Shared no-ToolStarted refusal for the candidate gate — event +
    /// neutral tool message + Denied feedback (the breaker aggregates at
    /// round granularity and blocks repeated refusals).
    async fn refuse_candidate(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tc: &ToolCall,
        code: &str,
        msg: &str,
        counts: Option<(usize, usize)>,
        lane: bool,
    ) -> Result<CandidateGateDecision, AgentLoopError> {
        let mut payload = serde_json::json!({
            "tool": tc.name,
            "call_id": tc.call_id,
            "status": "error",
            "error": code,
        });
        // Only lane refusals carry the dispatch target: `count_unbound`
        // fires in a lane with no count domain (main/grill belt-and-braces),
        // where no dispatch occurred (review fix 2026-08-14).
        if lane {
            payload["target"] = serde_json::json!("external_retrieval");
        }
        if let Some((count, cap)) = counts {
            payload["candidate_count"] = serde_json::json!(count);
            payload["candidate_cap"] = serde_json::json!(cap);
        }
        writer.record(EventType::ToolCompleted, payload).await?;
        messages.push(Message {
            role: Role::Tool,
            content: msg.to_string(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        Ok(CandidateGateDecision::Refused(
            ToolResult {
                output: msg.to_string(),
                exit_code: Some(1),
                output_encoding: None,
                // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24)：拒绝信封带
                // 结构化错误码与计数——机械审查层据此精确识别候选超限/
                // 计数域未绑定异常，而非靠剩余池近似。
                structured: {
                    let mut s = serde_json::json!({ "error": code });
                    if let Some((count, cap)) = counts {
                        s["candidate_count"] = serde_json::json!(count);
                        s["candidate_cap"] = serde_json::json!(cap);
                    }
                    Some(s)
                },
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
