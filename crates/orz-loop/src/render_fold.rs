//! P2-13 B2 渲染折叠（render fold）核心纯函数（2026-09-03）。
//!
//! 设计权威：`BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md`
//! v0.8 §9/§11/§12（ADR-0010 §14.52）——方案 B：黑板存储零改写，折叠是
//! `blackboard_read` 的纯渲染语义。可折叠分区（exec / edits / tool_actions）
//! 的记录按写时 (round, domain) 章（B1）分段：
//!
//! - 默认视图（折叠态）展开子集 = 当前域段 ∪ 最近 K 轮 ∪ 最近 20% 行
//!   （并集；§11.1），更早内容按域段聚合为标注行
//!   `[域段 normal r1–r30 · N 条 · 摘要预览]`；
//! - 显式展开 = 折叠态 + `domain`+`round_from`/`round_to` 目标段行（R2，
//!   与 `receipt_id`/`since` 互斥 fail loud）；
//! - 旧无章行（round=0/domain=None）归独立 `pre-stamp` 段：标注 = 计数 +
//!   时间范围，只能经 `since`/`receipt_id` 展开（R1）；
//! - 触发 = 分区 live 字符 ≥ T（64K 默认）或对话黑板 live 紧凑 JSON
//!   字节 ≥ W（10MiB 默认，v0.7 存储字节口径），T/W/K 均可 env 覆盖。
//!
//! 本模块只承载机械纯函数与参数定档：分段、展开子集、域名校验、展开参数
//! 解析与触发判定，不含任何写面/事件/持久化（折叠零持久化，无
//! `blackboard_fold` 事件，R3）。本模块承载机械纯函数、参数定档以及可折叠分区
//! （exec / edits / tool_actions）的折叠视图装配与压缩快照计算。

use orz_assurance::lif::Domain;

use crate::blackboard::{Blackboard, EditRecord, ExecSection, ToolActionRecord};
use crate::summary::truncate_chars;

/// T：单分区折叠触发阈值（字符，v0.7 定档 64K；渲染口径）。
pub const DEFAULT_FOLD_PARTITION_CHARS: usize = 64_000;
/// W：对话黑板 live 预算（存储字节，v0.7 定档 10 MiB；侧车紧凑 JSON）。
pub const DEFAULT_FOLD_BOARD_BYTES: usize = 10 * 1024 * 1024;
/// K：默认展开的最近轮数（v0.4 定档 10）。
pub const DEFAULT_FOLD_TAIL_ROUNDS: u64 = 10;
/// 默认展开的最近行占比下限（v0.4 定档 20%；最近 K 轮行数不足时放宽）。
pub const DEFAULT_FOLD_TAIL_ROWS_PERCENT: usize = 20;

/// env 覆盖名（§11.1 落地形态：编译期默认 + env 覆盖，沿用 ORZ_* 模式）。
pub const FOLD_PARTITION_CHARS_ENV: &str = "ORZ_BLACKBOARD_FOLD_PARTITION_CHARS";
pub const FOLD_BOARD_BYTES_ENV: &str = "ORZ_BLACKBOARD_LIVE_BUDGET_BYTES";
pub const FOLD_TAIL_ROUNDS_ENV: &str = "ORZ_BLACKBOARD_FOLD_TAIL_ROUNDS";

/// 折叠参数（T/W/K/20%）：生产默认 + env 覆盖；测试直接构造，绕开进程级
/// env 的并发竞态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldParams {
    pub partition_threshold_chars: usize,
    pub board_bytes_threshold: usize,
    pub tail_rounds: u64,
    pub tail_rows_percent: usize,
}

impl Default for FoldParams {
    fn default() -> Self {
        Self {
            partition_threshold_chars: DEFAULT_FOLD_PARTITION_CHARS,
            board_bytes_threshold: DEFAULT_FOLD_BOARD_BYTES,
            tail_rounds: DEFAULT_FOLD_TAIL_ROUNDS,
            tail_rows_percent: DEFAULT_FOLD_TAIL_ROWS_PERCENT,
        }
    }
}

