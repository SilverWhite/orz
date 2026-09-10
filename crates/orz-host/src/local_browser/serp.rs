//! SERP engine chain for the `browser_control search` action
//! (P0-0v, 2026-09-10).
//!
//! This module owns the machine-side, testable pieces:
//! - fixed engine order and `en-US` search URLs;
//! - session-level failure memo and pacing/limit state;
//! - Bing `/ck/a` and DuckDuckGo `uddg` redirect decoding;
//! - low-quality domain weighting through the shared
//!   `orz_assurance::source_weighting::SourceWeightConfig` judge;
//! - bounded envelope shaping (per-field caps; see the `SERP_*_MAX_CHARS`
//!   constants — no total-payload truncation since P2-2, 2026-09-10).
//!
//! It does NOT do HTTP scraping or browser automation. The real navigation
//! and fixed-expression extraction stay in [`super::cdp`] so the search
//! path keeps the real Chromium TLS fingerprint and CDP discipline.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use base64::Engine as _;
use orz_assurance::source_weighting::SourceWeightConfig;
use serde::Serialize;
use serde_json::Value;
use url::Url;

/// Mechanical cap on the model-visible `query` string.
pub const SERP_MAX_SEARCH_QUERY_CHARS: usize = 500;

/// Maximum organic results returned to the model per successful search.
pub const SERP_MAX_RESULTS: usize = 10;

/// Maximum snippet length returned to the model.
pub const SERP_SNIPPET_MAX_CHARS: usize = 200;

/// Maximum title length returned to the model.
pub const SERP_TITLE_MAX_CHARS: usize = 200;

/// Maximum result-URL length returned to the model.
///
/// P2-2 (2026-09-10): the former total-payload byte cap is replaced by
/// per-field caps. Real SERP destinations are far below this bound, but a
/// single engine-supplied `href` is attacker-influenced text and must not be
/// able to grow the model-visible envelope without limit.
pub const SERP_URL_MAX_CHARS: usize = 2048;

/// Maximum length of a source-weight `reason` code returned to the model.
pub const SERP_REASON_MAX_CHARS: usize = 200;

/// Minimum cool-down between two searches in one browser session.
pub const SERP_SEARCH_COOLDOWN: Duration = Duration::from_secs(5);

/// Maximum SERP **engine navigations** per browser session.
///
/// P2-4 (2026-09-10): this is the physical anti-bot backstop shared by every
/// lane (one browser per process), NOT the lane budget — that lives in the
/// loop layer (`SerpSearchBudget`, per run on main / per activation on the
/// external retrieval lane) where lane identity exists. A single `auto`
/// search may navigate up to three engines, so the ceiling counts
/// navigations rather than calls.
pub const SERP_MAX_NAVIGATIONS_PER_SESSION: usize = 40;

/// Fixed engine chain (user ruling 2026-09-10: Google → Bing → DuckDuckGo).
pub const SERP_ENGINE_ORDER: [SerpEngine; 3] =
    [SerpEngine::Google, SerpEngine::Bing, SerpEngine::DuckDuckGo];

/// The three engine choices in the fixed chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SerpEngine {
    Google,
    Bing,
    DuckDuckGo,
}

impl SerpEngine {
    pub fn as_str(self) -> &'static str {
        match self {
            SerpEngine::Google => "google",
            SerpEngine::Bing => "bing",
            SerpEngine::DuckDuckGo => "duckduckgo",
        }
    }
}

impl std::fmt::Display for SerpEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A raw organic hit extracted from a fixed CDP expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerpHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// A bounded, weighted SERP result returned to the model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SerpResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub tier: &'static str,
    pub weight: f64,
    pub reason: String,
}

