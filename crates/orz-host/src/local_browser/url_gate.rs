//! URL navigation gate for the `local_browser` lane (ADR-0010 §3.7.3).
//!
//! The gate runs BEFORE the initial navigation and is re-checked after every
//! redirect (`Page.frameNavigated` + a final `location.href` check). Default
//! policy: only `http`/`https` public addresses — `file://`, browser-internal
//! pages, localhost, private IPs, cloud metadata and embedded credentials are
//! all rejected. Webpage content is evidence, never instruction; the gate is
//! fail-closed (a DNS failure blocks the URL).
//!
//! Two layers:
//! - [`check_navigation_url_sync`] — pure URL-shape checks (scheme, length,
//!   credentials, hostname blocklist, IP-literal policy). No I/O.
//! - [`check_navigation_url`] — sync layer + DNS resolution and resolved-
//!   address policy, reusing the `web_fetch` SSRF guard
//!   (`orz_tools::...::web_fetch::ssrf`).

use std::net::IpAddr;

use orz_tools::implementations::grok_build::web_fetch::WebFetchError;
use orz_tools::implementations::grok_build::web_fetch::ssrf;
use url::Url;

/// Max accepted URL length (defense in depth — the browser accepts longer).
pub const MAX_URL_LEN: usize = 4096;

/// Hostnames rejected without DNS resolution (defense in depth — the SSRF
/// resolver would also block them; this fast path never depends on DNS).
const BLOCKED_HOST_NAMES: &[&str] = &[
    "localhost",
    "metadata",
    "metadata.internal",
    "metadata.google.internal",
];

/// Why a navigation URL was rejected. Each maps to a stable tool error code
/// (see [`UrlGateError::tool_error_code`]) so the model sees an explicit
/// failure, never a silent fallback (ADR-0010 §3.7.2).
#[derive(Debug, Clone, thiserror::Error)]
pub enum UrlGateError {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("URL exceeds maximum length of {max} characters")]
    TooLong { max: usize },

    #[error("unsupported URL scheme: {scheme} (only http/https allowed)")]
    NotHttp { scheme: String },

    #[error("URLs with embedded credentials are not allowed")]
    CredentialsInUrl,

    #[error("hostname is blocked by policy: {host}")]
    BlockedHost { host: String },

    #[error("host resolves to a private/internal/metadata address: {host}")]
    PrivateAddress { host: String },

    #[error("DNS resolution failed for {host} (fail-closed)")]
    DnsFailure { host: String },
}

impl UrlGateError {
    /// Stable tool-error code surfaced on `ToolCompleted{status:"error"}`.
    pub fn tool_error_code(&self) -> &'static str {
        match self {
            UrlGateError::InvalidUrl(_) => "browser_read_invalid_url",
            UrlGateError::TooLong { .. } => "browser_read_url_too_long",
            UrlGateError::NotHttp { .. } => "browser_read_blocked_scheme",
            UrlGateError::CredentialsInUrl => "browser_read_credentials_in_url",
            UrlGateError::BlockedHost { .. } => "browser_read_blocked_host",
            UrlGateError::PrivateAddress { .. } => "browser_read_private_address",
            UrlGateError::DnsFailure { .. } => "browser_read_dns_failure",
        }
    }
}

/// Pure URL-shape check — no DNS, no network. Suitable for the pre-navigation
/// check and for every `frameNavigated` re-check (redirects).
pub fn check_navigation_url_sync(url: &Url) -> Result<(), UrlGateError> {
    if url.as_str().len() > MAX_URL_LEN {
        return Err(UrlGateError::TooLong { max: MAX_URL_LEN });
    }
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(UrlGateError::NotHttp {
                scheme: scheme.to_string(),
            });
        }
    }
    // Embedded credentials must never reach the browser profile (they would
    // be visible in navigation history of the shared headless profile).
    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlGateError::CredentialsInUrl);
    }
    let Some(host) = url.host_str() else {
        return Err(UrlGateError::InvalidUrl(url::ParseError::EmptyHost));
    };
    // Bare host form for IP parsing: strip IPv6 brackets and any zone id
    // (`[fe80::1%lo0]` → `fe80::1`).
    let host_bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host)
        .split('%')
        .next()
        .unwrap_or(host);
    let host_lc = host_bare.trim_end_matches('.').to_ascii_lowercase();
    if BLOCKED_HOST_NAMES.contains(&host_lc.as_str()) {
        return Err(UrlGateError::BlockedHost {
            host: host.to_string(),
        });
    }
    // IP literals: synchronous policy check (no DNS involved). Mapped IPv6
    // forms are handled by `is_blocked_for_host` (::ffff:127.0.0.1 etc.).
    if let Ok(ip) = host_bare.parse::<IpAddr>() {
        if ssrf::is_blocked_for_host(ip, host_bare, /* allow_local = */ false) {
            return Err(UrlGateError::PrivateAddress {
                host: host.to_string(),
            });
        }
        return Ok(());
    }
    // Single-label hostnames are rejected fail-closed (defense in depth; a
    // corporate intranet resolves in DNS, so the resolver path cannot be
    // trusted to classify them).
    if host_lc.split('.').count() < 2 {
        return Err(UrlGateError::BlockedHost {
            host: host.to_string(),
        });
    }
    Ok(())
}

