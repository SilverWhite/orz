//! Five-section template summary (P0-D S3, 2026-08-14).
//!
//! ADR-0010 v1.10 §14.10 ③ / CONTEXT_COMPACTION_DESIGN §4: the template
//! summary has five fixed slots — 目的 / 计划 / 变动文件路径 are mechanically
//! filled from the blackboard (plan + edit actions); 注意事项 = HA 结构化
//! 事实聚合（阶段 (c) 定稿，ADR-0010 §14.30 / 设计 §4.4.1——只机械聚合
//! controller 已写入的结构化记录，零模型调用）；后续衔接 = 固定中性占位
//! （不交助理层，由主模型自行判断，设计 §4.4.2）。The compaction makes
//! ZERO model calls so it never re-bills the folded view under a foreign
//! prefix. Character limits 3/3/5/3/3K = 17K total bound the marker and
//! archive rendering.
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

/// 注意事项槽空态文案（阶段 (c) 定稿，ADR-0010 §14.30 / 设计 §4.4.1）：
/// 三数据源均无失败事实时显示「（无注意事项）」——不再使用阶段 (b)
/// 「机械模式无模型槽位」措辞。
pub const NOTES_FACTS_EMPTY: &str = "（无注意事项）";

/// 后续衔接槽固定中性占位（阶段 (c) 定稿，设计 §4.4.2）：不聚合任何
/// 「当前步/下一步/待办」——助理层不变量=不理解语义，机械建议可能与主
/// 模型实际评估冲突；措辞显式声明「后续衔接由主模型自行判断」，仅保留
/// 回查入口（blackboard_read 分区 + 摘要存档 + 外挂台账路径）。
/// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-2, 2026-08-27)：分区清单与
/// `build_summary_marker` 的回查行同步补 internal_ret / external_ret
/// （检索分区 live-only，不进 epoch 归档）。
pub const MECHANICAL_CONTINUATION_PLACEHOLDER: &str = "（后续衔接由主模型自行判断：可回查 blackboard_read 分区 plan/edits/tool_actions/exec/actions/internal_ret/external_ret（历史 plan epoch 用 epoch 参数；检索分区 live-only）、摘要存档与外挂台账）";

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

