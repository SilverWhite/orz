//! Source quality weighting — FUS-SOURCE-WEIGHTING / ADR-0010 §3.7 条 12
//! (GAP-SOURCE-WEIGHTING-IMPL, 2026-08-13).
//!
//! The MECHANICAL first-layer source tier judge shared by both retrieval
//! modes (`framework_fallback` web_search/web_fetch and `local_browser`
//! browser_read) — one classifier, no per-mode forks (design §4). It maps a
//! web URL/ref to a relative sorting multiplier:
//!
//! - `authoritative` 1.1 — government / agency whitelist (direct adoption);
//! - `default` 1.0 — everything outside the whitelist and low-quality lists;
//! - `low_quality` 0.7 — seeded platform/media/URL-form rules + account-level
//!   personal-homepage markers (model layer handles account-level rules that
//!   the URL cannot express; v0 annotate-and-rank, no interception).
//!
//! The seed lists live in `runtime/source-quality-seed-lists-v0.1.json`
//! (machine-readable, NOT hardcoded in Rust); `ORZ_SOURCE_WEIGHTING_CONFIG`
//! may point at an external JSON file to add/remove entries at runtime.
//! This module owns no authorization semantics: weighting is a retrieval
//! quality layer and never bypasses ACAF tickets or permission gates.

use std::path::Path;

use serde::{Deserialize, Serialize};
use url::Url;

/// Fixed relative multipliers (relative ranking, not 0-1 confidence —
/// design §3: >1 allowed; the event schema constrains these exact values).
pub const WEIGHT_AUTHORITATIVE: f64 = 1.1;
pub const WEIGHT_DEFAULT: f64 = 1.0;
pub const WEIGHT_LOW_QUALITY: f64 = 0.7;

/// The mechanical source tier label (event schema enum — never free text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTier {
    Authoritative,
    Default,
    LowQuality,
}

impl SourceTier {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceTier::Authoritative => "authoritative",
            SourceTier::Default => "default",
            SourceTier::LowQuality => "low_quality",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            SourceTier::Authoritative => WEIGHT_AUTHORITATIVE,
            SourceTier::Default => WEIGHT_DEFAULT,
            SourceTier::LowQuality => WEIGHT_LOW_QUALITY,
        }
    }
}

/// One mechanical classification result — tier + multiplier + a stable
/// machine-readable reason (written into the evidence ledger).
#[derive(Debug, Clone, PartialEq)]
pub struct SourceWeight {
    pub tier: SourceTier,
    pub weight: f64,
    /// Stable reason code, e.g. `whitelist_suffix:gov.cn` or
    /// `low_quality_platform:csdn.net`.
    pub reason: String,
}

/// Whitelist — government / agency sources (1.1, direct adoption).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Whitelist {
    /// Domain suffixes, e.g. `gov.cn`, `gov.hk`, `edu.cn`, `mil`.
    #[serde(default)]
    pub suffixes: Vec<String>,
    /// Explicit intergovernmental / agency domains, e.g. `un.org`,
    /// `who.int`, `cas.cn`.
    #[serde(default)]
    pub domains: Vec<String>,
}

/// Low-quality sources (0.7 — annotation/downgrade tier, never a block).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowQuality {
    /// Platform-level domains (whole site, incl. subdomains), e.g.
    /// `csdn.net`, `zhihu.com`, `weibo.com`.
    #[serde(default)]
    pub platform_domains: Vec<String>,
    /// Independent news/finance media domains (0.7 per seed list §2.2).
    #[serde(default)]
    pub media_domains: Vec<String>,
    /// Full-URL lowercase patterns, e.g. `baijiahao.baidu.com/s?id=`.
    #[serde(default)]
    pub url_patterns: Vec<String>,
    /// Path markers that turn an otherwise whitelisted host (edu.cn
    /// personal homepages) into a low-quality source, e.g. `/~`, `/%7e`.
    #[serde(default)]
    pub personal_path_markers: Vec<String>,
}

/// Machine-readable source-quality configuration. The embedded seed lists
/// are the default; an external JSON file (via
/// `ORZ_SOURCE_WEIGHTING_CONFIG`) replaces them at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceWeightConfig {
    #[serde(default)]
    pub whitelist: Whitelist,
    #[serde(default)]
    pub low_quality: LowQuality,
}

