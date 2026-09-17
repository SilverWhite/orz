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
use orz_assurance::lif::Domain;

pub const SUMMARY_MAX_TOTAL_CHARS: usize = 17_000;
/// 目的 / 计划 / 变动文件路径 / 注意事项 / 后续衔接 — 3/3/5/3/3K.
pub const SUMMARY_SLOT_LIMITS: [usize; 5] = [3_000, 3_000, 5_000, 3_000, 3_000];

/// Bounded retries for persisting the summary archive — a write failure is
/// an audit gap and must be retried explicitly, then reported.
pub const ARCHIVE_WRITE_MAX_ATTEMPTS: usize = 3;

/// 注意事项槽空态文案（阶段 (c) 定稿，ADR-0010 §14.30 / 设计 §4.4.1）：
/// 三数据源均无失败事实时显示「（无）」——P2-13 D4（2026-09-03，
/// ADR-0010 §14.52）将压缩五段槽空槽统一渲染为「（无）」，取代阶段 (c)
/// 「（无注意事项）」与更早的「（未设置）」/空串混用。
pub const NOTES_FACTS_EMPTY: &str = "（无）";

/// 后续衔接槽固定中性占位（阶段 (c) 定稿，设计 §4.4.2）：不聚合任何
/// 「当前步/下一步/待办」——助理层不变量=不理解语义，机械建议可能与主
/// 模型实际评估冲突；措辞显式声明「后续衔接由主模型自行判断」，仅保留
/// 回查入口（blackboard_read 分区 + 摘要存档 + 外挂台账路径）。
/// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-2, 2026-08-27)：分区清单与
/// `build_summary_marker` 的回查行同步补 internal_ret / external_ret
/// （检索分区 live-only，不进 epoch 归档）。
pub const MECHANICAL_CONTINUATION_PLACEHOLDER: &str = "（后续衔接由主模型自行判断：可回查 blackboard_read 分区 plan/edits/tool_actions/exec/actions/internal_ret/external_ret（检索分区 live-only；历史记录全量保留在 live 黑板，按域/轮数展开或 since/receipt_id 回查——epoch 归档读已于生产面退役）、摘要存档与外挂台账）";

