//! 358（2026-10-26）：只读 scout 子代理的结果成形。
//!
//! 设计：`docs/SUBAGENT_SCOUT_DESIGN_2026-10-26.md` §3.2。scout 在自己的
//! 窗口里扫文件，回来只给一张 ≤2K 的 `findings[]` 表（每条带 `path:line`
//! 证据与置信度）；全文落 `scout-report/<report_id>.md`，主模型按路径
//! 用 `read_file` 取，不把全文塞进主 run 上下文。

use serde::{Deserialize, Serialize};

use crate::gateway::model::{Message, Role};

/// 模型面摘要硬上限（设计 §3.2：正文 ≤2K 字符）。
pub const SCOUT_SUMMARY_MAX_CHARS: usize = 2000;
/// 解析失败时回退摘要的截断长度（设计 §3.2：summary ≤400 字）。
pub const SCOUT_FALLBACK_SUMMARY_MAX_CHARS: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScoutStatus {
    Ok,
    Partial,
    Failed,
}

impl ScoutStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScoutStatus::Ok => "ok",
            ScoutStatus::Partial => "partial",
            ScoutStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScoutConfidence {
    High,
    Mid,
    Low,
}

impl ScoutConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScoutConfidence::High => "high",
            ScoutConfidence::Mid => "mid",
            ScoutConfidence::Low => "low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutFinding {
    pub claim: String,
    /// `path:line`——设计 §3.2 的硬证据面。
    pub evidence: String,
    pub confidence: ScoutConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutStats {
    pub files_read: u64,
    pub tokens_used: u64,
    pub wall_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutReport {
    pub report_id: String,
    pub status: ScoutStatus,
    pub summary: String,
    pub findings: Vec<ScoutFinding>,
    pub stats: ScoutStats,
    /// 358 修复（审查 P1）：失败时给结构化错误类别与最后一步，而不是只
    /// 塞进 summary 文本（设计 §3.3）。
    #[serde(default)]
    pub error: Option<ScoutError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutError {
    /// `timeout` / `agent_loop` / `parse` / `concurrency`。
    pub class: String,
    pub message: String,
    /// 最后执行的工具名（无则 None）。
    pub last_step: Option<String>,
}

impl ScoutReport {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// `spawn_scout` 的任务契约（`task` / `scope.paths` / `budget`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScoutTask {
    pub task: String,
    pub paths: Vec<String>,
    /// `scope.tools`——只读工具子集（投影后再收窄；空 = 投影全量）。
    pub tools: Vec<String>,
    pub max_files: Option<u64>,
    pub max_tokens: Option<u64>,
    pub max_wall_s: Option<u64>,
}

impl ScoutTask {
    /// 从 `spawn_scout` 的工具参数解析；缺失 `task` 时回退整段参数。
    pub fn from_arguments(arguments: &serde_json::Value) -> Self {
        let task = arguments
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let paths = arguments
            .get("scope")
            .and_then(|v| v.get("paths"))
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let budget = arguments.get("budget");
        let budget_u64 = |key: &str| {
            budget
                .and_then(|b| b.get(key))
                .and_then(|v| v.as_u64())
                .filter(|v| *v > 0)
        };
        let tools = arguments
            .get("scope")
            .and_then(|v| v.get("tools"))
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            task,
            paths,
            tools,
            max_files: budget_u64("max_files"),
            max_tokens: budget_u64("max_tokens"),
            max_wall_s: budget_u64("max_wall_s"),
        }
    }

    /// 交给 scout 子代理的任务目标（IPG 输入，也是逐轮 prompt 的 goal）。
    pub fn render_goal(&self) -> String {
        let mut goal = if self.task.trim().is_empty() {
            "Survey the scoped paths and return findings with path:line evidence.".to_string()
        } else {
            self.task.clone()
        };
        if !self.paths.is_empty() {
            goal.push_str(
                "\n\nScope (advisory paths — the read face is open, not a hard boundary):",
            );
            for path in &self.paths {
                goal.push_str("\n- ");
                goal.push_str(path);
            }
        }
        if !self.tools.is_empty() {
            goal.push_str("\n\nAllowed tools (subset of the read-only face):");
            for tool in &self.tools {
                goal.push_str("\n- ");
                goal.push_str(tool);
            }
        }
        if self.max_files.is_some() || self.max_tokens.is_some() || self.max_wall_s.is_some() {
            goal.push_str(&format!(
                "\n\nBudget: max_files={} max_tokens={} max_wall_s={} (exceeding any cap returns partial findings).",
                self.max_files.map_or("none".to_string(), |v| v.to_string()),
                self.max_tokens.map_or("none".to_string(), |v| v.to_string()),
                self.max_wall_s.map_or("none".to_string(), |v| v.to_string()),
            ));
        }
        goal
    }
}

/// 解析 scout 的模型输出：优先 ```json 块，其次首个 `{...}`；失败返回 None。
pub fn parse_scout_payload(text: &str) -> Option<(String, Vec<ScoutFinding>)> {
    let candidates = json_candidates(text);
    for candidate in candidates {
        if let Ok(raw) = serde_json::from_str::<RawScoutPayload>(&candidate) {
            let findings = raw
                .findings
                .into_iter()
                .filter_map(|f| {
                    let evidence = f.evidence.trim().to_string();
                    if evidence.is_empty() || !evidence.contains(':') {
                        return None;
                    }
                    let claim = f.claim.trim().to_string();
                    if claim.is_empty() {
                        return None;
                    }
                    Some(ScoutFinding {
                        claim,
                        evidence,
                        confidence: parse_confidence(f.confidence.as_deref()),
                    })
                })
                .collect();
            let summary = raw.summary.trim().to_string();
            return Some((summary, findings));
        }
    }
    None
}

#[derive(Debug, Deserialize)]
struct RawScoutPayload {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    findings: Vec<RawScoutFinding>,
}

#[derive(Debug, Deserialize)]
struct RawScoutFinding {
    #[serde(default)]
    claim: String,
    #[serde(default)]
    evidence: String,
    #[serde(default)]
    confidence: Option<String>,
}

fn parse_confidence(value: Option<&str>) -> ScoutConfidence {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "high" => ScoutConfidence::High,
        "mid" | "medium" => ScoutConfidence::Mid,
        _ => ScoutConfidence::Low,
    }
}