impl Default for SourceWeightConfig {
    fn default() -> Self {
        Self::embedded()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceWeightError {
    #[error("source weight config io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("source weight config parse error: {0}")]
    Json(#[from] serde_json::Error),
}

impl SourceWeightConfig {
    /// The repository seed lists (`runtime/source-quality-seed-lists-v0.1.json`)
    /// compiled into the binary as the default — the lists still live in the
    /// JSON contract, never as Rust literals. NOTE (repo boundary,
    /// 2026-08-13): the seed list file lives in the parent workspace repo's
    /// `runtime/` directory (outside this git repository) and must be
    /// committed there before a fresh checkout of `orz` alone can build; see
    /// GAP_SOURCE_WEIGHTING_IMPL_AUDIT_2026-08-13.md §7.
    /// CONTAINER MOUNT CONTRACT (ORZ-BUILD-MOUNT-001, 2026-08-17):
    /// same as `candidate_prefilter` — a container build must mount the
    /// parent repo at `/orz` with workdir `/orz/orz` so `../../../runtime`
    /// resolves to `/orz/runtime`; mounting only this submodule breaks
    /// compilation with a misleading "couldn't read ...runtime/...json".
    pub fn embedded() -> Self {
        Self::from_json_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../runtime/source-quality-seed-lists-v0.1.json"
        )))
        .expect("embedded source quality seed list must parse")
    }

    /// Parse a JSON config. Unknown fields are tolerated so the machine
    /// contract can grow additively (schema_version/description are
    /// documentation-only today).
    pub fn from_json_str(json: &str) -> Result<Self, SourceWeightError> {
        let mut cfg: Self = serde_json::from_str(json)?;
        cfg.normalize();
        Ok(cfg)
    }