impl FoldParams {
    /// 编译期默认 + env 覆盖：缺失/非法（非正整数）/0 一律回退默认。
    pub fn from_env() -> Self {
        let positive_usize = |env: &str, default: usize| -> usize {
            std::env::var(env)
                .ok()
                .and_then(|v| v.trim().parse::<usize>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(default)
        };
        let positive_u64 = |env: &str, default: u64| -> u64 {
            std::env::var(env)
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(default)
        };
        let mut p = Self::default();
        p.partition_threshold_chars =
            positive_usize(FOLD_PARTITION_CHARS_ENV, p.partition_threshold_chars);
        p.board_bytes_threshold = positive_usize(FOLD_BOARD_BYTES_ENV, p.board_bytes_threshold);
        p.tail_rounds = positive_u64(FOLD_TAIL_ROUNDS_ENV, p.tail_rounds);
        p
    }

    /// 最近行占比下限（20%）→ 行数（向上取整，总行数为 0 时保持 0）。
    pub fn tail_row_count(&self, total: usize) -> usize {
        let percent = self.tail_rows_percent.min(100);
        if total == 0 || percent == 0 {
            return 0;
        }
        (total * percent).div_ceil(100)
    }
}

/// 折叠态触发判定（§9.2.4）：分区 live 字符 ≥ T，或对话黑板 live 合计
/// （紧凑 JSON 字节，v0.7 存储口径）≥ W。两者是 OR——后者保证会话晚期
/// （整板逼近 W）即便小分区也进入折叠视图。
pub fn fold_triggered(partition_chars: usize, board_bytes: usize, p: &FoldParams) -> bool {
    partition_chars >= p.partition_threshold_chars || board_bytes >= p.board_bytes_threshold
}

/// 展开目标（R2）：`domain` + `round_from`/`round_to`（含边界，相等 = 单轮）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldExpand {
    pub domain: Domain,
    pub round_from: u64,
    pub round_to: u64,
}

impl FoldExpand {
    /// 行是否落入展开目标（pre-stamp 行 round=0 恒不命中——R1：pre-stamp
    /// 只能经 since/receipt_id 展开）。
    pub fn matches(&self, round: u64, domain: Option<Domain>) -> bool {
        domain == Some(self.domain)
            && round >= 1
            && round >= self.round_from
            && round <= self.round_to
    }
}

/// 域名字符串 → LIF 域（R2 展开参数合法枚举；与 temporal `as_str` 同词汇）。
pub fn parse_domain_name(s: &str) -> Option<Domain> {
    match s {
        "start" => Some(Domain::Start),
        "normal" => Some(Domain::Normal),
        "pressure" => Some(Domain::Pressure),
        "low_progress" => Some(Domain::LowProgress),
        "stuck" => Some(Domain::Stuck),
        _ => None,
    }
}

/// 从 `blackboard_read` 参数解析展开三元组（R2 组合守卫的第一层）：
/// - `domain` / `round_from` / `round_to` 必须同时给出（三者缺一显式报错）；
/// - `domain` 必须是非空合法域名；round 必须是 ≥1 的整数且 from ≤ to；
/// - 三个参数都未提供 = `Ok(None)`（普通折叠态/全量读取）。
///
/// 与 `receipt_id` / `since` / `epoch` 的互斥由调用方（host_exec）在第二层
/// 校验（fail loud，绝不静默忽略）。
pub fn parse_fold_expand(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<FoldExpand>, String> {
    let has = |k: &str| args.contains_key(k);
    if !has("domain") && !has("round_from") && !has("round_to") {
        return Ok(None);
    }
    let mut missing = Vec::new();
    for key in ["domain", "round_from", "round_to"] {
        if !has(key) {
            missing.push(key);
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "invalid blackboard_read expand: domain/round_from/round_to 必须同时给出（缺 {}）；\
             省略全部三个参数读取默认折叠视图",
            missing.join(", ")
        ));
    }
    let domain = match args.get("domain").and_then(serde_json::Value::as_str) {
        Some(s) => match parse_domain_name(s) {
            Some(d) => d,
            None => {
                return Err(format!(
                    "invalid blackboard_read domain: {} — domain 必须是 \
                     start|normal|pressure|low_progress|stuck 之一",
                    args["domain"]
                ));
            }
        },
        None => {
            return Err(format!(
                "invalid blackboard_read domain: {} — domain 必须是字符串 \
                 （start|normal|pressure|low_progress|stuck）",
                args["domain"]
            ));
        }
    };
    let round = |key: &str| -> Result<u64, String> {
        match args.get(key) {
            Some(v) => match v.as_u64() {
                Some(n) if n >= 1 => Ok(n),
                _ => Err(format!(
                    "invalid blackboard_read {key}: {v} — 必须是 ≥1 的整数（会话相对轮号）"
                )),
            },
            None => unreachable!("missing checked above"),
        }
    };
    let round_from = round("round_from")?;
    let round_to = round("round_to")?;
    if round_to < round_from {
        return Err(format!(
            "invalid blackboard_read round range: round_to={round_to} < round_from={round_from} \
             — 轮数范围含边界，round_to 不得小于 round_from（相等 = 单轮）"
        ));
    }
    Ok(Some(FoldExpand {
        domain,
        round_from,
        round_to,
    }))
}

