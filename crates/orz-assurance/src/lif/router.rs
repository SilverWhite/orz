//! 0am P8（2026-10-03，S2 刺激面路由表实现批）：刺激路由器——动作类标签器、
//! 拒绝类分组与 [`stimulus_targets`] 纯分派。设计权威＝
//! `docs/LIF_RLI_STIMULUS_ROUTING_TABLE_S2_DESIGN_2026-10-03.md`（v1.1；
//! S3 重放 J1–J5 已收，175 批）。原则边界：只定义**通道看见什么**
//! （P1–P5/P7/P8 标签面）；**零触发条件**（P9——阈值/k/域机/提醒沿全部在
//! 既有动力学内，本模块不携带任何「当 X 即 Y」规则）。
//!
//! 同源复用（S2 §5）：生产喂入点（orz-loop）与本crate重放件
//! （`examples/rli_shadow_replay`）共用本模块的标签器与分派——对拍一致性
//! 由构造保证。工具名字面单一源（0ao 扫描钉）：在册常量经
//! `crate::tool_names` 引用。

use crate::lif::channels::{ChannelKind, SLOW_WALL_MS_THRESHOLD, ToolEvent, ToolOutcome};
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
    /// 提交类（submit）。
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
                Some(cmd)
                    if VERIFY_LEXICON
                        .iter()
                        .any(|tok| word_boundary_contains(cmd, tok)) =>
                {
                    ActionClass::Verify
                }
                _ => ActionClass::Neutral,
            },
            _ => ActionClass::Neutral,
        }
    }
}

/// S2 §4.1 终端验证词表（闭集初版）。匹配口径（v1.1）：大小写不敏感子串＋
/// 两侧字母数字词边界（复合命令的子命令可命中；`Makefile` 不误命中 `make`）。
/// 词表扩展只随批次修订（闭集纪律，禁实现层临时加词）。
pub const VERIFY_LEXICON: [&str; 26] = [
    "cargo test",
    "cargo build",
    "cargo check",
    "cargo clippy",
    "cargo fmt",
    "pytest",
    "python -m pytest",
    "python -m unittest",
    "unittest",
    "make",
    "npm test",
    "npm run build",
    "npx tsc",
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

/// 词边界包含（大小写不敏感）：命中处前一字节与尾后一字节均非 ASCII 字母数字。
pub fn word_boundary_contains(hay: &str, needle: &str) -> bool {
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return false;
    }
    let eq = |a: u8, b: u8| a.eq_ignore_ascii_case(&b);
    for i in 0..=(h.len() - n.len()) {
        if !h[i..i + n.len()].iter().zip(n).all(|(&a, &b)| eq(a, b)) {
            continue;
        }
        let before_ok = i == 0 || !h[i - 1].is_ascii_alphanumeric();
        let after = i + n.len();
        let after_ok = after >= h.len() || !h[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
    }
    false
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
    /// 动作类标签（标签成因段的「动作类计数」来源）。
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

/// Slow 通道时长权值（生产同式：分钟数 clamp 到 [1, 5]；SLOW_W_MAX）。
fn slow_weight(wall_ms: u64) -> f64 {
    (wall_ms as f64 / 60_000.0).clamp(1.0, 5.0)
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
    /// 认领通道（None＝本事件不注入——如 snapshot 非跨档）。
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
