//! 0am P8（2026-10-03，S2 刺激面路由表实现批）：刺激路由器——动作类标签器、
//! 拒绝类分组与 [`stimulus_targets`] 纯分派。设计权威＝
//! `docs/LIF_RLI_STIMULUS_ROUTING_TABLE_S2_DESIGN_2026-10-03.md`（父仓
//! docs/，不在本仓；v1.1；S3 重放 J1–J5 已收，175 批）。原则边界：只定义
//! **通道看见什么**
//! （P1–P5/P7/P8 标签面）；**零触发条件**（P9——阈值/k/域机/提醒沿全部在
//! 既有动力学内，本模块不携带任何「当 X 即 Y」规则）。
//!
//! 同源复用（S2 §5）：生产喂入点（orz-loop）与本crate重放件
//! （`examples/rli_shadow_replay`）共用本模块的标签器与分派——对拍一致性
//! 由构造保证。工具名字面单一源（0ao 扫描钉）：在册常量经
//! `crate::tool_names` 引用。

use crate::lif::channels::{
    ChannelKind, SLOW_W_MAX, SLOW_WALL_MS_THRESHOLD, ToolEvent, ToolOutcome,
};
use crate::tool_names::{BLACKBOARD_WRITE_TOOL_NAME, CONTEXT_COMPRESS_TOOL_NAME};

/// S2 §4.1 动作类（闭集）。`None`（未路由）＝喂入点未携带标签——按旧四值
/// 语义处理（Success→Prog、wall>阈→Slow；合成事件与旧测试的兼容语义）；
/// 生产喂入点（tool_run 完成臂/ToolError 臂）恒填。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionClass {
    /// 变更类：durable 工件编辑（`search_replace`）。
    Mutate,
    /// 验证类：`run_tests` 工具＋终端验证词表（[`VERIFY_LEXICON`]）。
    Verify,
    /// 阅读/检索类（read_file/list_dir/grep/search_tool/web_*/lsp）。
    Retrieve,
    /// 黑板/会话类（blackboard_*/context_compress/compaction_whitelist_add/
    /// todo_write/update_goal/ask_user_question）。
    Session,
    /// 提交类（submit）。当前载体面无 `submit` 工具（保留类，码面
    /// `submit_disabled`/`submit_lane_denied` 拒绝路径使用）。
    Submit,
    /// 中性：其余终端命令与非工具事件——仅计步，不注入任何通道。
    #[default]
    Neutral,
}

impl ActionClass {
    /// 从工具名与终端命令串判类（命令串仅对 `run_terminal_cmd` 参与词表
    /// 匹配；其余工具按名直判）。
    pub fn of_tool(tool: &str, command: Option<&str>) -> Self {
        match tool {
            "search_replace" => ActionClass::Mutate,
            // 0am 审查处置批（2026-10-03，用户裁决）：codex/opencode/hashline
            // 适配器载体的结构化变更工具——否则非 grok_build 载体 Prog 结构
            // 死窗（变更类缺口）。
            "apply_patch" | "write" | "edit" => ActionClass::Mutate,
            "run_tests" => ActionClass::Verify,
            "read_file" | "list_dir" | "grep" | "search_tool" | "web_search" | "web_fetch"
            | "lsp" => ActionClass::Retrieve,
            t if t == BLACKBOARD_WRITE_TOOL_NAME || t == CONTEXT_COMPRESS_TOOL_NAME => {
                ActionClass::Session
            }
            "blackboard_read"
            | "compaction_whitelist_add"
            | "todo_write"
            | "update_goal"
            | "ask_user_question" => ActionClass::Session,
            "submit" => ActionClass::Submit,
            "run_terminal_cmd" => match command {
                Some(cmd) if command_matches_verify_lexicon(cmd) => ActionClass::Verify,
                _ => ActionClass::Neutral,
            },
            _ => ActionClass::Neutral,
        }
    }
}