/// One engine's outcome inside a single `search` call (P2-1, 2026-09-10).
///
/// The failure envelope must name the engine it failed on, otherwise the
/// model cannot tell "Bing is blocked" from "the whole lane is down" and has
/// no basis for steering the next call.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SerpEngineAttempt {
    pub engine: SerpEngine,
    /// `ok` / `failed` / `skipped` (skipped = memoized earlier failure in
    /// this browser session). `pending` is internal and never emitted.
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl SerpEngineAttempt {
    pub(crate) fn pending(engine: SerpEngine) -> Self {
        Self {
            engine,
            status: "pending",
            error_class: None,
            reason: None,
        }
    }

    pub(crate) fn ok(engine: SerpEngine) -> Self {
        Self {
            engine,
            status: "ok",
            error_class: None,
            reason: None,
        }
    }

    pub(crate) fn failed(engine: SerpEngine, class: SerpFailureClass, reason: &str) -> Self {
        Self {
            engine,
            status: "failed",
            error_class: Some(class.as_str().to_string()),
            reason: Some(truncate_chars(reason, SERP_REASON_MAX_CHARS)),
        }
    }

    /// A memoized engine is not retried this session; the model still needs
    /// to see that it was skipped and why.
    pub(crate) fn skipped(engine: SerpEngine, class: SerpFailureClass) -> Self {
        Self {
            engine,
            status: "skipped",
            error_class: Some(class.as_str().to_string()),
            reason: Some("skipped: failed earlier in this browser session".to_string()),
        }
    }
}

/// Why an engine failed for this session. Only these classes are memoized:
/// network failures, CAPTCHA/consent walls, and empty organic results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SerpFailureClass {
    Network,
    Captcha,
    Empty,
    /// URL gate / policy block. Not a network failure but still memoized so
    /// a poisoned navigation target is not retried every search.
    Blocked,
}

impl SerpFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            SerpFailureClass::Network => "network",
            SerpFailureClass::Captcha => "captcha",
            SerpFailureClass::Empty => "empty",
            SerpFailureClass::Blocked => "blocked",
        }
    }
}

/// Session-scoped SERP state. Lives for the lifetime of one
/// [`super::CdpBrowserSession`] and is invisible to the model.
#[derive(Debug, Clone, Default)]
pub struct SerpSessionState {
    failures: HashMap<SerpEngine, SerpFailureClass>,
    last_search_at: Option<Instant>,
    navigations: usize,
}

impl SerpSessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn failure(&self, engine: SerpEngine) -> Option<SerpFailureClass> {
        self.failures.get(&engine).copied()
    }

    pub fn record_failure(&mut self, engine: SerpEngine, class: SerpFailureClass) {
        self.failures.insert(engine, class);
    }

    pub fn ceiling_reached(&self) -> bool {
        self.navigations >= SERP_MAX_NAVIGATIONS_PER_SESSION
    }

    /// P2-3（2026-09-10）：本会话已发生的引擎导航次数（只读事实——供宿主
    /// 上报给 loop，loop 层据此为检索车道保留底线额度）。
    pub fn navigations(&self) -> u32 {
        self.navigations as u32
    }

    /// Record one engine navigation (called right before the real
    /// `Page.navigate`); the session ceiling counts these, not searches.
    pub fn record_navigation(&mut self) {
        self.navigations = self.navigations.saturating_add(1);
    }

    /// Mark a search as starting and return the cool-down sleep the caller
    /// should perform BEFORE navigating. The first search in a session is
    /// immediate; a prior search less than [`SERP_SEARCH_COOLDOWN`] ago is
    /// padded to the minimum, then 0–50% of the full cool-down is added as
    /// jitter.
    pub fn begin_search(&mut self, query: &str, now: Instant) -> Duration {
        let wait = match self.last_search_at {
            None => Duration::ZERO,
            Some(last) => {
                let elapsed = now.saturating_duration_since(last);
                SERP_SEARCH_COOLDOWN.saturating_sub(elapsed)
                    + Duration::from_millis(jitter_millis(query, now))
            }
        };
        self.last_search_at = Some(now);
        wait
    }
}

/// The engines still eligible for this session, keeping the fixed chain
/// order after removing memoized failures.
pub fn available_engines(state: &SerpSessionState) -> Vec<SerpEngine> {
    SERP_ENGINE_ORDER
        .iter()
        .copied()
        .filter(|engine| state.failure(*engine).is_none())
        .collect()
}

/// Construct the fixed `en-US` search URL for an engine.
pub fn search_url(engine: SerpEngine, query: &str) -> String {
    let q: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    match engine {
        SerpEngine::Google => format!("https://www.google.com/search?q={q}&hl=en-US&gl=us"),
        SerpEngine::Bing => {
            format!("https://www.bing.com/search?q={q}&setmkt=en-US&count=10&first=1")
        }
        SerpEngine::DuckDuckGo => {
            format!("https://html.duckduckgo.com/html/?q={q}&kl=us-en")
        }
    }
}

