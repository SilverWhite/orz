//! Five-section template summary (P0-D S3, 2026-08-14).
//!
//! ADR-0010 v1.10 §14.10 ③ / CONTEXT_COMPACTION_DESIGN §4: the template
//! summary has five fixed slots — 目的 / 计划 / 变动文件路径 are mechanically
//! filled from the blackboard (plan + edit actions), 注意事项 / 后续衔接 are
//! model-generated and marked `derived_unverified`. Character limits
//! 3/3/5/3/3K = 17K total; the LLM output is validated mechanically and
//! redone ≤3 times; the termination state keeps the mechanical slots and
//! marks the marker `summary_incomplete`.
//!
//! P0-D review fix (2026-08-14, ADR-0010 v1.14): the degeneration guard is
//! ORZ's own 300-effective-char gate (CJK ideographs count double — the
//! orz-compaction 500 gate was calibrated for English); the path slot is
//! capped by Top-40 AND 5K chars; archive writes retry explicitly and the
//! failure is surfaced in the marker/event instead of being swallowed.

use std::path::Path;

use crate::blackboard::{Blackboard, PlanStep, StepStatus};
use orz_assurance::journal::sha256_hex;

pub const SUMMARY_MAX_TOTAL_CHARS: usize = 17_000;
/// 目的 / 计划 / 变动文件路径 / 注意事项 / 后续衔接 — 3/3/5/3/3K.
pub const SUMMARY_SLOT_LIMITS: [usize; 5] = [3_000, 3_000, 5_000, 3_000, 3_000];
pub const SUMMARY_MAX_ATTEMPTS: u32 = 3;
/// Completion budget for one summary call (17K chars ≈ ≤12K tokens).
pub const SUMMARY_MAX_TOKENS: u32 = 12_000;
/// ORZ's own degeneration gate (P0-D review fix 2026-08-14): a summary
/// whose EFFECTIVE length is below 300 is degenerate. The orz-compaction
/// 500-char gate was calibrated for English text; CJK ideographs carry
/// roughly twice the information of one English character, so each CJK
/// char counts as 2 effective chars (150 CJK chars pass the gate).
pub const SUMMARY_MIN_EFFECTIVE_CHARS: usize = 300;

/// Bounded retries for persisting the summary archive — a write failure is
/// an audit gap and must be retried explicitly, then reported.
pub const ARCHIVE_WRITE_MAX_ATTEMPTS: usize = 3;

pub const NOTES_OPEN: &str = "[注意事项]";
pub const NOTES_CLOSE: &str = "[/注意事项]";
pub const CONTINUATION_OPEN: &str = "[后续衔接]";
pub const CONTINUATION_CLOSE: &str = "[/后续衔接]";

/// Estimated tokens of one summary marker in the kept context. The marker
/// carries the five slots (up to ~17K chars ≈ 8.5K tokens under the
/// chars/2 estimate) plus framing — 9K is the conservative ceiling.
/// (P0-D review fix 2026-08-14: the previous 2K constant undercounted the
/// marker by up to ~4× and skewed the reduction guard.)
pub const SUMMARY_MARKER_ESTIMATE_TOKENS: u64 = 9_000;

/// One summary chat call's wall-clock budget (ADR-0010 v1.10 §4.2 "超时
/// 120s" — the session transport's own timeouts are far longer and must
/// not hold the emergency path).
pub const SUMMARY_CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

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

/// Effective character length for the degeneration gate: CJK ideographs
/// count as 2, everything else as 1.
pub fn effective_summary_chars(s: &str) -> usize {
    s.chars()
        .map(|c| if is_cjk_ideograph(c) { 2 } else { 1 })
        .sum()
}

fn is_cjk_ideograph(c: char) -> bool {
    matches!(c as u32,
        0x3400..=0x4DBF   // CJK Extension A
        | 0x4E00..=0x9FFF // CJK Unified Ideographs
        | 0xF900..=0xFAFF // CJK Compatibility Ideographs
        | 0x20000..=0x2A6DF // Extension B
        | 0x2A700..=0x2B73F // Extension C
        | 0x2B740..=0x2B81F // Extension D
        | 0x2B820..=0x2CEAF // Extension E
    )
}

/// ORZ degeneration guard — replaces the orz-compaction 500-char gate.
pub fn is_degenerate_summary(output: &str) -> bool {
    effective_summary_chars(output) < SUMMARY_MIN_EFFECTIVE_CHARS
}

