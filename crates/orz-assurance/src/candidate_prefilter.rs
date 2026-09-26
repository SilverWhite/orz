//! Mechanical candidate prefilter — FUS-RETRIEVAL-MECH P0-B step 3
//! (2026-08-14) / ADR-0010 §3.7 条 12 第二层.
//!
//! The prefilter turns the raw web_search citation pool into a PURIFIED,
//! SORTED candidate pool for the sub-agent's initial selection:
//!
//! - canonical URL / host-level dedup (first-seen order);
//! - known failure forms removed only when mechanically clear from the URL
//!   itself (`bad_url` / `login_wall` / `redirect_chain`);
//! - tier/weight from the shared [`SourceWeightConfig`] judge plus a
//!   lexical query relevance grade — sorting signals, never interception;
//! - every removal is recorded with a stable machine-readable reason
//!   (the committed result's `prefilter_log`); retained candidates carry
//!   canonical_url/tier/weight/relevance/non-fatal form reasons.
//!
//! Design §2.3 discipline: mechanical rules are conservative — only
//! clearly-bad candidates are removed; unknown stays retained. Oversized
//! pages and robots exclusion are NOT determinable from a bare URL offline,
//! so v0 leaves them retained (unknown) and registers the boundary.
//! This module owns no authorization semantics: purification is a retrieval
//! quality layer and never bypasses ACAF tickets or permission gates.

use std::path::Path;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::source_weighting::{SourceTier, SourceWeight, SourceWeightConfig};

/// Lexical relevance grade for one candidate (query × URL overlap —
/// mechanical word matching, never semantic judgment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelevanceGrade {
    Tangential,
    Partial,
    Direct,
}

impl RelevanceGrade {
    pub fn as_str(self) -> &'static str {
        match self {
            RelevanceGrade::Direct => "direct",
            RelevanceGrade::Partial => "partial",
            RelevanceGrade::Tangential => "tangential",
        }
    }
}

/// Stable removal reason code (event schema enum — never free text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalReason {
    BadUrl,
    LoginWall,
    RedirectChain,
    DuplicateCanonical,
    DuplicateHost,
}

impl RemovalReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RemovalReason::BadUrl => "bad_url",
            RemovalReason::LoginWall => "login_wall",
            RemovalReason::RedirectChain => "redirect_chain",
            RemovalReason::DuplicateCanonical => "duplicate_canonical",
            RemovalReason::DuplicateHost => "duplicate_host",
        }
    }
}

/// One retained candidate — the metadata the structured result's
/// `candidate_pool` carries (schema `candidate_pool_entry`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PrefilteredCandidate {
    pub url: String,
    pub canonical_url: String,
    pub tier: SourceTier,
    pub mechanical_weight: f64,
    pub weight_reason: String,
    pub relevance: RelevanceGrade,
    /// Non-fatal mechanical form reasons (normalization signals); empty =
    /// no known failure form flagged. Removal reasons never land here —
    /// they ride `RemovedCandidate.reason`.
    pub form_reasons: Vec<String>,
}

/// One removal — the structured result's `prefilter_log` entry source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedCandidate {
    pub url: String,
    /// `None` only when the URL is unparseable (`bad_url`) — no canonical
    /// identity exists to record.
    pub canonical_url: Option<String>,
    pub reason: RemovalReason,
}

/// The full prefilter outcome for one candidate pool.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrefilterReport {
    pub retained: Vec<PrefilteredCandidate>,
    pub removed: Vec<RemovedCandidate>,
}

/// Canonical URL identity + non-fatal normalization signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalCandidate {
    pub canonical_url: String,
    pub host: String,
    /// Path is the bare root (`/`) — the host-level dedup identity.
    pub homepage: bool,
    pub form_reasons: Vec<String>,
}

