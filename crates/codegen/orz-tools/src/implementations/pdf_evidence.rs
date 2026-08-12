//! Content-addressed PDF evidence store.
//!
//! PDF-first sources (papers, standards, technical reports) are downloaded
//! into a local, SHA-256-addressed evidence store so page text, metadata and
//! extraction records are reproducible (ADR-0010 §3.7.6). The store layout
//! follows the pre-ADR design's evidence store:
//!
//! ```text
//! {root}/{sha256[..2]}/{sha256_full}/
//!     original.pdf      raw bytes (never modified)
//!     pages.jsonl       one `PageEntry` per line, 1-based page numbers
//!     metadata.json     written LAST — its presence is the completion marker
//! ```
//!
//! Failure modes are explicit (ADR-0010 §3.7.2): `INVALID_PDF` /
//! `TOO_LARGE` / `TIMEOUT` are hard errors; a PDF without a usable text
//! layer is NOT an error — it is ingested with `has_text_layer=false` and
//! `extraction_status="no_text_layer"` so the caller can report it as a
//! metadata-only evidence instead of silently downgrading.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::read_file::metadata::is_pdf_magic;
use super::read_file::pdf::{MAX_PDF_BYTES, PDF_PROCESS_TIMEOUT};

/// Envelope schema version for `metadata.json` (sidecar discipline).
pub const PDF_EVIDENCE_SCHEMA_VERSION: &str = "0.1.0-draft";

/// Maximum inline return text (page text preview) per ingest / read call.
/// Mirrors `MAX_READ_CHARS` (browser_read) and the web_fetch inline budget.
pub const MAX_RETURN_TEXT_CHARS: usize = 100_000;

/// pdf_oxide version recorded in extraction records. The workspace pin is
/// EXACT (`=0.3.46`, review 2026-08-11 D2-1 — a caret range drifted to 0.3.46
/// while this constant said 0.3.43). Bump both together.
const PDF_OXIDE_VERSION: &str = "0.3.46";

/// Page text preview separator.
const PAGE_HEADER_PREFIX: &str = "--- Page ";

/// Marker prefix in the tool output. Cross-crate contract between orz-tools
/// (producer) and orz-loop `build_evidence_record` (consumer); tests on both
/// sides lock the format. Format changes must be synchronized.
pub const PDF_EVIDENCE_MARKER_PREFIX: &str = "PDF evidence: ";

/// Truncation footer for ingest output (web_fetch name space).
pub const PDF_INLINE_TRUNCATED_FOOTER_PREFIX: &str = "[web_fetch pdf content truncated:";

/// One page of extracted text.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageEntry {
    /// 1-based page number.
    pub page: usize,
    pub text: String,
    pub char_count: usize,
}

/// Disk envelope for `metadata.json` (sidecar discipline: schema_version
/// first, presence of the file = completion marker).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PdfMetadata {
    pub schema_version: String,
    /// `"sha256:{64 hex}"` — the content address of `original.pdf`.
    pub document_id: String,
    pub source_url: String,
    /// RFC 3339.
    pub downloaded_at: String,
    pub sha256: String,
    pub bytes: usize,
    pub pages: usize,
    pub has_text_layer: bool,
    /// `"ok"` | `"no_text_layer"`.
    pub extraction_status: String,
    pub parser: String,
    pub total_chars: usize,
}

/// Successful ingest result. A PDF with no text layer is still `Ok` — the
/// caller must surface `has_text_layer=false` explicitly.
#[derive(Debug)]
pub struct PdfIngestResult {
    /// `"sha256:{64 hex}"`.
    pub document_id: String,
    pub sha256_hex: String,
    pub page_count: usize,
    pub has_text_layer: bool,
    pub total_chars: usize,
    /// First-page preview: marker line + page text, truncated to
    /// `MAX_RETURN_TEXT_CHARS` with a truncation footer appended.
    pub return_text: String,
    pub truncated: bool,
    pub dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum PdfEvidenceError {
    #[error("not a valid PDF (magic or parse): {0}")]
    InvalidPdf(String),
    #[error("PDF exceeds maximum size of {max} bytes (got {actual})")]
    TooLarge { max: usize, actual: usize },
    #[error("PDF extraction timed out after {0}s")]
    Timeout(u64),
    #[error("invalid document id: {0}")]
    InvalidDocumentId(String),
    #[error("document not found in evidence store: {0}")]
    NotFound(String),
    #[error("invalid page range: {0}")]
    InvalidPageRange(String),
    #[error("evidence metadata corrupt: {0}")]
    Corrupt(String),
    #[error("evidence store IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl PdfEvidenceError {
    /// Stable error-code prefix for model-visible messages.
    pub fn tool_error_code(&self) -> &'static str {
        match self {
            Self::InvalidPdf(_) => "web_fetch_pdf_invalid",
            Self::TooLarge { .. } => "web_fetch_pdf_too_large",
            Self::Timeout(_) => "web_fetch_pdf_timeout",
            Self::InvalidDocumentId(_) => "web_fetch_pdf_invalid_document_id",
            Self::NotFound(_) => "web_fetch_pdf_not_found",
            Self::InvalidPageRange(_) => "web_fetch_pdf_invalid_page_range",
            Self::Corrupt(_) => "web_fetch_pdf_corrupt",
            Self::Io(_) => "web_fetch_pdf_io_error",
        }
    }
}

