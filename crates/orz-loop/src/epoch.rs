//! Blackboard plan-epoch archives (ADR-0010 §14.15 / v1.15, 2026-08-14;
//! v1.15⑧ 补强 2026-08-15: timestamp-stamped monotonic epoch numbers).
//!
//! P2-13（2026-09-03，ADR-0010 §14.52）：生产黑板生命周期已切换为会话作用域
//! （B1 会话化基础 + B2 渲染折叠，见下）；本文件的 plan-epoch 归档面仅剩
//! `--plan` 诊断/测试路径（B3 已随 P2-13 实施批完成生产面退役：生产入口
//! 不配置归档目录、不触发 with_plan/rotate/claim，`.gsa/blackboard/
//! epoch-*.json` 生产面不再写入；marker `plan_epoch` 行与 blackboard_read
//! `epoch` 参数均已收口，仅 `--plan`/测试域保留归档读）。
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

use orz_assurance::lif::Domain;

use crate::blackboard::{
    ActionBoard, ActionResult, Blackboard, EditRecord, EpochSnapshot, ExecSection,
    ExternalRetSection, InternalRetSection, PlanSection, ToolActionRecord,
};
// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31）：actions 结果板固定形态行
// 与 exec 行截断复用 summary.rs 同口径辅助（D1=(c) 已确立的机械语义）。
use crate::summary::{failure_envelope_fields, truncate_chars};
// P2-13 B2 渲染折叠（2026-09-03）：纯函数核心在 render_fold（分段/展开子集/
// 参数定档），本模块组装各分区行文本。
use crate::render_fold::{self, FoldExpand, FoldParams, FoldRowMeta, SegmentKind, SegmentRun};

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
///
/// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B 按需点读）：
/// `receipt_id` = 结果栏单条 receipt 的 order_id 点读（仅与 `section=actions`
/// 组合有效；与 `epoch` 组合 = 归档快照点读；与 `since` 同时给 = 忽略 since——
/// 点读按 id 寻址，时间过滤不适用）；无 `receipt_id` 时输出与 S1 逐字节一致。
#[allow(clippy::too_many_arguments)] // 分区渲染签名 = 5 分区 + section/since + 方案B 点读参数（与 run_turn 同纪律）
pub fn render_section(
    plan: &PlanSection,
    edits: &[EditRecord],
    tool_actions: &[ToolActionRecord],
    exec: &ExecSection,
    actions: &ActionBoard,
    section: &str,
    since: Option<&str>,
    receipt_id: Option<&str>,
) -> String {
    // 方案 B 参数组合守卫：receipt_id 仅对 actions 分区有效；非 actions 分区
    // 携带 receipt_id = 显式报错（fail loud，同未知分区风格），绝不静默忽略。
    if receipt_id.is_some() && section != "actions" {
        return format!(
            "receipt_id 仅与 section=actions 组合有效（点读结果栏单条 receipt）；\
             当前 section={section} 不支持 receipt_id"
        );
    }
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
            // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the mechanical
            // delivery status line (harness-computed workspace change list;
            // tool output — rendered by `submit`, cleared on rotation).
            if let Some(status) = &plan.delivery_status {
                lines.push(status.clone());
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
                // B2 补齐渲染 cap（2026-09-03）：edits 此前无硬上限，
                // conversation 轴后必须与 exec 同口径有界（超限截断 + 指针）。
                render_capped_rows("edits", lines)
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
                // B2 补齐渲染 cap（同 edits 理由；tool_actions 此前无上限）。
                render_capped_rows("tool_actions", lines)
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
                // B1（2026-09-03）：exec 行结构化为 ExecEntry——渲染只取
                // 文本载荷；round/domain/ts 章供 B2 折叠/展开消费（本
                // 阶段渲染口径逐字节不变）。
                .map(|entry| truncate_chars(&entry.text, EXEC_LINE_MAX_CHARS))
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
            // 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B）：
            // 按需点读优先——receipt_id 给定即返回单条 receipt 的固定形态行 +
            // 有界完整内容（response/error 全文），整段渲染不参与（避免大载荷
            // 再次注入消息面）。
            if let Some(receipt_id) = receipt_id {
                return render_receipt_point_read(actions, receipt_id, plan.plan_epoch);
            }
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
                    // TraceStore；模型按需经 blackboard_read receipt_id
                    // 点读回查（方案 B，ADR-0010 §14.31 / 设计 §4.5）。
                    let (step, code) = failure_envelope_fields(&result.error);
                    // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.3): the
                    // resident board adds only a short `changed: N files`
                    // count (cache-hit-rate discipline); the full diff/delta
                    // stays in the receipt point-read.
                    let changed = result
                        .response
                        .as_ref()
                        .and_then(|r| {
                            r.get("workspace_delta")
                                .and_then(|d| d.as_array())
                                .map(|a| a.len())
                        })
                        .or_else(|| {
                            result
                                .response
                                .as_ref()
                                .and_then(|r| r.get("diff"))
                                .map(|_| 1)
                        });
                    // AGENT-DELIVERY-FLOW (2026-08-23, 审查处理 O6): when
                    // the host truncated the per-call delta, the resident
                    // count shows the capped entries with a `+` marker so
                    // the short count never reads as the true total (the
                    // receipt point-read carries the truncation flag).
                    let delta_truncated = result
                        .response
                        .as_ref()
                        .and_then(|r| r.get("workspace_delta_truncated"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let changed_str = changed
                        .map(|n| {
                            format!(
                                " changed: {}{} files",
                                n,
                                if delta_truncated { "+" } else { "" }
                            )
                        })
                        .unwrap_or_default();
                    let line = format!(
                        "{} ok={} step={} code={} trace_id={}{}",
                        result.order_id, result.ok, step, code, result.trace_id, changed_str,
                    );
                    lines.push(line);
                }
            }
            lines.join("\n")
        }
        other => {
            format!(
                "unknown blackboard section: {other} (expected \
                 plan|edits|tool_actions|exec|actions|session|internal_ret|external_ret; \
                 session 面是 live 会话状态、由 controller 直接渲染，不进归档；\
                 internal_ret / external_ret 是 live 检索分区、不进 epoch 归档)"
            )
        }
    }
}

// ===========================================================================
// P2-13 B2 渲染折叠（2026-09-03，设计 §9/§11/§12，ADR-0010 §14.52）
// ===========================================================================
//
// 方案 B：可折叠分区（exec / edits / tool_actions）的 live 读取在触发折叠态
// （分区 live 字符 ≥ T 或整板 live 字节 ≥ W）或携带显式展开参数时走本组
// 渲染；其余路径（含归档 epoch 读）保持逐字节不变。存储零改写、无事件。
//
// 折叠视图输出 = 默认展开子集（当前域段 ∪ 最近 K 轮 ∪ 最近 20% 行）+ 显式
// 展开目标行；其余行按域段聚合为标注行。标注行/行文本受 FOLDABLE_* 上限
// 约束（edits/tool_actions 由此补齐渲染 cap——B2 排期项，与 exec 同口径）。

/// 折叠视图/补齐 cap：单行截断上限（与 exec 行 200 字符同口径）。
pub const FOLDABLE_ROW_MAX_CHARS: usize = 200;
/// 补齐 cap：单分区可见行上限（与 exec 50 条同口径）。
pub const FOLDABLE_SECTION_ROW_CAP: usize = 50;
/// 补齐 cap：单分区可见字符上限（与 exec 4K 同口径）。
pub const FOLDABLE_SECTION_MAX_CHARS: usize = 4_000;
/// 折叠标注行长度上限（标注是总览行，必须紧凑）。
pub const FOLD_ANNOTATION_MAX_CHARS: usize = 160;

/// 单行 r 区间显示：`r1` / `r1–r30`（设计标注格式，含边界）。
fn round_span(first: u64, last: u64) -> String {
    if first == last {
        format!("r{first}")
    } else {
        format!("r{first}–r{last}")
    }
}

/// RFC 3339 since 过滤闭包（与 render_section 同口径：非法/不可解析回退
/// 不过滤，绝不因过滤边界隐藏记录）。
fn since_filter(since: Option<&str>) -> impl Fn(&str) -> bool {
    let since_dt = since.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
    move |ts: &str| -> bool {
        match since_dt {
            None => true,
            Some(dt) => chrono::DateTime::parse_from_rfc3339(ts)
                .map(|t| t >= dt)
                .unwrap_or(true),
        }
    }
}

/// B2 折叠触发计量（§11.1 T 的字符口径落地）：分区 live 全量行渲染估算
/// 字符 = Σ(行文本字符 + 行分隔 1)，行文本取未截断的渲染形态。T=64K 量级
/// 阈值下，估算口径的少量偏差不影响触发判定（实现决策，登记于 B2 审计）。
pub fn live_foldable_partition_chars(section: &str, bb: &Blackboard) -> usize {
    let mut chars = 0usize;
    match section {
        "edits" => {
            for r in &bb.edits {
                chars += r.timestamp.chars().count()
                    + 1
                    + crate::controller::format_edit_record(r).chars().count()
                    + 1;
            }
        }
        "tool_actions" => {
            for r in &bb.tool_actions {
                chars += r.timestamp.chars().count() + 1 + r.tool.chars().count() + 1;
            }
        }
        "exec" => {
            for e in bb.exec.results.iter().chain(bb.exec.errors.iter()) {
                chars += e.text.chars().count() + 1;
            }
        }
        _ => {}
    }
    chars
}

/// 渲染 cap 辅助（edits/tool_actions 补齐；exec 既有分支保持原样以保证
/// 逐字节不变）：行数超 `FOLDABLE_SECTION_ROW_CAP` 时只留最近行并加头行；
/// 字符超 `FOLDABLE_SECTION_MAX_CHARS` 时明细整体省略（同 exec 头行 +
/// 计数行纪律）。`label` = 分区名（edits/tool_actions）。
fn render_capped_rows(label: &str, lines: Vec<String>) -> String {
    if lines.is_empty() {
        return format!("(no {label} yet)");
    }
    let mut lines: Vec<String> = lines
        .into_iter()
        .map(|l| truncate_chars(&l, FOLDABLE_ROW_MAX_CHARS))
        .collect();
    let total = lines.len();
    let omitted = lines.len().saturating_sub(FOLDABLE_SECTION_ROW_CAP);
    if omitted > 0 {
        lines.drain(0..omitted);
        lines.insert(
            0,
            format!(
                "[{label}: 共 {total} 条，仅显示最近 {FOLDABLE_SECTION_ROW_CAP} 条（较早条目省略 {omitted} 条）]"
            ),
        );
    }
    let mut text = lines.join("\n");
    if text.chars().count() > FOLDABLE_SECTION_MAX_CHARS {
        let head = if omitted > 0 {
            format!(
                "[{label}: 共 {total} 条，仅显示最近 {FOLDABLE_SECTION_ROW_CAP} 条（较早条目省略 {omitted} 条）]"
            )
        } else {
            format!(
                "[{label}: 共 {total} 条，未省略；明细超 {FOLDABLE_SECTION_MAX_CHARS} 字符上限]"
            )
        };
        text = format!(
            "{head}\n[{label}: 全部省略（共 {total} 条）；完整内容见 \
             blackboard_read 分区 {label} 与存档]"
        );
    }
    text
}