/// HA 结构化事实聚合（阶段 (c) 定稿，2026-08-19，ADR-0010 §14.30 /
/// CONTEXT_COMPACTION_DESIGN §4.4.1）：注意事项槽 = 助理层唯一新增输出，
/// 只机械聚合 controller 已写入的结构化记录——
///   1. `plan.steps` 中 `Failed(receipt_id)` / `Blocked` 的步骤
///      （step id + 目标 + receipt_id）；
///   2. `failure_agg` F4 失败目标聚合行（P2-12，2026-09-02 方案 A——
///      替换原「exec.errors 最近 5 条截断」窗口语义：同一目标按 F4 身份
///      (kind, id) 聚合为一行，含计数 + 错误码集合 + 首末发生时间 + 行内
///      域序列标注；跨 marker 重复顺带消除，epoch 轮换重置；exec 分区
///      原文不复制进压缩——压缩不携带日志级明细，黑板上仍可回查）；
///   3. `actions.results` 最近 3 条失败 receipt（order_id / step / code /
///      trace_id）。
///
/// 排序=计划面失败/阻塞 → 失败目标聚合 → 动作失败（计划面优先，影响最大）；
/// 空时「（无注意事项）」；≤3K 超限截断并给「其余 N 条见 blackboard_read
/// 分区/摘要存档」指针。压缩内部失败（guard/archive/外挂台账）继续走
/// marker 既有独立标注，不进本槽。零模型调用。
///
/// 审查处理（2026-09-02）：`failure_agg` 不是 `blackboard_read` 查询分区
/// （PULL 面零新增），槽位 3K 溢出时被挤掉的聚合行经普通回查入口取不到
/// ——本函数把未进槽的失败目标行以完整行文本随 `NotesFacts` 带出，由
/// 压缩存档以补全段保存（见 [`summary_archive_markdown`]），使「其余 N 条
/// 见 … 摘要存档」指针保持可回查、不误导模型。
pub fn render_facts_notes(blackboard: &Blackboard) -> NotesFacts {
    let mut lines: Vec<String> = Vec::new();
    let mut failure_rows: Vec<String> = Vec::new();

    // 1) 计划面失败/阻塞步骤（按计划顺序，设计排序第一位）。
    for step in &blackboard.plan.steps {
        let (label, receipt) = match &step.status {
            StepStatus::Failed(ev) => ("失败", Some(ev.receipt_id.as_str())),
            StepStatus::Blocked => ("受阻", None),
            _ => continue,
        };
        let line = match receipt {
            Some(r) => format!(
                "[步骤 {}] {}（{}，receipt: {}）",
                step.id, step.goal, label, r
            ),
            None => format!("[步骤 {}] {}（{}）", step.id, step.goal, label),
        };
        lines.push(line);
    }

    // 2) F4 失败目标聚合行（P2-12 方案 A，2026-09-02）：按首次出现顺序，
    //    每行 = 一个 (kind, id) 目标（计数 + 错误码集合 + 首末墙钟秒 +
    //    行内域序列）。
    for row in &blackboard.failure_agg.rows {
        let line = render_failure_target_row(row);
        failure_rows.push(line.clone());
        lines.push(line);
    }

    // 3) 动作失败 receipt（最近 3 条失败，保持原顺序；
    //    order_id / step / code / trace_id）。
    let recent_failures: Vec<_> = blackboard
        .actions
        .results
        .iter()
        .rev()
        .filter(|r| !r.ok)
        .take(NOTES_FACTS_ACTION_FAILURES_MAX)
        .collect();
    for result in recent_failures.into_iter().rev() {
        let (step, code) = failure_envelope_fields(&result.error);
        lines.push(format!(
            "[动作失败] {} step={} code={} trace_id={}",
            result.order_id, step, code, result.trace_id
        ));
    }

    let (text, hidden) = render_notes_capped(lines);
    // 槽内文本只在整行边界被挤出（render_notes_capped 的指针腾位按
    // `\n` 截断，绝不留下半行），所以「未进槽的聚合行」可用完整行
    // 判定：行文本未出现在槽位文本中即为补全段成员。
    let hidden_failure_rows = if hidden > 0 {
        failure_rows
            .into_iter()
            .filter(|row| !text.lines().any(|shown| shown == row.as_str()))
            .collect()
    } else {
        Vec::new()
    };
    NotesFacts {
        text,
        hidden_failure_rows,
    }
}

/// 注意事项槽渲染结果（P2-12 审查处理，2026-09-02）：`text` 是 ≤3K 的
/// 槽位文本（marker 与摘要存档的注意事项段共用）；`hidden_failure_rows`
/// 是溢出且未出现在槽位文本中的失败目标聚合行（完整行文本）——存档侧
/// 以补全段保存，保证「其余 N 条见 … 摘要存档」指针可回查。
pub struct NotesFacts {
    /// 注意事项槽位文本（≤3K，含溢出指针）。
    pub text: String,
    /// 溢出未进槽的失败目标聚合行；无溢出或未被挤出时为空。
    pub hidden_failure_rows: Vec<String>,
}

/// 动作失败 receipt 上限（最近 3 条失败）。
const NOTES_FACTS_ACTION_FAILURES_MAX: usize = 3;

/// 单个失败目标聚合行渲染（P2-12，2026-09-02 方案 A）。域序列如
/// `normal(r10–12)→pressure(r13)`；错误码行内集合全留/不留（全留=集合
/// 成员不截断，超 3K 槽上限走既有「显式截断 + 指针」纪律）。
fn render_failure_target_row(row: &crate::failure_agg::FailureTargetRow) -> String {
    let mut parts = vec![format!(
        "[失败目标 {}] {} ×{}",
        row.kind, row.preview, row.count
    )];
    if !row.codes.is_empty() {
        let inner = row
            .codes
            .iter()
            .map(|c| format!("{}×{}", c.code, c.count))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("codes=[{inner}]"));
    }
    parts.push(format!("首末 {:.0}s–{:.0}s", row.first_t, row.last_t));
    if !row.segments.is_empty() {
        let seq = row
            .segments
            .iter()
            .map(render_domain_segment)
            .collect::<Vec<_>>()
            .join("→");
        parts.push(format!("域 {seq}"));
    }
    parts.join(" | ")
}