/// 提取候选 JSON：```json 围栏优先，其次是首个 `{` 到最后 `}` 的整段。
fn json_candidates(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(start) = text.find("```json") {
        let rest = &text[start + "```json".len()..];
        if let Some(end) = rest.find("```") {
            out.push(rest[..end].trim().to_string());
        }
    }
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                out.push(text[start..=end].trim().to_string());
            }
        }
    }
    out
}

/// 解析失败时的回退：全文压成 ≤400 字摘要，findings 为空。
pub fn fallback_summary(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(&compact, SCOUT_FALLBACK_SUMMARY_MAX_CHARS)
}

/// 模型面摘要（≤2K）：report 头 + findings 索引行 + summary + findings 表
/// + 全文路径。索引行供主模型按设计 §4/§7 代写 findings 分区。
pub fn render_scout_summary(report: &ScoutReport, archive_path: Option<&str>) -> String {
    let index_summary = truncate_chars(&report.summary, 60);
    let mut out = format!(
        "[SCOUT {}] status={} files={} tokens={} wall={}s\nindex: {} | {} files | {} findings | {}\nsummary: {}\nfindings ({}):",
        report.report_id,
        report.status.as_str(),
        report.stats.files_read,
        report.stats.tokens_used,
        report.stats.wall_s,
        report.report_id,
        report.stats.files_read,
        report.findings.len(),
        index_summary,
        report.summary,
        report.findings.len(),
    );
    if let Some(error) = &report.error {
        let line = format!(
            "\nerror: {} | {} | last_step={}",
            error.class,
            error.message,
            error.last_step.as_deref().unwrap_or("none")
        );
        if out.chars().count() + line.chars().count() <= SCOUT_SUMMARY_MAX_CHARS {
            out.push_str(&line);
        }
    }
    for finding in &report.findings {
        let line = format!(
            "\n- {} | {} | {}",
            finding.claim,
            finding.evidence,
            finding.confidence.as_str()
        );
        if out.chars().count() + line.chars().count() > SCOUT_SUMMARY_MAX_CHARS {
            out.push_str("\n…(summary truncated; see full report)");
            break;
        }
        out.push_str(&line);
    }
    if let Some(path) = archive_path {
        let tail = format!("\nfull report: {path}");
        if out.chars().count() + tail.chars().count() <= SCOUT_SUMMARY_MAX_CHARS {
            out.push_str(&tail);
        }
    }
    let hint =
        "\n(findings index line available above; write it to the findings partition if useful)";
    if out.chars().count() + hint.chars().count() <= SCOUT_SUMMARY_MAX_CHARS {
        out.push_str(hint);
    }
    truncate_chars(&out, SCOUT_SUMMARY_MAX_CHARS)
}