/// Parse the fixed CDP extraction payload
/// `{"results":[{title,url,snippet}],"captcha":bool}` into raw hits.
pub fn parse_extraction(engine: SerpEngine, value: &Value) -> SerpExtraction {
    let captcha = value
        .get("captcha")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let hits = value
        .get("results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(parse_hit)
                .filter(|hit| !hit.url.trim().is_empty())
                .take(SERP_MAX_RESULTS)
                .collect()
        })
        .unwrap_or_default();
    // Some engines may expose extra engine-specific flags in future; keeping
    // the parse surface explicit means unknown fields are simply ignored.
    let _ = engine;
    SerpExtraction { hits, captcha }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerpExtraction {
    pub hits: Vec<SerpHit>,
    pub captcha: bool,
}

fn parse_hit(value: &Value) -> Option<SerpHit> {
    let obj = value.as_object()?;
    let title = obj
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let url = obj.get("url").and_then(Value::as_str).unwrap_or("").trim();
    let snippet = obj
        .get("snippet")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if title.is_empty() && url.is_empty() {
        return None;
    }
    Some(SerpHit {
        title: title.to_string(),
        url: url.to_string(),
        snippet: snippet.to_string(),
    })
}

/// Decode engine redirect URLs BEFORE they are handed to the model.
/// Bing `/ck/a?...&u=...` and DDG `uddg=...` query parameters are decoded
/// once; other engines pass through unchanged. When the payload cannot be
/// proven to be a destination URL the raw link is kept — never a decoded
/// blob that only *looks* like text.
pub fn decode_result_url(engine: SerpEngine, raw: &str) -> String {
    match engine {
        SerpEngine::Bing => decode_bing_redirect(raw).unwrap_or_else(|| raw.to_string()),
        SerpEngine::DuckDuckGo => decode_ddg_redirect(raw).unwrap_or_else(|| raw.to_string()),
        SerpEngine::Google => raw.to_string(),
    }
}

/// Pull one query parameter out of an absolute, relative or protocol-relative
/// URL. Engines emit all three shapes (`/ck/a?...`, `//host/l?...`), so the
/// parser is fed a synthetic base before giving up.
fn query_param(raw: &str, key: &str) -> Option<String> {
    let parsed = Url::parse(raw)
        .or_else(|_| Url::parse(&format!("https://www.bing.com{raw}")))
        .or_else(|_| Url::parse(&format!("https://duckduckgo.com{raw}")))
        .ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
        .filter(|v| !v.trim().is_empty())
}

/// Bing `/ck/a?...&u=<payload>` encodes the destination as base64 whose
/// payload carries a 2-character marker — every live organic result probed
/// on 2026-09-10 was `u=a1<base64url>`, so decoding the value as-is yields
/// either invalid UTF-8 or a shifted-garbage string (both were observed).
/// Candidates are tried in order (as-is, then marker-stripped) and only a
/// value that parses as a real destination URL is accepted.
fn decode_bing_redirect(raw: &str) -> Option<String> {
    let value = query_param(raw, "u")?;
    let mut candidates: Vec<&str> = vec![value.as_str()];
    if let Some(stripped) = value.strip_prefix("a1") {
        candidates.push(stripped);
    }
    candidates
        .into_iter()
        .find_map(|candidate| decode_candidate(candidate, "https://www.bing.com"))
}

/// DDG `/l/?uddg=<percent-encoded-absolute-url>`: `query_pairs` already
/// percent-decodes, so the value only needs destination validation.
fn decode_ddg_redirect(raw: &str) -> Option<String> {
    let value = query_param(raw, "uddg")?;
    normalize_decoded_url(&value, "https://duckduckgo.com")
}

/// Base64 (URL-safe or standard, padding optional) → destination URL.
fn decode_candidate(value: &str, base: &str) -> Option<String> {
    // Some payloads are the plain (percent-decoded) destination URL.
    if let Some(url) = normalize_decoded_url(value, base) {
        return Some(url);
    }
    decode_base64_url(value, base)
}

fn decode_base64_url(value: &str, base: &str) -> Option<String> {
    use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
    let trimmed = value.trim().trim_end_matches('=');
    if trimmed.is_empty() {
        return None;
    }
    for engine in [&URL_SAFE_NO_PAD, &STANDARD_NO_PAD] {
        if let Ok(bytes) = engine.decode(trimmed)
            && let Ok(text) = String::from_utf8(bytes)
            && let Some(url) = normalize_decoded_url(&text, base)
        {
            return Some(url);
        }
    }
    None
}