    /// Load a JSON config file.
    pub fn from_json_file(path: &Path) -> Result<Self, SourceWeightError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json_str(&text)
    }

    /// Runtime override channel: `ORZ_SOURCE_WEIGHTING_CONFIG` (absolute or
    /// cwd-relative JSON path). Unset → embedded seed lists; a broken
    /// override falls back to the embedded lists with an explicit warn
    /// (quality layer — the failure is visible in tracing, never silent).
    pub fn from_env_or_default() -> Self {
        match std::env::var("ORZ_SOURCE_WEIGHTING_CONFIG") {
            Ok(path) if !path.trim().is_empty() => match Self::from_json_file(Path::new(&path)) {
                Ok(cfg) => cfg,
                Err(err) => {
                    tracing::warn!(
                        %err,
                        path = %path,
                        "ORZ_SOURCE_WEIGHTING_CONFIG failed to load — using embedded seed lists"
                    );
                    Self::embedded()
                }
            },
            _ => Self::embedded(),
        }
    }

    /// Mechanical tier judge — the ONE classifier both retrieval modes use.
    /// Order matters: low-quality URL forms and personal-homepage markers
    /// win over whitelist suffixes (e.g. `edu.cn/~user` is a personal page,
    /// not an agency site); then low-quality domains, then the whitelist,
    /// then default. Scheme-less refs are retried as `https://`.
    pub fn classify(&self, url_or_ref: &str) -> SourceWeight {
        let raw = url_or_ref.trim();
        if raw.is_empty() {
            return self.default_with("empty_source_ref");
        }
        let parsed = Url::parse(raw)
            .or_else(|_| Url::parse(&format!("https://{raw}")))
            .ok();
        let Some(url) = parsed else {
            return self.default_with("url_unparseable_default");
        };
        let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
        let path = url.path().to_ascii_lowercase();
        let full = url.as_str().to_ascii_lowercase();

        // 1. Low-quality full-URL patterns (most specific first).
        for pattern in &self.low_quality.url_patterns {
            if full.contains(pattern) {
                return self.low_quality_with(format!("low_quality_url_pattern:{pattern}"));
            }
        }
        // 2. Personal-homepage markers on whitelist-suffix academic hosts
        //    (seed §2.3: `edu.cn` personal pages are low quality).
        if self.host_matches(&host, "edu.cn") {
            for marker in &self.low_quality.personal_path_markers {
                if path.contains(marker) {
                    return self.low_quality_with("personal_homepage_on_edu_cn".to_string());
                }
            }
        }
        // 3. Low-quality domains (platform + independent media).
        for domain in &self.low_quality.platform_domains {
            if self.host_matches(&host, domain) {
                return self.low_quality_with(format!("low_quality_platform:{domain}"));
            }
        }
        for domain in &self.low_quality.media_domains {
            if self.host_matches(&host, domain) {
                return self.low_quality_with(format!("low_quality_media:{domain}"));
            }
        }
        // 4. Whitelist — government/agency suffixes then explicit domains.
        for suffix in &self.whitelist.suffixes {
            if self.host_matches(&host, suffix) {
                return SourceWeight {
                    tier: SourceTier::Authoritative,
                    weight: WEIGHT_AUTHORITATIVE,
                    reason: format!("whitelist_suffix:{suffix}"),
                };
            }
        }
        for domain in &self.whitelist.domains {
            if self.host_matches(&host, domain) {
                return SourceWeight {
                    tier: SourceTier::Authoritative,
                    weight: WEIGHT_AUTHORITATIVE,
                    reason: format!("whitelist_domain:{domain}"),
                };
            }
        }
        self.default_with("default")
    }

    fn host_matches(&self, host: &str, entry: &str) -> bool {
        let entry = entry.trim().trim_start_matches('*').trim_start_matches('.');
        host == entry || host.ends_with(&format!(".{entry}"))
    }

    fn low_quality_with(&self, reason: String) -> SourceWeight {
        SourceWeight {
            tier: SourceTier::LowQuality,
            weight: WEIGHT_LOW_QUALITY,
            reason,
        }
    }

    fn default_with(&self, reason: &str) -> SourceWeight {
        SourceWeight {
            tier: SourceTier::Default,
            weight: WEIGHT_DEFAULT,
            reason: reason.to_string(),
        }
    }

    /// Lowercase + dedupe entries and trim `*.` prefixes so config authors
    /// may write either `gov.cn` or `*.gov.cn`.
    fn normalize(&mut self) {
        for list in [
            &mut self.whitelist.suffixes,
            &mut self.whitelist.domains,
            &mut self.low_quality.platform_domains,
            &mut self.low_quality.media_domains,
            &mut self.low_quality.url_patterns,
            &mut self.low_quality.personal_path_markers,
        ] {
            let mut seen = std::collections::BTreeSet::new();
            let mut normalized = Vec::with_capacity(list.len());
            for entry in list.drain(..) {
                let entry = entry.trim().to_ascii_lowercase();
                if seen.insert(entry.clone()) {
                    normalized.push(entry);
                }
            }
            *list = normalized;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cfg() -> SourceWeightConfig {
        SourceWeightConfig::embedded()
    }

    fn tier_of(cfg: &SourceWeightConfig, url: &str) -> SourceTier {
        cfg.classify(url).tier
    }

    #[test]
    fn whitelist_suffixes_are_authoritative() {
        let cfg = default_cfg();
        for url in [
            "https://www.gov.cn/policy/1",
            "https://stats.gov.hk/",
            "https://www.gov.mo/",
            "https://www.gov.tw/",
            "https://usa.gov/",
            "https://www.example.edu.cn/",
            "https://www.cas.cn/",
            "https://www.mod.mil.cn/",
            "https://army.mil/",
        ] {
            assert_eq!(tier_of(&cfg, url), SourceTier::Authoritative, "{url}");
            assert_eq!(cfg.classify(url).weight, WEIGHT_AUTHORITATIVE);
            assert!(cfg.classify(url).reason.starts_with("whitelist_"));
        }
    }

    #[test]
    fn whitelist_explicit_domains_match_with_subdomains() {
        let cfg = default_cfg();
        for url in [
            "https://un.org/",
            "https://news.un.org/",
            "https://www.who.int/",
            "https://imf.org/",
            "https://www.worldbank.org/",
            "https://oecd.org/",
            "https://www.wto.org/",
            "https://nato.int/",
            "https://europa.eu/",
            "https://www.cass.cn/",
            "https://www.cae.cn/",
        ] {
            assert_eq!(tier_of(&cfg, url), SourceTier::Authoritative, "{url}");
        }
    }

    #[test]
    fn platform_domains_are_low_quality() {
        let cfg = default_cfg();
        for url in [
            "https://blog.csdn.net/foo",
            "https://zhuanlan.zhihu.com/p/1",
            "https://mp.weixin.qq.com/s/x",
            "https://weibo.com/u/123",
            "https://www.toutiao.com/article/1",
            "https://www.jianshu.com/p/1",
            "https://dy.163.com/article/1",
            "https://mp.sohu.com/a/1",
            "https://om.qq.com/article/1",
            "https://mp.dayu.com/article/1",
            "https://user.github.io/",
            "https://b23.tv/abc",
        ] {
            assert_eq!(tier_of(&cfg, url), SourceTier::LowQuality, "{url}");
            assert_eq!(cfg.classify(url).weight, WEIGHT_LOW_QUALITY);
            assert!(cfg.classify(url).reason.starts_with("low_quality_"));
        }
    }

    #[test]
    fn independent_media_domains_are_low_quality() {
        let cfg = default_cfg();
        for url in [
            "https://36kr.com/p/1",
            "https://www.huxiu.com/article/1",
            "https://news.sina.com.cn/",
            "https://finance.sina.com.cn/",
            "https://news.163.com/",
            "https://money.163.com/",
            "https://news.qq.com/",
            "https://finance.qq.com/",
            "https://news.sohu.com/",
            "https://www.eastmoney.com/",
            "https://xueqiu.com/",
        ] {
            assert_eq!(tier_of(&cfg, url), SourceTier::LowQuality, "{url}");
            assert!(cfg.classify(url).reason.starts_with("low_quality_media:"));
        }
    }

    #[test]
    fn url_patterns_are_low_quality() {
        let cfg = default_cfg();
        for url in [
            "https://baijiahao.baidu.com/s?id=123",
            "https://www.bilibili.com/read/cv123",
            "https://www.bilibili.com/opus/456",
        ] {
            assert_eq!(tier_of(&cfg, url), SourceTier::LowQuality, "{url}");
            assert!(
                cfg.classify(url)
                    .reason
                    .starts_with("low_quality_url_pattern:")
            );
        }
    }

    #[test]
    fn edu_cn_personal_homepages_are_low_quality() {
        let cfg = default_cfg();
        assert_eq!(
            tier_of(&cfg, "https://www.example.edu.cn/~alice/"),
            SourceTier::LowQuality
        );
        assert_eq!(
            tier_of(&cfg, "https://home.example.edu.cn/%7Ealice/"),
            SourceTier::LowQuality
        );
        // Same host without a personal marker stays authoritative.
        assert_eq!(
            tier_of(&cfg, "https://www.example.edu.cn/faculty/"),
            SourceTier::Authoritative
        );
    }

    #[test]
    fn default_tier_for_unknown_sources() {
        let cfg = default_cfg();
        for url in [
            "https://example.com/",
            "https://doc.rust-lang.org/reference",
            "https://www.sina.com.cn/", // only news/finance subdomains are seeded
            "not a url at all",
            "",
        ] {
            assert_eq!(tier_of(&cfg, url), SourceTier::Default, "{url:?}");
            assert_eq!(cfg.classify(url).weight, WEIGHT_DEFAULT);
        }
    }

    #[test]
    fn scheme_less_refs_are_retried_as_https() {
        let cfg = default_cfg();
        assert_eq!(
            tier_of(&cfg, "www.gov.cn/policy"),
            SourceTier::Authoritative
        );
        assert_eq!(tier_of(&cfg, "blog.csdn.net/foo"), SourceTier::LowQuality);
    }

    #[test]
    fn custom_config_overrides_lists() {
        let cfg = SourceWeightConfig::from_json_str(
            r#"{
                "whitelist": { "suffixes": ["gov.cn"], "domains": ["example.org"] },
                "low_quality": { "platform_domains": ["bad.example"] }
            }"#,
        )
        .unwrap();
        assert_eq!(
            tier_of(&cfg, "https://x.gov.cn/"),
            SourceTier::Authoritative
        );
        assert_eq!(
            tier_of(&cfg, "https://www.example.org/"),
            SourceTier::Authoritative
        );
        assert_eq!(
            tier_of(&cfg, "https://bad.example/"),
            SourceTier::LowQuality
        );
        // Unseeded list from the embedded default is gone.
        assert_eq!(tier_of(&cfg, "https://zhihu.com/"), SourceTier::Default);
    }

    #[test]
    fn config_file_loads_and_env_override_is_honored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source-quality.json");
        std::fs::write(
            &path,
            r#"{
                "whitelist": { "suffixes": ["example.gov"], "domains": [] },
                "low_quality": {}
            }"#,
        )
        .unwrap();
        let cfg = SourceWeightConfig::from_json_file(&path).unwrap();
        assert_eq!(
            tier_of(&cfg, "https://x.example.gov/"),
            SourceTier::Authoritative
        );

        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var(
                "ORZ_SOURCE_WEIGHTING_CONFIG",
                path.to_string_lossy().to_string(),
            );
        }
        let from_env = SourceWeightConfig::from_env_or_default();
        assert_eq!(
            tier_of(&from_env, "https://x.example.gov/"),
            SourceTier::Authoritative
        );
        unsafe {
            std::env::remove_var("ORZ_SOURCE_WEIGHTING_CONFIG");
        }
        assert_eq!(
            tier_of(
                &SourceWeightConfig::from_env_or_default(),
                "https://x.gov.cn/"
            ),
            SourceTier::Authoritative
        );
    }

    #[test]
    fn embedded_config_matches_seed_document() {
        let cfg = default_cfg();
        assert!(cfg.whitelist.suffixes.contains(&"gov.cn".to_string()));
        assert!(cfg.whitelist.domains.contains(&"un.org".to_string()));
        assert!(
            cfg.low_quality
                .platform_domains
                .contains(&"csdn.net".to_string())
        );
        assert!(
            cfg.low_quality
                .media_domains
                .contains(&"36kr.com".to_string())
        );
        assert!(
            cfg.low_quality
                .url_patterns
                .contains(&"baijiahao.baidu.com/s?id=".to_string())
        );
    }
}
