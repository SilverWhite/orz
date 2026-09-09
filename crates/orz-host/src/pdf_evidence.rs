//! PDF evidence pipeline — host side (2026-08-11, track-A closing slice).
//!
//! Two download channels (user ruling): a **browser channel** for whitelisted
//! paper-library domains (`ORZ_PDF_BROWSER_DOMAINS`, e.g. `*.cnki.net`) so
//! school-subscribed libraries are reachable through the operator's manual
//! login (D-13 headed browser), and the **direct channel** (web_fetch, inside
//! orz-tools) for everything else. Routing happens here, at the `call_tool`
//! chokepoint: a `web_fetch` whose URL matches the whitelist is intercepted
//! and executed through the browser; everything else falls through to the
//! toolset (where the direct channel ingests PDFs inline).
//!
//! Whitelist failure mode (user ruling): a whitelist hit that fails in the
//! browser (not logged in, paywall, timeout) is an EXPLICIT error — never an
//! automatic fallback to the direct channel (ADR-0010 §3.7.2).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use orz_loop::host::{ToolDef, ToolError, ToolResult};
use serde_json::Value;

use crate::local_browser::{BrowserDownloadOutcome, BrowserSession, CdpError};

/// Comma-separated browser-channel domain whitelist. `*` wildcards a leading
/// label: `*.cnki.net` matches `kns.cnki.net` but NOT the apex `cnki.net`.
pub const PDF_DOMAINS_ENV: &str = "ORZ_PDF_BROWSER_DOMAINS";

/// Matches the evidence-core cap (read_file/pdf.rs MAX_PDF_BYTES).
const MAX_PDF_BYTES: usize = 50 * 1024 * 1024;

/// Max characters of inline page text returned per `pdf_read` call.
const MAX_READ_CHARS: usize = 100_000;

/// Footer prefix for truncated `pdf_read` output (evidence layer matches it).
pub const PDF_READ_TRUNCATED_FOOTER_PREFIX: &str = "\n\n[pdf_read content truncated:";

/// Test seam (set_force_rescan precedent): production reads the env var.
static DOMAINS_OVERRIDE: Mutex<Option<String>> = Mutex::new(None);

/// Override the whitelist for tests. `None` restores env-driven behavior.
#[doc(hidden)]
pub fn set_domains_override(domains: Option<String>) {
    *DOMAINS_OVERRIDE.lock().unwrap() = domains;
}

/// Split a raw env value into normalized domain entries.
fn parse_domains(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn whitelist() -> Vec<String> {
    let raw = DOMAINS_OVERRIDE
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| std::env::var(PDF_DOMAINS_ENV).unwrap_or_default());
    parse_domains(&raw)
}

/// Exact wildcard semantics (locked by tests):
/// - `example.com` matches the host `example.com` only (case-insensitive,
///   trailing dot normalized);
/// - `*.example.com` matches any subdomain of `example.com`, NOT the apex;
/// - `*` matches everything (documented as dangerous — default is unset).
pub fn domain_matches(entry: &str, host: &str) -> bool {
    let host = host.to_ascii_lowercase().trim_end_matches('.').to_string();
    let entry = entry.to_ascii_lowercase().trim_end_matches('.').to_string();
    if entry == "*" {
        return true;
    }
    if let Some(domain) = entry.strip_prefix("*.") {
        host.ends_with(&format!(".{domain}")) && host != domain
    } else {
        host == entry
    }
}

/// Whether a URL should be fetched through the browser channel.
pub fn route_for_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let host = host.trim_end_matches('.');
    whitelist().iter().any(|e| domain_matches(e, host))
}

/// `{cwd}/.gsa/pdf-evidence` — content-addressed evidence store root.
pub fn evidence_root(cwd: &Path) -> PathBuf {
    cwd.join(".gsa").join("pdf-evidence")
}

/// `{cwd}/.gsa/pdf-downloads-{session8|HOST}` — browser staging root.
/// Each call uses a fresh unique subdirectory (review 2026-08-11 P2-3):
/// "newest file in dir" semantics are then exact — a leftover from a
/// crashed or timed-out call can never be mistaken for the new download.
pub fn download_dir(cwd: &Path, session_id: Option<&str>) -> PathBuf {
    let suffix: String = session_id
        .map(|s| s.chars().take(8).collect())
        .unwrap_or_else(|| "HOST".to_string());
    cwd.join(".gsa").join(format!("pdf-downloads-{suffix}"))
}