/// 折叠视图总上限：超 `FOLDABLE_SECTION_MAX_CHARS` 时优先保留显式展开目标
/// 行（`protected` 标记，B2 复审：绝不静默丢失请求行；目标自身超上限时
/// 显式提示缩小范围），其余内容从最旧整行开始丢弃、保留最近部分；被丢
/// 内容永远可经展开参数/分区全文回查，不丢失存储。无保护行时行为与 B2
/// 初版一致（最近优先 + 头行说明）。
pub(crate) fn cap_fold_view(
    section: &str,
    rows_total: usize,
    segments_total: usize,
    lines: Vec<String>,
    protected: &[bool],
) -> String {
    let full = lines.join("\n");
    if full.chars().count() <= FOLDABLE_SECTION_MAX_CHARS {
        return full;
    }
    let has_protected = protected.iter().any(|p| *p);
    let head = if has_protected {
        format!(
            "[{section}: 折叠视图 {rows_total} 条记录 / {segments_total} 个域段；\
             超 {FOLDABLE_SECTION_MAX_CHARS} 字符上限：优先保留展开目标行，\
             其余保留最近内容（可用 domain+round_from/round_to 精确展开回查）]"
        )
    } else {
        format!(
            "[{section}: 折叠视图 {rows_total} 条记录 / {segments_total} 个域段；\
             最早内容超 {FOLDABLE_SECTION_MAX_CHARS} 字符上限已省略（可用 \
             domain+round_from/round_to 精确展开回查）]"
        )
    };
    let budget = FOLDABLE_SECTION_MAX_CHARS.saturating_sub(head.chars().count() + 1);
    let mut kept: Vec<String> = Vec::new();
    let mut used = 0usize;
    // 1) 显式展开目标行优先（按组装序，不被“最近优先”裁掉）。
    if has_protected {
        let mut dropped_target = 0usize;
        for (line, prot) in lines.iter().zip(protected) {
            if !*prot {
                continue;
            }
            let add = line.chars().count() + 1;
            if used + add > budget {
                dropped_target += 1;
                continue;
            }
            used += add;
            kept.push(line.clone());
        }
        if dropped_target > 0 {
            // 目标自身超上限：显式 fail loud（绝不静默空回）。
            kept.push(format!(
                "（展开目标另有 {dropped_target} 行因超 {FOLDABLE_SECTION_MAX_CHARS} \
                 字符视图上限未显示——请缩小轮数范围分批展开）"
            ));
            return format!("{head}\n{}", kept.join("\n"));
        }
    }
    // 2) 其余内容从最旧（行首）开始丢：倒序累积最近行，再反转保持时间序。
    let mut keep_rev: Vec<String> = Vec::new();
    for (line, prot) in lines.iter().zip(protected).rev() {
        if *prot {
            continue;
        }
        let add = line.chars().count() + 1;
        if used + add > budget {
            break;
        }
        used += add;
        keep_rev.push(line.clone());
    }
    keep_rev.reverse();
    kept.extend(keep_rev);
    if kept.is_empty() {
        head
    } else {
        format!("{head}\n{}", kept.join("\n"))
    }
}

/// 折叠段标注行（共用格式；`preview` 已由各分区组装并截断）。
fn segment_annotation(
    kind: SegmentKind,
    first: u64,
    last: u64,
    count: usize,
    preview: String,
) -> String {
    let line = match kind {
        // R1：pre-stamp 段只给计数 + 时间范围，无轮号区间。
        SegmentKind::PreStamp => {
            format!("[域段 pre-stamp（旧行无章） · {count} 条 · {preview}]")
        }
        SegmentKind::Domain(d) => format!(
            "[域段 {} {} · {count} 条 · {preview}]",
            d.as_str(),
            round_span(first, last)
        ),
    };
    truncate_chars(&line, FOLD_ANNOTATION_MAX_CHARS)
}

/// 组装折叠视图行（B2 复审 2026-09-03 统一三段共用的装配逻辑）：
/// - 标注行落在该段**首个折叠行**的位置（段内“展开前缀 + 折叠尾/中”时
///   标注随行序后移，保持时间序；B2 初版固定放段首，复审修正）；
/// - 显式展开目标行（`explicit_target`）标记 protected——超 4K 上限时
///   优先保留，绝不静默丢失请求行；
/// - `preview` 由各分区按自身记录语义提供（edits=路径 / tool_actions=
///   类别计数 / exec=文本预览；pre-stamp=时间范围），`folded_idx` 为段内
///   折叠行索引（升序，非空）。
fn fold_view_lines(
    rows: &[FoldRowMeta],
    segments: &[SegmentRun],
    expanded: &[bool],
    explicit_target: &[bool],
    preview: impl Fn(SegmentKind, &[usize]) -> String,
    no_match_note: Option<String>,
) -> (Vec<String>, Vec<bool>) {
    let mut lines: Vec<String> = Vec::new();
    let mut protected: Vec<bool> = Vec::new();
    for seg in segments {
        let folded_idx: Vec<usize> = (seg.start..seg.end).filter(|&i| !expanded[i]).collect();
        let annotation = if folded_idx.is_empty() {
            None
        } else {
            let mut lo = u64::MAX;
            let mut hi = 0u64;
            for &i in &folded_idx {
                if rows[i].round >= 1 {
                    lo = lo.min(rows[i].round);
                    hi = hi.max(rows[i].round);
                }
            }
            let (lo, hi) = if lo == u64::MAX {
                (0u64, 0u64)
            } else {
                (lo, hi)
            };
            Some(segment_annotation(
                seg.kind,
                lo,
                hi,
                folded_idx.len(),
                preview(seg.kind, &folded_idx),
            ))
        };
        let first_folded = folded_idx.first().copied();
        match first_folded {
            // 无折叠行：全段展开行直接顺序输出。
            None => {
                for i in seg.start..seg.end {
                    if expanded[i] {
                        lines.push(rows[i].text.clone());
                        protected.push(explicit_target[i]);
                    }
                }
            }
            // 有折叠行：标注落在首个折叠行位置（首折叠行必为折叠态，随后的
            // 折叠行不再重复输出），展开行按行序穿插。
            Some(first) => {
                let ann = annotation.expect("first folded row implies annotation");
                for i in seg.start..seg.end {
                    if expanded[i] {
                        lines.push(rows[i].text.clone());
                        protected.push(explicit_target[i]);
                    } else if i == first {
                        lines.push(ann.clone());
                        protected.push(false);
                    }
                }
            }
        }
    }
    if let Some(note) = no_match_note {
        lines.push(note);
        protected.push(false);
    }
    (lines, protected)
}

/// 显式展开目标行标记：命中 `domain`+轮数范围的行在折叠视图中受 4K 上限
/// 保护（含已属默认展开子集的行——默认行也可能被“最近优先”cap 裁掉）。
fn explicit_target_flags(rows: &[FoldRowMeta], expand: Option<&FoldExpand>) -> Vec<bool> {
    match expand {
        Some(q) => rows.iter().map(|r| q.matches(r.round, r.domain)).collect(),
        None => vec![false; rows.len()],
    }
}

/// 展开无匹配提示（仅折叠态：域段不存在 / 轮数范围外 / pre-stamp 无轮号）。
fn no_match_expand_note(expand: Option<&FoldExpand>, matched: bool) -> Option<String> {
    match expand {
        Some(q) if !matched => Some(format!(
            "（展开目标 {} r{}–r{} 无匹配行——域段不存在或轮数范围外；\
             pre-stamp 旧行无轮号，只能用 since/receipt_id 点读）",
            q.domain.as_str(),
            q.round_from,
            q.round_to
        )),
        _ => None,
    }
}

// ---- P2-14 S1：三段共用标注预览（render_*_folded 与快照入口同源） ----

/// edits 段标注预览：pre-stamp = 时间范围（折叠子集内首末时间戳）；带章段
/// = 折叠子集内最新两条文件路径（Top-2 路径口径，与既有折叠视图一致）。
fn edits_segment_preview(recs: &[&EditRecord], kind: SegmentKind, folded_idx: &[usize]) -> String {
    match kind {
        SegmentKind::PreStamp => {
            let first_ts = recs[folded_idx[0]].timestamp.as_str();
            let last_ts = recs[folded_idx[folded_idx.len() - 1]].timestamp.as_str();
            if first_ts.is_empty() && last_ts.is_empty() {
                "无时间戳".to_string()
            } else {
                format!("时间 {first_ts}–{last_ts}")
            }
        }
        SegmentKind::Domain(d) => {
            let mut files: Vec<&str> = Vec::new();
            for &i in folded_idx.iter().rev() {
                let f = recs[i].file.as_str();
                if !files.contains(&f) {
                    files.push(f);
                    if files.len() == 2 {
                        break;
                    }
                }
            }
            if files.is_empty() {
                d.as_str().to_string()
            } else {
                format!("路径 {}", files.join("；"))
            }
        }
    }
}

/// tool_actions 段标注预览：折叠子集内类别计数 Top-2（read/edit/terminal/
/// retrieval/other 序，stable 同计数保序）。
fn tool_actions_segment_preview(
    recs: &[&ToolActionRecord],
    kind: SegmentKind,
    folded_idx: &[usize],
) -> String {
    match kind {
        SegmentKind::PreStamp => {
            let first_ts = recs[folded_idx[0]].timestamp.as_str();
            let last_ts = recs[folded_idx[folded_idx.len() - 1]].timestamp.as_str();
            if first_ts.is_empty() && last_ts.is_empty() {
                "无时间戳".to_string()
            } else {
                format!("时间 {first_ts}–{last_ts}")
            }
        }
        SegmentKind::Domain(_) => {
            let category_rank = ["read", "edit", "terminal", "retrieval", "other"];
            let mut counts: Vec<(&str, usize)> = Vec::new();
            for cat in category_rank {
                let n = folded_idx
                    .iter()
                    .filter(|&&i| recs[i].category == cat)
                    .count();
                if n > 0 {
                    counts.push((cat, n));
                }
            }
            counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
            let top: Vec<String> = counts
                .into_iter()
                .take(2)
                .map(|(c, n)| format!("{c}×{n}"))
                .collect();
            if top.is_empty() {
                "无类别计数".to_string()
            } else {
                top.join("，")
            }
        }
    }
}

/// exec 段标注预览：pre-stamp = 时间范围；带章段 = 折叠子集内最新两条
/// 文本预览（“摘要预览”口径，去重、非空）。
fn exec_segment_preview(
    rows: &[FoldRowMeta],
    recs: &[&crate::blackboard::ExecEntry],
    kind: SegmentKind,
    folded_idx: &[usize],
) -> String {
    match kind {
        SegmentKind::PreStamp => {
            let first_ts = recs[folded_idx[0]].ts.as_str();
            let last_ts = recs[folded_idx[folded_idx.len() - 1]].ts.as_str();
            if first_ts.is_empty() && last_ts.is_empty() {
                "无时间戳".to_string()
            } else {
                format!("时间 {first_ts}–{last_ts}")
            }
        }
        SegmentKind::Domain(_) => {
            let mut previews: Vec<String> = Vec::new();
            for &i in folded_idx.iter().rev() {
                let t = rows[i].text.trim();
                if !t.is_empty() && !previews.iter().any(|p| p == t) {
                    previews.push(t.to_string());
                    if previews.len() == 2 {
                        break;
                    }
                }
            }
            if previews.is_empty() {
                "（无文本预览）".to_string()
            } else {
                previews.join("；")
            }
        }
    }
}

