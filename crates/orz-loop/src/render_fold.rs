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
//! `blackboard_fold` 事件，R3）。各分区行文本的组装在 epoch.rs 消费本模块。

use orz_assurance::lif::Domain;

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
}