/// 域段书签渲染：`normal(r10–12)`（单轮段压缩为 `normal(r10)`）。
fn render_domain_segment(seg: &crate::failure_agg::DomainSegment) -> String {
    if seg.from_round == seg.to_round {
        format!("{}(r{})", seg.domain.as_str(), seg.from_round)
    } else {
        format!(
            "{}(r{}–{})",
            seg.domain.as_str(),
            seg.from_round,
            seg.to_round
        )
    }
}

/// 按字符截断并加「…」提示（不超过 `max_chars`）。
///
/// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31）：exec 段单行渲染复用本
/// 口径（199 字符 + 「…」）——epoch.rs 与注意事项槽共用同一截断语义，避免
/// 两处口径漂移。
pub(crate) fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// 失败 receipt 的 error 信封字段（controller 结构化写入 `{step, code, ...}`）；
/// 字段缺失时机械回退到 `?`（fail-closed，不编造）。
///
/// 2026-08-19 黑板缓存成本设计（ADR-0010 §14.31）：actions 结果板固定形态
/// 行（`step/code` 缺失回退 `?`）复用本函数——与 D1=(c) 注意事项槽失败
/// receipt 同口径。
pub(crate) fn failure_envelope_fields(error: &Option<serde_json::Value>) -> (String, String) {
    let obj = error.as_ref().and_then(|v| v.as_object());
    let get = |key: &str| -> String {
        obj.and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "?".to_string())
    };
    (get("step"), get("code"))
}

/// 注意事项槽 ≤3K 上限渲染：按行顺序填放；放不下的行计入「其余 N 条」；
/// 末尾给 blackboard_read 分区/摘要存档指针（指针必须可见——必要时弹出
/// 已容纳行腾位，被弹出的行同样计入 N）。返回（槽位文本，被隐藏行数——
/// 含为腾指针弹出的行；P2-12 审查处理 2026-09-02 供溢出补全段判定）。
fn render_notes_capped(lines: Vec<String>) -> (String, usize) {
    if lines.is_empty() {
        return (NOTES_FACTS_EMPTY.to_string(), 0);
    }
    let mut out = String::new();
    let mut hidden = 0usize;
    for line in &lines {
        let candidate = if out.is_empty() {
            line.clone()
        } else {
            format!("{out}\n{line}")
        };
        if candidate.chars().count() <= SUMMARY_SLOT_LIMITS[3] {
            out = candidate;
        } else {
            hidden += 1;
        }
    }
    if hidden > 0 {
        loop {
            let pointer = format!("其余 {hidden} 条见 blackboard_read 分区/摘要存档");
            // 指针必须可见：放不下时挤出已容纳行（被挤出的行同样计入 N）。
            if out.is_empty()
                || out.chars().count() + 1 + pointer.chars().count() <= SUMMARY_SLOT_LIMITS[3]
            {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&pointer);
                break;
            }
            match out.rfind('\n') {
                Some(idx) => {
                    out.truncate(idx);
                    hidden += 1;
                }
                None => {
                    out.clear();
                    hidden += 1;
                }
            }
        }
    }
    (out, hidden)
}

