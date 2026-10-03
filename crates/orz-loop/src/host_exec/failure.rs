//! 失败信封与漏斗：`ToolFailureOutcome` 形状判定、失败目标聚合、LIF deny 通道、订单失败回写。
//! 0ai (2026-09-16) 拆分自 `host_exec.rs`（机械搬移，行为不变）。

use crate::console::{
    CODE_BROWSER_LAUNCH_FAILED, CODE_CONTENT_ANCHOR_MISMATCH, CODE_EXECUTION_FAILED,
    CODE_TOOL_NOT_FOUND, CODE_TOOL_TIMEOUT,
};
use crate::controller::AgentLoopController;
#[cfg(test)]
use crate::gateway::model::ToolCall;
use crate::host::ToolError;
#[cfg(test)]
use crate::host::{LoopHost, PermitDecision, ToolResult};
#[cfg(test)]
use orz_assurance::EventType;

/// 0q（ADR-0010 §14.63）：漏斗 `stamp_failure` 的原始结果形状——调用点
/// 只如实上报执行/装配结果，盖章与否由漏斗内集中形状谓词裁决。
pub(crate) enum ToolFailureOutcome<'a> {
    /// Err 臂：host 工具错误（code = ToolErrorKind 结构化码）。
    HostError(&'a ToolError),
    /// 拒绝完成（结构化 code；调用点保证无 `policy_denial` 信封）。
    Refused(&'a str),
    /// Ok 臂完成退出码（`None` = running/无退出语义）。
    CommandExit(Option<i32>),
    /// 子代理墙钟掐杀的 in-flight 合成收口（0k P3-4）：args 不可及，
    /// 身份定义性 None——恒落「有意不聚合」标记。
    SyntheticTimeout,
}

impl AgentLoopController {
    /// P2-10 R2 (2026-08-31): feed a structured denial event into the LIF
    /// deny channel — the shared entry point for every refusal path
    /// (anchor mismatch / candidate gate / retired / sealed / permission /
    /// ACAF / retrieval-mode / role / plan / budget). Mirrors
    /// `orz_assurance::lif::classify_event_outcome` exactly: a denial is
    /// its own outcome, not a host error (err) and not a D2 value exit
    /// (Other). `wall_ms` is None for no-ToolStarted refusals (no execution
    /// time was spent).
    ///
    /// 0am 审查处置（2026-10-03）：`deny_code`（结构化拒绝码）在此单源解析
    /// 为拒绝类标签维（S2 §4.2/§5 兑现——`DenyClass::of_code` 只在喂入点
    /// 调用，标签随事件入引擎）；通道值语义不变（Deny 1.0 照旧，deny 优先
    /// 分派）；未知码落 `Other`＝「其他拒绝」。
    pub(crate) fn feed_lif_deny(&self, wall_ms: Option<u64>, deny_code: &str) {
        let deny_class = orz_assurance::lif::DenyClass::of_code(deny_code);
        self.lif
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .on_tool_event(
                AgentLoopController::now_epoch_secs(),
                orz_assurance::lif::ToolEvent {
                    outcome: orz_assurance::lif::ToolOutcome::Deny,
                    wall_ms,
                    policy_denied: false,
                    routing: Some(orz_assurance::lif::StimulusRouting {
                        class: orz_assurance::lif::ActionClass::Neutral,
                        deny_class: Some(deny_class),
                        non_zero_exit: false,
                        write_control_block: false,
                    }),
                },
            );
    }

    /// P2-12 COMPRESSION-LINGUISTIC-FORMAL-LAYER 方案 A（2026-09-02）：
    /// F4 失败目标聚合的「写时盖章」入口——每次携带 `failure_target` 身份
    /// 的失败事件在写入时，取该事件所属决策轮的 LIF 域值与轮号盖章并记入
    /// 黑板 `failure_agg` 分区（epoch 作用域、随轮换重置）。时间为
    /// run-relative 墙钟秒（与 temporal `t` 同刻度）；`code` 必须是结构化
    /// 错误码（refusal code / ToolErrorKind 码），永不解析日志文本。
    fn note_failure_agg(&self, ft: &serde_json::Value, code: &str) {
        let Some(kind) = ft.get("kind").and_then(serde_json::Value::as_str) else {
            return;
        };
        let Some(id) = ft.get("id").and_then(serde_json::Value::as_str) else {
            return;
        };
        let Some(preview) = crate::failure_target::target_preview(ft) else {
            return;
        };
        let (round, domain, t_rel) = {
            let mut lif = self.lif.lock().unwrap_or_else(|e| e.into_inner());
            let now = AgentLoopController::now_epoch_secs();
            lif.ensure_run_origin(now);
            let round = lif.temporal().round();
            let domain = lif.temporal().current_domain();
            let t_rel = lif.run_relative_secs(now);
            (round, domain, t_rel)
        };
        self.blackboard
            .write()
            .failure_agg
            .record(kind, id, &preview, code, t_rel, round, domain);
    }

    /// 结构化 host 工具错误码（P2-10 R2 ToolErrorKind 三态 → 既有 console
    /// 码族；P2-12 聚合错误码集合同源）。
    ///
    /// 可复核口径（2026-09-02 审查登记）：对应 `tool_completed` 事件的
    /// `error` 字段是自由文本，本码只存在于聚合侧——§5 离线 verifier 须
    /// 复刻同一映射核对（或经独立 schema-first 切片给事件补结构化 code），
    /// 不得从 error 文本推导。
    fn host_error_code(e: &ToolError) -> &'static str {
        match e {
            ToolError::NotFound(_) => CODE_TOOL_NOT_FOUND,
            ToolError::Timeout(_) => CODE_TOOL_TIMEOUT,
            ToolError::ExecutionFailed(_) => CODE_EXECUTION_FAILED,
            // 0ac S3①：真实 cause 同族——稳定壳码不变（cause 另在
            // `tool_completed.cause` 上自描述）。
            ToolError::ExecutionFailedCaused { .. } => CODE_EXECUTION_FAILED,
            ToolError::BrowserLaunchFailed(_) => CODE_BROWSER_LAUNCH_FAILED,
            // P1-2a：启动成功但动作失败——不新增稳定码，归 ExecutionFailed 系。
            ToolError::BrowserStepFailed { .. } => CODE_EXECUTION_FAILED,
        }
    }

    /// 0ac S3① (2026-09-13, design §10.1 / §10.3 item 1)：失败载荷的
    /// **真实 `cause`** ——「单事件自描述」的写入侧（F-003 验收样本：
    /// 确定性不可达必须一步报因，而不是只给壳码）。取值优先级：
    ///
    /// 1. 工具错误 `details.cause`（生产侧自报的稳定码：本地分段检索的
    ///    `network_no_response` / `network_error` / `capability_unreachable`
    ///    / `empty_result` / `no_progress` 即由此进 `tool_completed`）；
    /// 2. 宿主错误码（`tool_not_found` / `tool_timeout` /
    ///    `execution_failed` 等真实类别）；
    /// 3. 拒绝码 / 命令退出码（`command_exit_<n>`）/ 合成超时
    ///    （`tool_timeout`）。
    ///
    /// 形状门与法官族 4（`failure_cause_shape`）**同一谓词**：只在失败形状
    /// （`exit_code != 0` 或 `status == "error"`）上写，绝不把 cause 挂到成功
    /// 形状。S2 壳码集合（`browser_launch_failed` / `tool_failed` / `failed`
    /// / `error` / `unknown_error`）由 schema `not.enum` 机械拒绝，故
    /// `browser_launch_failed` 映射到真实类别 `capability_unreachable`。
    fn failure_cause(
        payload: &serde_json::Value,
        outcome: &ToolFailureOutcome<'_>,
    ) -> Option<String> {
        let failure_shaped = payload.get("status").and_then(serde_json::Value::as_str)
            == Some("error")
            || payload
                .get("exit_code")
                .and_then(serde_json::Value::as_i64)
                .is_some_and(|code| code != 0);
        if !failure_shaped {
            return None;
        }
        if let ToolFailureOutcome::HostError(e) = outcome
            && let ToolError::ExecutionFailedCaused { cause, .. } = e
            && !cause.trim().is_empty()
        {
            return Some(cause.to_string());
        }
        // 宿主桥接未带 cause 时退回载荷上的既有 cause（外部生产侧自报），
        // 仍限失败形状（形状门在上方）。
        if let Some(cause) = payload
            .get("cause")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|cause| !cause.is_empty())
        {
            return Some(cause.to_string());
        }
        let derived = match outcome {
            ToolFailureOutcome::HostError(e) => match Self::host_error_code(e) {
                CODE_BROWSER_LAUNCH_FAILED => "capability_unreachable",
                other => other,
            },
            ToolFailureOutcome::Refused(code) => code,
            ToolFailureOutcome::CommandExit(Some(code)) => {
                return Some(format!("command_exit_{code}"));
            }
            ToolFailureOutcome::CommandExit(None) => return None,
            ToolFailureOutcome::SyntheticTimeout => CODE_TOOL_TIMEOUT,
        };
        (!derived.is_empty()).then(|| derived.to_string())
    }

    /// 0q 统一失败事件管线（2026-09-08，ADR-0010 §14.63）：写入侧边界
    /// 单一漏斗——工具完成装配点的唯一 `stamp_failure`。形状谓词集中在此
    /// 一处定义（OTel 语义约定形态），覆盖面由结构保证而非路径记忆：
    ///
    /// - 盖章（写 `failure_agg` 聚合 + 事件挂 `failure_target`）= 四写点
    ///   语义逐项对齐，不扩不缩：
    ///   ① host ToolError（Err 臂）→ code = ToolErrorKind 结构化码；
    ///   ② Ok 臂命令级失败（exit≠0 且无拒绝信封）→ code = `exit_{n}`；
    ///   ③ 锚点核证拒单（`content_anchor_mismatch`）；
    ///   ④ 候选门拒单（`*_candidate_*` 码族）；
    ///   且 `failure_target(tool, args)` 命中（None-identity 维持不进聚合
    ///   现状——0p S2 复审同款：覆盖面由谓词与身份联合决定）。
    /// - 标记（事件挂 `failure_agg_absent: true`）= error 形状、身份可及
    ///   工具、谓词未盖章且无 `policy_denial` 信封——「漏斗已评估、有意
    ///   不聚合」的 journal 可见事实。`policy_denial` 载荷自身即是否决
    ///   证据（0p S2 裁决：信封优先、不盖章），无需标记。
    /// - 其余（成功、中性、身份不可及工具）零作用。
    ///
    /// 每个工具结果恰过一次漏斗（error 形状装配点各接线一次）；漏盖由
    /// journal-conformance 法官对账族机械证明（`failure_agg_coverage`，
    /// 以 run_started `failure_pipeline: "funnel-v1"` 为 grandfather 锚）。
    pub(crate) fn stamp_failure(
        &self,
        payload: &mut serde_json::Value,
        tool: &str,
        arguments: &serde_json::Value,
        outcome: ToolFailureOutcome<'_>,
    ) {
        // 0ac S3① (2026-09-13, design §10.1 / §10.3 item 1)：失败载荷补 `cause`
        // ——每个 error 形状恰过一次本漏斗，cause 与身份面
        // （`failure_target`）并行、互不依赖（S2 契约：cause 无 target /
        // target 无 cause 两种形状都合法）。详见 `Self::failure_cause`。
        if let Some(cause) = Self::failure_cause(payload, &outcome) {
            payload["cause"] = serde_json::json!(cause);
        }
        // 身份不可及工具：谓词与标记都不适用（法官族按同表跳过）。
        if !crate::failure_target::identity_capable(tool) {
            return;
        }
        // 请求侧结构化拒绝信封（权限/ACAF/检索模式/两段门）＝0p S2 裁决
        // 的「信封优先、不盖章」——既不聚合也不标记，事件自证评估事实。
        if payload.get("policy_denial").is_some() {
            return;
        }
        let code: Option<String> = match &outcome {
            ToolFailureOutcome::HostError(e) => Some(Self::host_error_code(e).to_string()),
            ToolFailureOutcome::Refused(code) => {
                // 拒绝码白名单＝四写点中的两个拒单族（③④）；其余拒绝
                // （计划轮/角色门/注入预算/测试运行器缺席等）维持不进
                // 聚合现状，仅落标记。
                let refused = *code == CODE_CONTENT_ANCHOR_MISMATCH
                    || code.ends_with("_candidate_count_unbound")
                    || code.ends_with("_candidate_url_missing")
                    || code.ends_with("_candidate_cap_exceeded");
                refused.then(|| (*code).to_string())
            }
            ToolFailureOutcome::CommandExit(exit) => match exit {
                // Ok 臂：非零退出 = 命令级失败（0p S1 复审 F-C 口径原样
                // 收编）；零退出 / 无退出语义（running:true）非 error
                // 形状——漏斗零作用，不标记。
                Some(n) if *n != 0 => Some(format!("exit_{n}")),
                _ => return,
            },
            ToolFailureOutcome::SyntheticTimeout => None,
        };
        let Some(code) = code else {
            // 谓词未命中：error 形状仍在漏斗上——落「有意不聚合」标记。
            payload["failure_agg_absent"] = serde_json::json!(true);
            return;
        };
        let Some(ft) = crate::failure_target::failure_target(tool, arguments) else {
            // 形状命中但身份 None（全库 grep / 缺参 / run_tests 固定命令）：
            // 维持不进聚合现状（0q §3.2-1），同样落标记。
            payload["failure_agg_absent"] = serde_json::json!(true);
            return;
        };
        payload["failure_target"] = ft.clone();
        // 事件与聚合同源同刻：聚合写入先于事件发出（调用点保证本函数
        // 在 writer.record 之前执行）。
        self.note_failure_agg(&ft, &code);
    }

    /// 0q 第五族（ADR-0010 §14.63 裁决 ②）：console 订单失败收据的漏斗
    /// 接线——订单完成装配（receipt error 信封处）同过单一漏斗。身份
    /// `action_target {id = sha256(order_id), action}` 写入黑板
    /// `failure_agg`（订单业务失败此前不进聚合，是 0q 治本清单上的已知
    /// 缺口），receipt 错误信封同批挂载身份（「事件与聚合同源同刻」的
    /// 订单面等价物；收据信封不是 journal 事件，法官族面不涉及本族）。
    /// 聚合 code = 结构化 ConsoleError 码（order_stale / execution_failed
    /// 等），永不解析消息文本。
    pub(crate) fn note_order_failure(
        &self,
        error_value: &mut serde_json::Value,
        order_id: &str,
        action: &str,
        code: &str,
    ) {
        let ft = crate::failure_target::action_failure_target(order_id, action);
        self.note_failure_agg(&ft, code);
        error_value["failure_target"] = ft;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use crate::host::{PermitError, RiskClass, ToolRegistry};
    use async_trait::async_trait;
    use orz_assurance::JournalRecorder;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// P2-12（2026-09-02 方案 A）：F4 失败目标聚合的「写时盖章」e2e——
    /// 同一目标（read_file → file_target）连续两次 host 级超时：黑板
    /// `failure_agg` 分区只留一行（count=2、codes=[tool_timeout×2]、
    /// 域序列按失败事件所属决策轮盖章），事件面每个失败仍携带
    /// `failure_target`；exec 分区原文照旧，压缩「注意事项」槽才消费
    /// 聚合行（零模型、确定性）。
    #[tokio::test]
    async fn failure_target_aggregation_stamps_on_write() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        struct FailingTwiceHost {
            journal: JournalRecorder,
            calls: AtomicU64,
        }
        #[async_trait::async_trait]
        impl LoopHost for FailingTwiceHost {
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
                if n < 2 {
                    Err(ToolError::Timeout(
                        "tool killed after 300s wall-clock budget".into(),
                    ))
                } else {
                    Ok(crate::controller_test_support::ok_result())
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
        let host = FailingTwiceHost {
            journal,
            calls: AtomicU64::new(0),
        };
        let read = |id: &str| {
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"file_path": "a.txt"}),
                call_id: id.to_string(),
            }])
        };
        let fake = Arc::new(FakeProvider::new(vec![
            read("call-f1"),
            read("call-f2"),
            // 文本轮被 counterexample gate 拦截一轮，随后才是最终答案。
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(
                &host,
                "测试失败目标聚合",
                "RUN-FAIL-AGG",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok(), "{result:?}");

        // 事件面：两次失败均带 failure_target（file_target + path）。
        let completed: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        assert_eq!(completed.len(), 2, "{completed:?}");
        for c in &completed {
            assert_eq!(c["status"], "error");
            assert_eq!(c["failure_target"]["kind"], "file_target");
            assert_eq!(c["failure_target"]["path"], "a.txt");
        }

        // 黑板聚合分区：一行、count=2、结构化码集、域序列按轮盖章。
        let bb = controller.blackboard().read();
        let agg = &bb.failure_agg;
        assert_eq!(agg.rows.len(), 1, "{agg:?}");
        let row = &agg.rows[0];
        assert_eq!(row.kind, "file_target");
        assert_eq!(
            row.id,
            orz_assurance::journal::sha256_hex("a.txt".as_bytes())
        );
        assert_eq!(row.preview, "a.txt");
        assert_eq!(row.count, 2);
        assert_eq!(
            row.codes,
            vec![crate::failure_agg::CodeCount {
                code: "tool_timeout".into(),
                count: 2,
            }]
        );
        assert_eq!(row.segments.len(), 1);
        assert_eq!(row.segments[0].domain, orz_assurance::lif::Domain::Start);
        assert_eq!(row.segments[0].from_round, 1);

        // exec 分区原文照旧；压缩「注意事项」槽只呈现聚合行。
        assert_eq!(bb.exec.errors.len(), 2);
        let notes = crate::summary::render_facts_notes(&bb);
        assert!(
            notes.text.contains("[失败目标 file_target] a.txt ×2"),
            "{}",
            notes.text
        );
        assert!(
            notes.text.contains("codes=[tool_timeout×2]"),
            "{}",
            notes.text
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ==== 0q 统一失败事件管线（2026-09-08，ADR-0010 §14.63）====
    // 漏斗 `stamp_failure` 的集中形状谓词矩阵 + Ok 臂 e2e 对拍。

    /// 谓词矩阵：盖章 / 标记 / 零作用三态在单一漏斗上逐形状核对。
    #[test]
    fn funnel_shape_predicate_matrix() {
        let controller = AgentLoopController::default();
        let cmd_args = serde_json::json!({"command": "pip install fasttext"});
        let grep_args = serde_json::json!({"pattern": "x"});
        let url_args = serde_json::json!({"url": "http://example.com/a"});
        let tool_not_found = ToolError::NotFound("no such tool".into());

        // ① host ToolError + 身份命中 → 盖章（聚合行 + 事件身份）。
        let mut payload = serde_json::json!({"tool": "run_terminal_cmd"});
        controller.stamp_failure(
            &mut payload,
            "run_terminal_cmd",
            &cmd_args,
            ToolFailureOutcome::HostError(&tool_not_found),
        );
        assert!(payload.get("failure_target").is_some());
        assert!(payload.get("failure_agg_absent").is_none());
        assert_eq!(controller.blackboard().read().failure_agg.rows.len(), 1);

        // ② Ok 臂命令级失败 + 身份命中 → 盖章（exit_{n}）。不同命令 =
        // 不同 (kind, id) 行（聚合按身份去重，同命令会并为一行）。
        let mut payload = serde_json::json!({"tool": "run_terminal_cmd"});
        controller.stamp_failure(
            &mut payload,
            "run_terminal_cmd",
            &serde_json::json!({"command": "make -j8"}),
            ToolFailureOutcome::CommandExit(Some(2)),
        );
        assert!(payload.get("failure_target").is_some());
        assert_eq!(controller.blackboard().read().failure_agg.rows.len(), 2);

        // ③ 锚点拒单 + ④ 候选门拒单（白名单拒绝码）→ 盖章。
        let mut payload = serde_json::json!({"tool": "search_replace"});
        controller.stamp_failure(
            &mut payload,
            "search_replace",
            &serde_json::json!({"file_path": "a.rs"}),
            ToolFailureOutcome::Refused(CODE_CONTENT_ANCHOR_MISMATCH),
        );
        assert!(payload.get("failure_target").is_some());
        let mut payload = serde_json::json!({"tool": "web_fetch"});
        controller.stamp_failure(
            &mut payload,
            "web_fetch",
            &url_args,
            ToolFailureOutcome::Refused("web_fetch_candidate_cap_exceeded"),
        );
        assert!(payload.get("failure_target").is_some());
        assert_eq!(controller.blackboard().read().failure_agg.rows.len(), 4);

        // 标记态：error 形状 + 身份 None（全库 grep）→ 只落
        // failure_agg_absent，聚合零行（None-identity 现状不变）。
        let mut payload = serde_json::json!({"tool": "grep"});
        controller.stamp_failure(
            &mut payload,
            "grep",
            &grep_args,
            ToolFailureOutcome::HostError(&tool_not_found),
        );
        assert_eq!(payload["failure_agg_absent"], serde_json::json!(true));
        assert!(payload.get("failure_target").is_none());

        // 标记态：非白名单拒绝码（计划轮/角色门/预算/测试运行器缺席）。
        let mut payload = serde_json::json!({"tool": "read_file"});
        controller.stamp_failure(
            &mut payload,
            "read_file",
            &serde_json::json!({"file_path": "a.py"}),
            ToolFailureOutcome::Refused("missing_test_runner"),
        );
        assert_eq!(payload["failure_agg_absent"], serde_json::json!(true));

        // 零作用态：零退出 / 无退出语义（非 error 形状）不盖章不标记。
        let mut payload = serde_json::json!({"tool": "run_terminal_cmd"});
        controller.stamp_failure(
            &mut payload,
            "run_terminal_cmd",
            &cmd_args,
            ToolFailureOutcome::CommandExit(Some(0)),
        );
        let mut payload = serde_json::json!({"tool": "run_terminal_cmd"});
        controller.stamp_failure(
            &mut payload,
            "run_terminal_cmd",
            &cmd_args,
            ToolFailureOutcome::CommandExit(None),
        );
        assert!(payload.get("failure_target").is_none());
        assert!(payload.get("failure_agg_absent").is_none());

        // 零作用态：policy_denial 信封优先（0p S2 口径）——两字段都不落。
        let mut payload = serde_json::json!({
            "tool": "grep",
            "policy_denial": {"source": "permission", "code": "session_volume_notice", "reason": "r"},
        });
        controller.stamp_failure(
            &mut payload,
            "grep",
            &grep_args,
            ToolFailureOutcome::CommandExit(Some(1)),
        );
        assert!(payload.get("failure_target").is_none());
        assert!(payload.get("failure_agg_absent").is_none());

        // 零作用态：身份不可及工具（计划轮黑板面等）——跳过。
        let mut payload = serde_json::json!({"tool": "plan_write"});
        controller.stamp_failure(
            &mut payload,
            "plan_write",
            &serde_json::json!({}),
            ToolFailureOutcome::HostError(&tool_not_found),
        );
        assert!(payload.get("failure_target").is_none());
        assert!(payload.get("failure_agg_absent").is_none());

        // 聚合面终态：只有四个盖章形状入行，标记态零行。
        let agg = &controller.blackboard().read().failure_agg;
        assert_eq!(agg.rows.len(), 4, "{agg:?}");
    }

    /// Ok 臂命令级失败 e2e 对拍（写点 ②）：run_terminal_cmd exit≠0 →
    /// 完成事件挂 failure_target（此前仅 Err 臂挂载）+ 聚合行
    /// cmd_target·exit_{n}，事件与聚合同源。
    #[tokio::test]
    async fn funnel_ok_arm_command_failure_stamps_agg_and_event() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        struct CmdFailHost {
            journal: JournalRecorder,
        }
        #[async_trait]
        impl LoopHost for CmdFailHost {
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
                Ok(ToolResult {
                    output: "error: externally-managed-environment".to_string(),
                    exit_code: Some(1),
                    output_encoding: Some("utf-8".to_string()),
                    ..Default::default()
                })
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
        let host = CmdFailHost { journal };
        let call = |id: &str| {
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "run_terminal_cmd".to_string(),
                arguments: serde_json::json!({"command": "pip install fasttext"}),
                call_id: id.to_string(),
            }])
        };
        let fake = Arc::new(FakeProvider::new(vec![
            call("call-q1"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(
                &host,
                "测试 Ok 臂命令级失败漏斗",
                "RUN-OK-EXIT",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok(), "{result:?}");

        let completed: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        assert_eq!(completed.len(), 1, "{completed:?}");
        let c = &completed[0];
        // 事件面：命令级失败携带身份（0q 补齐 Ok 臂事件面对账物）。
        assert_eq!(c["exit_code"], 1);
        assert_eq!(c["failure_target"]["kind"], "cmd_target");
        assert!(c.get("failure_agg_absent").is_none());

        // 聚合面：cmd_target 一行，code = exit_1。
        let bb = controller.blackboard().read();
        let agg = &bb.failure_agg;
        assert_eq!(agg.rows.len(), 1, "{agg:?}");
        let row = &agg.rows[0];
        assert_eq!(row.kind, "cmd_target");
        assert_eq!(
            row.id,
            orz_assurance::journal::sha256_hex(b"pip install fasttext")
        );
        assert_eq!(row.count, 1);
        assert_eq!(row.codes[0].code, "exit_1");

        // run_started 带 funnel-v1 版本锚（法官对账族的 grandfather 锚）。
        let started: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::RunStarted)
            .map(|e| e.payload)
            .collect();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0]["failure_pipeline"], "funnel-v1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// None-identity e2e 对拍：全库 grep（无 path）host 错误 → 完成事件
    /// 落「有意不聚合」标记，聚合零行——漏盖与有意排除在 journal 面可分。
    #[tokio::test]
    async fn funnel_none_identity_error_carries_marker_only() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        struct GrepFailHost {
            journal: JournalRecorder,
        }
        #[async_trait]
        impl LoopHost for GrepFailHost {
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
                Err(ToolError::ExecutionFailed("rg crashed".into()))
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
        let host = GrepFailHost { journal };
        let call = |id: &str| {
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "grep".to_string(),
                arguments: serde_json::json!({"pattern": "x"}),
                call_id: id.to_string(),
            }])
        };
        let fake = Arc::new(FakeProvider::new(vec![
            call("call-g1"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(
                &host,
                "测试 None-identity 标记",
                "RUN-GREP-MARKER",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await;
        assert!(result.is_ok(), "{result:?}");

        let completed: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        assert_eq!(completed.len(), 1, "{completed:?}");
        assert_eq!(completed[0]["failure_agg_absent"], serde_json::json!(true));
        assert!(completed[0].get("failure_target").is_none());
        assert!(controller.blackboard().read().failure_agg.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-10 R2 (2026-08-31): a permission-denied tool call feeds the LIF
    /// deny channel (τ=120s, θ=4) — the refusal is a deny event, NOT an err
    /// (host execution error) and NOT a D2 value exit (Other).
    #[tokio::test]
    async fn permission_deny_feeds_lif_deny_channel() {
        struct DenyHost {
            journal: JournalRecorder,
        }
        #[async_trait]
        impl LoopHost for DenyHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            async fn request_permission(
                &self,
                _risk: RiskClass,
                _tool: &str,
                _args: &serde_json::Value,
            ) -> Result<PermitDecision, PermitError> {
                Ok(PermitDecision::Deny)
            }
            async fn call_tool(
                &self,
                _name: &str,
                _args: serde_json::Value,
                _call_id: &str,
            ) -> Result<ToolResult, ToolError> {
                unreachable!("denied tools never execute");
            }
        }

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = DenyHost { journal };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("search_replace", "call-r2-1")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "改文件",
                "RUN-R2-DENY",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let guard = controller.lif.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            guard.deny().u() > 0.0,
            "deny channel must receive the permission denial (u={})",
            guard.deny().u()
        );
        assert_eq!(
            guard.temporal().total_tool_events(),
            1,
            "denial counts as one tool event"
        );
        assert_eq!(
            guard.temporal().total_errors(),
            0,
            "a denial is not an execution error"
        );
        assert_eq!(guard.err().fire_count(), 0);
        drop(guard);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-10 R2 (2026-08-31): a sealed-tool refusal (host_exec no-ToolStarted
    /// narrow gate) feeds the deny channel the same way as a permission
    /// denial — the structured refusal code is a deny, not an err.
    #[tokio::test]
    async fn sealed_tool_denial_feeds_lif_deny_channel() {
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
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("compaction_whitelist_add", "call-r2-2")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "写白名单",
                "RUN-R2-SEALED",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let guard = controller.lif.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            guard.deny().u() > 0.0,
            "deny channel must receive the sealed-tool denial (u={})",
            guard.deny().u()
        );
        assert_eq!(guard.temporal().total_tool_events(), 1);
        assert_eq!(guard.temporal().total_errors(), 0);
        drop(guard);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-10 R2 (2026-08-31): an anchor-mismatch refusal (search_replace
    /// carrying a stale expected_anchor) feeds the deny channel — the
    /// GetPut write guard is a structured rejection, not an execution
    /// error (design §5.2 `anchor_target` family).
    #[tokio::test]
    async fn anchor_mismatch_feeds_lif_deny_channel() {
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
        // Target file with actual content "actual" (size=6); the expected
        // anchor points at "expected" (size=8) — the size fast-path alone
        // must reject before any edit.
        let target = host.session_cwd().join("guard_target.txt");
        std::fs::write(&target, "actual").unwrap();
        let expected_sha256 = orz_assurance::sha256_hex(b"expected");
        let tc = ToolCall {
            name: "search_replace".to_string(),
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
            call_id: "call-r2-3".to_string(),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tc]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "改文件",
                "RUN-R2-ANCHOR",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let guard = controller.lif.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            guard.deny().u() > 0.0,
            "deny channel must receive the anchor mismatch (u={})",
            guard.deny().u()
        );
        assert_eq!(guard.temporal().total_tool_events(), 1);
        assert_eq!(guard.temporal().total_errors(), 0);
        drop(guard);
        // Zero edits: the target file is untouched.
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "actual");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── IP2a (FIX_PLAN 2026-08-06 D-3): policy-aware tool projection ──────
}
