//! `local_browser` lane (ADR-0010 §3.7) — real browser retrieval via a
//! headless Chrome/Edge on an isolated profile (GAP-RETRIEVAL-TOOLS
//! boundary, 2026-08-10: capability was explicitly `Unsupported`).
//!
//! MVP scope (user ruling 2026-08-10): pure webpage reading — gate the URL,
//! navigate, extract rendered text, record evidence. The PDF evidence
//! pipeline (2026-08-11) adds `download_or_read` — an owned-tab download
//! into an isolated staging dir for whitelisted paper-library domains
//! (`ORZ_PDF_BROWSER_DOMAINS`), consumed by the host's PDF evidence store.
//! Still NO form interaction, NO arbitrary JS evaluation (ADR-0010 §3.7.3
//! keeps those out of the MVP; the only `Runtime.evaluate` calls are
//! host-owned fixed expressions).
//!
//! Architecture:
//! - [`BrowserSession`] trait — the tool side depends on this, so tests and
//!   conformance captures inject fakes (same pattern as `ModelGateway` /
//!   `LoopHost` / `connect_inprocess`).
//! - [`LocalBrowserManager`] — owns one [`CdpBrowserSession`] per session
//!   (launch once, reuse across runs, kill on shutdown) plus profile-dir
//!   cleanup; fail-closed with [`UnavailableBrowserSession`] when nothing
//!   was launched.
//! - `browser_read` — the single host-owned tool (project_doc_index
//!   pattern): `{ "url": "https://…", "mode": "full|preview|keywords" }` →
//!   read-only page text (P0-B step 4: full / preview / keyword excerpts).
//!   One tab per call, never handed to the model (§3.7.6).

mod cdp;
mod discovery;
mod url_gate;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use orz_loop::host::{ToolDef, ToolError, ToolResult};
use serde_json::json;

pub use cdp::{CdpBrowserSession, CdpConfig, CdpError, PageReadOutcome};
pub use discovery::{
    BrowserBinary, DiscoveryError, DiscoveryOrigin, ORZ_BROWSER_PATH_ENV, find_browser,
};
pub use url_gate::{UrlGateError, check_navigation_url, check_navigation_url_sync};

/// `browser_read` read-scope values (P0-B step 4, 2026-08-14).
pub const MODE_FULL: &str = "full";
pub const MODE_PREVIEW: &str = "preview";
pub const MODE_KEYWORDS: &str = "keywords";

/// Max characters of page text returned to the model in full mode (tool
/// contract — §3.7.7 download-size limits; aligned with the Python LBR
/// precedent of 100k chars). Truncation appends a mechanical footer so the
/// evidence layer maps it to `partial_text_observed`.
pub const MAX_READ_CHARS: usize = 100_000;

/// Max characters returned in preview mode — the first portion of the
/// rendered page text (P0-B step 4). A page short enough to fit is returned
/// complete (evidence stays `full_text_observed`); otherwise a mechanical
/// footer marks the preview as partial.
pub const PREVIEW_READ_CHARS: usize = 4_000;

/// Keyword-extraction bounds (P0-B step 4): at most this many terms, each
/// at most this many chars, excerpts of ±this many chars around a match, at
/// most this many merged excerpts per term, and this many total excerpt
/// chars returned to the model.
pub const MAX_KEYWORDS: usize = 16;
pub const MAX_KEYWORD_CHARS: usize = 64;
pub const KEYWORD_EXCERPT_RADIUS: usize = 160;
pub const KEYWORD_EXCERPTS_PER_TERM: usize = 3;
pub const KEYWORD_TOTAL_CHARS: usize = 12_000;

/// Footer prefix when the page text is truncated (evidence layer matches
/// this prefix to downgrade visibility — same shape as web_fetch's
/// `"\n\n[web_fetch content truncated: ...]"`: a leading blank line
/// separates the footer from page text).
pub const TRUNCATED_FOOTER_PREFIX: &str = "\n\n[browser_read content truncated:";

/// Outcome of one `download_or_read` call.
#[derive(Debug, Clone)]
pub enum BrowserDownloadOutcome {
    /// A download completed into the staging dir. The file is NOT yet
    /// validated — the PDF evidence pipeline verifies magic/parse.
    Pdf { path: PathBuf, final_url: String },
    /// The navigation produced a rendered page instead of a download.
    Page(PageReadOutcome),
}

/// The read-only browser surface the tools and probe rely on.
#[async_trait]
pub trait BrowserSession: Send + Sync {
    /// Read one URL. Implementations must enforce the URL gate themselves
    /// (fail-closed) and return explicit errors — never a silent fallback.
    async fn read_page(&self, url: &str) -> Result<PageReadOutcome, CdpError>;

