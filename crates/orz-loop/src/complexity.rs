//! 会话繁杂度（0be 四项④，2026-09-21 用户二次裁定「机制照抄疲劳度、只需
//! 一个指标」；观察项 `OBS-RLI-SESSION-FATIGUE`）。
//!
//! 与 [`crate::fatigue`]（水位疲劳）**同形不同源**：水位＝黑板 live 字节
//! （工作总量代理）；繁杂度＝RLI 影子四条复极点通道的预测误差比放大 c
//! （会话内预测准确性相对**本会话前段基线**的退化——作用域＝本会话，
//! 不设跨会话桥接；跨会话当新任务重开）。阈值不复用绝对水位，而由 RLI 侧
//! **分位数自校准**（θ85/θ95/θ99，θ 同法；详见
//! `orz_assurance::lif::RliComplexity`）：越线（且高于基线，c > 1）即锁存，
//! 档键 `q85`/`q95`/`q99`——与水位档 `50`/`70`/`90` **分开命名**、分开
//! 簿记（宿主会话侧车两套字段）。
//!
//! 形态（照抄 E9）：仅用户面机械提醒——一次运行最多一条（取最高未投递档，
//! 并把本次全部未投递越线档一并落档，巨幅跳跃不刷屏）；不注入模型上下文、
//! 不锁工具面；用户忽略不重复；失败 run 不更新档位（SUCCESS-ONLY，宿主面）。
//!
//! 输入＝[`RliComplexityReading`]（RLI 侧读数：就绪／样本／c／基线／θ／
//! 锁存）；影子未启用或样本不足（未就绪）＝不投递（机械如实——退化路径）。

use orz_assurance::lif::RliComplexityReading;

/// 档位键（与水位的 `50`/`70`/`90` 分开命名）。顺序 = 递增。
pub const COMPLEXITY_TIER_Q85: &str = "q85";
pub const COMPLEXITY_TIER_Q95: &str = "q95";
pub const COMPLEXITY_TIER_Q99: &str = "q99";
pub const COMPLEXITY_TIER_ORDER: [&str; 3] = [
    COMPLEXITY_TIER_Q85,
    COMPLEXITY_TIER_Q95,
    COMPLEXITY_TIER_Q99,
];

/// 一次待投递的繁杂度提醒。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexityNotice {
    /// 档位键（`q85`/`q95`/`q99`）——宿主持久化到会话侧车，避免重复提醒。
    pub tier: &'static str,
    /// 用户侧文本（机械、事实、软措辞；含 c 与自校准阈值数字）。
    pub text: String,
}

/// 一次繁杂度判定结果：提醒 + 本次应落档的档位键集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexityDecision {
    /// 待投递提醒（已锁存且未投递的最高档）。
    pub notice: ComplexityNotice,
    /// 本次应落档的全部档位键（升序，含 `notice.tier`）——同时锁存多档时
    /// 低档一并落档，避免后续 run 补发已跳过的提示。
    pub tiers_to_mark: Vec<&'static str>,
}

