//! 0bv（2026-09-26）：`web_search` 浏览器 SERP 链首的宿主实现与装配。
//!
//! 链语义（0bs ⑧ 裁决面）：`web_search` 检索链 = 浏览器 SERP（若可用）→
//! 本地 HTTP 分段 → provider 合成。本模块把宿主侧浏览器会话经 orz-tools 的
//! 资源缝（[`orz_tools::types::resources::BrowserSerpBackend`]）注入工具侧；
//! 与 `browser_control search` 共用同一句柄槽、同一会话 SERP 状态（软备忘／
//! 冷却／引擎导航上限）——两条路径的真实导航数都并入 loop 层同一预算账本
//! （单账本并账；见 `host_exec/serp.rs` 的结算面）。
//!
//! 裁决 D-f（0bv，2026-09-26）：适配器**不自动拉起浏览器**。浏览器未就绪即
//! `browser_unavailable` 让渡（链走本地 HTTP → provider 兜底）——web_search
//! 不以启动 Chromium 为副作用；会话键（`session_id`）在宿主构造后才设置，
//! 此处拉起还会有 profile 键分歧面。跟进项：共享会话键的自动拉起。

use std::sync::{Arc, Mutex};
use std::time::Instant;

use orz_tools::types::resources::{
    BrowserSerpBackend, BrowserSerpFailure, BrowserSerpHit, BrowserSerpOutcome,
};

use crate::local_browser::SharedBrowser;

/// 与 `browser_control search` 共用同一浏览器句柄槽的链首实现。
#[derive(Clone)]
pub struct HostBrowserSerp {
    browser: Arc<Mutex<SharedBrowser>>,
}

impl HostBrowserSerp {
    pub fn new(browser: Arc<Mutex<SharedBrowser>>) -> Self {
        Self { browser }
    }
}

#[async_trait::async_trait]
impl BrowserSerpBackend for HostBrowserSerp {
    async fn search(&self, query: &str) -> Result<BrowserSerpOutcome, BrowserSerpFailure> {
        let browser = self
            .browser
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if !browser.ready() {
            return Err(BrowserSerpFailure {
                cause: "browser_unavailable".to_string(),
                engine: None,
                detail: "browser session is not ready; the lane yields to local_http \
                         (lazy launch belongs to the browser tool paths)"
                    .to_string(),
                navigations: 0,
                session_used: 0,
                session_cap: 0,
            });
        }
        let started = Instant::now();
        // 与 `browser_control search` 完全同径执行（引擎链／软备忘／冷却／
        // 会话上限都在会话内实现；信封即取证面）。
        let args = serde_json::json!({ "action": "search", "query": query });
        let call = crate::local_browser::handle_browser_control(browser.as_ref(), &args).await;
        let (session_used, session_cap) = browser
            .serp_session_navigations()
            .await
            .unwrap_or((0, crate::local_browser::SERP_MAX_NAVIGATIONS_PER_SESSION as u32));
        let envelope = match call {
            Ok(result) => serde_json::from_str::<serde_json::Value>(&result.output)
                .unwrap_or(serde_json::Value::Null),
            Err(err) => {
                return Err(BrowserSerpFailure {
                    cause: "host_error".to_string(),
                    engine: None,
                    detail: bounded(&format!("{err:?}")),
                    navigations: 0,
                    session_used,
                    session_cap,
                });
            }
        };
        let navigations = envelope
            .get("engine_attempts")
            .and_then(|v| v.as_array())
            .map(|attempts| {
                attempts
                    .iter()
                    .filter(|attempt| {
                        matches!(
                            attempt.get("status").and_then(|v| v.as_str()),
                            Some("ok") | Some("failed")
                        )
                    })
                    .count() as u32
            })
            .unwrap_or(0);
        let hits = parse_hits(&envelope);
        if !hits.is_empty() {
            return Ok(BrowserSerpOutcome {
                engine: envelope
                    .get("engine")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                hits,
                navigations: navigations.max(1),
                session_used,
                session_cap,
                waited_ms: started.elapsed().as_millis() as u64,
            });
        }
        // 零命中／拒绝信封：如实让渡（cause 取机器可读小集；detail 有界）。
        let serialized = serde_json::to_string(&envelope).unwrap_or_default();
        let cause = if serialized.contains("browser_control_search_cap_exceeded") {
            "ceiling"
        } else if serialized.contains("all_engines_failed") {
            "all_engines_failed"
        } else {
            "empty"
        };
        Err(BrowserSerpFailure {
            cause: cause.to_string(),
            engine: envelope
                .get("engine")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            detail: bounded(&serialized),
            navigations,
            session_used,
            session_cap,
        })
    }
}

/// 信封 → 有界命中行（`tier`/`weight`/`reason` 与 `browser_control search`
/// 同口径；缺字段的行如实跳过，不虚构）。
fn parse_hits(envelope: &serde_json::Value) -> Vec<BrowserSerpHit> {
    envelope
        .get("results")
        .and_then(|v| v.as_array())
        .map(|results| {
            results
                .iter()
                .filter_map(|r| {
                    Some(BrowserSerpHit {
                        title: r.get("title")?.as_str()?.to_string(),
                        url: r.get("url")?.as_str()?.to_string(),
                        snippet: r
                            .get("snippet")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        tier: r
                            .get("tier")
                            .and_then(|v| v.as_str())
                            .unwrap_or("default")
                            .to_string(),
                        weight: r.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.5),
                        reason: r
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 诊断 detail 的有界渲染（模型面/结构化面都不得被未定界文本撑爆）。
fn bounded(text: &str) -> String {
    const CAP: usize = 300;
    if text.chars().count() <= CAP {
        return text.to_string();
    }
    let mut out: String = text.chars().take(CAP).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 未就绪句柄 ⇒ 车道如实不可用（让渡语义；不尝试拉起）。
    #[tokio::test]
    async fn unavailable_browser_reports_browser_unavailable() {
        let browser: Arc<Mutex<SharedBrowser>> = Arc::new(Mutex::new(Arc::new(
            crate::local_browser::UnavailableBrowserSession::new("test-only".into()),
        )));
        let serp = HostBrowserSerp::new(browser);
        let err = serp.search("rust borrow checker").await.unwrap_err();
        assert_eq!(err.cause, "browser_unavailable");
        assert_eq!(err.navigations, 0);
        assert_eq!(err.session_used, 0);
    }

    #[test]
    fn parse_hits_reads_weighted_envelope_rows() {
        let envelope = serde_json::json!({
            "results": [
                {
                    "title": "Rust docs",
                    "url": "https://doc.rust-lang.org/book/",
                    "snippet": "the book",
                    "tier": "authoritative",
                    "weight": 0.9,
                    "reason": "gov-education"
                },
                {"title": "no url row"}
            ]
        });
        let hits = parse_hits(&envelope);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].tier, "authoritative");
        assert_eq!(hits[0].weight, 0.9);
        assert_eq!(hits[0].url, "https://doc.rust-lang.org/book/");
    }

    #[test]
    fn bounded_caps_detail_length() {
        let long = "x".repeat(1000);
        let out = bounded(&long);
        assert_eq!(out.chars().count(), 301);
        assert!(out.ends_with('…'));
        assert_eq!(bounded("short"), "short");
    }
}
