//! 会话繁杂度 → 0bf ③（2026-09-22）**复合总值口径**（用户令：「繁杂度这么
//! 容易触发的话，和水路疲劳度绑在一起做加权，一起算一个总值」——不加会话
//! 长度门；仅用户面、不注入模型；长任务语义由水位疲劳天然承担）。
//!
//! 与 [`crate::fatigue`]（水位疲劳）**同形不同源**：水位＝黑板 live 字节
//! （工作总量代理）；繁杂度＝RLI 影子四条复极点通道的预测误差比放大 c
//! （会话内预测准确性相对**本会话前段基线**的退化——作用域＝本会话，
//! 不设跨会话桥接；跨会话当新任务重开）。阈值不复用绝对水位，而由 RLI
//! 侧**分位数自校准**（θ85/θ95/θ99；详见 `orz_assurance::lif::RliComplexity`）。
//!
//! **0bf 起**（本文件主职责）：
//! - `cplx_pct`：把 c 映射到 0–100 的**分位刻度**——c ≤ 1 = 0（未高于基线；
//!   缺读数/未就绪 = 0，FR-7 不落假值）；(1, θ85] → (0, 60]；(θ85, θ95] →
//!   (60, 80]；(θ95, θ99] → (80, 100]；c > θ99 = 100。锚点＝会话自校准，
//!   **线性分段、零拟合**；θ 非单调时逐段宽度兜底（见 [`cplx_percent`]）。
//! - `burden`：**总值 = min(100, fat_pct + 0.3 × cplx_pct)**——水位为工作
//!   总量基线（原语义不变），繁杂度作为**至多 +30 的增益**（单靠繁杂度
//!   不足以触档：用户令「繁杂度容易触发」的校准面）。权重 `0.3` 为初值，
//!   读数后可调（S3/S4 标定项）。
//! - 档位＝`50`/`70`/`90`（沿用疲劳档梯，取最高未投递档；跳跃档一并联档
//!   不补发）；宿主持久化键＝`burden_tiers_notified`（替代 0be 的
//!   `fatigue_tiers_notified`/`complexity_tiers_notified` 两键）。
//!
//! 形态（照抄 E9）：仅用户面机械提醒——一次运行最多一条；不注入模型上下文、
//! 不锁工具面；用户忽略不重复；失败 run 不更新档位（SUCCESS-ONLY，宿主面）。

use orz_assurance::lif::RliComplexityReading;

/// 繁杂度增益权重（burden 总值中 `cplx_pct` 的系数）。初值 0.3（S1 预注册；
/// 读数后标定）。语义：繁杂度至多把总值抬 30 点——单靠繁杂度不触 50 档。
pub const BURDEN_CPLX_WEIGHT: f64 = 0.3;

/// `cplx_pct` 分段锚点（c 处于相应分位线时对应的百分位刻度）。
const CPLX_PCT_Q85: f64 = 60.0;
const CPLX_PCT_Q95: f64 = 80.0;
const CPLX_PCT_Q99: f64 = 100.0;

/// 总值档位（与疲劳 50/70/90 同梯；顺序 = 递增）。
pub const BURDEN_TIER_ORDER: [&str; 3] = ["50", "70", "90"];

/// 一次待投递的会话负担提醒。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BurdenNotice {
    /// 档位键（`50`/`70`/`90`）——宿主持久化到会话侧车，避免重复提醒。
    pub tier: &'static str,
    /// 用户侧文本（机械、事实、软措辞；含总值与两路分量的数字）。
    pub text: String,
}

/// 一次负担判定结果：提醒 + 本次应落档的档位键集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BurdenDecision {
    /// 待投递提醒（已达且未投递的最高档）。
    pub notice: BurdenNotice,
    /// 本次应落档的全部档位键（升序，含 `notice.tier`）——跳跃时低档一并
    /// 落档，避免后续 run 补发已跳过的提示。
    pub tiers_to_mark: Vec<&'static str>,
}