/// 计算应当提醒的档位（已锁存且未投递）。返回 `None` = 无新档（未就绪／
/// 无锁存／都已投递）。一次运行最多给一条：取最高未投递档，并把本次全部
/// 未投递档一并放进 `tiers_to_mark`（跳过即视为已投递，低档不滞留补发）。
pub fn pending_complexity_notice(
    reading: &RliComplexityReading,
    notified_tiers: &[String],
) -> Option<ComplexityDecision> {
    if !reading.ready {
        return None;
    }
    let c = reading.c?;
    let candidates: Vec<&'static str> = COMPLEXITY_TIER_ORDER
        .iter()
        .enumerate()
        .filter(|(idx, _)| reading.latched[*idx])
        .map(|(_, key)| *key)
        .filter(|key| !notified_tiers.iter().any(|t| t == key))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let tier = candidates.last().copied().expect("candidates non-empty");
    let idx = COMPLEXITY_TIER_ORDER
        .iter()
        .position(|key| *key == tier)
        .expect("tier comes from COMPLEXITY_TIER_ORDER");
    let theta = reading.theta[idx];
    let text = match tier {
        COMPLEXITY_TIER_Q85 => format!(
            "（会话提示：本会话的运行模式较前段更难预测（预测误差比放大 c={c:.2}，\
             越过本会话自校准 q85={theta:.2} 线）。信息提醒——若线索开始发散，\
             可考虑收束当前线索。）"
        ),
        COMPLEXITY_TIER_Q95 => format!(
            "（会话提示：本会话的运行模式较前段明显更难预测（c={c:.2}，越过 \
             q95={theta:.2} 线）——建议适时换新对话：新对话＝新会话，繁杂度与\
             前段基线重新累积。）"
        ),
        COMPLEXITY_TIER_Q99 => format!(
            "（会话提示：本会话的预测难度到达本会话内最高水位（c={c:.2}，越过 \
             q99={theta:.2} 线）——继续同一会话的复杂度已显著高于前段；是否换\
             对话由你决定。）"
        ),
        _ => unreachable!("complexity tier order covers q85/q95/q99"),
    };
    Some(ComplexityDecision {
        notice: ComplexityNotice { tier, text },
        tiers_to_mark: candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notified(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|s| s.to_string()).collect()
    }

    fn reading(ready: bool, c: Option<f64>, theta: [f64; 3], latched: [bool; 3]) -> RliComplexityReading {
        RliComplexityReading {
            ready,
            samples: 40,
            c,
            rho_base: Some(0.4),
            theta,
            latched,
        }
    }

    #[test]
    fn not_ready_or_missing_c_yields_none() {
        // 未就绪（基线未冻结）= 不投递（退化路径，机械如实）。
        assert_eq!(
            pending_complexity_notice(
                &reading(false, None, [1.0; 3], [true, true, true]),
                &[]
            ),
            None
        );
        // 就绪但读数缺失 = 同样不投递。
        assert_eq!(
            pending_complexity_notice(&reading(true, None, [1.0; 3], [true, false, false]), &[]),
            None
        );
    }

    #[test]
    fn tier_q85_fires_once_and_text_carries_numbers() {
        let r = reading(true, Some(1.35), [1.20, 1.60, 2.40], [true, false, false]);
        let d = pending_complexity_notice(&r, &[]).expect("q85 latched");
        assert_eq!(d.notice.tier, COMPLEXITY_TIER_Q85);
        assert_eq!(d.tiers_to_mark, vec![COMPLEXITY_TIER_Q85]);
        assert!(d.notice.text.contains("c=1.35"), "c in text: {}", d.notice.text);
        assert!(
            d.notice.text.contains("q85=1.20"),
            "theta in text: {}",
            d.notice.text
        );
        // 已投递 → 不重复。
        assert_eq!(
            pending_complexity_notice(&r, &notified(&[COMPLEXITY_TIER_Q85])),
            None
        );
    }

    #[test]
    fn jump_marks_all_candidates_and_reports_highest() {
        let r = reading(true, Some(1.8), [1.2, 1.5, 2.6], [true, true, false]);
        let d = pending_complexity_notice(&r, &[]).expect("two tiers latched");
        assert_eq!(d.notice.tier, COMPLEXITY_TIER_Q95);
        assert_eq!(
            d.tiers_to_mark,
            vec![COMPLEXITY_TIER_Q85, COMPLEXITY_TIER_Q95]
        );
        assert!(d.notice.text.contains("c=1.80"));
    }

    #[test]
    fn later_latch_fires_after_earlier_delivery() {
        let r = reading(true, Some(2.9), [1.2, 1.5, 2.2], [true, true, true]);
        let d = pending_complexity_notice(
            &r,
            &notified(&[COMPLEXITY_TIER_Q85, COMPLEXITY_TIER_Q95]),
        )
        .expect("q99 latched");
        assert_eq!(d.notice.tier, COMPLEXITY_TIER_Q99);
        assert_eq!(d.tiers_to_mark, vec![COMPLEXITY_TIER_Q99]);
        assert!(d.notice.text.contains("q99=2.20"));
    }

    #[test]
    fn all_delivered_yields_none() {
        let r = reading(true, Some(3.0), [1.2, 1.5, 2.2], [true, true, true]);
        assert_eq!(
            pending_complexity_notice(
                &r,
                &notified(&[
                    COMPLEXITY_TIER_Q85,
                    COMPLEXITY_TIER_Q95,
                    COMPLEXITY_TIER_Q99
                ])
            ),
            None
        );
    }
}
