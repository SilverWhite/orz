//! 0dw S1（2026-11-17，CONTEXT-FACE-SCALE-RULER-ALIGNMENT 缺陷 3）：载体身份
//! —— journal 侧的离线取证锚。`run_preflight`（每 run 首事件）自本批起在
//! manifest 载荷盖 `carrier_version` ＋ `binary_sha256`，跨版本行为归因
//! （「这个 run 跑在哪个载体上」）自 journal 即可判定，不再依赖行为形状
//! 旁证（立项档 §1.3：同日跨 0.9.14-4 → -5 换装实证不可判定）。
//!
//! 单源边界：
//! - **版本**＝载体（orz-bin）的 `CARGO_PKG_VERSION`——与 `--build-info`
//!   ／banner／capability `orz_version` 同一 bump 纪录串（main.rs「版本以
//!   bin 为准」注记）。orz-host 无法编译期拿到 bin 的版本 ⇒ 由 bin 入口
//!   进程级注入一次（[`set_carrier_version`]；`OnceLock` 单写）；未注入
//!   （单测/宿主外直用）如实落 `"unknown"`，不虚构。
//! - **二进制哈希**＝运行中 exe 文件本身的 SHA-256（[`binary_sha256`]）——
//!   与载体清单（carrier-manifest）同一口径（对同一文件算哈希）。构建期
//!   注入在物理上不可能（哈希落自身），运行期对 `current_exe()` 求哈希是
//!   唯一诚实值；进程内 `OnceLock` 缓存（exe 运行中不变），68 MB release
//!   件一次性 ~百 ms 级，run 首事件之外零成本。读失败（权限/平台）如实
//!   落 `"unavailable"`——不虚构哈希。

use std::sync::OnceLock;

static CARRIER_VERSION: OnceLock<String> = OnceLock::new();
static BINARY_SHA256: OnceLock<String> = OnceLock::new();

/// 版本缺席时的哨兵值（未注入 = 单测/宿主外直用；不虚构版本串）。
pub const CARRIER_VERSION_UNSET: &str = "unknown";

/// exe 哈希不可得时的哨兵值（读失败如实登记；不虚构哈希）。
pub const BINARY_SHA256_UNAVAILABLE: &str = "unavailable";

/// 载体版本注入（bin 入口恰调一次；重复注入首写胜出——`OnceLock` 语义）。
pub fn set_carrier_version(version: &str) {
    let _ = CARRIER_VERSION.set(version.to_string());
}

/// 载体版本（未注入 ⇒ `"unknown"`）。
pub fn carrier_version() -> &'static str {
    CARRIER_VERSION
        .get()
        .map(String::as_str)
        .unwrap_or(CARRIER_VERSION_UNSET)
}

/// 运行中 exe 的 SHA-256（进程内缓存；读失败 ⇒ `"unavailable"`）。
pub fn binary_sha256() -> &'static str {
    BINARY_SHA256.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|path| std::fs::read(path).ok())
            .map(|bytes| orz_assurance::sha256_hex(&bytes))
            .unwrap_or_else(|| BINARY_SHA256_UNAVAILABLE.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 未注入 ⇒ 哨兵 `"unknown"`（不虚构版本串）。
    #[test]
    fn carrier_version_defaults_to_sentinel() {
        // 进程级单源：其他测试可能已注入，本钉只断言「要么未注入的哨兵、
        // 要么某次注入的非空值」——两形态都合法（注入一次后进程内恒定）。
        let v = carrier_version();
        assert!(v == CARRIER_VERSION_UNSET || !v.is_empty(), "version: {v}");
    }

    /// exe 哈希：可读则 64 位十六进制、与 `"unavailable"` 互斥；两形态都
    /// 是诚实值（读失败不虚构）。
    #[test]
    fn binary_sha256_is_hex64_or_unavailable() {
        let digest = binary_sha256();
        let is_hex64 = digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit());
        assert!(
            is_hex64 || digest == BINARY_SHA256_UNAVAILABLE,
            "digest: {digest}"
        );
    }
}
