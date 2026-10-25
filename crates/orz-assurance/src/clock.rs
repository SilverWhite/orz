//! 北京时间（UTC+8）单一时间源——353 批（2026-10-25 用户裁决）。
//!
//! 全仓时间戳统一为**本机时钟的北京时间表示**（RFC 3339，`+08:00` 偏移）：
//! 日志面（journal / session / 通知）、模型面（时间轴读数）、安全材料
//! （credential / permit / ACAF）一律走这里，不再混用 UTC `Z` 与文件
//! mtime 的本地基准——用户 2026-10-25 裁决："时间混乱确实麻烦，统一使用
//! 本机时间戳，北京时间"。
//!
//! 用固定偏移而非 `chrono::Local`：中国无夏令时，固定 +08:00 结果确定、
//! 跨机器一致、不依赖 `TZ` 环境变量；同一时刻的 `Z` 串与 `+08:00` 串是
//! 同一瞬时，`DateTime::parse_from_rfc3339` 比较面不受影响（安全材料
//! 的比较一律解析而非字典序，见 `acaf::predates_restart_cutoff`）。

use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};

/// 北京时间偏移（秒）＝+8h。
pub const BEIJING_OFFSET_SECS: i32 = 8 * 3600;

/// 固定 +08:00 时区。
pub fn beijing() -> FixedOffset {
    FixedOffset::east_opt(BEIJING_OFFSET_SECS).expect("+08:00 is a valid fixed offset")
}

/// 当前时刻的北京时间 RFC 3339 串（纳秒精度，形如
/// `2026-10-25T12:55:00.828193200+08:00`）。
pub fn now() -> String {
    Utc::now().with_timezone(&beijing()).to_rfc3339()
}

/// 当前时刻的北京时间 RFC 3339 串（指定精度，恒带 `+08:00`）。
pub fn now_opts(secs: SecondsFormat) -> String {
    Utc::now()
        .with_timezone(&beijing())
        .to_rfc3339_opts(secs, false)
}

/// 把任意时刻转成北京时间 RFC 3339 串（安全材料的签发/过期面用）。
pub fn to_beijing_opts(value: DateTime<Utc>, secs: SecondsFormat) -> String {
    value.with_timezone(&beijing()).to_rfc3339_opts(secs, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beijing_offset_is_plus_eight() {
        assert_eq!(beijing().local_minus_utc(), 8 * 3600);
    }

    #[test]
    fn now_carries_plus_eight_offset_and_round_trips() {
        let stamp = now();
        assert!(
            stamp.ends_with("+08:00"),
            "北京时间串必须以 +08:00 收尾: {stamp}"
        );
        let parsed = DateTime::parse_from_rfc3339(&stamp).expect("必须可解析");
        assert_eq!(parsed.offset().local_minus_utc(), 8 * 3600);
    }

    #[test]
    fn same_instant_z_and_beijing_compare_equal() {
        let utc = Utc::now();
        let utc_stamp = utc.to_rfc3339_opts(SecondsFormat::Secs, true);
        let bj_stamp = to_beijing_opts(utc, SecondsFormat::Secs);
        let a = DateTime::parse_from_rfc3339(&utc_stamp).unwrap();
        let b = DateTime::parse_from_rfc3339(&bj_stamp).unwrap();
        assert_eq!(a, b, "同一瞬时的两种表示必须解析为同一时刻");
    }
}