/// Estimated tokens of one summary marker in the kept context.
/// P2-14（2026-09-04，ADR-0010 §14.54 / 设计稿 §4）：v0.3 折叠视图快照
/// marker 总量 20_000 字符定档 → chars/2 ≈ 10K + 余量 ≈ 11K 起步（S4 实测
/// 后定档）。缩减守卫用同一估算常量（对仍走 v0.2 的检索车道略保守 ≤2K，
/// 相对 200K 触发窗口可忽略——登记于 P2-14 复审处理）。
pub const SUMMARY_MARKER_ESTIMATE_TOKENS: u64 = 11_000;

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
///
/// 0p S2 复审 P1-2 修复（B5 第 5 漏斗，2026-09-07，ADR-0010 §14.61）：
/// 压缩存档是「会话快照」家族最大体量落卷面（`.gsa/compaction/*.md`，
/// 含冻结台账视图原文），写盘点统一接 orz-secrets 机械脱敏——key 不落
/// 卷不变量；台账漏斗的脱敏产出在折叠视图嵌入处被内存原文重现的旁路
/// 由此闭合（审计面可接受脱敏失真，用户裁决「审计部分不怕」）。
pub fn write_archive_retry(archive_dir: &Path, archive_path: &Path, markdown: &str) -> bool {
    let scrubbed = orz_secrets::redact_secrets(markdown);
    for _ in 0..ARCHIVE_WRITE_MAX_ATTEMPTS {
        if std::fs::create_dir_all(archive_dir).is_ok()
            && std::fs::write(archive_path, scrubbed.as_bytes()).is_ok()
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
        .unwrap_or_else(|| "（无）".to_string());
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
        out = "（无）".to_string();
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
        out = "（无）".to_string();
    } else if overflow > 0 {
        let holder = epoch_archive
            .unwrap_or(archive_path)
            .to_string_lossy()
            .to_string();
        // P2-13 B3（2026-09-03，ADR-0010 §14.52）：生产面不再有 plan-epoch
        // 快照指针——holder 在 --plan/测试域为 epoch 快照、生产面回退摘要
        // 存档，措辞统一为中性「见 {holder}」，不再声称 plan epoch。
        out.push_str(&format!("\n（其余 {overflow} 条路径见 {holder}）"));
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
/// 空时「（无）」（P2-13 D4 统一口径）；≤3K 超限截断并给「其余 N 条见 blackboard_read
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
/// 0p S1（2026-09-07）：`selfhistory::render_failures_only` 同源复用——
/// blackboard_read failures_only 面与压缩注意事项槽共用同一行语义。
pub(crate) fn render_failure_target_row(row: &crate::failure_agg::FailureTargetRow) -> String {
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
pub(crate) fn render_domain_segment(seg: &crate::failure_agg::DomainSegment) -> String {
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
    session_snapshot: Option<&str>,
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28): the fixed external ledger path — the marker line points a
    // restored conversation at the surviving append-only history.
    ledger_path: Option<&Path>,
    // v7（S1 修订批）：原文定位指针四项（设计 §3.5.1）——每条压缩 marker 必带。
    locators: &LocatorPointers,
) -> String {
    // 机械模式（2026-08-18 B 定案，ADR-0010 §14.29）：压缩恒写存档，
    // digest 恒存在——无「摘要重试失败」终止态。
    let digest_line = format!("摘要 digest: sha256:{digest}");
    let ledger_line =
        ledger_path.map_or_else(String::new, |p| format!("历史摘要累积于 {}\n", p.display()));
    let locator_lines = locators.render_marker_lines(archive_path, digest);
    format!(
        "[前文上下文已压缩 v0.2]\n\
         {guard_note}\
         {archive_note}\
         {ledger_line}\
         摘要 ID: {id}\n被压轮次: {rounds_dropped} 轮\n\
         摘要存档: {}\n{digest_line}\n\
         黑板会话: {}\n\
         {locator_lines}\n\
         目的: {}\n\
         计划: {}\n\
         变动文件路径: {}\n\
         注意事项: {}\n\
         后续衔接: {}\n\
         回查: blackboard_read（分区 plan / edits / tool_actions / exec / actions / \
         internal_ret / external_ret；历史全量保留在 live，可按域/轮数展开）\n\
         [/前文上下文已压缩]",
        archive_path.display(),
        session_snapshot.unwrap_or("（无）"),
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

/// **v7 语义轨 marker**（S1 修订批，2026-09-15，设计 §3.4.1／§3.5.1；用户
/// 裁定 DP-14／DP-17）：模型产出的**语义摘要替换被压区**——marker 承载该
/// 摘要正文（这就是压缩后继续携带的语义面）、四项原文定位指针、以及结构化
/// 轨的分工声明（机械层只做指针化，**不宣称语义保全**）。
///
/// prefix 仍为 `[前文上下文已压缩`（滚动单 marker 的删除口径与注入块识别
/// 共用同一前缀；`is_injected_block_text` 已登记）——`v0.3-语义` 版本号区分
/// 机械快照与模型摘要两条轨。
#[allow(clippy::too_many_arguments)]
pub fn build_model_summary_marker(
    id: &str,
    digest: &str,
    archive_path: &Path,
    summary: &str,
    reason: &str,
    rounds_dropped: u32,
    round_from: Option<u64>,
    r_keep: u64,
    session_snapshot: Option<&str>,
    archive_write_failed: bool,
    ledger_path: Option<&Path>,
    locators: &LocatorPointers,
) -> String {
    let ledger_line =
        ledger_path.map_or_else(String::new, |p| format!("历史摘要累积于 {}\n", p.display()));
    let locator_lines = locators.render_marker_lines(archive_path, digest);
    let round_note = match round_from {
        Some(f) if f <= r_keep.saturating_sub(1) => {
            format!(
                "（会话轮 r{f}–r{}，保留尾首轮 r_keep={r_keep}）",
                r_keep - 1
            )
        }
        _ => format!("（保留尾首轮 r_keep={r_keep}）"),
    };
    format!(
        "[前文上下文已压缩 {version}]\n\
         {archive_note}\
         {ledger_line}\
         摘要 ID: {id}\n被压轮次: {rounds_dropped} 轮{round_note}\n\
         摘要存档: {archive_path}\n摘要 digest: sha256:{digest}\n\
         黑板会话: {session}\n\
         语义摘要来源: 模型产出（reason={reason}，mode=model_summary）\n\
         {locator_lines}\n\
         == 语义摘要（模型产出，原文替换） ==\n{summary}\n\n\
         == 结构化轨（机械） ==\n\
         工具／命令／结果类原文已由机械压缩移出模型上下文（台账摘要行每字段上限 \
         300 字符——机械层只做指针化，不宣称语义保全）。**逐字原文的权威载体是 \
         journal（上表 journal run+seq）**：台账只有摘要行、本 compaction 存档只存 \
         摘要与指针、sidecar 不含已被移出的区间（逐字分页回读属 S2、尚未实现）。\n\
         == 查询指针 ==\n{pointer}\n\
         [/前文上下文已压缩]",
        version = MODEL_SUMMARY_MARKER_VERSION,
        archive_note = if archive_write_failed {
            "存档写入失败：摘要未落盘，需处理\n"
        } else {
            ""
        },
        archive_path = archive_path.display(),
        session = session_snapshot.unwrap_or("（无）"),
        pointer = FOLD_SNAPSHOT_POINTER_TEXT,
    )
}

/// v7 语义轨 marker 版本号（机械快照 v0.3 ／ 语义摘要 v0.3-语义）。
pub const MODEL_SUMMARY_MARKER_VERSION: &str = "v0.3-语义";

/// 语义轨 marker／存档的固定开销估算（token，chars/2 估算口径）：框架行 ＋
/// 四项定位指针 ＋ 分工声明。缩减判定用（摘要块不小于被压区 ⇒ 不替换）。
pub const MODEL_SUMMARY_MARKER_OVERHEAD_TOKENS: u64 = 1_200;

/// 语义轨替换文本的估算体量（token）＝ 摘要正文字数/2 ＋ 固定开销。
pub fn model_summary_marker_estimate(summary: &str) -> u64 {
    summary.chars().count() as u64 / 2 + MODEL_SUMMARY_MARKER_OVERHEAD_TOKENS
}

/// 语义轨压缩存档（审计副本）：模型产出的语义摘要 ＋ 四项原文定位指针 ＋
/// 结构化轨的分工声明。**不复制被压区逐字原文**——原文在 run journal 与
/// conversation sidecar（`.gsa/compaction/` 只留摘要与指针，避免同一份逐字
/// 内容在三处重复膨胀，设计 §3.5「A 与 B 两类留存量」）。
#[allow(clippy::too_many_arguments)]
pub fn model_summary_archive_markdown(
    id: &str,
    reason: &str,
    rounds_dropped: u32,
    r_keep: u64,
    session: Option<&str>,
    archive_path: &Path,
    summary: &str,
    locators: &LocatorPointers,
) -> String {
    let locator_lines = locators.render_marker_lines(archive_path, "（见 marker 行）");
    format!(
        "# ORZ 会话压缩摘要（语义轨 {version}，模型产出）{id}\n\n\
         - 状态: complete（mode=model_summary；模型产出摘要替换被压区）\n\
         - 事件 reason: {reason}\n\
         - 被压轮次: {rounds_dropped} 轮\n- 保留尾首轮 r_keep: {r_keep}\n\
         - 黑板会话: {session}\n- 摘要存档: {path}\n\n\
         ## 原文定位（四项）\n{locator_lines}\n\n\
         ## 语义摘要（模型产出，原文替换；被压区逐字本体以 journal 为准——\
         sidecar 只含未被移出的对话）\n{summary}\n\n\
         ## 结构化轨（机械）\n\
         工具／命令／结果类原文由机械压缩移出模型上下文（台账摘要行每字段上限 300 \
         字符）；机械层只做指针化，**不宣称语义保全**。\n",
        version = MODEL_SUMMARY_MARKER_VERSION,
        path = archive_path.display(),
        session = session.unwrap_or("（无）"),
    )
}

/// sha256 digest of the archive content (the marker's digest binding).
pub fn archive_digest(markdown: &str) -> String {
    sha256_hex(markdown.as_bytes())
}

// ===========================================================================
// P2-14 压缩 marker 折叠视图快照 v0.3（2026-09-04，ADR-0010 §14.54 / 设计稿
// CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN v0.2 定稿）
// ===========================================================================
// 五段槽 v0.2 路径保留为兼容回退（车道范围裁决 2026-09-04 复审处理：检索/
// grill 等非 Main 车道压缩、以及旧会话消息无轮章 r_keep 不可推导的主会话
// 压缩继续走 v0.2）；主会话压缩点生成 v0.3 = 压缩点冻结的黑板折叠视图快照：
//   A 框架（ID / 被压轮次与轮区间 / r_keep / 会话 / 存档路径 + digest）
//   B 近窗明细（round < r_keep，已排除保留尾；视图 cap 同源）
//   C 旧段聚合（≤30 条标注行，取最接近近窗者，更早省略留指针）
//   D failure_agg（≤3K，溢出随存档 annex 补全段保存）
//   E 查询指针（省略区/旧区的显式回查指令）
// marker 总量 ≤ 20_000 字符（定档）；分区块级超限一律「截断 + 指针」，绝不
// 静默丢失。生成零模型调用；快照与 blackboard_read 折叠渲染同源
// （render_*_snapshot / cap_fold_view / segment 标注词汇）。

/// v0.3 marker 版本号（prefix 不变，CONTEXT_COMPRESSED_PREFIX 识别不受影响）。
pub const COMPACTION_MARKER_VERSION: &str = "v0.3";

/// A 框架块预算（600 字符；设计稿 §2.2）。
pub const COMPACTION_FRAMEWORK_CHARS: usize = 600;
/// D failure_agg 块预算（3_000 字符；设计稿 §2.2）。
pub const COMPACTION_FAILURE_CHARS: usize = 3_000;
/// E 查询指针块预算（1_000 字符；设计稿 §2.2）。
pub const COMPACTION_POINTER_CHARS: usize = 1_000;

/// env 覆盖名（编译期默认 + env 覆盖，沿用 ORZ_* 模式；设计稿 §4）。
pub const SNAPSHOT_DETAIL_CHARS_ENV: &str = "ORZ_COMPACTION_SNAPSHOT_DETAIL_CHARS";
pub const SNAPSHOT_SEGMENT_LINES_ENV: &str = "ORZ_COMPACTION_SNAPSHOT_SEGMENT_LINES";
pub const MARKER_TOTAL_CHARS_ENV: &str = "ORZ_COMPACTION_MARKER_TOTAL_CHARS";
/// B 近窗明细块级预算默认（8_000 字符）。
pub const DEFAULT_SNAPSHOT_DETAIL_CHARS: usize = 8_000;
/// C 旧段聚合标注行数上限默认（30 条）。
pub const DEFAULT_SNAPSHOT_SEGMENT_LINES: usize = 30;
/// marker 总量上限默认（20_000 字符，设计稿 §4 定档）。
pub const DEFAULT_MARKER_TOTAL_CHARS: usize = 20_000;

/// v0.3 marker 预算（编译期默认 + env 覆盖；测试直接构造绕开进程级 env
/// 并发竞态，与 `render_fold::FoldParams` 同纪律）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionBudget {
    /// B 近窗明细块级字符预算。
    pub detail_chars: usize,
    /// C 旧段聚合标注行数上限（跨分区合计）。
    pub segment_lines: usize,
    /// A+B+C+D+E 总量字符上限。
    pub total_chars: usize,
    /// D failure_agg 块字符预算。
    pub failure_chars: usize,
}

impl Default for CompactionBudget {
    fn default() -> Self {
        Self {
            detail_chars: DEFAULT_SNAPSHOT_DETAIL_CHARS,
            segment_lines: DEFAULT_SNAPSHOT_SEGMENT_LINES,
            total_chars: DEFAULT_MARKER_TOTAL_CHARS,
            failure_chars: COMPACTION_FAILURE_CHARS,
        }
    }
}

impl CompactionBudget {
    /// 编译期默认 + env 覆盖：缺失/非法/0 一律回退默认。
    pub fn from_env() -> Self {
        let positive = |env: &str, default: usize| -> usize {
            std::env::var(env)
                .ok()
                .and_then(|v| v.trim().parse::<usize>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(default)
        };
        Self {
            detail_chars: positive(SNAPSHOT_DETAIL_CHARS_ENV, DEFAULT_SNAPSHOT_DETAIL_CHARS),
            segment_lines: positive(SNAPSHOT_SEGMENT_LINES_ENV, DEFAULT_SNAPSHOT_SEGMENT_LINES),
            total_chars: positive(MARKER_TOTAL_CHARS_ENV, DEFAULT_MARKER_TOTAL_CHARS),
            failure_chars: COMPACTION_FAILURE_CHARS,
        }
    }
}

/// 主会话压缩点的折叠快照 LIF 上下文（round, domain）——由调用方传入
/// （`SharedLoopServices` 无 LIF 引用；设计稿 §6 实施影响）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldSnapshotCtx {
    pub current_round: u64,
    pub current_domain: Domain,
}

/// **v7 原文定位指针**（S1 修订批，2026-09-15，设计 §3.5.1；用户裁定）：
/// 每次压缩产出的 marker／摘要块必须携带四项定位指针，使模型（与事后审计）
/// 能按指针回读逐字原文——S1 只有 compaction 存档路径＋digest＋台账路径，
/// 缺 journal 与 sidecar 定位。
///
/// 1. compaction 存档路径 ＋ digest（下两字段之外的 `archive_path`/`digest`）；
/// 2. 台账 `[seq]` 区间（本窗口 epoch 内已折叠行；跨压缩连续的全局行键）；
/// 3. journal `run id ＋ sequence 区间`（本窗口 epoch 的事件跨度）；
/// 4. conversation sidecar 路径（`.gsa/conversations/<session8>.json`）。
///
/// 口径（设计 §3.5.1「三键口径」同源）：S1／修订批给**窗口 epoch 级**跨度
/// （段级 LIF 轮区间随尾批 §7 落地）；缺项如实渲染「（无）」，不虚构。
/// 逐字分页回读（按区间重放）仍属 S2。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocatorPointers {
    /// 台账 `[seq]` 区间（闭区间；None = 本窗口未折叠/无外挂台账）。
    pub ledger_seq: Option<(u64, u64)>,
    /// 外挂台账文件路径（None = 无）。
    pub ledger_path: Option<String>,
    /// journal run id（None = grill 丢弃模式 / 无 journal 的测试面）。
    pub journal_run: Option<String>,
    /// journal sequence 闭区间（本窗口 epoch）。
    pub journal_seq: Option<(u64, u64)>,
    /// conversation sidecar 路径。
    pub conversation_path: Option<String>,
    /// **本地面是否全量保留**（2026-09-16 v8 实现批）：v8 的按块压缩/截断
    /// **不 drain** 会话本体 ⇒ sidecar 仍含被处理分块的逐字原文，定位文案必须
    /// 如实说「在被其中」（v7 的「被压区不在其中」是 drain 语义，随勘误作废）。
    pub local_face_full: bool,
}

impl LocatorPointers {
    /// 四项定位指针的 marker／摘要块渲染（缺项如实写「（无）」）。
    pub fn render_marker_lines(&self, archive_path: &Path, digest: &str) -> String {
        fn range(r: Option<(u64, u64)>) -> String {
            match r {
                Some((a, b)) => format!("{a}–{b}"),
                None => "（无）".to_string(),
            }
        }
        // 审查修正批（2026-09-15，审查 P2③）：**逐项标注载体到哪一层**——
        // compaction 存档按设计只存摘要与指针，台账行每字段 300 字符上限。
        //
        // v8 实现批（2026-09-16，审查 R-4）：`local_face_full` ＝ 按块压缩/
        // 截断路径（**不 drain**，本地面全量保留）⇒ sidecar 与**按块档案**都是
        // 逐字载体，「被压区不在其中」不再成立（那是 v7 drain 语义）；模板轨
        // （检索/grill 车道与会话收尾的机械 drain）仍按原口径如实标注。
        let ledger_file = self
            .ledger_path
            .as_deref()
            .map(|p| format!("（文件 {p}；每字段 300 字符摘要行）"))
            .unwrap_or_default();
        let sidecar_line = match (self.local_face_full, self.conversation_path.as_deref()) {
            (true, Some(path)) => format!(
                "会话档案（sidecar）: {path}（**含被处理分块的逐字原文**——v8 按块压缩/截断不 drain 会话本体）"
            ),
            (true, None) => {
                "会话档案（sidecar）: （无）——本地面全量留档于会话本体（未 drain）".to_string()
            }
            (false, Some(path)) => {
                format!("会话档案（sidecar）: {path}（未被移出的对话；被压区不在其中）")
            }
            (false, None) => "会话档案（sidecar）: （无）".to_string(),
        };
        format!(
            "原文定位（四项）:\n\
             - compaction 存档: {}（sha256:{}；只存摘要与指针，不含被处理分块逐字原文）\n\
             - 台账 [seq] 区间: {}{ledger_file}\n\
             - journal: run={} seq={}（逐事件全文＝**逐字原文的权威载体**）\n\
             - {sidecar_line}",
            archive_path.display(),
            digest,
            range(self.ledger_seq),
            self.journal_run.as_deref().unwrap_or("（无）"),
            range(self.journal_seq),
        )
    }
}

/// 单次 v0.3 marker 生成的输入（调用方在 drain 后定稿）。
#[derive(Debug, Clone, Copy)]
pub struct FoldSnapshotInput<'a> {
    pub id: &'a str,
    pub archive_path: &'a Path,
    pub session: Option<&'a str>,
    pub rounds_dropped: u32,
    /// 被压区间的首轮（首条被 drain 声明消息的轮章；旧会话无章时 None）。
    pub round_from: Option<u64>,
    /// 保留尾首轮（保留尾首条声明消息的轮章；行排除边界 round >= r_keep）。
    pub r_keep: u64,
    pub ctx: FoldSnapshotCtx,
    pub guard_failed: bool,
    pub archive_write_failed: bool,
    /// marker A 注记的外挂台账路径提示（主车道折叠态；与 v0.2 marker
    /// 「历史摘要累积于 …」行同口径）。
    pub ledger_note: Option<&'a str>,
    /// 存档「冻结折叠视图」段的字节固定指针消息（`fold_state.folded_ledger`
    /// 原文；与 v0.2 存档段一致，None = 未折叠/无指针）。
    pub frozen_ledger: Option<&'a str>,
    /// v7 原文定位指针四项（S1 修订批；缺项如实渲染「（无）」）。
    pub locators: &'a LocatorPointers,
    pub budget: CompactionBudget,
}

/// v0.3 生成统计（供校验与遥测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldSnapshotStats {
    pub marker_chars: usize,
    /// 因 B 块级预算被省略的明细行数（含整分区未入块的行）。
    pub b_omitted_lines: usize,
    /// 因 C 条数上限被省略的段标注数。
    pub c_omitted_segments: usize,
    /// 因 D 块级预算被挤出、随存档 annex 保存的失败目标行数。
    pub d_omitted_rows: usize,
}

