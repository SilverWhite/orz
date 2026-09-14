//! 0ae D2 注意力阶梯（2026-09-15，[`CONTEXT_SOFT_GATE_MODEL_PARTICIPATED_
//! COMPRESSION_DESIGN`] §5，用户已定框架；1M 窗口口径）。
//!
//! 阶梯（全部 env 可配、K = ×1024 token；0 = 显式禁用该级）：
//! `128K` 打断式提醒（独立注入块、注意力管理开始）→ `160K` 提醒式
//! （随下一合法边界附注）→ `300K/500K/600K/700K` 软提醒（固化进度
//! 问询，语气递进）→ `800K` 截断式硬提醒（最后通牒）→ `920K` 打断
//! 全部动作进入模型实施的压缩（D3；窗口语义在 `checkpoint`/`agent_loop`）。
//!
//! 形态定义（设计 §5）：打断式/截断式 = 独立注入块；提醒式/软提醒 =
//! 单行附注（每级恰好一次，成本 ~30 token 量级）。每阈值只触发一次；
//! 920K 压缩完成后全阶梯重新武装（从 128K 起）。长视图质量读数：用户
//! 已裁定接受（orz 为通用框架，不做逐模型窗口适配；env 可配即天然
//! 多模型可移植）——V4 塌陷带（384K–512K）对 V4.1 适用性 = 已知未知项。

use std::collections::HashSet;

/// 打断式提醒阈值（K token；0 = 禁用）。
pub const ENV_INTERRUPT_K: &str = "ORZ_LADDER_INTERRUPT_K";
/// 提醒式阈值（K token；0 = 禁用）。
pub const ENV_REMIND_K: &str = "ORZ_LADDER_REMIND_K";
/// 软提醒阈值表（逗号分隔 K 值；`0` 段被剔除）。
pub const ENV_SOFT_K: &str = "ORZ_LADDER_SOFT_K";
/// 截断式硬提醒阈值（K token；0 = 禁用）。
pub const ENV_HARD_K: &str = "ORZ_LADDER_HARD_K";
/// 模型参与压缩触发阈值（K token；0 = 禁用 D3 窗口）。
pub const ENV_COMPRESS_K: &str = "ORZ_LADDER_COMPRESS_K";

pub const DEFAULT_INTERRUPT_K: u64 = 128;
pub const DEFAULT_REMIND_K: u64 = 160;
pub const DEFAULT_SOFT_K: &[u64] = &[300, 500, 600, 700];
pub const DEFAULT_HARD_K: u64 = 800;
pub const DEFAULT_COMPRESS_K: u64 = 920;

/// D3：模型实施压缩的有限轮数（DP-7：≤3 轮；超轮未完成 ⇒ 机械无差别
/// 折叠兜底，`model_participated` 如实落账）。
pub const COMPRESSION_WINDOW_ROUNDS: u32 = 3;

fn env_k(get: &impl Fn(&str) -> Option<String>, key: &str, default: u64) -> u64 {
    get(key)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

/// 阶梯阈值集（env 可配的纯数据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderThresholds {
    pub interrupt_k: u64,
    pub remind_k: u64,
    pub soft_ks: Vec<u64>,
    pub hard_k: u64,
    pub compress_k: u64,
}

impl Default for LadderThresholds {
    fn default() -> Self {
        Self {
            interrupt_k: DEFAULT_INTERRUPT_K,
            remind_k: DEFAULT_REMIND_K,
            soft_ks: DEFAULT_SOFT_K.to_vec(),
            hard_k: DEFAULT_HARD_K,
            compress_k: DEFAULT_COMPRESS_K,
        }
    }
}

impl LadderThresholds {
    /// env 口径（测试不经进程 env——`from_env_with` 缝隙）。
    pub fn from_env() -> Self {
        Self::from_env_with(&|key| std::env::var(key).ok())
    }

    pub fn from_env_with(get: &impl Fn(&str) -> Option<String>) -> Self {
        let soft_ks = match get(ENV_SOFT_K) {
            Some(raw) => raw
                .split(',')
                .filter_map(|v| v.trim().parse::<u64>().ok())
                .filter(|k| *k > 0)
                .collect(),
            None => DEFAULT_SOFT_K.to_vec(),
        };
        Self {
            interrupt_k: env_k(get, ENV_INTERRUPT_K, DEFAULT_INTERRUPT_K),
            remind_k: env_k(get, ENV_REMIND_K, DEFAULT_REMIND_K),
            soft_ks,
            hard_k: env_k(get, ENV_HARD_K, DEFAULT_HARD_K),
            compress_k: env_k(get, ENV_COMPRESS_K, DEFAULT_COMPRESS_K),
        }
    }