/// Machine-readable prefilter configuration. The embedded seed lists are
/// the default; an external JSON file (via `ORZ_CANDIDATE_PREFILTER_CONFIG`)
/// replaces them at runtime — same contract shape as source weighting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePrefilterConfig {
    /// Query params stripped during canonicalization (tracking/click IDs).
    #[serde(default)]
    pub tracking_query_params: Vec<String>,
    /// URL substrings (lowercase) that mark an explicit login wall.
    #[serde(default)]
    pub login_wall_url_markers: Vec<String>,
    /// Query keys that mark a redirector wrapper (`?url=`, `?next=`, ...).
    #[serde(default)]
    pub redirect_query_keys: Vec<String>,
    /// Full-URL lowercase patterns for known redirector/shortener services.
    #[serde(default)]
    pub redirect_url_patterns: Vec<String>,
}

impl Default for CandidatePrefilterConfig {
    fn default() -> Self {
        Self::embedded()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CandidatePrefilterError {
    #[error("candidate prefilter config io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("candidate prefilter config parse error: {0}")]
    Json(#[from] serde_json::Error),
}

impl CandidatePrefilterConfig {
    /// The repository seed lists (`runtime/candidate-prefilter-config-v0.1.json`)
    /// compiled into the binary as the default (repo boundary note mirrors
    /// source_weighting: the file lives in the parent workspace repo and
    /// must be committed there before a fresh `orz`-only checkout builds).
    /// CONTAINER MOUNT CONTRACT (ORZ-BUILD-MOUNT-001, 2026-08-17):
    /// `../../../runtime` resolves from `CARGO_MANIFEST_DIR` — a container
    /// build must mount the parent repo at `/orz` and run with workdir
    /// `/orz/orz` (i.e. `/orz/orz/crates/orz-assurance/../../../runtime` =
    /// `/orz/runtime`); mounting only this submodule breaks compilation.
    pub fn embedded() -> Self {
        Self::from_json_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../runtime/candidate-prefilter-config-v0.1.json"
        )))
        .expect("embedded candidate prefilter config must parse")
    }

    pub fn from_json_str(json: &str) -> Result<Self, CandidatePrefilterError> {
        let mut cfg: Self = serde_json::from_str(json)?;
        cfg.normalize();
        Ok(cfg)
    }