/// S2 §4.1 终端验证词表（闭集初版）。匹配口径（v1.1）：大小写不敏感子串＋
/// 两侧词边界（字母数字与 `_` 均为词字符——0am 审查处置批 2026-10-03：修
/// `make_x`/`_pytest` 误命中；复合命令的子命令可命中，`Makefile` 不误命中
/// `make`）。词表扩展只随批次修订（闭集纪律，禁实现层临时加词）。
/// 0am 审查处置批（2026-10-03，用户裁决）：`cargo fmt` → `cargo fmt
/// --check`（对齐 S2 设计原意——变更性 `cargo fmt` 不再归验证类）；追加
/// `npm run test`/`nextest`/`pnpm test`/`yarn test`/`bun test`。
pub const VERIFY_LEXICON: [&str; 31] = [
    "cargo test",
    "cargo build",
    "cargo check",
    "cargo clippy",
    "cargo fmt --check",
    "pytest",
    "python -m pytest",
    "python -m unittest",
    "unittest",
    "make",
    "npm test",
    "npm run build",
    "npm run test",
    "npx tsc",
    "nextest",
    "pnpm test",
    "yarn test",
    "bun test",
    "go test",
    "go build",
    "go vet",
    "gradle",
    "mvn",
    "dotnet test",
    "dotnet build",
    "cmake --build",
    "rake",
    "eslint",
    "ruff",
    "pylint",
    "mypy",
];

/// 0am 审查处置批（2026-10-03，用户裁决）：**首词守卫表**——只读/中性
/// 命令首词（`grep -rn pytest .` 退出码 1 不再假判 Verify）。闭集纪律同
/// 词表（禁实现层临时加词）；分段语义见 [`command_matches_verify_lexicon`]。
pub const VERIFY_NON_ACTION_HEADS: [&str; 29] = [
    "grep", "rg", "find", "echo", "cat", "git", "ls", "sed", "awk", "head", "tail", "which",
    "where", "man", "less", "wc", "sort", "uniq", "diff", "stat", "file", "du", "df", "ps", "env",
    "printenv", "pwd", "whoami", "date",
];

/// 词边界包含（大小写不敏感）：命中处前一字节与尾后一字节均非词字符
/// （词字符＝ASCII 字母数字或 `_`；0am 审查处置批 2026-10-03 加 `_`——
/// `make_x`/`_pytest` 不再误命中）。
pub fn word_boundary_contains(hay: &str, needle: &str) -> bool {
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return false;
    }
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let eq = |a: u8, b: u8| a.eq_ignore_ascii_case(&b);
    for i in 0..=(h.len() - n.len()) {
        if !h[i..i + n.len()].iter().zip(n).all(|(&a, &b)| eq(a, b)) {
            continue;
        }
        let before_ok = i == 0 || !is_word(h[i - 1]);
        let after = i + n.len();
        let after_ok = after >= h.len() || !is_word(h[after]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// 段首词是否属守卫表（[`VERIFY_NON_ACTION_HEADS`]）：取段内首 token 的
/// 路径 basename（小写化）比对。
fn segment_head_guarded(segment: &str) -> bool {
    let first = segment.split_whitespace().next().unwrap_or("");
    let base = first
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(first)
        .to_ascii_lowercase();
    !base.is_empty() && VERIFY_NON_ACTION_HEADS.contains(&base.as_str())
}

/// 终端命令串的验证词表匹配总口径（0am 审查处置批，2026-10-03）：
/// ① `.exe` 归一（`cargo.exe test`/`python.exe -m pytest` 不再漏命中）；
/// ② 按 `&&`/`||`/`;`/`|` 切段（`||` 优先于 `|`——先切 `&&`/`||` 再切
/// `;`/`|`），每段首词属守卫表则该段不参与词表匹配（grep/echo 假验证
/// 失败剔除），其余段照常逐词匹配；任一非守卫段命中即 Verify。
fn command_matches_verify_lexicon(cmd: &str) -> bool {
    let normalized = cmd.replace(".exe", "");
    let mut segments = vec![normalized.as_str()];
    for sep in ["&&", "||", ";", "|"] {
        let mut next = Vec::new();
        for seg in segments {
            next.extend(seg.split(sep));
        }
        segments = next;
    }
    segments
        .into_iter()
        .filter(|seg| !segment_head_guarded(seg))
        .any(|seg| {
            VERIFY_LEXICON
                .iter()
                .any(|tok| word_boundary_contains(seg, tok))
        })
}

/// S2 §4.2 拒绝类分组（Deny 通道标签维；P8 成因段「拒绝类名＋计数」的
/// 闭集词表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyClass {
    /// 权限/控制票据（permission_*／control_ticket_rejected／control_tool_lane）。
    PermissionTicket,
    /// 计划/车道（plan_*／submit_*／console_*／order_slot_busy）。
    PlanLane,
    /// 门/护栏（锚点失配/封存/退役/候选/轮注入预算）。
    GateGuard,
    /// 检索启用（retrieval_*／nested_subagent）。
    RetrievalEnable,
    /// 写控兜底 block（S2 v1.1 细化：经 call_id join
    /// `write_control_review(review=block)` 或宿主块面判定；warn 不入）。
    WriteControl,
    /// policy_denial 信封标记（无更细码时）。
    PolicyMarker,
    /// 其余（missing_test_runner 等）。
    Other,
}

impl DenyClass {
    /// 从结构化拒绝码分组（[`is_denial_code`] 词表内判类）。
    pub fn of_code(code: &str) -> Self {
        let permission = code.starts_with("permission_")
            || code.starts_with("control_ticket_rejected")
            || code.starts_with("control_tool_lane");
        if permission {
            return DenyClass::PermissionTicket;
        }
        let plan = code.starts_with("plan_")
            || code.starts_with("submit_")
            || code.starts_with("console_")
            || code == "order_slot_busy";
        if plan {
            return DenyClass::PlanLane;
        }
        let guard = code.starts_with("content_anchor")
            || code.starts_with("sealed_tool")
            || code.starts_with("retired_tool")
            || code.ends_with("_candidate_count_unbound")
            || code.ends_with("_candidate_url_missing")
            || code.ends_with("_candidate_cap_exceeded")
            || code.starts_with("round_inject_budget");
        if guard {
            return DenyClass::GateGuard;
        }
        if code.starts_with("retrieval_") || code.starts_with("nested_subagent") {
            return DenyClass::RetrievalEnable;
        }
        DenyClass::Other
    }
}

/// 喂入点携带的机械路由键（S2 §5：`tool_name`/`command` 的**标签化结果**＋
/// 分派所需的载荷事实；journal 已含全部字段，零新增采集面）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StimulusRouting {
    /// 动作类（0am 审查处置批勘误，2026-10-03）：动作类**经路由间接成为
    /// 标签**——Mutate→「变更成功」、Verify→「验证失败」入通道标签环；
    /// 无独立的「动作类计数」渲染面。
    pub class: ActionClass,
    /// Deny 标签维——**喂入点已解析**（结构化拒绝码经
    /// [`DenyClass::of_code`]、写控块＝[`DenyClass::WriteControl`]；
    /// [`is_denial_code`] 判定亦在喂入点）。仅 deny 事件携带。
    pub deny_class: Option<DenyClass>,
    /// 载荷事实：非零 exit（D2 值语义）——H2 填平（非验证类非零退出入 Err）
    /// 的判据位。`timed_out`/host error 由 outcome==Error 承载，不重复。
    pub non_zero_exit: bool,
    /// 载荷事实：写控兜底 block（宿主 ToolError 臂判定；warn 不置位）。
    pub write_control_block: bool,
}