/// The `pdf_read` tool definition (host-owned, always declared — reads the
/// local evidence store, no browser dependency; gated by retrieval mode in
/// the relay, like `project_doc_index`).
pub fn pdf_read_tool_def() -> ToolDef {
    ToolDef {
        name: "pdf_read".to_string(),
        description: "Read pages from a locally stored PDF evidence document \
             (returned by web_fetch as `document_id=sha256:...`). Returns the \
             extracted text layer of the requested pages — no re-download, \
             no network. `page_range` uses 1-based page numbers, supports \
             '3', '1-5' and '5-' (to the last page); an explicit range is \
             capped at 20 pages, omitting it reads all pages (truncated to \
             the output cap). Scanned PDFs without a text layer report an \
             explicit error."
            .to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "document_id": {
                    "type": "string",
                    "description": "Evidence document id, exactly as returned \
                         by web_fetch (sha256:...)."
                },
                "page_range": {
                    "type": "string",
                    "description": "1-based page range: '3', '1-5', '5-'. \
                         Defaults to all pages (truncated to the output cap)."
                },
            },
            "required": ["document_id"],
        }),
    }
}

fn tool_err(code: &str, msg: String) -> ToolError {
    ToolError::ExecutionFailed(format!("{msg} [{code}]"))
}

/// Browser-channel `web_fetch` interception: download (or read) via the
/// browser, then run the evidence pipeline on the downloaded bytes.
pub async fn handle_browser_pdf(
    cwd: &Path,
    session_id: Option<&str>,
    browser: &dyn BrowserSession,
    url: &str,
) -> Result<ToolResult, ToolError> {
    // User ruling: whitelist hit + browser unavailable = explicit failure,
    // NEVER a silent fallback to the direct channel.
    if !browser.ready() {
        return Err(tool_err(
            "web_fetch_pdf_browser_unavailable",
            "web_fetch failed: URL matches ORZ_PDF_BROWSER_DOMAINS but the \
             browser lane is unavailable (probe failed or mode ≠ \
             local_browser). Log in through the browser window and retry — \
             no automatic fallback to direct download"
                .to_string(),
        ));
    }

    // Fresh unique staging subdir per call (P2-3): "newest file" is exact,
    // and a leftover from a crashed call can never be mistaken for this
    // download. `uuid` is the workspace dep (v7).
    let dl_dir =
        download_dir(cwd, session_id).join(&uuid::Uuid::new_v4().simple().to_string()[..8]);
    let outcome = browser
        .download_or_read(url, &dl_dir)
        .await
        .map_err(map_browser_download_error)?;

    match outcome {
        BrowserDownloadOutcome::Page(page) => {
            // Whitelist hit but the response rendered a page (not a
            // download): return the page read, shaped like browser_read.
            let mut text = page.text;
            let mut truncated = false;
            if text.chars().count() > MAX_READ_CHARS {
                text = text.chars().take(MAX_READ_CHARS).collect();
                truncated = true;
            }
            if truncated {
                text.push_str("\n\n[browser_read content truncated: 100000 chars, page text only]");
            }
            let out = serde_json::json!({
                "url": page.final_url,
                "title": page.title,
                "content": text,
                "truncated": truncated,
            });
            Ok(ToolResult {
                output: serde_json::to_string(&out).map_err(|e| {
                    ToolError::ExecutionFailed(format!("browser pdf read serialize: {e}"))
                })?,
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            })
        }
        BrowserDownloadOutcome::Pdf { path, final_url } => {
            // Size check BEFORE reading the whole file into memory
            // (review 2026-08-11 P2-2 — `Browser.setDownloadBehavior` has
            // no byte cap of its own; a 10GB download must not OOM).
            let file_len = std::fs::metadata(&path).map_err(|e| {
                tool_err(
                    "web_fetch_pdf_io_error",
                    format!("web_fetch failed: stat downloaded file: {e}"),
                )
            })?;
            if file_len.len() > MAX_PDF_BYTES as u64 {
                let _ = std::fs::remove_dir_all(&dl_dir);
                return Err(tool_err(
                    "web_fetch_pdf_too_large",
                    format!(
                        "web_fetch failed: downloaded PDF is {:.1} MB, exceeds the {:.0} MB limit",
                        file_len.len() as f64 / 1_048_576.0,
                        MAX_PDF_BYTES as f64 / 1_048_576.0,
                    ),
                ));
            }
            let bytes = std::fs::read(&path).map_err(|e| {
                tool_err(
                    "web_fetch_pdf_io_error",
                    format!("web_fetch failed: read downloaded file: {e}"),
                )
            })?;
            let root = evidence_root(cwd);
            // Ingested (or failed): the staging copy is disposable. Removed
            // on EVERY path after download (P3-1) — success, size-limit
            // above, and ingest failure alike; crash leftovers are covered
            // by the retention `pdf-downloads-` sweep.
            let ingest = match orz_tools::implementations::pdf_evidence::ingest_pdf_bytes(
                bytes, &root, &final_url,
            )
            .await
            {
                Ok(ingest) => ingest,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dl_dir);
                    return Err(map_pdf_evidence_error(e));
                }
            };
            let _ = std::fs::remove_dir_all(&dl_dir);
            Ok(ToolResult {
                output: ingest.return_text,
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            })
        }
    }
}

