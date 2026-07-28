"""Retrieval workflow — LBR-001 Phase 3.

Orchestrates the full browser-to-evidence-store pipeline:

1. Launch / connect to local Chrome via CDP
2. Navigate to a paper page, read content, detect PDF links
3. Download the PDF, validate it, store it in the evidence store
4. Return the ``document_id`` and structured source record

Progress events are emitted as typed :class:`TuiEvent` instances so the
TUI can display retrieval state without importing from this module.
"""

from __future__ import annotations

import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable
from urllib.error import URLError
from urllib.request import Request, urlopen

from .browser_retrieval import BrowserCDPClient, PageContent, PageLink
from .endpoint_canonicalizer import canonicalize_network_endpoint
from .errors import AssuranceError
from .evidence_store import (
    document_exists,
    get_document_path,
    store_pdf,
    store_source_record,
)
from .pdf_evidence import build_page_index, validate_pdf
from .utils import sha256_file, utc_now


# ── progress event types (TUI-compatible) ────────────────────────────────────


@dataclass
class RetrievalProgress:
    """Emitted at each stage of the retrieval pipeline.

    The TUI projector can consume these to update the ExplorerPane
    event groups and ContentPane status.
    """

    stage: str           # "launching", "connecting", "navigating", "reading",
                          # "finding_pdf", "downloading", "validating",
                          # "storing", "indexing", "done", "failed"
    message: str
    detail: str = ""
    timestamp: float = field(default_factory=time.time)


ProgressCallback = Callable[[RetrievalProgress], None]


# ── workflow result ──────────────────────────────────────────────────────────


@dataclass
class RetrievalResult:
    """Complete result of a retrieval workflow run."""

    document_id: str = ""
    source_url: str = ""
    title: str = ""
    doi: str = ""
    authors: list[str] = field(default_factory=list)
    year: int | None = None
    page_count: int = 0
    total_chars: int = 0
    pdf_candidates_found: int = 0
    evidence_level: str = "A"
    source_record_path: str = ""
    error: str = ""


# ── workflow ─────────────────────────────────────────────────────────────────