/// v0.3 装配输出：marker（A–E）、存档 markdown（同源 + annex + 冻结指针）、
/// failure_agg 溢出补全行、统计。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldSnapshotOutput {
    pub marker: String,
    pub archive: String,
    pub failure_annex: Vec<String>,
    pub stats: FoldSnapshotStats,
}

/// E 查询指针固定模板（机械、零模型；块预算 1K 内）。
pub const FOLD_SNAPSHOT_POINTER_TEXT: &str = "回查：blackboard_read section=exec|edits|tool_actions，\
先看域段标注，再带 domain + round_from/round_to 精确展开；pre-stamp 旧行（round=0）用 \
since/receipt_id 展开；更早历史见摘要存档与 run journal；plan/actions/internal_ret/\
external_ret 等非折叠分区不在 marker 内，直接 blackboard_read 对应分区";

/// 行集合按字符上限保留头部、尾部溢出行逐条挤出以容纳指针（与既有
/// `render_notes_capped` 同纪律：整行边界操作、指针必须可见）。返回
/// （块文本，被挤出/未显示的完整行——顺序保持）。
fn cap_rows_with_annex(
    lines: Vec<String>,
    max_chars: usize,
    pointer: impl Fn(usize) -> String,
) -> (String, Vec<String>) {
    if lines.is_empty() {
        return (NOTES_FACTS_EMPTY.to_string(), Vec::new());
    }
    let mut shown: Vec<String> = Vec::new();
    let mut used = 0usize;
    for line in &lines {
        let add = line.chars().count() + if shown.is_empty() { 0 } else { 1 };
        if used + add > max_chars {
            break;
        }
        shown.push(line.clone());
        used += add;
    }
    let mut hidden: Vec<String> = lines[shown.len()..].to_vec();
    let mut out = shown.join("\n");
    if hidden.is_empty() {
        return (out, hidden);
    }
    loop {
        let note = pointer(hidden.len());
        if out.is_empty() || out.chars().count() + 1 + note.chars().count() <= max_chars {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&note);
            break;
        }
        match out.rfind('\n') {
            Some(idx) => {
                hidden.insert(0, out[idx + 1..].to_string());
                out.truncate(idx);
            }
            None => {
                hidden.insert(0, out.clone());
                out.clear();
            }
        }
    }
    (out, hidden)
}

