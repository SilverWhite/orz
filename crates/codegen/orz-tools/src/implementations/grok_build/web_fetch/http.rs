//! HTTP client construction for `web_fetch`.
//!
//! REV-083-11（2026-09-27）：客户端按 **fetch/hop 粒度构建**——每次抓取把
//! 「本 hop 解析并校验通过的地址集」pin 进专用客户端（[`build_pinned_client`]，
//! `resolve_to_addrs`），连接只能落在已校验地址上，校验与连接之间无法换地址
//! （封 DNS rebinding TOCTOU）。既有共享客户端（ArcSwapOption 缓存＋transport
//! 错误后失效重建）随本改造退役：每 fetch 独立连接池下，transport 错误天然
//! 不再污染后续抓取（原防连接池中毒语义等价承接）。

use super::config::WebFetchParams;
use super::error::WebFetchError;

fn builder(params: &WebFetchParams) -> Result<reqwest::ClientBuilder, WebFetchError> {
    let mut builder = reqwest::Client::builder()
        .timeout(params.timeout_secs())
        .connect_timeout(std::time::Duration::from_secs(10))
        // We manage redirects for SSRF.
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(2)
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .tcp_nodelay(true)
        // Reduce size of incoming payloads.
        .gzip(true)
        .brotli(true)
        .deflate(true);

    // Route all traffic through the egress proxy when configured.（代理形态下
    // 目标 host 不在本地解析——pin 无效果但无害，SSRF 校验仍先行执行。）
    if let Some(ref endpoint) = params.proxy_endpoint {
        let proxy = reqwest::Proxy::all(endpoint)
            .map_err(|e| WebFetchError::ProxyConfigError(e.to_string()))?;
        builder = builder.proxy(proxy);
    }

    Ok(builder)
}

/// SSRF pin 客户端（REV-083-11）：`host` 的本地解析一律返回 `addrs`
/// （本 hop 已通过 [`super::ssrf::resolve_and_check`] 校验的地址集）——
/// 请求只能连接到校验过的地址。
pub(crate) fn build_pinned_client(
    params: &WebFetchParams,
    host: &str,
    addrs: &[std::net::SocketAddr],
) -> Result<reqwest::Client, WebFetchError> {
    builder(params)?
        .resolve_to_addrs(host, addrs)
        .build()
        .map_err(WebFetchError::ClientBuildError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pinned_client_succeeds() {
        let params = WebFetchParams::default();
        let addrs = vec!["127.0.0.1:1".parse().unwrap()];
        assert!(build_pinned_client(&params, "example.invalid", &addrs).is_ok());
    }

    #[test]
    fn build_pinned_client_with_proxy_endpoint() {
        let params = WebFetchParams {
            proxy_endpoint: Some("https://proxy.corp.example.com".into()),
            ..Default::default()
        };
        // Should succeed — reqwest accepts the proxy URL.
        let addrs = vec!["127.0.0.1:1".parse().unwrap()];
        assert!(build_pinned_client(&params, "example.invalid", &addrs).is_ok());
    }

    #[test]
    fn build_pinned_client_with_invalid_proxy_endpoint() {
        let params = WebFetchParams {
            proxy_endpoint: Some("not a valid url".into()),
            ..Default::default()
        };
        let result = build_pinned_client(&params, "example.invalid", &[]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("proxy"),
            "Expected proxy-related error, got: {err}"
        );
    }
}
