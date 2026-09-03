//! 会话疲劳度（P2-13 B3，2026-09-03，ADR-0010 §14.52 / 设计 v0.8
//! §10.1/§11.2/§13.2；B3 复审裁决登记于 REVIEW_HANDLING——移除压缩轮数
//! 门槛、只按黑板 live 字节水位给档）。
//!
//! 度量 = 黑板 live 侧车紧凑 JSON 字节 / W（`FoldParams::board_bytes_
//! threshold`，默认 10 MiB，env `ORZ_BLACKBOARD_LIVE_BUDGET_BYTES`
//! 覆盖——与 B2 折叠共用同一 W，见 render_fold）。单调、机械、可复核。
//! 语义 = 工作总量代理（非复杂度模型）。
//!
//! 档位（每档越线提醒一次，用户忽略不重复；50/70/90 均只按水位判定，
//! 无压缩轮数门槛——W=10 MiB 对纯文本已足够宽松，压缩次数不参与）：
//! - 50%：信息层无门槛（建议收束当前线索/核对遗留）；
//! - 70%：建议换对话（新对话 = 新黑板，旧黑板随会话存档）；
//! - 90%：接近寿命上限提示始终给出（事实层，措辞软、由用户决定）。
//!
//! 单次运行只给一条：取“达到且未提醒”的最高档；同一次判定内把所有已
//! 越线的未提醒档一并落档（巨幅跳跃不刷屏，跳过的低档不滞留到后续 run
//! 倒序补发——B3 复审 P2-1 裁决）。

/// 档位键与阈值（百分比）。顺序 = 递增。
pub const FATIGUE_TIER_50: &str = "50";
pub const FATIGUE_TIER_70: &str = "70";
pub const FATIGUE_TIER_90: &str = "90";
pub const FATIGUE_TIER_ORDER: [(&str, u64); 3] = [
    (FATIGUE_TIER_50, 50),
    (FATIGUE_TIER_70, 70),
    (FATIGUE_TIER_90, 90),
];

/// 一次待投递的疲劳提醒。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatigueNotice {
    /// 档位键（`50`/`70`/`90`）——宿主持久化到会话侧车，避免重复提醒。
    pub tier: &'static str,
    /// 用户侧文本（机械、事实、软措辞）。
    pub text: String,
}

/// 一次疲劳判定结果：提醒 + 本次应落档的档位键集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatigueDecision {
    /// 待投递提醒（达到且未提醒的最高档）。
    pub notice: FatigueNotice,
    /// 本次应落档的全部档位键（升序，含 `notice.tier`）——巨幅跳跃时
    /// 已越线的低档一并落档，避免后续 run 补发已跳过的低档提示。
    pub tiers_to_mark: Vec<&'static str>,
}

/// 水位百分比（整数截断，0 ≤ pct ≤ 100）。threshold = 0 视为未配置
/// （返回 0，不产生提醒——W 恒为正，防御性回退）。
pub fn fatigue_percent(bytes: usize, threshold: usize) -> u64 {
    if threshold == 0 {
        return 0;
    }
    ((bytes as u128 * 100) / threshold as u128).min(100) as u64
}

/// W（黑板 live 预算，存储字节）——与 B2 折叠共用同一预算与 env 覆盖
/// （`ORZ_BLACKBOARD_LIVE_BUDGET_BYTES`，默认 10 MiB）。render_fold 为
/// crate 内私有模块，宿主面经本入口读取同一口径。
pub fn live_budget_bytes() -> usize {
    crate::render_fold::FoldParams::from_env().board_bytes_threshold
}