impl StimulusRouting {
    /// 生产喂入点的标准装配。
    pub fn new(class: ActionClass) -> Self {
        Self {
            class,
            deny_class: None,
            non_zero_exit: false,
            write_control_block: false,
        }
    }
}

/// 单事件刺激分派结果（通道 → 注入值；`prog` 为 set 语义）。
///
/// 语义＝S2 路由表 §2/§3（deny 优先；验证通过零注入且 Slow 豁免验证类；
/// 未路由事件回退旧四值语义）。**不含任何触发判断**（P9）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StimulusTargets {
    pub prog_set: bool,
    pub verify: bool,
    pub err: bool,
    pub deny: bool,
    /// Slow 注入（Some(权值)＝wall>60s 的时长加权；P4 先例）。
    pub slow: Option<f64>,
    /// 分派后回读的动作类（未路由＝Neutral 的 legacy 读法不回填）。
    pub class: Option<ActionClass>,
    /// Deny 标签维（P8 成因段用）。
    pub deny_class: Option<DenyClass>,
}

impl StimulusTargets {
    /// 全空（中性事件：仅计步）。
    pub fn neutral() -> Self {
        Self {
            prog_set: false,
            verify: false,
            err: false,
            deny: false,
            slow: None,
            class: None,
            deny_class: None,
        }
    }
}

/// Slow 通道时长权值（P4 先例式、与 orz-loop 生产同式：分钟数 clamp 到
/// [1, `SLOW_W_MAX`]）。0am 审查处置批（2026-10-03）改 `pub`——重放件
/// （`examples/rli_shadow_replay`）复用本函数，消除第三份拷贝。
pub fn slow_weight(wall_ms: u64) -> f64 {
    (wall_ms as f64 / 60_000.0).clamp(1.0, SLOW_W_MAX)
}

