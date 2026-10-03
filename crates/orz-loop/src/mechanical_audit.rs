//! MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / BACKLOG 0g /
//! TODO P0-0g): 静默机械审查层——运行内审查表，以对象键为单位记录执行
//! 事实；每对象键仅保留最后一轮结果（覆盖写）；不给建议、不注入运行中
//! 反馈；最终答案前中立问询轮随 [COUNTEREXAMPLE_GATE] 同轮以独立块
//! `[MECHANICAL_AUDIT v0.1]` 注入报告。报告收敛为执行事实摘要（仅三类：
//! 执行事实 / 墙钟 / 异常事实；0cn S2 起预算行退役）；step/契约类只事件
//! 留痕、不上报告。
//!
//! 对象键（设计 §2.4）：
//! - `file:<path>`：search_replace 结果（matched/created、diff 规模）；
//! - `cmd:<call_id>`：run_terminal_cmd 结果（exit code、timeout、stdout
//!   截断）；
//! - `plan`：首轮计划门结果（accepted/degraded、步骤数、最近修订）；
//! - `budget`：**已退役**（0cn S2，2026-10-02，ADR-0010 §14.82）——轮数
//!   记次注入（「已用 N/999 轮」）生产零写入，枚举/键保留仅供历史
//!   journal 回放校验；
//! - `retrieval:<n>`：检索派发（候选 / 上限）；
//! - `rli.notice.<kind>`：**0cp D4**（2026-10-03，ADR-0010 §14.83）RLI
//!   提醒附注式直投留痕——fire 当刻行文附于回传内容后＋journal 一条
//!   （不进报告块；盲区闭环）。

use orz_assurance::EventType;
use orz_assurance::lif::RliNoticeKind;

use crate::controller::EventWriter;
use crate::gateway::model::ToolCall;
use crate::host::ToolResult;

/// **0cp D4**：`rli.notice.<kind>` 对象键的 kind 段（snake_case；单一源，
/// 写入点与 schema 描述同引）。四种触发源＝四类 [`RliNoticeKind`]。
pub(crate) fn rli_notice_key(kind: RliNoticeKind) -> &'static str {
    match kind {
        RliNoticeKind::StreakCrossed => "streak_crossed",
        RliNoticeKind::MigrationConfirmed => "migration_confirmed",
        RliNoticeKind::CoverageGap => "coverage_gap",
        RliNoticeKind::DomainSpikeEntry => "domain_spike_entry",
    }
}

/// 报告块前缀——注册进 `prompt::is_injected_block_text`（机械注入文本，
/// 绝不持久化回会话）。
pub(crate) const MECHANICAL_AUDIT_PREFIX: &str = "[MECHANICAL_AUDIT";

/// 审查表容量上限（超限丢最旧键；每键保留最新）。
pub(crate) const MECHANICAL_AUDIT_CAPACITY: usize = 128;