/// 繁杂度百分位刻度（0–100；见模块注释的分段定义）。
///
/// 未就绪 / 无 c / c ≤ 1（未高于基线）= 0（读数面「不可得」由调用方以
/// `ready` 区分——本函数只做分位映射）。
pub fn cplx_percent(reading: &RliComplexityReading) -> f64 {
    if !reading.ready {
        return 0.0;
    }
    let Some(c) = reading.c else {
        return 0.0;
    };
    if !c.is_finite() || c <= 1.0 {
        return 0.0;
    }
    let [q85, q95, q99] = reading.theta;
    // θ 由分位数自校准给出，小样本下可能非单调——逐段宽度兜底（段宽 ≥
    // 1e-6），保证映射有限且单调不减。
    let seg = |lo: f64, hi: f64, base: f64, span: f64| -> f64 {
        let width = (hi - lo).max(1e-6);
        base + span * ((c - lo) / width).clamp(0.0, 1.0)
    };
    if q85 <= 1.0 || c <= q85 {
        // c ∈ (1, θ85]：锚 (1, 0) → (θ85, 60)。
        return seg(1.0, q85.max(1.0 + 1e-6), 0.0, CPLX_PCT_Q85);
    }
    let q95 = q95.max(q85);
    if c <= q95 {
        return seg(q85, q95, CPLX_PCT_Q85, CPLX_PCT_Q95 - CPLX_PCT_Q85);
    }
    let q99 = q99.max(q95);
    if c <= q99 {
        return seg(q95, q99, CPLX_PCT_Q95, CPLX_PCT_Q99 - CPLX_PCT_Q95);
    }
    CPLX_PCT_Q99
}

/// 会话负担总值（`min(100, fat_pct + w × cplx_pct)`，四舍五入取整）。
/// 水位分量沿用 `orz_loop::fatigue::fatigue_percent` 的原量纲与语义。
pub fn burden_total(fatigue_percent: u64, reading: Option<&RliComplexityReading>) -> u64 {
    let cplx = reading.map_or(0.0, cplx_percent);
    let total = fatigue_percent as f64 + BURDEN_CPLX_WEIGHT * cplx;
    total.round().clamp(0.0, 100.0) as u64
}