    pub fn from_json_file(path: &Path) -> Result<Self, CandidatePrefilterError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json_str(&text)
    }

    /// Runtime override channel: `ORZ_CANDIDATE_PREFILTER_CONFIG`.
    /// Unset → embedded seed lists; a broken override falls back to the
    /// embedded lists with an explicit warn (quality layer — the failure is
    /// visible in tracing, never silent).
    pub fn from_env_or_default() -> Self {
        match std::env::var("ORZ_CANDIDATE_PREFILTER_CONFIG") {
            Ok(path) if !path.trim().is_empty() => match Self::from_json_file(Path::new(&path)) {
                Ok(cfg) => cfg,
                Err(err) => {
                    tracing::warn!(
                        %err,
                        path = %path,
                        "ORZ_CANDIDATE_PREFILTER_CONFIG failed to load — using embedded seed lists"
                    );
                    Self::embedded()
                }
            },
            _ => Self::embedded(),
        }
    }

    fn normalize(&mut self) {
        for list in [
            &mut self.tracking_query_params,
            &mut self.login_wall_url_markers,
            &mut self.redirect_query_keys,
            &mut self.redirect_url_patterns,
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

/// Canonicalize one candidate URL. Returns `None` for mechanically-bad
/// URLs (unparseable, non-http(s) scheme, empty host) — the `bad_url`
/// removal reason. Normalization: lowercase scheme/host, strip `www.`,
/// drop default ports, drop the fragment, strip tracking query params,
/// sort remaining query pairs, lowercase percent-encoded hex, normalize
/// the path to `/` for a bare root.
pub fn canonicalize(raw: &str, config: &CandidatePrefilterConfig) -> Option<CanonicalCandidate> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed = parse_http_url(raw)?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let host_raw = parsed.host_str()?;
    if host_raw.is_empty() {
        return None;
    }
    let mut form_reasons = Vec::new();
    let mut host = host_raw.to_ascii_lowercase();
    if let Some(stripped) = host.strip_prefix("www.") {
        host = stripped.to_string();
        form_reasons.push("www_normalized".to_string());
    }
    let port = parsed.port();
    let default_port = match scheme.as_str() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    };
    let port_str = match port {
        Some(p) if default_port != Some(p) => {
            form_reasons.push("non_default_port_kept".to_string());
            format!(":{p}")
        }
        // The url crate normalizes explicit default ports away at parse
        // time (port() returns None for `:443` on https), so no reason is
        // observable for the default-port case.
        _ => String::new(),
    };
    if parsed.fragment().is_some() {
        form_reasons.push("fragment_stripped".to_string());
    }
    let path = parsed.path();
    let path = if path.is_empty() {
        "/".to_string()
    } else {
        lowercase_percent_hex(path)
    };
    // Query pairs are percent-DECODED by url::Url — rebuild canonically so
    // `?a=1&b=2` and `?b=2&a=1` (and different encodings of the same pair)
    // produce one identity.
    let pairs: Vec<(String, String)> = parsed.query_pairs().into_owned().collect();
    let mut kept: Vec<(String, String)> = Vec::new();
    let mut tracking_stripped = false;
    for (key, value) in pairs {
        if config
            .tracking_query_params
            .iter()
            .any(|tracked| key.eq_ignore_ascii_case(tracked))
        {
            tracking_stripped = true;
        } else {
            kept.push((key.to_ascii_lowercase(), value));
        }
    }
    let original_order: Vec<String> = kept.iter().map(|(k, _)| k.clone()).collect();
    kept.sort();
    let sorted_order: Vec<String> = kept.iter().map(|(k, _)| k.clone()).collect();
    if tracking_stripped {
        form_reasons.push("tracking_params_stripped".to_string());
    }
    if !kept.is_empty() && original_order != sorted_order {
        form_reasons.push("query_params_sorted".to_string());
    }
    let query = if kept.is_empty() {
        String::new()
    } else {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (key, value) in &kept {
            serializer.append_pair(key, value);
        }
        format!("?{}", serializer.finish())
    };
    let homepage = path == "/";
    Some(CanonicalCandidate {
        canonical_url: format!("{scheme}://{host}{port_str}{path}{query}"),
        host,
        homepage,
        form_reasons,
    })
}