/// Ingest raw PDF bytes into the evidence store and produce the inline
/// preview text. Idempotent per content address: re-ingesting identical bytes
/// overwrites the same directory with identical content.
pub async fn ingest_pdf_bytes(
    bytes: Vec<u8>,
    evidence_root: &Path,
    source_url: &str,
) -> Result<PdfIngestResult, PdfEvidenceError> {
    if bytes.len() > MAX_PDF_BYTES {
        return Err(PdfEvidenceError::TooLarge {
            max: MAX_PDF_BYTES,
            actual: bytes.len(),
        });
    }
    if !is_pdf_magic(&bytes) {
        return Err(PdfEvidenceError::InvalidPdf(
            "missing %PDF magic bytes".to_string(),
        ));
    }

    let digest = Sha256::digest(&bytes);
    let sha256_hex = hex_string(&digest);
    let document_id = format!("sha256:{sha256_hex}");
    let dir = evidence_root.join(&sha256_hex[..2]).join(&sha256_hex);

    let extracted = tokio::time::timeout(
        PDF_PROCESS_TIMEOUT,
        tokio::task::spawn_blocking({
            let dir = dir.clone();
            let document_id = document_id.clone();
            let sha256_hex = sha256_hex.clone();
            let source_url = source_url.to_string();
            move || extract_and_persist(bytes, &dir, &document_id, &sha256_hex, &source_url)
        }),
    )
    .await;

    match extracted {
        Err(_elapsed) => Err(PdfEvidenceError::Timeout(PDF_PROCESS_TIMEOUT.as_secs())),
        Ok(Err(join_err)) => Err(PdfEvidenceError::Corrupt(format!(
            "extraction task failed: {join_err}"
        ))),
        Ok(Ok(res)) => res,
    }
}

/// Blocking extraction + persistence, run inside `spawn_blocking`.
fn extract_and_persist(
    bytes: Vec<u8>,
    dir: &Path,
    document_id: &str,
    sha256_hex: &str,
    source_url: &str,
) -> Result<PdfIngestResult, PdfEvidenceError> {
    let doc = pdf_oxide::PdfDocument::from_bytes(bytes.clone())
        .map_err(|e| PdfEvidenceError::InvalidPdf(format!("failed to parse: {e}")))?;
    let page_count = doc
        .page_count()
        .map_err(|e| PdfEvidenceError::InvalidPdf(format!("failed to read page count: {e}")))?;
    if page_count == 0 {
        return Err(PdfEvidenceError::InvalidPdf("PDF has no pages".to_string()));
    }

    // Extract text layer per page. A fully blank layer is a metadata-only
    // evidence, not a hard failure.
    let mut page_entries = Vec::with_capacity(page_count);
    let mut total_chars = 0usize;
    for idx in 0..page_count {
        let text = match doc.extract_text(idx) {
            Ok(t) => t,
            Err(e) => {
                return Err(PdfEvidenceError::InvalidPdf(format!(
                    "failed to extract page {}: {e}",
                    idx + 1
                )));
            }
        };
        total_chars += text.chars().count();
        page_entries.push(PageEntry {
            page: idx + 1,
            text: text.clone(),
            char_count: text.chars().count(),
        });
    }
    let has_text_layer = page_entries.iter().any(|p| !p.text.trim().is_empty());

    // Persist: original.pdf -> pages.jsonl -> metadata.json (completion
    // marker written last; torn writes self-heal by re-ingest).
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("original.pdf"), &bytes)?;
    let mut pages_jsonl = String::new();
    for entry in &page_entries {
        let line = serde_json::to_string(entry)
            .map_err(|e| PdfEvidenceError::Corrupt(format!("serialize pages.jsonl: {e}")))?;
        pages_jsonl.push_str(&line);
        pages_jsonl.push('\n');
    }
    std::fs::write(dir.join("pages.jsonl"), pages_jsonl)?;

    let metadata = PdfMetadata {
        schema_version: PDF_EVIDENCE_SCHEMA_VERSION.to_string(),
        document_id: document_id.to_string(),
        source_url: source_url.to_string(),
        downloaded_at: chrono::Utc::now().to_rfc3339(),
        sha256: sha256_hex.to_string(),
        bytes: bytes.len(),
        pages: page_count,
        has_text_layer,
        extraction_status: if has_text_layer {
            "ok".to_string()
        } else {
            "no_text_layer".to_string()
        },
        parser: format!("pdf_oxide {PDF_OXIDE_VERSION}"),
        total_chars,
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| PdfEvidenceError::Corrupt(format!("serialize metadata.json: {e}")))?;
    std::fs::write(dir.join("metadata.json"), metadata_json)?;

    let (return_text, truncated) = build_return_text(&page_entries, has_text_layer, document_id);

    Ok(PdfIngestResult {
        document_id: document_id.to_string(),
        sha256_hex: sha256_hex.to_string(),
        page_count,
        has_text_layer,
        total_chars,
        return_text,
        truncated,
        dir: dir.to_path_buf(),
    })
}