/// 全文落盘形态（Markdown）：报告头 + findings 表 + scout 原始输出。
pub fn render_scout_report_markdown(report: &ScoutReport, raw_output: &str) -> String {
    let mut out = format!(
        "# Scout report {}\n\n- status: {}\n- files_read: {}\n- tokens_used: {}\n- wall_s: {}\n\n## Summary\n\n{}\n\n## Findings\n\n",
        report.report_id,
        report.status.as_str(),
        report.stats.files_read,
        report.stats.tokens_used,
        report.stats.wall_s,
        report.summary,
    );
    if report.findings.is_empty() {
        out.push_str("(none)\n");
    } else {
        out.push_str("| claim | evidence | confidence |\n|---|---|---|\n");
        for finding in &report.findings {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                finding.claim.replace('|', "\\|"),
                finding.evidence.replace('|', "\\|"),
                finding.confidence.as_str()
            ));
        }
    }
    if let Some(error) = &report.error {
        out.push_str(&format!(
            "\n## Error\n\n- class: {}\n- message: {}\n- last_step: {}\n",
            error.class,
            error.message,
            error.last_step.as_deref().unwrap_or("none")
        ));
    }
    out.push_str("\n## Raw scout output\n\n```text\n");
    out.push_str(raw_output);
    out.push_str("\n```\n");
    out
}

/// 358 修复（审查 P1）：超时后从子会话里捞回最后一段 assistant 文本，
/// 有可解析 findings 就降级 `partial`（设计 §3.3 的预算超限口径）。
fn last_assistant_text(conversation: &[Message]) -> String {
    conversation
        .iter()
        .rev()
        .find(|m| m.role == Role::Assistant && !m.content.trim().is_empty())
        .map(|m| m.content.clone())
        .unwrap_or_default()
}

/// 358 修复（审查 P2）：失败报告里的“最后一步”。
fn last_tool_name(conversation: &[Message]) -> Option<String> {
    conversation
        .iter()
        .rev()
        .flat_map(|m| m.tool_calls.iter().rev())
        .next()
        .map(|c| c.name.clone())
}