/// 折叠行元数据（epoch.rs 把各分区记录映射为行文本 + 章后交给本模块分段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldRowMeta {
    pub text: String,
    pub round: u64,
    pub domain: Option<Domain>,
}

/// 段类型：`Domain(d)` = 带章连续段；`PreStamp` = 旧无章独立段（R1：不并入
/// normal，标注只给计数 + 时间范围）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    PreStamp,
    Domain(Domain),
}

/// 一段连续的同类行（`start..end`，end 不含）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentRun {
    pub start: usize,
    pub end: usize,
    pub kind: SegmentKind,
}

/// 按存储序切段：连续同域行并一段（R6：跨段同域不合并、标注按时间序），
/// 旧无章行（domain=None）为独立 pre-stamp 段。
pub fn segment_runs(rows: &[FoldRowMeta]) -> Vec<SegmentRun> {
    let mut out: Vec<SegmentRun> = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        let kind = match rows[i].domain {
            Some(d) => SegmentKind::Domain(d),
            None => SegmentKind::PreStamp,
        };
        let mut j = i + 1;
        while j < rows.len() {
            let next = match rows[j].domain {
                Some(d) => SegmentKind::Domain(d),
                None => SegmentKind::PreStamp,
            };
            if next != kind {
                break;
            }
            j += 1;
        }
        out.push(SegmentRun {
            start: i,
            end: j,
            kind,
        });
        i = j;
    }
    out
}

/// 折叠布局：分段 + 每行是否在默认展开子集。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldLayout {
    pub segments: Vec<SegmentRun>,
    pub expanded: Vec<bool>,
}

/// 计算默认展开子集（§9.2.1/§11.1）：
/// 1. 当前域段 = 轮区间含 `current_round` 的最晚段（“当前轮所在域连续段”；
///    若尚无行写于当前轮，回退到最晚段——同域驻留期内轮号领先于写行的
///    正常读取窗口）；
/// 2. 最近 K 轮 = 行 round ∈ [current_round−K+1, current_round]（含边界）；
/// 3. 最近 20% 行 = 存储序尾部的 `tail_row_count` 行（放宽下限，保证
///    短尾也可见）。
///
/// 三者并集；pre-stamp 行 round=0 不计入“最近轮”，但可作为“最近 20% 行”
/// 尾行成员（存储序视角的近期行）。
pub fn fold_layout(
    rows: &[FoldRowMeta],
    current_round: u64,
    current_domain: Domain,
    p: &FoldParams,
) -> FoldLayout {
    let segments = segment_runs(rows);
    let mut expanded = vec![false; rows.len()];
    if rows.is_empty() || segments.is_empty() {
        return FoldLayout { segments, expanded };
    }
    // 当前域段：优先取轮区间含 current_round 的最晚段；否则回退最晚段
    // （实现决策：域机器刚迁移但尚无该域写行时，最晚段是“当前轮所在域
    // 连续段”在无行轮上的最佳近似；登记于 B2 审计）。
    let mut current_segment = segments.len() - 1;
    for (idx, seg) in segments.iter().enumerate().rev() {
        let (mut lo, mut hi) = (u64::MAX, 0u64);
        for r in &rows[seg.start..seg.end] {
            if r.round >= 1 {
                lo = lo.min(r.round);
                hi = hi.max(r.round);
            }
        }
        if lo != u64::MAX && lo <= current_round && current_round <= hi {
            current_segment = idx;
            break;
        }
    }
    // `current_domain` 留作语义锚点：含当前轮的段若存在，其尾域与 LIF
    // 当前域同源（写时盖章）；无行轮回退最晚段时不另作域过滤（见上注）。
    let _ = current_domain;
    for slot in expanded[segments[current_segment].start..segments[current_segment].end].iter_mut()
    {
        *slot = true;
    }
    // 最近 K 轮（round 窗口含边界；current_round=0 时窗口为空，无行命中）。
    if current_round >= 1 {
        let lower = current_round.saturating_sub(p.tail_rounds.saturating_sub(1));
        for (i, r) in rows.iter().enumerate() {
            // pre-stamp 行 round=0 不属于任何轮窗口（R1：只能 since/receipt_id）。
            if r.round >= 1 && r.round >= lower && r.round <= current_round {
                expanded[i] = true;
            }
        }
    }
    // 最近 20% 行（尾部放宽下限）。
    let tail = p.tail_row_count(rows.len());
    for slot in expanded[rows.len().saturating_sub(tail)..].iter_mut() {
        *slot = true;
    }
    FoldLayout { segments, expanded }
}