def run_retrieval(
    url: str,
    *,
    port: int = 9222,
    launch_browser: bool = True,
    browser: str = "chrome",
    headless: bool = False,
    on_progress: ProgressCallback | None = None,
    download_timeout: int = 60,
    max_pdf_bytes: int = 100 * 1024 * 1024,
) -> RetrievalResult:
    """Execute the full retrieval pipeline for a paper URL.

    Parameters
    ----------
    url:
        The URL of the paper page (DOI, publisher, or arXiv).
    port:
        CDP debugging port for Chrome.
    launch_browser:
        If ``True``, launch Chrome automatically.  If ``False``, connect
        to an already-running instance on *port*.
    browser:
        ``"chrome"`` or ``"edge"``.
    headless:
        If ``True``, run Chrome in headless mode (no visible window).
    on_progress:
        Optional callback receiving :class:`RetrievalProgress` events.
    download_timeout:
        Timeout in seconds for the PDF download.
    max_pdf_bytes:
        Maximum PDF file size to accept (default 100 MB).

    Returns
    -------
    :
        A :class:`RetrievalResult` with the document ID, metadata, and
        any error information.
    """
    result = RetrievalResult(source_url=url)
    client: BrowserCDPClient | None = None
    proc = None
    target_id: str = ""
    pdf_path: Path | None = None
    tmpdir: tempfile.TemporaryDirectory | None = None

    def _emit(stage: str, message: str, detail: str = "") -> None:
        if on_progress:
            on_progress(RetrievalProgress(stage=stage, message=message, detail=detail))

    try:
        # ── Step 1: Launch / connect browser ─────────────────────────────
        _emit("launching", "Starting browser for retrieval", url)
        client = BrowserCDPClient(port=port)

        if launch_browser:
            try:
                proc = BrowserCDPClient.launch_browser(
                    port=port, browser=browser, headless=headless,
                )
            except AssuranceError as exc:
                result.error = f"Browser launch failed: {exc}"
                _emit("failed", result.error)
                return result

        # Wait for browser to be ready
        for attempt in range(10):
            if client.connect():
                break
            time.sleep(0.5)
        else:
            result.error = (
                f"Cannot connect to browser on port {port}. "
                f"Start Chrome with: chrome --remote-debugging-port={port}"
            )
            _emit("failed", result.error)
            return result

        _emit("connecting", f"Connected to {browser} on port {port}")

        # ── Step 2: Navigate to paper page ──────────────────────────────
        _emit("navigating", f"Opening paper page", url)
        target_id = client.new_tab("about:blank", background=True)
        client.navigate(target_id, url)

        # ── Step 3: Read page content ───────────────────────────────────
        _emit("reading", "Extracting page content")
        content = client.read_page(target_id, max_chars=200_000)

        result.title = content.title
        result.pdf_candidates_found = len(content.pdf_candidates)

        # Extract metadata for richer source records
        meta = client.get_page_metadata(target_id)
        doi = _extract_doi(url, content, meta)
        if doi:
            result.doi = doi

        authors = _extract_authors(meta)
        if authors:
            result.authors = authors

        year = _extract_year(meta)
        if year:
            result.year = year

        _emit(
            "reading",
            f"Page read: {content.char_count} chars, "
            f"{len(content.links)} links, {len(content.pdf_candidates)} PDF candidates",
        )

        # ── Step 4: Find and download PDF ───────────────────────────────
        if not content.pdf_candidates:
            result.error = "No PDF links found on the page"
            _emit("failed", result.error)
            return result

        _emit("finding_pdf", f"Found {len(content.pdf_candidates)} PDF candidate(s)")

        pdf_url = content.pdf_candidates[0]
        _emit("downloading", f"Downloading PDF", pdf_url)

        tmpdir = tempfile.TemporaryDirectory()
        pdf_path = Path(tmpdir.name) / "paper.pdf"

        try:
            _download_file(pdf_url, pdf_path, timeout=download_timeout, max_bytes=max_pdf_bytes)
        except AssuranceError as exc:
            # Try next candidate if available
            if len(content.pdf_candidates) > 1:
                pdf_url = content.pdf_candidates[1]
                _emit("downloading", f"Retrying with second candidate", pdf_url)
                try:
                    _download_file(pdf_url, pdf_path, timeout=download_timeout, max_bytes=max_pdf_bytes)
                except AssuranceError as exc2:
                    result.error = f"PDF download failed: {exc2}"
                    _emit("failed", result.error)
                    return result
            else:
                result.error = f"PDF download failed: {exc}"
                _emit("failed", result.error)
                return result

        # ── Step 5: Validate PDF ────────────────────────────────────────
        _emit("validating", "Validating PDF")
        validation = validate_pdf(pdf_path)
        if not validation.valid_pdf:
            result.error = f"Downloaded file is not a valid PDF: {validation.status}"
            _emit("failed", result.error)
            return result
        if not validation.has_text_layer:
            _emit("validating", "PDF has no text layer — storing anyway", validation.status)

        # ── Step 6: Store in evidence store ─────────────────────────────
        _emit("storing", "Storing in evidence store")
        document_id = store_pdf(
            pdf_path,
            source_url=url,
            doi=doi,
            title=content.title,
            authors=authors,
            year=year,
        )
        result.document_id = document_id
        result.page_count = validation.page_count

        # ── Step 7: Build source record ─────────────────────────────────
        source_record = {
            "schema_version": "0.1.0-draft",
            "record_kind": "pdf_evidence_source_record",
            "source_id": f"src_{document_id[7:17]}",
            "task_id": f"retrieval_{utc_now()[:10].replace('-', '')}",
            "source_type": "paper_pdf",
            "evidence_level": "A" if validation.has_text_layer else "B",
            "title": content.title,
            "original_url": url,
            "final_url": pdf_url,
            "redirect_chain": [url] if url != pdf_url else [],
            "accessed_at": utc_now(),
            "doi": doi,
            "document_id": document_id,
            "work_id": f"doi:{doi}" if doi else "",
        }
        src_path = store_source_record(document_id, source_record)
        result.source_record_path = str(src_path)

        # ── Step 8: Index for reading ───────────────────────────────────
        _emit("indexing", "Building page index")
        page_index = build_page_index(pdf_path, document_id)
        result.total_chars = page_index.total_chars

        # Clean up temp PDF
        if tmpdir:
            tmpdir.cleanup()
            tmpdir = None
            pdf_path = None

        _emit(
            "done",
            f"Stored as {document_id}: {validation.page_count} pages, "
            f"{page_index.total_chars} chars",
        )
        return result

    except Exception as exc:
        result.error = f"Unexpected error: {exc}"
        _emit("failed", result.error)
        return result

    finally:
        # Cleanup
        if client and target_id:
            try:
                client.close_tab(target_id)
            except Exception:
                pass
        if client:
            client.disconnect()
        if proc:
            try:
                proc.terminate()
            except Exception:
                pass
        if tmpdir:
            try:
                tmpdir.cleanup()
            except Exception:
                pass


