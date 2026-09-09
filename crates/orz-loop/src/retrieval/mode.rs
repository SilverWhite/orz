//! Retrieval enable gate + deprecated wire-mode compatibility surface — 0t
//! (2026-09-09, ADR-0010 §14.65 / 设计 v1.3).
//!
//! 检索模式三值状态（`off`/`local_browser`/`framework_fallback`）已 γ 整体
//! 退役：不再作为激活状态，机械降级与 mode-transition 事件族不再产出。
//! 原 `off` 的「未授权默认无可检索工具」语义由**独立检索启用门**承接——
//! 未启用会话主面与外部 lane 均无检索工具，任何检索派发 fail-closed 拒绝
//! （拒绝码 `retrieval_not_enabled`，脱离模式状态机）。
//!
//! [`RetrievalMode`] 保留为**废弃的 wire 兼容解析类型**：orz-bin env/CLI 与
//! ACP 会话创建面继续接受旧值（`off`/`local_browser`/`framework_fallback`）
//! 以避免旧调用方硬失败，但值不再有 lane/降级语义；非 `off` 旧值仅经
//! [`RetrievalMode::enables_retrieval`] 映射为 enable gate 开（legacy
//! 兼容通道，见 `orz-host` 会话创建与 `orz-bin` env 解析）。schema 侧枚举
//! 与 `verify_retrieval_mode` conformance 族不做枚举收缩，保留为旧 v0.2 刊
//! 只读回放规则（R3 生产者侧退役策略）。

use serde::{Deserialize, Serialize};

/// DEPRECATED (0t, 2026-09-09, ADR-0010 §14.65): 三值检索模式的旧 wire
/// 值。仅用于 orz-bin env/CLI 与 ACP 会话创建面的**兼容解析**（保留字段、
/// 标注废弃、不再影响 lane 语义）；新代码不得把它当激活状态存储或读取。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    Off,
    LocalBrowser,
    FrameworkFallback,
}

impl RetrievalMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RetrievalMode::Off => "off",
            RetrievalMode::LocalBrowser => "local_browser",
            RetrievalMode::FrameworkFallback => "framework_fallback",
        }
    }

    /// Parse the legacy session-level wire form (ACP session/new + env/CLI).
    pub fn from_wire(value: Option<&str>) -> Option<RetrievalMode> {
        match value {
            Some("local_browser") => Some(RetrievalMode::LocalBrowser),
            Some("framework_fallback") => Some(RetrievalMode::FrameworkFallback),
            Some("off") => Some(RetrievalMode::Off),
            _ => None,
        }
    }

    /// Legacy 兼容映射：非 `off` 旧值 ⇒ 检索启用（lane 差异不再存在）。
    pub fn enables_retrieval(&self) -> bool {
        !matches!(self, RetrievalMode::Off)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0t (2026-09-09, ADR-0010 §14.65): wire 兼容解析保留三个旧值；非 off
    /// 值仅映射为启用门开，不再区分车道。
    #[test]
    fn legacy_wire_mode_parses_and_maps_to_gate_only() {
        assert_eq!(
            RetrievalMode::from_wire(Some("off")),
            Some(RetrievalMode::Off)
        );
        assert_eq!(
            RetrievalMode::from_wire(Some("local_browser")),
            Some(RetrievalMode::LocalBrowser)
        );
        assert_eq!(
            RetrievalMode::from_wire(Some("framework_fallback")),
            Some(RetrievalMode::FrameworkFallback)
        );
        assert_eq!(RetrievalMode::from_wire(Some("bogus")), None);
        assert_eq!(RetrievalMode::from_wire(None), None);

        assert!(!RetrievalMode::Off.enables_retrieval());
        assert!(RetrievalMode::LocalBrowser.enables_retrieval());
        assert!(RetrievalMode::FrameworkFallback.enables_retrieval());
    }

    /// 三值序列化 wire 名保持稳定（旧侧车/参数兼容读取依赖它）。
    #[test]
    fn legacy_wire_serde_names_stable() {
        assert_eq!(RetrievalMode::Off.as_str(), "off");
        assert_eq!(RetrievalMode::LocalBrowser.as_str(), "local_browser");
        assert_eq!(
            RetrievalMode::FrameworkFallback.as_str(),
            "framework_fallback"
        );
        assert_eq!(
            serde_json::to_string(&RetrievalMode::FrameworkFallback).unwrap(),
            "\"framework_fallback\""
        );
    }
}