/// 计算应当提醒的档位（已达且未投递）。返回 `None` = 无新档。
/// 一次运行最多给一条：取最高未投递档，并把本次全部未投递档一并放进
/// `tiers_to_mark`（跳过即视为已投递，低档不滞留补发）。
pub fn pending_burden_notice(
    fatigue_percent: u64,
    reading: Option<&RliComplexityReading>,
    notified_tiers: &[String],
) -> Option<BurdenDecision> {
    let total = burden_total(fatigue_percent, reading);
    let cplx = reading.map_or(0.0, cplx_percent);
    let gain = BURDEN_CPLX_WEIGHT * cplx;
    let candidates: Vec<&'static str> = BURDEN_TIER_ORDER
        .iter()
        .copied()
        .filter(|key| {
            let threshold: u64 = key.parse().expect("tier keys are numeric");
            total >= threshold && !notified_tiers.iter().any(|t| t.as_str() == *key)
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let tier = candidates.last().copied().expect("candidates non-empty");
    let text = match tier {
        "50" => format!(
            "（会话提示：本会话负担总值 {total}%＝水位 {fatigue_percent}% ＋ 繁杂度 \
             增益 {gain:.0}%。信息提醒——若线索开始发散，可考虑收束当前线索。）"
        ),
        "70" => format!(
            "（会话提示：本会话负担总值 {total}%＝水位 {fatigue_percent}% ＋ 繁杂度 \
             增益 {gain:.0}%——建议适时换新对话：新对话＝新会话，水位与繁杂度\
             基线重新累积。）"
        ),
        "90" => format!(
            "（会话提示：本会话负担总值 {total}%＝水位 {fatigue_percent}% ＋ 繁杂度 \
             增益 {gain:.0}%——接近本会话寿命区间；继续同一会话的积累成本已显著\
             高于前段，是否换对话由你决定。）"
        ),
        _ => unreachable!("burden tier order covers 50/70/90"),
    };
    Some(BurdenDecision {
        notice: BurdenNotice { tier, text },
        tiers_to_mark: candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notified(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|s| s.to_string()).collect()
    }

    fn reading(ready: bool, c: Option<f64>, theta: [f64; 3]) -> RliComplexityReading {
        RliComplexityReading {
            ready,
            samples: 40,
            c,
            rho_base: Some(0.4),
            theta,
            latched: [false; 3],
        }
    }

    #[test]
    fn not_ready_or_missing_c_yields_water_only() {
        // 未就绪（基线未冻结）/读数缺失 = 繁杂度分量 0（FR-7：不落假值），
        // 总值退回水位原值。
        let r = reading(false, None, [1.0; 3]);
        assert_eq!(cplx_percent(&r), 0.0);
        assert_eq!(burden_total(42, Some(&r)), 42);
        let r = reading(true, None, [1.0; 3]);
        assert_eq!(cplx_percent(&r), 0.0);
        assert_eq!(burden_total(42, Some(&r)), 42);
        // 无读数（影子未启用）= 0。
        assert_eq!(burden_total(42, None), 42);
        assert_eq!(pending_burden_notice(40, Some(&r), &[]), None);
    }

    #[test]
    fn cplx_percent_piecewise_anchors() {
        let theta = [1.20, 1.60, 2.40];
        // c ≤ 1 = 0（未高于基线）。
        assert_eq!(cplx_percent(&reading(true, Some(0.8), theta)), 0.0);
        assert_eq!(cplx_percent(&reading(true, Some(1.0), theta)), 0.0);
        // 锚点：θ85 → 60；θ95 → 80；θ99 → 100。
        assert!((cplx_percent(&reading(true, Some(1.20), theta)) - 60.0).abs() < 1e-9);
        assert!((cplx_percent(&reading(true, Some(1.60), theta)) - 80.0).abs() < 1e-9);
        assert!((cplx_percent(&reading(true, Some(2.40), theta)) - 100.0).abs() < 1e-9);
        // c > θ99 = 100（封顶）。
        assert_eq!(cplx_percent(&reading(true, Some(9.0), theta)), 100.0);
        // 段中线性：c=1.10 ∈ (1, 1.20] → 60×(0.10/0.20) = 30。
        assert!((cplx_percent(&reading(true, Some(1.10), theta)) - 30.0).abs() < 1e-9);
        // 段中线性：c=1.40 ∈ (1.20, 1.60] → 60 + 20×(0.20/0.40) = 70。
        assert!((cplx_percent(&reading(true, Some(1.40), theta)) - 70.0).abs() < 1e-9);
    }

    #[test]
    fn burden_total_binds_fatigue_and_complexity() {
        let theta = [1.20, 1.60, 2.40];
        let max_c = reading(true, Some(9.0), theta);
        // 水位单独：原语义保留（长任务语义由水位承担）。
        assert_eq!(burden_total(50, None), 50);
        assert_eq!(burden_total(90, None), 90);
        // 繁杂度单独：c > θ99（cplx_pct = 100）→ 至多 +30。
        assert_eq!(burden_total(20, Some(&max_c)), 50);
        // 单靠繁杂度不越 70 档（39 + 30 = 69）。
        assert_eq!(burden_total(39, Some(&max_c)), 69);
        // 组合：40 + 0.3×100 = 70。
        assert_eq!(burden_total(40, Some(&max_c)), 70);
        // 上限：95 + 30 → 封顶 100。
        assert_eq!(burden_total(95, Some(&max_c)), 100);
    }

    #[test]
    fn tier_fires_once_and_text_carries_numbers() {
        let r = reading(true, Some(1.35), [1.20, 1.60, 2.40]);
        let d = pending_burden_notice(30, Some(&r), &[]).expect("tier 50 reached");
        assert_eq!(d.notice.tier, "50");
        assert_eq!(d.tiers_to_mark, vec!["50"]);
        assert!(
            d.notice.text.contains("负担总值 50%"),
            "total in text: {}",
            d.notice.text
        );
        assert!(
            d.notice.text.contains("水位 30%"),
            "fatigue in text: {}",
            d.notice.text
        );
        // 已投递 → 不重复。
        assert_eq!(
            pending_burden_notice(30, Some(&r), &notified(&["50"])),
            None
        );
    }

    #[test]
    fn jump_marks_all_candidates_and_reports_highest() {
        let r = reading(true, Some(9.0), [1.2, 1.5, 2.6]);
        // 60 + 30 = 90 → 三档一并落档，只报最高档。
        let d = pending_burden_notice(60, Some(&r), &[]).expect("tiers reached");
        assert_eq!(d.notice.tier, "90");
        assert_eq!(d.tiers_to_mark, vec!["50", "70", "90"]);
        assert!(d.notice.text.contains("负担总值 90%"));
    }

    #[test]
    fn later_latch_fires_after_earlier_delivery() {
        let r = reading(true, Some(9.0), [1.2, 1.5, 2.2]);
        let d =
            pending_burden_notice(60, Some(&r), &notified(&["50", "70"])).expect("tier 90 reached");
        assert_eq!(d.notice.tier, "90");
        assert_eq!(d.tiers_to_mark, vec!["90"]);
    }

    #[test]
    fn all_delivered_yields_none() {
        let r = reading(true, Some(9.0), [1.2, 1.5, 2.2]);
        assert_eq!(
            pending_burden_notice(60, Some(&r), &notified(&["50", "70", "90"])),
            None
        );
    }
}
