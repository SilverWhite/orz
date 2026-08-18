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
// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31）：actions 结果板固定形态行
// 与 exec 行截断复用 summary.rs 同口径辅助（D1=(c) 已确立的机械语义）。
use crate::summary::{failure_envelope_fields, truncate_chars};

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
                    crate::blackboard::StepStatus::Done(_) => "done",
                    crate::blackboard::StepStatus::Failed(_) => "failed",
                    crate::blackboard::StepStatus::Blocked => "blocked",
                };
                // P0-E 计划视图补渲染步骤 ID (2026-08-17, ADR-0010 §14.21 项 2):
                // 步骤门要求 console 订单 step_id 精确绑定当前可执行步骤；渲染
                // 每步 id 作为行首标识，模型从计划视图直接取用，无需猜测。
                // 本函数同时服务 live 视图与归档 epoch 读（controller 同源）。
                lines.push(format!(
                    "- [{status}] {}: {} (actions: {}; evidence: {})",
                    step.id,
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
            // 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.2）：
            // - 每条结果/错误行按字符截断到 200 字符（复用 summary.rs
            //   truncate_chars 口径：199 字符 + 「…」）；
            // - EXEC_RENDER_CAP=50 保留（较早条目省略 + 头行）；
            // - 段总长上限 4K 字符（约 2K token）：超限时明细行整体省略，
            //   仅保留头行 + 计数行（与 actions 段计数行同 bracket 风格），
            //   完整内容仍可经 blackboard_read 分区/存档回查。
            const EXEC_RENDER_CAP: usize = 50;
            const EXEC_LINE_MAX_CHARS: usize = 200;
            const EXEC_SECTION_MAX_CHARS: usize = 4_000;

            let total = exec.results.len() + exec.errors.len();
            if total == 0 {
                return "(no exec results yet)".to_string();
            }
            let mut lines: Vec<String> = exec
                .results
                .iter()
                .chain(exec.errors.iter())
                .map(|s| truncate_chars(s, EXEC_LINE_MAX_CHARS))
                .collect();
            if lines.len() > EXEC_RENDER_CAP {
                let omitted = lines.len() - EXEC_RENDER_CAP;
                lines.drain(0..omitted);
                lines.insert(
                    0,
                    format!(
                        "[exec: 共 {total} 条，仅显示最近 {EXEC_RENDER_CAP} 条（较早条目省略 {omitted} 条）]"
                    ),
                );
            }
            let mut text = lines.join("\n");
            if text.chars().count() > EXEC_SECTION_MAX_CHARS {
                let omitted = total.saturating_sub(EXEC_RENDER_CAP);
                let head = if omitted > 0 {
                    format!(
                        "[exec: 共 {total} 条，仅显示最近 {EXEC_RENDER_CAP} 条（较早条目省略 {omitted} 条）]"
                    )
                } else {
                    format!("[exec: 共 {total} 条，未省略；明细超 4K 字符上限]")
                };
                text = format!(
                    "{head}\n[exec: 全部省略（共 {total} 条）；完整内容见 blackboard_read 分区 exec 与存档]"
                );
            }
            text
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
                    // 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.1）：
                    // 结果板不再嵌入 response JSON / 完整 error 对象——每条
                    // 固定形态（约 ≤90 字符）：
                    //   <order_id> ok=<bool> step=<step|?> code=<code|?>
                    //   trace_id=<trace_id>
                    // step/code 取信封既有字段，缺失回退 `?`（与 D1=(c)
                    // failure_envelope_fields 同口径）；大载荷留在存档与
                    // TraceStore，模型按需经 trace 回查。
                    let (step, code) = failure_envelope_fields(&result.error);
                    let line = format!(
                        "{} ok={} step={} code={} trace_id={}",
                        result.order_id, result.ok, step, code, result.trace_id,
                    );
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
                step_id: None,
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
        // 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.1）：结果板固定
        // 形态行——成功/失败 receipt 均不再嵌入 response JSON / 完整 error
        // 对象；step/code 缺失回退 `?`。
        assert!(text.contains("ORD-000001 ok=true step=? code=? trace_id=t000001"));
        assert!(
            text.contains("ORD-000002 ok=false step=policy code=policy_denied trace_id=t000002")
        );
        assert!(
            !text.contains("response="),
            "result lines must not embed response JSON: {text}"
        );
        assert!(
            !text.contains("\"output\""),
            "result lines must not embed response JSON: {text}"
        );

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

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.1）：actions 结果
    /// 板瘦身——大 response JSON / 完整 error 对象不再进渲染面；每条固定
    /// 形态 `<order_id> ok=<bool> step=<step|?> code=<code|?> trace_id=...`
    /// （缺失回退 `?`，与 D1=(c) failure_envelope_fields 同口径）。
    #[test]
    fn render_actions_results_slim_fixed_shape_no_response_json() {
        let mut board = ActionBoard::default();
        board.push_result(ActionResult {
            order_id: "ORD-OK-1".into(),
            ok: true,
            response: Some(serde_json::json!({
                "output": "x".repeat(10_000),
                "nested": {"deep": "y".repeat(5_000)},
            })),
            error: None,
            trace_id: "t-ok-1".into(),
            timestamp: "2026-08-19T00:00:00Z".into(),
        });
        board.push_result(ActionResult {
            order_id: "ORD-ERR-1".into(),
            ok: false,
            response: None,
            error: Some(serde_json::json!({
                "step": "execute",
                "code": "boom",
                "message": "m".repeat(5_000),
            })),
            trace_id: "t-err-1".into(),
            timestamp: "2026-08-19T00:00:01Z".into(),
        });
        // 信封字段整体缺失 → step/code 机械回退 `?`（不编造、不隐藏）。
        board.push_result(ActionResult {
            order_id: "ORD-ERR-2".into(),
            ok: false,
            response: None,
            error: None,
            trace_id: "t-err-2".into(),
            timestamp: "2026-08-19T00:00:02Z".into(),
        });

        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
        );
        assert!(
            text.contains("ORD-OK-1 ok=true step=? code=? trace_id=t-ok-1"),
            "{text}"
        );
        assert!(
            text.contains("ORD-ERR-1 ok=false step=execute code=boom trace_id=t-err-1"),
            "{text}"
        );
        assert!(
            text.contains("ORD-ERR-2 ok=false step=? code=? trace_id=t-err-2"),
            "{text}"
        );
        // 大载荷一律不进渲染面。
        assert!(!text.contains("xxxx"), "{text}");
        assert!(!text.contains("mmmm"), "{text}");
        assert!(!text.contains("response="), "{text}");
        assert!(!text.contains("nested"), "{text}");
        // 每条结果行有界（固定形态约 ≤90 字符，宽松断言 ≤200）。
        for line in text.lines() {
            assert!(line.chars().count() <= 200, "line too long: {line}");
        }
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.1）：RESULTS_RENDER_CAP
    /// =10 与总数头行保留——50 条 receipt 只渲染最近 10 条，头行机械可数。
    #[test]
    fn render_actions_results_caps_at_latest_10_with_count_line() {
        let mut board = ActionBoard::default();
        for i in 0..50 {
            board.push_result(ActionResult {
                order_id: format!("ORD-{i:03}"),
                ok: true,
                response: Some(serde_json::json!({"output": format!("payload-{i}")})),
                error: None,
                trace_id: format!("t-{i:03}"),
                timestamp: format!("2026-08-19T00:{i:02}:00Z"),
            });
        }
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
        );
        assert!(
            text.contains("[actions: 共 50 条，仅显示最近 10 条（较早省略 40 条）]"),
            "{text}"
        );
        // 最近 10 条 = ORD-040..ORD-049；更早的 receipt 不渲染。
        assert!(
            text.contains("ORD-049 ok=true step=? code=? trace_id=t-049"),
            "{text}"
        );
        assert!(
            text.contains("ORD-040 ok=true step=? code=? trace_id=t-040"),
            "{text}"
        );
        assert!(!text.contains("ORD-039 ok="), "{text}");
        assert!(!text.contains("ORD-000 ok="), "{text}");
        // 瘦身后 response 载荷一律不出现。
        assert!(!text.contains("payload-"), "{text}");
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.2）：exec 每条
    /// 结果/错误行按字符截断到 200 字符（199 字符 + 「…」，复用 summary.rs
    /// truncate_chars 口径）。
    #[test]
    fn render_exec_truncates_lines_to_200_chars() {
        let mut exec_section = ExecSection::default();
        exec_section.results.push(format!("ok {}", "x".repeat(500)));
        exec_section.errors.push(format!("err {}", "y".repeat(500)));
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &exec_section,
            &ActionBoard::default(),
            "exec",
            None,
        );
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        // 每条截断到 200 字符：199 字符 + 「…」。
        for line in &lines {
            assert_eq!(line.chars().count(), 200, "line: {line}");
            assert!(line.ends_with('…'), "line must end with ellipsis: {line}");
        }
        assert!(lines[0].starts_with("ok xxx"), "{text}");
        assert!(lines[1].starts_with("err yyy"), "{text}");
        // 原文尾部不得残留。
        assert!(!text.contains("xxx00"), "{text}");
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.2）：EXEC_RENDER_CAP
    /// =50 保留——60 条短行只渲染最近 50 条（较早条目省略 + 头行），段总长
    /// 仍在 4K 上限内则保留明细。
    #[test]
    fn render_exec_caps_at_latest_50_with_head_line() {
        let mut exec_section = ExecSection::default();
        for i in 0..60 {
            exec_section.results.push(format!("line-{i:02} run"));
        }
        exec_section.errors.push("short error".into());
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &exec_section,
            &ActionBoard::default(),
            "exec",
            None,
        );
        let lines: Vec<&str> = text.lines().collect();
        // 头行 + 最近 50 条（结果 line-11..line-59 + 错误 short error）。
        assert_eq!(lines.len(), 51, "{text}");
        assert!(
            lines[0].starts_with("[exec: 共 61 条，仅显示最近 50 条（较早条目省略 11 条）]"),
            "{text}"
        );
        assert!(text.contains("line-11 run"), "{text}");
        assert!(text.contains("line-59 run"), "{text}");
        assert!(text.contains("short error"), "{text}");
        assert!(!text.contains("line-00 run"), "{text}");
        assert!(!text.contains("line-10 run"), "{text}");
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.2）：段总长上限
    /// 4K 字符（约 2K token）——明细行整体省略，仅保留头行 + 计数行（与
    /// actions 段计数行同 bracket 风格），完整内容仍可经分区/存档回查。
    #[test]
    fn render_exec_section_caps_total_chars_at_4k() {
        // ① 50 条超长行：行数未超 EXEC_RENDER_CAP，但逐行截断后仍远超 4K。
        let mut exec_section = ExecSection::default();
        for _ in 0..50 {
            exec_section.results.push("长".repeat(500));
        }
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &exec_section,
            &ActionBoard::default(),
            "exec",
            None,
        );
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(
            text.starts_with("[exec: 共 50 条，未省略；明细超 4K 字符上限]"),
            "{text}"
        );
        assert!(
            text.contains("完整内容见 blackboard_read 分区 exec 与存档"),
            "{text}"
        );
        assert!(!text.contains("长长"), "{text}");

        // ② 60 条超长行：50 上限先触发（省略 10 条），段总长仍超限 →
        // 保留头行（含省略计数）+ 计数行。
        let mut exec_section = ExecSection::default();
        for _ in 0..60 {
            exec_section.errors.push("错".repeat(500));
        }
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &exec_section,
            &ActionBoard::default(),
            "exec",
            None,
        );
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(
            text.starts_with("[exec: 共 60 条，仅显示最近 50 条（较早条目省略 10 条）]"),
            "{text}"
        );
        assert!(
            text.contains("完整内容见 blackboard_read 分区 exec 与存档"),
            "{text}"
        );
        assert!(!text.contains("错错"), "{text}");
    }

    /// P0-E 计划视图补渲染步骤 ID (2026-08-17, ADR-0010 §14.21 项 2): every
    /// plan step line carries its id as the leading token after the status —
    /// the model echoes it into `step_id` when writing console orders, no
    /// guessing. The same renderer serves live views and archived epoch
    /// reads, so one test locks both surfaces.
    #[test]
    fn render_plan_section_includes_step_ids() {
        use crate::blackboard::PlanAction;
        let mut plan = crate::blackboard::PlanSection {
            plan_id: Some("PLAN-VIEW-1".into()),
            plan_epoch: 3,
            goal: Some("构建 ELF".into()),
            ..Default::default()
        };
        plan.steps.push(PlanStep {
            id: "s1".into(),
            goal: "侦查源码".into(),
            actions: vec![PlanAction {
                step_id: "s1".into(),
                do_action: "workspace.list_dir".into(),
                with: serde_json::json!({"path": "."}),
            }],
            acceptance: "清单".into(),
            evidence: vec!["tree.txt".into()],
            status: StepStatus::InProgress,
        });
        plan.steps.push(PlanStep {
            id: "s2".into(),
            goal: "构建并验证".into(),
            actions: Vec::new(),
            acceptance: String::new(),
            evidence: Vec::new(),
            status: StepStatus::Pending,
        });

        let text = render_section(
            &plan,
            &[],
            &[],
            &ExecSection::default(),
            &ActionBoard::default(),
            "plan",
            None,
        );
        assert!(text.contains("goal: 构建 ELF"), "{text}");
        assert!(text.contains("plan_id: PLAN-VIEW-1"), "{text}");
        assert!(text.contains("plan_epoch: 3"), "{text}");
        // The step id is the leading token after the status bracket —
        // exactly what the model echoes into `step_id`.
        assert!(
            text.contains("- [in-progress] s1: 侦查源码 (actions: 1; evidence: 1)"),
            "{text}"
        );
        assert!(
            text.contains("- [pending] s2: 构建并验证 (actions: 0; evidence: 0)"),
            "{text}"
        );

        // Empty plan stays explicit about having no steps.
        let empty = render_section(
            &crate::blackboard::PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &ActionBoard::default(),
            "plan",
            None,
        );
        assert!(empty.contains("(no steps)"), "{empty}");
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