/// B2 edits 折叠视图（live；仅折叠态/显式展开时调用）。`records` 为分区
/// 全量（since 在此过滤，语义同非折叠分支）；pre-stamp 标注带时间范围
/// （R1），带章段标注带最新两条文件路径（Top-N 路径口径，实现决策登记于
/// B2 审计）。
pub fn render_edits_folded(
    records: &[EditRecord],
    since: Option<&str>,
    current_round: u64,
    current_domain: Domain,
    params: &FoldParams,
    expand: Option<&FoldExpand>,
) -> String {
    let filter = since_filter(since);
    let recs: Vec<&EditRecord> = records.iter().filter(|r| filter(&r.timestamp)).collect();
    if recs.is_empty() {
        return "(no edit records)".to_string();
    }
    let rows: Vec<FoldRowMeta> = recs
        .iter()
        .map(|r| FoldRowMeta {
            text: truncate_chars(
                &format!(
                    "{} {}",
                    r.timestamp,
                    crate::controller::format_edit_record(r)
                ),
                FOLDABLE_ROW_MAX_CHARS,
            ),
            round: r.round,
            domain: r.domain,
        })
        .collect();
    let layout = render_fold::fold_layout(&rows, current_round, current_domain, params);
    let expanded = match expand {
        Some(q) => render_fold::merge_expand(&layout, &rows, q),
        None => layout.expanded.clone(),
    };
    let explicit_target = explicit_target_flags(&rows, expand);
    let no_match_note = no_match_expand_note(expand, explicit_target.iter().any(|b| *b));
    let (lines, protected) = fold_view_lines(
        &rows,
        &layout.segments,
        &expanded,
        &explicit_target,
        |kind, folded_idx| edits_segment_preview(&recs, kind, folded_idx),
        no_match_note,
    );
    cap_fold_view(
        "edits",
        recs.len(),
        layout.segments.len(),
        lines,
        &protected,
    )
}

/// B2 tool_actions 折叠视图（live）。标注带折叠段内类别计数 Top-2
/// （read/edit/terminal/retrieval/other，实现决策登记于 B2 审计）。
pub fn render_tool_actions_folded(
    records: &[ToolActionRecord],
    since: Option<&str>,
    current_round: u64,
    current_domain: Domain,
    params: &FoldParams,
    expand: Option<&FoldExpand>,
) -> String {
    let filter = since_filter(since);
    let recs: Vec<&ToolActionRecord> = records.iter().filter(|r| filter(&r.timestamp)).collect();
    if recs.is_empty() {
        return "(no tool actions yet)".to_string();
    }
    let rows: Vec<FoldRowMeta> = recs
        .iter()
        .map(|r| FoldRowMeta {
            text: truncate_chars(
                &format!("{} {}", r.timestamp, r.tool),
                FOLDABLE_ROW_MAX_CHARS,
            ),
            round: r.round,
            domain: r.domain,
        })
        .collect();
    let layout = render_fold::fold_layout(&rows, current_round, current_domain, params);
    let expanded = match expand {
        Some(q) => render_fold::merge_expand(&layout, &rows, q),
        None => layout.expanded.clone(),
    };
    let explicit_target = explicit_target_flags(&rows, expand);
    let no_match_note = no_match_expand_note(expand, explicit_target.iter().any(|b| *b));
    let (lines, protected) = fold_view_lines(
        &rows,
        &layout.segments,
        &expanded,
        &explicit_target,
        |kind, folded_idx| tool_actions_segment_preview(&recs, kind, folded_idx),
        no_match_note,
    );
    cap_fold_view(
        "tool_actions",
        recs.len(),
        layout.segments.len(),
        lines,
        &protected,
    )
}

/// B2 exec 折叠视图（live；results 后 errors 的存储序即轮序近似，与
/// 非折叠渲染同链序）。标注带折叠段最新两条文本预览（“摘要预览”口径，
/// 实现决策登记于 B2 审计；exec 无条目级 since 过滤——pre-stamp 旧行
/// ts 空，见 B2 边界登记）。
pub fn render_exec_folded(
    exec: &ExecSection,
    current_round: u64,
    current_domain: Domain,
    params: &FoldParams,
    expand: Option<&FoldExpand>,
) -> String {
    let recs: Vec<&crate::blackboard::ExecEntry> =
        exec.results.iter().chain(exec.errors.iter()).collect();
    if recs.is_empty() {
        return "(no exec results yet)".to_string();
    }
    let rows: Vec<FoldRowMeta> = recs
        .iter()
        .map(|e| FoldRowMeta {
            text: truncate_chars(&e.text, FOLDABLE_ROW_MAX_CHARS),
            round: e.round,
            domain: e.domain,
        })
        .collect();
    let layout = render_fold::fold_layout(&rows, current_round, current_domain, params);
    let expanded = match expand {
        Some(q) => render_fold::merge_expand(&layout, &rows, q),
        None => layout.expanded.clone(),
    };
    let explicit_target = explicit_target_flags(&rows, expand);
    let no_match_note = no_match_expand_note(expand, explicit_target.iter().any(|b| *b));
    let (lines, protected) = fold_view_lines(
        &rows,
        &layout.segments,
        &expanded,
        &explicit_target,
        |kind, folded_idx| exec_segment_preview(&rows, &recs, kind, folded_idx),
        no_match_note,
    );
    cap_fold_view("exec", recs.len(), layout.segments.len(), lines, &protected)
}

// ---- P2-14 S1：压缩 marker 折叠视图快照（2026-09-04，ADR-0010 §14.54）----

/// 一条段标注（marker 块 C 候选）：`span_end_round` = 该段折叠行的最大轮号
/// （pre-stamp 段 = 0）——跨分区「取最接近近窗」的确定性排序键。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldAnnotation {
    pub text: String,
    pub span_end_round: u64,
}

/// 单分区折叠视图快照（marker 块 B/C 数据源）：与 blackboard_read 折叠
/// 渲染**同源**（同一分段 / 展开子集 / 标注词汇 / 行级 200 字符截断），
/// 差异：
/// ① 固定按折叠视图生成（强制折叠，不依赖 T/W 是否已达阈值）；
/// ② 行输入先按 `round < r_keep` 过滤（保留尾行不进 marker），且 pre-stamp
///    行（round=0）一律折叠归 C（marker 口径 §3.1：旧无章行不进块 B）。
/// marker 的有界性由装配层执行：B 明细按折叠视图既有 50 行/4K cap 截断
/// （`cap_fold_view`），C 标注经 `select_annotations_closest_to_window`
/// 取 ≤30 条并给溢出指针；本入口不承担块级预算，只保证与 blackboard_read
/// 同源的候选行/标注行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldPartitionSnapshot {
    /// 块 B 明细行（展开子集真实行，round ∈ [1, r_keep)），行序 = 存储序。
    pub detail_lines: Vec<String>,
    /// 块 C 候选标注（段序 = 视图序；每条 = 一个含折叠行的段）。
    pub annotations: Vec<FoldAnnotation>,
    /// r_keep 过滤后的分区总行数（空分区 = 0）。
    pub rows_total: usize,
    /// 过滤后的段数。
    pub segments_total: usize,
}