/// `mechanical_audit_update` 事件 `kind` 闭枚举（单一源；写入点一律引用
/// 常量）。runtime schema
/// `mechanical-audit-update-event-payload-v0.2.schema.json` 的 kind 枚举与
/// payload required 由本模块测试的契约钉子逐字互证——新增 kind 必须同批
/// 同步 schema 与钉子（0AE-C2：0ae 批「实现常量先行、枚举不对账」曾使
/// 首次阶梯触发即产生 schema-invalid journal）。
pub(crate) const KIND_TOOL_RESULT: &str = "tool_result";
pub(crate) const KIND_PLAN_GATE: &str = "plan_gate";
/// **已退役（历史回放保留）**：0cn S2（2026-10-02，ADR-0010 §14.82 用户
/// 裁决）——budget 轮数记次注入（「已用 N/999 轮」逐批覆盖写）整体撤除；
/// 999 为占位值不接真实预算语义、不设替代物（接受时间感回归，轮次感由
/// temporal／域迁移行族承载）。枚举值保留只为**历史 journal 仍可校验**
/// （删值＝让旧刊判 invalid）；生产零写入由单测钉子钉住（本模块
/// `mechanical_audit_report_injected_with_final_answer_gate`）。
#[allow(dead_code)] // 保留供历史 journal 校验与 schema 枚举钉子（生产零写入）
pub(crate) const KIND_BUDGET: &str = "budget";
/// **已退役（历史回放保留）**：0ae D2 注意力阶梯的触发 kind。动态上下文
/// 滑块 S1（2026-09-15，设计 §3.4 用户裁定 R3）把 D2 整体下线，生产侧
/// **零写入点**；枚举值保留只为**历史 journal 仍可校验**（移动端/A-B 会
/// 回放旧 run 的 journal，删值＝让旧刊判 invalid）。新写入一律用
/// [`KIND_CONTEXT_SCALE`]。
#[allow(dead_code)] // 保留供历史 journal 校验与 schema 枚举钉子（生产零写入）
pub(crate) const KIND_ATTENTION_LADDER: &str = "attention_ladder";
/// 动态上下文滑块 S1（2026-09-15，设计 §3.2/§3.4）：实际上下文刻度提醒
/// 与压缩开窗的落账 kind——键形 `context_scale:<500k|900k|first_fold>`
/// （A4 首次驱逐固化提醒、A6 两级刻度提醒、A7 随提醒开窗）。
pub(crate) const KIND_CONTEXT_SCALE: &str = "context_scale";
pub(crate) const KIND_MODEL_COMPRESSION: &str = "model_compression";
pub(crate) const KIND_PLAN_WRITE_GUIDANCE: &str = "plan_write_guidance";
/// 0av S1（2026-09-20 立项，S3 摩擦 N3）：检索批次数读数落盘面——倒数行
/// 只入模型面消息、headless 侧车受 500K 归档里程碑门（ADR-0010 §14.68）
/// ⇒ 未达里程碑的 run 结构性无落盘面，0ar 判据 7 后段不可核。本 kind 在
/// **每批检索收尾时**落一条 journal 读数（activation_id/usable/cap/
/// retrieval_calls/terminal_reason，与 batch_close 单源 helper 同值），
/// 模型面零改动、不进审查表/报告块。payload 形状与本族其余 kind 的
/// {key,round,summary,anomaly} 不同（schema 按 kind 条件分支校验）。
pub(crate) const KIND_RETRIEVAL_BATCH: &str = "retrieval_batch";
/// 0bg S2（2026-09-22，用户裁决「连带记录」）：LIF 域迁移**连带记录**——
/// LIF `域迁移+n` 即时计数徽章撤出模型面后（模型面只留 RLI 确认式），迁移
/// 事实改由机械层留痕：key＝`lif.domain_migration`（每键一条覆盖写）、
/// summary＝`{from}→{to}@r{n}；累计 m 次`、anomaly＝null（**不进报告块**）。
/// **只记不发模型**、逐次历史由 journal 事件流（本 kind 的覆盖写＋LIF
/// 事件面）可离线复算。payload 形状＝均一四键（同 tool_result 族）。
pub(crate) const KIND_LIF_DOMAIN: &str = "lif_domain";
/// **0cp S2（D4，2026-10-03，163 批立项）**：RLI 提醒**附注式直投**留痕
/// ——三类 RliNotice（StreakCrossed／MigrationConfirmed／CoverageGap）与
/// 0cp D7 新增 DomainSpikeEntry 在**触发当刻**转附注式直投：agent_loop 于
/// 下一工具批回传装配点取走（`take_pending_rli_notices`，取走即置位
/// `delivered`），把行文附于回传内容后（行 ≤320B（0am P8-b 重订）、只读
/// 数无建议）并落本
/// kind 一条 journal 行——162 批 §3.1「提醒面 journal 盲区」闭环（fire
/// 全集可由 journal 独立复算）。key＝`rli.notice.<kind>`（每键覆盖写，逐
/// 次历史由 journal 事件流可复算）、anomaly＝null（不进报告块）。取代
/// 152 批 §5.2「注入不再增加」的**局部**取代声明见 ADR-0010 §14.83：fire
/// 当刻事件附注放行、周期性读数注入维持出局。
pub(crate) const KIND_RLI_NOTICE: &str = "rli_notice";

/// 一条对象键的审查结果（每键至多一条，新结果覆盖旧结果）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuditEntry {
    pub key: String,
    /// 写入该键最后一个结果的模型轮。
    pub round: u32,
    /// 机械事实摘要（无建议、无引导）。
    pub summary: String,
    /// 异常事实（exit code / 超时 / 锚点拒单 / 候选超限）；`None` = 无。
    pub anomaly: Option<String>,
}

/// 运行内审查表（run 级状态——run 起始创建、run 结束即弃）。
#[derive(Debug, Default)]
pub(crate) struct MechanicalAuditState {
    entries: Vec<AuditEntry>,
    /// 检索派发序号——`retrieval:<n>` 键的递增源（run 内单调）。
    retrieval_seq: u32,
}