/// `pdf_read` tool execution: strict argument parsing, read from the local
/// evidence store, explicit errors only.
pub async fn handle_pdf_read(cwd: &Path, args: &Value) -> Result<ToolResult, ToolError> {
    let obj = args.as_object().ok_or_else(|| {
        tool_err(
            "pdf_read_invalid_arguments",
            "pdf_read failed: arguments must be a JSON object".to_string(),
        )
    })?;
    let unknown: Vec<&String> = obj
        .keys()
        .filter(|k| k.as_str() != "document_id" && k.as_str() != "page_range")
        .collect();
    if !unknown.is_empty() {
        return Err(tool_err(
            "pdf_read_invalid_arguments",
            format!(
                "pdf_read failed: unknown argument(s): {}",
                unknown
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    let document_id = obj
        .get("document_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            tool_err(
                "pdf_read_missing_document_id",
                "pdf_read failed: requires a non-empty `document_id` argument".to_string(),
            )
        })?;
    let page_range = obj.get("page_range").and_then(|v| v.as_str());

    let root = evidence_root(cwd);
    let meta = orz_tools::implementations::pdf_evidence::read_metadata(&root, document_id)
        .map_err(map_pdf_read_error)?;
    if !meta.has_text_layer {
        return Err(tool_err(
            "pdf_read_no_text_layer",
            format!(
                "pdf_read failed: document {document_id} has no usable text \
                 layer (scanned PDF — OCR is out of scope)"
            ),
        ));
    }
    let indices = match page_range {
        Some(spec) => orz_tools::implementations::pdf_evidence::parse_page_indices(
            spec, meta.pages,
        )
        .map_err(|e| {
            tool_err(
                "pdf_read_invalid_page_range",
                format!("pdf_read failed: {e}"),
            )
        })?,
        None => (0..meta.pages).collect(),
    };

    let pages = orz_tools::implementations::pdf_evidence::read_pages(&root, document_id, &indices)
        .map_err(map_pdf_read_error)?;

    let mut text = String::new();
    for entry in &pages {
        text.push_str(&format!("--- Page {} ---\n", entry.page));
        text.push_str(&entry.text);
        text.push('\n');
    }
    let mut truncated = false;
    if text.chars().count() > MAX_READ_CHARS {
        text = text.chars().take(MAX_READ_CHARS).collect();
        truncated = true;
    }
    if truncated {
        text.push_str(PDF_READ_TRUNCATED_FOOTER_PREFIX);
        text.push_str(" 100000 chars]");
    }
    Ok(ToolResult {
        output: text,
        exit_code: Some(0),
        output_encoding: None,
        structured: None,
        ..Default::default()
    })
}

/// Map a browser-channel download error to an explicit `[web_fetch_pdf_*]`
/// failure (stable codes, ADR-0010 §3.7.2).
fn map_browser_download_error(e: CdpError) -> ToolError {
    match e {
        CdpError::UrlGate(gate_err) => tool_err(
            "web_fetch_pdf_blocked",
            format!(
                "web_fetch refused [{}]: {}",
                gate_err.tool_error_code(),
                gate_err
            ),
        ),
        CdpError::DownloadCanceled => tool_err(
            "web_fetch_pdf_download_canceled",
            "web_fetch failed: the browser canceled the download (paywall or \
             login wall — log in through the browser window and retry)"
                .to_string(),
        ),
        CdpError::LoadTimeout { .. } => tool_err(
            "web_fetch_pdf_timeout",
            "web_fetch failed: page download timed out".to_string(),
        ),
        CdpError::TotalTimeout { .. } => tool_err(
            "web_fetch_pdf_timeout",
            "web_fetch failed: total download budget exceeded".to_string(),
        ),
        other => tool_err(
            "web_fetch_pdf_browser_failed",
            format!("web_fetch failed: browser download: {other}"),
        ),
    }
}

/// Map an evidence-core error for the browser channel (web_fetch namespace).
fn map_pdf_evidence_error(
    e: orz_tools::implementations::pdf_evidence::PdfEvidenceError,
) -> ToolError {
    tool_err(e.tool_error_code(), format!("web_fetch failed: {e}"))
}

/// Map an evidence-core error for the `pdf_read` namespace.
fn map_pdf_read_error(e: orz_tools::implementations::pdf_evidence::PdfEvidenceError) -> ToolError {
    let code = match e.tool_error_code() {
        "web_fetch_pdf_invalid_document_id" => "pdf_read_invalid_document_id",
        "web_fetch_pdf_not_found" => "pdf_read_not_found",
        "web_fetch_pdf_corrupt" => "pdf_read_corrupt",
        "web_fetch_pdf_io_error" => "pdf_read_io_error",
        other => other,
    };
    tool_err(code, format!("pdf_read failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tmp_cwd(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("orz-pdf-evidence-{label}-{}", std::process::id()))
    }

    // ── whitelist semantics ──────────────────────────────────────────────

    #[test]
    fn domain_matches_exact_and_wildcard() {
        assert!(domain_matches("*.cnki.net", "www.cnki.net"));
        assert!(domain_matches("*.cnki.net", "kns.cnki.net"));
        assert!(!domain_matches("*.cnki.net", "cnki.net")); // apex NOT matched
        assert!(!domain_matches("*.cnki.net", "evil-cnki.net"));
        assert!(domain_matches("cnki.net", "cnki.net"));
        assert!(!domain_matches("cnki.net", "www.cnki.net"));
        assert!(domain_matches("*", "anything.example.org"));
        assert!(!domain_matches("example.com", "example.com.evil.net"));
        // Case-insensitive + trailing dot normalization.
        assert!(domain_matches("Example.COM", "example.com"));
        assert!(domain_matches("*.Example.COM", "WWW.example.com"));
        assert!(domain_matches("example.com.", "example.com"));
    }

    #[test]
    fn domain_matches_ip_literals_only_all_match() {
        assert!(!domain_matches("*.cnki.net", "10.0.0.1"));
        assert!(domain_matches("*", "10.0.0.1"));
    }

    #[tokio::test]
    async fn route_for_url_parses_env_whitelist() {
        // Serialize against other tests mutating the override/env.
        let _lock = crate::tests::tests_env_lock().lock().await;
        set_domains_override(Some(" *.cnki.net , example.com".to_string()));
        assert!(route_for_url("https://kns.cnki.net/kcms/detail"));
        assert!(route_for_url("https://example.com/paper.pdf"));
        assert!(!route_for_url("https://cnki.net/x")); // apex not matched
        assert!(!route_for_url("https://arxiv.org/paper.pdf"));
        assert!(!route_for_url("not a url"));
        assert!(!route_for_url("file:///c:/x.pdf"));
        set_domains_override(None);
    }

    #[tokio::test]
    async fn unset_whitelist_routes_everything_direct() {
        let _lock = crate::tests::tests_env_lock().lock().await;
        set_domains_override(None);
        // Ensure env is clean for this test.
        unsafe { std::env::remove_var(PDF_DOMAINS_ENV) };
        assert!(!route_for_url("https://kns.cnki.net/x"));
        assert!(!route_for_url("https://example.com/x"));
    }

    // ── routing through the browser channel ──────────────────────────────

    /// Test browser whose download outcome can be scripted. Mirrors the
    /// local_browser test stub shape (struct is cfg(test) there, so this
    /// one is crate-local). `download_or_read` simulates the real browser:
    /// the file is materialized into the handler's staging dir and the
    /// returned path points there — so the handler's cleanup assertion
    /// exercises the real contract.
    struct ScriptedBrowser {
        ready_flag: bool,
        outcome: Result<BrowserDownloadOutcome, CdpError>,
    }

    #[async_trait::async_trait]
    impl BrowserSession for ScriptedBrowser {
        async fn read_page(
            &self,
            _url: &str,
            _mode: crate::local_browser::ReadMode,
        ) -> Result<crate::local_browser::PageReadOutcome, CdpError> {
            unreachable!("pdf tests never read pages")
        }
        async fn control(
            &self,
            _action: crate::local_browser::BrowserControlAction,
            _timeout: std::time::Duration,
        ) -> Result<crate::local_browser::BrowserControlOutcome, CdpError> {
            unreachable!("pdf tests never run browser_control")
        }
        async fn download_or_read(
            &self,
            _url: &str,
            download_dir: &Path,
        ) -> Result<BrowserDownloadOutcome, CdpError> {
            match &self.outcome {
                Ok(BrowserDownloadOutcome::Pdf { path, final_url }) => {
                    let bytes = std::fs::read(path).map_err(|e| CdpError::Io(e.to_string()))?;
                    std::fs::create_dir_all(download_dir)
                        .map_err(|e| CdpError::Io(e.to_string()))?;
                    let staged = download_dir.join("paper.pdf");
                    std::fs::write(&staged, &bytes).map_err(|e| CdpError::Io(e.to_string()))?;
                    Ok(BrowserDownloadOutcome::Pdf {
                        path: staged,
                        final_url: final_url.clone(),
                    })
                }
                other => other.clone(),
            }
        }
        fn ready(&self) -> bool {
            self.ready_flag
        }
        async fn shutdown(&self) {}
    }

    fn test_pdf_bytes() -> Vec<u8> {
        orz_tools::implementations::read_file::pdf::make_test_pdf(&["Alpha", "Beta"])
    }

    #[tokio::test]
    async fn whitelist_hit_browser_pipeline_ingests() {
        let cwd = tmp_cwd("browser-ingest");
        std::fs::create_dir_all(&cwd).unwrap();
        // Source bytes live outside the staging tree; the scripted browser
        // materializes them into the handler's per-call staging subdir.
        let source_dir = tmp_cwd("browser-ingest-src");
        std::fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("paper.pdf");
        std::fs::write(&source, test_pdf_bytes()).unwrap();
        let staging_root = download_dir(&cwd, Some("SESS1234"));
        let browser = ScriptedBrowser {
            ready_flag: true,
            outcome: Ok(BrowserDownloadOutcome::Pdf {
                path: source.clone(),
                final_url: "https://kns.cnki.net/paper.pdf".to_string(),
            }),
        };

        let result = handle_browser_pdf(
            &cwd,
            Some("SESS1234"),
            &browser,
            "https://kns.cnki.net/paper.pdf",
        )
        .await
        .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(
            result.output.contains("PDF evidence: 2 pages"),
            "{}",
            result.output
        );
        assert!(
            result.output.contains("document_id=sha256:"),
            "{}",
            result.output
        );
        assert!(result.output.contains("Alpha"));
        // Evidence stored under {cwd}/.gsa/pdf-evidence/.
        let root = evidence_root(&cwd);
        assert!(root.exists());
        // Staging subdir cleaned up (P3-1: every post-download path); the
        // staging ROOT stays (per-call subdirs are unique; the retention
        // `pdf-downloads-` sweep covers crash leftovers).
        let leftover = std::fs::read_dir(&staging_root)
            .map(|rd| rd.flatten().count())
            .unwrap_or(0);
        assert_eq!(leftover, 0, "staging subdirs must be cleaned");
    }

    #[tokio::test]
    async fn whitelist_hit_ingest_failure_cleans_staging() {
        let cwd = tmp_cwd("browser-ingest-fail");
        std::fs::create_dir_all(&cwd).unwrap();
        // HTML masquerade — ingest must fail hard (web_fetch_pdf_invalid).
        let source_dir = tmp_cwd("browser-ingest-fail-src");
        std::fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("paper.pdf");
        std::fs::write(&source, b"<html>login wall</html>").unwrap();
        let staging_root = download_dir(&cwd, Some("SESS1234"));
        let browser = ScriptedBrowser {
            ready_flag: true,
            outcome: Ok(BrowserDownloadOutcome::Pdf {
                path: source.clone(),
                final_url: "https://kns.cnki.net/paper.pdf".to_string(),
            }),
        };

        let err = handle_browser_pdf(
            &cwd,
            Some("SESS1234"),
            &browser,
            "https://kns.cnki.net/paper.pdf",
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("web_fetch_pdf_invalid"), "{err}");
        let leftover = std::fs::read_dir(&staging_root)
            .map(|rd| rd.flatten().count())
            .unwrap_or(0);
        assert_eq!(leftover, 0, "staging must be cleaned on ingest failure");
    }

    #[tokio::test]
    async fn whitelist_hit_browser_unavailable_is_explicit_failure() {
        let cwd = tmp_cwd("browser-unavail");
        std::fs::create_dir_all(&cwd).unwrap();
        let browser = ScriptedBrowser {
            ready_flag: false,
            outcome: Err(CdpError::Io("should not be called".into())),
        };
        let err = handle_browser_pdf(&cwd, None, &browser, "https://kns.cnki.net/paper.pdf")
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("web_fetch_pdf_browser_unavailable"), "{msg}");
        assert!(msg.contains("no automatic fallback"), "{msg}");
    }

    #[tokio::test]
    async fn whitelist_hit_html_page_returns_page_read() {
        let cwd = tmp_cwd("browser-page");
        std::fs::create_dir_all(&cwd).unwrap();
        let browser = ScriptedBrowser {
            ready_flag: true,
            outcome: Ok(BrowserDownloadOutcome::Page(
                crate::local_browser::PageReadOutcome {
                    final_url: "https://kns.cnki.net/kcms/detail".to_string(),
                    title: "CNKI".to_string(),
                    text: "search results".to_string(),
                },
            )),
        };
        let result = handle_browser_pdf(&cwd, None, &browser, "https://kns.cnki.net/kcms/detail")
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["title"], "CNKI");
        assert_eq!(parsed["content"], "search results");
        assert!(!result.output.contains("document_id="));
    }

    #[tokio::test]
    async fn whitelist_hit_canceled_download_is_explicit() {
        let cwd = tmp_cwd("browser-canceled");
        std::fs::create_dir_all(&cwd).unwrap();
        let browser = ScriptedBrowser {
            ready_flag: true,
            outcome: Err(CdpError::DownloadCanceled),
        };
        let err = handle_browser_pdf(&cwd, None, &browser, "https://kns.cnki.net/paper.pdf")
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("web_fetch_pdf_download_canceled"),
            "{err}"
        );
        assert!(err.to_string().contains("log in"), "{err}");
    }

    // ── pdf_read ─────────────────────────────────────────────────────────

    async fn seed_evidence(cwd: &Path) -> String {
        let root = evidence_root(cwd);
        let res = orz_tools::implementations::pdf_evidence::ingest_pdf_bytes(
            test_pdf_bytes(),
            &root,
            "https://example.com/paper.pdf",
        )
        .await
        .unwrap();
        res.document_id
    }

    #[tokio::test]
    async fn pdf_read_returns_requested_pages() {
        let cwd = tmp_cwd("read-ok");
        std::fs::create_dir_all(&cwd).unwrap();
        let doc_id = seed_evidence(&cwd).await;
        let result = handle_pdf_read(&cwd, &json!({"document_id": doc_id, "page_range": "1-2"}))
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains("--- Page 1 ---"));
        assert!(result.output.contains("Alpha"));
        assert!(result.output.contains("--- Page 2 ---"));
        assert!(result.output.contains("Beta"));
    }

    #[tokio::test]
    async fn pdf_read_cross_run_persistence() {
        let cwd = tmp_cwd("read-cross-run");
        std::fs::create_dir_all(&cwd).unwrap();
        let doc_id = seed_evidence(&cwd).await;
        // "New session": a fresh handler reads the same store — no network.
        let result = handle_pdf_read(&cwd, &json!({"document_id": doc_id}))
            .await
            .unwrap();
        assert!(result.output.contains("Alpha"));
    }

    #[tokio::test]
    async fn pdf_read_error_codes() {
        let cwd = tmp_cwd("read-errors");
        std::fs::create_dir_all(&cwd).unwrap();
        let doc_id = seed_evidence(&cwd).await;

        // Missing document_id.
        let err = handle_pdf_read(&cwd, &json!({})).await.unwrap_err();
        assert!(
            err.to_string().contains("pdf_read_missing_document_id"),
            "{err}"
        );
        // Invalid document_id.
        let err = handle_pdf_read(&cwd, &json!({"document_id": "sha256:abc"}))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("pdf_read_invalid_document_id"),
            "{err}"
        );
        // Unknown id.
        let err = handle_pdf_read(
            &cwd,
            &json!({"document_id": format!("sha256:{}", "c".repeat(64))}),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("pdf_read_not_found"), "{err}");
        // Bad page range.
        let err = handle_pdf_read(&cwd, &json!({"document_id": doc_id, "page_range": "3-1"}))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("pdf_read_invalid_page_range"),
            "{err}"
        );
        // Unknown argument.
        let err = handle_pdf_read(&cwd, &json!({"document_id": doc_id, "pages": "1"}))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("pdf_read_invalid_arguments"),
            "{err}"
        );
    }
}