fn lowercase_percent_hex(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            out.push('%');
            out.push(bytes[i + 1].to_ascii_lowercase() as char);
            out.push(bytes[i + 2].to_ascii_lowercase() as char);
            i += 3;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Parse a candidate reference as an http(s) URL, tolerating the two
/// scheme-less citation shapes: `host/path` (no scheme — retry with
/// `https://`) and `host:port/path` (the url crate mis-reads the hostname
/// as a scheme when it contains dots — retry the whole raw string).
fn parse_http_url(raw: &str) -> Option<Url> {
    let direct = Url::parse(raw).ok();
    let direct = match direct {
        Some(url)
            if url.scheme() != "http" && url.scheme() != "https" && url.scheme().contains('.') =>
        {
            Url::parse(&format!("https://{raw}")).ok()
        }
        other => other,
    };
    direct.or_else(|| Url::parse(&format!("https://{raw}")).ok())
}

/// Detect a mechanically-clear failure form from the URL alone. Returns
/// the first (most specific) reason; `None` = no known failure form
/// (unknown stays retained — design §2.3).
pub fn detect_failure(raw: &str, config: &CandidatePrefilterConfig) -> Option<RemovalReason> {
    let raw = raw.trim();
    if raw.is_empty() || canonicalize(raw, config).is_none() {
        return Some(RemovalReason::BadUrl);
    }
    let lowered = raw.to_ascii_lowercase();
    if config
        .login_wall_url_markers
        .iter()
        .any(|marker| lowered.contains(marker))
    {
        return Some(RemovalReason::LoginWall);
    }
    if let Some(url) = parse_http_url(raw) {
        // Host-aware redirector matching — a pattern like `t.co/` must not
        // match `not-t.co/`: exact host (or subdomain) plus path/query
        // prefix only.
        if config
            .redirect_url_patterns
            .iter()
            .any(|pattern| matches_redirect_pattern(&url, pattern))
        {
            return Some(RemovalReason::RedirectChain);
        }
        for (key, _) in url.query_pairs() {
            if config
                .redirect_query_keys
                .iter()
                .any(|redirect_key| key.eq_ignore_ascii_case(redirect_key))
            {
                return Some(RemovalReason::RedirectChain);
            }
        }
    }
    None
}

/// Host-boundary-aware redirector/shortener pattern match. A pattern is
/// `host[/rest]`: the host must match exactly or as a subdomain, and the
/// path+query must start with the (lowercased) rest.
fn matches_redirect_pattern(url: &Url, pattern: &str) -> bool {
    let (pattern_host, pattern_rest) = match pattern.split_once('/') {
        Some((host, rest)) => (host, format!("/{rest}")),
        None => (pattern, String::new()),
    };
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if host != pattern_host && !host.ends_with(&format!(".{pattern_host}")) {
        return false;
    }
    if pattern_rest.is_empty() {
        return true;
    }
    let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
    let path_and_query = format!("{}{}", url.path().to_ascii_lowercase(), query);
    path_and_query.starts_with(&pattern_rest)
}

/// The mechanical prefilter: purify + sort, never intercept. Preserves
/// first-seen order across equal sort keys.
pub fn prefilter(
    urls: &[String],
    query: &str,
    weight_config: &SourceWeightConfig,
    prefilter_config: &CandidatePrefilterConfig,
) -> PrefilterReport {
    let mut retained: Vec<(usize, PrefilteredCandidate)> = Vec::new();
    let mut removed: Vec<RemovedCandidate> = Vec::new();
    let mut seen_canonical: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_homepage_hosts: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for (index, raw) in urls.iter().enumerate() {
        if let Some(reason) = detect_failure(raw, prefilter_config) {
            let canonical_url = canonicalize(raw, prefilter_config).map(|c| c.canonical_url);
            removed.push(RemovedCandidate {
                url: raw.clone(),
                canonical_url,
                reason,
            });
            continue;
        }
        let Some(canonical) = canonicalize(raw, prefilter_config) else {
            // Defensive — detect_failure covers this; keep the audit trail.
            removed.push(RemovedCandidate {
                url: raw.clone(),
                canonical_url: None,
                reason: RemovalReason::BadUrl,
            });
            continue;
        };
        if !seen_canonical.insert(canonical.canonical_url.clone()) {
            removed.push(RemovedCandidate {
                url: raw.clone(),
                canonical_url: Some(canonical.canonical_url),
                reason: RemovalReason::DuplicateCanonical,
            });
            continue;
        }
        if canonical.homepage && !seen_homepage_hosts.insert(canonical.host.clone()) {
            removed.push(RemovedCandidate {
                url: raw.clone(),
                canonical_url: Some(canonical.canonical_url),
                reason: RemovalReason::DuplicateHost,
            });
            continue;
        }
        let weighted: SourceWeight = weight_config.classify(raw);
        retained.push((
            index,
            PrefilteredCandidate {
                url: raw.clone(),
                canonical_url: canonical.canonical_url,
                tier: weighted.tier,
                mechanical_weight: weighted.weight,
                weight_reason: weighted.reason,
                relevance: compute_relevance(query, raw),
                form_reasons: canonical.form_reasons,
            },
        ));
    }

    retained.sort_by(|(left_index, left), (right_index, right)| {
        right
            .mechanical_weight
            .partial_cmp(&left.mechanical_weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.relevance.cmp(&left.relevance))
            .then_with(|| left_index.cmp(right_index))
    });

    PrefilterReport {
        retained: retained
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect(),
        removed,
    }
}

/// Lexical query × URL overlap — ASCII word tokens (len >= 2) plus CJK
/// bigrams; the haystack is the percent-decoded, lowercased URL so encoded
/// CJK paths can match. No keywords → neutral `partial` (unknown retained);
/// overlap 0 → `tangential`; 1 → `partial`; >= 2 → `direct`.
fn compute_relevance(query: &str, url: &str) -> RelevanceGrade {
    let keywords = query_keywords(query);
    if keywords.is_empty() {
        return RelevanceGrade::Partial;
    }
    let haystack = urlencoding::decode(url)
        .map(|decoded| decoded.to_ascii_lowercase())
        .unwrap_or_else(|_| url.to_ascii_lowercase());
    let overlap = keywords
        .iter()
        .filter(|keyword| haystack.contains(keyword.as_str()))
        .count();
    match overlap {
        0 => RelevanceGrade::Tangential,
        1 => RelevanceGrade::Partial,
        _ => RelevanceGrade::Direct,
    }
}

fn query_keywords(query: &str) -> Vec<String> {
    let mut keywords: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for token in query.split(|c: char| !c.is_alphanumeric()) {
        let token = token.to_ascii_lowercase();
        if token.len() >= 2 && seen.insert(token.clone()) {
            keywords.push(token);
        }
    }
    let chars: Vec<char> = query.chars().collect();
    for pair in chars.windows(2) {
        if is_cjk(pair[0]) && is_cjk(pair[1]) {
            let bigram: String = pair.iter().collect();
            if seen.insert(bigram.clone()) {
                keywords.push(bigram);
            }
        }
    }
    keywords
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xF900..=0xFAFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cfg() -> CandidatePrefilterConfig {
        CandidatePrefilterConfig::embedded()
    }

    fn weight_cfg() -> SourceWeightConfig {
        SourceWeightConfig::embedded()
    }

    #[test]
    fn canonicalize_strips_tracking_defaults_and_fragments() {
        let cfg = default_cfg();
        let canonical = canonicalize(
            "HTTPS://WWW.Example.COM:443/path/?utm_source=x&b=2&a=1#frag",
            &cfg,
        )
        .unwrap();
        assert_eq!(canonical.canonical_url, "https://example.com/path/?a=1&b=2");
        assert_eq!(canonical.host, "example.com");
        assert!(!canonical.homepage);
        assert!(
            canonical
                .form_reasons
                .contains(&"www_normalized".to_string())
        );
        assert!(
            canonical
                .form_reasons
                .contains(&"tracking_params_stripped".to_string())
        );
        assert!(
            canonical
                .form_reasons
                .contains(&"fragment_stripped".to_string())
        );
        assert!(
            canonical
                .form_reasons
                .contains(&"query_params_sorted".to_string())
        );
    }

    #[test]
    fn canonicalize_handles_root_and_percent_hex() {
        let cfg = default_cfg();
        let root = canonicalize("https://www.example.com", &cfg).unwrap();
        assert_eq!(root.canonical_url, "https://example.com/");
        assert!(root.homepage);
        let hex = canonicalize("https://example.com/a%2Fb", &cfg).unwrap();
        assert_eq!(hex.canonical_url, "https://example.com/a%2fb");
    }

    #[test]
    fn scheme_less_refs_with_port_are_accepted() {
        let cfg = default_cfg();
        // `example.com:8080/path` parses as a dotted "scheme" — the parse
        // fallback must retry the whole string as https.
        let canonical = canonicalize("example.com:8080/path", &cfg).unwrap();
        assert_eq!(canonical.canonical_url, "https://example.com:8080/path");
        assert!(
            canonical
                .form_reasons
                .contains(&"non_default_port_kept".to_string())
        );
        assert_eq!(detect_failure("example.com:8080/path", &cfg), None);
    }

    #[test]
    fn canonicalize_rejects_bad_urls() {
        let cfg = default_cfg();
        for bad in [
            "",
            "not a url at all",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "mailto:a@example.com",
            "ftp://example.com/x",
        ] {
            assert_eq!(canonicalize(bad, &cfg), None, "{bad:?}");
        }
        assert_eq!(
            detect_failure("javascript:alert(1)", &cfg),
            Some(RemovalReason::BadUrl)
        );
    }

    #[test]
    fn dedup_canonical_and_host_homepages_keep_different_pages() {
        let cfg = default_cfg();
        let report = prefilter(
            &[
                "https://example.com/page?a=1".to_string(),
                "https://example.com/page?utm_source=x&a=1".to_string(),
                "https://example.com/".to_string(),
                "https://example.com/?x=1".to_string(),
                "https://example.com/other".to_string(),
            ],
            "example page",
            &weight_cfg(),
            &cfg,
        );
        assert_eq!(report.removed.len(), 2);
        assert_eq!(report.removed[0].reason, RemovalReason::DuplicateCanonical);
        assert_eq!(report.removed[1].reason, RemovalReason::DuplicateHost);
        let retained: Vec<&str> = report.retained.iter().map(|c| c.url.as_str()).collect();
        assert_eq!(
            retained,
            vec![
                "https://example.com/page?a=1",
                "https://example.com/",
                "https://example.com/other"
            ]
        );
    }

    #[test]
    fn known_failure_forms_are_removed_with_reasons() {
        let cfg = default_cfg();
        let report = prefilter(
            &[
                "https://example.com/article".to_string(),
                "https://example.com/login?next=/article".to_string(),
                "https://www.google.com/url?q=https://example.com".to_string(),
                "https://example.com/?redirect_url=https://other.example".to_string(),
            ],
            "example article",
            &weight_cfg(),
            &cfg,
        );
        assert_eq!(report.removed.len(), 3);
        assert_eq!(report.removed[0].reason, RemovalReason::LoginWall);
        assert_eq!(report.removed[1].reason, RemovalReason::RedirectChain);
        assert_eq!(report.removed[2].reason, RemovalReason::RedirectChain);
        assert_eq!(report.retained.len(), 1);
        assert_eq!(report.retained[0].url, "https://example.com/article");
    }

    #[test]
    fn redirect_pattern_matching_is_host_aware() {
        let cfg = default_cfg();
        // Host-boundary: `t.co/` must not match `not-t.co/`.
        assert_eq!(detect_failure("https://not-t.co/abc", &cfg), None);
        assert_eq!(
            detect_failure("https://t.co/abc", &cfg),
            Some(RemovalReason::RedirectChain)
        );
        assert_eq!(
            detect_failure("https://www.bing.com/ck/a?x=1", &cfg),
            Some(RemovalReason::RedirectChain)
        );
        // Same host, unrelated path — retained.
        assert_eq!(detect_failure("https://www.bing.com/docs", &cfg), None);
    }

    #[test]
    fn relevance_grades_query_url_overlap() {
        assert_eq!(
            compute_relevance("rust channels", "https://doc.rust-lang.org/book/channels"),
            RelevanceGrade::Direct
        );
        assert_eq!(
            compute_relevance("rust channels", "https://example.com/other"),
            RelevanceGrade::Tangential
        );
        assert_eq!(
            compute_relevance("", "https://example.com/other"),
            RelevanceGrade::Partial
        );
        // Percent-encoded CJK path matches the decoded query bigram.
        assert_eq!(
            compute_relevance(
                "融合架构",
                "https://example.com/%E8%9E%8D%E5%90%88%E6%9E%B6%E6%9E%84"
            ),
            RelevanceGrade::Direct
        );
    }

    #[test]
    fn sorting_puts_weight_and_relevance_first() {
        let cfg = default_cfg();
        let report = prefilter(
            &[
                "https://example.com/low-quality-unrelated".to_string(),
                "https://www.gov.cn/policy/rust".to_string(),
                "https://example.com/rust-guide".to_string(),
            ],
            "rust policy",
            &weight_cfg(),
            &cfg,
        );
        assert_eq!(report.retained.len(), 3);
        // Authoritative gov.cn with direct relevance first; then the
        // default tier direct match; then the low-overlap default tier.
        assert_eq!(report.retained[0].tier, SourceTier::Authoritative);
        assert_eq!(report.retained[0].mechanical_weight, 1.1);
        assert_eq!(report.retained[1].url, "https://example.com/rust-guide");
        assert_eq!(
            report.retained[2].url,
            "https://example.com/low-quality-unrelated"
        );
        assert_eq!(report.retained[2].relevance, RelevanceGrade::Tangential);
    }

    #[test]
    fn custom_config_overrides_lists() {
        let cfg = CandidatePrefilterConfig::from_json_str(
            r#"{
                "tracking_query_params": ["spm"],
                "login_wall_url_markers": ["/member-login"],
                "redirect_query_keys": ["jump"],
                "redirect_url_patterns": ["short.example/"]
            }"#,
        )
        .unwrap();
        assert_eq!(
            canonicalize("https://example.com/?spm=1", &cfg)
                .unwrap()
                .canonical_url,
            "https://example.com/"
        );
        assert_eq!(
            detect_failure("https://example.com/member-login", &cfg),
            Some(RemovalReason::LoginWall)
        );
        // Unseeded defaults are gone.
        assert_eq!(detect_failure("https://example.com/login", &cfg), None);
        assert_eq!(detect_failure("https://example.com/?next=/x", &cfg), None);
        assert_eq!(
            detect_failure("https://example.com/?jump=/x", &cfg),
            Some(RemovalReason::RedirectChain)
        );
    }

    #[test]
    fn config_file_loads_and_env_override_is_honored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("candidate-prefilter.json");
        std::fs::write(
            &path,
            r#"{
                "tracking_query_params": ["ref"],
                "login_wall_url_markers": [],
                "redirect_query_keys": [],
                "redirect_url_patterns": []
            }"#,
        )
        .unwrap();
        let cfg = CandidatePrefilterConfig::from_json_file(&path).unwrap();
        assert_eq!(
            canonicalize("https://example.com/?ref=1", &cfg)
                .unwrap()
                .canonical_url,
            "https://example.com/"
        );

        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var(
                "ORZ_CANDIDATE_PREFILTER_CONFIG",
                path.to_string_lossy().to_string(),
            );
        }
        let from_env = CandidatePrefilterConfig::from_env_or_default();
        assert_eq!(
            canonicalize("https://example.com/?ref=1", &from_env)
                .unwrap()
                .canonical_url,
            "https://example.com/"
        );
        unsafe {
            std::env::remove_var("ORZ_CANDIDATE_PREFILTER_CONFIG");
        }
        assert_eq!(
            canonicalize(
                "https://example.com/?utm_source=1",
                &CandidatePrefilterConfig::from_env_or_default()
            )
            .unwrap()
            .canonical_url,
            "https://example.com/"
        );
    }

    #[test]
    fn embedded_config_matches_seed_document() {
        let cfg = default_cfg();
        assert!(
            cfg.tracking_query_params
                .contains(&"utm_source".to_string())
        );
        assert!(cfg.login_wall_url_markers.contains(&"/login".to_string()));
        assert!(cfg.redirect_query_keys.contains(&"redirect".to_string()));
        assert!(!cfg.redirect_query_keys.contains(&"next".to_string()));
        assert!(
            cfg.redirect_url_patterns
                .contains(&"google.com/url?".to_string())
        );
    }
}