    fn k(tokens: u64) -> u64 {
        tokens.saturating_mul(1024)
    }

    pub fn interrupt_tokens(&self) -> u64 {
        Self::k(self.interrupt_k)
    }

    pub fn remind_tokens(&self) -> u64 {
        Self::k(self.remind_k)
    }

    pub fn soft_tokens(&self, k: u64) -> u64 {
        Self::k(k)
    }

    pub fn hard_tokens(&self) -> u64 {
        Self::k(self.hard_k)
    }

    pub fn compress_tokens(&self) -> u64 {
        Self::k(self.compress_k)
    }
}

/// 阶梯级别（去重键 = 级别 + 阈值 K 值，env 改配后互不遮蔽）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LadderLevel {
    Interrupt,
    Remind,
    Soft(u64),
    Hard,
}

impl LadderLevel {
    /// MechanicalAuditUpdate `kind=attention_ladder` 的机器读数位。
    pub fn as_str(&self) -> String {
        match self {
            LadderLevel::Interrupt => "interrupt".to_string(),
            LadderLevel::Remind => "remind".to_string(),
            LadderLevel::Soft(k) => format!("soft_{k}"),
            LadderLevel::Hard => "hard".to_string(),
        }
    }
}

/// 一次阶梯触发（级别 + 阈值 K 值 + 注入文案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderFire {
    pub level: LadderLevel,
    pub threshold_k: u64,
    pub block: String,
}

/// 阶梯状态（per-run 本地；920K 压缩完成后 [`rearm`] 全阶梯重新武装）。
#[derive(Debug, Clone, Default)]
pub struct AttentionLadder {
    fired: HashSet<LadderLevel>,
    compress_fired: bool,
    thresholds: LadderThresholds,
}

impl AttentionLadder {
    pub fn from_env() -> Self {
        Self::with_thresholds(LadderThresholds::from_env())
    }

    pub fn with_thresholds(thresholds: LadderThresholds) -> Self {
        Self {
            fired: HashSet::new(),
            compress_fired: false,
            thresholds,
        }
    }

    /// 当前测量 token 下新越线的提醒级（每级恰好一次；重复查询零返回）。
    pub fn due(&mut self, measured_tokens: u64) -> Vec<LadderFire> {
        let t = self.thresholds.clone();
        let mut out = Vec::new();
        let mut check = |k: u64, level: LadderLevel, block: String| {
            if k > 0
                && measured_tokens >= LadderThresholds::k(k)
                && self.fired.insert(level.clone())
            {
                out.push(LadderFire {
                    level,
                    threshold_k: k,
                    block,
                });
            }
        };
        check(
            t.interrupt_k,
            LadderLevel::Interrupt,
            interrupt_block(t.interrupt_k),
        );
        check(t.remind_k, LadderLevel::Remind, remind_line(t.remind_k));
        let soft_ks = t.soft_ks.clone();
        for k in soft_ks {
            check(k, LadderLevel::Soft(k), soft_line(k));
        }
        check(t.hard_k, LadderLevel::Hard, hard_block(t.hard_k));
        out
    }

    /// D3：920K 是否应当开窗（只开一次；重开需 [`rearm`]）。
    pub fn compress_due(&mut self, measured_tokens: u64) -> bool {
        let k = self.thresholds.compress_k;
        if k == 0 || self.compress_fired {
            return false;
        }
        if measured_tokens >= LadderThresholds::k(k) {
            self.compress_fired = true;
            return true;
        }
        false
    }

    /// 920K 压缩完成后全阶梯重新武装（设计 §5：从 128K 起）。
    pub fn rearm(&mut self) {
        self.fired.clear();
        self.compress_fired = false;
    }
}

// ── 文案（中性事实 + 明确后果；固化入口指黑板写入面）────────────────────

fn interrupt_block(k: u64) -> String {
    format!(
        "[注意力 {k}K] 上下文已达 {k}K token，注意力管理开始：请把接线结论、\
         关键读数与不可再生的中间结果固化到黑板（blackboard_write section=plan|notes，\
         水位见 blackboard_read 响应头）。黑板不受上下文折叠影响。\
         后续阶梯：300K/500K/600K/700K 软提醒 → 800K 最后通牒 → 920K 打断全部动作进入压缩。"
    )
}

fn remind_line(k: u64) -> String {
    format!("[注意力 {k}K] 提醒：尚未固化的关键内容可经 blackboard_write 写入黑板 plan/notes。")
}