    /// Download-or-read one URL into `download_dir` (PDF evidence pipeline,
    /// 2026-08-11). The URL gate applies on entry and on every redirect; a
    /// canceled download is an explicit error, never a fallback to a read.
    /// Implementations must clean the staging dir before downloading so
    /// "newest file" semantics are exact.
    async fn download_or_read(
        &self,
        url: &str,
        download_dir: &Path,
    ) -> Result<BrowserDownloadOutcome, CdpError>;

    /// True when a browser is actually available (drives tool declaration).
    fn ready(&self) -> bool;

    /// Session teardown: kill the process tree, best-effort remove the
    /// profile directory.
    async fn shutdown(&self);
}

/// The real implementation: one headless browser per session, launched
/// lazily by the capability probe and reused across runs (same profile dir →
/// no profile-lock conflicts).
pub struct LocalBrowserManager {
    /// tokio Mutex: `CdpBrowserSession::read_page` needs `&mut self` across
    /// awaits, so the guard is held over the whole read (std Mutex guards
    /// must not cross awaits — codebase rule).
    inner: tokio::sync::Mutex<Option<CdpBrowserSession>>,
}

impl LocalBrowserManager {
    /// Wrap an already-launched session (the probe does the launching).
    pub fn new(session: CdpBrowserSession) -> Self {
        Self {
            inner: tokio::sync::Mutex::new(Some(session)),
        }
    }

    /// The profile directory this manager would use — kept here so the
    /// probe and the manager agree on one layout.
    pub fn profile_dir_for(workspace: &std::path::Path, session_id: &str) -> PathBuf {
        let suffix: String = session_id.chars().take(8).collect();
        workspace
            .join(".gsa")
            .join(format!("chrome-profile-{suffix}"))
    }
}

#[async_trait]
impl BrowserSession for LocalBrowserManager {
    async fn read_page(&self, url: &str) -> Result<PageReadOutcome, CdpError> {
        let mut guard = self.inner.lock().await;
        let session = guard
            .as_mut()
            .ok_or_else(|| CdpError::Io("browser session not launched".into()))?;
        session.read_page(url).await
    }

    async fn download_or_read(
        &self,
        url: &str,
        download_dir: &Path,
    ) -> Result<BrowserDownloadOutcome, CdpError> {
        let mut guard = self.inner.lock().await;
        let session = guard
            .as_mut()
            .ok_or_else(|| CdpError::Io("browser session not launched".into()))?;
        session.download_or_read(url, download_dir).await
    }

    fn ready(&self) -> bool {
        // try_lock: a read in progress must not block declaration checks.
        // The process must be alive too — a browser killed out-of-band (crash,
        // OOM, external kill) must report not-ready so the next probe
        // relaunches it (D-3 self-healing covers process death, not just
        // launch failure).
        self.inner
            .try_lock()
            .map(|mut g| g.as_mut().is_some_and(|s| s.is_alive()))
            .unwrap_or(false)
    }

    async fn shutdown(&self) {
        let mut guard = self.inner.lock().await;
        if let Some(mut session) = guard.take() {
            session.shutdown().await;
        }
    }
}

/// Fail-closed placeholder — returned when no browser was launched (probe
/// failed or mode is off). `browser_read` must report an explicit error
/// through this, never silently succeed.
pub struct UnavailableBrowserSession {
    reason: String,
}

impl UnavailableBrowserSession {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

#[async_trait]
impl BrowserSession for UnavailableBrowserSession {
    async fn read_page(&self, _url: &str) -> Result<PageReadOutcome, CdpError> {
        Err(CdpError::Io(format!(
            "browser unavailable: {}",
            self.reason
        )))
    }

    async fn download_or_read(
        &self,
        _url: &str,
        _download_dir: &Path,
    ) -> Result<BrowserDownloadOutcome, CdpError> {
        Err(CdpError::Io(format!(
            "browser unavailable: {}",
            self.reason
        )))
    }

    fn ready(&self) -> bool {
        false
    }

