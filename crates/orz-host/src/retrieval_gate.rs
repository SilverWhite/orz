//! 检索启用门（session-level retrieval enable gate）——**单源解析**。
//!
//! 历史与现状：0t（2026-09-09，ADR-0010 §14.65）把该门定为 fail-closed
//! （缺省关，未启用会话无任何检索工具）。**2026-10-17 用户令**：
//! 「正常版本的 orz 检索门默认开即可，检索部分是很重要的信息获取，得常开」
//! ——缺省改为**开**；显式关保留（`ORZ_RETRIEVAL_ENABLED=0`／
//! legacy `--retrieval-mode off`／`ORZ_RETRIEVAL_MODE=off`）。
//!
//! 优先级（本文件是唯一实现；`orz-bin` 入口与本 crate 各会话创建面共用）：
//!   显式 `ORZ_RETRIEVAL_ENABLED`（`1`/`true`/`yes`/`on` ⇒ 开，其余显式值 ⇒ 关）
//!   > legacy `ORZ_RETRIEVAL_MODE` 兼容映射（非 `off` ⇒ 开，打印弃用提示）
//!   > **默认开**。
//!
//! 边界：本门只决定「会话是否注册检索工具族」（主面 + 外部 lane 的
//! `browser_read`／`web_search`／`web_fetch`）。浏览器本身可用性仍是
//! 事件事实（`browser_launch_result`），不因门开而被假定可用。

use orz_loop::controller::RetrievalMode;

/// 纯解析核（可测无环境依赖）：`explicit`＝`ORZ_RETRIEVAL_ENABLED` 原值、
/// `legacy`＝`ORZ_RETRIEVAL_MODE` 原值，二者缺省即「未设置」。
pub fn resolve_retrieval_enabled(explicit: Option<&str>, legacy: Option<&str>) -> bool {
    if let Some(v) = explicit {
        return matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
    }
    if let Some(mode) = legacy.and_then(|v| RetrievalMode::from_wire(Some(v))) {
        // DEPRECATED（0t）：非 off 旧值 ⇒ 开；仅作兼容映射，不再有 lane 语义。
        return mode.enables_retrieval();
    }
    true
}

/// 会话级检索启用门解析（env 薄壳；唯一入口给 orz-bin 与 host 各面用）。
pub fn retrieval_enabled_from_env() -> bool {
    let explicit = std::env::var("ORZ_RETRIEVAL_ENABLED").ok();
    let legacy = std::env::var("ORZ_RETRIEVAL_MODE").ok();
    if explicit.is_none() && legacy.is_some() {
        eprintln!(
            "warning: --retrieval-mode / ORZ_RETRIEVAL_MODE is deprecated \
             (0t γ, ADR-0010 §14.65) — lane selection is automatic; the \
             enable gate is ON by default, use ORZ_RETRIEVAL_ENABLED=0 to \
             switch it off"
        );
    }
    resolve_retrieval_enabled(explicit.as_deref(), legacy.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-17 用户令：缺省**开**（此前 fail-closed 缺省关）。
    #[test]
    fn default_is_enabled() {
        assert!(resolve_retrieval_enabled(None, None));
    }

    /// 显式关仍生效（用户在需要确定性/离线的场合可关）。
    #[test]
    fn explicit_off_wins() {
        for v in ["0", "false", "no", "off", "FALSE", " off "] {
            assert!(!resolve_retrieval_enabled(Some(v), None), "value {v:?}");
        }
        // legacy off 亦关；两种显式并存时显式门优先。
        assert!(!resolve_retrieval_enabled(None, Some("off")));
        assert!(resolve_retrieval_enabled(Some("1"), Some("off")));
        assert!(!resolve_retrieval_enabled(Some("0"), Some("local_browser")));
    }

    /// 显式开与 legacy 非 off 映射保持旧行为。
    #[test]
    fn explicit_on_and_legacy_mapping() {
        for v in ["1", "true", "yes", "on", "ON", " yes "] {
            assert!(resolve_retrieval_enabled(Some(v), None), "value {v:?}");
        }
        assert!(resolve_retrieval_enabled(None, Some("local_browser")));
        assert!(resolve_retrieval_enabled(None, Some("framework_fallback")));
        // 无法解析的 legacy 值不构成显式关 ⇒ 落到默认（开）。
        assert!(resolve_retrieval_enabled(None, Some("bogus")));
    }
}