/// Persist the summary archive with bounded retries. Returns whether the
/// file exists after the attempts; a failure is NEVER silent here — the
/// caller surfaces it in the marker and the `context_compressed` event.
pub fn write_archive_retry(archive_dir: &Path, archive_path: &Path, markdown: &str) -> bool {
    for _ in 0..ARCHIVE_WRITE_MAX_ATTEMPTS {
        if std::fs::create_dir_all(archive_dir).is_ok()
            && std::fs::write(archive_path, markdown).is_ok()
            && archive_path.exists()
        {
            return true;
        }
    }
    false
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

/// Path-slot Top-N (design §10 / audit D-4): at most 40 plan-epoch edits by
/// insertion order; the overflow line points at the current plan-epoch
/// snapshot (v1.15, 2026-08-14 — the epoch archive is the permanent holder
/// of the full path/action list; the summary archive keeps the human-
/// readable projection).
const PATH_TOP_N: usize = 40;

/// Mechanical slot rendering — 目的 from the plan goal, 计划 from the plan
/// steps, 变动文件路径 from the blackboard edit records. `epoch_archive`
/// is the current plan-epoch snapshot path — the overflow pointer target
/// when the path slot overflows (falls back to the compaction archive).
pub fn mechanical_slots(
    blackboard: &Blackboard,
    archive_path: &Path,
    epoch_archive: Option<&Path>,
) -> (String, String, String) {
    let purpose = blackboard
        .plan
        .goal
        .clone()
        .unwrap_or_else(|| "（未设置）".to_string());
    let plan = render_plan(&blackboard.plan.steps);
    let paths = render_paths(blackboard, archive_path, epoch_archive);
    (purpose, plan, paths)
}

fn step_status_label(status: &StepStatus) -> &'static str {
    match status {
        StepStatus::Pending => "待办",
        StepStatus::InProgress => "进行中",
        StepStatus::Done(_) => "已完成",
        StepStatus::Failed(_) => "失败",
        StepStatus::Blocked => "受阻",
    }
}