/// 单事件刺激分派（S2 路由表 §2/§3 的机械实现；deny 优先）。
///
/// - `routing: None`＝未路由（合成事件/旧调用面）→ 旧四值语义：Error→Err、
///   Deny→Deny、Success→Prog(set)、Other→无；wall>60s→Slow（P4 先例）。
/// - `Some(r)`＝S2 语义：deny（信封/拒绝码/写控块）优先；验证类失败→Verify、
///   通过→零注入且 Slow 豁免；变更类成功→Prog(set)、失败→Err；Error→Err；
///   Other 且非零 exit→Err（H2 填平）；其余中性；Slow＝非验证类 wall>60s。
pub fn stimulus_targets(ev: &ToolEvent) -> StimulusTargets {
    let slow = ev
        .wall_ms
        .filter(|w| *w > SLOW_WALL_MS_THRESHOLD)
        .map(slow_weight);
    let Some(r) = ev.routing else {
        // 旧四值语义（legacy fallback）。
        let (prog_set, err, deny) = match ev.outcome {
            ToolOutcome::Error => (false, true, false),
            ToolOutcome::Deny => (false, false, true),
            ToolOutcome::Success => (true, false, false),
            ToolOutcome::Other => (false, false, false),
        };
        return StimulusTargets {
            prog_set,
            verify: false,
            err,
            deny,
            slow,
            class: None,
            deny_class: None,
        };
    };
    // deny 优先：信封标记（policy_denied）优先；其次喂入点已解析的
    // 拒绝类（结构化拒绝码／写控兜底块）。
    let deny_class = if ev.policy_denied {
        Some(r.deny_class.unwrap_or(DenyClass::PolicyMarker))
    } else if r.write_control_block && r.deny_class.is_none() {
        Some(DenyClass::WriteControl)
    } else {
        r.deny_class
    };
    if deny_class.is_some() {
        return StimulusTargets {
            prog_set: false,
            verify: false,
            err: false,
            deny: true,
            slow,
            class: Some(r.class),
            deny_class,
        };
    }
    if r.class == ActionClass::Verify {
        // 验证类：失败（host error/timeout 或非零 exit）→ Verify；通过 → 零
        // 注入（基线参考语义）；Slow 豁免（S2 v1.1 H4 修正）。
        let failed = matches!(ev.outcome, ToolOutcome::Error) || r.non_zero_exit;
        return StimulusTargets {
            prog_set: false,
            verify: failed,
            err: false,
            deny: false,
            slow: None,
            class: Some(r.class),
            deny_class: None,
        };
    }
    let err = match ev.outcome {
        ToolOutcome::Error => true,
        ToolOutcome::Other => r.non_zero_exit, // H2 填平：非验证类非零退出
        _ => false,
    };
    let prog_set = matches!(ev.outcome, ToolOutcome::Success) && r.class == ActionClass::Mutate;
    StimulusTargets {
        prog_set,
        verify: false,
        err,
        deny: false,
        slow,
        class: Some(r.class),
        deny_class: None,
    }
}

/// 非工具事件总线刺激（Ctx/Infra 通道认领；S2 §3 上下文(4)＋宿主资源(6)＋
/// 传输(1)族）。**零触发语义**——仅「通道看见什么」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusStimulus {
    /// Ctx：`context_compressed`（压缩 marker）。
    ContextCompressed,
    /// Ctx：`ledger_fold_advance`（台账折叠推进）。
    LedgerFoldAdvance,
    /// Infra：`ledger_fold_write_failed`（折叠外联写失败＝机械降级审计）。
    LedgerFoldWriteFailed,
    /// Infra：`transport_retry`（传输健康面）。
    TransportRetry,
    /// Infra：`tool_availability_check` 探针翻转。
    AvailabilityFlip,
    /// Infra：`host_resource_denied`（soft 档拦截）。
    HostResourceDenied,
    /// Infra：`resource_limit_hit`（内核 Job 顶格命中）。
    ResourceLimitHit,
    /// Infra：`host_resource_snapshot` 档位值——**跨档才注入**（v1.1 过滤：
    /// run 首测＝基线不计；引擎记忆上一档位做 crossing 判定）。
    HostResourceSnapshotTier(ResourceTier),
}

/// 宿主资源档位（0z 分档封闭集；`host_resource_snapshot.tier` 的机械映射；
/// 未知档位＝None 不参与跨档判定，journal 照记）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceTier {
    Normal,
    Watch,
    Soft,
    ReclaimDirect,
    Hard,
}

impl ResourceTier {
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(ResourceTier::Normal),
            "watch" => Some(ResourceTier::Watch),
            "soft" => Some(ResourceTier::Soft),
            "reclaim_direct" => Some(ResourceTier::ReclaimDirect),
            "hard" => Some(ResourceTier::Hard),
            _ => None,
        }
    }
}