    async fn shutdown(&self) {}
}

/// Shared handle type used by `OrzHost` and the probe.
pub type SharedBrowser = Arc<dyn BrowserSession>;

/// Serializes browser launch vs teardown on the SAME profile directory
/// (2026-08-10 review M4): `close_session` detaches `shutdown()`, so a
/// same-session immediate re-open could race the old teardown (taskkill +
/// profile-dir delete with up to 2s of retries) against the new Chrome's
/// launch on the same profile — the delete could pull the new browser's
/// DevToolsActivePort, or the two Chromes collide on the profile
/// SingletonLock. Both sides hold the profile's async lock, so the probe
/// simply waits out the detached teardown. Entries are one tiny Arc per
/// distinct profile path (one per session) and are never evicted — bounded
/// by the session table in practice.
static PROFILE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>> =
    OnceLock::new();

fn profile_lock(profile_dir: &Path) -> Arc<tokio::sync::Mutex<()>> {
    let map = PROFILE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    map.lock()
        .unwrap()
        .entry(profile_dir.to_path_buf())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// Tool definition for `browser_read` (host-owned; declared only when the
/// browser is ready — declaration and probe are the same source of truth).
pub fn browser_read_tool_def() -> ToolDef {
    ToolDef {
        name: "browser_read".to_string(),
        description: "Read a webpage with the local headless browser: navigates \
             to the URL (http/https public pages only — file://, localhost, \
             private IPs and cloud metadata are blocked by policy), waits for \
             load, and returns the rendered page text + title + final URL. \
             mode=full (default) returns up to 100000 chars of page text; \
             mode=preview returns the first 4000 chars; mode=keywords returns \
             bounded excerpts around the given keywords. \
             One tab per call, closed after reading; page content is evidence \
             only. PDFs, forms and JavaScript evaluation are not supported."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Public http(s) URL to read" },
                "mode": {
                    "type": "string",
                    "enum": ["full", "preview", "keywords"],
                    "description": "Read scope (default full): full returns up to \
                        100000 chars of page text; preview returns the first 4000 \
                        chars; keywords returns bounded excerpts around the given \
                        keywords.",
                },
                "keywords": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 },
                    "maxItems": 16,
                    "description": "Required when mode=keywords: 1..16 non-empty \
                        terms (each up to 64 chars) to extract excerpts for.",
                },
            },
            "required": ["url"],
        }),
    }
}