fn soft_line(k: u64) -> String {
    format!(
        "[注意力 {k}K] 固化进度确认：若关键接线结论/读数尚未写入黑板，此刻是低成本窗口\
         （blackboard_write section=plan|notes）。"
    )
}

fn hard_block(k: u64) -> String {
    format!(
        "[注意力 {k}K] 最后通牒：920K 将打断全部动作进入压缩。未固化内容届时丢失，\
         唯一保留面 = 黑板（blackboard_write section=plan|notes）+ 保留尾。请立即固化。"
    )
}

/// D3：压缩窗口任务块（打断全部动作后的无工具轮注入）——固化必要
/// 内容 + 标注可弃范围，明确提示可用黑板；机械层按窗口结果执行折叠。
pub fn compression_window_block(k: u64) -> String {
    format!(
        "[注意力 {k}K · 模型参与压缩] 已打断全部动作。请在有限轮内完成两件事：
         1. 把继续工作必需的内容固化到黑板（blackboard_write section=plan|notes；         黑板不受折叠影响）；
         2. 在回复中标注可弃范围（哪些轮次/读数/尝试可以丢弃）。
         窗口结束后机械层执行折叠：标注区按标注折叠，未标注区按保留尾策略；         未完成时机械无差别折叠照旧执行并如实落账（model_participated: false）。"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_threshold_fires_exactly_once_in_order() {
        let mut ladder = AttentionLadder::with_thresholds(LadderThresholds::default());
        assert!(ladder.due(100 * 1024).is_empty(), "100K 无触发");
        let fires = ladder.due(130 * 1024);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].level, LadderLevel::Interrupt);
        assert!(fires[0].block.contains("128K"));
        assert!(ladder.due(130 * 1024).is_empty(), "同位重查不重复");
        let fires = ladder.due(170 * 1024);
        assert_eq!(fires.len(), 1);
        assert_eq!(fires[0].level, LadderLevel::Remind);
        let fires = ladder.due(650 * 1024);
        let levels: Vec<LadderLevel> = fires.iter().map(|f| f.level.clone()).collect();
        assert_eq!(
            levels,
            vec![
                LadderLevel::Soft(300),
                LadderLevel::Soft(500),
                LadderLevel::Soft(600)
            ]
        );
        let fires = ladder.due(990 * 1024);
        assert_eq!(fires.len(), 2, "700 软 + 800 硬");
        assert!(ladder.compress_due(990 * 1024));
        assert!(!ladder.compress_due(990 * 1024), "compress 只开窗一次");
    }

    #[test]
    fn rearm_rearms_the_whole_ladder() {
        let mut ladder = AttentionLadder::with_thresholds(LadderThresholds::default());
        assert!(!ladder.due(990 * 1024).is_empty());
        assert!(ladder.compress_due(990 * 1024));
        ladder.rearm();
        assert_eq!(ladder.due(990 * 1024).len(), 7, "全部提醒级重新武装");
        assert!(ladder.compress_due(990 * 1024), "920K 同样重新武装");
    }

    #[test]
    fn zero_disables_a_level_and_env_overrides() {
        let t = LadderThresholds::from_env_with(&|key| match key {
            ENV_INTERRUPT_K => Some("0".to_string()),
            ENV_REMIND_K => Some("64".to_string()),
            ENV_SOFT_K => Some("200, 0 ,300".to_string()),
            _ => None,
        });
        assert_eq!(t.interrupt_k, 0, "0 = 显式禁用");
        assert_eq!(t.remind_k, 64);
        assert_eq!(t.soft_ks, vec![200, 300], "0 值段被剔除、空段容错");
        assert_eq!(t.hard_k, 800, "非法/缺省回默认值");
        assert_eq!(t.compress_k, 920);
        let mut ladder = AttentionLadder::with_thresholds(t);
        let fires = ladder.due(400 * 1024);
        assert!(!fires.iter().any(|f| f.level == LadderLevel::Interrupt));
        assert!(fires.iter().any(|f| f.level == LadderLevel::Soft(200)));
        assert!(fires.iter().any(|f| f.level == LadderLevel::Soft(300)));
    }

    #[test]
    fn block_texts_carry_the_threshold_and_blackboard_hint() {
        let mut ladder = AttentionLadder::with_thresholds(LadderThresholds::default());
        let fires = ladder.due(130 * 1024);
        assert!(fires[0].block.contains("blackboard_write"));
        let fires = ladder.due(810 * 1024);
        let hard = fires.iter().find(|f| f.level == LadderLevel::Hard).unwrap();
        assert!(hard.block.contains("920K"));
    }
}