/// 计算应当提醒的档位（达到且未提醒）。返回 `None` = 无新档（当前水位
/// 未越过任何新档位）。一次运行最多给一条：取达到的最高未提醒档，并把
/// 本次已越线的所有未提醒档一并放进 `tiers_to_mark`（跳过即视为已投递，
/// 低档不滞留补发）。
pub fn pending_fatigue_notice(
    bytes: usize,
    threshold: usize,
    notified_tiers: &[String],
) -> Option<FatigueDecision> {
    let pct = fatigue_percent(bytes, threshold);
    if pct == 0 {
        return None;
    }
    let budget_mib = threshold as f64 / (1024.0 * 1024.0);
    let used_mib = bytes as f64 / (1024.0 * 1024.0);
    // 已提醒档的最高水位线：低于该线的未提醒档视为已被跳越作废（前缀闭包
    // 纪律），只考虑高于该线且 ≤ 当前水位的“新前沿”档位。
    let max_notified_level = notified_tiers
        .iter()
        .filter_map(|t| {
            FATIGUE_TIER_ORDER
                .iter()
                .find(|(key, _)| key == t)
                .map(|(_, level)| *level)
        })
        .max()
        .unwrap_or(0);
    let frontier: Vec<&'static str> = FATIGUE_TIER_ORDER
        .iter()
        .filter(|(_, level)| *level > max_notified_level && *level <= pct)
        .map(|(tier, _)| *tier)
        .collect();
    if frontier.is_empty() {
        return None;
    }
    let tier = frontier
        .last()
        .copied()
        .expect("frontier is non-empty: last element exists");
    let text = match tier {
        FATIGUE_TIER_50 => format!(
            "（会话提示：黑板使用约 {pct}%（{used_mib:.1}/{budget_mib:.1} MiB）。\
             信息提醒——若工作量大，可考虑适时收束当前线索、核对遗留。）"
        ),
        FATIGUE_TIER_70 => format!(
            "（会话提示：黑板使用约 {pct}%（{used_mib:.1}/{budget_mib:.1} MiB），\
             已达建议换对话水位——是否换新对话由你决定：新对话会开启新黑板，\
             旧黑板随会话存档。）"
        ),
        FATIGUE_TIER_90 => format!(
            "（会话提示：黑板使用约 {pct}%（{used_mib:.1}/{budget_mib:.1} MiB），\
             接近寿命上限——继续使用会增加轮间复杂度与存档压力；是否换对话由你\
             决定。）"
        ),
        _ => unreachable!("fatigue tier order covers 50/70/90"),
    };
    Some(FatigueDecision {
        notice: FatigueNotice { tier, text },
        tiers_to_mark: frontier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notified(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn percent_is_floor_and_bounded() {
        assert_eq!(fatigue_percent(5 * 1024 * 1024, 10 * 1024 * 1024), 50);
        assert_eq!(fatigue_percent(7 * 1024 * 1024, 10 * 1024 * 1024), 70);
        assert_eq!(fatigue_percent(9 * 1024 * 1024, 10 * 1024 * 1024), 90);
        assert_eq!(fatigue_percent(0, 10 * 1024 * 1024), 0);
        assert_eq!(fatigue_percent(99, 10), 100);
        assert_eq!(fatigue_percent(10, 0), 0);
    }

    #[test]
    fn tier_50_no_gate_and_once_per_tier() {
        let w = 10 * 1024 * 1024;
        let bytes = (w as f64 * 0.52) as usize;
        let d = pending_fatigue_notice(bytes, w, &[]).expect("50% reached");
        assert_eq!(d.notice.tier, FATIGUE_TIER_50);
        assert!(d.notice.text.contains("51%"));
        assert_eq!(d.tiers_to_mark, vec![FATIGUE_TIER_50]);
        // 已提醒过 50 → 不再重复（水位未到 70）。
        assert_eq!(
            pending_fatigue_notice(bytes, w, &notified(&[FATIGUE_TIER_50])),
            None
        );
    }

    #[test]
    fn tier_70_advice_given_by_water_level_without_compression_gate() {
        let w = 10 * 1024 * 1024;
        let bytes = (w as f64 * 0.75) as usize;
        let d =
            pending_fatigue_notice(bytes, w, &notified(&[FATIGUE_TIER_50])).expect("70% reached");
        assert_eq!(d.notice.tier, FATIGUE_TIER_70);
        // 无压缩轮数门槛：直接给建议。
        assert!(d.notice.text.contains("已达建议换对话水位"));
        assert_eq!(d.tiers_to_mark, vec![FATIGUE_TIER_70]);
        assert_eq!(
            pending_fatigue_notice(bytes, w, &notified(&[FATIGUE_TIER_50, FATIGUE_TIER_70])),
            None
        );
    }

    #[test]
    fn tier_90_always_reports_at_water_level() {
        let w = 10 * 1024 * 1024;
        let bytes = (w as f64 * 0.95) as usize;
        let d = pending_fatigue_notice(bytes, w, &notified(&[FATIGUE_TIER_50, FATIGUE_TIER_70]))
            .expect("90% reached");
        assert_eq!(d.notice.tier, FATIGUE_TIER_90);
        assert!(d.notice.text.contains("95%"));
        assert_eq!(d.tiers_to_mark, vec![FATIGUE_TIER_90]);
        // 90 也提醒过 → 静默。
        assert_eq!(
            pending_fatigue_notice(
                bytes,
                w,
                &notified(&[FATIGUE_TIER_50, FATIGUE_TIER_70, FATIGUE_TIER_90])
            ),
            None
        );
    }

    #[test]
    fn large_jump_marks_all_crossed_tiers_and_never_replays_lower() {
        let w = 10 * 1024 * 1024;
        let bytes = (w as f64 * 0.95) as usize;
        // 一次从 <50 跳到 95：只取最高未提醒档（90），但 50/70/90 全部
        // 落档——后续 run 不再倒序补发 70/50。
        let d = pending_fatigue_notice(bytes, w, &[]).expect("jump reaches 90");
        assert_eq!(d.notice.tier, FATIGUE_TIER_90);
        assert_eq!(
            d.tiers_to_mark,
            vec![FATIGUE_TIER_50, FATIGUE_TIER_70, FATIGUE_TIER_90]
        );
        let marked = d
            .tiers_to_mark
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        assert_eq!(pending_fatigue_notice(bytes, w, &marked), None);
    }

    #[test]
    fn legacy_higher_tier_notified_supersedes_lower_unnotified() {
        let w = 10 * 1024 * 1024;
        // 侧车异常态（如只记了 70）：低档 50 视为已被跳越作废，不补发。
        let bytes = (w as f64 * 0.75) as usize;
        assert_eq!(
            pending_fatigue_notice(bytes, w, &notified(&[FATIGUE_TIER_70])),
            None
        );
        let bytes90 = (w as f64 * 0.95) as usize;
        let d = pending_fatigue_notice(bytes90, w, &notified(&[FATIGUE_TIER_70]))
            .expect("90 still fresh");
        assert_eq!(d.notice.tier, FATIGUE_TIER_90);
        assert_eq!(d.tiers_to_mark, vec![FATIGUE_TIER_90]);
    }

    #[test]
    fn normal_climb_delivers_each_tier_once_in_order() {
        let w = 10 * 1024 * 1024;
        let mut notified = notified(&[]);
        let b50 = (w as f64 * 0.52) as usize;
        let d = pending_fatigue_notice(b50, w, &notified).expect("50");
        notified.extend(d.tiers_to_mark.iter().map(|s| s.to_string()));
        let b70 = (w as f64 * 0.75) as usize;
        let d = pending_fatigue_notice(b70, w, &notified).expect("70");
        assert_eq!(d.notice.tier, FATIGUE_TIER_70);
        notified.extend(d.tiers_to_mark.iter().map(|s| s.to_string()));
        let b90 = (w as f64 * 0.95) as usize;
        let d = pending_fatigue_notice(b90, w, &notified).expect("90");
        assert_eq!(d.notice.tier, FATIGUE_TIER_90);
        notified.extend(d.tiers_to_mark.iter().map(|s| s.to_string()));
        assert_eq!(
            notified,
            vec![
                FATIGUE_TIER_50.to_string(),
                FATIGUE_TIER_70.to_string(),
                FATIGUE_TIER_90.to_string()
            ]
        );
        assert_eq!(pending_fatigue_notice(b90, w, &notified), None);
    }
}