/// Marker line + page preview, truncated to `MAX_RETURN_TEXT_CHARS` with the
/// web_fetch-space footer appended (cross-crate contract — see module docs).
fn build_return_text(
    page_entries: &[PageEntry],
    has_text_layer: bool,
    document_id: &str,
) -> (String, bool) {
    let mut text = String::new();
    for entry in page_entries {
        if entry.text.trim().is_empty() {
            continue;
        }
        writeln!(&mut text, "{PAGE_HEADER_PREFIX}{} ---", entry.page).ok();
        text.push_str(&entry.text);
        text.push('\n');
    }
    let truncated = text.chars().count() > MAX_RETURN_TEXT_CHARS;
    if truncated {
        text = text.chars().take(MAX_RETURN_TEXT_CHARS).collect();
    }

    let text_layer = if has_text_layer { "yes" } else { "no" };
    let mut out = format!(
        "{PDF_EVIDENCE_MARKER_PREFIX}{} pages, document_id={document_id}, text_layer={text_layer}",
        page_entries.len()
    );
    if !text.is_empty() {
        out.push_str("\n\n");
        out.push_str(&text);
    }
    if truncated {
        let total = page_entries.iter().map(|p| p.char_count).sum::<usize>();
        write!(
            &mut out,
            "\n\n{PDF_INLINE_TRUNCATED_FOOTER_PREFIX} {} chars]",
            total
        )
        .ok();
    }
    (out, truncated)
}

/// Resolve a `"sha256:{64 hex}"` document id to its store directory.
/// The path derives solely from the hex string — traversal-proof.
pub fn document_dir(evidence_root: &Path, document_id: &str) -> Result<PathBuf, PdfEvidenceError> {
    let hex = document_id
        .strip_prefix("sha256:")
        .ok_or_else(|| PdfEvidenceError::InvalidDocumentId(document_id.to_string()))?;
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(PdfEvidenceError::InvalidDocumentId(document_id.to_string()));
    }
    Ok(evidence_root.join(&hex[..2]).join(hex))
}

/// Read the persisted metadata envelope for a document.
pub fn read_metadata(
    evidence_root: &Path,
    document_id: &str,
) -> Result<PdfMetadata, PdfEvidenceError> {
    let dir = document_dir(evidence_root, document_id)?;
    let path = dir.join("metadata.json");
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw)
            .map_err(|e| PdfEvidenceError::Corrupt(format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(PdfEvidenceError::NotFound(document_id.to_string()))
        }
        Err(e) => Err(PdfEvidenceError::Io(e)),
    }
}

/// Read extracted pages for a document, filtered to 0-based `page_indices`.
/// A corrupt `pages.jsonl` is a hard, explicit error — no silent self-heal.
pub fn read_pages(
    evidence_root: &Path,
    document_id: &str,
    page_indices: &[usize],
) -> Result<Vec<PageEntry>, PdfEvidenceError> {
    let dir = document_dir(evidence_root, document_id)?;
    let path = dir.join("pages.jsonl");
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(PdfEvidenceError::NotFound(document_id.to_string()));
        }
        Err(e) => return Err(PdfEvidenceError::Io(e)),
    };
    let mut pages = Vec::new();
    for (lineno, line) in raw.lines().enumerate() {
        let entry: PageEntry = serde_json::from_str(line).map_err(|e| {
            PdfEvidenceError::Corrupt(format!("{}:{}: {e}", path.display(), lineno + 1))
        })?;
        pages.push(entry);
    }
    Ok(page_indices
        .iter()
        .filter_map(|&idx| pages.iter().find(|p| p.page == idx + 1).cloned())
        .collect())
}