/// 把显式展开目标并入默认展开子集（R2：显式展开 = 折叠态 + 目标段行；
/// 目标行在默认子集外时追加）。返回新的展开标记。
pub fn merge_expand(layout: &FoldLayout, rows: &[FoldRowMeta], q: &FoldExpand) -> Vec<bool> {
    let mut expanded = layout.expanded.clone();
    for (i, r) in rows.iter().enumerate() {
        if q.matches(r.round, r.domain) {
            expanded[i] = true;
        }
    }
    expanded
}

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
pub fn render_capped_rows(label: &str, lines: Vec<String>) -> String {
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
pub fn cap_fold_view(
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
    let layout = fold_layout(&rows, current_round, current_domain, params);
    let expanded = match expand {
        Some(q) => merge_expand(&layout, &rows, q),
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
    let layout = fold_layout(&rows, current_round, current_domain, params);
    let expanded = match expand {
        Some(q) => merge_expand(&layout, &rows, q),
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
    let layout = fold_layout(&rows, current_round, current_domain, params);
    let expanded = match expand {
        Some(q) => merge_expand(&layout, &rows, q),
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
    let mut layout = fold_layout(rows, current_round, current_domain, params);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(text: &str, round: u64, domain: Option<Domain>) -> FoldRowMeta {
        FoldRowMeta {
            text: text.to_string(),
            round,
            domain,
        }
    }

    #[test]
    fn defaults_match_parameter_table() {
        let p = FoldParams::default();
        assert_eq!(p.partition_threshold_chars, 64_000);
        assert_eq!(p.board_bytes_threshold, 10 * 1024 * 1024);
        assert_eq!(p.tail_rounds, 10);
        assert_eq!(p.tail_rows_percent, 20);
        assert_eq!(p.tail_row_count(0), 0);
        assert_eq!(p.tail_row_count(3), 1);
        assert_eq!(p.tail_row_count(10), 2);
        assert_eq!(p.tail_row_count(1000), 200);
    }

    #[test]
    fn fold_trigger_is_or_of_t_and_w() {
        let p = FoldParams::default();
        assert!(!fold_triggered(10_000, 1024, &p));
        assert!(fold_triggered(64_000, 1024, &p));
        assert!(fold_triggered(10_000, 10 * 1024 * 1024, &p));
    }

    #[test]
    fn domain_parse_accepts_five_names_only() {
        for (name, expected) in [
            ("start", Domain::Start),
            ("normal", Domain::Normal),
            ("pressure", Domain::Pressure),
            ("low_progress", Domain::LowProgress),
            ("stuck", Domain::Stuck),
        ] {
            assert_eq!(parse_domain_name(name), Some(expected));
        }
        assert_eq!(parse_domain_name("pre_stamp"), None);
        assert_eq!(parse_domain_name("NORMAL"), None);
        assert_eq!(parse_domain_name(""), None);
    }

    #[test]
    fn expand_parse_all_or_none_fail_loud() {
        let mut args = serde_json::Map::new();
        assert_eq!(parse_fold_expand(&args).unwrap(), None);
        args.insert("domain".into(), "normal".into());
        let err = parse_fold_expand(&args).unwrap_err();
        assert!(
            err.contains("round_from") && err.contains("round_to"),
            "{err}"
        );
        args.insert("round_from".into(), 10.into());
        let err = parse_fold_expand(&args).unwrap_err();
        assert!(err.contains("round_to"), "{err}");
        args.insert("round_to".into(), 20.into());
        let q = parse_fold_expand(&args).unwrap().unwrap();
        assert_eq!(q.domain, Domain::Normal);
        assert_eq!((q.round_from, q.round_to), (10, 20));
    }

    #[test]
    fn expand_parse_rejects_bad_values() {
        let mut base = serde_json::Map::new();
        base.insert("domain".into(), "normal".into());
        base.insert("round_from".into(), 10.into());
        base.insert("round_to".into(), 20.into());
        let mut args = base.clone();
        args.insert("domain".into(), "bogus".into());
        assert!(parse_fold_expand(&args).unwrap_err().contains("domain"));
        let mut args = base.clone();
        args.insert("domain".into(), 3.into());
        assert!(parse_fold_expand(&args).unwrap_err().contains("domain"));
        let mut args = base.clone();
        args.insert("round_from".into(), 0.into());
        assert!(parse_fold_expand(&args).unwrap_err().contains("round_from"));
        let mut args = base.clone();
        args.insert("round_to".into(), "x".into());
        assert!(parse_fold_expand(&args).unwrap_err().contains("round_to"));
        let mut args = base;
        args.insert("round_to".into(), 9.into());
        assert!(
            parse_fold_expand(&args)
                .unwrap_err()
                .contains("round_to=9 < round_from=10")
        );
    }

    #[test]
    fn segment_runs_split_by_domain_and_prestamp() {
        let rows = vec![
            meta("a", 0, None),                   // pre-stamp
            meta("b", 1, Some(Domain::Normal)),   // normal r1
            meta("c", 5, Some(Domain::Normal)),   // normal 并入
            meta("d", 6, Some(Domain::Pressure)), // pressure 开新段
            meta("e", 7, Some(Domain::Normal)),   // normal 回切独立段（R6 不合并）
        ];
        let segs = segment_runs(&rows);
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].kind, SegmentKind::PreStamp);
        assert_eq!(segs[0].start..segs[0].end, 0..1);
        assert_eq!(segs[1].kind, SegmentKind::Domain(Domain::Normal));
        assert_eq!(segs[1].start..segs[1].end, 1..3);
        assert_eq!(segs[2].kind, SegmentKind::Domain(Domain::Pressure));
        assert_eq!(segs[2].start..segs[2].end, 3..4);
        assert_eq!(segs[3].kind, SegmentKind::Domain(Domain::Normal));
        assert_eq!(segs[3].start..segs[3].end, 4..5);
    }

    #[test]
    fn layout_expands_current_segment_and_folds_older_domain() {
        // normal r1–r12（12 行）→ pressure r13–r18（6 行），当前轮 r18/pressure。
        let mut rows = Vec::new();
        for r in 1..=12 {
            rows.push(meta(&format!("n{r}"), r, Some(Domain::Normal)));
        }
        for r in 13..=18 {
            rows.push(meta(&format!("p{r}"), r, Some(Domain::Pressure)));
        }
        let p = FoldParams {
            tail_rounds: 5,
            tail_rows_percent: 20,
            ..FoldParams::default()
        };
        let layout = fold_layout(&rows, 18, Domain::Pressure, &p);
        // 当前域段 = pressure r13–r18 全展开。
        for i in 12..18 {
            assert!(layout.expanded[i], "row {i} should be in current segment");
        }
        // 最近 5 轮 = r14–r18（pressure 已在当前段；无新增）。
        // 最近 20% 行 = ceil(18×0.2)=4 → 尾部 4 行已含。
        // normal r1–r12 全部折叠。
        for i in 0..12 {
            assert!(!layout.expanded[i], "row {i} should be folded");
        }
        assert_eq!(layout.segments.len(), 2);
    }

    #[test]
    fn layout_tail_rounds_reaches_into_older_segment() {
        // normal r1–r45 → pressure r46–r48 → normal r49–r50；当前 r50/normal。
        let mut rows = Vec::new();
        for r in 1..=45 {
            rows.push(meta(&format!("n{r}"), r, Some(Domain::Normal)));
        }
        for r in 46..=48 {
            rows.push(meta(&format!("p{r}"), r, Some(Domain::Pressure)));
        }
        for r in 49..=50 {
            rows.push(meta(&format!("n2{r}"), r, Some(Domain::Normal)));
        }
        let p = FoldParams {
            tail_rounds: 10,
            tail_rows_percent: 20,
            ..FoldParams::default()
        };
        let layout = fold_layout(&rows, 50, Domain::Normal, &p);
        // 当前域段 = 最新 normal 段（r49–r50）。
        assert!(layout.expanded[48] && layout.expanded[49]);
        // 最近 10 轮 = r41–r50：pressure r46–r48 与旧 normal 段尾部
        // r41–r45 也进入展开子集 → 旧 normal 段部分折叠。
        for i in 40..50 {
            assert!(layout.expanded[i], "row {i} in last-10-round window");
        }
        assert!(!layout.expanded[0], "r1 折叠");
        assert!(!layout.expanded[39], "r40 折叠");
        assert_eq!(layout.segments.len(), 3);
    }

    #[test]
    fn single_domain_partition_expands_all_rows_without_annotations() {
        // B2 复审登记（2026-09-03）：设计 §9.2.1 口径——当前域段整段展开；
        // 单域（单段）会话不存在“更早域段”，折叠视图不产生标注行，旧内容
        // 只受视图字符 cap 约束并经 domain+round 展开回查（无段标注总览
        // 属该口径的既定后果，遥测复核见设计 §13.3）。
        let mut rows = Vec::new();
        for r in 1..=300 {
            rows.push(meta(&format!("n{r}"), r, Some(Domain::Normal)));
        }
        let p = FoldParams {
            tail_rounds: 10,
            tail_rows_percent: 20,
            ..FoldParams::default()
        };
        let layout = fold_layout(&rows, 300, Domain::Normal, &p);
        assert_eq!(layout.segments.len(), 1);
        assert!(
            layout.expanded.iter().all(|e| *e),
            "single segment = current segment → 整段展开（无折叠行即无标注）"
        );
    }

    #[test]
    fn prestamp_rows_stay_folded_until_no_stamped_rows() {
        let mut rows = vec![meta("legacy1", 0, None), meta("legacy2", 0, None)];
        for r in 1..=8 {
            rows.push(meta(&format!("s{r}"), r, Some(Domain::Start)));
        }
        let p = FoldParams {
            tail_rounds: 10,
            tail_rows_percent: 20,
            ..FoldParams::default()
        };
        let layout = fold_layout(&rows, 8, Domain::Start, &p);
        // 当前段 = start 段全展开；尾部 20% 落在带章行（存储序在 pre-stamp
        // 之后）——pre-stamp 行保持折叠（R1：只能 since/receipt_id 展开）。
        assert!(!layout.expanded[0] && !layout.expanded[1]);
        assert!(layout.expanded.iter().skip(2).all(|e| *e));
        assert_eq!(layout.segments[0].kind, SegmentKind::PreStamp);
        // 全 pre-stamp（无带章行）时回退最晚段 = pre-stamp 段整体展开：
        // 折叠视图对全旧行分区退化为全量（与无章会话语义一致）。
        let legacy_only = vec![meta("a", 0, None), meta("b", 0, None)];
        let l2 = fold_layout(&legacy_only, 8, Domain::Normal, &p);
        assert!(l2.expanded.iter().all(|e| *e));
    }

    #[test]
    fn merge_expand_adds_target_rows_only() {
        let mut rows = Vec::new();
        for r in 1..=30 {
            rows.push(meta(&format!("n{r}"), r, Some(Domain::Normal)));
        }
        let p = FoldParams::default();
        let base = fold_layout(&rows, 30, Domain::Normal, &p);
        assert!(base.expanded.iter().all(|e| *e), "single-segment 全展开");
        // 人为把 r1–r25 置为折叠，展开 r1–r3 后应恢复。
        let mut folded = base.clone();
        for i in 0..25 {
            folded.expanded[i] = false;
        }
        let q = FoldExpand {
            domain: Domain::Normal,
            round_from: 1,
            round_to: 3,
        };
        let merged = merge_expand(&folded, &rows, &q);
        assert!(merged[0] && merged[1] && merged[2]);
        assert!(!merged[3], "r4 仍在折叠区");
        // pre-stamp / 异域 / 轮号范围外不命中。
        let q2 = FoldExpand {
            domain: Domain::Pressure,
            round_from: 1,
            round_to: 3,
        };
        let merged2 = merge_expand(&folded, &rows, &q2);
        assert!(!merged2[0]);
        let q3 = FoldExpand {
            domain: Domain::Normal,
            round_from: 40,
            round_to: 50,
        };
        let merged3 = merge_expand(&folded, &rows, &q3);
        assert_eq!(merged3, folded.expanded, "范围外展开不新增任何行");
    }

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