/// 折叠快照装配的内部块（B/C/D/E 文本，空态统一「（无）」）。
struct FoldBlocks {
    b: String,
    c: String,
    d: String,
    e: String,
    /// C 块因条数上限被省略的段标注数（供统计与指针）。
    c_omitted: usize,
}

/// D failure_agg 块：全量行按序渲染 ≤3K；溢出行以完整行随 `annex` 带出
/// （随存档补全段保存——failure_agg 非 blackboard_read 查询分区）。
fn render_failure_target_block(blackboard: &Blackboard, budget: usize) -> (String, Vec<String>) {
    let rows: Vec<String> = blackboard
        .failure_agg
        .rows
        .iter()
        .map(render_failure_target_row)
        .collect();
    let (text, hidden) = cap_rows_with_annex(rows, budget, |n| {
        format!("其余 {n} 条见摘要存档失败目标聚合补全段")
    });
    (text, hidden)
}

/// 单分区近窗明细文本：与 blackboard_read 折叠渲染同源的视图 cap（50 行/
/// 4K 字符，头行说明省略——`cap_fold_view`），空分区返回 None。
fn render_partition_detail(
    section: &'static str,
    snapshot: &crate::render_fold::FoldPartitionSnapshot,
) -> Option<String> {
    if snapshot.detail_lines.is_empty() {
        return None;
    }
    let protected = vec![false; snapshot.detail_lines.len()];
    Some(crate::render_fold::cap_fold_view(
        section,
        snapshot.rows_total,
        snapshot.segments_total,
        snapshot.detail_lines.clone(),
        &protected,
    ))
}

