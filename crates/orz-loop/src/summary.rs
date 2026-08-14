//! Five-section template summary (P0-D S3, 2026-08-14).
//!
//! ADR-0010 v1.10 §14.10 ③ / CONTEXT_COMPACTION_DESIGN §4: the template
//! summary has five fixed slots — 目的 / 计划 / 变动文件路径 are mechanically
//! filled from the blackboard (plan + edit actions), 注意事项 / 后续衔接 are
//! model-generated and marked `derived_unverified`. Character limits
//! 3/3/5/3/3K = 17K total; the LLM output is validated mechanically and
//! redone ≤3 times; the termination state keeps the mechanical slots and
//! marks the marker `summary_incomplete`.

use std::path::Path;

use crate::blackboard::{Blackboard, PlanStep, StepStatus};
use orz_assurance::journal::sha256_hex;

pub const SUMMARY_MAX_TOTAL_CHARS: usize = 17_000;
/// 目的 / 计划 / 变动文件路径 / 注意事项 / 后续衔接 — 3/3/5/3/3K.
pub const SUMMARY_SLOT_LIMITS: [usize; 5] = [3_000, 3_000, 5_000, 3_000, 3_000];
pub const SUMMARY_MAX_ATTEMPTS: u32 = 3;
/// Completion budget for one summary call (17K chars ≈ ≤12K tokens).
pub const SUMMARY_MAX_TOKENS: u32 = 12_000;
/// Reused orz-compaction guard: a summary seed shorter than 500 chars is
/// degenerate (ADR-0010 v1.10 §14.10 ⑤ — `MIN_SUMMARY_SEED_CHARS`).
pub const MIN_SUMMARY_SEED_CHARS: usize = orz_compaction::MIN_SUMMARY_SEED_CHARS;

pub const NOTES_OPEN: &str = "[注意事项]";
pub const NOTES_CLOSE: &str = "[/注意事项]";
pub const CONTINUATION_OPEN: &str = "[后续衔接]";
pub const CONTINUATION_CLOSE: &str = "[/后续衔接]";

/// Estimated tokens of one summary marker in the kept context (chars/2
/// of the marker content — conservative CJK-aware guess).
pub const SUMMARY_MARKER_ESTIMATE_TOKENS: u64 = 2_000;

/// The five summary slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummarySlots {
    pub purpose: String,
    pub plan: String,
    pub paths: String,
    pub notes: String,
    pub continuation: String,
}

