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
    ActionBoard, EditRecord, EpochSnapshot, ExecSection, PlanSection, ToolActionRecord,
};

/// Archive directory name under the session cwd's `.gsa` root.
pub const EPOCH_ARCHIVE_DIR: &str = ".gsa/blackboard";
/// Bounded retries for persisting an epoch archive (same as compaction).
pub const EPOCH_ARCHIVE_MAX_ATTEMPTS: usize = 3;
/// F4 (2026-08-15): bounded collision retries when claiming an epoch
/// against concurrent processes.
pub const EPOCH_CLAIM_MAX_ATTEMPTS: usize = 8;

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
            let Some(rest) = name
                .strip_prefix("epoch-")
                .and_then(|r| r.strip_suffix(".json"))
            else {
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

/// Highest epoch number already claimed via a `.claim-<n>` reservation file
/// (`None` = no claim yet). Claim files outlive their claimant on purpose
/// (F4, 2026-08-15): a process that crashes between claiming and writing
/// the snapshot still owns the number — timestamp-stamped numbering makes
/// burning one number free, and reuse would break the one-to-one identity
/// convention. Retention sweeps stale claims by age (see `orz-host`).
fn max_claimed_epoch(archive_dir: &Path) -> Option<u64> {
    let mut max_epoch = None;
    if let Ok(entries) = std::fs::read_dir(archive_dir) {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let Some(rest) = name.strip_prefix(".claim-") else {
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
/// clock rollback and same-millisecond serial approvals. F4 (2026-08-15):
/// `.claim-*` reservations are part of `disk_max`, so a crashed claimant's
/// number is never re-proposed.
pub fn next_plan_epoch_from_archive(archive_dir: &Path) -> u64 {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let disk_max = max_archived_epoch(archive_dir)
        .into_iter()
        .chain(max_claimed_epoch(archive_dir))
        .max();
    now_ms.max(disk_max.map_or(0, |m| m + 1))
}

/// Atomically claim a plan epoch against concurrent processes (F4,
/// 2026-08-15, BACKLOG 6e 复查遗留).
///
/// Creates the `.claim-<plan_epoch>` reservation file with `create_new` —
/// exactly one process can win per epoch number. Returns `true` only for
/// the winner. On a collision the caller should bump and retry (bounded by
/// [`EPOCH_CLAIM_MAX_ATTEMPTS`]). The reservation is intentionally NOT
/// removed on success: it remains as the durable proof that this number was
/// allocated, so even a crash between claim and snapshot write cannot lead
/// to reuse. Retention sweeps old claim files by age (they are only
/// meaningful near their timestamp).
pub fn claim_plan_epoch(archive_dir: &Path, plan_epoch: u64) -> bool {
    std::fs::create_dir_all(archive_dir).is_ok()
        && std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(archive_dir.join(format!(".claim-{plan_epoch}")))
            .is_ok()
}

/// Persist one epoch snapshot atomically with bounded retries. Returns
/// whether the final file exists after the attempts; failures are logged
/// by the caller.
///
/// F2 (2026-08-15, BACKLOG 6e 复查遗留): the snapshot is written to a
/// `<name>.json.tmp` sibling first and re-parsed as a self-check, then
/// renamed over the final `epoch-<n>.json` (same directory → same
/// filesystem → atomic on the local disk). A crash before the rename
/// leaves only the `.tmp` file, which the `epoch-*.json` scans ignore —
/// the highest-numbered FINAL file always points at a complete snapshot,
/// so a half-written file can never become the restore entry.
pub fn write_epoch_archive_retry(archive_dir: &Path, snapshot: &EpochSnapshot) -> bool {
    let Ok(json) = serde_json::to_string_pretty(snapshot) else {
        return false;
    };
    let path = epoch_archive_path(archive_dir, snapshot.plan_epoch);
    let tmp_path = path.with_extension("json.tmp");
    for _ in 0..EPOCH_ARCHIVE_MAX_ATTEMPTS {
        if std::fs::create_dir_all(archive_dir).is_err() {
            continue;
        }
        if std::fs::write(&tmp_path, &json).is_err() {
            continue;
        }
        // Self-check before the file may become final: a snapshot that
        // cannot be parsed back must never be published under the final
        // name (a torn write or a serializer bug must not poison restore).
        let parseable = std::fs::read_to_string(&tmp_path)
            .ok()
            .and_then(|text| serde_json::from_str::<EpochSnapshot>(&text).ok())
            .is_some();
        if !parseable {
            let _ = std::fs::remove_file(&tmp_path);
            continue;
        }
        if std::fs::rename(&tmp_path, &path).is_ok() {
            return true;
        }
    }
    let _ = std::fs::remove_file(&tmp_path);
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
            tracing::warn!(
                "epoch archive unreadable ({}): {e} — skipped",
                path.display()
            );
            None
        }
    }
}

/// Load the highest-numbered epoch snapshot (restore entry). `None` = no
/// usable archive yet. Scans the archive directly (the next-epoch number
/// may be a wall-clock value greater than every archived file) and walks
/// epochs from high to low, returning the FIRST parseable snapshot.
///
/// F2 (2026-08-15, BACKLOG 6e 复查遗留): a corrupt/unreadable highest file
/// (e.g. a pre-F2 half-written archive, or an externally damaged file) must
/// not make recovery fail — the previous valid snapshot is the fallback.
pub fn latest_epoch_snapshot(archive_dir: &Path) -> Option<EpochSnapshot> {
    let mut epochs: Vec<u64> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(archive_dir) {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let Some(rest) = name
                .strip_prefix("epoch-")
                .and_then(|r| r.strip_suffix(".json"))
            else {
                continue;
            };
            if let Ok(epoch) = rest.parse::<u64>() {
                epochs.push(epoch);
            }
        }
    }
    epochs.sort_unstable_by(|a, b| b.cmp(a));
    for epoch in epochs {
        if let Some(snapshot) = load_epoch_snapshot(archive_dir, epoch) {
            return Some(snapshot);
        }
    }
    None
}

/// Render one blackboard partition for `blackboard_read`, shared by the
/// live view and archived epoch snapshots. `since` (RFC 3339) filters
/// timestamped entries; plan/exec carry no per-entry timestamps.
pub fn render_section(
    plan: &PlanSection,
    edits: &[EditRecord],
    tool_actions: &[ToolActionRecord],
    exec: &ExecSection,
    actions: &ActionBoard,
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
                lines.push(format!(
                    "- [{status}] {} (actions: {}; evidence: {})",
                    step.goal,
                    step.actions.len(),
                    step.evidence.len(),
                ));
            }
            lines.join("\n")
        }
        "edits" => {
            let lines: Vec<String> = edits
                .iter()
                .filter(|r| after_since(&r.timestamp))
                .map(|r| {
                    format!(
                        "{} {}",
                        r.timestamp,
                        crate::controller::format_edit_record(r)
                    )
                })
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
        // P0-C orz 内嵌集成 S2 (2026-08-15): the console action board —
        // registration (assistant-refreshed buttons), the pending order
        // (model-written single slot) and the result receipts (issuance).
        // Bounded renders: registration may grow with Profile/Bundle; the
        // board's result list is already capped at 50, and the text view
        // shows the latest 10 with an explicit count.
        "actions" => {
            let mut lines: Vec<String> = Vec::new();
            lines.push("== registration ==".to_string());
            if actions.registration.is_empty() {
                lines.push("(no registered actions)".to_string());
            }
            for reg in &actions.registration {
                let params =
                    serde_json::to_string(&reg.parameters).unwrap_or_else(|_| "{}".to_string());
                lines.push(format!(
                    "{} — {} params={params}",
                    reg.name, reg.description
                ));
            }
            lines.push("== order ==".to_string());
            match &actions.order {
                Some(order) => {
                    let args = serde_json::to_string(&order.arguments)
                        .unwrap_or_else(|_| "{}".to_string());
                    lines.push(format!(
                        "{} action={} round={} plan_epoch={} run_id={} args={args}",
                        order.order_id, order.action, order.round, order.plan_epoch, order.run_id,
                    ));
                }
                None => lines.push("(no pending order)".to_string()),
            }
            lines.push("== results ==".to_string());
            if actions.results.is_empty() {
                lines.push("(no results yet)".to_string());
            } else {
                const RESULTS_RENDER_CAP: usize = 10;
                if actions.results.len() > RESULTS_RENDER_CAP {
                    let omitted = actions.results.len() - RESULTS_RENDER_CAP;
                    lines.push(format!(
                        "[actions: 共 {} 条，仅显示最近 {RESULTS_RENDER_CAP} 条（较早省略 {omitted} 条）]",
                        actions.results.len(),
                    ));
                }
                for result in actions.results.iter().rev().take(RESULTS_RENDER_CAP) {
                    let line = if result.ok {
                        let response = serde_json::to_string(&result.response)
                            .unwrap_or_else(|_| "{}".to_string());
                        format!(
                            "{} ok=true trace_id={} response={response}",
                            result.order_id, result.trace_id,
                        )
                    } else {
                        let detail = result
                            .error
                            .as_ref()
                            .and_then(|e| e.get("step"))
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("?");
                        let code = result
                            .error
                            .as_ref()
                            .and_then(|e| e.get("code"))
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("?");
                        format!(
                            "{} ok=false step={detail} code={code} trace_id={}",
                            result.order_id, result.trace_id,
                        )
                    };
                    lines.push(line);
                }
            }
            lines.join("\n")
        }
        other => {
            format!(
                "unknown blackboard section: {other} (expected plan|edits|tool_actions|exec|actions)"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{ActionBoard, ActionResult, PlanStep, SharedBlackboard, StepStatus};

    #[test]
    fn render_actions_section_shows_registration_order_and_results() {
        let mut board = ActionBoard::default();
        board.set_registration(vec![crate::blackboard::ActionRegistration {
            name: "workspace.read_file".into(),
            description: "read a file".into(),
            parameters: serde_json::json!({"required": ["target_file"]}),
        }]);
        board
            .write_order(crate::blackboard::ActionOrder {
                order_id: "ORD-000001".into(),
                action: "workspace.read_file".into(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                round: 0,
                plan_epoch: 1,
                run_id: "RUN-1".into(),
            })
            .unwrap();
        board.push_result(ActionResult {
            order_id: "ORD-000001".into(),
            ok: true,
            response: Some(serde_json::json!({"output": "hi"})),
            error: None,
            trace_id: "t000001".into(),
            timestamp: "2026-08-15T00:00:00Z".into(),
        });
        board.push_result(ActionResult {
            order_id: "ORD-000002".into(),
            ok: false,
            response: None,
            error: Some(serde_json::json!({
                "step": "policy",
                "code": "policy_denied",
                "message": "denied",
                "trace_id": "t000002",
            })),
            trace_id: "t000002".into(),
            timestamp: "2026-08-15T00:00:01Z".into(),
        });

        let plan = PlanSection::default();
        let text = render_section(
            &plan,
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
        );
        assert!(text.contains("== registration =="));
        assert!(text.contains("workspace.read_file"));
        assert!(text.contains("== order =="));
        assert!(text.contains("ORD-000001 action=workspace.read_file round=0 plan_epoch=1"));
        assert!(text.contains("== results =="));
        assert!(text.contains("ORD-000001 ok=true trace_id=t000001"));
        assert!(text.contains("ORD-000002 ok=false step=policy code=policy_denied"));

        // 未知分区显式报错并列出新分区。
        let unknown = render_section(
            &plan,
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "bogus",
            None,
        );
        assert!(unknown.contains("unknown blackboard section: bogus"));
        assert!(unknown.contains("actions"));
    }

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
        assert!(
            after_sweep >= next,
            "reuse after sweep: {after_sweep} < {next}"
        );
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
            goal: "旧步骤".into(),
            actions: Vec::new(),
            acceptance: String::new(),
            evidence: Vec::new(),
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
        // F2: the atomic write leaves only the final file — no `.json.tmp`
        // residue that could be mistaken for a snapshot.
        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".json.tmp"))
            .collect();
        assert!(leftovers.is_empty(), "tmp residue: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn latest_epoch_snapshot_falls_back_from_corrupt_highest() {
        let dir = std::env::temp_dir().join(format!(
            "orz-epoch-fallback-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut bb = crate::blackboard::Blackboard::new();
        bb.plan.plan_id = Some("PLAN-OLD".into());
        bb.plan.plan_epoch = 7;
        bb.edits.push(EditRecord {
            file: "old.py".into(),
            old_lines: 1,
            new_lines: 2,
            timestamp: "2026-08-14T00:00:00Z".into(),
        });
        let snapshot = bb.epoch_snapshot("2026-08-14T01:00:00Z");
        assert!(write_epoch_archive_retry(&dir, &snapshot));

        // A half-written/highest file (pre-F2 torn write or external
        // damage) must not break restore — the previous valid snapshot is
        // the fallback.
        std::fs::write(dir.join("epoch-8.json"), "{\"plan_epoch\": 8, ").unwrap();
        let restored = latest_epoch_snapshot(&dir).expect("falls back to valid snapshot");
        assert_eq!(restored.plan_epoch, 7);
        assert_eq!(restored.plan.plan_id.as_deref(), Some("PLAN-OLD"));

        // A stray `.json.tmp` (crash before rename) never becomes the
        // restore entry and never influences the next-epoch scan.
        std::fs::write(dir.join("epoch-999.json.tmp"), "{}").unwrap();
        assert_eq!(latest_epoch_snapshot(&dir).expect("loads").plan_epoch, 7);
        let next = next_plan_epoch_from_archive(&dir);
        assert!(next > 8, "tmp must not extend the epoch scan: {next}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn claim_epoch_is_atomic_and_never_reuses_reserved_numbers() {
        let dir = std::env::temp_dir().join(format!(
            "orz-epoch-claim-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        // Exactly one claimant wins per epoch number.
        assert!(claim_plan_epoch(&dir, 42));
        assert!(
            !claim_plan_epoch(&dir, 42),
            "second claimant of the same epoch must lose"
        );
        assert!(claim_plan_epoch(&dir, 43));

        // Claim files (including a crashed claimant's) reserve numbers in
        // the next-epoch scan — no reuse even in the same millisecond.
        let next = next_plan_epoch_from_archive(&dir);
        assert!(next > 43, "reserved epoch reused: {next}");

        // Retention-style sweep of stale claims (by age) is safe because
        // timestamp-stamped numbers are always smaller than future numbers.
        assert!(dir.join(".claim-42").is_file());
        assert!(dir.join(".claim-43").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_rotated_at_archives_still_load_as_persisted_at() {
        // F9 (2026-08-15): pre-rename archives carry `rotated_at`; the
        // serde alias keeps them loadable (the JSON key is `persisted_at`
        // for new writes).
        let old = r#"{
            "plan_id": "PLAN-OLD",
            "plan_epoch": 1,
            "plan": {
                "plan_id": "PLAN-OLD",
                "plan_epoch": 1,
                "goal": "旧任务",
                "steps": [],
                "analysis": [],
                "decisions": [],
                "auth_grants": []
            },
            "edits": [],
            "tool_actions": [],
            "exec": {
                "results": [],
                "observations": [],
                "errors": [],
                "auth_requests": []
            },
            "rotated_at": "2026-08-14T00:00:00Z"
        }"#;
        let snapshot: EpochSnapshot = serde_json::from_str(old).expect("alias loads old archives");
        assert_eq!(snapshot.persisted_at, "2026-08-14T00:00:00Z");

        // New writes use the corrected key.
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"persisted_at\""), "{json}");
        assert!(!json.contains("\"rotated_at\""), "{json}");
    }
}
