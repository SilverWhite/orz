//! Blackboard plan-epoch archives (ADR-0010 §14.15 / v1.15, 2026-08-14;
//! v1.15⑧ 补强 2026-08-15: timestamp-stamped monotonic epoch numbers).
//!
//! The blackboard's lifetime is a plan epoch: rotation archives the old
//! epoch's full path/action records as a deterministic JSON snapshot
//! (`.gsa/blackboard/epoch-<plan_epoch>.json`), clears the epoch-scoped
//! work partitions and writes the new plan. The same archives are the
//! cross-epoch `blackboard_read` source and the restore source.
//!
//! Discipline mirrors the compaction summary archive: bounded retry on
//! write, warn-not-swallow on corrupt reads, and the 7-day retention sweep
//! (see `orz-host::retention`).

use std::path::{Path, PathBuf};

use crate::blackboard::{
    EditRecord, EpochSnapshot, ExecSection, PlanSection, ToolActionRecord,
};

/// Archive directory name under the session cwd's `.gsa` root.
pub const EPOCH_ARCHIVE_DIR: &str = ".gsa/blackboard";
/// Bounded retries for persisting an epoch archive (same as compaction).
pub const EPOCH_ARCHIVE_MAX_ATTEMPTS: usize = 3;

/// Full path of one epoch archive.
pub fn epoch_archive_path(archive_dir: &Path, plan_epoch: u64) -> PathBuf {
    archive_dir.join(format!("epoch-{plan_epoch}.json"))
}

/// Highest epoch number already archived on disk (`None` = no archive yet).
fn max_archived_epoch(archive_dir: &Path) -> Option<u64> {
    let mut max_epoch = None;
    if let Ok(entries) = std::fs::read_dir(archive_dir) {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let Some(rest) = name.strip_prefix("epoch-").and_then(|r| r.strip_suffix(".json")) else {
                continue;
            };
            if let Ok(epoch) = rest.parse::<u64>() {
                if max_epoch.map_or(true, |m| epoch > m) {
                    max_epoch = Some(epoch);
                }
            }
        }
    }
    max_epoch
}

/// Next epoch number for a new plan approval (v1.15⑧, 2026-08-15):
/// a timestamp-stamped monotonic number — `max(now_ms, disk_max + 1)`.
///
/// Embedding the wall-clock timestamp in the identity itself (rather than
/// only in the file name) means references stay unique after the 7-day
/// retention sweep: old epoch numbers are always smaller than any future
/// number, so `plan_epoch` in markers / `blackboard_read` can never collide
/// with a later reuse. `disk_max + 1` keeps the sequence monotonic across
/// clock rollback and same-millisecond serial approvals.
pub fn next_plan_epoch_from_archive(archive_dir: &Path) -> u64 {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    now_ms.max(max_archived_epoch(archive_dir).map_or(0, |m| m + 1))
}

/// Persist one epoch snapshot with bounded retries. Returns whether the
/// file exists after the attempts; failures are logged by the caller.
pub fn write_epoch_archive_retry(archive_dir: &Path, snapshot: &EpochSnapshot) -> bool {
    let Ok(json) = serde_json::to_string_pretty(snapshot) else {
        return false;
    };
    let path = epoch_archive_path(archive_dir, snapshot.plan_epoch);
    for _ in 0..EPOCH_ARCHIVE_MAX_ATTEMPTS {
        if std::fs::create_dir_all(archive_dir).is_ok()
            && std::fs::write(&path, &json).is_ok()
            && path.exists()
        {
            return true;
        }
    }
    false
}

/// Load one epoch snapshot. Missing → `None` silently; corrupt/unreadable
/// → warn and `None` (sidecar discipline — never fail a caller over an
/// archive file).
pub fn load_epoch_snapshot(archive_dir: &Path, plan_epoch: u64) -> Option<EpochSnapshot> {
    let path = epoch_archive_path(archive_dir, plan_epoch);
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<EpochSnapshot>(&text) {
            Ok(snapshot) => Some(snapshot),
            Err(e) => {
                tracing::warn!("epoch archive corrupt ({}): {e} — skipped", path.display());
                None
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!("epoch archive unreadable ({}): {e} — skipped", path.display());
            None
        }
    }
}