impl BusStimulus {
    /// 认领通道（0am 审查处置批勘误，2026-10-03：snapshot 跨档过滤已前移
    /// 引擎 [`super::LifEngine::on_bus_event`]——当前全变体可认领；`None`
    /// 为防御性保留）。
    pub fn channel(self) -> Option<ChannelKind> {
        match self {
            BusStimulus::ContextCompressed | BusStimulus::LedgerFoldAdvance => {
                Some(ChannelKind::Ctx)
            }
            BusStimulus::LedgerFoldWriteFailed
            | BusStimulus::TransportRetry
            | BusStimulus::AvailabilityFlip
            | BusStimulus::HostResourceDenied
            | BusStimulus::ResourceLimitHit
            | BusStimulus::HostResourceSnapshotTier(_) => Some(ChannelKind::Infra),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lif::channels::ToolEvent;

    fn routed(class: ActionClass) -> ToolEvent {
        ToolEvent {
            outcome: ToolOutcome::Success,
            wall_ms: None,
            policy_denied: false,
            routing: Some(StimulusRouting::new(class)),
        }
    }

    #[test]
    fn verify_lexicon_word_boundary() {
        assert!(word_boundary_contains(
            "cd /workspace && .venv/bin/python -m unittest discover -s tests",
            "unittest"
        ));
        assert!(!word_boundary_contains("cat Makefile", "make"));
        assert!(word_boundary_contains("cargo test --lib", "cargo test"));
        assert!(!word_boundary_contains("cargo testing", "cargo test"));
    }

    #[test]
    fn action_class_table() {
        assert_eq!(
            ActionClass::of_tool("search_replace", None),
            ActionClass::Mutate
        );
        assert_eq!(ActionClass::of_tool("run_tests", None), ActionClass::Verify);
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("python -m pytest -q")),
            ActionClass::Verify
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("pwd && ls -la")),
            ActionClass::Neutral
        );
        assert_eq!(
            ActionClass::of_tool(BLACKBOARD_WRITE_TOOL_NAME, None),
            ActionClass::Session
        );
    }

    /// 0am 审查处置批（2026-10-03，用户裁决）：`cargo fmt --check` 命中且
    /// 变更性 `cargo fmt` 不命中（对齐 S2 设计原意）；追加 JS 生态 runner
    /// 词条命中。
    #[test]
    fn verify_lexicon_revision_fmt_check_and_ecosystem_runners() {
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("cargo fmt --check")),
            ActionClass::Verify
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("cargo fmt src/lib.rs")),
            ActionClass::Neutral
        );
        for cmd in [
            "npm run test",
            "cargo nextest run",
            "pnpm test",
            "yarn test",
            "bun test",
        ] {
            assert_eq!(
                ActionClass::of_tool("run_terminal_cmd", Some(cmd)),
                ActionClass::Verify,
                "{cmd}"
            );
        }
    }

    /// 0am 审查处置批：词边界把 `_` 视为词字符——`make_x`/`_pytest` 不再
    /// 误命中；正常词仍命中。
    #[test]
    fn word_boundary_treats_underscore_as_word_char() {
        assert!(!word_boundary_contains("make_x", "make"));
        assert!(!word_boundary_contains("_pytest plugins", "pytest"));
        assert!(word_boundary_contains("run pytest -q", "pytest"));
        assert!(
            ActionClass::of_tool("run_terminal_cmd", Some("make_x fast")) != ActionClass::Verify
        );
    }

    /// 0am 审查处置批：`.exe` 归一——`cargo.exe test`/`python.exe -m pytest`
    /// 不再漏命中。
    #[test]
    fn exe_normalization_hits_lexicon() {
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("cargo.exe test --lib")),
            ActionClass::Verify
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("python.exe -m pytest -q")),
            ActionClass::Verify
        );
    }

    /// 0am 守卫表（0am 审查处置批，2026-10-03）：grep/echo 等只读首词的
    /// 假验证失败不再判 Verify；复合命令守卫段剔除、非守卫段照常命中；
    /// 守卫词带路径 basename／大小写同样剔除。
    #[test]
    fn guard_table_excludes_non_action_heads() {
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("grep -rn pytest .")),
            ActionClass::Neutral
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("echo cargo test")),
            ActionClass::Neutral
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("cd x && python -m unittest")),
            ActionClass::Verify
        );
        assert_eq!(
            ActionClass::of_tool(
                "run_terminal_cmd",
                Some("cat Makefile | grep cargo || cargo test --lib")
            ),
            ActionClass::Verify
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("git status && ls -la")),
            ActionClass::Neutral
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("/usr/bin/grep -rn nextest .")),
            ActionClass::Neutral
        );
        assert_eq!(
            ActionClass::of_tool("run_terminal_cmd", Some("GREP -rn pytest .")),
            ActionClass::Neutral
        );
    }

    /// 0am 审查处置批：codex/opencode/hashline 适配器载体的结构化变更工具
    /// → Mutate（否则非 grok_build 载体 Prog 结构死窗）。
    #[test]
    fn adapter_structured_mutate_tools() {
        assert_eq!(
            ActionClass::of_tool("apply_patch", None),
            ActionClass::Mutate
        );
        assert_eq!(ActionClass::of_tool("write", None), ActionClass::Mutate);
        assert_eq!(ActionClass::of_tool("edit", None), ActionClass::Mutate);
    }

    /// 0am 审查处置批：`of_code` 六分组全覆盖钉（每类 ≥2 码；含
    /// `missing_test_runner`→Other）。WriteControl 不由 `of_code` 产出
    /// ——喂入点经写控块面 join（第六分组走分派）。
    #[test]
    fn deny_class_of_code_covers_all_groups() {
        for code in [
            "permission_deny",
            "permission_defer",
            "control_tool_lane_denied",
            "control_ticket_rejected:missing_target_argument",
        ] {
            assert_eq!(
                DenyClass::of_code(code),
                DenyClass::PermissionTicket,
                "{code}"
            );
        }
        for code in [
            "plan_round_tool_denied",
            "plan_write_disabled",
            "submit_lane_denied",
            "console_return_lane_denied",
            "order_slot_busy",
        ] {
            assert_eq!(DenyClass::of_code(code), DenyClass::PlanLane, "{code}");
        }
        for code in [
            "content_anchor_mismatch",
            "sealed_tool_denied",
            "retired_tool_denied",
            "round_inject_budget_exceeded",
            "web_fetch_candidate_cap_exceeded",
        ] {
            assert_eq!(DenyClass::of_code(code), DenyClass::GateGuard, "{code}");
        }
        for code in [
            "retrieval_not_enabled",
            "retrieval_role_shell_denied",
            "nested_subagent_dispatch_refused",
        ] {
            assert_eq!(
                DenyClass::of_code(code),
                DenyClass::RetrievalEnable,
                "{code}"
            );
        }
        for code in ["missing_test_runner", "some_unknown_future_code"] {
            assert_eq!(DenyClass::of_code(code), DenyClass::Other, "{code}");
        }
        let ev = ToolEvent {
            outcome: ToolOutcome::Error,
            wall_ms: None,
            policy_denied: false,
            routing: Some(StimulusRouting {
                write_control_block: true,
                ..StimulusRouting::new(ActionClass::Neutral)
            }),
        };
        assert_eq!(
            stimulus_targets(&ev).deny_class,
            Some(DenyClass::WriteControl)
        );
    }

    /// 0am 审查处置批（2026-10-03，用户裁决）：资源档位振荡钉——Normal→
    /// Soft→Normal→Soft 逐次跨档注入、同档不注入、未知档经 `from_wire`
    /// 得 `None`（喂入点不构造刺激＝不注入）。
    #[test]
    fn resource_tier_oscillation_injects_per_crossing() {
        assert_eq!(
            ResourceTier::from_wire("normal"),
            Some(ResourceTier::Normal)
        );
        assert_eq!(ResourceTier::from_wire("bogus-tier"), None);
        let mut engine = crate::lif::LifEngine::new();
        engine.enable_rli_shadow();
        engine.on_bus_event(
            10.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Normal),
        );
        assert_eq!(engine.infra().u(), 0.0, "run 首测＝基线不注入");
        engine.on_bus_event(
            11.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Soft),
        );
        let u1 = engine.infra().u();
        assert!(u1 > 0.0, "第一次跨档注入");
        engine.on_bus_event(
            12.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Normal),
        );
        let u2 = engine.infra().u();
        assert!(u2 > u1, "第二次跨档注入");
        engine.on_bus_event(
            13.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Soft),
        );
        let u3 = engine.infra().u();
        assert!(u3 > u2, "第三次跨档注入");
        engine.on_bus_event(
            14.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Soft),
        );
        assert_eq!(
            engine.infra().u().to_bits(),
            u3.to_bits(),
            "同档不注入（位级不变）"
        );
    }

    #[test]
    fn s2_dispatch_matrix() {
        // 验证通过（61s）：零注入＋Slow 豁免（legacy 会 prog+slow）。
        let ev = ToolEvent {
            outcome: ToolOutcome::Success,
            wall_ms: Some(61_000),
            policy_denied: false,
            routing: Some(StimulusRouting::new(ActionClass::Verify)),
        };
        let t = stimulus_targets(&ev);
        assert!(!t.prog_set && !t.verify && !t.err && t.slow.is_none());
        // 验证失败：verify，不串 err。
        let ev = ToolEvent {
            outcome: ToolOutcome::Other,
            wall_ms: Some(100),
            policy_denied: false,
            routing: Some(StimulusRouting {
                non_zero_exit: true,
                ..StimulusRouting::new(ActionClass::Verify)
            }),
        };
        let t = stimulus_targets(&ev);
        assert!(t.verify && !t.err);
        // H2 填平：非验证类非零退出入 Err（legacy 黑洞）。
        let ev = ToolEvent {
            outcome: ToolOutcome::Other,
            wall_ms: Some(10),
            policy_denied: false,
            routing: Some(StimulusRouting {
                non_zero_exit: true,
                ..StimulusRouting::new(ActionClass::Neutral)
            }),
        };
        let t = stimulus_targets(&ev);
        assert!(t.err && !t.prog_set);
        // 变更类成功：Prog set；变更类失败：Err。
        let t = stimulus_targets(&routed(ActionClass::Mutate));
        assert!(t.prog_set);
        // 写控块：deny 优先（WriteControl）。
        let ev = ToolEvent {
            outcome: ToolOutcome::Error,
            wall_ms: None,
            policy_denied: false,
            routing: Some(StimulusRouting {
                write_control_block: true,
                ..StimulusRouting::new(ActionClass::Neutral)
            }),
        };
        let t = stimulus_targets(&ev);
        assert!(t.deny && !t.err && t.deny_class == Some(DenyClass::WriteControl));
        // 结构化拒绝码（喂入点已解析）：deny＋标签维。
        let ev = ToolEvent {
            outcome: ToolOutcome::Other,
            wall_ms: None,
            policy_denied: false,
            routing: Some(StimulusRouting {
                deny_class: Some(DenyClass::PermissionTicket),
                ..StimulusRouting::new(ActionClass::Neutral)
            }),
        };
        let t = stimulus_targets(&ev);
        assert!(t.deny && t.deny_class == Some(DenyClass::PermissionTicket));
    }

    #[test]
    fn legacy_fallback_unrouted() {
        // 未路由事件保持旧四值语义（合成事件兼容面）。
        let ev = ToolEvent::success(Some(61_000));
        let t = stimulus_targets(&ev);
        assert!(t.prog_set && t.slow.is_some() && t.class.is_none());
        let ev = ToolEvent::error(None);
        assert!(stimulus_targets(&ev).err);
    }

    #[test]
    fn engine_bank_and_bus() {
        use crate::lif::{BusStimulus, LifEngine};
        let mut engine = LifEngine::new();
        engine.enable_rli_shadow();
        // 验证失败入 verify（1D＋影子），Slow 豁免（S2 v1.1 H4）。
        engine.on_tool_event(
            10.0,
            ToolEvent {
                outcome: ToolOutcome::Other,
                wall_ms: Some(61_000),
                policy_denied: false,
                routing: Some(StimulusRouting {
                    non_zero_exit: true,
                    ..StimulusRouting::new(ActionClass::Verify)
                }),
            },
        );
        assert!(engine.verify().u() > 0.0);
        assert_eq!(engine.slow().u(), 0.0);
        assert!(
            engine
                .rli_shadow()
                .unwrap()
                .channel(ChannelKind::Verify)
                .u()
                > 0.0
        );
        // 变更类成功入 prog；中性成功不入（Prog 收窄）。
        engine.on_tool_event(
            11.0,
            ToolEvent {
                outcome: ToolOutcome::Success,
                wall_ms: None,
                policy_denied: false,
                routing: Some(StimulusRouting::new(ActionClass::Mutate)),
            },
        );
        engine.on_tool_event(
            12.0,
            ToolEvent {
                outcome: ToolOutcome::Success,
                wall_ms: None,
                policy_denied: false,
                routing: Some(StimulusRouting::new(ActionClass::Neutral)),
            },
        );
        assert!(engine.prog().u() > 0.0);
        // 总线：压缩入 Ctx；snapshot 首测不注入、跨档注入（v1.1 过滤）。
        engine.on_bus_event(20.0, BusStimulus::ContextCompressed);
        engine.on_bus_event(
            21.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Watch),
        );
        assert_eq!(engine.infra().u(), 0.0);
        engine.on_bus_event(
            22.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Soft),
        );
        assert!(engine.infra().u() > 0.0);
        assert!(engine.ctx().u() > 0.0);
        assert!(engine.rli_shadow().unwrap().channel(ChannelKind::Infra).u() > 0.0);
    }

    #[test]
    fn engine_streak_cause_segment() {
        use crate::lif::{LifEngine, RLI_STREAK_K};
        let mut engine = LifEngine::new();
        engine.enable_rli_shadow();
        // k 个连续验证失败 → streak fire 带标签成因段（「源：验证失败×k」）。
        for i in 0..RLI_STREAK_K + 2 {
            engine.on_tool_event(
                10.0 + i as f64,
                ToolEvent {
                    outcome: ToolOutcome::Other,
                    wall_ms: None,
                    policy_denied: false,
                    routing: Some(StimulusRouting {
                        non_zero_exit: true,
                        ..StimulusRouting::new(ActionClass::Verify)
                    }),
                },
            );
        }
        let shadow = engine.rli_shadow().unwrap();
        let fired = shadow
            .notices()
            .iter()
            .any(|n| n.text.contains("源：验证失败×"));
        assert!(
            fired,
            "streak notice must carry the label cause: {:?}",
            shadow
                .notices()
                .iter()
                .map(|n| n.text.clone())
                .collect::<Vec<_>>()
        );
        for n in shadow.notices() {
            assert!(
                n.text.len() <= crate::lif::RLI_NOTICE_TEXT_BUDGET,
                "notice width budget: {}",
                n.text.len()
            );
        }
    }

    /// 0am 审查处置批（2026-10-03，用户裁决）：streak 观察域显式排除
    /// Ctx/Infra——总线注入抬高单事件水平后，连续动作样上旧「除 Prog 外
    /// 全部」口径会成单事件回声 fire；收窄后 ctx/infra u>0（数据面不变）
    /// 但无任何 fire；随后 k 个 Verify 失败仍 fire（既有钉保住）。
    #[test]
    fn engine_streak_skips_ctx_and_infra_but_keeps_verify() {
        use crate::lif::{BusStimulus, LifEngine, RLI_STREAK_K};
        let mut engine = LifEngine::new();
        engine.enable_rli_shadow();
        // Ctx 三连注入 + Infra 三次跨档（Watch＝基线、Soft/Hard/Normal 逐档
        // crossing）→ 两通道 u 高台。
        for i in 0..3u32 {
            engine.on_bus_event(10.0 + f64::from(i), BusStimulus::ContextCompressed);
        }
        engine.on_bus_event(
            13.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Watch),
        );
        engine.on_bus_event(
            14.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Soft),
        );
        engine.on_bus_event(
            15.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Hard),
        );
        engine.on_bus_event(
            16.0,
            BusStimulus::HostResourceSnapshotTier(ResourceTier::Normal),
        );
        assert!(engine.ctx().u() > 0.0);
        assert!(engine.infra().u() > 0.0);
        // 中性动作样 ×3（不注入任何通道＝纯采样点）：ctx/infra u 仍 ≥ θ，
        // 旧口径在此已产 streak fire；收窄后必须零提醒。
        for i in 0..3u32 {
            engine.on_tool_event(20.0 + f64::from(i), ToolEvent::other(None));
        }
        let shadow = engine.rli_shadow().unwrap();
        assert!(shadow.channel(ChannelKind::Ctx).u() > 0.0);
        assert!(shadow.channel(ChannelKind::Infra).u() > 0.0);
        let texts: Vec<String> = shadow.notices().iter().map(|n| n.text.clone()).collect();
        assert!(texts.is_empty(), "ctx/infra 不得产 streak fire: {texts:?}");
        // k 个 Verify 失败仍 fire（既有钉保住）。
        for i in 0..RLI_STREAK_K + 2 {
            engine.on_tool_event(
                30.0 + i as f64,
                ToolEvent {
                    outcome: ToolOutcome::Other,
                    wall_ms: None,
                    policy_denied: false,
                    routing: Some(StimulusRouting {
                        non_zero_exit: true,
                        ..StimulusRouting::new(ActionClass::Verify)
                    }),
                },
            );
        }
        let shadow = engine.rli_shadow().unwrap();
        assert!(
            shadow
                .notices()
                .iter()
                .any(|n| n.text.contains("源：验证失败×")),
            "verify streak must still fire: {:?}",
            shadow
                .notices()
                .iter()
                .map(|n| n.text.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn bus_stimulus_channels() {
        assert_eq!(
            BusStimulus::ContextCompressed.channel(),
            Some(ChannelKind::Ctx)
        );
        assert_eq!(
            BusStimulus::TransportRetry.channel(),
            Some(ChannelKind::Infra)
        );
    }
}