impl SummarySlots {
    /// Total character count across all five slots.
    pub fn total_chars(&self) -> usize {
        self.purpose.chars().count()
            + self.plan.chars().count()
            + self.paths.chars().count()
            + self.notes.chars().count()
            + self.continuation.chars().count()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SummaryError {
    #[error("summary slot {0} missing from model output")]
    MissingSlot(String),
    #[error("summary slot {0} exceeds {1} chars")]
    SlotTooLong(&'static str, usize),
    #[error("summary total exceeds {0} chars")]
    TotalTooLong(usize),
    #[error("degenerate summary output")]
    Degenerate,
}

/// Path-slot Top-N pointer for the overflow line (the full path list stays
/// on the blackboard, indexable via `blackboard_read` — never truncated
/// content).
const PATH_TOP_N: usize = 40;

/// Mechanical slot rendering — 目的 from the plan goal, 计划 from the plan
/// steps, 变动文件路径 from the blackboard edit records.
pub fn mechanical_slots(blackboard: &Blackboard) -> (String, String, String) {
    let purpose = blackboard
        .plan
        .goal
        .clone()
        .unwrap_or_else(|| "（未设置）".to_string());
    let plan = render_plan(&blackboard.plan.steps);
    let paths = render_paths(blackboard);
    (purpose, plan, paths)
}

fn step_status_label(status: &StepStatus) -> &'static str {
    match status {
        StepStatus::Pending => "待办",
        StepStatus::InProgress => "进行中",
        StepStatus::Completed => "已完成",
        StepStatus::Blocked => "受阻",
    }
}

fn render_plan(steps: &[PlanStep]) -> String {
    let mut out = String::new();
    for (i, step) in steps.iter().enumerate() {
        let line = format!("{}. [{}] {}", i + 1, step_status_label(&step.status), step.description);
        if out.chars().count() + line.chars().count() + 1 > SUMMARY_SLOT_LIMITS[1] {
            break;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&line);
    }
    if out.is_empty() {
        out = "（未设置）".to_string();
    }
    out
}

fn render_paths(blackboard: &Blackboard) -> String {
    let mut out = String::new();
    let mut overflow = 0usize;
    for edit in &blackboard.edits {
        let line = format!(
            "{}（{}→{} 行，{}）",
            edit.file, edit.old_lines, edit.new_lines, edit.timestamp
        );
        let next = if out.is_empty() {
            line.clone()
        } else {
            format!("{out}\n{line}")
        };
        if out.chars().count() < SUMMARY_SLOT_LIMITS[2] && next.chars().count() <= SUMMARY_SLOT_LIMITS[2]
        {
            out = next;
        } else {
            overflow += 1;
        }
    }
    if out.is_empty() {
        out = "（本窗口无编辑）".to_string();
    } else if overflow > 0 {
        out.push_str(&format!(
            "\n（其余 {overflow} 条路径见 blackboard_read 分区 edits；存档摘要保留 Top-{PATH_TOP_N} 索引化指针）"
        ));
    }
    out
}

/// The five-section summary system prompt — instructs the model to fill
/// ONLY 注意事项 / 后续衔接 from verifiable ledger/blackboard facts.
pub fn summary_system_prompt() -> String {
    "你是 ORZ 会话压缩器。你只填写两个槽位：\n\
     [注意事项] — 本窗口内必须记住的关键事实、风险与推断（推断内容须标注“（未验证）”）；\n\
     [后续衔接] — 下一步应继续做什么、回查入口（文件/证据路径、blackboard_read 分区）。\n\
     纪律：只能引用输入中台账/黑板可验证的事实；不得编造路径或结论；每槽不超过 3000 字符；\
     输出只包含这两个槽位，不要输出其他内容。"
        .to_string()
}

/// Build the summary user input: the mechanical slots block followed by the
/// already-collapsed history prefix (tool records were mechanically handled
/// by the first layer before this call).
pub fn summary_user_prompt(mechanical: &SummarySlots) -> String {
    format!(
        "机械槽位（你不需要修改）：\n目的: {}\n计划: {}\n变动文件路径: {}\n\n\
         以下为已坍缩的历史前缀（动作台账 + 最近轮），请据此填写 [注意事项] 与 [后续衔接]：",
        mechanical.purpose, mechanical.plan, mechanical.paths
    )
}

fn extract_slot(output: &str, open: &str, close: &str) -> Result<String, SummaryError> {
    let start = output
        .find(open)
        .ok_or_else(|| SummaryError::MissingSlot(open.to_string()))?;
    let after_open = start + open.len();
    let end = output[after_open..]
        .find(close)
        .map(|i| after_open + i)
        .ok_or_else(|| SummaryError::MissingSlot(close.to_string()))?;
    Ok(output[after_open..end].trim().to_string())
}

/// Parse and validate the model's two generated slots against the limits.
pub fn parse_model_output(
    output: &str,
    mechanical: &SummarySlots,
) -> Result<SummarySlots, SummaryError> {
    let notes = extract_slot(output, NOTES_OPEN, NOTES_CLOSE)?;
    let continuation = extract_slot(output, CONTINUATION_OPEN, CONTINUATION_CLOSE)?;
    if orz_compaction::is_degenerate_summary(output) {
        return Err(SummaryError::Degenerate);
    }
    if notes.chars().count() > SUMMARY_SLOT_LIMITS[3] {
        return Err(SummaryError::SlotTooLong("注意事项", SUMMARY_SLOT_LIMITS[3]));
    }
    if continuation.chars().count() > SUMMARY_SLOT_LIMITS[4] {
        return Err(SummaryError::SlotTooLong("后续衔接", SUMMARY_SLOT_LIMITS[4]));
    }
    let slots = SummarySlots {
        purpose: mechanical.purpose.clone(),
        plan: mechanical.plan.clone(),
        paths: mechanical.paths.clone(),
        notes,
        continuation,
    };
    if slots.total_chars() > SUMMARY_MAX_TOTAL_CHARS {
        return Err(SummaryError::TotalTooLong(SUMMARY_MAX_TOTAL_CHARS));
    }
    Ok(slots)
}

/// The archive file (markdown) for one summary — the audit copy with digest.
pub fn summary_archive_markdown(
    id: &str,
    slots: &SummarySlots,
    rounds_dropped: u32,
    incomplete: bool,
) -> String {
    let mut out = format!(
        "# ORZ 会话压缩摘要 {id}\n\n\
         - 状态: {}\n- derived_unverified: 注意事项/后续衔接为模型生成，未机械验证\n\
         - 被压轮次: {rounds_dropped}\n\n\
         ## 目的\n{}\n\n## 计划\n{}\n\n## 变动文件路径\n{}\n\n## 注意事项\n{}\n\n## 后续衔接\n{}\n",
        if incomplete { "summary_incomplete" } else { "complete" },
        slots.purpose,
        slots.plan,
        slots.paths,
        if slots.notes.is_empty() {
            "（生成失败）"
        } else {
            &slots.notes
        },
        if slots.continuation.is_empty() {
            "（生成失败）"
        } else {
            &slots.continuation
        },
    );
    if incomplete {
        out.push_str("\n> 摘要重试后仍失败：仅机械段有效，最近尾已扩大，后续轮次仍可正常执行。\n");
    }
    out
}

/// Build the rolling single marker (`[前文上下文已压缩` prefix — D3-1
/// restore-retained) carrying the summary content, the archive pointer and
/// the digest.
pub fn build_summary_marker(
    id: &str,
    digest: &str,
    archive_path: &Path,
    slots: &SummarySlots,
    rounds_dropped: u32,
    incomplete: bool,
) -> String {
    let state = if incomplete { "（summary_incomplete）" } else { "" };
    format!(
        "[前文上下文已压缩 v0.2 {state}]\n\
         摘要 ID: {id}\n被压轮次: {rounds_dropped} 轮\n\
         摘要存档: {}\n摘要 digest: sha256:{}\n\
         目的: {}\n\
         计划: {}\n\
         变动文件路径: {}\n\
         注意事项: {}\n\
         后续衔接: {}\n\
         回查: blackboard_read（分区 plan / edits / tool_actions / exec）\n\
         [/前文上下文已压缩]",
        archive_path.display(),
        digest,
        slots.purpose,
        slots.plan,
        slots.paths,
        if slots.notes.is_empty() {
            "（生成失败）"
        } else {
            &slots.notes
        },
        if slots.continuation.is_empty() {
            "（生成失败）"
        } else {
            &slots.continuation
        },
    )
}

/// sha256 digest of the archive content (the marker's digest binding).
pub fn archive_digest(markdown: &str) -> String {
    sha256_hex(markdown.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{EditRecord, PlanStep, StepStatus, SharedBlackboard};

    fn slots() -> SummarySlots {
        SummarySlots {
            purpose: "修复缓存回归".into(),
            plan: "1. 复现\n2. 定位".into(),
            paths: "src/controller.rs（12→34 行，2026-08-14T00:00:00Z）".into(),
            notes: "确认由前缀缓存引起".into(),
            continuation: "下一步：跑回归测试".into(),
        }
    }

    #[test]
    fn parse_valid_model_output() {
        let mechanical = slots();
        let output =
            format!(
                "{NOTES_OPEN} 确认由前缀缓存引起；{} [/注意事项]\n\
                 {CONTINUATION_OPEN} 下一步：跑回归测试；{}{CONTINUATION_CLOSE}",
                "补充说明。".repeat(60),
                "继续执行。".repeat(60),
            );
        let parsed = parse_model_output(&output, &mechanical).unwrap();
        assert!(parsed.notes.starts_with("确认由前缀缓存引起"));
        assert!(parsed.continuation.starts_with("下一步：跑回归测试"));
        assert!(parsed.total_chars() <= SUMMARY_MAX_TOTAL_CHARS);
    }

    #[test]
    fn parse_missing_slot_rejected() {
        let mechanical = slots();
        let output = "[注意事项] 只有注意事项";
        assert!(matches!(
            parse_model_output(output, &mechanical),
            Err(SummaryError::MissingSlot(_))
        ));
    }

    #[test]
    fn parse_oversized_slot_rejected() {
        let mechanical = slots();
        let output = format!(
            "{NOTES_OPEN}{}{NOTES_CLOSE}{CONTINUATION_OPEN}x{CONTINUATION_CLOSE}",
            "长".repeat(SUMMARY_SLOT_LIMITS[3] + 1)
        );
        assert!(matches!(
            parse_model_output(&output, &mechanical),
            Err(SummaryError::SlotTooLong(..))
        ));
    }

    #[test]
    fn mechanical_slots_from_blackboard() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.plan.goal = Some("修复 bug".into());
            w.plan.steps.push(PlanStep {
                id: "s1".into(),
                description: "复现".into(),
                status: StepStatus::InProgress,
            });
            w.edits.push(EditRecord {
                file: "a.py".into(),
                old_lines: 1,
                new_lines: 2,
                timestamp: "2026-08-14T00:00:00Z".into(),
            });
        }
        let (purpose, plan, paths) = mechanical_slots(&bb.read());
        assert_eq!(purpose, "修复 bug");
        assert!(plan.contains("复现"));
        assert!(plan.contains("进行中"));
        assert!(paths.contains("a.py"));
        assert!(paths.contains("1→2 行"));
    }

    #[test]
    fn marker_carries_content_pointer_and_digest() {
        let slots = slots();
        let markdown = summary_archive_markdown("compaction-RUN-X-001", &slots, 3, false);
        let digest = archive_digest(&markdown);
        let marker = build_summary_marker(
            "compaction-RUN-X-001",
            &digest,
            Path::new(".gsa/compaction/compaction-RUN-X-001.md"),
            &slots,
            3,
            false,
        );
        assert!(marker.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX));
        assert!(marker.contains(&digest));
        assert!(marker.contains("compaction-RUN-X-001.md"));
        assert!(marker.contains("修复缓存回归"));
        assert!(crate::prompt::is_restore_retained_block(&marker));
        assert!(crate::prompt::is_injected_block_text(&marker));
    }

    #[test]
    fn incomplete_marker_flags_state() {
        let slots = SummarySlots {
            notes: String::new(),
            continuation: String::new(),
            ..slots()
        };
        let marker = build_summary_marker(
            "compaction-RUN-X-002",
            "d" .repeat(64).as_str(),
            Path::new(".gsa/compaction/x.md"),
            &slots,
            2,
            true,
        );
        assert!(marker.contains("summary_incomplete"));
        assert!(marker.contains("（生成失败）"));
    }
}