/// Load the highest-numbered epoch snapshot (restore entry). `None` = no
/// archive yet. Scans the archive directly (the next-epoch number may be a
/// wall-clock value greater than every archived file).
pub fn latest_epoch_snapshot(archive_dir: &Path) -> Option<EpochSnapshot> {
    load_epoch_snapshot(archive_dir, max_archived_epoch(archive_dir)?)
}

/// Render one blackboard partition for `blackboard_read`, shared by the
/// live view and archived epoch snapshots. `since` (RFC 3339) filters
/// timestamped entries; plan/exec carry no per-entry timestamps.
pub fn render_section(
    plan: &PlanSection,
    edits: &[EditRecord],
    tool_actions: &[ToolActionRecord],
    exec: &ExecSection,
    section: &str,
    since: Option<&str>,
) -> String {
    let since_dt = since.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
    let after_since = |ts: &str| -> bool {
        match since_dt {
            None => true,
            Some(dt) => chrono::DateTime::parse_from_rfc3339(ts)
                .map(|t| t >= dt)
                // The record's own timestamp unparseable — keep it
                // (lenient: never hide records over a filter edge).
                .unwrap_or(true),
        }
    };
    match section {
        "plan" => {
            let goal = plan.goal.as_deref().unwrap_or("(no goal set)");
            let mut lines = vec![format!("goal: {goal}")];
            if let Some(plan_id) = &plan.plan_id {
                lines.push(format!("plan_id: {plan_id}"));
                lines.push(format!("plan_epoch: {}", plan.plan_epoch));
            }
            if plan.steps.is_empty() {
                lines.push("(no steps)".to_string());
            }
            for step in &plan.steps {
                let status = match step.status {
                    crate::blackboard::StepStatus::Pending => "pending",
                    crate::blackboard::StepStatus::InProgress => "in-progress",
                    crate::blackboard::StepStatus::Completed => "completed",
                    crate::blackboard::StepStatus::Blocked => "blocked",
                };
                lines.push(format!("- [{status}] {}", step.description));
            }
            lines.join("\n")
        }
        "edits" => {
            let lines: Vec<String> = edits
                .iter()
                .filter(|r| after_since(&r.timestamp))
                .map(|r| format!("{} {}", r.timestamp, crate::controller::format_edit_record(r)))
                .collect();
            if lines.is_empty() {
                "(no edit records)".to_string()
            } else {
                lines.join("\n")
            }
        }
        "tool_actions" => {
            let mut lines: Vec<String> = Vec::new();
            for category in ["read", "edit", "terminal", "retrieval", "other"] {
                let entries: Vec<String> = tool_actions
                    .iter()
                    .filter(|r| r.category == category)
                    .filter(|r| after_since(&r.timestamp))
                    .map(|r| format!("{} {}", r.timestamp, r.tool))
                    .collect();
                if !entries.is_empty() {
                    lines.push(format!("== {category} =="));
                    lines.extend(entries);
                }
            }
            if lines.is_empty() {
                "(no tool actions yet)".to_string()
            } else {
                lines.join("\n")
            }
        }
        "exec" => {
            let mut lines = Vec::new();
            lines.extend(exec.results.iter().cloned());
            lines.extend(exec.errors.iter().cloned());
            if lines.is_empty() {
                "(no exec results yet)".to_string()
            } else {
                const EXEC_RENDER_CAP: usize = 50;
                if lines.len() > EXEC_RENDER_CAP {
                    let omitted = lines.len() - EXEC_RENDER_CAP;
                    let head = format!(
                        "[exec: 共 {} 条，仅显示最近 {EXEC_RENDER_CAP} 条（较早条目省略 {omitted} 条）]",
                        lines.len(),
                    );
                    lines.drain(0..omitted);
                    lines.insert(0, head);
                }
                lines.join("\n")
            }
        }
        other => format!(
            "unknown blackboard section: {other} (expected plan|edits|tool_actions|exec)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{PlanStep, SharedBlackboard, StepStatus};

    #[test]
    fn next_epoch_is_timestamp_stamped_and_monotonic() {
        let dir = std::env::temp_dir().join(format!(
            "orz-epoch-scan-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let first = next_plan_epoch_from_archive(&dir);
        assert!(first >= before, "timestamp base: {first} < {before}");
        std::fs::write(dir.join("epoch-3.json"), "{}").unwrap();
        std::fs::write(dir.join("epoch-7.json"), "{}").unwrap();
        std::fs::write(dir.join("epoch-2.json"), "{}").unwrap();
        std::fs::write(dir.join("other.txt"), "{}").unwrap();
        let next = next_plan_epoch_from_archive(&dir);
        assert!(next > 7, "must exceed every archived epoch: {next}");
        assert!(next >= first, "monotonic: {next} < {first}");
        // Sweeping every archive must not reuse old numbers (time base).
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let after_sweep = next_plan_epoch_from_archive(&dir);
        assert!(after_sweep >= next, "reuse after sweep: {after_sweep} < {next}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotation_archives_and_restores() {
        let mut bb = crate::blackboard::Blackboard::new();
        bb.plan.plan_id = Some("PLAN-A".into());
        bb.plan.plan_epoch = 1;
        bb.plan.goal = Some("旧任务".into());
        bb.plan.steps.push(PlanStep {
            id: "s1".into(),
            description: "旧步骤".into(),
            status: StepStatus::InProgress,
        });
        bb.edits.push(EditRecord {
            file: "a.py".into(),
            old_lines: 1,
            new_lines: 2,
            timestamp: "2026-08-14T00:00:00Z".into(),
        });
        bb.tool_actions.push(ToolActionRecord {
            category: "read".to_string(),
            tool: "read_file".into(),
            timestamp: "2026-08-14T00:00:00Z".into(),
        });
        bb.exec.results.push("ok".into());
        bb.gate_log.gate_decisions.push("keep".into());

        let snapshot = bb
            .rotate_to_plan(
                "PLAN-B".into(),
                2,
                "新任务".into(),
                vec!["新步骤".into()],
                "2026-08-14T01:00:00Z",
            )
            .expect("rotation succeeds")
            .expect("old epoch snapshot");

        assert_eq!(snapshot.plan_epoch, 1);
        assert_eq!(snapshot.edits.len(), 1);
        assert_eq!(snapshot.tool_actions.len(), 1);
        assert_eq!(snapshot.exec.results, vec!["ok"]);
        assert_eq!(bb.plan.plan_id.as_deref(), Some("PLAN-B"));
        assert_eq!(bb.plan.plan_epoch, 2);
        assert!(bb.edits.is_empty());
        assert!(bb.tool_actions.is_empty());
        assert!(bb.exec.results.is_empty());
        // Exempt partitions survive.
        assert_eq!(bb.gate_log.gate_decisions, vec!["keep"]);

        // Same-plan revision: no rotation, no clearing.
        assert!(
            bb.rotate_to_plan(
                "PLAN-B".into(),
                2,
                "新任务（修订）".into(),
                vec!["新步骤".into(), "追加步骤".into()],
                "2026-08-14T02:00:00Z",
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(bb.plan.plan_epoch, 2);
        assert_eq!(bb.plan.goal.as_deref(), Some("新任务（修订）"));

        // Restore the archived epoch.
        let mut restored = crate::blackboard::Blackboard::new();
        restored.restore_epoch_snapshot(&snapshot);
        assert_eq!(restored.plan.plan_id.as_deref(), Some("PLAN-A"));
        assert_eq!(restored.plan.plan_epoch, 1);
        assert_eq!(restored.edits.len(), 1);
    }

    #[test]
    fn epoch_archive_write_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "orz-epoch-roundtrip-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.plan.plan_id = Some("PLAN-A".into());
            w.plan.plan_epoch = 1;
            w.edits.push(EditRecord {
                file: "x.py".into(),
                old_lines: 1,
                new_lines: 3,
                timestamp: "2026-08-14T00:00:00Z".into(),
            });
        }
        let snapshot = bb.read().epoch_snapshot("2026-08-14T01:00:00Z");
        assert!(write_epoch_archive_retry(&dir, &snapshot));
        let loaded = load_epoch_snapshot(&dir, 1).expect("loads");
        assert_eq!(loaded.plan_epoch, 1);
        assert_eq!(loaded.edits[0].file, "x.py");
        assert!(latest_epoch_snapshot(&dir).is_some());
        assert!(load_epoch_snapshot(&dir, 99).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