/// The archive file (markdown) for one summary — the audit copy with digest.
/// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26 / 设计 §3.5 第 1 步):
/// when the loop is folded, the frozen action-ledger block is appended as
/// its own section — the compaction drains the folded region from
/// `messages`, so the archive is the ONLY place that preserves exactly what
/// the model saw (工具名/目标/结果指针/最终回复). None when not folded.
/// P2-12 审查处理（2026-09-02）：注意事项槽 3K 溢出时，被挤出槽的失败目标
/// 聚合行以补全段追加（`failure_annex`）——`failure_agg` 不是 blackboard_read
/// 查询分区，溢出指针「其余 N 条见 … 摘要存档」靠本段保持可回查。
pub fn summary_archive_markdown(
    id: &str,
    slots: &SummarySlots,
    rounds_dropped: u32,
    guard_failed: bool,
    failure_annex: Option<&[String]>,
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
            NOTES_FACTS_EMPTY
        } else {
            &slots.notes
        },
        if slots.continuation.is_empty() {
            MECHANICAL_CONTINUATION_PLACEHOLDER
        } else {
            &slots.continuation
        },
    );
    if let Some(rows) = failure_annex
        && !rows.is_empty()
    {
        out.push_str("\n## 失败目标聚合（注意事项槽 3K 溢出补全）\n\n");
        for row in rows {
            out.push_str(row);
            out.push('\n');
        }
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
         回查: blackboard_read（分区 plan / edits / tool_actions / exec / actions / \
         internal_ret / external_ret；历史 plan epoch 用 epoch 参数）\n\
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
            NOTES_FACTS_EMPTY
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
    use crate::blackboard::{
        ActionResult, EditRecord, FailedEvidence, PlanStep, SharedBlackboard, StepStatus,
    };
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{FinishReason, ModelGateway, ToolCall};
    use crate::host::ToolResult;
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;

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
                round: 0,
                domain: None,
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
                    round: 0,
                    domain: None,
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

    fn failed_step(id: &str, goal: &str, receipt: &str) -> PlanStep {
        PlanStep {
            id: id.into(),
            goal: goal.into(),
            actions: Vec::new(),
            acceptance: String::new(),
            evidence: Vec::new(),
            status: StepStatus::Failed(FailedEvidence {
                receipt_id: receipt.into(),
            }),
        }
    }

    fn blocked_step(id: &str, goal: &str) -> PlanStep {
        PlanStep {
            id: id.into(),
            goal: goal.into(),
            actions: Vec::new(),
            acceptance: String::new(),
            evidence: Vec::new(),
            status: StepStatus::Blocked,
        }
    }

    fn action_result(
        order_id: &str,
        ok: bool,
        error: Option<serde_json::Value>,
        trace: &str,
    ) -> ActionResult {
        ActionResult {
            order_id: order_id.into(),
            action: None,
            ok,
            response: None,
            error,
            trace_id: trace.into(),
            timestamp: "2026-08-19T00:00:00Z".into(),
            round: 0,
            domain: None,
        }
    }

    #[test]
    fn facts_notes_empty_shows_no_notes() {
        // 阶段 (c) 定稿（ADR-0010 §14.30 / 设计 §4.4.1）：三数据源均无
        // 失败事实时显示「（无注意事项）」——不再用「机械模式无模型槽位」。
        let bb = SharedBlackboard::new();
        assert_eq!(render_facts_notes(&bb.read()).text, NOTES_FACTS_EMPTY);
        assert!(
            render_facts_notes(&bb.read())
                .hidden_failure_rows
                .is_empty()
        );
        // 只有成功动作/已完成步骤不算注意事项。
        {
            let mut w = bb.write();
            w.plan.steps.push(PlanStep {
                id: "s1".into(),
                goal: "已成功".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::Done(crate::blackboard::DoneEvidence {
                    receipt_id: "ORD-1".into(),
                    direct: None,
                }),
            });
            w.actions
                .results
                .push(action_result("ORD-2", true, None, "t2"));
        }
        assert_eq!(render_facts_notes(&bb.read()).text, NOTES_FACTS_EMPTY);
    }

    #[test]
    fn facts_notes_orders_plan_then_failure_targets_then_actions() {
        // P2-12 方案 A（2026-09-02）：排序=计划面失败/阻塞 → 失败目标聚合
        // → 动作失败；exec 分区原文不再复制进槽（原「最近 5 条截断错误」
        // 窗口被 F4 聚合行替换——压缩不携带日志级明细）。
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.plan.steps.push(failed_step("s1", "失败步骤", "ORD-1"));
            w.plan.steps.push(blocked_step("s2", "受阻步骤"));
            w.plan.steps.push(PlanStep {
                id: "s3".into(),
                goal: "进行中步骤".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::InProgress,
            });
            w.failure_agg.record(
                "cmd_target",
                "id-c1",
                "python train.py --epochs 50",
                "tool_timeout",
                10.0,
                10,
                orz_assurance::lif::Domain::Normal,
            );
            w.failure_agg.record(
                "cmd_target",
                "id-c1",
                "python train.py --epochs 50",
                "execution_failed",
                90.0,
                13,
                orz_assurance::lif::Domain::Pressure,
            );
            // exec 分区原文仍在（模型可回查 exec 分区），但不再进注意事项槽。
            w.exec.errors.push("执行错误一".into());
            w.actions.results.push(action_result(
                "ORD-2",
                false,
                Some(serde_json::json!({
                    "step": "policy",
                    "code": "policy_denied",
                    "message": "denied",
                })),
                "t2",
            ));
            w.actions
                .results
                .push(action_result("ORD-3", true, None, "t3"));
        }
        let notes = render_facts_notes(&bb.read());
        let lines: Vec<&str> = notes.text.lines().collect();
        assert_eq!(
            lines,
            vec![
                "[步骤 s1] 失败步骤（失败，receipt: ORD-1）",
                "[步骤 s2] 受阻步骤（受阻）",
                "[失败目标 cmd_target] python train.py --epochs 50 ×2 | \
                 codes=[tool_timeout×1, execution_failed×1] | 首末 10s–90s | \
                 域 normal(r10)→pressure(r13)",
                "[动作失败] ORD-2 step=policy code=policy_denied trace_id=t2",
            ]
        );
        // 成功 receipt 不入列；exec 原文行不再出现在槽内。
        assert!(lines.iter().all(|l| !l.starts_with("[执行错误]")));
        assert!(notes.hidden_failure_rows.is_empty());
    }

    #[test]
    fn facts_notes_failure_targets_merge_same_and_keep_distinct() {
        // 同一目标多次失败聚为一行（计数 + 首末时间 + 行内域序列）；不同
        // 目标保持各自行（首次出现顺序）。
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.failure_agg.record(
                "cmd_target",
                "id-a",
                "python train.py",
                "tool_timeout",
                5.0,
                1,
                orz_assurance::lif::Domain::Start,
            );
            w.failure_agg.record(
                "cmd_target",
                "id-b",
                "python val.py",
                "tool_timeout",
                7.0,
                2,
                orz_assurance::lif::Domain::Start,
            );
            w.failure_agg.record(
                "cmd_target",
                "id-a",
                "python train.py",
                "tool_timeout",
                20.0,
                3,
                orz_assurance::lif::Domain::Normal,
            );
        }
        let notes = render_facts_notes(&bb.read());
        let lines: Vec<&str> = notes.text.lines().collect();
        assert_eq!(
            lines,
            vec![
                "[失败目标 cmd_target] python train.py ×2 | codes=[tool_timeout×2] | \
                 首末 5s–20s | 域 start(r1)→normal(r3)",
                "[失败目标 cmd_target] python val.py ×1 | codes=[tool_timeout×1] | \
                 首末 7s–7s | 域 start(r2)",
            ]
        );
    }

    #[test]
    fn facts_notes_caps_many_failure_rows_with_pointer() {
        // 大量失败目标行：槽位 ≤3K，溢出给「其余 N 条见 blackboard_read
        // 分区/摘要存档」指针（既有 render_notes_capped 纪律）。
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            for i in 0..200 {
                w.failure_agg.record(
                    "cmd_target",
                    &format!("id-{i}"),
                    &format!("cmd {i} {}", "x".repeat(60)),
                    "tool_timeout",
                    i as f64,
                    i as u64 + 1,
                    orz_assurance::lif::Domain::Start,
                );
            }
        }
        let notes = render_facts_notes(&bb.read());
        assert!(
            notes.text.chars().count() <= SUMMARY_SLOT_LIMITS[3],
            "notes slot must stay within 3K: {}",
            notes.text.chars().count()
        );
        assert!(notes.text.contains("其余"), "{}", notes.text);
        assert!(
            notes.text.contains("见 blackboard_read 分区/摘要存档"),
            "{}",
            notes.text
        );
        assert!(notes.text.lines().last().unwrap().starts_with("其余 "));
        // P2-12 审查处理（2026-09-02）：溢出行随 NotesFacts 带出——存档
        // 补全段据此保存被 3K 槽挤掉的聚合行（failure_agg 非查询分区，
        // 指针「其余 N 条见 … 摘要存档」靠补全段保持可回查）。
        assert!(!notes.hidden_failure_rows.is_empty());
        assert!(
            notes
                .hidden_failure_rows
                .iter()
                .all(|row| row.starts_with("[失败目标 "))
        );
        assert!(
            notes
                .hidden_failure_rows
                .iter()
                .all(|row| !notes.text.lines().any(|shown| shown == row.as_str()))
        );
    }

    #[test]
    fn facts_notes_takes_last_3_failed_receipts_skipping_ok() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            // 失败：r1/r3/r4/r6；成功：r0/r2/r5 —— 最近 3 条失败 = r3/r4/r6。
            w.actions
                .results
                .push(action_result("r0", true, None, "t0"));
            w.actions.results.push(action_result(
                "r1",
                false,
                Some(serde_json::json!({"step": "execute", "code": "boom"})),
                "t1",
            ));
            w.actions
                .results
                .push(action_result("r2", true, None, "t2"));
            w.actions.results.push(action_result(
                "r3",
                false,
                Some(serde_json::json!({"step": "policy", "code": "policy_denied"})),
                "t3",
            ));
            w.actions.results.push(action_result(
                "r4",
                false,
                Some(serde_json::json!({"step": "execute", "code": "boom"})),
                "t4",
            ));
            w.actions
                .results
                .push(action_result("r5", true, None, "t5"));
            w.actions
                .results
                .push(action_result("r6", false, None, "t6"));
        }
        let notes = render_facts_notes(&bb.read());
        let lines: Vec<&str> = notes.text.lines().collect();
        assert_eq!(
            lines,
            vec![
                "[动作失败] r3 step=policy code=policy_denied trace_id=t3",
                "[动作失败] r4 step=execute code=boom trace_id=t4",
                // 信封缺失时机械回退 `?`，trace_id 取 receipt 自带字段。
                "[动作失败] r6 step=? code=? trace_id=t6",
            ]
        );
    }

    #[test]
    fn facts_notes_caps_at_3k_with_overflow_pointer() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            // 足够多的失败步骤保证溢出（每行 ~110 字符 × 40 ≈ 4.4K）。
            for i in 0..40 {
                w.plan.steps.push(failed_step(
                    &format!("s{i}"),
                    &format!("目标 {i} {}", "长".repeat(80)),
                    &format!("ORD-{i}"),
                ));
            }
        }
        let notes = render_facts_notes(&bb.read());
        assert!(
            notes.text.chars().count() <= SUMMARY_SLOT_LIMITS[3],
            "notes slot must stay within 3K: {}",
            notes.text.chars().count()
        );
        assert!(notes.text.contains("其余"), "{}", notes.text);
        assert!(
            notes.text.contains("见 blackboard_read 分区/摘要存档"),
            "{}",
            notes.text
        );
        assert!(notes.text.lines().last().unwrap().starts_with("其余 "));
        // 溢出指针是末尾一行，且被截掉的条目数机械可数（行数 < 40）。
        assert!(notes.text.lines().count() < 40, "{}", notes.text);
        // 本次溢出全部来自计划面步骤（无失败目标行）——补全段应为空。
        assert!(notes.hidden_failure_rows.is_empty());
    }

    #[test]
    fn facts_notes_single_overlong_line_falls_back_to_pointer_only() {
        // 边界：单条事实超过 3K 时整行无法容纳——槽位退化为仅指针，
        // 且「其余 N 条」须计入这条超长行本身（N=1）。
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.plan
                .steps
                .push(failed_step("s1", &"长".repeat(4_000), "ORD-1"));
        }
        let notes = render_facts_notes(&bb.read());
        assert_eq!(notes.text, "其余 1 条见 blackboard_read 分区/摘要存档");
        assert!(notes.hidden_failure_rows.is_empty());
    }

    #[test]
    fn archive_annex_carries_overflowed_failure_rows() {
        // P2-12 审查处理（2026-09-02）：failure_agg 不是 blackboard_read
        // 查询分区——注意事项槽 3K 溢出时被挤掉的聚合行必须随压缩存档以
        // 补全段保存，指针「其余 N 条见 … 摘要存档」才可回查。
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            for i in 0..200 {
                w.failure_agg.record(
                    "cmd_target",
                    &format!("id-{i}"),
                    &format!("cmd {i} {}", "x".repeat(60)),
                    "tool_timeout",
                    i as f64,
                    i as u64 + 1,
                    orz_assurance::lif::Domain::Start,
                );
            }
        }
        let notes = render_facts_notes(&bb.read());
        assert!(notes.text.contains("其余"), "{}", notes.text);
        assert!(!notes.hidden_failure_rows.is_empty());

        // 补全段：只含「槽内看不到」的完整行，且存档 markdown 实际包含。
        let slots = SummarySlots {
            notes: notes.text.clone(),
            ..slots()
        };
        let markdown = summary_archive_markdown(
            "compaction-RUN-X-003",
            &slots,
            3,
            false,
            Some(&notes.hidden_failure_rows),
            None,
        );
        assert!(
            markdown.contains("## 失败目标聚合（注意事项槽 3K 溢出补全）"),
            "{markdown}"
        );
        for row in &notes.hidden_failure_rows {
            assert!(markdown.contains(row.as_str()), "annex missing: {row}");
        }
        assert!(
            notes
                .hidden_failure_rows
                .iter()
                .all(|row| !notes.text.contains(row.as_str())),
            "hidden row must stay out of the capped slot text"
        );

        // 无溢出（None）时存档不含补全段——存量存档内容不变。
        let markdown_plain =
            summary_archive_markdown("compaction-RUN-X-004", &slots, 3, false, None, None);
        assert!(!markdown_plain.contains("## 失败目标聚合"));
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
            summary_archive_markdown("compaction-RUN-X-001", &slots, 3, false, None, None);
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
        let markdown = summary_archive_markdown(
            "compaction-RUN-X-001",
            &slots,
            3,
            false,
            None,
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
            3,
            Some(Path::new(".gsa/ledger/current.md")),
        );
        assert!(marker.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX));
    }

    #[test]
    fn continuation_placeholder_and_empty_notes_in_marker_and_archive() {
        // 2026-08-19 阶段 (c) 定稿（ADR-0010 §14.30）：注意事项空态为
        // 「（无注意事项）」；后续衔接为固定中性占位（不交助理层、由主
        // 模型自行判断）——无 summary_incomplete 终止态、无「生成失败」。
        let slots = SummarySlots {
            notes: String::new(),
            continuation: String::new(),
            ..slots()
        };
        let markdown =
            summary_archive_markdown("compaction-RUN-X-002", &slots, 2, false, None, None);
        assert!(markdown.contains(NOTES_FACTS_EMPTY));
        assert!(markdown.contains(MECHANICAL_CONTINUATION_PLACEHOLDER));
        assert!(!markdown.contains("生成失败"));
        assert!(!markdown.contains("机械模式无模型槽位"));
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
        assert!(marker.contains(NOTES_FACTS_EMPTY));
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

    /// P0-D review fix (2026-08-14, ADR-0010 v1.14): a summary archive write
    /// failure is retried and then EXPLICITLY reported in the event and the
    /// marker — never swallowed.
    #[tokio::test]
    async fn summary_archive_write_failure_is_reported() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let blocked_cwd = test_dir();
        // `.gsa` exists as a FILE — create_dir_all(.gsa/compaction) fails.
        std::fs::write(blocked_cwd.join(".gsa"), "occupied").unwrap();
        let host = BlockedArchiveHost {
            inner: TestHost {
                journal,
                tool_result: Some(ToolResult {
                    output: "x".repeat(600),
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                }),
            },
            blocked_cwd: blocked_cwd.clone(),
        };
        let tool_call = |id: &str| ScriptedResponse {
            text: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: id.to_string(),
            }],
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: Some(300_000),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            tool_call("call-a1"),
            tool_call("call-a2"),
            tool_call("call-a3"),
            tool_call("call-a4"),
            // Round a4 also reports 300K — the fallback re-fires on the next
            // loop-top (机械模式：两次压缩均零模型调用，无摘要项).
            ScriptedResponse::text("候选答案"),
            ScriptedResponse::text("最终答案"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway)
            .with_context_compact(150_000, 400, 20, 100_000)
            .with_summary_guards(1, 1.0);
        controller
            .run_turn(
                &host,
                "压缩测试",
                "RUN-COMPACT-ARCHIVE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let compact_events: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ContextCompressed)
            .map(|e| e.payload)
            .collect();
        // The fallback re-fires while the measured tokens stay over the
        // safety line — every summary attempt reports the write failure.
        assert_eq!(compact_events.len(), 2, "{compact_events:?}");
        for event in &compact_events {
            assert_eq!(event["archive_write_failed"], true);
            assert_eq!(event["summary_incomplete"], false);
            assert!(event["summary_path"].as_str().is_some());
        }
        // The archive path/digest are still reported (the intended pointer),
        // and the marker carries the explicit failure note.
        let received = fake.received_requests();
        assert!(
            received.iter().any(|r| r
                .messages
                .iter()
                .any(|m| { m.content.contains("存档写入失败：摘要未落盘") })),
            "archive failure must reach the model: {received:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&blocked_cwd);
    }
}