/// 生成 A–E 块文本（marker 与存档共用块级渲染）。B 明细已由调用方按块级
/// 预算预裁，`b_omitted_*` 供指针；C/D 在此装配。
fn assemble_blocks(
    bb: &Blackboard,
    ctx: FoldSnapshotCtx,
    r_keep: u64,
    params: &crate::render_fold::FoldParams,
    budget: CompactionBudget,
    b_sections: &[(&'static str, String)],
    b_omitted_lines: usize,
    b_omitted_sections: &[&'static str],
) -> FoldBlocks {
    let mut b_text = String::new();
    for (section, body) in b_sections {
        if !b_text.is_empty() {
            b_text.push('\n');
        }
        b_text.push_str(&format!("-- {section} --\n{body}"));
    }
    if !b_omitted_sections.is_empty() && b_omitted_lines > 0 {
        b_text.push_str(&format!(
            "\n（近窗明细另有 {b_omitted_lines} 行（{}）因块预算 {} 字符省略——可经 \
             blackboard_read section=exec|edits|tool_actions 展开回查）",
            b_omitted_sections.join("、"),
            budget.detail_chars
        ));
    }
    if b_text.is_empty() {
        b_text = NOTES_FACTS_EMPTY.to_string();
    }

    // C 旧段聚合：跨分区标注合并，取最接近近窗的 ≤segment_lines 条。
    let snapshots = [
        (
            "exec",
            crate::render_fold::render_exec_snapshot(
                &bb.exec,
                ctx.current_round,
                ctx.current_domain,
                params,
                r_keep,
            ),
        ),
        (
            "edits",
            crate::render_fold::render_edits_snapshot(
                &bb.edits,
                ctx.current_round,
                ctx.current_domain,
                params,
                r_keep,
            ),
        ),
        (
            "tool_actions",
            crate::render_fold::render_tool_actions_snapshot(
                &bb.tool_actions,
                ctx.current_round,
                ctx.current_domain,
                params,
                r_keep,
            ),
        ),
    ];
    let mut sections: Vec<(&'static str, Vec<crate::render_fold::FoldAnnotation>)> = Vec::new();
    for (name, snap) in snapshots {
        if !snap.annotations.is_empty() {
            sections.push((name, snap.annotations));
        }
    }
    let (kept, c_omitted) =
        crate::render_fold::select_annotations_closest_to_window(sections, budget.segment_lines);
    let mut c_text = String::new();
    let mut current: Option<&'static str> = None;
    for (section, ann) in kept {
        if current != Some(section) {
            if !c_text.is_empty() {
                c_text.push('\n');
            }
            c_text.push_str(&format!("-- {section} --"));
            current = Some(section);
        }
        c_text.push('\n');
        c_text.push_str(&ann.text);
    }
    if c_omitted > 0 {
        c_text.push_str(&format!(
            "\n（其余 {c_omitted} 段因条数上限 {} 省略——见 blackboard_read \
             section=exec|edits|tool_actions，先看域段标注，再带 domain + \
             round_from/round_to 精确展开；pre-stamp 旧行用 since/receipt_id）",
            budget.segment_lines
        ));
    }
    if c_text.is_empty() {
        c_text = NOTES_FACTS_EMPTY.to_string();
    }

    // D failure_agg（溢出 annex 由调用方写存档补全段）。
    let (d_text, _hidden) = render_failure_target_block(bb, budget.failure_chars);

    // E 查询指针（固定模板；预算 1K）。
    let mut e_text = FOLD_SNAPSHOT_POINTER_TEXT.to_string();
    if e_text.chars().count() > COMPACTION_POINTER_CHARS {
        e_text = truncate_chars(&e_text, COMPACTION_POINTER_CHARS);
    }

    FoldBlocks {
        b: b_text,
        c: c_text,
        d: d_text,
        e: e_text,
        c_omitted,
    }
}

/// A 框架 + B/C/D/E 的最终 marker 文本（含 digest 绑定；digest 为存档摘要）。
#[allow(clippy::too_many_arguments)]
fn assemble_marker_text(
    version: &str,
    segment_cap: usize,
    id: &str,
    rounds_dropped: u32,
    round_from: Option<u64>,
    r_keep: u64,
    archive_path: &Path,
    digest: &str,
    session: Option<&str>,
    guard_failed: bool,
    archive_write_failed: bool,
    ledger: Option<&str>,
    // v7（S1 修订批）：原文定位指针四项——每次压缩的 marker 必带，使模型
    // 与事后审计都能回读被压区原文（设计 §3.5.1）。
    locators: &LocatorPointers,
    blocks: &FoldBlocks,
) -> String {
    let mut head = format!("[前文上下文已压缩 {version}]\n");
    if guard_failed {
        head.push_str("机制失败：缩减守卫连续不满足，已强制压缩，需处理\n");
    }
    if archive_write_failed {
        head.push_str("存档写入失败：摘要未落盘，需处理\n");
    }
    if let Some(p) = ledger {
        head.push_str(&format!("历史摘要累积于 {p}\n"));
    }
    let paren = match round_from {
        Some(f) if f <= r_keep.saturating_sub(1) => {
            format!("会话轮 r{f}–r{}，保留尾首轮 r_keep={r_keep}", r_keep - 1)
        }
        _ => format!("保留尾首轮 r_keep={r_keep}"),
    };
    let locator_lines = locators.render_marker_lines(archive_path, digest);
    format!(
        "{head}摘要 ID: {id}\n被压轮次: {rounds_dropped} 轮（{paren}）\n\
         摘要存档: {}\n摘要 digest: sha256:{digest}\n黑板会话: {}\n\n\
         {locator_lines}\n\n\
         == 近窗明细（round < r_keep，已排除保留尾） ==\n{}\n\n\
         == 旧段聚合（≤{segment_cap} 条标注行） ==\n{}\n\n\
         == 失败目标聚合 ==\n{}\n\n\
         == 查询指针 ==\n{}\n\
         [/前文上下文已压缩]",
        archive_path.display(),
        session.unwrap_or("（无）"),
        blocks.b,
        blocks.c,
        blocks.d,
        blocks.e,
    )
}

/// 装配 v0.3 marker + 存档（含 D annex / 冻结折叠指针）。B 明细先按块级
/// 预算预裁（链序 exec → edits → tool_actions），总量仍超限时从 B 尾部
/// 逐段裁切并补指针——分区块级超限一律「截断 + 指针」，绝不静默丢失。
pub fn build_fold_snapshot_marker(
    bb: &Blackboard,
    input: &FoldSnapshotInput<'_>,
) -> FoldSnapshotOutput {
    let budget = input.budget;
    let params = crate::render_fold::FoldParams::from_env();
    // 各分区近窗明细（视图 cap 内文本；空分区不入块）。
    let mut kept_b: Vec<(&'static str, String)> = Vec::new();
    let mut omitted_b: Vec<(&'static str, String)> = Vec::new();
    for (section, snapshot) in [
        (
            "exec",
            crate::render_fold::render_exec_snapshot(
                &bb.exec,
                input.ctx.current_round,
                input.ctx.current_domain,
                &params,
                input.r_keep,
            ),
        ),
        (
            "edits",
            crate::render_fold::render_edits_snapshot(
                &bb.edits,
                input.ctx.current_round,
                input.ctx.current_domain,
                &params,
                input.r_keep,
            ),
        ),
        (
            "tool_actions",
            crate::render_fold::render_tool_actions_snapshot(
                &bb.tool_actions,
                input.ctx.current_round,
                input.ctx.current_domain,
                &params,
                input.r_keep,
            ),
        ),
    ] {
        if let Some(body) = render_partition_detail(section, &snapshot) {
            kept_b.push((section, body));
        }
    }
    // B 块级预算预裁（确定性链序，超限从尾部裁切）。
    let mut used = 0usize;
    let mut kept_cursor = Vec::new();
    for (section, body) in kept_b {
        let add =
            section.len() + 2 + body.chars().count() + if kept_cursor.is_empty() { 0 } else { 2 };
        if used + add > budget.detail_chars {
            omitted_b.push((section, body));
            continue;
        }
        used += add;
        kept_cursor.push((section, body));
    }
    let mut b_sections = kept_cursor;
    let mut b_omitted = omitted_b;
    let mut b_omitted_lines: usize = b_omitted.iter().map(|(_, b)| b.lines().count()).sum();
    let omitted_names: Vec<&str> = b_omitted.iter().map(|(s, _)| *s).collect();
    let mut blocks = assemble_blocks(
        bb,
        input.ctx,
        input.r_keep,
        &params,
        budget,
        &b_sections,
        b_omitted_lines,
        &omitted_names,
    );
    // 总量超限兜底：逐段裁掉 B 尾部明细（逆链序：tool_actions → edits →
    // exec），指针随裁切重建。
    let mut c_omitted = blocks.c_omitted;
    loop {
        let probe = assemble_marker_text(
            COMPACTION_MARKER_VERSION,
            budget.segment_lines,
            input.id,
            input.rounds_dropped,
            input.round_from,
            input.r_keep,
            input.archive_path,
            &"0".repeat(64),
            input.session,
            input.guard_failed,
            input.archive_write_failed,
            input.ledger_note,
            input.locators,
            &blocks,
        );
        if probe.chars().count() <= budget.total_chars || b_sections.is_empty() {
            break;
        }
        let (section, body) = b_sections.pop().expect("non-empty checked above");
        b_omitted.push((section, body));
        b_omitted_lines = b_omitted.iter().map(|(_, b)| b.lines().count()).sum();
        let omitted_names: Vec<&str> = b_omitted.iter().map(|(s, _)| *s).collect();
        blocks = assemble_blocks(
            bb,
            input.ctx,
            input.r_keep,
            &params,
            budget,
            &b_sections,
            b_omitted_lines,
            &omitted_names,
        );
        c_omitted = blocks.c_omitted;
    }
    // D annex 单独取一次（与块内 D 文本同源；写存档补全段用）。
    let (_d_text, d_annex) = render_failure_target_block(bb, budget.failure_chars);
    let omitted_names: Vec<&str> = b_omitted.iter().map(|(s, _)| *s).collect();
    blocks = assemble_blocks(
        bb,
        input.ctx,
        input.r_keep,
        &params,
        budget,
        &b_sections,
        b_omitted_lines,
        &omitted_names,
    );
    let mut archive = format!(
        "# ORZ 会话压缩摘要（折叠视图快照 {version}）{id}\n\n\
         - 状态: complete（机械模式，零模型调用——ADR-0010 §14.54）\n\
         - 被压轮次: {rounds_dropped} 轮\n- 保留尾首轮 r_keep: {r_keep}\n\
         - 黑板会话: {session}\n- 守卫强制: {guard}\n- 摘要存档: {path}\n\n\
         ## 近窗明细（round < r_keep，已排除保留尾）\n{b}\n\n\
         ## 旧段聚合（≤{cap} 条标注行）\n{c}\n\n\
         ## 失败目标聚合\n{d}\n\n\
         ## 查询指针\n{e}\n",
        version = COMPACTION_MARKER_VERSION,
        id = input.id,
        rounds_dropped = input.rounds_dropped,
        r_keep = input.r_keep,
        session = input.session.unwrap_or("（无）"),
        guard = if input.guard_failed {
            "是（缩减守卫连续不满足，已强制压缩）"
        } else {
            "否"
        },
        path = input.archive_path.display(),
        b = blocks.b,
        cap = budget.segment_lines,
        c = blocks.c,
        d = blocks.d,
        e = blocks.e,
    );
    if !d_annex.is_empty() {
        archive.push_str("\n## 失败目标聚合（D 块溢出补全）\n\n");
        for row in &d_annex {
            archive.push_str(row);
            archive.push('\n');
        }
    }
    if let Some(ledger) = input.frozen_ledger {
        archive.push_str("\n## 折叠视图（冻结快照：外挂指针）\n\n```\n");
        archive.push_str(ledger);
        archive.push_str("\n```\n");
    }
    let digest = archive_digest(&archive);
    let marker = assemble_marker_text(
        COMPACTION_MARKER_VERSION,
        budget.segment_lines,
        input.id,
        input.rounds_dropped,
        input.round_from,
        input.r_keep,
        input.archive_path,
        &digest,
        input.session,
        input.guard_failed,
        input.archive_write_failed,
        input.ledger_note,
        input.locators,
        &blocks,
    );
    let marker_chars = marker.chars().count();
    let d_omitted_rows = d_annex.len();
    FoldSnapshotOutput {
        marker,
        archive,
        failure_annex: d_annex,
        stats: FoldSnapshotStats {
            marker_chars,
            b_omitted_lines,
            c_omitted_segments: c_omitted,
            d_omitted_rows,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{
        ActionResult, EditRecord, FailedEvidence, PlanStep, SharedBlackboard, StepStatus,
    };

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
                run: String::new(),
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

    /// P2-13 D4（2026-09-03，ADR-0010 §14.52）：压缩五段槽保留字段，
    /// 空槽统一渲染「（无）」——取代「（未设置）」/空串/「（本窗口无
    /// 编辑）」/「（无注意事项）」混用；非空槽只放真实内容。
    #[test]
    fn empty_slots_render_uniform_wu_in_marker_and_archive() {
        let bb = SharedBlackboard::new();
        let (purpose, plan, paths) =
            mechanical_slots(&bb.read(), Path::new(".gsa/compaction/x.md"), None);
        assert_eq!(purpose, "（无）");
        assert_eq!(plan, "（无）");
        assert_eq!(paths, "（无）");
        let notes = render_facts_notes(&bb.read());
        assert_eq!(notes.text, NOTES_FACTS_EMPTY);
        let slots = SummarySlots {
            purpose,
            plan,
            paths,
            notes: String::new(),
            continuation: String::new(),
        };
        let markdown =
            summary_archive_markdown("compaction-RUN-WU-001", &slots, 1, false, None, None);
        for expected in [
            "## 目的\n（无）",
            "## 计划\n（无）",
            "## 变动文件路径\n（无）",
            "## 注意事项\n（无）",
        ] {
            assert!(
                markdown.contains(expected),
                "missing {expected:?}: {markdown}"
            );
        }
        let digest = archive_digest(&markdown);
        let marker = build_summary_marker(
            "compaction-RUN-WU-001",
            &digest,
            Path::new(".gsa/compaction/x.md"),
            &slots,
            1,
            false,
            false,
            None,
            None,
            test_locators(),
        );
        for expected in [
            "目的: （无）",
            "计划: （无）",
            "变动文件路径: （无）",
            NOTES_FACTS_EMPTY,
        ] {
            assert!(marker.contains(expected), "missing {expected}: {marker}");
        }
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
                    run: String::new(),
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
        // --plan/测试域（显式传 epoch_archive）：溢出指针指向 plan-epoch
        // 快照而非压缩摘要存档（B3 后生产面无此指针，回退摘要存档）。
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
        // 失败事实时显示「（无）」——不再用「机械模式无模型槽位」。
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
            Some("SESSION-abc"),
            Some(Path::new(".gsa/ledger/current.md")),
            test_locators(),
        );
        assert!(marker.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX));
        assert!(marker.contains(&digest));
        assert!(marker.contains("compaction-RUN-X-001.md"));
        assert!(marker.contains("历史摘要累积于 .gsa/ledger/current.md"));
        assert!(marker.contains("修复缓存回归"));
        assert!(marker.contains("黑板会话: SESSION-abc"));
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
            Some("SESSION-abc"),
            Some(Path::new(".gsa/ledger/current.md")),
            test_locators(),
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
            Some("SESSION-abc"),
            None,
            test_locators(),
        );
        assert!(!marker.contains("summary_incomplete"));
        assert!(marker.contains(NOTES_FACTS_EMPTY));
        assert!(marker.contains(MECHANICAL_CONTINUATION_PLACEHOLDER));
        assert!(marker.contains(&format!("sha256:{digest}")));
        assert!(marker.contains("黑板会话: SESSION-abc"));
    }

    /// v7（S1 修订批，设计 §3.5.1，DP-17）：每条压缩 marker 必须携带**四项
    /// 原文定位指针**（compaction 存档＋digest／台账 `[seq]` 区间／journal
    /// run+sequence／conversation sidecar 路径），缺项如实写「（无）」——
    /// 机械 marker 与语义 marker 共用同一渲染函数（本钉走语义 marker）。
    #[test]
    fn compact_marker_carries_four_locator_pointers() {
        let locators = LocatorPointers {
            ledger_seq: Some((7, 23)),
            ledger_path: Some(".gsa/ledger/current.md".to_string()),
            journal_run: Some("RUN-LOC-0".to_string()),
            journal_seq: Some((11, 88)),
            conversation_path: Some(".gsa/conversations/session1.json".to_string()),
            local_face_full: false,
        };
        let summary = "[SEMANTIC_SUMMARY]\n目标: x\n已完成: y\n[/SEMANTIC_SUMMARY]";
        let marker = build_model_summary_marker(
            "compaction-RUN-LOC-0001",
            &"a".repeat(64),
            Path::new(".gsa/compaction/compaction-RUN-LOC-0001.md"),
            summary,
            "model_selected",
            2,
            Some(1),
            3,
            Some("SESSION-loc"),
            false,
            Some(Path::new(".gsa/ledger/current.md")),
            &locators,
        );
        for expected in [
            "原文定位（四项）",
            "compaction 存档: .gsa/compaction/compaction-RUN-LOC-0001.md",
            "台账 [seq] 区间: 7–23（文件 .gsa/ledger/current.md；每字段 300 字符摘要行）",
            "journal: run=RUN-LOC-0 seq=11–88",
            "会话档案（sidecar）: .gsa/conversations/session1.json",
            "mode=model_summary",
        ] {
            assert!(marker.contains(expected), "missing {expected:?}: {marker}");
        }
        // 模型摘要正文进 marker（压缩后被压区由它承载）；机械层不宣称语义保全。
        assert!(marker.contains(summary), "{marker}");
        assert!(marker.contains("不宣称语义保全"), "{marker}");
        // 审查修正批（2026-09-15，审查 P2③）：四项指针必须**逐项标注载体到
        // 哪一层**——被压区逐字原文只在 journal（drain 后不在 messages/sidecar，
        // compaction 存档只存摘要与指针、台账行 300 字符上限），不得再写
        // 「逐字原文见四项指针」的过承诺。
        // v8 实现批（2026-09-16，审查 R-4）：模板轨（本钉，`local_face_full=false`）
        // 口径不变；按块压缩/截断路径置真 ⇒ 文案改口为「含被处理分块的逐字原文」。
        for expected in [
            "不含被处理分块逐字原文",
            "每字段 300 字符摘要行",
            "逐字原文的权威载体",
            "被压区不在其中",
        ] {
            assert!(marker.contains(expected), "missing {expected:?}: {marker}");
        }
        assert!(marker.starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX));
        // 缺项如实渲染「（无）」（不虚构指针）。
        let empty = build_model_summary_marker(
            "compaction-RUN-LOC-0002",
            &"b".repeat(64),
            Path::new(".gsa/compaction/x.md"),
            summary,
            "model_selected",
            1,
            None,
            2,
            None,
            false,
            None,
            &LocatorPointers::default(),
        );
        assert!(empty.contains("台账 [seq] 区间: （无）"), "{empty}");
        assert!(empty.contains("journal: run=（无） seq=（无）"), "{empty}");
        assert!(empty.contains("会话档案（sidecar）: （无）"), "{empty}");
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
            None,
            None,
            test_locators(),
        );
        assert!(marker.contains("机制失败：缩减守卫连续不满足"));
        assert!(marker.contains("存档写入失败：摘要未落盘"));
        assert!(marker.contains("黑板会话: （无）"));
        assert!(crate::prompt::is_restore_retained_block(&marker));
    }

    // ---- P2-14 S1：v0.3 折叠视图快照 marker（2026-09-04，ADR-0010
    // §14.54 / 设计稿 §2、§7 矩阵 4–6）----

    const FOLD_ID: &str = "compaction-RUN-V03-001";

    /// v7（S1 修订批）：测试用定位指针（None 项如实渲染「（无）」）。
    fn test_locators() -> &'static LocatorPointers {
        static CELL: std::sync::OnceLock<LocatorPointers> = std::sync::OnceLock::new();
        CELL.get_or_init(LocatorPointers::default)
    }

    fn fold_input(budget: CompactionBudget, r_keep: u64) -> FoldSnapshotInput<'static> {
        FoldSnapshotInput {
            id: FOLD_ID,
            archive_path: Path::new(".gsa/compaction/x.md"),
            session: Some("SESSION-v03"),
            rounds_dropped: 3,
            round_from: Some(1),
            r_keep,
            ctx: FoldSnapshotCtx {
                current_round: 200,
                current_domain: orz_assurance::lif::Domain::Normal,
            },
            guard_failed: false,
            archive_write_failed: false,
            ledger_note: None,
            frozen_ledger: None,
            // v7（S1 修订批）：直调测试的定位指针（None 项如实渲染「（无）」）。
            locators: test_locators(),
            budget,
        }
    }

    /// §7 矩阵 5 + 空态：全空黑板 → B/C/D 统一「（无）」，A–E 块齐全、
    /// 总量有界、digest 绑定、restore 保留块识别不变。
    #[test]
    fn fold_snapshot_empty_blackboard_blocks_and_bounds() {
        let bb = SharedBlackboard::new();
        let out =
            build_fold_snapshot_marker(&bb.read(), &fold_input(CompactionBudget::default(), 1));
        assert!(
            out.marker.starts_with("[前文上下文已压缩 v0.3]"),
            "{}",
            out.marker
        );
        for head in [
            "== 近窗明细（round < r_keep，已排除保留尾） ==",
            "== 旧段聚合（≤30 条标注行） ==",
            "== 失败目标聚合 ==",
            "== 查询指针 ==",
            "[/前文上下文已压缩]",
        ] {
            assert!(out.marker.contains(head), "missing {head}: {}", out.marker);
        }
        // B/C/D 全空 → 「（无）」出现 ≥3 次（每块一次）。
        assert!(
            out.marker.matches(NOTES_FACTS_EMPTY).count() >= 3,
            "{}",
            out.marker
        );
        assert!(out.marker.contains("sha256:"));
        assert!(out.marker.contains("保留尾首轮 r_keep=1"));
        assert!(out.marker.chars().count() <= CompactionBudget::default().total_chars);
        assert!(crate::prompt::is_restore_retained_block(&out.marker));
        assert!(!out.archive.contains("D 块溢出补全"));
        assert_eq!(out.failure_annex.len(), 0);
    }

    /// §7 矩阵 1（装配级）：round ≥ r_keep 的行（保留尾）不进 marker——
    /// B 明细/归档文本均不得携带保留尾内容。
    #[test]
    fn fold_snapshot_excludes_retained_tail_rows() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            for r in 1..=12u64 {
                let text = if r < 5 {
                    format!("kept-row-{r}")
                } else {
                    format!("retained-row-{r}")
                };
                w.exec.results.push(crate::blackboard::ExecEntry::stamped(
                    text,
                    r,
                    orz_assurance::lif::Domain::Normal,
                    format!("2026-09-04T00:00:{r:02}Z"),
                ));
            }
        }
        let out =
            build_fold_snapshot_marker(&bb.read(), &fold_input(CompactionBudget::default(), 5));
        assert!(out.marker.contains("kept-row-1"), "{}", out.marker);
        assert!(
            !out.marker.contains("retained-row-"),
            "保留尾行不得进 marker: {}",
            out.marker
        );
        assert!(
            !out.archive.contains("retained-row-"),
            "存档同口径不得含保留尾行: {}",
            out.archive
        );
    }

    /// §7 矩阵 3/6（装配级）：C 条数上限 + 溢出指针；总量预算封顶不静默
    /// 丢失（B 裁切补指针）；未达 T/W 阈值也强制折叠且有界（快照强制）。
    #[test]
    fn fold_snapshot_caps_c_segments_and_total_budget() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            // 40 行交替域段 → 40 个段标注候选（当前轮 200 在 K 窗外，
            // 仅尾部 20% 展开，其余折叠）。
            for r in 1..=40u64 {
                let domain = if r % 2 == 0 {
                    orz_assurance::lif::Domain::Pressure
                } else {
                    orz_assurance::lif::Domain::Normal
                };
                w.exec.results.push(crate::blackboard::ExecEntry::stamped(
                    format!("exec-{r} {}", "x".repeat(180)),
                    r,
                    domain,
                    format!("2026-09-04T00:00:{r:02}Z"),
                ));
            }
        }
        let budget = CompactionBudget {
            detail_chars: 300,
            segment_lines: 2,
            total_chars: 1_800,
            failure_chars: 300,
        };
        let out = build_fold_snapshot_marker(&bb.read(), &fold_input(budget, 100));
        assert!(
            out.marker.chars().count() <= budget.total_chars,
            "marker 超总量预算: {} / {}",
            out.marker.chars().count(),
            budget.total_chars
        );
        assert!(
            out.stats.c_omitted_segments > 0,
            "C 必须省略段并给指针: {:?}",
            out.stats
        );
        assert!(out.marker.contains("段因条数上限"), "{}", out.marker);
        // B 块预算（300 字符）必然裁掉部分分区 → 指针不静默。
        assert!(
            out.marker.contains("因块预算") || out.marker.contains("省略"),
            "{}",
            out.marker
        );
        assert!(out.archive.contains("== 近窗明细") || out.archive.contains("## 近窗明细"));
    }

    /// §7 矩阵 7（D annex）：failure_agg 溢出随存档补全段保存，marker 指针
    /// 指向存档补全段（可回查）。
    #[test]
    fn fold_snapshot_failure_annex_persists_in_archive() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            for i in 0..120 {
                w.failure_agg.record(
                    "cmd_target",
                    &format!("id-{i}"),
                    &format!("cmd {i} {}", "长".repeat(90)),
                    "tool_timeout",
                    i as f64,
                    i as u64 + 1,
                    orz_assurance::lif::Domain::Start,
                );
            }
        }
        let budget = CompactionBudget {
            failure_chars: 300,
            ..CompactionBudget::default()
        };
        let out = build_fold_snapshot_marker(&bb.read(), &fold_input(budget, 100));
        assert!(out.stats.d_omitted_rows > 0, "{:?}", out.stats);
        assert!(
            out.marker.contains("见摘要存档失败目标聚合补全段"),
            "{}",
            out.marker
        );
        assert!(
            out.archive.contains("## 失败目标聚合（D 块溢出补全）"),
            "{}",
            out.archive
        );
        assert!(!out.failure_annex.is_empty());
        for row in &out.failure_annex {
            assert!(out.archive.contains(row.as_str()), "annex missing: {row}");
        }
    }
}