/// Hex-encode a SHA-256 digest.
pub fn hex_string(digest: &[u8]) -> String {
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        write!(&mut s, "{b:02x}").ok();
    }
    s
}

// Re-export the page-range parser used by the `pdf_read` tool so both
// callers share one semantic (1-based, comma-separated, "1-" open-ended,
// max 20 pages per call).
pub use super::read_file::pdf::parse_page_range as parse_page_indices;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::implementations::read_file::pdf::make_test_pdf;

    fn tmp_root() -> tempfile::TempDir {
        tempfile::TempDir::new().unwrap()
    }

    #[tokio::test]
    async fn ingest_text_layer_pdf() {
        let root = tmp_root();
        let pdf = make_test_pdf(&["Alpha", "Beta"]);
        let pdf_len = pdf.len();
        let res = ingest_pdf_bytes(pdf, root.path(), "https://example.com/a.pdf")
            .await
            .unwrap();

        assert!(
            res.document_id.starts_with("sha256:"),
            "{}",
            res.document_id
        );
        assert_eq!(res.sha256_hex.len(), 64);
        assert_eq!(res.page_count, 2);
        assert!(res.has_text_layer);
        assert!(res.return_text.contains("PDF evidence: 2 pages"));
        assert!(res.return_text.contains(&res.document_id));
        assert!(res.return_text.contains("text_layer=yes"));
        assert!(res.return_text.contains("--- Page 1 ---"));
        assert!(res.return_text.contains("Alpha"));
        assert!(res.return_text.contains("Beta"));
        assert!(!res.truncated);

        let dir = res.dir;
        assert!(dir.join("original.pdf").exists());
        assert!(dir.join("pages.jsonl").exists());
        assert!(dir.join("metadata.json").exists());

        let meta = read_metadata(root.path(), &res.document_id).unwrap();
        assert_eq!(meta.schema_version, PDF_EVIDENCE_SCHEMA_VERSION);
        assert_eq!(meta.document_id, res.document_id);
        assert_eq!(meta.pages, 2);
        assert!(meta.has_text_layer);
        assert_eq!(meta.extraction_status, "ok");
        assert_eq!(meta.source_url, "https://example.com/a.pdf");
        assert!(meta.downloaded_at.contains('T'));
        assert_eq!(meta.bytes, pdf_len);
    }

    #[tokio::test]
    async fn ingest_no_text_layer_is_not_hard_failure() {
        let root = tmp_root();
        // A page with only whitespace text → blank text layer.
        let pdf = make_test_pdf(&["   "]);
        let res = ingest_pdf_bytes(pdf, root.path(), "https://example.com/scan.pdf")
            .await
            .unwrap();

        assert!(!res.has_text_layer);
        assert!(res.return_text.contains("text_layer=no"));
        let meta = read_metadata(root.path(), &res.document_id).unwrap();
        assert_eq!(meta.extraction_status, "no_text_layer");
        // pages.jsonl still written (blank lines).
        let pages = read_pages(root.path(), &res.document_id, &[0]).unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].text.trim().is_empty());
    }

    #[tokio::test]
    async fn ingest_invalid_pdf_garbage_bytes() {
        let root = tmp_root();
        let err = ingest_pdf_bytes(b"not a pdf at all".to_vec(), root.path(), "u")
            .await
            .unwrap_err();
        assert!(matches!(err, PdfEvidenceError::InvalidPdf(_)));
        assert_eq!(err.tool_error_code(), "web_fetch_pdf_invalid");
    }

    #[tokio::test]
    async fn ingest_html_masquerade_rejected() {
        let root = tmp_root();
        let err = ingest_pdf_bytes(
            b"<html><body>login wall</body></html>".to_vec(),
            root.path(),
            "u",
        )
        .await
        .unwrap_err();
        assert!(matches!(err, PdfEvidenceError::InvalidPdf(_)));
    }

    #[tokio::test]
    async fn ingest_magic_ok_but_parse_fails() {
        let root = tmp_root();
        // %PDF magic present but body is garbage — parser rejects.
        let bytes = b"%PDF-1.4\nthis is not a real pdf structure";
        let err = ingest_pdf_bytes(bytes.to_vec(), root.path(), "u")
            .await
            .unwrap_err();
        assert!(matches!(err, PdfEvidenceError::InvalidPdf(_)), "{err:?}");
    }

    #[tokio::test]
    async fn ingest_too_large() {
        let root = tmp_root();
        let bytes = vec![0u8; MAX_PDF_BYTES + 1];
        let err = ingest_pdf_bytes(bytes, root.path(), "u").await.unwrap_err();
        assert!(matches!(err, PdfEvidenceError::TooLarge { .. }));
        assert_eq!(err.tool_error_code(), "web_fetch_pdf_too_large");
    }

    #[tokio::test]
    async fn ingest_truncates_long_text_with_footer() {
        let root = tmp_root();
        // Multiple 30k-char pages (a single >32k text object is truncated by
        // the pdf_oxide parser at 32767 chars — engine behavior, see audit).
        let pages: Vec<String> = (0..6).map(|_| "x".repeat(30_000)).collect();
        let refs: Vec<&str> = pages.iter().map(|s| s.as_str()).collect();
        let pdf = make_test_pdf(&refs);
        let res = ingest_pdf_bytes(pdf, root.path(), "u").await.unwrap();
        assert!(res.truncated);
        assert!(res.total_chars > MAX_RETURN_TEXT_CHARS);
        assert!(res.return_text.contains(PDF_INLINE_TRUNCATED_FOOTER_PREFIX));
        // Marker always preserved before the page text.
        assert!(res.return_text.starts_with(PDF_EVIDENCE_MARKER_PREFIX));
    }

    #[test]
    fn document_dir_validation_rejects_traversal() {
        let root = tmp_root();
        for bad in ["sha256:abc", "../etc", "x", "", "sha256:"] {
            assert!(
                matches!(
                    document_dir(root.path(), bad),
                    Err(PdfEvidenceError::InvalidDocumentId(_))
                ),
                "should reject: {bad:?}"
            );
        }
        let ok_id = format!("sha256:{}", "a".repeat(64));
        let dir = document_dir(root.path(), &ok_id).unwrap();
        assert!(dir.ends_with(format!("{}/{}", "a".repeat(2), "a".repeat(64))));
        assert!(dir.starts_with(root.path()));
    }

    #[tokio::test]
    async fn read_pages_round_trip() {
        let root = tmp_root();
        let pdf = make_test_pdf(&["One", "Two", "Three"]);
        let res = ingest_pdf_bytes(pdf, root.path(), "u").await.unwrap();

        // Reuse the shared page-range parser semantics (0-based indices).
        let indices = parse_page_indices("1-2", 3).unwrap();
        let pages = read_pages(root.path(), &res.document_id, &indices).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].page, 1);
        assert_eq!(pages[1].page, 2);
        assert_eq!(pages[0].text, "One");
        assert_eq!(pages[1].text, "Two");
        assert_eq!(pages[0].char_count, 3);
    }

    #[tokio::test]
    async fn read_pages_not_found() {
        let root = tmp_root();
        let id = format!("sha256:{}", "b".repeat(64));
        let err = read_pages(root.path(), &id, &[0]).unwrap_err();
        assert!(matches!(err, PdfEvidenceError::NotFound(_)));
        assert_eq!(err.tool_error_code(), "web_fetch_pdf_not_found");
    }

    #[tokio::test]
    async fn corrupt_pages_jsonl_is_explicit() {
        let root = tmp_root();
        let pdf = make_test_pdf(&["Alpha"]);
        let res = ingest_pdf_bytes(pdf, root.path(), "u").await.unwrap();
        std::fs::write(res.dir.join("pages.jsonl"), b"{not json\n").unwrap();
        let err = read_pages(root.path(), &res.document_id, &[0]).unwrap_err();
        assert!(matches!(err, PdfEvidenceError::Corrupt(_)));
        assert_eq!(err.tool_error_code(), "web_fetch_pdf_corrupt");
    }

    #[tokio::test]
    async fn reingest_same_bytes_is_idempotent() {
        let root = tmp_root();
        let pdf = make_test_pdf(&["Alpha"]);
        let a = ingest_pdf_bytes(pdf.clone(), root.path(), "u")
            .await
            .unwrap();
        let b = ingest_pdf_bytes(pdf, root.path(), "u").await.unwrap();
        assert_eq!(a.document_id, b.document_id);
        assert_eq!(a.dir, b.dir);
        let pages = read_pages(root.path(), &a.document_id, &[0]).unwrap();
        assert_eq!(pages[0].text, "Alpha");
    }

    #[test]
    fn hex_string_encoding() {
        assert_eq!(hex_string(&[0xAB, 0xCD, 0x00, 0xFF]), "abcd00ff");
    }
}