/// Accept an absolute http(s) URL, or resolve a protocol-relative / absolute
/// path target against the engine origin (Bing encodes its vertical tabs as
/// `/images/...`). Anything else is not a destination and is rejected.
fn normalize_decoded_url(text: &str, base: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(url) = Url::parse(text)
        && matches!(url.scheme(), "http" | "https")
        && url.has_host()
    {
        return Some(text.to_string());
    }
    // Protocol-relative and absolute-path payloads resolve against the
    // engine origin. (A bare "https:/path" string is NOT handled: the URL
    // parser would fold `path` into the host and invent a destination.)
    let candidate = if let Some(rest) = text.strip_prefix("//") {
        format!("https://{rest}")
    } else if text.starts_with('/') {
        format!("{base}{text}")
    } else {
        return None;
    };
    match Url::parse(&candidate) {
        Ok(url)
            if matches!(url.scheme(), "http" | "https")
                && url.has_host()
                && url.path().len() > 1 =>
        {
            Some(candidate)
        }
        _ => None,
    }
}

/// Apply the shared source-quality judge and stable tier ordering:
/// `authoritative` → `default` → `low_quality`; same tier keeps engine order.
pub fn stabilize_and_weight(
    engine: SerpEngine,
    hits: Vec<SerpHit>,
    config: &SourceWeightConfig,
) -> Vec<SerpResult> {
    let mut results: Vec<SerpResult> = hits
        .into_iter()
        .map(|hit| {
            let url = decode_result_url(engine, &hit.url);
            let judged = config.classify(&url);
            SerpResult {
                title: truncate_chars(&hit.title, SERP_TITLE_MAX_CHARS),
                url: truncate_chars(&url, SERP_URL_MAX_CHARS),
                snippet: truncate_chars(&hit.snippet, SERP_SNIPPET_MAX_CHARS),
                tier: judged.tier.as_str(),
                weight: judged.weight,
                reason: truncate_chars(&judged.reason, SERP_REASON_MAX_CHARS),
            }
        })
        .collect();
    results.sort_by_key(|r| tier_rank(r.tier));
    results
}