/// 统计 scout 会话里读过的不同文件数（`read_file` 路径 + `grep` 的文件
/// 路径；`grep` 的目录路径不计入）。
pub fn count_files_read(conversation: &[Message]) -> u64 {
    let mut paths = std::collections::BTreeSet::new();
    let mut calls = 0u64;
    for message in conversation {
        if message.role != Role::Assistant {
            continue;
        }
        for call in &message.tool_calls {
            let (keys, files_only): (&[&str], bool) = match call.name.as_str() {
                "read_file" => (&["path", "file_path", "target_file"], false),
                "grep" => (&["path"], true),
                _ => continue,
            };
            calls += 1;
            for key in keys {
                if let Some(path) = call.arguments.get(*key).and_then(|v| v.as_str()) {
                    if path.is_empty() {
                        continue;
                    }
                    if files_only && std::path::Path::new(path).extension().is_none() {
                        continue;
                    }
                    paths.insert(path.to_string());
                }
            }
        }
    }
    if paths.is_empty() {
        calls
    } else {
        paths.len() as u64
    }
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

impl crate::controller::AgentLoopController {
    /// 358b（2026-10-26）：`spawn_scout` 的独立执行路径——不建 activation、
    /// 不写 evidence / 黑板分区；scout 只读侦察，结果由 [`ScoutReport`]
    /// 成形并落 `scout-report/<id>.md`。
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn run_scout_subagent(
        &self,
        host: &dyn crate::host::LoopHost,
        writer: &mut crate::controller::EventWriter<'_>,
        run_gateway: &std::sync::Arc<dyn crate::gateway::model::ModelGateway>,
        tc: &crate::gateway::model::ToolCall,
        messages: &mut Vec<crate::gateway::model::Message>,
        tool_defs: &[crate::host::ToolDef],
        orientation: Option<&mut crate::orientation::OrientationSessionState>,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
    ) -> Result<crate::host::ToolResult, crate::controller::AgentLoopError> {
        use std::time::{Duration, Instant};

        use crate::agent_loop::{LoopProfile, SharedLoopServices, run_agent_loop};
        use crate::agents::{RetrievalSubagent, SubagentRole};
        use crate::gateway::model::{Message, Role};
        use crate::host::ToolResult;

        // 358c：同 run 并发上限（默认 2）——超出排队；permit 持到函数结束。
        let _scout_permit = match self.scout_semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                let msg = "scout unavailable: concurrency gate closed".to_string();
                writer
                    .record(
                        orz_assurance::EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "target": "scout",
                            "agent_role": "scout",
                            "status": "error",
                            "error": "scout_concurrency_closed",
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                return Ok(crate::host::ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    ..Default::default()
                });
            }
        };

        let task = ScoutTask::from_arguments(&tc.arguments);
        let goal = task.render_goal();
        let session_id = orientation
            .as_ref()
            .map(|o| o.session_id.clone())
            .unwrap_or_else(|| writer.run_id().to_string());
        let session8: String = session_id.chars().take(8).collect();
        let seq = self
            .next_scout_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let report_id = format!("SC-{:04}", seq + 1);
        let child_run_id = format!("scout-{session8}-{:04}", seq + 1);
        let activation_id = child_run_id.clone();
        // 358 修复（审查 P0）：与 ToolCompleted 配对——Scout 早退发生在
        // `run_retrieval_subagent` 的 ToolStarted 之前，必须在这里补。
        writer
            .record(
                orz_assurance::EventType::ToolStarted,
                serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "target": "scout",
                    "agent_role": "scout",
                    "child_run_id": child_run_id,
                    "parent_run_id": writer.run_id(),
                }),
            )
            .await?;
        let mut sub_tool_defs = Self::subagent_tool_projection(
            tool_defs,
            host.tools_registry(),
            SubagentRole::Scout,
            self.retrieval_enabled,
        );
        if !task.tools.is_empty() {
            sub_tool_defs.retain(|d| task.tools.iter().any(|t| t == &d.name));
        }
        let subagent = RetrievalSubagent::new(SubagentRole::Scout, run_gateway.clone());
        // 358 修复（审查 P1）：与主车道轮数上限取 min（主车道 0=unlimited
        // 不截断；与检索子代理同纪律）。
        let main_cap = (self.max_tool_rounds != 0).then_some(self.max_tool_rounds);
        let scout_cap = scout_max_tool_rounds();
        let profile_rounds = main_cap.map_or(scout_cap, |m| m.min(scout_cap));
        let profile = LoopProfile::retrieval(
            SubagentRole::Scout,
            &goal,
            profile_rounds,
            0,
            &activation_id,
            None,
            None,
            None,
        );
        let in_flight_tools = std::sync::Mutex::new(Vec::new());
        let svc = SharedLoopServices {
            blackboard: &self.blackboard,
            lif: &self.lif,
            denial_state: &self.denial_state,
            pacing_rounds: &self.pacing_rounds,
            context_compact: &self.context_compact,
            // scout 不参与检索证据台账（没有 [DOC]/[SOURCE] 行契约）。
            evidence: None,
            policy_revision: &self.policy_revision,
            max_inject_tokens_per_round: self.max_inject_tokens_per_round,
            blackboard_archive_dir: self.blackboard_archive_dir(),
            session_id: self.session_id.as_deref(),
            // 358c：scout 的超时孤儿补全随预算/并发批处理；本路径的
            // 墙钟到点只标记 `status=failed`，不伪造工具收口。
            in_flight_tools: Some(&in_flight_tools),
            retrieval_calls: None,
            usage_meter: &self.usage_meter,
        };
        let mut conversation = vec![Message {
            role: Role::User,
            content: goal.clone(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }];
        let started = Instant::now();
        let tokens_before = self
            .usage_meter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .token_totals();
        let wall_s = task.max_wall_s.unwrap_or(300).clamp(1, 900);
        let loop_future = Box::pin(run_agent_loop(
            &svc,
            self,
            writer,
            host,
            &subagent,
            &profile,
            &goal,
            &sub_tool_defs,
            &mut conversation,
            orientation,
            cancel,
            heartbeat,
            run_gateway,
        ));
        let outcome = tokio::time::timeout(Duration::from_secs(wall_s), loop_future).await;
        // 358 修复（审查 P2）：超时后把仍登记在册的子工具补一条
        // ToolCompleted(error)——不留悬空 ToolStarted。
        if outcome.is_err() {
            let pending = in_flight_tools
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            for (tool, call_id) in pending {
                writer
                    .record(
                        orz_assurance::EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tool,
                            "call_id": call_id,
                            "target": "scout",
                            "agent_role": "scout",
                            "status": "error",
                            "error": "scout_timeout",
                        }),
                    )
                    .await?;
            }
        }
        let tokens_after = self
            .usage_meter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .token_totals();
        let stats = ScoutStats {
            files_read: count_files_read(&conversation),
            tokens_used: tokens_after
                .0
                .saturating_sub(tokens_before.0)
                .saturating_add(tokens_after.1.saturating_sub(tokens_before.1)),
            wall_s: started.elapsed().as_secs(),
        };
        let (mut status, summary, findings, raw_output, error) = match outcome {
            Ok(Ok(loop_outcome)) => {
                let raw = loop_outcome.last_text.unwrap_or_default();
                match parse_scout_payload(&raw) {
                    Some((summary, findings)) => (ScoutStatus::Ok, summary, findings, raw, None),
                    None => (
                        ScoutStatus::Partial,
                        fallback_summary(&raw),
                        Vec::new(),
                        raw,
                        Some(ScoutError {
                            class: "parse".to_string(),
                            message: "scout output was not valid ScoutReport JSON".to_string(),
                            last_step: last_tool_name(&conversation),
                        }),
                    ),
                }
            }
            Ok(Err(e)) => (
                ScoutStatus::Failed,
                format!("scout failed: {e}"),
                Vec::new(),
                String::new(),
                Some(ScoutError {
                    class: "agent_loop".to_string(),
                    message: e.to_string(),
                    last_step: last_tool_name(&conversation),
                }),
            ),
            Err(_) => {
                let raw = last_assistant_text(&conversation);
                match parse_scout_payload(&raw) {
                    Some((summary, findings)) => {
                        (ScoutStatus::Partial, summary, findings, raw, None)
                    }
                    None => (
                        ScoutStatus::Failed,
                        format!("scout timed out after {wall_s}s"),
                        Vec::new(),
                        raw,
                        Some(ScoutError {
                            class: "timeout".to_string(),
                            message: format!("scout timed out after {wall_s}s"),
                            last_step: last_tool_name(&conversation),
                        }),
                    ),
                }
            }
        };
        if status != ScoutStatus::Failed
            && (task.max_files.is_some_and(|cap| stats.files_read > cap)
                || task.max_tokens.is_some_and(|cap| stats.tokens_used > cap)
                || task.max_wall_s.is_some_and(|cap| stats.wall_s > cap))
        {
            status = ScoutStatus::Partial;
        }
        let report = ScoutReport {
            report_id,
            status,
            summary: truncate_chars(
                &if summary.is_empty() {
                    fallback_summary(&raw_output)
                } else {
                    summary
                },
                SCOUT_FALLBACK_SUMMARY_MAX_CHARS,
            ),
            findings,
            stats,
            error,
        };
        let archive = persist_scout_report(writer, &report, &raw_output);
        let output = render_scout_summary(&report, archive.as_deref());
        let exit_code = if report.status == ScoutStatus::Failed {
            1
        } else {
            0
        };
        writer
            .record(
                orz_assurance::EventType::ToolCompleted,
                serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "target": "scout",
                    "agent_role": "scout",
                    // 358c：inspect 父子视图——child run 与 parent run 的显式
                    // 绑定（子代理事件与主 run 同 journal，child_run_id 是
                    // 逻辑子 run 的稳定标识）。
                    "child_run_id": child_run_id,
                    "parent_run_id": writer.run_id(),
                    "report_id": report.report_id,
                    "files_read": report.stats.files_read,
                    "tokens_used": report.stats.tokens_used,
                    "wall_s": report.stats.wall_s,
                    "exit_code": exit_code,
                    "status": if exit_code == 0 { "ok" } else { "error" },
                }),
            )
            .await?;
        messages.push(Message {
            role: Role::Tool,
            content: output.clone(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        Ok(ToolResult {
            output,
            exit_code: Some(exit_code),
            structured: Some(report.to_json()),
            ..Default::default()
        })
    }
}

/// 358b：scout 的独立轮数上限（默认 30；env 可覆盖，0/非法回默认）。
fn scout_max_tool_rounds() -> u32 {
    std::env::var("ORZ_SCOUT_MAX_TOOL_ROUNDS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|v| *v > 0)
        .unwrap_or(30)
}

/// 358b：全文落 `<journal_dir>/scout-report/<report_id>.md`（与
/// `retrieval-results/` 同族；落盘前机械脱敏）。
fn persist_scout_report(
    writer: &crate::controller::EventWriter<'_>,
    report: &ScoutReport,
    raw: &str,
) -> Option<String> {
    let dir = writer.journal_dir()?.join("scout_report");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{}.md", report.report_id));
    let body = render_scout_report_markdown(report, raw);
    std::fs::write(&path, orz_secrets::redact_secrets(&body).as_bytes()).ok()?;
    Some(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::ToolCall;

    #[test]
    fn parses_json_findings_and_drops_evidence_less_rows() {
        let text = "前言\n```json\n{\"summary\":\"3 处解码点\",\"findings\":[\
            {\"claim\":\"A\",\"evidence\":\"src/a.rs:12\",\"confidence\":\"high\"},\
            {\"claim\":\"B\",\"evidence\":\"src/b.rs:7\",\"confidence\":\"mid\"},\
            {\"claim\":\"no evidence\",\"evidence\":\"\",\"confidence\":\"high\"}]}\n```\n";
        let (summary, findings) = parse_scout_payload(text).expect("json parsed");
        assert_eq!(summary, "3 处解码点");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].evidence, "src/a.rs:12");
        assert_eq!(findings[1].confidence, ScoutConfidence::Mid);
    }

    #[test]
    fn summary_is_bounded_and_carries_archive_path() {
        let report = ScoutReport {
            report_id: "SC-0001".to_string(),
            status: ScoutStatus::Ok,
            summary: "结论".to_string(),
            findings: (0..80)
                .map(|i| ScoutFinding {
                    claim: format!("claim-{i}-{}", "x".repeat(30)),
                    evidence: format!("src/f{i}.rs:{i}"),
                    confidence: ScoutConfidence::High,
                })
                .collect(),
            stats: ScoutStats {
                files_read: 12,
                tokens_used: 18400,
                wall_s: 42,
            },
            error: None,
        };
        let text = render_scout_summary(&report, Some("/tmp/SC-0001.md"));
        assert!(text.chars().count() <= SCOUT_SUMMARY_MAX_CHARS);
        assert!(text.contains("SC-0001"));
    }

    #[test]
    fn markdown_keeps_raw_output() {
        let report = ScoutReport {
            report_id: "SC-0002".to_string(),
            status: ScoutStatus::Partial,
            summary: "部分".to_string(),
            findings: vec![ScoutFinding {
                claim: "A".to_string(),
                evidence: "a.rs:1".to_string(),
                confidence: ScoutConfidence::Low,
            }],
            stats: ScoutStats::default(),
            error: None,
        };
        let text = render_scout_report_markdown(&report, "raw text");
        assert!(text.contains("| A | a.rs:1 | low |"));
        assert!(text.contains("raw text"));
    }

    #[test]
    fn counts_distinct_read_files() {
        let mut conversation = Vec::new();
        for (i, path) in ["a.rs", "b.rs", "a.rs"].iter().enumerate() {
            conversation.push(Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    call_id: format!("c{i}"),
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": path}),
                }],
                reasoning_content: None,
                round: None,
            });
        }
        assert_eq!(count_files_read(&conversation), 2);
    }

    /// 358 修复（审查 P2）：`grep` 的文件路径计入 `files_read`，目录不计。
    #[test]
    fn counts_grep_file_paths() {
        let conversation = vec![Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![
                ToolCall {
                    call_id: "c0".to_string(),
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": "a.rs"}),
                },
                ToolCall {
                    call_id: "c1".to_string(),
                    name: "grep".to_string(),
                    arguments: serde_json::json!({"path": "b.rs"}),
                },
                ToolCall {
                    call_id: "c2".to_string(),
                    name: "grep".to_string(),
                    arguments: serde_json::json!({"path": "src"}),
                },
            ],
            reasoning_content: None,
            round: None,
        }];
        assert_eq!(count_files_read(&conversation), 2);
    }

    /// 358 修复（审查 P1/P2）：摘要带 findings 索引行与结构化错误行。
    #[test]
    fn summary_carries_index_line_and_error() {
        let report = ScoutReport {
            report_id: "SC-0002".to_string(),
            status: ScoutStatus::Failed,
            summary: "timed out".to_string(),
            findings: Vec::new(),
            stats: ScoutStats::default(),
            error: Some(ScoutError {
                class: "timeout".to_string(),
                message: "scout timed out after 2s".to_string(),
                last_step: Some("read_file".to_string()),
            }),
        };
        let out = render_scout_summary(&report, None);
        assert!(out.contains("index: SC-0002 | 0 files | 0 findings | timed out"));
        assert!(out.contains("error: timeout | scout timed out after 2s | last_step=read_file"));
    }

    #[test]
    fn task_goal_carries_scope_and_budget() {
        let task = ScoutTask::from_arguments(&serde_json::json!({
            "task": "列出解码点",
            "scope": { "paths": ["src"], "tools": ["read_file"] },
            "budget": { "max_files": 12 }
        }));
        assert_eq!(task.task, "列出解码点");
        assert_eq!(task.tools, vec!["read_file".to_string()]);
        assert_eq!(task.max_files, Some(12));
        let goal = task.render_goal();
        assert!(goal.contains("- src"));
        assert!(goal.contains("Allowed tools"));
        assert!(goal.contains("max_files=12"));
    }
}