/// The `browser_read` tool execution: parse args strictly, delegate to the
/// session, map outcomes to `ToolResult` (exit 0 with the read text + a
/// structured footer) and errors to explicit `ToolError`s.
pub async fn handle_browser_read(
    browser: &dyn BrowserSession,
    args: &serde_json::Value,
) -> Result<ToolResult, ToolError> {
    // Strict argument parsing: exactly one `url` string, non-empty. Unknown
    // keys are rejected (not silently ignored) — every failure carries a
    // stable `[browser_read_*]` error code (§3.7.2 explicit errors).
    let obj = args.as_object().ok_or_else(|| {
        ToolError::ExecutionFailed(
            "browser_read failed [browser_read_invalid_arguments]: arguments must be a JSON object"
                .to_string(),
        )
    })?;
    let unknown: Vec<&String> = obj
        .keys()
        .filter(|k| !matches!(k.as_str(), "url" | "mode" | "keywords"))
        .collect();
    if !unknown.is_empty() {
        return Err(ToolError::ExecutionFailed(format!(
            "browser_read failed [browser_read_invalid_arguments]: unknown argument(s): {}",
            unknown
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let url = obj
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed(
                "browser_read failed [browser_read_missing_url]: requires a non-empty `url` argument"
                    .to_string(),
            )
        })?;
    let mode = match obj.get("mode") {
        None => MODE_FULL,
        Some(v) => match v.as_str() {
            Some(m @ (MODE_FULL | MODE_PREVIEW | MODE_KEYWORDS)) => m,
            _ => {
                return Err(ToolError::ExecutionFailed(
                    "browser_read failed [browser_read_invalid_arguments]: `mode` must be \
                     one of \"full\", \"preview\" or \"keywords\""
                        .to_string(),
                ));
            }
        },
    };
    let keywords = match obj.get("keywords") {
        Some(_) if mode != MODE_KEYWORDS => {
            return Err(ToolError::ExecutionFailed(
                "browser_read failed [browser_read_invalid_arguments]: `keywords` is only \
                 allowed in keywords mode"
                    .to_string(),
            ));
        }
        None if mode == MODE_KEYWORDS => {
            return Err(ToolError::ExecutionFailed(
                "browser_read failed [browser_read_invalid_arguments]: keywords mode requires \
                 a non-empty `keywords` array"
                    .to_string(),
            ));
        }
        None => Vec::new(),
        Some(v) => {
            let arr = v.as_array().ok_or_else(|| {
                ToolError::ExecutionFailed(
                    "browser_read failed [browser_read_invalid_arguments]: `keywords` must \
                     be an array of non-empty strings"
                        .to_string(),
                )
            })?;
            if arr.is_empty() || arr.len() > MAX_KEYWORDS {
                return Err(ToolError::ExecutionFailed(format!(
                    "browser_read failed [browser_read_invalid_arguments]: `keywords` must \
                     contain 1..{MAX_KEYWORDS} terms"
                )));
            }
            let mut kws = Vec::with_capacity(arr.len());
            for item in arr {
                let term = item
                    .as_str()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        ToolError::ExecutionFailed(
                            "browser_read failed [browser_read_invalid_arguments]: \
                             `keywords` entries must be non-empty strings"
                                .to_string(),
                        )
                    })?;
                if term.chars().count() > MAX_KEYWORD_CHARS {
                    return Err(ToolError::ExecutionFailed(format!(
                        "browser_read failed [browser_read_invalid_arguments]: keyword \
                         exceeds {MAX_KEYWORD_CHARS} chars"
                    )));
                }
                if kws.iter().any(|k| k == term) {
                    return Err(ToolError::ExecutionFailed(
                        "browser_read failed [browser_read_invalid_arguments]: \
                         `keywords` must be unique"
                            .to_string(),
                    ));
                }
                kws.push(term.to_string());
            }
            kws
        }
    };

    match browser.read_page(url).await {
        Ok(outcome) => {
            let mut text = outcome.text;
            let mut truncated = false;
            let content = match mode {
                MODE_FULL => {
                    if text.chars().count() > MAX_READ_CHARS {
                        text = text.chars().take(MAX_READ_CHARS).collect();
                        truncated = true;
                    }
                    let mut output = text;
                    if truncated {
                        output.push_str(TRUNCATED_FOOTER_PREFIX);
                        output.push_str(&format!(" {} chars, page text only]", MAX_READ_CHARS));
                    }
                    output
                }
                MODE_PREVIEW => {
                    if text.chars().count() > PREVIEW_READ_CHARS {
                        text = text.chars().take(PREVIEW_READ_CHARS).collect();
                        truncated = true;
                        text.push_str(TRUNCATED_FOOTER_PREFIX);
                        text.push_str(&format!(
                            " preview, first {} chars, page text only]",
                            PREVIEW_READ_CHARS
                        ));
                    }
                    text
                }
                MODE_KEYWORDS => {
                    let (excerpts, matched_terms, excerpt_count) =
                        keyword_excerpts(&text, &keywords);
                    let mut output = if excerpts.is_empty() {
                        format!(
                            "[browser_read keywords: no matching excerpts for {} term(s)]",
                            keywords.len()
                        )
                    } else {
                        excerpts
                    };
                    output.push_str(TRUNCATED_FOOTER_PREFIX);
                    output.push_str(&format!(
                        " keyword excerpts ({} terms, {} excerpts), page text only]",
                        matched_terms, excerpt_count
                    ));
                    truncated = true;
                    output
                }
                _ => unreachable!("mode validated above"),
            };
            let out = json!({
                "url": outcome.final_url,
                "title": outcome.title,
                "mode": mode,
                "content": content,
                "truncated": truncated,
            });
            let mut out = out;
            if mode == MODE_KEYWORDS {
                out["keywords"] = json!(keywords);
            }
            if mode == MODE_PREVIEW {
                out["preview_chars"] = json!(PREVIEW_READ_CHARS);
            }
            Ok(ToolResult {
                output: serde_json::to_string(&out).map_err(|e| {
                    ToolError::ExecutionFailed(format!("browser_read serialize: {e}"))
                })?,
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
            })
        }
        Err(CdpError::UrlGate(gate_err)) => Err(ToolError::ExecutionFailed(format!(
            "browser_read refused [{}]: {}",
            gate_err.tool_error_code(),
            gate_err
        ))),
        Err(CdpError::EmptyContent) => Err(ToolError::ExecutionFailed(
            "browser_read failed [browser_read_empty_content]: page produced no \
             readable text (empty page, or a login/CAPTCHA wall — not a silent \
             success)"
                .to_string(),
        )),
        Err(CdpError::LoadTimeout { .. }) => Err(ToolError::ExecutionFailed(
            "browser_read failed [browser_read_load_timeout]: page load timed out".to_string(),
        )),
        Err(CdpError::TotalTimeout { .. }) => Err(ToolError::ExecutionFailed(
            "browser_read failed [browser_read_total_timeout]: total page-read \
             budget exceeded"
                .to_string(),
        )),
        Err(e) => Err(ToolError::ExecutionFailed(format!(
            "browser_read failed [browser_read_failed]: {e}"
        ))),
    }
}