/// Full navigation check: sync shape gate + DNS resolution with the shared
/// SSRF policy (fail-closed: DNS failure blocks the URL).
pub async fn check_navigation_url(raw: &str) -> Result<(), UrlGateError> {
    let url = Url::parse(raw)?;
    check_navigation_url_sync(&url)?;
    match ssrf::check_ssrf(&url, /* allow_local = */ false).await {
        Ok(()) => Ok(()),
        Err(WebFetchError::SsrfBlocked { host, .. }) => Err(UrlGateError::PrivateAddress { host }),
        Err(WebFetchError::DnsResolution { host, .. }) | Err(WebFetchError::DnsEmpty(host)) => {
            Err(UrlGateError::DnsFailure { host })
        }
        // Unreachable for URLs that passed the sync layer (the sync layer
        // rejects the remaining WebFetchError cases first); fail-closed
        // regardless.
        Err(other) => Err(UrlGateError::DnsFailure {
            host: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_public_https_and_http() {
        for raw in [
            "https://example.com/",
            "http://example.com/page?q=1",
            "https://sub.example.com/a/b",
        ] {
            let url = Url::parse(raw).unwrap();
            assert!(check_navigation_url_sync(&url).is_ok(), "{raw}");
        }
    }

    #[test]
    fn rejects_non_http_schemes() {
        for (raw, scheme) in [
            ("file:///etc/passwd", "file"),
            ("chrome://settings", "chrome"),
            ("edge://settings", "edge"),
            ("about:blank", "about"),
            ("data:text/html,<b>hi</b>", "data"),
            ("javascript:alert(1)", "javascript"),
            ("chrome-extension://abc/", "chrome-extension"),
        ] {
            let url = Url::parse(raw).unwrap();
            match check_navigation_url_sync(&url) {
                Err(UrlGateError::NotHttp { scheme: got }) => assert_eq!(got, scheme, "{raw}"),
                other => panic!("{raw}: expected NotHttp, got {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_overlong_urls() {
        let long = format!("https://example.com/{}", "x".repeat(MAX_URL_LEN + 1));
        let url = Url::parse(&long).unwrap();
        assert!(matches!(
            check_navigation_url_sync(&url),
            Err(UrlGateError::TooLong { .. })
        ));
    }

    #[test]
    fn rejects_embedded_credentials() {
        for raw in [
            "https://user@example.com/",
            "https://user:pass@example.com/",
        ] {
            let url = Url::parse(raw).unwrap();
            assert!(
                matches!(
                    check_navigation_url_sync(&url),
                    Err(UrlGateError::CredentialsInUrl)
                ),
                "{raw}"
            );
        }
    }

    #[test]
    fn rejects_blocked_hostnames_fast_path() {
        // Trailing-dot forms are normalized before the blocklist lookup.
        for raw in [
            "http://localhost/",
            "http://localhost./",
            "http://metadata/",
            "http://metadata.google.internal/",
            "http://metadata.internal/",
            "https://METADATA.GOOGLE.INTERNAL/",
        ] {
            let url = Url::parse(raw).unwrap();
            assert!(
                matches!(
                    check_navigation_url_sync(&url),
                    Err(UrlGateError::BlockedHost { .. })
                ),
                "{raw}"
            );
        }
    }

    #[test]
    fn rejects_single_label_hostnames() {
        let url = Url::parse("http://intranet/").unwrap();
        assert!(matches!(
            check_navigation_url_sync(&url),
            Err(UrlGateError::BlockedHost { .. })
        ));
    }

    #[test]
    fn rejects_private_ip_literals_synchronously() {
        for raw in [
            "http://127.0.0.1:8080/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://100.64.0.1/",
            "http://[::1]/",
            "http://[::ffff:127.0.0.1]/",
        ] {
            let url = Url::parse(raw).unwrap();
            assert!(
                matches!(
                    check_navigation_url_sync(&url),
                    Err(UrlGateError::PrivateAddress { .. })
                ),
                "{raw}"
            );
        }
    }

    #[test]
    fn allows_public_ip_literals() {
        for raw in ["https://1.1.1.1/", "https://8.8.8.8/"] {
            let url = Url::parse(raw).unwrap();
            assert!(check_navigation_url_sync(&url).is_ok(), "{raw}");
        }
    }

    #[test]
    fn full_check_rejects_private_literals_without_dns() {
        // The async wrapper fast-fails on IP literals — no DNS involved, so
        // these run in any test environment.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            for raw in [
                "http://127.0.0.1/",
                "http://169.254.169.254/latest/meta-data/",
                "file:///etc/passwd",
            ] {
                assert!(check_navigation_url(raw).await.is_err(), "{raw}");
            }
        });
    }

    #[test]
    fn tool_error_codes_are_stable() {
        let cases = [
            (
                UrlGateError::InvalidUrl(url::ParseError::EmptyHost),
                "browser_read_invalid_url",
            ),
            (
                UrlGateError::TooLong { max: 1 },
                "browser_read_url_too_long",
            ),
            (
                UrlGateError::NotHttp {
                    scheme: "file".into(),
                },
                "browser_read_blocked_scheme",
            ),
            (
                UrlGateError::CredentialsInUrl,
                "browser_read_credentials_in_url",
            ),
            (
                UrlGateError::BlockedHost { host: "x".into() },
                "browser_read_blocked_host",
            ),
            (
                UrlGateError::PrivateAddress { host: "x".into() },
                "browser_read_private_address",
            ),
            (
                UrlGateError::DnsFailure { host: "x".into() },
                "browser_read_dns_failure",
            ),
        ];
        for (err, code) in cases {
            assert_eq!(err.tool_error_code(), code);
        }
    }
}