impl MechanicalAuditState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 记录/覆盖一个对象键。返回轻量 `mechanical_audit_update` 事件 payload
    /// （键/轮/摘要/异常；覆盖写动作与首次写入都留痕）。
    pub(crate) fn record(
        &mut self,
        key: impl Into<String>,
        round: u32,
        summary: impl Into<String>,
        anomaly: Option<String>,
    ) -> serde_json::Value {
        let key = key.into();
        let summary = summary.into();
        if let Some(entry) = self.entries.iter_mut().find(|e| e.key == key) {
            entry.round = round;
            entry.summary = summary;
            entry.anomaly = anomaly;
        } else {
            self.entries.push(AuditEntry {
                key: key.clone(),
                round,
                summary,
                anomaly,
            });
            if self.entries.len() > MECHANICAL_AUDIT_CAPACITY {
                // 容量有界：超限丢最旧键（首见顺序最旧）。
                self.entries.remove(0);
            }
        }
        let entry = self
            .entries
            .iter()
            .find(|e| e.key == key)
            .expect("entry recorded above");
        serde_json::json!({
            "key": entry.key,
            "round": entry.round,
            "summary": entry.summary,
            "anomaly": entry.anomaly,
        })
    }

    /// 记录一次检索派发（`retrieval:<n>`；n 为 run 内派发序号）。
    pub(crate) fn record_retrieval(
        &mut self,
        round: u32,
        summary: impl Into<String>,
        anomaly: Option<String>,
    ) -> serde_json::Value {
        self.retrieval_seq = self.retrieval_seq.saturating_add(1);
        self.record(
            format!("retrieval:{}", self.retrieval_seq),
            round,
            summary,
            anomaly,
        )
    }

    /// 报告块（设计 §2.4：收敛为执行事实摘要——仅三类；每键至多一条；
    /// 无建议、无引导）。`wallclock` 为 run 已用墙钟。0cn S2 起预算行
    /// 退役（ADR-0010 §14.82）：不再渲染轮数记次，墙钟行保留。
    pub(crate) fn report(&self, wallclock: Option<std::time::Duration>) -> String {
        let mut lines = vec![format!("{MECHANICAL_AUDIT_PREFIX} v0.1]")];
        let exec: Vec<&AuditEntry> = self
            .entries
            .iter()
            .filter(|e| e.key.starts_with("file:") || e.key.starts_with("cmd:"))
            .collect();
        if exec.is_empty() {
            lines.push("执行事实：暂无".to_string());
        } else {
            lines.push("执行事实：".to_string());
            for e in exec {
                lines.push(format!("- {} → {}", e.key, e.summary));
            }
        }
        if let Some(d) = wallclock {
            lines.push(format!("墙钟约 {}s", d.as_secs()));
        }
        let anomalies: Vec<&AuditEntry> = self
            .entries
            .iter()
            .filter(|e| e.anomaly.is_some())
            .collect();
        if anomalies.is_empty() {
            lines.push("异常事实：无".to_string());
        } else {
            lines.push("异常事实：".to_string());
            for e in anomalies {
                lines.push(format!(
                    "- {} → {}",
                    e.key,
                    e.anomaly.as_deref().unwrap_or_default()
                ));
            }
        }
        lines.push("[/MECHANICAL_AUDIT]".to_string());
        lines.join("\n")
    }

    /// 当前条目数（测试/容量断言；仅测试使用）。
    #[cfg(test)]
    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// 工具结果的静默审计记录——按对象键分类：
/// - `search_replace` → `file:<path>`（含 diff 规模/锚点拒单异常）；
/// - `run_terminal_cmd` → `cmd:<call_id>`（exit code/超时/截断）；
/// - 检索派发族 → `retrieval:<n>`（候选/上限，从结构化 payload 读）。
///
/// 其余工具不进入审查表（执行事实聚焦动作与文件 delta；设计 §2.4 数据
/// 面仅列上述键）。写入时以轻量 `mechanical_audit_update` 事件留痕。
pub(crate) async fn record_tool_result(
    audit: &mut MechanicalAuditState,
    writer: &mut EventWriter<'_>,
    tc: &ToolCall,
    result: &ToolResult,
    round: u32,
    retrieval_candidates: Option<(usize, u32)>,
) -> Result<(), AgentLoopError> {
    let (key, summary, anomaly) = classify(tc, result, retrieval_candidates);
    let Some(key) = key else {
        return Ok(());
    };
    let payload = if key == "retrieval" {
        audit.record_retrieval(round, summary, anomaly)
    } else {
        audit.record(key, round, summary, anomaly)
    };
    writer
        .record(
            EventType::MechanicalAuditUpdate,
            serde_json::json!({
                "kind": KIND_TOOL_RESULT,
                "payload": payload,
            }),
        )
        .await?;
    Ok(())
}