/// Nearest char boundary at or before `idx` (never splits a UTF-8 char).
fn floor_char_boundary(text: &str, mut idx: usize) -> usize {
    if idx >= text.len() {
        return text.len();
    }
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Nearest char boundary at or after `idx` (never splits a UTF-8 char).
fn ceil_char_boundary(text: &str, mut idx: usize) -> usize {
    let len = text.len();
    if idx >= len {
        return len;
    }
    while idx < len && !text.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

/// Mechanical keyword-excerpt extraction (P0-B step 4): case-insensitive
/// substring matching for pure-ASCII text+terms, exact substring matching
/// otherwise; overlapping/adjacent matches merge into one excerpt; each term
/// contributes at most [`KEYWORD_EXCERPTS_PER_TERM`] merged spans and the
/// total output is bounded by [`KEYWORD_TOTAL_CHARS`]. Returns the joined
/// excerpts, the number of terms with ≥1 match, and the excerpt count.
fn keyword_excerpts(text: &str, keywords: &[String]) -> (String, usize, usize) {
    let mut matched_terms = 0usize;
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for kw in keywords {
        let matches: Vec<(usize, usize)> = if text.is_ascii() && kw.is_ascii() {
            let lower_text = text.to_ascii_lowercase();
            let lower_kw = kw.to_ascii_lowercase();
            lower_text
                .match_indices(&lower_kw)
                .map(|(i, m)| (i, i + m.len()))
                .collect()
        } else {
            text.match_indices(kw.as_str())
                .map(|(i, m)| (i, i + m.len()))
                .collect()
        };
        if !matches.is_empty() {
            matched_terms += 1;
        }
        // Merge overlapping/adjacent matches within one term.
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (s, e) in matches {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 {
                    last.1 = last.1.max(e);
                } else {
                    merged.push((s, e));
                }
            } else {
                merged.push((s, e));
            }
        }
        spans.extend(merged.into_iter().take(KEYWORD_EXCERPTS_PER_TERM));
    }
    // Global merge across terms (overlapping excerpts collapse).
    spans.sort_unstable();
    let mut merged_spans: Vec<(usize, usize)> = Vec::new();
    for (s, e) in spans {
        if let Some(last) = merged_spans.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
            } else {
                merged_spans.push((s, e));
            }
        } else {
            merged_spans.push((s, e));
        }
    }

    let mut out = String::new();
    let mut total_chars = 0usize;
    let mut excerpt_count = 0usize;
    for (s, e) in merged_spans {
        if total_chars >= KEYWORD_TOTAL_CHARS {
            break;
        }
        let start = floor_char_boundary(text, s.saturating_sub(KEYWORD_EXCERPT_RADIUS));
        let end = ceil_char_boundary(text, (e + KEYWORD_EXCERPT_RADIUS).min(text.len()));
        let excerpt: String = text[start..end].chars().collect();
        let excerpt_chars = excerpt.chars().count();
        if !out.is_empty() {
            out.push_str("\n---\n");
            total_chars += 5;
        }
        if total_chars + excerpt_chars > KEYWORD_TOTAL_CHARS {
            let remaining = KEYWORD_TOTAL_CHARS.saturating_sub(total_chars);
            let clipped: String = excerpt.chars().take(remaining).collect();
            if start > 0 {
                out.push('…');
            }
            out.push_str(&clipped);
            if end < text.len() {
                out.push('…');
            }
            total_chars = KEYWORD_TOTAL_CHARS;
        } else {
            if start > 0 {
                out.push('…');
            }
            out.push_str(&excerpt);
            if end < text.len() {
                out.push('…');
            }
            total_chars += excerpt_chars;
        }
        excerpt_count += 1;
    }
    (out, matched_terms, excerpt_count)
}

/// Whether `browser_read` should be declared/run (same source of truth as
/// the capability probe).
pub fn browser_ready(browser: &dyn BrowserSession) -> bool {
    browser.ready()
}