# ── helpers ──────────────────────────────────────────────────────────────────


def _download_file(
    url: str, dest: Path, timeout: int = 60, max_bytes: int = 100 * 1024 * 1024,
) -> None:
    """Download *url* to *dest* with safety limits."""
    try:
        canonicalize_network_endpoint(url)
    except AssuranceError:
        raise AssuranceError(f"Endpoint rejected by canonicalizer: {url}")

    req = Request(url, headers={"User-Agent": "GSA-Retrieval/0.1"})
    try:
        resp = urlopen(req, timeout=timeout)
    except URLError as exc:
        raise AssuranceError(f"Download failed: {exc}") from exc

    content_type = resp.headers.get("Content-Type", "")
    content_length = resp.headers.get("Content-Length")

    if content_length:
        size = int(content_length)
        if size > max_bytes:
            raise AssuranceError(
                f"PDF too large: {size} bytes (max {max_bytes})"
            )
        if size < 256:
            raise AssuranceError(
                f"File too small: {size} bytes — likely not a real PDF"
            )

    data = resp.read(max_bytes + 1)
    if len(data) > max_bytes:
        raise AssuranceError(f"PDF exceeds max size of {max_bytes} bytes")

    # Quick check: does it look like a PDF?
    if not data.startswith(b"%PDF"):
        # Might still be a valid PDF with a BOM or leading whitespace
        stripped = data.lstrip()
        if not stripped.startswith(b"%PDF"):
            raise AssuranceError(
                f"Downloaded file does not start with PDF signature "
                f"(Content-Type: {content_type})"
            )

    dest.write_bytes(data)


def _extract_doi(url: str, content: PageContent, meta: dict) -> str:
    """Extract DOI from metadata or URL."""
    # From citation metadata
    for cite in meta.get("citation", []):
        name = cite.get("name", "")
        if "doi" in name.lower():
            doi_val = cite.get("content", "")
            if doi_val:
                return doi_val

    # From URL
    import re
    doi_match = re.search(r'10\.\d{4,}/[^\s\'"<>?#]+', url)
    if doi_match:
        return doi_match.group(0)

    # From page text
    doi_match = re.search(r'DOI:\s*(10\.\d{4,}/[^\s]+)', content.text, re.IGNORECASE)
    if doi_match:
        return doi_match.group(1)

    # From meta tags
    for mt in meta.get("meta_tags", []):
        if mt.get("name") in ("citation_doi", "dc.identifier.doi", "DOI"):
            return mt.get("content", "")

    return ""


def _extract_authors(meta: dict) -> list[str]:
    """Extract author names from page metadata."""
    authors: list[str] = []

    # From citation metadata
    for cite in meta.get("citation", []):
        name = cite.get("name", "")
        if "author" in name.lower():
            content = cite.get("content", "")
            if content:
                authors.append(content)

    # From meta tags
    for mt in meta.get("meta_tags", []):
        if mt.get("name", "").startswith("citation_author"):
            content = mt.get("content", "")
            if content:
                authors.append(content)

    return authors


def _extract_year(meta: dict) -> int | None:
    """Extract publication year from metadata."""
    for mt in meta.get("meta_tags", []):
        if mt.get("name") in ("citation_publication_date", "citation_date", "dc.date"):
            content = mt.get("content", "")
            import re
            m = re.search(r"(\d{4})", content)
            if m:
                return int(m.group(1))

    for mt in meta.get("meta_tags", []):
        if mt.get("name") == "citation_year":
            try:
                return int(mt.get("content", ""))
            except (ValueError, TypeError):
                pass

    return None