fn render_plan(steps: &[PlanStep]) -> String {
    let mut out = String::new();
    for (i, step) in steps.iter().enumerate() {
        let line = format!(
            "{}. [{}] {}",
            i + 1,
            step_status_label(&step.status),
            step.goal
        );
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

fn render_paths(
    blackboard: &Blackboard,
    archive_path: &Path,
    epoch_archive: Option<&Path>,
) -> String {
    let mut out = String::new();
    let mut overflow = 0usize;
    for edit in blackboard.edits.iter().take(PATH_TOP_N) {
        let line = format!(
            "{}（{}→{} 行，{}）",
            edit.file, edit.old_lines, edit.new_lines, edit.timestamp
        );
        let next = if out.is_empty() {
            line.clone()
        } else {
            format!("{out}\n{line}")
        };
        if out.chars().count() < SUMMARY_SLOT_LIMITS[2]
            && next.chars().count() <= SUMMARY_SLOT_LIMITS[2]
        {
            out = next;
        } else {
            overflow += 1;
        }
    }
    overflow += blackboard.edits.len().saturating_sub(PATH_TOP_N);
    if out.is_empty() {
        out = "（本窗口无编辑）".to_string();
    } else if overflow > 0 {
        let holder = epoch_archive
            .unwrap_or(archive_path)
            .to_string_lossy()
            .to_string();
        out.push_str(&format!(
            "\n（其余 {overflow} 条路径见本 plan epoch 快照 {holder}）",
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
    if is_degenerate_summary(output) {
        return Err(SummaryError::Degenerate);
    }
    if notes.chars().count() > SUMMARY_SLOT_LIMITS[3] {
        return Err(SummaryError::SlotTooLong(
            "注意事项",
            SUMMARY_SLOT_LIMITS[3],
        ));
    }
    if continuation.chars().count() > SUMMARY_SLOT_LIMITS[4] {
        return Err(SummaryError::SlotTooLong(
            "后续衔接",
            SUMMARY_SLOT_LIMITS[4],
        ));
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
/// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26 / 设计 §3.5 第 1 步):
/// when the loop is folded, the frozen action-ledger block is appended as
/// its own section — the compaction drains the folded region from
/// `messages`, so the archive is the ONLY place that preserves exactly what
/// the model saw (工具名/目标/结果指针/最终回复). None when not folded.
pub fn summary_archive_markdown(
    id: &str,
    slots: &SummarySlots,
    rounds_dropped: u32,
    incomplete: bool,
    guard_failed: bool,
    ledger: Option<&str>,
) -> String {
    let mut out = format!(
        "# ORZ 会话压缩摘要 {id}\n\n\
         - 状态: {}\n- derived_unverified: 注意事项/后续衔接为模型生成，未机械验证\n\
         - 被压轮次: {rounds_dropped}\n\
         - 守卫强制: {}\n\n\
         ## 目的\n{}\n\n## 计划\n{}\n\n## 变动文件路径\n{}\n\n## 注意事项\n{}\n\n## 后续衔接\n{}\n",
        if incomplete {
            "summary_incomplete"
        } else {
            "complete"
        },
        if guard_failed {
            "是（缩减守卫连续不满足，已强制压缩）"
        } else {
            "否"
        },
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
    if let Some(ledger) = ledger {
        // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18,
        // ADR-0010 §14.28): the frozen "ledger" the model saw is now the
        // byte-fixed pointer message — the folded rows themselves survive
        // in the append-only external ledger file (never drained by
        // compaction). The archive still records exactly what the model
        // saw plus the pointer to the surviving projection.
        out.push_str("\n## 折叠视图（冻结快照：外挂指针）\n\n```\n");
        out.push_str(ledger);
        out.push_str("\n```\n");
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
    guard_failed: bool,
    archive_write_failed: bool,
    plan_epoch: u64,
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28): the fixed external ledger path — the marker line points a
    // restored conversation at the surviving append-only history.
    ledger_path: Option<&Path>,
) -> String {
    let state = if incomplete {
        "（summary_incomplete）"
    } else {
        ""
    };
    // P0-D S6 (2026-08-14): when no archive was written (termination state)
    // the digest placeholder must be explicit instead of a misleading
    // 64-zero digest — the event already carries `summary_digest: null`.
    let digest_line = if digest.is_empty() {
        "摘要 digest: （未生成——摘要重试失败）".to_string()
    } else {
        format!("摘要 digest: sha256:{digest}")
    };
    let ledger_line =
        ledger_path.map_or_else(String::new, |p| format!("历史摘要累积于 {}\n", p.display()));
    format!(
        "[前文上下文已压缩 v0.2 {state}]\n\
         {guard_note}\
         {archive_note}\
         {ledger_line}\
         摘要 ID: {id}\n被压轮次: {rounds_dropped} 轮\n\
         摘要存档: {}\n{digest_line}\n\
         黑板 plan_epoch: {}\n\
         目的: {}\n\
         计划: {}\n\
         变动文件路径: {}\n\
         注意事项: {}\n\
         后续衔接: {}\n\
         回查: blackboard_read（分区 plan / edits / tool_actions / exec / actions；历史 plan epoch 用 epoch 参数）\n\
         [/前文上下文已压缩]",
        archive_path.display(),
        if plan_epoch > 0 {
            plan_epoch.to_string()
        } else {
            "（未设置）".to_string()
        },
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
        guard_note = if guard_failed {
            "机制失败：缩减守卫连续不满足，已强制压缩，需处理\n"
        } else {
            ""
        },
        archive_note = if archive_write_failed {
            "存档写入失败：摘要未落盘，需处理\n"
        } else {
            ""
        },
        ledger_line = ledger_line,
    )
}

/// sha256 digest of the archive content (the marker's digest binding).
pub fn archive_digest(markdown: &str) -> String {
    sha256_hex(markdown.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{EditRecord, PlanStep, SharedBlackboard, StepStatus};

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
        let output = format!(
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
                goal: "复现".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::InProgress,
            });
            w.edits.push(EditRecord {
                file: "a.py".into(),
                old_lines: 1,
                new_lines: 2,
                timestamp: "2026-08-14T00:00:00Z".into(),
            });
        }
        let (purpose, plan, paths) =
            mechanical_slots(&bb.read(), Path::new(".gsa/compaction/x.md"), None);
        assert_eq!(purpose, "修复 bug");
        assert!(plan.contains("复现"));
        assert!(plan.contains("进行中"));
        assert!(paths.contains("a.py"));
        assert!(paths.contains("1→2 行"));
    }

    #[test]
    fn render_paths_caps_at_top_40_with_archive_pointer() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            for i in 0..45 {
                w.edits.push(EditRecord {
                    file: format!("f{i}.py"),
                    old_lines: 1,
                    new_lines: 2,
                    timestamp: "2026-08-14T00:00:00Z".into(),
                });
            }
        }
        let archive = Path::new(".gsa/compaction/compaction-RUN-X-0099.md");
        let epoch_archive = Path::new(".gsa/blackboard/epoch-3.json");
        let (_purpose, _plan, paths) = mechanical_slots(&bb.read(), archive, Some(epoch_archive));
        // Top-40 by insertion order: f0..f39 appear, f40..f44 are overflow.
        assert!(paths.contains("f0.py"));
        assert!(paths.contains("f39.py"));
        assert!(!paths.contains("f40.py"));
        assert!(paths.contains("其余 5 条路径"));
        // v1.15: the overflow pointer targets the plan-epoch snapshot, not
        // the compaction archive.
        assert!(paths.contains(epoch_archive.to_string_lossy().as_ref()));
        assert!(!paths.contains(archive.to_string_lossy().as_ref()));
    }

    #[test]
    fn degenerate_guard_english_threshold() {
        let short = "x".repeat(SUMMARY_MIN_EFFECTIVE_CHARS - 1);
        assert!(is_degenerate_summary(&short));
        let boundary = "x".repeat(SUMMARY_MIN_EFFECTIVE_CHARS);
        assert!(!is_degenerate_summary(&boundary));
    }

    #[test]
    fn degenerate_guard_chinese_counts_double() {
        // 149 CJK chars = 298 effective chars — degenerate.
        assert!(is_degenerate_summary(&"汉".repeat(149)));
        // 150 CJK chars = 300 effective chars — accepted.
        assert!(!is_degenerate_summary(&"汉".repeat(150)));
        // Mixed: 100 CJK + 100 ASCII = 300 effective chars — accepted.
        let mixed = format!("{}{}", "汉".repeat(100), "x".repeat(100));
        assert!(!is_degenerate_summary(&mixed));
    }

    #[test]
    fn archive_write_retry_persists_and_reports_failure() {
        let dir = std::env::temp_dir().join(format!(
            "orz-summary-archive-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let archive_dir = dir.join(".gsa").join("compaction");
        let archive_path = archive_dir.join("compaction-RUN-X-0001.md");
        assert!(write_archive_retry(&archive_dir, &archive_path, "# 摘要"));
        assert!(archive_path.exists());

        // A path occupied by a FILE can never become the archive dir — the
        // bounded retries all fail and the caller must report it.
        let blocked = dir.join("blocked");
        std::fs::write(&blocked, "occupied").unwrap();
        let blocked_archive = blocked.join("compaction-RUN-X-0002.md");
        assert!(!write_archive_retry(&blocked, &blocked_archive, "# 摘要"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn marker_carries_content_pointer_and_digest() {
        let slots = slots();
        let markdown =
            summary_archive_markdown("compaction-RUN-X-001", &slots, 3, false, false, None);
        let digest = archive_digest(&markdown);
        let marker = build_summary_marker(
            "compaction-RUN-X-001",
            &digest,
            Path::new(".gsa/compaction/compaction-RUN-X-001.md"),
            &slots,
            3,
            false,
            false,
            false,
            3,
            Some(Path::new(".gsa/ledger/current.md")),
        );
        assert!(marker.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX));
        assert!(marker.contains(&digest));
        assert!(marker.contains("compaction-RUN-X-001.md"));
        assert!(marker.contains("历史摘要累积于 .gsa/ledger/current.md"));
        assert!(marker.contains("修复缓存回归"));
        assert!(marker.contains("黑板 plan_epoch: 3"));
        assert!(crate::prompt::is_restore_retained_block(&marker));
        assert!(crate::prompt::is_injected_block_text(&marker));
    }

    #[test]
    fn archive_appends_frozen_pointer_section_when_folded() {
        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26 + §14.28
        // external-file design): the compaction archive preserves the
        // frozen view the model saw — the byte-fixed pointer message (the
        // folded rows themselves survive in the append-only external
        // ledger file, which compaction never drains).
        let slots = slots();
        let pointer =
            crate::action_ledger::build_pointer_message(Path::new(".gsa/ledger/current.md"));
        let markdown = summary_archive_markdown(
            "compaction-RUN-X-001",
            &slots,
            3,
            false,
            false,
            Some(&pointer),
        );
        assert!(
            markdown.contains("## 折叠视图（冻结快照：外挂指针）"),
            "{markdown}"
        );
        assert!(markdown.contains(&pointer), "{markdown}");
        let digest = archive_digest(&markdown);
        let marker = build_summary_marker(
            "compaction-RUN-X-001",
            &digest,
            Path::new(".gsa/compaction/compaction-RUN-X-001.md"),
            &slots,
            3,
            false,
            false,
            false,
            3,
            Some(Path::new(".gsa/ledger/current.md")),
        );
        assert!(marker.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX));
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
            "",
            Path::new(".gsa/compaction/x.md"),
            &slots,
            2,
            true,
            false,
            false,
            2,
            None,
        );
        assert!(marker.contains("summary_incomplete"));
        assert!(marker.contains("（生成失败）"));
        assert!(marker.contains("摘要 digest: （未生成"));
        assert!(!marker.contains("sha256:000000"));
        assert!(marker.contains("黑板 plan_epoch: 2"));
    }

    #[test]
    fn marker_reports_guard_and_archive_failures() {
        let slots = slots();
        let marker = build_summary_marker(
            "compaction-RUN-X-003",
            "d".repeat(64).as_str(),
            Path::new(".gsa/compaction/x.md"),
            &slots,
            2,
            false,
            true,
            true,
            0,
            None,
        );
        assert!(marker.contains("机制失败：缩减守卫连续不满足"));
        assert!(marker.contains("存档写入失败：摘要未落盘"));
        assert!(marker.contains("黑板 plan_epoch: （未设置）"));
        assert!(crate::prompt::is_restore_retained_block(&marker));
    }
}