/// The capability probe's launch step (ADR-0010 §3.7.1): discover a browser
/// binary, start a headless instance on the session's isolated profile, and
/// wait for a reachable devtools endpoint. Failures return a `Degraded`
/// reason string with a subdivided cause — never a silent fallback.
pub async fn probe_launch(
    workspace: &std::path::Path,
    session_id: &str,
) -> Result<LocalBrowserManager, String> {
    let binary = find_browser(
        std::env::var(discovery::ORZ_BROWSER_PATH_ENV)
            .ok()
            .as_deref(),
    )
    .map_err(|e| format!("browser_not_found: {e}"))?;
    tracing::info!(
        browser = %binary.path.display(),
        origin = ?binary.origin,
        "local_browser: launching headless browser"
    );
    let profile_dir = LocalBrowserManager::profile_dir_for(workspace, session_id);
    // Hold the profile lock across the launch (review M4): a detached
    // teardown of the previous session on this profile must finish first.
    let profile = profile_lock(&profile_dir);
    let _profile_guard = profile.lock().await;
    let session = CdpBrowserSession::launch(binary.path, profile_dir, CdpConfig::default())
        .await
        .map_err(|e| e.to_string())?;
    Ok(LocalBrowserManager::new(session))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Env gate for the live-browser e2e (mirrors the Python LBR e2e
    /// `GSA_RUN_LIVE_BROWSER_TESTS` precedent): the test spawns a REAL
    /// headless browser on the machine — never in CI.
    const LIVE_BROWSER_ENV: &str = "GSA_RUN_LIVE_BROWSER_TESTS";

    /// local_browser (2026-08-10): end-to-end against a real headless
    /// Chrome/Edge — discovery → launch → DevToolsActivePort → read a public
    /// page → rendered text → teardown (process tree + profile dir). Env-
    /// gated; requires network access to example.com.
    #[tokio::test]
    #[ignore = "live browser e2e — GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored local_browser_e2e"]
    async fn local_browser_e2e() {
        if std::env::var_os(LIVE_BROWSER_ENV).is_none() {
            return; // env-gated; the #[ignore] marker is the primary switch
        }
        let workspace =
            std::env::temp_dir().join(format!("orz-browser-e2e-{:?}", std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(&workspace).unwrap();

        let mut session = CdpBrowserSession::launch(
            find_browser(None)
                .expect("a Chrome/Edge binary must be discoverable")
                .path,
            LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-TEST"),
            CdpConfig {
                load_timeout: std::time::Duration::from_secs(30),
                total_budget: std::time::Duration::from_secs(60),
            },
        )
        .await
        .expect("headless browser must launch");
        assert!(
            session.is_alive(),
            "browser process must be alive after launch"
        );
        let outcome = session
            .read_page("https://example.com/")
            .await
            .expect("example.com must be readable");
        assert!(outcome.text.contains("Example Domain"), "{}", outcome.text);
        assert_eq!(outcome.final_url, "https://example.com/");
        assert!(!outcome.title.is_empty());

        // Teardown: kill the tree + delete the profile dir.
        let profile = LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-TEST");
        assert!(profile.exists(), "profile dir must have been created");
        session.shutdown().await;
        assert!(!profile.exists(), "profile dir must be cleaned up");
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// The URL gate must reject a private/metadata URL before the browser
    /// ever navigates — even in a live session, and even when the private
    /// address is hidden behind a DNS name (the DNS layer of the gate, not
    /// just IP literals: `*.nip.io` resolves `169.254.169.254.nip.io` to the
    /// cloud-metadata address).
    #[tokio::test]
    #[ignore = "live browser e2e — GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored local_browser_e2e"]
    async fn local_browser_e2e_gate_blocks_metadata_url() {
        if std::env::var_os(LIVE_BROWSER_ENV).is_none() {
            return;
        }
        let workspace =
            std::env::temp_dir().join(format!("orz-browser-e2e-{:?}", std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(&workspace).unwrap();

        let mut session = CdpBrowserSession::launch(
            find_browser(None)
                .expect("a Chrome/Edge binary must be discoverable")
                .path,
            LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-GATE"),
            CdpConfig::default(),
        )
        .await
        .expect("headless browser must launch");
        // DNS-resolving metadata hostname — the sync layer alone would pass
        // this; the DNS layer must reject it before navigation. Either a
        // private-address block (DNS resolved — the intended case) or a
        // fail-closed DNS failure proves the async gate engaged; both block
        // the navigation. (A literal-IP PrivateAddress case is covered
        // deterministically by the unit test `full_check_rejects_private_...`.)
        let err = session
            .read_page("http://169.254.169.254.nip.io/latest/meta-data/")
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                CdpError::UrlGate(UrlGateError::PrivateAddress { .. })
                    | CdpError::UrlGate(UrlGateError::DnsFailure { .. })
            ),
            "{err:?}"
        );
        session.shutdown().await;
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// PDF evidence (2026-08-11 review C1-1): real-Chrome download e2e —
    /// download a public test PDF through the CDP channel, assert the file
    /// lands in the staging dir with %PDF magic, and that the final URL
    /// comes from the download events (not about:blank). This is the live
    /// check the browser download channel needs: the scripted ws tests
    /// cannot catch a silently-suppressed event family (P1-1) or a lost
    /// source_url (P1-2).
    #[tokio::test]
    #[ignore = "live browser e2e — GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored local_browser_e2e_pdf_download"]
    async fn local_browser_e2e_pdf_download() {
        if std::env::var_os(LIVE_BROWSER_ENV).is_none() {
            return; // env-gated; the #[ignore] marker is the primary switch
        }
        let workspace = std::env::temp_dir().join(format!(
            "orz-browser-e2e-pdf-{:?}",
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(&workspace).unwrap();

        let mut session = CdpBrowserSession::launch(
            find_browser(None)
                .expect("a Chrome/Edge binary must be discoverable")
                .path,
            LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-PDF"),
            CdpConfig::default(),
        )
        .await
        .expect("headless browser must launch");

        let staging = workspace.join("staging");
        let url = "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf";
        let outcome = session
            .download_or_read(url, &staging)
            .await
            .expect("real Chrome must download the PDF");
        match outcome {
            BrowserDownloadOutcome::Pdf { path, final_url } => {
                assert!(path.exists(), "downloaded file must exist");
                let bytes = std::fs::read(&path).unwrap();
                assert!(
                    bytes.starts_with(b"%PDF-"),
                    "downloaded bytes must be a real PDF (magic)"
                );
                assert!(
                    final_url.contains("dummy.pdf"),
                    "final_url must be the downloaded resource URL, got: {final_url}"
                );
                assert_ne!(
                    final_url, "about:blank",
                    "P1-2: source_url must not degrade"
                );
            }
            other => panic!("expected a PDF download, got {other:?}"),
        }

        session.shutdown().await;
        let _ = std::fs::remove_dir_all(&workspace);
    }

    struct StubBrowser {
        outcome: Result<PageReadOutcome, CdpError>,
        download: Result<BrowserDownloadOutcome, CdpError>,
    }

    #[async_trait]
    impl BrowserSession for StubBrowser {
        async fn read_page(&self, _url: &str) -> Result<PageReadOutcome, CdpError> {
            self.outcome.clone()
        }

        async fn download_or_read(
            &self,
            _url: &str,
            _download_dir: &Path,
        ) -> Result<BrowserDownloadOutcome, CdpError> {
            self.download.clone()
        }

        fn ready(&self) -> bool {
            true
        }

        async fn shutdown(&self) {}
    }

    fn sample_outcome(text: &str) -> PageReadOutcome {
        PageReadOutcome {
            final_url: "https://example.com/page".to_string(),
            title: "Example".to_string(),
            text: text.to_string(),
        }
    }

    /// Cross-module test helper (crate tests inject a ready browser lane).
    pub(crate) fn ready_stub_browser() -> SharedBrowser {
        Arc::new(StubBrowser {
            outcome: Ok(sample_outcome("hello page")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("hello page"))),
        })
    }

    #[tokio::test]
    async fn read_success_wraps_text_with_meta() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("hello world")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("hello world"))),
        };
        let result = handle_browser_read(&browser, &json!({"url": "https://example.com/"}))
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["title"], "Example");
        assert_eq!(parsed["content"], "hello world");
        assert_eq!(parsed["truncated"], false);
    }

    #[tokio::test]
    async fn read_truncates_with_mechanical_footer() {
        let long = "x".repeat(MAX_READ_CHARS + 500);
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(&long)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(&long))),
        };
        let result = handle_browser_read(&browser, &json!({"url": "https://example.com/"}))
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["truncated"], true);
        assert!(
            parsed["content"]
                .as_str()
                .unwrap()
                .contains(TRUNCATED_FOOTER_PREFIX),
            "truncated output must carry the mechanical footer"
        );
        assert!(
            parsed["content"].as_str().unwrap().chars().count() < MAX_READ_CHARS + 200,
            "output must stay near the cap"
        );
    }

    /// P0-B step 4 (2026-08-14): preview mode returns the first
    /// `PREVIEW_READ_CHARS` chars with a mechanical footer + `truncated:
    /// true` when the page is longer than the preview budget; a short page
    /// returns complete (no footer, `truncated: false`).
    #[tokio::test]
    async fn preview_mode_truncates_long_pages_and_keeps_short_pages() {
        let long = "x".repeat(PREVIEW_READ_CHARS + 500);
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(&long)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(&long))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "preview"}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["mode"], "preview");
        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["preview_chars"], PREVIEW_READ_CHARS);
        assert!(
            parsed["content"]
                .as_str()
                .unwrap()
                .contains(TRUNCATED_FOOTER_PREFIX),
            "long preview must carry the mechanical footer"
        );

        let short = "short page text";
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(short)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(short))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "preview"}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["truncated"], false);
        assert_eq!(parsed["content"], short);
        assert!(
            !parsed["content"]
                .as_str()
                .unwrap()
                .contains(TRUNCATED_FOOTER_PREFIX),
            "short preview must stay footer-free (complete content)"
        );
    }

    /// P0-B step 4 (2026-08-14): keywords mode returns bounded excerpts
    /// around matches (case-insensitive for ASCII), always `truncated: true`
    /// with the mechanical footer, and echoes the requested keywords.
    #[tokio::test]
    async fn keywords_mode_extracts_bounded_excerpts() {
        let page = "The quick brown fox jumps over the lazy dog. \
                    The QUICK fox is fast.";
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(page)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(page))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["quick"]}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["mode"], "keywords");
        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["keywords"][0], "quick");
        let content = parsed["content"].as_str().unwrap();
        assert!(content.contains("quick brown fox"), "{content}");
        assert!(content.contains("QUICK fox"), "{content}");
        assert!(
            content.contains(TRUNCATED_FOOTER_PREFIX),
            "keywords output must carry the mechanical footer"
        );
        assert!(content.contains("keyword excerpts"), "{content}");
    }

    /// P0-B step 4 (2026-08-14): keywords mode without any match returns a
    /// neutral no-match note (still a successful read, still partial).
    #[tokio::test]
    async fn keywords_mode_no_match_returns_neutral_note() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("page without the term")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(
                "page without the term",
            ))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["absent"]}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["truncated"], true);
        assert!(
            parsed["content"]
                .as_str()
                .unwrap()
                .contains("no matching excerpts"),
            "{parsed}"
        );
    }

    /// P0-B step 4 (2026-08-14): strict argument validation for the new
    /// mode/keywords surface — invalid mode, keywords outside keywords mode,
    /// empty/duplicate/oversized terms and non-array keywords all carry the
    /// stable `[browser_read_invalid_arguments]` code.
    #[tokio::test]
    async fn mode_and_keywords_arguments_are_strictly_validated() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("x")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("x"))),
        };
        let cases = [
            json!({"url": "https://example.com/", "mode": "summary"}),
            json!({"url": "https://example.com/", "keywords": ["a"]}),
            json!({"url": "https://example.com/", "mode": "keywords"}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": []}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["a", ""]}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["a", "a"]}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": "a"}),
            json!({
                "url": "https://example.com/",
                "mode": "keywords",
                "keywords": ["x".repeat(MAX_KEYWORD_CHARS + 1)],
            }),
            json!({
                "url": "https://example.com/",
                "mode": "keywords",
                "keywords": (0..=MAX_KEYWORDS).map(|i| format!("k{i}")).collect::<Vec<_>>(),
            }),
        ];
        for args in cases {
            let err = handle_browser_read(&browser, &args).await.unwrap_err();
            assert!(
                err.to_string().contains("[browser_read_invalid_arguments]"),
                "args {args}: {err}"
            );
        }
    }

    /// P0-B step 4 (2026-08-14): the tool definition declares the mode
    /// enum and the keywords array so the model can select read scope.
    #[test]
    fn tool_def_declares_scope_modes() {
        let def = browser_read_tool_def();
        assert_eq!(def.parameters["properties"]["mode"]["enum"][0], "full");
        assert_eq!(def.parameters["properties"]["mode"]["enum"][1], "preview");
        assert_eq!(def.parameters["properties"]["mode"]["enum"][2], "keywords");
        assert_eq!(
            def.parameters["properties"]["keywords"]["maxItems"],
            MAX_KEYWORDS
        );
        assert_eq!(def.parameters["required"][0], "url");
    }

    #[tokio::test]
    async fn url_gate_error_maps_to_stable_code() {
        let browser = StubBrowser {
            outcome: Err(CdpError::UrlGate(UrlGateError::PrivateAddress {
                host: "10.0.0.1".into(),
            })),
            download: Err(CdpError::UrlGate(UrlGateError::PrivateAddress {
                host: "10.0.0.1".into(),
            })),
        };
        let err = handle_browser_read(&browser, &json!({"url": "http://10.0.0.1/"}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("refused"), "{err}");
        // The stable code must be findable in the error text.
        let code = UrlGateError::PrivateAddress { host: "x".into() }.tool_error_code();
        assert!(!code.is_empty());
    }

    #[tokio::test]
    async fn empty_content_is_explicit_error_not_success() {
        let browser = StubBrowser {
            outcome: Err(CdpError::EmptyContent),
            download: Err(CdpError::EmptyContent),
        };
        let err = handle_browser_read(&browser, &json!({"url": "https://example.com/"}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no readable text"), "{err}");
    }

    #[tokio::test]
    async fn missing_url_is_invalid_arguments() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("x")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("x"))),
        };
        let err = handle_browser_read(&browser, &json!({})).await.unwrap_err();
        assert!(err.to_string().contains("`url`"), "{err}");
        // Every argument failure carries a stable error code.
        assert!(
            err.to_string().contains("[browser_read_missing_url]"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn unknown_arguments_are_rejected() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("x")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("x"))),
        };
        let err = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "timeout": 5}),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("timeout"), "{err}");
        assert!(
            err.to_string().contains("[browser_read_invalid_arguments]"),
            "{err}"
        );
        let err = handle_browser_read(&browser, &json!(42)).await.unwrap_err();
        assert!(
            err.to_string().contains("[browser_read_invalid_arguments]"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn unavailable_session_fails_closed() {
        let browser = UnavailableBrowserSession::new("browser_launch_failed: test".into());
        assert!(!browser.ready());
        let err = browser.read_page("https://example.com/").await.unwrap_err();
        assert!(err.to_string().contains("browser unavailable"), "{err}");
    }

    #[test]
    fn profile_dir_layout_is_gsa_scoped() {
        let dir =
            LocalBrowserManager::profile_dir_for(std::path::Path::new(r"C:\work"), "RUN12345678");
        assert!(dir.starts_with(r"C:\work\.gsa"));
        // session_id.chars().take(8) — "RUN12345678" → "RUN12345"
        assert!(
            dir.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("chrome-profile-RUN12345")
        );
    }
}
