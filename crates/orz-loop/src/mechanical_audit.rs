//! MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / BACKLOG 0g /
//! TODO P0-0g): 静默机械审查层——运行内审查表，以对象键为单位记录执行
//! 事实；每对象键仅保留最后一轮结果（覆盖写）；不给建议、不注入运行中
//! 反馈；最终答案前中立问询轮随 [COUNTEREXAMPLE_GATE] 同轮以独立块
//! `[MECHANICAL_AUDIT v0.1]` 注入报告。报告收敛为执行事实摘要（仅三类：
//! 执行事实 / 预算 / 异常事实）；step/契约类只事件留痕、不上报告。
//!
//! 对象键（设计 §2.4）：
//! - `file:<path>`：search_replace 结果（matched/created、diff 规模）；
//! - `cmd:<call_id>`：run_terminal_cmd 结果（exit code、timeout、stdout
//!   截断）；
//! - `plan`：首轮计划门结果（accepted/degraded、步骤数、最近修订）；
//! - `budget`：轮数 / 墙钟用量；
//! - `retrieval:<n>`：检索派发（候选 / 上限）。

use orz_assurance::EventType;

use crate::controller::EventWriter;
use crate::gateway::model::ToolCall;
use crate::host::ToolResult;

/// 报告块前缀——注册进 `prompt::is_injected_block_text`（机械注入文本，
/// 绝不持久化回会话）。
pub(crate) const MECHANICAL_AUDIT_PREFIX: &str = "[MECHANICAL_AUDIT";

/// 审查表容量上限（超限丢最旧键；每键保留最新）。
pub(crate) const MECHANICAL_AUDIT_CAPACITY: usize = 128;

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

    /// 记录/覆盖预算键（每轮末调用；覆盖写）。
    pub(crate) fn record_budget(
        &mut self,
        round: u32,
        rounds_used: u32,
        max_rounds: u32,
    ) -> serde_json::Value {
        self.record(
            "budget",
            round,
            format!("已用 {rounds_used}/{max_rounds} 轮"),
            None,
        )
    }

    /// 报告块（设计 §2.4：收敛为执行事实摘要——仅三类；每键至多一条；
    /// 无建议、无引导）。`wallclock` 为 run 已用墙钟。
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
        let budget_line = self
            .entries
            .iter()
            .find(|e| e.key == "budget")
            .map(|e| e.summary.clone())
            .unwrap_or_else(|| "预算：暂无".to_string());
        lines.push(match wallclock {
            Some(d) => format!("预算：{}，墙钟约 {}s", budget_line, d.as_secs()),
            None => format!("预算：{}", budget_line),
        });
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
                "kind": "tool_result",
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
    use crate::gateway::model::ToolCall;
    use crate::host::ToolResult;

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
    fn report_contains_only_exec_budget_and_anomalies() {
        let mut audit = MechanicalAuditState::new();
        audit.record("file:a.py", 1, "search_replace 成功（+1 文件）", None);
        audit.record(
            "cmd:call-2",
            2,
            "exit 1，超时",
            Some("exit 1（120s 超时）".to_string()),
        );
        audit.record("budget", 2, "已用 5/999 轮", None);
        // step/契约类只事件留痕、不上报告：记录到表中但报告不含。
        audit.record("step:order", 2, "step 顺序事件", None);
        let report = audit.report(Some(std::time::Duration::from_secs(90)));
        assert!(report.contains("[MECHANICAL_AUDIT v0.1]"));
        assert!(report.contains("file:a.py → search_replace 成功（+1 文件）"));
        assert!(report.contains("cmd:call-2 → exit 1，超时"));
        assert!(report.contains("已用 5/999 轮"));
        assert!(report.contains("墙钟约 90s"));
        assert!(report.contains("cmd:call-2 → exit 1（120s 超时）"));
        assert!(!report.contains("step:order"), "step/契约类不得进报告");
        assert!(!report.contains("建议"), "报告不得含建议");
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
}