/// C 块选取（marker 设计 §3.3 / §8 R3）：跨分区候选标注合并后取**最接近
/// 近窗**的 `cap` 条——排序键 = `span_end_round` 降序（越晚的段越近；
/// pre-stamp 段 = 0 恒最远），同轮时按分区序（exec → edits →
/// tool_actions）与段视图序稳定（later 段优先）；保留后在各自分区内按
/// 视图序复原（输出与折叠视图行序一致）。返回（保留的 `(section, ann)`
/// 对，省略总数）。空/全 pre-stamp 输入返回空 + 0。
pub fn select_annotations_closest_to_window(
    sections: Vec<(&'static str, Vec<FoldAnnotation>)>,
    cap: usize,
) -> (Vec<(&'static str, FoldAnnotation)>, usize) {
    let total: usize = sections.iter().map(|(_, anns)| anns.len()).sum();
    if cap == 0 || total == 0 {
        return (Vec::new(), total);
    }
    let mut keyed: Vec<((u64, usize, usize), &'static str, FoldAnnotation)> = Vec::new();
    let mut section_rank = 0usize;
    for (section, anns) in &sections {
        for (i, ann) in anns.iter().enumerate() {
            keyed.push(((ann.span_end_round, section_rank, i), section, ann.clone()));
        }
        section_rank += 1;
    }
    keyed.sort_by(|a, b| {
        // 近窗最近 = span_end_round 最大；同轮 = section_rank 小者先；
        // 再同 = 段视图序后者（i 大者）先——三者均确定性。
        b.0.0
            .cmp(&a.0.0)
            .then_with(|| a.0.1.cmp(&b.0.1))
            .then_with(|| b.0.2.cmp(&a.0.2))
    });
    let kept = keyed.into_iter().take(cap).collect::<Vec<_>>();
    let mut kept_sorted = kept;
    // 复原输出序：分区序 + 段视图序（确定性；marker 排版与折叠视图同序）。
    kept_sorted.sort_by(|a, b| {
        let (sa, ia) = (a.1, a.0.2);
        let (sb, ib) = (b.1, b.0.2);
        let ra = sections
            .iter()
            .position(|(s, _)| *s == sa)
            .unwrap_or(usize::MAX);
        let rb = sections
            .iter()
            .position(|(s, _)| *s == sb)
            .unwrap_or(usize::MAX);
        ra.cmp(&rb).then_with(|| ia.cmp(&ib))
    });
    let out = kept_sorted
        .into_iter()
        .map(|(_, section, ann)| (section, ann))
        .collect();
    (out, total.saturating_sub(cap))
}

/// exec 分区快照（r_keep = 排除边界；`u64::MAX` = 不过滤，供同源对照测试）。
pub fn render_exec_snapshot(
    exec: &ExecSection,
    current_round: u64,
    current_domain: Domain,
    params: &FoldParams,
    r_keep: u64,
) -> FoldPartitionSnapshot {
    let recs: Vec<&crate::blackboard::ExecEntry> = exec
        .results
        .iter()
        .chain(exec.errors.iter())
        .filter(|e| e.round < r_keep)
        .collect();
    let rows: Vec<FoldRowMeta> = recs
        .iter()
        .map(|e| FoldRowMeta {
            text: truncate_chars(&e.text, FOLDABLE_ROW_MAX_CHARS),
            round: e.round,
            domain: e.domain,
        })
        .collect();
    partition_snapshot(&rows, current_round, current_domain, params, |kind, idx| {
        exec_segment_preview(&rows, &recs, kind, idx)
    })
}

/// edits 分区快照（无 since——压缩点是全板冻结，不做时间窗口过滤）。
pub fn render_edits_snapshot(
    records: &[EditRecord],
    current_round: u64,
    current_domain: Domain,
    params: &FoldParams,
    r_keep: u64,
) -> FoldPartitionSnapshot {
    let recs: Vec<&EditRecord> = records.iter().filter(|r| r.round < r_keep).collect();
    let rows: Vec<FoldRowMeta> = recs
        .iter()
        .map(|r| FoldRowMeta {
            text: truncate_chars(
                &format!(
                    "{} {}",
                    r.timestamp,
                    crate::controller::format_edit_record(r)
                ),
                FOLDABLE_ROW_MAX_CHARS,
            ),
            round: r.round,
            domain: r.domain,
        })
        .collect();
    partition_snapshot(&rows, current_round, current_domain, params, |kind, idx| {
        edits_segment_preview(&recs, kind, idx)
    })
}

/// tool_actions 分区快照。
pub fn render_tool_actions_snapshot(
    records: &[ToolActionRecord],
    current_round: u64,
    current_domain: Domain,
    params: &FoldParams,
    r_keep: u64,
) -> FoldPartitionSnapshot {
    let recs: Vec<&ToolActionRecord> = records.iter().filter(|r| r.round < r_keep).collect();
    let rows: Vec<FoldRowMeta> = recs
        .iter()
        .map(|r| FoldRowMeta {
            text: truncate_chars(
                &format!("{} {}", r.timestamp, r.tool),
                FOLDABLE_ROW_MAX_CHARS,
            ),
            round: r.round,
            domain: r.domain,
        })
        .collect();
    partition_snapshot(&rows, current_round, current_domain, params, |kind, idx| {
        tool_actions_segment_preview(&recs, kind, idx)
    })
}

/// 快照公共装配：分段 → 默认展开子集 → pre-stamp 强制折叠 → 拆分 B 明细
/// （展开真实行）与 C 标注（每折叠段一条；计数只含折叠行，与折叠视图标注
/// 完全同口径）。空输入返回空快照（rows_total=0）。
fn partition_snapshot(
    rows: &[FoldRowMeta],
    current_round: u64,
    current_domain: Domain,
    params: &FoldParams,
    preview: impl Fn(SegmentKind, &[usize]) -> String,
) -> FoldPartitionSnapshot {
    if rows.is_empty() {
        return FoldPartitionSnapshot {
            detail_lines: Vec::new(),
            annotations: Vec::new(),
            rows_total: 0,
            segments_total: 0,
        };
    }
    let mut layout = render_fold::fold_layout(rows, current_round, current_domain, params);
    // marker 口径：pre-stamp 行一律折叠（不进块 B；折叠视图“全旧行展开”
    // 的退化为 marker 不适用——marker 必须保持有界聚合）。
    for (i, r) in rows.iter().enumerate() {
        if r.round == 0 {
            layout.expanded[i] = false;
        }
    }
    let mut detail_lines = Vec::new();
    let mut annotations = Vec::new();
    for seg in &layout.segments {
        let folded_idx: Vec<usize> = (seg.start..seg.end)
            .filter(|&i| !layout.expanded[i])
            .collect();
        for i in seg.start..seg.end {
            if layout.expanded[i] && rows[i].round >= 1 {
                detail_lines.push(rows[i].text.clone());
            }
        }
        if folded_idx.is_empty() {
            continue;
        }
        let mut lo = u64::MAX;
        let mut hi = 0u64;
        for &i in &folded_idx {
            if rows[i].round >= 1 {
                lo = lo.min(rows[i].round);
                hi = hi.max(rows[i].round);
            }
        }
        let (lo, hi) = if lo == u64::MAX {
            (0u64, 0u64)
        } else {
            (lo, hi)
        };
        let ann = segment_annotation(
            seg.kind,
            lo,
            hi,
            folded_idx.len(),
            preview(seg.kind, &folded_idx),
        );
        annotations.push(FoldAnnotation {
            text: ann,
            span_end_round: hi,
        });
    }
    FoldPartitionSnapshot {
        detail_lines,
        annotations,
        rows_total: rows.len(),
        segments_total: layout.segments.len(),
    }
}

/// THIN-HARNESS-REDESIGN R2a (2026-08-27, §4.4): 检索分区条目上限（8K
/// 字符，与方案 B receipt 点读同口径——用户定档「8K 够用，再多去原文档/
/// 存档查找」）。指针摘要中「条目上限 8K」即指本上限。
pub const RETRIEVAL_SECTION_ENTRY_MAX_CHARS: usize = 8_000;

/// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-3, 2026-08-27)：检索分区渲染
/// **总**上限——8K 是逐条上限，条目数无界时单次 blackboard_read 可能
/// 回传数百 K（注意力稀释从「注入」转移到「按需拉取」）；总字符上限
/// 保证一次拉取有界，超限时显式标注省略条数与完整内容去向。
pub const RETRIEVAL_SECTION_TOTAL_MAX_CHARS: usize = 32_000;

/// Push one render line and account for it in the running total (incl. the
/// trailing `\n` the final join adds).
fn push_retrieval_line(lines: &mut Vec<String>, total_chars: &mut usize, line: String) {
    *total_chars += line.chars().count() + 1;
    lines.push(line);
}

/// THIN-HARNESS-REDESIGN R2a (2026-08-27, §4.4): render a live retrieval
/// partition (`internal_ret` / `external_ret`) for `blackboard_read`.
/// Live-only — retrieval partitions never enter epoch snapshots (results
/// ride the journal + `retrieval-results` artifacts), so the controller
/// rejects `epoch` on these sections before reaching here. Each entry
/// (response / one source line) is bounded at `RETRIEVAL_SECTION_ENTRY_MAX_CHARS`
/// with a truncation marker; the whole render is additionally bounded at
/// `RETRIEVAL_SECTION_TOTAL_MAX_CHARS` with an omitted-count note; the
/// full text stays in the journal archive.
pub fn render_retrieval_section(
    section: &str,
    internal: &InternalRetSection,
    external: &ExternalRetSection,
) -> String {
    let (label, response, entries, ledger) = match section {
        "internal_ret" => (
            "internal_ret",
            internal.response.as_deref(),
            internal.project_docs.as_slice(),
            internal.source_ledger.as_slice(),
        ),
        "external_ret" => (
            "external_ret",
            external.response.as_deref(),
            external.web_sources.as_slice(),
            external.source_ledger.as_slice(),
        ),
        _ => {
            return format!(
                "unknown retrieval section: {section} (expected internal_ret|external_ret)"
            );
        }
    };
    let mut lines: Vec<String> = Vec::new();
    let mut total_chars: usize = 0;
    push_retrieval_line(&mut lines, &mut total_chars, format!("== {label} =="));
    match response {
        Some(text) if !text.trim().is_empty() => {
            let bounded = truncate_chars(text, RETRIEVAL_SECTION_ENTRY_MAX_CHARS);
            if bounded != text {
                push_retrieval_line(
                    &mut lines,
                    &mut total_chars,
                    format!(
                        "[response: 超过 {RETRIEVAL_SECTION_ENTRY_MAX_CHARS} 字符上限，已截断；\
                     完整内容见 journal retrieval-results 存档]"
                    ),
                );
            }
            push_retrieval_line(&mut lines, &mut total_chars, bounded);
        }
        _ => push_retrieval_line(&mut lines, &mut total_chars, "(no response)".to_string()),
    }
    push_retrieval_line(&mut lines, &mut total_chars, "== entries ==".to_string());
    if entries.is_empty() {
        push_retrieval_line(&mut lines, &mut total_chars, "(none)".to_string());
    } else {
        let mut omitted = 0usize;
        for (i, entry) in entries.iter().enumerate() {
            let line = format!(
                "- {}",
                truncate_chars(entry, RETRIEVAL_SECTION_ENTRY_MAX_CHARS)
            );
            if total_chars + line.chars().count() + 1 > RETRIEVAL_SECTION_TOTAL_MAX_CHARS {
                omitted = entries.len() - i;
                break;
            }
            push_retrieval_line(&mut lines, &mut total_chars, line);
        }
        if omitted > 0 {
            push_retrieval_line(
                &mut lines,
                &mut total_chars,
                format!(
                    "…已省略 {omitted} 条（渲染达总上限 \
                 {RETRIEVAL_SECTION_TOTAL_MAX_CHARS}；完整内容见 journal \
                 retrieval-results 存档）"
                ),
            );
        }
    }
    push_retrieval_line(&mut lines, &mut total_chars, "== ledger ==".to_string());
    if ledger.is_empty() {
        push_retrieval_line(&mut lines, &mut total_chars, "(none)".to_string());
    } else {
        let mut omitted = 0usize;
        for (i, entry) in ledger.iter().enumerate() {
            let line = format!(
                "- {}",
                truncate_chars(entry, RETRIEVAL_SECTION_ENTRY_MAX_CHARS)
            );
            if total_chars + line.chars().count() + 1 > RETRIEVAL_SECTION_TOTAL_MAX_CHARS {
                omitted = ledger.len() - i;
                break;
            }
            push_retrieval_line(&mut lines, &mut total_chars, line);
        }
        if omitted > 0 {
            push_retrieval_line(
                &mut lines,
                &mut total_chars,
                format!(
                    "…已省略 {omitted} 条（渲染达总上限 \
                 {RETRIEVAL_SECTION_TOTAL_MAX_CHARS}；完整内容见 journal \
                 retrieval-results 存档）"
                ),
            );
        }
    }
    lines.join("\n")
}

/// PULL 自描述增量头上限（与 controller.rs `attach_pull_delta` 的
/// `HEADER_CAP` 对齐；2026-08-31 审查处理 L3 预留——保证「增量头 + 点读体」
/// 合并后仍 ≤ 8 KiB，机器可读面不与文本面截断漂移）。
pub const PULL_HEADER_MAX_BYTES: usize = 256;

/// 方案 B 点读上限（2026-08-19 用户定档 8K；2026-08-31 审查处理 L3 改字节
/// 口径：8 KiB − 增量头预算 = 7 936 B。原 8_000 字符口径在 CJK 内容下可达
/// 24 KiB 字节，超出 structured 面 8 KiB cap 且与增量头叠加必截尾；字节口径
/// 保证 human 面与机器面（entries ≤ 8 KiB）加头后仍一致）。再多去原文档/
/// 存档查找——约 4K token，单次点读载荷有界。
pub const RECEIPT_DETAIL_MAX_BYTES: usize = 8_192 - PULL_HEADER_MAX_BYTES;

/// 方案 B 按需点读（2026-08-19 黑板缓存成本设计 §4.5）：结果栏单条 receipt
/// 的完整内容——固定形态行 + `response=<JSON 完整内容>`（成功）/ `error=<JSON
/// 完整内容>`（失败，含 message/upstream，此前模型从未见过这两项）。内容为
/// 存储结构化值的重序列化（键/值/嵌套完整，非字节级原文——键序/空白可能
/// 规范化，2026-08-19 全面审查 O3 登记）。整体超
/// `RECEIPT_DETAIL_MAX_BYTES` 按字节截断 detail + 「…」+ 指针行（完整内容
/// 见存档 epoch-N.json / TraceStore trace_id=…）；合法但未找到 = 显式
/// 「not found」+ 提示旧 epoch 归档（live 板仅保留最近 50 条）。
fn render_receipt_point_read(actions: &ActionBoard, receipt_id: &str, plan_epoch: u64) -> String {
    let Some(result) = actions.results.iter().find(|r| r.order_id == receipt_id) else {
        return format!(
            "blackboard_read receipt_id={receipt_id} not found — 结果栏仅保留 \
             最近 50 条 receipt；检查 order_id 拼写（结果行行首）；更早轮次 \
             请试旧 epoch 归档（epoch-N.json）"
        );
    };
    let (step, code) = failure_envelope_fields(&result.error);
    let head = format!(
        "{} ok={} step={} code={} trace_id={}",
        result.order_id, result.ok, step, code, result.trace_id,
    );
    let detail = if result.ok {
        match &result.response {
            Some(json) => format!(
                "response={}",
                serde_json::to_string(json).unwrap_or_else(|_| "<unserializable>".to_string())
            ),
            None => "response=(none)".to_string(),
        }
    } else {
        match &result.error {
            Some(json) => format!(
                "error={}",
                serde_json::to_string(json).unwrap_or_else(|_| "<unserializable>".to_string())
            ),
            None => "error=(none)".to_string(),
        }
    };
    let body = format!("{head}\n{detail}");
    if body.len() <= RECEIPT_DETAIL_MAX_BYTES {
        return body;
    }
    // OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计 §3.2)：
    // 点读指针改向——receipt 为 run_terminal 结果时优先指向落盘文件
    // `.gsa/session/terminal/<order_id>.log`（发放时 tool_call_id =
    // order_id 小写，bash 输出按 call_id 落盘）并附 read_file 指引；
    // 非终端 receipt（无落盘文件）保留存档/TraceStore 指针兜底
    // （顺带闭合 F1：TraceStore 不在模型直接工具面）。判定按发放时订单
    // 动作名（`ActionResult.action`，审查处理 P1）——响应信封形状
    // `{"output": string}` 被 read_file/grep/run_tests 等 text-output
    // 动作共用，不能作为终端判据；旧 epoch 归档无 action 字段时回退存档
    // 指针（安全方向）。
    let pointer = match receipt_terminal_log_path(result) {
        Some(path) => {
            format!("完整内容见 {path}，请使用 read_file 读取（大文件用 offset/limit 分页）")
        }
        None => format!(
            "完整内容见存档（epoch-{plan_epoch}.json）/ TraceStore trace_id={}",
            result.trace_id
        ),
    };
    // 截断 detail 使整体（头行 + 截断 detail + 「…」 + 指针行）≤ 上限；截断
    // 复用 summary.rs truncate_chars 口径（上限内自动以「…」结尾）。
    // 尾部记账：detail 之后只追加 '\n' + 指针行（「…」已计入 truncate_chars
    // 输出的 detail_budget 内，不另行占位）——整体 ≤ 8_000（2026-08-19
    // 全面审查 N1 修正注释，数学口径不变）。
    let head_bytes = head.len() + 1; // 头行 + '\n'
    let tail_bytes = 1 + pointer.len(); // '\n' + 指针行
    let detail_budget = RECEIPT_DETAIL_MAX_BYTES
        .saturating_sub(head_bytes)
        .saturating_sub(tail_bytes)
        .max(1);
    let truncated = truncate_bytes(&detail, detail_budget);
    format!("{head}\n{truncated}\n{pointer}")
}

/// 按字节截断并加「…」提示（不超过 `max_bytes`，UTF-8 安全；2026-08-31
/// 审查处理 L3——receipt 点读从字符口径改字节口径，与机器面 cap 对齐，
/// 语义镜像 summary.rs `truncate_chars`：超限时保留 1 个「…」标记位）。
fn truncate_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut cut = max_bytes.saturating_sub("…".len());
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = s[..cut].to_string();
    out.push('…');
    out
}

/// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计 §3.2)：
/// receipt 是否为 run_terminal 结果的机械判定——按发放时订单动作名
/// `workspace.run_terminal`（`ActionResult.action`，审查处理 P1 修复）且
/// 响应信封为 `{"output": <string>}`（console `text_output_response_schema`
/// 契约防御）。命中返回落盘文件相对路径；否则 None（存档指针兜底）。
fn receipt_terminal_log_path(result: &ActionResult) -> Option<String> {
    if result.action.as_deref() != Some(crate::console::TERMINAL_SERVICE_NAME) {
        return None;
    }
    let response = result.response.as_ref()?;
    let obj = response.as_object()?;
    match obj.get("output") {
        Some(serde_json::Value::String(_)) => Some(format!(
            ".gsa/session/terminal/{}.log",
            result.order_id.to_lowercase()
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{ActionBoard, ActionResult, PlanStep, SharedBlackboard, StepStatus};

    /// THIN-HARNESS-REDESIGN R2a (§4.4): 检索分区渲染——response /
    /// entries / ledger 三块，内部与外部分区各归其位；未知分区显式报错。
    #[test]
    fn render_retrieval_section_serves_internal_and_external() {
        let internal = InternalRetSection {
            project_docs: vec!["design.md".to_string(), "gate.rs".to_string()],
            source_ledger: vec!["docs/index".to_string()],
            response: Some("检索完成\n[DOC] design.md".to_string()),
            stamp: None,
        };
        let external = ExternalRetSection {
            web_sources: vec!["https://example.com/paper".to_string()],
            source_ledger: vec!["SRC-001 https://example.com/paper".to_string()],
            response: Some("网页检索完成".to_string()),
            stamp: None,
        };

        let internal_text = render_retrieval_section("internal_ret", &internal, &external);
        assert!(
            internal_text.contains("== internal_ret =="),
            "{internal_text}"
        );
        assert!(internal_text.contains("检索完成"), "{internal_text}");
        assert!(internal_text.contains("- design.md"), "{internal_text}");
        assert!(internal_text.contains("- docs/index"), "{internal_text}");
        assert!(!internal_text.contains("example.com"), "{internal_text}");

        let external_text = render_retrieval_section("external_ret", &internal, &external);
        assert!(
            external_text.contains("== external_ret =="),
            "{external_text}"
        );
        assert!(external_text.contains("网页检索完成"), "{external_text}");
        assert!(
            external_text.contains("https://example.com/paper"),
            "{external_text}"
        );
        assert!(!external_text.contains("design.md"), "{external_text}");

        let unknown = render_retrieval_section("bogus", &internal, &external);
        assert!(
            unknown.contains("unknown retrieval section: bogus"),
            "{unknown}"
        );
    }

    /// THIN-HARNESS-REDESIGN R2a (§4.4): response 超过 8K 条目上限时截断
    /// 并显式标注（完整内容留在 journal / retrieval-results 存档）；空
    /// 分区渲染 (no response) / (none)。
    #[test]
    fn render_retrieval_section_caps_response_at_8k() {
        let long_response = "x".repeat(RETRIEVAL_SECTION_ENTRY_MAX_CHARS + 100);
        let internal = InternalRetSection {
            project_docs: Vec::new(),
            source_ledger: Vec::new(),
            response: Some(long_response.clone()),
            stamp: None,
        };
        let external = ExternalRetSection::default();
        let text = render_retrieval_section("internal_ret", &internal, &external);
        assert!(
            text.contains("超过 8000 字符上限，已截断"),
            "truncation marker: {text}"
        );
        assert!(
            !text.contains(&long_response),
            "full response must not render: {text}"
        );
        assert!(text.contains("(none)"), "{text}");

        let empty_internal = InternalRetSection::default();
        let empty = render_retrieval_section("internal_ret", &empty_internal, &external);
        assert!(empty.contains("(no response)"), "{empty}");
    }

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-3)：渲染总上限——条目数
    /// 无界时单次拉取不超 `RETRIEVAL_SECTION_TOTAL_MAX_CHARS`，超限显式
    /// 标注省略条数（完整内容留在 journal 存档）。
    #[test]
    fn render_retrieval_section_caps_total_size() {
        // 每条目 900 字符；总上限 32K → 大约 30 条后截断（加上 response/
        // 头行预算），不会渲染全部 100 条。
        let many_entries: Vec<String> = (0..100)
            .map(|i| format!("https://example.com/source-{i:03}-{}", "x".repeat(860)))
            .collect();
        let internal = InternalRetSection {
            project_docs: many_entries.clone(),
            source_ledger: many_entries.clone(),
            response: Some("检索完成".to_string()),
            stamp: None,
        };
        let external = ExternalRetSection::default();
        let text = render_retrieval_section("internal_ret", &internal, &external);

        assert!(
            text.chars().count() <= RETRIEVAL_SECTION_TOTAL_MAX_CHARS + 200,
            "total render must stay bounded ({} chars)",
            text.chars().count()
        );
        assert!(
            text.contains("已省略"),
            "omitted-count note present: {text}"
        );
        assert!(
            !text.contains("source-099"),
            "late entries must not render: {text}"
        );
    }

    #[test]
    fn render_actions_section_shows_registration_order_and_results() {
        let mut board = ActionBoard::default();
        board.set_registration(vec![crate::blackboard::ActionRegistration {
            name: "workspace.read_file".into(),
            description: "read a file".into(),
            parameters: serde_json::json!({"required": ["target_file"]}),
            target_policy: crate::entities::TargetPolicy::None,
        }]);
        board
            .write_order(crate::blackboard::ActionOrder {
                order_id: "ORD-000001".into(),
                action: "workspace.read_file".into(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                target: None,
                step_id: None,
                round: 0,
                plan_epoch: 1,
                run_id: "RUN-1".into(),
            })
            .unwrap();
        board.push_result(ActionResult {
            order_id: "ORD-000001".into(),
            action: Some("workspace.read_file".into()),
            ok: true,
            response: Some(serde_json::json!({"output": "hi"})),
            error: None,
            trace_id: "t000001".into(),
            timestamp: "2026-08-15T00:00:00Z".into(),
            round: 0,
            domain: None,
        });
        board.push_result(ActionResult {
            order_id: "ORD-000002".into(),
            action: Some("workspace.read_file".into()),
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
            round: 0,
            domain: None,
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
            action: Some("workspace.read_file".into()),
            ok: true,
            response: Some(serde_json::json!({
                "output": "x".repeat(10_000),
                "nested": {"deep": "y".repeat(5_000)},
            })),
            error: None,
            trace_id: "t-ok-1".into(),
            timestamp: "2026-08-19T00:00:00Z".into(),
            round: 0,
            domain: None,
        });
        board.push_result(ActionResult {
            order_id: "ORD-ERR-1".into(),
            action: Some("workspace.run_tests".into()),
            ok: false,
            response: None,
            error: Some(serde_json::json!({
                "step": "execute",
                "code": "boom",
                "message": "m".repeat(5_000),
            })),
            trace_id: "t-err-1".into(),
            timestamp: "2026-08-19T00:00:01Z".into(),
            round: 0,
            domain: None,
        });
        // 信封字段整体缺失 → step/code 机械回退 `?`（不编造、不隐藏）。
        board.push_result(ActionResult {
            order_id: "ORD-ERR-2".into(),
            action: Some("workspace.run_tests".into()),
            ok: false,
            response: None,
            error: None,
            trace_id: "t-err-2".into(),
            timestamp: "2026-08-19T00:00:02Z".into(),
            round: 0,
            domain: None,
        });

        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
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
                action: Some("workspace.run_terminal".into()),
                ok: true,
                response: Some(serde_json::json!({"output": format!("payload-{i}")})),
                error: None,
                trace_id: format!("t-{i:03}"),
                timestamp: format!("2026-08-19T00:{i:02}:00Z"),
                round: 0,
                domain: None,
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

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.3 + 审查处理 O6): the
    /// resident changed-count is a short marker — a truncated host delta
    /// renders `changed: N+ files` so the count never reads as the true
    /// total; an untruncated delta keeps the plain count.
    #[test]
    fn render_actions_changed_count_marks_truncated_delta() {
        let mut board = ActionBoard::default();
        board.push_result(ActionResult {
            order_id: "ORD-DELTA-TRUNC".into(),
            action: Some("workspace.run_terminal".into()),
            ok: true,
            response: Some(serde_json::json!({
                "output": "ok",
                "workspace_delta": [{"path": "a.txt", "kind": "added", "size": 3}],
                "workspace_delta_truncated": true,
            })),
            error: None,
            trace_id: "t-trunc".into(),
            timestamp: "2026-08-23T00:00:00Z".into(),
            round: 0,
            domain: None,
        });
        board.push_result(ActionResult {
            order_id: "ORD-DELTA-FULL".into(),
            action: Some("workspace.run_terminal".into()),
            ok: true,
            response: Some(serde_json::json!({
                "output": "ok",
                "workspace_delta": [{"path": "b.txt", "kind": "added", "size": 3}],
                "workspace_delta_truncated": false,
            })),
            error: None,
            trace_id: "t-full".into(),
            timestamp: "2026-08-23T00:00:01Z".into(),
            round: 0,
            domain: None,
        });
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
            None,
        );
        assert!(
            text.contains(
                "ORD-DELTA-TRUNC ok=true step=? code=? trace_id=t-trunc changed: 1+ files"
            ),
            "{text}"
        );
        assert!(
            text.contains("ORD-DELTA-FULL ok=true step=? code=? trace_id=t-full changed: 1 files"),
            "{text}"
        );
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.2）：exec 每条
    /// 结果/错误行按字符截断到 200 字符（199 字符 + 「…」，复用 summary.rs
    /// truncate_chars 口径）。
    #[test]
    fn render_exec_truncates_lines_to_200_chars() {
        let mut exec_section = ExecSection::default();
        exec_section
            .results
            .push(format!("ok {}", "x".repeat(500)).into());
        exec_section
            .errors
            .push(format!("err {}", "y".repeat(500)).into());
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &exec_section,
            &ActionBoard::default(),
            "exec",
            None,
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
            exec_section.results.push(format!("line-{i:02} run").into());
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
            exec_section.results.push("长".repeat(500).into());
        }
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &exec_section,
            &ActionBoard::default(),
            "exec",
            None,
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
            exec_section.errors.push("错".repeat(500).into());
        }
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &exec_section,
            &ActionBoard::default(),
            "exec",
            None,
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

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B）：点读
    /// 成功/失败 receipt 返回固定形态行 + 完整内容——成功 `response=<JSON
    /// 原文>`、失败 `error=<JSON 原文>`（含 message/upstream，此前模型从未
    /// 见过这两项）；`since` 与 receipt_id 同时给 = 忽略 since（点读按 id
    /// 寻址，时间过滤不适用）。
    #[test]
    fn render_actions_receipt_point_read_returns_full_response_and_error() {
        let mut board = ActionBoard::default();
        board.push_result(ActionResult {
            order_id: "ORD-PR-1".into(),
            action: Some("workspace.read_file".into()),
            ok: true,
            response: Some(serde_json::json!({
                "output": "完整成功输出",
                "nested": {"key": "value"},
            })),
            error: None,
            trace_id: "t-pr-1".into(),
            timestamp: "2026-08-19T01:00:00Z".into(),
            round: 0,
            domain: None,
        });
        board.push_result(ActionResult {
            order_id: "ORD-PR-2".into(),
            action: Some("workspace.run_tests".into()),
            ok: false,
            response: None,
            error: Some(serde_json::json!({
                "step": "execute",
                "code": "boom",
                "message": "执行失败详情",
                "upstream": {"exit_code": 1, "stderr": "tail"},
            })),
            trace_id: "t-pr-2".into(),
            timestamp: "2026-08-19T01:00:01Z".into(),
            round: 0,
            domain: None,
        });

        // 成功 receipt 点读（since 与 receipt_id 同时给 → 忽略 since）。
        let ok_text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            Some("2999-01-01T00:00:00Z"),
            Some("ORD-PR-1"),
        );
        assert!(
            ok_text.starts_with("ORD-PR-1 ok=true step=? code=? trace_id=t-pr-1\nresponse={"),
            "{ok_text}"
        );
        assert!(ok_text.contains("\"output\":\"完整成功输出\""), "{ok_text}");
        assert!(
            ok_text.contains("\"nested\":{\"key\":\"value\"}"),
            "{ok_text}"
        );
        // 失败 receipt 点读：完整信封（含 message/upstream）。
        let err_text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
            Some("ORD-PR-2"),
        );
        assert!(
            err_text
                .starts_with("ORD-PR-2 ok=false step=execute code=boom trace_id=t-pr-2\nerror={"),
            "{err_text}"
        );
        assert!(
            err_text.contains("\"message\":\"执行失败详情\""),
            "{err_text}"
        );
        assert!(
            err_text.contains("\"upstream\":{\"exit_code\":1,\"stderr\":\"tail\"}"),
            "{err_text}"
        );
        // 点读不掺入注册/订单/整段渲染。
        assert!(!err_text.contains("== registration =="), "{err_text}");
        assert!(!err_text.contains("== order =="), "{err_text}");
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B）：点读
    /// 上限 `RECEIPT_DETAIL_MAX_CHARS=8_000`——超限按字符截断 detail + 「…」
    /// 指针行。OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 /
    /// 设计 §3.2)：run_terminal receipt（订单动作 `workspace.run_terminal`
    /// + 响应信封 `{"output": string}`）的指针改向落盘文件
    /// `.gsa/session/terminal/<order_id>.log` + read_file 指引；非终端
    /// receipt 保留存档/TraceStore 兜底。
    #[test]
    fn render_actions_receipt_point_read_truncates_at_8k_with_pointer() {
        let mut board = ActionBoard::default();
        board.push_result(ActionResult {
            order_id: "ORD-PR-3".into(),
            action: Some(crate::console::TERMINAL_SERVICE_NAME.into()),
            ok: true,
            response: Some(serde_json::json!({"output": "x".repeat(20_000)})),
            error: None,
            trace_id: "t-pr-3".into(),
            timestamp: "2026-08-19T01:00:02Z".into(),
            round: 0,
            domain: None,
        });
        let plan = PlanSection {
            plan_epoch: 7,
            ..Default::default()
        };
        let text = render_section(
            &plan,
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
            Some("ORD-PR-3"),
        );
        assert!(
            text.len() <= RECEIPT_DETAIL_MAX_BYTES,
            "len {} > {RECEIPT_DETAIL_MAX_BYTES}: {text}",
            text.len()
        );
        assert!(
            text.starts_with("ORD-PR-3 ok=true step=? code=? trace_id=t-pr-3\nresponse="),
            "{text}"
        );
        // 截断以「…」收尾，随后是指针行。
        let detail_line = text.lines().nth(1).expect("detail line");
        assert!(detail_line.ends_with('…'), "{text}");
        assert!(
            text.ends_with(
                "完整内容见 .gsa/session/terminal/ord-pr-3.log，请使用 read_file 读取（大文件用 offset/limit 分页）"
            ),
            "{text}"
        );
    }

    /// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计 §3.2):
    /// 非终端 receipt（动作非 workspace.run_terminal）超限时保留存档/
    /// TraceStore 指针兜底——即使响应信封同为 `{"output": string}`
    /// （read_file/grep/run_tests 等 text-output 动作共用契约，审查处理
    /// P1 回归锁定：不得按信封形状误判为终端落盘指针）。
    #[test]
    fn render_actions_receipt_point_read_non_terminal_keeps_archive_pointer() {
        let mut board = ActionBoard::default();
        board.push_result(ActionResult {
            order_id: "ORD-PR-4".into(),
            action: Some("workspace.read_file".into()),
            ok: true,
            // read_file 动作与 run_terminal 共用 text_output_response_schema
            // ——`{"output": string}` 信封 + 非终端动作必须回退存档指针。
            response: Some(serde_json::json!({"output": "z".repeat(20_000)})),
            error: None,
            trace_id: "t-pr-4".into(),
            timestamp: "2026-08-19T01:00:03Z".into(),
            round: 0,
            domain: None,
        });
        let plan = PlanSection {
            plan_epoch: 8,
            ..Default::default()
        };
        let text = render_section(
            &plan,
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
            Some("ORD-PR-4"),
        );
        assert!(
            text.ends_with("完整内容见存档（epoch-8.json）/ TraceStore trace_id=t-pr-4"),
            "{text}"
        );
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B）：合法
    /// 但未找到 = 显式「not found」+ 提示旧 epoch 归档（live 板仅保留最近
    /// 50 条），绝不静默回退整段。
    #[test]
    fn render_actions_receipt_point_read_not_found_is_explicit() {
        let mut board = ActionBoard::default();
        board.push_result(ActionResult {
            order_id: "ORD-PR-9".into(),
            action: Some("workspace.read_file".into()),
            ok: true,
            response: Some(serde_json::json!({"output": "x"})),
            error: None,
            trace_id: "t-pr-9".into(),
            timestamp: "2026-08-19T01:00:09Z".into(),
            round: 0,
            domain: None,
        });
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
            Some("ORD-NOPE"),
        );
        assert!(text.contains("ORD-NOPE not found"), "{text}");
        assert!(text.contains("旧 epoch 归档"), "{text}");
        assert!(!text.contains("== results =="), "{text}");
    }

    /// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31 / §4.5，方案 B）：非
    /// actions 分区携带 receipt_id = 显式报错（fail loud，同未知分区风格），
    /// 未知分区同守卫（receipt_id 校验先于未知分区报错）。
    #[test]
    fn render_receipt_id_with_non_actions_section_errors() {
        let plan_text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &ActionBoard::default(),
            "plan",
            None,
            Some("ORD-1"),
        );
        assert!(
            plan_text.contains("receipt_id 仅与 section=actions 组合有效"),
            "{plan_text}"
        );
        assert!(plan_text.contains("section=plan"), "{plan_text}");

        let bogus_text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &ActionBoard::default(),
            "bogus",
            None,
            Some("ORD-1"),
        );
        assert!(
            bogus_text.contains("receipt_id 仅与 section=actions 组合有效"),
            "{bogus_text}"
        );
    }

    /// 方案 B 缓存纪律：无 receipt_id 时整段输出与 S1 逐字节一致——对含成功/
    /// 失败 receipt 的板做全量渲染并锁定精确输出（头行 / 固定形态行 / 无
    /// response 载荷），防止点读分支改动污染整段渲染。
    #[test]
    fn render_actions_without_receipt_id_matches_s1_output_byte_for_byte() {
        let mut board = ActionBoard::default();
        board.set_registration(vec![crate::blackboard::ActionRegistration {
            name: "workspace.read_file".into(),
            description: "read".into(),
            parameters: serde_json::json!({"required": ["target_file"]}),
            target_policy: crate::entities::TargetPolicy::None,
        }]);
        board.push_result(ActionResult {
            order_id: "ORD-B1".into(),
            action: Some("workspace.read_file".into()),
            ok: true,
            response: Some(serde_json::json!({"output": "secret"})),
            error: None,
            trace_id: "t-b1".into(),
            timestamp: "2026-08-19T02:00:00Z".into(),
            round: 0,
            domain: None,
        });
        board.push_result(ActionResult {
            order_id: "ORD-B2".into(),
            action: Some("workspace.run_tests".into()),
            ok: false,
            response: None,
            error: Some(serde_json::json!({
                "step": "policy",
                "code": "denied",
                "message": "m",
            })),
            trace_id: "t-b2".into(),
            timestamp: "2026-08-19T02:00:01Z".into(),
            round: 0,
            domain: None,
        });
        let text = render_section(
            &PlanSection::default(),
            &[],
            &[],
            &ExecSection::default(),
            &board,
            "actions",
            None,
            None,
        );
        assert_eq!(
            text,
            "== registration ==\n\
             workspace.read_file — read params={\"required\":[\"target_file\"]}\n\
             == order ==\n\
             (no pending order)\n\
             == results ==\n\
             ORD-B2 ok=false step=policy code=denied trace_id=t-b2\n\
             ORD-B1 ok=true step=? code=? trace_id=t-b1"
        );
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
            round: 0,
            domain: None,
        });
        bb.tool_actions.push(ToolActionRecord {
            category: "read".to_string(),
            tool: "read_file".into(),
            timestamp: "2026-08-14T00:00:00Z".into(),
            round: 0,
            domain: None,
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
                round: 0,
                domain: None,
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
            round: 0,
            domain: None,
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

    // -------------------------------------------------------------------
    // P2-13 B2 渲染折叠：分区视图组装（epoch.rs 消费 render_fold 纯函数）
    // -------------------------------------------------------------------

    fn edit(file: &str, ts: &str, round: u64, domain: Option<Domain>) -> EditRecord {
        EditRecord {
            file: file.to_string(),
            old_lines: 1,
            new_lines: 2,
            timestamp: ts.to_string(),
            round,
            domain,
        }
    }

    #[test]
    fn folded_edits_annotate_older_domain_and_keep_current_rows() {
        let mut records = Vec::new();
        for r in 1..=3 {
            records.push(edit(
                &format!("old-{r}.rs"),
                &format!("2026-09-03T00:00:0{r}Z"),
                r,
                Some(Domain::Normal),
            ));
        }
        for r in 4..=5 {
            records.push(edit(
                &format!("new-{r}.rs"),
                &format!("2026-09-03T00:00:0{r}Z"),
                r,
                Some(Domain::Pressure),
            ));
        }
        let text = render_edits_folded(
            &records,
            None,
            5,
            Domain::Pressure,
            &FoldParams {
                tail_rounds: 3,
                ..FoldParams::default()
            },
            None,
        );
        // K=3：最近轮窗口 r3–r5 → pressure r4–r5 与 normal r3 展开；旧
        // normal r1–r2 折叠为标注（含路径）。
        assert!(text.contains("[域段 normal r1–r2 · 2 条 · 路径"), "{text}");
        assert!(text.contains("new-4.rs"), "{text}");
        assert!(text.contains("new-5.rs"), "{text}");
        assert!(text.contains("00:00:03Z old-3.rs"), "{text}");
        assert!(!text.contains("00:00:01Z old-1.rs"), "{text}");
        assert!(!text.contains("00:00:02Z old-2.rs"), "{text}");

        // 显式展开 normal r1–r2：目标行追加，剩余 r3 仍折叠。
        let text2 = render_edits_folded(
            &records,
            None,
            5,
            Domain::Pressure,
            &FoldParams {
                tail_rounds: 3,
                ..FoldParams::default()
            },
            Some(&FoldExpand {
                domain: Domain::Normal,
                round_from: 1,
                round_to: 2,
            }),
        );
        assert!(text2.contains("00:00:01Z old-1.rs"), "{text2}");
        assert!(text2.contains("00:00:02Z old-2.rs"), "{text2}");
        assert!(!text2.contains("[域段 normal"), "{text2}");
        assert!(text2.contains("00:00:03Z old-3.rs"), "{text2}");
    }

    #[test]
    fn folded_edits_prestamp_annotation_counts_and_time_range() {
        let records = vec![
            edit("a.rs", "2026-09-03T00:00:00Z", 0, None),
            edit("b.rs", "2026-09-03T00:00:05Z", 0, None),
            edit("c.rs", "2026-09-03T00:00:06Z", 1, Some(Domain::Start)),
        ];
        let text = render_edits_folded(
            &records,
            None,
            1,
            Domain::Start,
            &FoldParams::default(),
            None,
        );
        assert!(
            text.contains("[域段 pre-stamp（旧行无章） · 2 条 · 时间"),
            "{text}"
        );
        assert!(
            text.contains("2026-09-03T00:00:00Z–2026-09-03T00:00:05Z"),
            "{text}"
        );
        assert!(!text.contains("a.rs"), "{text}");
    }

    #[test]
    fn folded_tool_actions_show_category_counts_in_annotation() {
        let mut records = Vec::new();
        for r in 1..=3 {
            records.push(ToolActionRecord {
                category: "read".into(),
                tool: format!("read_file r{r}"),
                timestamp: format!("2026-09-03T00:00:0{r}Z"),
                round: r,
                domain: Some(Domain::Normal),
            });
        }
        records.push(ToolActionRecord {
            category: "edit".into(),
            tool: "search_replace r4".into(),
            timestamp: "2026-09-03T00:00:04Z".into(),
            round: 4,
            domain: Some(Domain::Pressure),
        });
        let text = render_tool_actions_folded(
            &records,
            None,
            4,
            Domain::Pressure,
            &FoldParams {
                tail_rounds: 3,
                ..FoldParams::default()
            },
            None,
        );
        assert!(text.contains("[域段 normal r1 · 1 条 · read×1"), "{text}");
        assert!(text.contains("search_replace r4"), "{text}");
        assert!(text.contains("read_file r2"), "{text}");
        assert!(text.contains("read_file r3"), "{text}");
        assert!(!text.contains("read_file r1"), "{text}");
    }

    #[test]
    fn folded_exec_view_handles_prestamp_and_domain_segments() {
        use crate::blackboard::ExecEntry;
        let mut exec = ExecSection::default();
        exec.results.push(ExecEntry::from("legacy-old"));
        exec.results.push(ExecEntry::from("legacy-new"));
        for r in 1..=2 {
            exec.results.push(ExecEntry::stamped(
                format!("stamped-result-r{r}"),
                r,
                Domain::Start,
                format!("2026-09-03T00:00:0{r}Z"),
            ));
        }
        let text = render_exec_folded(&exec, 2, Domain::Start, &FoldParams::default(), None);
        // pre-stamp 旧行（无 ts）折叠为计数 + 无时间戳标注；Start 段展开。
        assert!(
            text.contains("[域段 pre-stamp（旧行无章） · 2 条 · 无时间戳]"),
            "{text}"
        );
        assert!(text.contains("stamped-result-r1"), "{text}");
        assert!(text.contains("stamped-result-r2"), "{text}");
        assert!(!text.contains("legacy-old"), "{text}");
    }

    #[test]
    fn folded_expand_outside_data_surfaces_no_match_note() {
        let records = vec![
            edit("a.rs", "2026-09-03T00:00:01Z", 1, Some(Domain::Normal)),
            edit("b.rs", "2026-09-03T00:00:02Z", 2, Some(Domain::Normal)),
        ];
        let text = render_edits_folded(
            &records,
            None,
            2,
            Domain::Normal,
            &FoldParams::default(),
            Some(&FoldExpand {
                domain: Domain::Stuck,
                round_from: 10,
                round_to: 20,
            }),
        );
        assert!(text.contains("展开目标 stuck r10–r20 无匹配行"), "{text}");
    }

    /// B2 复审（2026-09-03，cap 保护）：折叠视图超 4K 字符上限时，显式展开
    /// 目标行优先保留——目标虽是最旧行也不会被“最近优先”cap 静默裁掉。
    #[test]
    fn folded_exec_expand_target_survives_cap_overflow() {
        use crate::blackboard::ExecEntry;
        let mut exec = ExecSection::default();
        exec.results.push(ExecEntry::stamped(
            format!("OLD-A-{}", "a".repeat(150)),
            1,
            Domain::Normal,
            "2026-09-03T00:00:01Z".to_string(),
        ));
        exec.results.push(ExecEntry::stamped(
            format!("OLD-B-{}", "b".repeat(150)),
            2,
            Domain::Normal,
            "2026-09-03T00:00:02Z".to_string(),
        ));
        for r in 3..=40 {
            exec.results.push(ExecEntry::stamped(
                format!("recent-{r}-{}", "r".repeat(180)),
                r,
                Domain::Pressure,
                format!("2026-09-03T00:{r:02}:00Z"),
            ));
        }
        let text = render_exec_folded(
            &exec,
            40,
            Domain::Pressure,
            &FoldParams::default(),
            Some(&FoldExpand {
                domain: Domain::Normal,
                round_from: 1,
                round_to: 2,
            }),
        );
        // 默认视图的 pressure 行尾远超 4K；若按“最近优先”cap，OLD-A/OLD-B
        // 是最旧行会被裁掉——protected 语义保证其可见。
        assert!(text.contains("OLD-A-"), "{text}");
        assert!(text.contains("OLD-B-"), "{text}");
        assert!(text.contains("recent-40-"), "{text}");
        assert!(text.contains("优先保留展开目标行"), "{text}");
    }

    /// B2 复审（2026-09-03，标注定位）：段内部分展开（显式展开老前缀 +
    /// K 轮窗口展开尾部）时，标注行落在首个折叠行位置（r4–r8 前），不再
    /// 固定压在段首，保持行序可读。
    #[test]
    fn folded_edits_partial_expand_places_annotation_at_first_folded_row() {
        let mut records = Vec::new();
        for r in 1..=12 {
            records.push(edit(
                &format!("n{r}.rs"),
                &format!("2026-09-03T00:00:{r:02}Z"),
                r,
                Some(Domain::Normal),
            ));
        }
        for r in 13..=18 {
            records.push(edit(
                &format!("p{r}.rs"),
                &format!("2026-09-03T00:00:{r:02}Z"),
                r,
                Some(Domain::Pressure),
            ));
        }
        let text = render_edits_folded(
            &records,
            None,
            18,
            Domain::Pressure,
            &FoldParams::default(),
            Some(&FoldExpand {
                domain: Domain::Normal,
                round_from: 1,
                round_to: 3,
            }),
        );
        // 展开子集 = normal r1–r3（显式）+ r9–r12（K 窗口 r9–r18）+ pressure
        // r13–r18（当前段）；normal r4–r8 为折叠中部 → 标注范围 r4–r8。
        assert!(text.contains("[域段 normal r4–r8 · 5 条 · 路径"), "{text}");
        let pos = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle} missing"))
        };
        // 严格行序断言：r1、r2、r3 行都在标注之前，r9 行在标注之后。
        assert!(pos("00:00:01Z") < pos("[域段 normal r4–r8"), "{text}");
        assert!(pos("00:00:02Z") < pos("[域段 normal r4–r8"), "{text}");
        assert!(pos("00:00:03Z") < pos("[域段 normal r4–r8"), "{text}");
        assert!(pos("[域段 normal r4–r8") < pos("00:00:09Z"), "{text}");
    }

    // ---- P2-14 S1：压缩 marker 折叠视图快照（2026-09-04）----

    /// 同源对照 fixture：pre-stamp 2 行 + normal r1–r12 + pressure r13–r18，
    /// 当前轮 18 / pressure。供快照与 render_*_folded 逐行对照。
    fn snapshot_exec_fixture() -> crate::blackboard::ExecSection {
        let mut exec = crate::blackboard::ExecSection::default();
        exec.results
            .push(crate::blackboard::ExecEntry::from("legacy-1"));
        exec.results
            .push(crate::blackboard::ExecEntry::from("legacy-2"));
        for r in 1..=12 {
            exec.results.push(crate::blackboard::ExecEntry::stamped(
                format!("n{r}"),
                r,
                Domain::Normal,
                format!("2026-09-04T00:00:{r:02}Z"),
            ));
        }
        for r in 13..=18 {
            exec.results.push(crate::blackboard::ExecEntry::stamped(
                format!("p{r}"),
                r,
                Domain::Pressure,
                format!("2026-09-04T00:01:{r:02}Z"),
            ));
        }
        exec
    }

    /// 同源：r_keep = u64::MAX（不过滤）且视图未超 cap 时，快照的
    /// B 明细行 + C 标注行与 blackboard_read 折叠渲染输出逐行同源——
    /// 标注行 = 视图中 `[域段 …]` 行（同序），明细行 = 其余行（同序）。
    #[test]
    fn snapshot_matches_fold_view_line_for_line_when_unfiltered() {
        let exec = snapshot_exec_fixture();
        let p = FoldParams {
            tail_rounds: 5,
            tail_rows_percent: 20,
            ..FoldParams::default()
        };
        let view = render_exec_folded(&exec, 18, Domain::Pressure, &p, None);
        let snap = render_exec_snapshot(&exec, 18, Domain::Pressure, &p, u64::MAX);
        let view_lines: Vec<&str> = view.lines().collect();
        let ann_lines: Vec<&str> = view_lines
            .iter()
            .copied()
            .filter(|l| l.starts_with("[域段 "))
            .collect();
        let detail_expected: Vec<&str> = view_lines
            .iter()
            .copied()
            .filter(|l| !l.starts_with("[域段 "))
            .collect();
        assert_eq!(
            view_lines.len(),
            snap.detail_lines.len() + snap.annotations.len()
        );
        assert_eq!(
            snap.annotations
                .iter()
                .map(|a| a.text.as_str())
                .collect::<Vec<_>>(),
            ann_lines
        );
        assert_eq!(
            snap.detail_lines
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            detail_expected
        );
        assert_eq!(snap.rows_total, 20);
    }

    /// r_keep 排除：轮号 ≥ r_keep 的行不进快照（保留尾行不重复进 marker）。
    #[test]
    fn snapshot_excludes_rows_at_or_after_r_keep() {
        let exec = snapshot_exec_fixture();
        let p = FoldParams::default();
        // r_keep = 13：normal r1–r12（+ pre-stamp）保留；pressure r13–r18 剔除。
        let snap = render_exec_snapshot(&exec, 18, Domain::Pressure, &p, 13);
        assert_eq!(snap.rows_total, 14);
        assert!(
            snap.detail_lines.iter().all(|l| !l.starts_with('p')),
            "pressure 轮行不得进入快照"
        );
        assert!(
            snap.detail_lines.iter().any(|l| l.starts_with('n')),
            "normal 旧行仍应在展开子集内"
        );
    }

    /// pre-stamp 行一律归 C（不进 B），即使整分区只有旧无章行（强制折叠，
    /// 不复用 blackboard_read 的“全旧行展开”退化）。
    #[test]
    fn snapshot_prestamp_rows_always_fold_to_annotations() {
        let mut exec = crate::blackboard::ExecSection::default();
        exec.results
            .push(crate::blackboard::ExecEntry::from("legacy-a"));
        exec.results
            .push(crate::blackboard::ExecEntry::from("legacy-b"));
        let p = FoldParams::default();
        let snap = render_exec_snapshot(&exec, 1, Domain::Start, &p, u64::MAX);
        assert!(snap.detail_lines.is_empty(), "pre-stamp 行不得进 B");
        assert_eq!(snap.annotations.len(), 1);
        assert!(
            snap.annotations[0].text.contains("pre-stamp")
                && snap.annotations[0].text.contains("2 条"),
            "pre-stamp 段标注须含计数: {}",
            snap.annotations[0].text
        );
        assert_eq!(snap.annotations[0].span_end_round, 0);
        assert_eq!(snap.rows_total, 2);
    }

    /// pre-stamp 折叠语义对 edits / tool_actions 分区同源生效（与 exec 同
    /// 口径：不进 B、标注含计数、span_end_round=0）。
    #[test]
    fn snapshot_prestamp_rows_fold_across_edits_and_tool_actions() {
        let p = FoldParams::default();
        let records = vec![
            crate::blackboard::EditRecord {
                file: "a.py".to_string(),
                old_lines: 1,
                new_lines: 2,
                timestamp: "2026-09-04T00:00:01Z".to_string(),
                round: 0,
                domain: None,
            },
            crate::blackboard::EditRecord {
                file: "b.rs".to_string(),
                old_lines: 3,
                new_lines: 5,
                timestamp: "2026-09-04T00:00:02Z".to_string(),
                round: 0,
                domain: None,
            },
        ];
        let snap = render_edits_snapshot(&records, 1, Domain::Start, &p, u64::MAX);
        assert!(snap.detail_lines.is_empty(), "edits pre-stamp 不进 B");
        assert_eq!(snap.annotations.len(), 1);
        assert!(snap.annotations[0].text.contains("pre-stamp"));
        assert_eq!(snap.annotations[0].span_end_round, 0);

        let actions = vec![
            crate::blackboard::ToolActionRecord {
                category: "read".to_string(),
                tool: "read_file".to_string(),
                timestamp: "2026-09-04T00:00:03Z".to_string(),
                round: 0,
                domain: None,
            },
            crate::blackboard::ToolActionRecord {
                category: "terminal".to_string(),
                tool: "run_terminal".to_string(),
                timestamp: "2026-09-04T00:00:04Z".to_string(),
                round: 0,
                domain: None,
            },
        ];
        let snap2 = render_tool_actions_snapshot(&actions, 1, Domain::Start, &p, u64::MAX);
        assert!(
            snap2.detail_lines.is_empty(),
            "tool_actions pre-stamp 不进 B"
        );
        assert_eq!(snap2.annotations.len(), 1);
        assert!(snap2.annotations[0].text.contains("pre-stamp"));
        assert_eq!(snap2.annotations[0].span_end_round, 0);
    }

    /// 空分区快照为空（marker 按「（无）」渲染，不产生假标注）。
    #[test]
    fn snapshot_empty_partition_is_empty() {
        let exec = crate::blackboard::ExecSection::default();
        let p = FoldParams::default();
        let snap = render_exec_snapshot(&exec, 1, Domain::Start, &p, u64::MAX);
        assert!(snap.detail_lines.is_empty());
        assert!(snap.annotations.is_empty());
        assert_eq!(snap.rows_total, 0);
        assert_eq!(snap.segments_total, 0);
    }

    /// §7 矩阵第 3 项：C 超过 30 段 → 取最接近近窗的 30 + 溢出指针。
    /// 跨分区合并后按 span_end_round 降序取 30（pre-stamp=0 恒最远）；
    /// 输出按分区序 + 段视图序复原；省略数供装配层写指针。
    #[test]
    fn select_annotations_caps_at_30_nearest_with_stable_order() {
        let ann = |text: &str, span_end_round: u64| FoldAnnotation {
            text: text.to_string(),
            span_end_round,
        };
        let exec_anns: Vec<FoldAnnotation> =
            (1..=20).map(|r| ann(&format!("exec r{r}"), r)).collect();
        let edits_anns: Vec<FoldAnnotation> =
            (21..=40).map(|r| ann(&format!("edits r{r}"), r)).collect();
        let tool_anns: Vec<FoldAnnotation> = vec![
            ann("tool pre-stamp", 0),
            ann("tool r41", 41),
            ann("tool r42", 42),
        ];
        let (kept, omitted) = select_annotations_closest_to_window(
            vec![
                ("exec", exec_anns.clone()),
                ("edits", edits_anns.clone()),
                ("tool_actions", tool_anns.clone()),
            ],
            30,
        );
        // 43 候选 → 保留 30、省略 13。
        assert_eq!(omitted, 43 - 30);
        assert_eq!(kept.len(), 30);
        // 最近者必含 tool_actions r42/r41、edits r40…r21 全部、exec r20…r13
        // （合计 2+20+8=30）；exec r12 及更早被挤出。
        let texts: Vec<&str> = kept.iter().map(|(_, a)| a.text.as_str()).collect();
        assert!(texts.contains(&"tool r42") && texts.contains(&"tool r41"));
        assert!(texts.contains(&"edits r21") && texts.contains(&"edits r40"));
        assert!(texts.contains(&"exec r20"));
        assert!(texts.contains(&"exec r13"));
        assert!(!texts.contains(&"exec r12"), "r12 应被挤出");
        assert!(!texts.contains(&"tool pre-stamp"), "pre-stamp 恒最远");
        assert_eq!(
            kept.iter().filter(|(s, _)| *s == "exec").count(),
            8,
            "exec 只保留最近 8 段"
        );
        // 复原序：分区序内按段视图序（view 序递增）。
        let mut last = "";
        for (section, _a) in &kept {
            if *section != last {
                assert!(matches!(*section, "exec" | "edits" | "tool_actions"));
                last = section;
            }
        }
        assert_eq!(kept.first().unwrap().0, "exec");
        assert_eq!(kept.last().unwrap().0, "tool_actions");
        // 单分区内保持视图序（数字递增）。
        let exec_kept: Vec<u64> = kept
            .iter()
            .filter(|(s, _)| *s == "exec")
            .map(|(_, a)| a.span_end_round)
            .collect();
        assert!(exec_kept.windows(2).all(|w| w[0] < w[1]));
        // cap=0/空输入退化。
        let (empty, om) = select_annotations_closest_to_window(vec![("exec", Vec::new())], 30);
        assert!(empty.is_empty() && om == 0);
        let (none, om2) = select_annotations_closest_to_window(vec![("exec", exec_anns)], 0);
        assert!(none.is_empty() && om2 == 20);
    }
}
