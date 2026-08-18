//! Five-section template summary (P0-D S3, 2026-08-14).
//!
//! ADR-0010 v1.10 §14.10 ③ / CONTEXT_COMPACTION_DESIGN §4: the template
//! summary has five fixed slots — 目的 / 计划 / 变动文件路径 are mechanically
//! filled from the blackboard (plan + edit actions); 注意事项 / 后续衔接 are
//! mechanical placeholders (2026-08-18 B 定案，ADR-0010 §14.29) until the
//! HA structured-fact aggregation phase lands (阶段 (c)) — the compaction
//! makes ZERO model calls so it never re-bills the folded view under a
//! foreign prefix. Character limits 3/3/5/3/3K = 17K total bound the marker
//! and archive rendering.
//!
//! 2026-08-18 B 定案（ADR-0010 §14.29）：原 LLM 槽位的退化门（300 等效字符
//! /CJK 双计）、重试与解析全部退役——压缩零模型调用、无失败槽位；路径槽
//! 仍受 Top-40 AND 5K 双上限约束；存档写失败显式重试并在 marker/事件中
//! 上报，绝不静默。

use std::path::Path;

use crate::blackboard::{Blackboard, PlanStep, StepStatus};
use orz_assurance::journal::sha256_hex;

pub const SUMMARY_MAX_TOTAL_CHARS: usize = 17_000;
/// 目的 / 计划 / 变动文件路径 / 注意事项 / 后续衔接 — 3/3/5/3/3K.
pub const SUMMARY_SLOT_LIMITS: [usize; 5] = [3_000, 3_000, 5_000, 3_000, 3_000];

/// Bounded retries for persisting the summary archive — a write failure is
/// an audit gap and must be retried explicitly, then reported.
pub const ARCHIVE_WRITE_MAX_ATTEMPTS: usize = 3;

/// 机械模式占位（2026-08-18 B 定案，D1=(b)）：压缩不再调用模型生成
/// 注意事项/后续衔接；阶段 (c)（HA 结构化事实聚合）落地前，这两槽以
/// 固定占位呈现，主模型按 marker 的回查入口自行承接。
pub const MECHANICAL_NOTES_PLACEHOLDER: &str =
    "（机械模式：无模型槽位；阶段 (c) 事实聚合落地前由主模型按 marker 回查自行判断）";
pub const MECHANICAL_CONTINUATION_PLACEHOLDER: &str = "（机械模式：回查 blackboard_read 分区 plan/edits/tool_actions/exec/actions（历史用 epoch 参数）与摘要存档）";

/// Estimated tokens of one summary marker in the kept context. The marker
/// carries the five slots (up to ~17K chars ≈ 8.5K tokens under the
/// chars/2 estimate) plus framing — 9K is the conservative ceiling.
/// (P0-D review fix 2026-08-14: the previous 2K constant undercounted the
/// marker by up to ~4× and skewed the reduction guard.)
pub const SUMMARY_MARKER_ESTIMATE_TOKENS: u64 = 9_000;

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
    guard_failed: bool,
    ledger: Option<&str>,
) -> String {
    let mut out = format!(
        "# ORZ 会话压缩摘要 {id}\n\n\
         - 状态: complete（机械模式，零模型调用——2026-08-18 B 定案，ADR-0010 §14.29）\n\
         - 被压轮次: {rounds_dropped}\n\
         - 守卫强制: {}\n\n\
         ## 目的\n{}\n\n## 计划\n{}\n\n## 变动文件路径\n{}\n\n## 注意事项\n{}\n\n## 后续衔接\n{}\n",
        if guard_failed {
            "是（缩减守卫连续不满足，已强制压缩）"
        } else {
            "否"
        },
        slots.purpose,
        slots.plan,
        slots.paths,
        if slots.notes.is_empty() {
            MECHANICAL_NOTES_PLACEHOLDER
        } else {
            &slots.notes
        },
        if slots.continuation.is_empty() {
            MECHANICAL_CONTINUATION_PLACEHOLDER
        } else {
            &slots.continuation
        },
    );
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
    guard_failed: bool,
    archive_write_failed: bool,
    plan_epoch: u64,
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28): the fixed external ledger path — the marker line points a
    // restored conversation at the surviving append-only history.
    ledger_path: Option<&Path>,
) -> String {
    // 机械模式（2026-08-18 B 定案，ADR-0010 §14.29）：压缩恒写存档，
    // digest 恒存在——无「摘要重试失败」终止态。
    let digest_line = format!("摘要 digest: sha256:{digest}");
    let ledger_line =
        ledger_path.map_or_else(String::new, |p| format!("历史摘要累积于 {}\n", p.display()));
    format!(
        "[前文上下文已压缩 v0.2]\n\
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
            MECHANICAL_NOTES_PLACEHOLDER
        } else {
            &slots.notes
        },
        if slots.continuation.is_empty() {
            MECHANICAL_CONTINUATION_PLACEHOLDER
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
        let markdown = summary_archive_markdown("compaction-RUN-X-001", &slots, 3, false, None);
        let digest = archive_digest(&markdown);
        let marker = build_summary_marker(
            "compaction-RUN-X-001",
            &digest,
            Path::new(".gsa/compaction/compaction-RUN-X-001.md"),
            &slots,
            3,
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
        let markdown =
            summary_archive_markdown("compaction-RUN-X-001", &slots, 3, false, Some(&pointer));
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
            3,
            Some(Path::new(".gsa/ledger/current.md")),
        );
        assert!(marker.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX));
    }

    #[test]
    fn mechanical_placeholder_slots_in_marker_and_archive() {
        // 2026-08-18 B 定案（D1=(b)）：压缩零模型调用，注意事项/后续衔接
        // 为固定机械占位——无 summary_incomplete 终止态、无「生成失败」。
        let slots = SummarySlots {
            notes: String::new(),
            continuation: String::new(),
            ..slots()
        };
        let markdown = summary_archive_markdown("compaction-RUN-X-002", &slots, 2, false, None);
        assert!(markdown.contains("机械模式"));
        assert!(markdown.contains(MECHANICAL_NOTES_PLACEHOLDER));
        assert!(markdown.contains(MECHANICAL_CONTINUATION_PLACEHOLDER));
        assert!(!markdown.contains("生成失败"));
        let digest = archive_digest(&markdown);
        let marker = build_summary_marker(
            "compaction-RUN-X-002",
            &digest,
            Path::new(".gsa/compaction/x.md"),
            &slots,
            2,
            false,
            false,
            2,
            None,
        );
        assert!(!marker.contains("summary_incomplete"));
        assert!(marker.contains(MECHANICAL_NOTES_PLACEHOLDER));
        assert!(marker.contains(MECHANICAL_CONTINUATION_PLACEHOLDER));
        assert!(marker.contains(&format!("sha256:{digest}")));
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
