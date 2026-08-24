//! GAP-RETRIEVAL-TOOLS / RETRIEVAL-SUBAGENT-WIRING (2026-08-10/25):
//! 会话级检索模式的机械 capability probe 与模式 A 自动定档——local_browser
//! probe 失败（`browser_launch_failed`）→ 机械降级 framework_fallback。
//! ACP 会话路径与 CLI 一次性运行路径共用本入口（ADR-0010 §14.40），
//! 避免 probe/降级逻辑漂移。

use std::path::Path;

use orz_loop::controller::{RetrievalCapability, RetrievalMode};

use crate::OrzHost;

/// RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25)：收紧的浏览器启动失败
/// 判定——精确匹配 `browser_launch_failed:` 前缀（带冒号分隔符），不接受
/// `browser_launch_failedX` 之类的宽松前缀。probe 产出的 reason 恒为
/// `format!("browser_launch_failed: {cause}")`。
pub(crate) fn is_browser_launch_failed(reason: &str) -> bool {
    reason.starts_with("browser_launch_failed:")
}

/// 模式 A 降级决策（纯函数，可测）：local_browser + 浏览器启动失败 →
/// Some(framework_fallback)；其余（浏览器可用、非启动原因、其他模式）→
/// None。页面级失败（LOGIN_REQUIRED/CAPTCHA/PAGE_BLOCKED 等 §3.7.2 显式
/// 状态）不在 probe 内产生——probe 只判浏览器启动可用性。
pub(crate) fn mode_a_degrade_target(
    mode: RetrievalMode,
    capability: &RetrievalCapability,
) -> Option<RetrievalMode> {
    if mode == RetrievalMode::LocalBrowser
        && matches!(
            capability,
            RetrievalCapability::Degraded(reason) if is_browser_launch_failed(reason)
        )
    {
        Some(RetrievalMode::FrameworkFallback)
    } else {
        None
    }
}

/// 模式 A probe 结果——`effective_mode`（降级后）/ `capability`（降级后以
/// 真实能力重探）/ `previous_mode`（降级前模式，transition 的 old_mode）/
/// `degraded`（是否发生机械降级）。
pub struct ModeAProbeOutcome {
    pub effective_mode: RetrievalMode,
    pub capability: RetrievalCapability,
    pub previous_mode: Option<RetrievalMode>,
    pub degraded: bool,
}

/// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：模式 A 机械
/// probe 定档的共享入口——probe 当前模式；local_browser 浏览器启动失败
/// 时机械降级 framework_fallback 并以真实能力重探（web_search_configured
/// 决定 Available / Degraded）——transition 的 capability_status 反映降级
/// 后模式，不残留浏览器失败原因。ACP 会话（快照改写 + transition pending）
/// 与 CLI 一次性运行（直接接线）共用。
pub async fn probe_retrieval_with_mode_a(
    mode: RetrievalMode,
    web_search_configured: bool,
    host: &mut OrzHost,
    workspace: &Path,
    session_id: &str,
) -> ModeAProbeOutcome {
    let mut capability =
        probe_retrieval_capability(mode, web_search_configured, host, workspace, session_id).await;
    let mut effective_mode = mode;
    let previous_mode = match mode_a_degrade_target(mode, &capability) {
        Some(target) => {
            effective_mode = target;
            capability = probe_retrieval_capability(
                target,
                web_search_configured,
                host,
                workspace,
                session_id,
            )
            .await;
            Some(mode)
        }
        None => None,
    };
    ModeAProbeOutcome {
        effective_mode,
        capability,
        previous_mode,
        degraded: previous_mode.is_some(),
    }
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): mechanical capability probe for the
/// session's mode (ADR-0010 §3.7.1 — never a silent fallback). Under
/// framework_fallback web_fetch is always available (no key dependency)
/// and web_search rides the configured API key — missing key = Degraded
/// with an explicit reason, never an implicit switch.
///
/// local_browser (2026-08-10): REAL probe — reuse an already-launched
/// session handle (kept on `StoredSession` so the browser survives across
/// runs on the same profile, no profile-lock conflicts), else discover +
/// launch a headless browser and inject it into the host. Launch failure is
/// `Degraded("browser_launch_failed: <cause>")` — explicit, never a silent
/// fallback to web tools (ADR-0010 §3.7.1/§3.7.2). A re-probe on a later
/// prompt retries the launch (self-healing: the browser or profile may
/// have become available meanwhile).
pub(crate) async fn probe_retrieval_capability(
    mode: RetrievalMode,
    web_search_configured: bool,
    host: &mut OrzHost,
    workspace: &Path,
    session_id: &str,
) -> RetrievalCapability {
    match mode {
        RetrievalMode::Off => {
            RetrievalCapability::Unsupported("retrieval_mode_not_selected".to_string())
        }
        RetrievalMode::LocalBrowser => {
            if host.browser_ready() {
                return RetrievalCapability::Available;
            }
            match crate::local_browser::probe_launch(workspace, session_id).await {
                Ok(manager) => {
                    host.set_browser_session(std::sync::Arc::new(manager));
                    RetrievalCapability::Available
                }
                Err(cause) => {
                    RetrievalCapability::Degraded(format!("browser_launch_failed: {cause}"))
                }
            }
        }
        RetrievalMode::FrameworkFallback => {
            if web_search_configured {
                RetrievalCapability::Available
            } else {
                RetrievalCapability::Degraded("web_search_not_configured".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RETRIEVAL-SUBAGENT-WIRING 审查处理：模式 A 降级决策规则——启动失败
    /// 降级 / 浏览器可用不降级 / 非启动原因不降级 / framework_fallback
    /// 不降级；前缀判定收紧（缺冒号分隔符不命中）。
    #[test]
    fn mode_a_degrade_rules() {
        // 启动失败 → 降级 framework_fallback。
        assert_eq!(
            mode_a_degrade_target(
                RetrievalMode::LocalBrowser,
                &RetrievalCapability::Degraded(
                    "browser_launch_failed: browser_not_found: x".into()
                )
            ),
            Some(RetrievalMode::FrameworkFallback)
        );
        // 浏览器可用 → 不降级。
        assert_eq!(
            mode_a_degrade_target(RetrievalMode::LocalBrowser, &RetrievalCapability::Available),
            None
        );
        // 非 browser_launch_failed 的 Degraded 原因不触发降级
        // （probe 只判浏览器启动；页面级失败是 §3.7.2 调用期显式状态）。
        assert_eq!(
            mode_a_degrade_target(
                RetrievalMode::LocalBrowser,
                &RetrievalCapability::Degraded("web_search_not_configured".into())
            ),
            None
        );
        // framework_fallback 不降级。
        assert_eq!(
            mode_a_degrade_target(
                RetrievalMode::FrameworkFallback,
                &RetrievalCapability::Available
            ),
            None
        );
        // 收紧：缺冒号分隔符的宽松前缀不命中（browser_launch_failedX）。
        assert_eq!(
            mode_a_degrade_target(
                RetrievalMode::LocalBrowser,
                &RetrievalCapability::Degraded("browser_launch_failedX".into())
            ),
            None
        );
        assert!(is_browser_launch_failed("browser_launch_failed: x"));
        assert!(!is_browser_launch_failed("browser_launch_failed"));
        assert!(!is_browser_launch_failed("browser_launch_failedX"));
    }
}