fn tier_rank(tier: &str) -> u8 {
    match tier {
        "authoritative" => 0,
        "low_quality" => 2,
        _ => 1,
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    if max_chars <= 1 {
        return input.chars().take(max_chars).collect();
    }
    let mut out: String = input.chars().take(max_chars - 1).collect();
    out.push('…');
    out
}

fn jitter_millis(query: &str, now: Instant) -> u64 {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    now.elapsed().as_nanos().hash(&mut hasher);
    let window = SERP_SEARCH_COOLDOWN.as_millis() as u64 / 2;
    hasher.finish() % (window + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn search_urls_are_fixed_en_us() {
        assert_eq!(
            search_url(SerpEngine::Google, "rust borrow checker"),
            "https://www.google.com/search?q=rust+borrow+checker&hl=en-US&gl=us"
        );
        assert_eq!(
            search_url(SerpEngine::Bing, "rust borrow checker"),
            "https://www.bing.com/search?q=rust+borrow+checker&setmkt=en-US&count=10&first=1"
        );
        assert_eq!(
            search_url(SerpEngine::DuckDuckGo, "rust borrow checker"),
            "https://html.duckduckgo.com/html/?q=rust+borrow+checker&kl=us-en"
        );
    }

    #[test]
    fn engine_order_and_labels_are_fixed() {
        assert_eq!(
            SERP_ENGINE_ORDER,
            [SerpEngine::Google, SerpEngine::Bing, SerpEngine::DuckDuckGo]
        );
        assert_eq!(SerpEngine::Google.as_str(), "google");
        assert_eq!(SerpEngine::Bing.as_str(), "bing");
        assert_eq!(SerpEngine::DuckDuckGo.as_str(), "duckduckgo");
        assert_eq!(SerpEngine::Bing.to_string(), "bing");
        assert_eq!(SerpFailureClass::Network.as_str(), "network");
        assert_eq!(SerpFailureClass::Captcha.as_str(), "captcha");
        assert_eq!(SerpFailureClass::Empty.as_str(), "empty");
        assert_eq!(SerpFailureClass::Blocked.as_str(), "blocked");
    }

    #[test]
    fn search_url_encodes_query_without_losing_market_params() {
        let url = search_url(SerpEngine::Bing, "rust serde&tokio");
        assert!(url.contains("q=rust+serde%26tokio"), "{url}");
        assert!(url.contains("setmkt=en-US"), "{url}");
    }

    #[test]
    fn bing_and_ddg_redirect_urls_decode() {
        // Live organic-result shape probed on 2026-09-10: the `u` payload
        // always carries the 2-char `a1` marker before the base64 body.
        let bing_live = "https://www.bing.com/ck/a?!&&p=f39d01267198674ec99b990\
                        2ad6456a49c84407113edf914d289fd168b709583&ptn=3&ver=2\
                        &u=a1aHR0cHM6Ly93d3cuMTA0LmNvbS50dy9jb21wYW55LzE1MmduZzVj&ntb=1";
        assert_eq!(
            decode_result_url(SerpEngine::Bing, bing_live),
            "https://www.104.com.tw/company/152gng5c"
        );
        // The marker-less form must keep working (other Bing surfaces and
        // older fixtures emit it).
        let bing_plain = "https://www.bing.com/ck/a?u=aHR0cHM6Ly9leGFtcGxlLmNvbS9hP2I9MQ";
        assert_eq!(
            decode_result_url(SerpEngine::Bing, bing_plain),
            "https://example.com/a?b=1"
        );
        // Relative-path payloads resolve against the engine origin.
        let bing_path = "https://www.bing.com/ck/a?u=a1L2ltYWdlcy9zZWFyY2g_cT1ydXN0";
        assert_eq!(
            decode_result_url(SerpEngine::Bing, bing_path),
            "https://www.bing.com/images/search?q=rust"
        );
        let ddg = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fx";
        assert_eq!(
            decode_result_url(SerpEngine::DuckDuckGo, ddg),
            "https://example.com/x"
        );
    }

    #[test]
    fn redirect_decoder_passes_through_non_redirect_urls() {
        let google = "https://example.com/a?b=1";
        assert_eq!(decode_result_url(SerpEngine::Google, google), google);
        assert_eq!(decode_result_url(SerpEngine::Bing, google), google);
        assert_eq!(decode_result_url(SerpEngine::DuckDuckGo, google), google);
        // An undecodable payload falls back to the raw link — never to a
        // shifted-garbage decode.
        assert_eq!(
            decode_result_url(SerpEngine::Bing, "/ck/a?u=not-base64"),
            "/ck/a?u=not-base64"
        );
        // A payload that decodes to text but not to a destination URL is
        // rejected the same way.
        assert_eq!(
            decode_result_url(SerpEngine::Bing, "/ck/a?u=aGVsbG8gd29ybGQ"),
            "/ck/a?u=aGVsbG8gd29ybGQ"
        );
    }

    #[test]
    fn parse_extraction_keeps_captcha_and_skips_malformed_urls() {
        let payload = json!({
            "captcha": true,
            "results": [
                {"title":"a","url":"https://a.example","snippet":"s"},
                {"title":"","url":"","snippet":""},
                {"title":"no url","snippet":"x"},
                "not-an-object"
            ]
        });
        let parsed = parse_extraction(SerpEngine::Bing, &payload);
        assert!(parsed.captcha);
        assert_eq!(parsed.hits.len(), 1);
        assert_eq!(parsed.hits[0].url, "https://a.example");
    }

    #[test]
    fn parse_extraction_caps_results_at_ten() {
        let results: Vec<Value> = (0..15)
            .map(|i| json!({"title": format!("t{i}"), "url": format!("https://e{i}.example")}))
            .collect();
        let parsed = parse_extraction(SerpEngine::Google, &json!({"results": results}));
        assert_eq!(parsed.hits.len(), SERP_MAX_RESULTS);
        assert_eq!(parsed.hits[9].title, "t9");
    }

    #[test]
    fn weighting_is_stable_and_truncates_display_fields() {
        let cfg = SourceWeightConfig::embedded();
        let hits = vec![
            SerpHit {
                title: "low".into(),
                url: "https://blog.csdn.net/a".into(),
                snippet: "low".into(),
            },
            SerpHit {
                title: "default-first".into(),
                url: "https://example.com/first".into(),
                snippet: "first".into(),
            },
            SerpHit {
                title: "default-second".into(),
                url: "https://example.org/second".into(),
                snippet: "second".into(),
            },
            SerpHit {
                title: "gov".into(),
                url: "https://example.gov.cn/a".into(),
                snippet: "x".repeat(SERP_SNIPPET_MAX_CHARS + 20),
            },
        ];
        let out = stabilize_and_weight(SerpEngine::Google, hits, &cfg);
        assert_eq!(out[0].tier, "authoritative");
        assert_eq!(out[1].tier, "default");
        assert_eq!(out[1].title, "default-first");
        assert_eq!(out[2].tier, "default");
        assert_eq!(out[2].title, "default-second");
        assert_eq!(out[3].tier, "low_quality");
        assert_eq!(out[0].snippet.chars().count(), SERP_SNIPPET_MAX_CHARS);
        assert!(out[0].snippet.ends_with('…'));
    }

    /// P2-2 (2026-09-10): the mechanical bound is per field, not per total
    /// payload — a pathological engine-supplied `href` cannot grow the
    /// model-visible envelope past `SERP_URL_MAX_CHARS`, and all ten results
    /// survive regardless of their combined size.
    #[test]
    fn weighted_results_are_bounded_per_field() {
        let cfg = SourceWeightConfig::embedded();
        let hits: Vec<SerpHit> = (0..SERP_MAX_RESULTS)
            .map(|i| SerpHit {
                title: format!("title-{i}"),
                url: format!("https://example.com/{i}/{}", "a".repeat(9_000)),
                snippet: "s".repeat(SERP_SNIPPET_MAX_CHARS + 50),
            })
            .collect();
        let out = stabilize_and_weight(SerpEngine::Google, hits, &cfg);
        assert_eq!(out.len(), SERP_MAX_RESULTS, "no total-payload truncation");
        for result in &out {
            assert_eq!(result.url.chars().count(), SERP_URL_MAX_CHARS);
            assert_eq!(result.snippet.chars().count(), SERP_SNIPPET_MAX_CHARS);
            assert!(!result.title.is_empty());
            assert!(result.title.chars().count() <= SERP_TITLE_MAX_CHARS);
        }
    }

    #[test]
    fn available_engines_skip_memoized_failures_in_fixed_order() {
        let mut state = SerpSessionState::new();
        assert_eq!(
            available_engines(&state),
            vec![SerpEngine::Google, SerpEngine::Bing, SerpEngine::DuckDuckGo]
        );
        state.record_failure(SerpEngine::Google, SerpFailureClass::Network);
        assert_eq!(
            available_engines(&state),
            vec![SerpEngine::Bing, SerpEngine::DuckDuckGo]
        );
        state.record_failure(SerpEngine::DuckDuckGo, SerpFailureClass::Captcha);
        assert_eq!(available_engines(&state), vec![SerpEngine::Bing]);
        state.record_failure(SerpEngine::Bing, SerpFailureClass::Empty);
        assert!(available_engines(&state).is_empty());
    }

    #[test]
    fn session_state_memoizes_failures_and_caps_navigations() {
        let mut state = SerpSessionState::new();
        let now = Instant::now();
        assert!(!state.ceiling_reached());
        state.record_failure(SerpEngine::Google, SerpFailureClass::Network);
        assert_eq!(
            state.failure(SerpEngine::Google),
            Some(SerpFailureClass::Network)
        );
        // P2-4：上限计的是引擎导航次数，不是 search 调用次数。
        for _ in 0..SERP_MAX_NAVIGATIONS_PER_SESSION {
            state.record_navigation();
            state.begin_search("q", now);
        }
        assert!(state.ceiling_reached());
    }

    #[test]
    fn session_state_pacing_has_minimum_plus_jitter() {
        let mut state = SerpSessionState::new();
        let now = Instant::now();
        let first = state.begin_search("q", now);
        assert_eq!(first, Duration::ZERO, "first search is immediate");
        let second = state.begin_search("q", now);
        assert!(
            second >= SERP_SEARCH_COOLDOWN && second <= SERP_SEARCH_COOLDOWN * 3 / 2,
            "immediate second search must be 5s + 0..50% jitter: {second:?}"
        );
    }

    #[test]
    fn failure_memo_overwrites_with_latest_class() {
        let mut state = SerpSessionState::new();
        state.record_failure(SerpEngine::Bing, SerpFailureClass::Network);
        state.record_failure(SerpEngine::Bing, SerpFailureClass::Captcha);
        assert_eq!(
            state.failure(SerpEngine::Bing),
            Some(SerpFailureClass::Captcha)
        );
        assert_eq!(state.failure(SerpEngine::Google), None);
    }
}