/// 分类函数——返回 (对象键, 摘要, 异常)。非审查面工具返回 `None`。
fn classify(
    tc: &ToolCall,
    result: &ToolResult,
    retrieval_candidates: Option<(usize, u32)>,
) -> (Option<String>, String, Option<String>) {
    match tc.name.as_str() {
        "search_replace" => {
            let path = tc
                .arguments
                .get("file_path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("<unknown>");
            // 锚点拒单：写前核证失败返回 order_stale 形态错误信封。
            let anchor_anomaly = result
                .structured
                .as_ref()
                .and_then(|s| s.get("error"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    if result.exit_code != Some(0) {
                        Some(format!("exit {}", result.exit_code.unwrap_or(-1)))
                    } else {
                        None
                    }
                });
            let summary = if result.exit_code == Some(0) {
                let delta = delta_summary(&result.workspace_delta);
                format!("search_replace 成功{}", delta)
            } else {
                format!(
                    "search_replace 未应用{}",
                    anchor_anomaly
                        .as_deref()
                        .map(|a| format!("（{a}）"))
                        .unwrap_or_default()
                )
            };
            (Some(format!("file:{path}")), summary, anchor_anomaly)
        }
        "run_terminal_cmd" => {
            // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2 审查处理
            // P1-2): 中间回报（命令仍在后台运行，exit_code=None）不是
            // 失败——摘要中性标注「运行中」，不记异常事实（杜绝机械审计
            // 层把合法长命令误报为 `exit -1`）。
            let mid_running = result.mid_run.is_some();
            let exit = if mid_running {
                "运行中".to_string()
            } else {
                result
                    .exit_code
                    .map(|c| format!("exit {c}"))
                    .unwrap_or_else(|| "exit n/a".to_string())
            };
            let timed = if result.timed_out { "，超时" } else { "" };
            // OUTPUT-DEGENERATION-GUARD (2026-08-19)：终端输出超限截断携带
            // `[truncated:` 机械标记（grok_build/bash 输出脚注）。设计 §2.4
            // 的 `cmd:` 事实 = exit code / timeout / stdout 截断——截断在
            // 摘要中单列，不再以文件 delta 顶替（文件改动归事件面与
            // `file:` 键；旧实现此处括号不配对且语义错位）。
            let truncated = if result.output.contains("[truncated:") {
                "，输出截断"
            } else {
                ""
            };
            let anomaly = if result.timed_out {
                Some("超时".to_string())
            } else if result.exit_code != Some(0) && !mid_running {
                Some(format!("exit {}", result.exit_code.unwrap_or(-1)))
            } else {
                None
            };
            (
                Some(format!("cmd:{}", tc.call_id)),
                format!("{exit}{timed}{truncated}"),
                anomaly,
            )
        }
        name if crate::relay::is_retrieval_dispatch_name(name)
            || crate::relay::is_retrieval_mode_gated_host_tool(name) =>
        {
            // 检索派发：候选计数/上限来自结构化 payload（候选超限拒单带
            // candidate_count）；正常结果不带时用激活候选计数（主车道
            // 派发后从 activation 读取），再退化为 n/a。
            let candidate = result
                .structured
                .as_ref()
                .and_then(|s| s.get("candidate_count"))
                .and_then(serde_json::Value::as_u64);
            let cap = result
                .structured
                .as_ref()
                .and_then(|s| s.get("candidate_cap"))
                .and_then(serde_json::Value::as_u64);
            let (candidate, cap) = match (candidate, cap) {
                (None, None) => match retrieval_candidates {
                    Some((c, m)) => (Some(c as u64), Some(m as u64)),
                    None => (None, None),
                },
                other => other,
            };
            let count_note = match (candidate, cap) {
                (Some(c), Some(m)) => format!("候选 {c}/{m}"),
                (Some(c), None) => format!("候选 {c}"),
                _ => "候选 n/a".to_string(),
            };
            let anomaly = if result.exit_code != Some(0) {
                // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24)：优先读结构化
                // 错误码（候选门拒绝信封 `{family}_candidate_*` 已透传到
                // ToolResult.structured）——精确识别候选超限/计数域未绑定，
                // 不再依赖"剩余池 ≥ cap"近似；计数近似保留为兜底。
                let code = result
                    .structured
                    .as_ref()
                    .and_then(|s| s.get("error"))
                    .and_then(serde_json::Value::as_str);
                if code.is_some_and(|c| c.contains("candidate_cap_exceeded")) {
                    Some(format!("候选超限（{count_note}）"))
                } else if code.is_some_and(|c| c.contains("candidate_count_unbound")) {
                    Some(format!("候选计数域未绑定（{count_note}）"))
                } else if candidate.zip(cap).is_some_and(|(c, m)| c >= m) {
                    Some(format!("候选超限（{count_note}）"))
                } else {
                    Some(format!("exit {}", result.exit_code.unwrap_or(-1)))
                }
            } else {
                None
            };
            // 键的序号由 `record_retrieval` 在记录时分配；此处只传分类。
            (Some("retrieval".to_string()), count_note, anomaly)
        }
        _ => (None, String::new(), None),
    }
}

/// diff 规模摘要（workspace_delta 的机械计数行；空 = 无变化）。
fn delta_summary(delta: &[crate::host::WorkspaceDeltaEntry]) -> String {
    if delta.is_empty() {
        return String::new();
    }
    let mut added = 0usize;
    let mut modified = 0usize;
    let mut deleted = 0usize;
    for e in delta {
        match e.kind {
            crate::host::WorkspaceDeltaKind::Added => added += 1,
            crate::host::WorkspaceDeltaKind::Modified => modified += 1,
            crate::host::WorkspaceDeltaKind::Deleted => deleted += 1,
        }
    }
    let mut parts = Vec::new();
    if added > 0 {
        parts.push(format!("+{added}"));
    }
    if modified > 0 {
        parts.push(format!("~{modified}"));
    }
    if deleted > 0 {
        parts.push(format!("-{deleted}"));
    }
    format!("（{} 文件）", parts.join("/"))
}

use crate::controller::AgentLoopError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use crate::gateway::model::ToolCall;
    use crate::host::ToolResult;
    use orz_assurance::{EventType, JournalRecorder, RunEvent};
    use std::sync::Arc;

    fn call(name: &str, id: &str, args: serde_json::Value) -> ToolCall {
        ToolCall {
            name: name.to_string(),
            call_id: id.to_string(),
            arguments: args,
        }
    }

    #[test]
    fn overwrite_keeps_one_entry_per_key_and_caps_capacity() {
        let mut audit = MechanicalAuditState::new();
        for round in 1..=MECHANICAL_AUDIT_CAPACITY + 10 {
            audit.record(
                format!("file:f{round:03}.txt"),
                round as u32,
                "search_replace 成功",
                None,
            );
        }
        assert_eq!(audit.entry_count(), MECHANICAL_AUDIT_CAPACITY);
        // 超限丢最旧首见键：f001..f010 已淘汰，f011 是当前最旧。
        let report = audit.report(None);
        assert!(!report.contains("file:f001.txt"));
        assert!(report.contains("file:f011.txt"));
        // 同键覆盖：不新增条目、保留最新轮。
        audit.record("file:f011.txt", 999, "search_replace 成功（第二轮）", None);
        assert_eq!(audit.entry_count(), MECHANICAL_AUDIT_CAPACITY);
        let report = audit.report(None);
        assert!(report.contains("file:f011.txt → search_replace 成功（第二轮）"));
        // 再写一条 → 最旧首见键（f011，覆盖后仍在首见序最旧位）淘汰；
        // 其余键保留各自最新结果。
        audit.record("file:extra.txt", 1000, "x", None);
        let report = audit.report(None);
        assert!(!report.contains("file:f011.txt"));
        assert!(report.contains("file:f012.txt"));
        assert!(report.contains("file:extra.txt"));
    }

    #[test]
    fn report_contains_only_exec_wallclock_and_anomalies() {
        let mut audit = MechanicalAuditState::new();
        audit.record("file:a.py", 1, "search_replace 成功（+1 文件）", None);
        audit.record(
            "cmd:call-2",
            2,
            "exit 1，超时",
            Some("exit 1（120s 超时）".to_string()),
        );
        // step/契约类只事件留痕、不上报告：记录到表中但报告不含。
        audit.record("step:order", 2, "step 顺序事件", None);
        let report = audit.report(Some(std::time::Duration::from_secs(90)));
        assert!(report.contains("[MECHANICAL_AUDIT v0.1]"));
        assert!(report.contains("file:a.py → search_replace 成功（+1 文件）"));
        assert!(report.contains("cmd:call-2 → exit 1，超时"));
        assert!(report.contains("墙钟约 90s"));
        assert!(report.contains("cmd:call-2 → exit 1（120s 超时）"));
        // 0cn S2（ADR-0010 §14.82）：预算行（轮数记次）退役——报告不得再
        // 渲染「预算：」行与「已用 N/M 轮」记次。
        assert!(!report.contains("预算"), "预算行已退役: {report}");
        assert!(!report.contains("已用"), "轮数记次已退役: {report}");
        assert!(!report.contains("step:order"), "step/契约类不得进报告");
        assert!(!report.contains("建议"), "报告不得含建议");
        // 无墙钟（None）时不渲染墙钟行。
        let report_no_wallclock = audit.report(None);
        assert!(!report_no_wallclock.contains("墙钟约"));
    }

    #[test]
    fn search_replace_records_anchor_anomaly() {
        let tc = call(
            "search_replace",
            "call-1",
            serde_json::json!({"file_path": "app/result.txt"}),
        );
        let r = ToolResult {
            output: "".to_string(),
            exit_code: Some(1),
            structured: Some(serde_json::json!({"error": "content_anchor_mismatch"})),
            ..Default::default()
        };
        let (key, summary, anomaly) = classify(&tc, &r, None);
        assert_eq!(key.as_deref(), Some("file:app/result.txt"));
        assert!(summary.contains("未应用"));
        assert_eq!(anomaly.as_deref(), Some("content_anchor_mismatch"));
    }

    #[test]
    fn terminal_records_timeout_anomaly() {
        let tc = call("run_terminal_cmd", "call-9", serde_json::json!({}));
        let r = ToolResult {
            output: "".to_string(),
            exit_code: None,
            timed_out: true,
            ..Default::default()
        };
        let (key, summary, anomaly) = classify(&tc, &r, None);
        assert_eq!(key.as_deref(), Some("cmd:call-9"));
        assert!(summary.contains("超时"));
        assert_eq!(anomaly.as_deref(), Some("超时"));

        // stdout 截断机械标记（grok_build/bash 输出脚注 `[truncated:`）→
        // 摘要单列"输出截断"，不再以文件 delta 顶替。
        let r2 = ToolResult {
            output: "head...\n[truncated: showing first/last 8000 of 20000]\n".to_string(),
            exit_code: Some(0),
            ..Default::default()
        };
        let (_, summary2, anomaly2) = classify(&tc, &r2, None);
        assert!(summary2.contains("输出截断"), "{summary2}");
        assert_eq!(anomaly2, None);
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2 审查处理 P1-2):
    /// 中间回报（命令仍在后台运行，exit_code=None + mid_run）不得记为
    /// 异常事实——摘要为中性「运行中」，anomaly=None。
    #[test]
    fn terminal_mid_run_is_not_anomaly() {
        let tc = call("run_terminal_cmd", "call-mid", serde_json::json!({}));
        let r = ToolResult {
            output: "[Command still running after 300s] PID: 1234 ...".to_string(),
            exit_code: None,
            timed_out: false,
            mid_run: Some(crate::host::ToolMidRunStatus {
                task_id: "call-mid".to_string(),
                pid: Some(1234),
                output_file: "/tmp/terminal/call-mid.log".to_string(),
                total_bytes: Some(8192),
            }),
            ..Default::default()
        };
        let (key, summary, anomaly) = classify(&tc, &r, None);
        assert_eq!(key.as_deref(), Some("cmd:call-mid"));
        assert!(summary.contains("运行中"), "{summary}");
        assert_eq!(anomaly, None, "mid-run must not be an audit anomaly");
    }

    #[test]
    fn retrieval_records_candidate_count_and_overlimit_anomaly() {
        // 正常派发：候选计数来自激活（候选/上限）。
        let tc = call("web_search", "call-w1", serde_json::json!({"query": "x"}));
        let r = ToolResult {
            output: "".to_string(),
            exit_code: Some(0),
            ..Default::default()
        };
        let (key, summary, anomaly) = classify(&tc, &r, Some((3, 8)));
        assert_eq!(key.as_deref(), Some("retrieval"));
        assert_eq!(summary, "候选 3/8");
        assert_eq!(anomaly, None);

        // 候选超限：计数达到上限 → 异常事实「候选超限」。
        let r2 = ToolResult {
            output: "".to_string(),
            exit_code: Some(1),
            ..Default::default()
        };
        let (_, summary2, anomaly2) = classify(&tc, &r2, Some((8, 8)));
        assert_eq!(summary2, "候选 8/8");
        assert_eq!(anomaly2.as_deref(), Some("候选超限（候选 8/8）"));

        // 结构化错误码优先：候选门拒绝信封携带
        // `web_fetch_candidate_cap_exceeded` → 精确"候选超限"，不依赖
        // 剩余池 ≥ cap 近似。
        let r3 = ToolResult {
            output: "".to_string(),
            exit_code: Some(1),
            structured: Some(serde_json::json!({
                "error": "web_fetch_candidate_cap_exceeded",
                "candidate_count": 8,
                "candidate_cap": 8,
            })),
            ..Default::default()
        };
        let (_, summary3, anomaly3) = classify(&tc, &r3, None);
        assert_eq!(summary3, "候选 8/8");
        assert_eq!(anomaly3.as_deref(), Some("候选超限（候选 8/8）"));

        // 计数域未绑定 → 独立异常事实。
        let r4 = ToolResult {
            output: "".to_string(),
            exit_code: Some(1),
            structured: Some(serde_json::json!({
                "error": "browser_read_candidate_count_unbound",
            })),
            ..Default::default()
        };
        let (_, _, anomaly4) = classify(&tc, &r4, None);
        assert_eq!(anomaly4.as_deref(), Some("候选计数域未绑定（候选 n/a）"));

        // 检索派发序号由 record_retrieval 分配（run 内单调）。
        let mut audit = MechanicalAuditState::new();
        // 无异常的检索派发不进执行事实（执行事实=file:/cmd: 键）。
        audit.record_retrieval(1, "候选 1/8", None);
        audit.record_retrieval(2, "候选 2/8", None);
        let report = audit.report(None);
        assert!(
            !report.contains("retrieval:"),
            "无异常检索不上报告：{report}"
        );
        // 带异常的检索派发进异常事实。
        audit.record_retrieval(3, "候选 8/8", Some("候选超限（候选 8/8）".to_string()));
        let report = audit.report(None);
        assert!(
            report.contains("retrieval:3 → 候选超限（候选 8/8）"),
            "{report}"
        );
    }

    // ── MECHANICAL-AUDIT-LAYER S2 (2026-08-24, ADR-0010 §14.39 / 设计 §5) ──

    /// 设计 §2.4/§5 验收 4：终答前反例自查轮同轮注入 [MECHANICAL_AUDIT
    /// v0.1] 独立块——报告收敛为执行事实摘要（动作/文件 delta/墙钟/异常
    /// 事实；0cn S2 起预算行退役），无建议；journal 以
    /// `mechanical_audit_update` 轻量事件留痕（plan_gate + tool_result）。
    /// **0cn S2 零写入钉子（ADR-0010 §14.82）**：budget 轮数记次注入撤除
    /// ⇒ 本 run 不得出现 `kind=budget` 事件（枚举值仅供历史 journal 校验）。
    #[tokio::test]
    async fn mechanical_audit_report_injected_with_final_answer_gate() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "exit 1".to_string(),
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "run_terminal_cmd".to_string(),
                arguments: serde_json::json!({ "command": "dir" }),
                call_id: "call-term-1".to_string(),
            }]),
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = stage_c_controller(gateway);
        let (response, _, _) = controller
            .run_turn(
                &host,
                "运行命令",
                "RUN-AUDIT",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(response, "终答");

        // 终答前中立问询轮同时携带 [MECHANICAL_AUDIT v0.1] 与
        // [COUNTEREXAMPLE_GATE]（同轮独立块）。
        let requests = fake.received_requests();
        let gate_round = requests
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.content.contains("[COUNTEREXAMPLE_GATE v0.1]"))
            })
            .expect("counterexample gate round");
        let audit_msg = gate_round
            .messages
            .iter()
            .find(|m| m.content.contains("[MECHANICAL_AUDIT v0.1]"))
            .expect("audit report injected in the same round");
        assert!(audit_msg.content.contains("执行事实"), "{audit_msg:?}");
        assert!(audit_msg.content.contains("墙钟约"), "{audit_msg:?}");
        assert!(audit_msg.content.contains("异常事实"), "{audit_msg:?}");
        // 0cn S2：预算行（轮数记次）退役——报告块不得再出现。
        assert!(
            !audit_msg.content.contains("预算") && !audit_msg.content.contains("已用"),
            "budget line retired (0cn S2): {audit_msg:?}"
        );
        assert!(
            audit_msg.content.contains("cmd:call-term-1"),
            "the cmd object key carries the latest result: {audit_msg:?}"
        );
        assert!(
            !audit_msg.content.contains("建议"),
            "audit report must never carry advice: {audit_msg:?}"
        );

        // journal 留痕：plan_gate / tool_result 两类机械审查事件；
        // budget 退役 ⇒ 零写入钉子（历史 journal 回放仍合法）。
        let events = events(&dir);
        let audit_events: Vec<&RunEvent> = events
            .iter()
            .filter(|e| e.event_type == EventType::MechanicalAuditUpdate)
            .collect();
        assert!(
            audit_events
                .iter()
                .any(|e| e.payload["kind"] == KIND_PLAN_GATE),
            "plan gate entry journaled: {audit_events:?}"
        );
        assert!(
            !audit_events
                .iter()
                .any(|e| e.payload["kind"] == KIND_BUDGET),
            "budget 轮数记次已撤除（0cn S2），本 run 不得出现: {audit_events:?}"
        );
        assert!(
            audit_events.iter().any(|e| {
                e.payload["kind"] == KIND_TOOL_RESULT
                    && e.payload["payload"]["key"] == "cmd:call-term-1"
            }),
            "tool_result entry journaled: {audit_events:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 0cp S2（D4，2026-10-03，ADR-0010 §14.83）──────────────────────

    /// **0cp D4 判据 4 钉**（设计 §5）：RLI fire → 模型面出现附注行＋**同刻
    /// journal `mechanical_audit_update{kind:rli_notice}` 行**（盲区闭环：
    /// fire 全集可由 journal 复算）；`delivered` 与附注装配数一致。连续 3
    /// 轮失败命令推 err 通道凑满 k=3（0cp D3；决策轮＋工具事件各为一动作
    /// 样），第 3 轮工具批回传附「RLI提醒: 持续越线…」行。
    #[tokio::test]
    async fn rli_notice_direct_push_attaches_line_and_journals() {
        use crate::gateway::model::Role;
        use orz_assurance::lif::{RLI_NOTICE_TEXT_BUDGET, RLI_T_HAT_ANNOTATION};
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            // timed_out ⇒ ToolOutcome::Error（err 通道 +1；exit 1 是 D2 值
            // 语义＝Other 不注 err——tool_run 分类口径）。
            tool_result: Some(ToolResult {
                output: "timeout".to_string(),
                exit_code: None,
                timed_out: true,
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        // plan 轮（首轮计划门）→ 三轮失败命令（streak 第 3 个动作样凑满）
        // → 终答。
        let mut scripts = vec![ScriptedResponse::tool_calls(vec![plan_write_call(
            "call-plan-1",
            valid_plan_json(),
        )])];
        for i in 0..3u32 {
            scripts.push(ScriptedResponse::tool_calls(vec![ToolCall {
                name: "run_terminal_cmd".to_string(),
                arguments: serde_json::json!({ "command": format!("fail-{i}") }),
                call_id: format!("call-term-{i}"),
            }]));
        }
        scripts.push(ScriptedResponse::text("草稿"));
        scripts.push(ScriptedResponse::text("终答"));
        let fake = Arc::new(FakeProvider::new(scripts));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = stage_c_controller(gateway);
        let (response, _, _) = controller
            .run_turn(
                &host,
                "运行命令",
                "RUN-RLI-PUSH",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(response, "终答");

        // 模型面：第 3 轮失败命令的回传附注行（含 D6 T̂ 注解；无周期注入）
        // ——历史重放同一份内容，末请求内恰一次出现（不重复注入）。
        let requests = fake.received_requests();
        let last = requests.last().expect("requests captured");
        let pushed: Vec<&crate::gateway::model::Message> = last
            .messages
            .iter()
            .filter(|m| m.role == Role::Tool && m.content.contains("RLI提醒: 持续越线"))
            .collect();
        assert_eq!(pushed.len(), 1, "恰一轮回传携带附注行");
        let line_start = pushed[0].content.find("RLI提醒:").unwrap();
        let line = &pushed[0].content[line_start..];
        assert!(line.contains(RLI_T_HAT_ANNOTATION), "D6 注解在行内: {line}");
        assert!(
            line.len() <= RLI_NOTICE_TEXT_BUDGET + "\n".len(),
            "行宽 ≤320B（0am P8-b 重订）纪律: {line}"
        );

        // journal：kind=rli_notice、key=rli.notice.streak_crossed、盲区闭环。
        let events = events(&dir);
        let rli_events: Vec<&RunEvent> = events
            .iter()
            .filter(|e| e.event_type == EventType::MechanicalAuditUpdate)
            .filter(|e| e.payload["kind"] == KIND_RLI_NOTICE)
            .collect();
        assert_eq!(rli_events.len(), 1, "恰一条 rli_notice journal 行");
        assert_eq!(
            rli_events[0].payload["payload"]["key"],
            "rli.notice.streak_crossed"
        );
        assert!(
            rli_events[0].payload["payload"]["summary"]
                .as_str()
                .unwrap()
                .contains("持续越线"),
            "{:?}",
            rli_events[0].payload
        );
        // delivered 与附注装配数一致（D4 投递语义）。
        let (delivered, _deferred, _headroom) = controller
            .lif
            .lock()
            .unwrap()
            .rli_shadow()
            .expect("shadow present")
            .notice_delivery_stats();
        assert_eq!(delivered, 1, "投递率分子＝附注装配数");
        assert!(controller.take_pending_rli_notices().is_empty(), "不重发");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 0ag 同型契约钉子（2026-09-15，0AE-C2 修复）─────────────────────
    //
    // runtime schema `mechanical-audit-update-event-payload-v0.2` 的 kind
    // 闭枚举与上方实现常量**逐字全等**（顺序即钉子断言序）；payload 的
    // required 键集同步互证。实现侧新增/改名 kind 而忘记同步 schema 时
    // 此钉变红（0AE-C2：0ae 批新写三种越界 kind，首次 128K 阶梯触发即
    // 会判 journal invalid）。

    #[test]
    fn mechanical_audit_update_schema_matches_kind_constants() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../runtime/mechanical-audit-update-event-payload-v0.2.schema.json"
        );
        let raw = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("runtime schema readable: {path}: {e}"));
        let schema: serde_json::Value = serde_json::from_str(&raw).expect("schema json");
        let kind_enum: Vec<String> = schema
            .pointer("/properties/kind/enum")
            .and_then(|v| v.as_array())
            .expect("kind enum present")
            .iter()
            .map(|v| v.as_str().expect("enum string").to_string())
            .collect();
        assert_eq!(
            kind_enum,
            vec![
                KIND_TOOL_RESULT.to_string(),
                KIND_PLAN_GATE.to_string(),
                KIND_BUDGET.to_string(),
                KIND_ATTENTION_LADDER.to_string(),
                KIND_CONTEXT_SCALE.to_string(),
                KIND_MODEL_COMPRESSION.to_string(),
                KIND_PLAN_WRITE_GUIDANCE.to_string(),
                KIND_RETRIEVAL_BATCH.to_string(),
                KIND_LIF_DOMAIN.to_string(),
                KIND_RLI_NOTICE.to_string(),
            ]
        );
        // 0av S1：schema 按 kind 条件分支校验 payload——retrieval_batch 走
        // 五键批读数形状，其余 kind 走 {key,round,summary,anomaly} 形状；
        // 两分支键集与本断言逐字互证（新增/改名 kind 忘同步 schema 时本钉
        // 变红）。
        let batch_required: Vec<String> = schema
            .pointer("/allOf/0/then/properties/payload/required")
            .and_then(|v| v.as_array())
            .expect("retrieval_batch payload branch present")
            .iter()
            .map(|v| v.as_str().expect("required key").to_string())
            .collect();
        assert_eq!(
            batch_required,
            vec![
                "activation_id".to_string(),
                "usable".to_string(),
                "cap".to_string(),
                "retrieval_calls".to_string(),
                "terminal_reason".to_string(),
            ]
        );
        let uniform_required: Vec<String> = schema
            .pointer("/allOf/1/then/properties/payload/required")
            .and_then(|v| v.as_array())
            .expect("uniform payload branch present")
            .iter()
            .map(|v| v.as_str().expect("required key").to_string())
            .collect();
        assert_eq!(
            uniform_required,
            vec![
                "key".to_string(),
                "round".to_string(),
                "summary".to_string(),
                "anomaly".to_string(),
            ]
        );
        // 2026-09-20 审查修复批：分支**触发面**互证——allOf/0 的 if 恰为
        // retrieval_batch、allOf/1 的 if 恰为其余 7 个 kind。此前只互证
        // required 键集，不校验分支触发条件：新增 kind 漏同步分支时其
        // payload 形状会静默失去约束（本钉堵住该缺口）。
        let batch_if: Vec<String> = schema
            .pointer("/allOf/0/if/properties/kind/const")
            .and_then(|v| v.as_str())
            .map(|s| vec![s.to_string()])
            .expect("batch branch if-const present");
        assert_eq!(batch_if, vec![KIND_RETRIEVAL_BATCH.to_string()]);
        let uniform_if: Vec<String> = schema
            .pointer("/allOf/1/if/properties/kind/enum")
            .and_then(|v| v.as_array())
            .expect("uniform branch if-enum present")
            .iter()
            .map(|v| v.as_str().expect("enum string").to_string())
            .collect();
        assert_eq!(
            uniform_if,
            vec![
                KIND_TOOL_RESULT.to_string(),
                KIND_PLAN_GATE.to_string(),
                KIND_BUDGET.to_string(),
                KIND_ATTENTION_LADDER.to_string(),
                KIND_CONTEXT_SCALE.to_string(),
                KIND_MODEL_COMPRESSION.to_string(),
                KIND_PLAN_WRITE_GUIDANCE.to_string(),
                KIND_LIF_DOMAIN.to_string(),
                KIND_RLI_NOTICE.to_string(),
            ]
        );
        // 分支触发面与 kind 枚举互补且不交——两分支并集恰为全枚举。
        let mut covered = uniform_if.clone();
        covered.push(KIND_RETRIEVAL_BATCH.to_string());
        let mut sorted_covered = covered.clone();
        sorted_covered.sort();
        let mut sorted_enum = kind_enum.clone();
        sorted_enum.sort();
        assert_eq!(
            sorted_covered, sorted_enum,
            "the two branch triggers must exactly partition the kind enum"
        );
    }
}
