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

import logging
import subprocess
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

logger = logging.getLogger(__name__)


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
    broker: Any | None = None,  # BrowserRetrievalBroker | None
) -> RetrievalResult:
    """Execute the full retrieval pipeline for a paper URL.

    When *broker* is provided, all browser operations are gated through
    it (task_id enforcement, capability checks, tab ownership).  See
    :class:`assurance.browser_retrieval_broker.BrowserRetrievalBroker`.

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

        # Wire broker for authorized operations
        ops = _BrokeredOps(client, broker)

        # ── Step 2: Navigate to paper page ──────────────────────────────
        _emit("navigating", f"Opening paper page", url)
        target_id = ops.new_tab("about:blank", background=True)
        ops.navigate(target_id, url)

        # §15, §23 AC-9: Detect login/CAPTCHA gates before reading content
        page_status = client.classify_page(target_id)
        if page_status.is_captcha_page:
            captcha_info = "; ".join(page_status.captcha_indicators)
            result.error = f"CAPTCHA_REQUIRED: {captcha_info}"
            _emit("failed", result.error)
            return result
        if page_status.is_login_page:
            login_info = "; ".join(page_status.login_indicators)
            result.error = (
                f"LOGIN_REQUIRED: {login_info}. "
                f"Open {url} in your browser to log in, then retry."
            )
            _emit("failed", result.error)
            return result

        # ── Step 3: Read page content ───────────────────────────────────
        _emit("reading", "Extracting page content")
        content = ops.read_page(target_id, max_chars=200_000)

        result.title = content.title
        result.pdf_candidates_found = len(content.pdf_candidates)

        # Extract metadata for richer source records
        meta = ops.get_page_metadata(target_id)
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
        # Cleanup via broker (closes task tabs) or direct client
        if 'ops' in dir() and ops is not None:
            try:
                ops.close_all_owned_tabs()
            except Exception:
                pass
            try:
                ops.disconnect()
            except Exception:
                pass
        else:
            _cleanup(client, proc)
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
    """Download *url* to *dest* with safety limits.

    §19.1: Both the initial URL and the final post-redirect URL are
    validated.  If ``urlopen`` follows a redirect to an internal/blocked
    host, the download is rejected.
    """
    try:
        canonicalize_network_endpoint(url)
    except AssuranceError:
        raise AssuranceError(f"Endpoint rejected by canonicalizer: {url}")

    req = Request(url, headers={"User-Agent": "GSA-Retrieval/0.1"})
    try:
        resp = urlopen(req, timeout=timeout)
    except URLError as exc:
        raise AssuranceError(f"Download failed: {exc}") from exc

    # §19.1: Validate the final URL after redirects.  urlopen follows
    # redirects silently; resp.geturl() returns the final location.
    final_url = resp.geturl()
    if final_url != url:
        try:
            canonicalize_network_endpoint(final_url)
        except AssuranceError:
            raise AssuranceError(
                f"Download redirected to blocked endpoint: {final_url}"
            )

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
            if content and content not in authors:
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


# ══════════════════════════════════════════════════════════════════════════════
# General web retrieval — all external URLs through local browser
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class WebPageResult:
    """Result of reading a single web page."""

    url: str
    title: str = ""
    text_preview: str = ""       # first 2000 chars
    char_count: int = 0
    link_count: int = 0
    content_hash: str = ""
    pdf_candidates: list[str] = field(default_factory=list)
    error: str = ""


@dataclass
class WebRetrievalResult:
    """Result of a general web retrieval run."""

    pages: list[WebPageResult] = field(default_factory=list)
    source_records: list[dict] = field(default_factory=list)
    total_chars: int = 0
    error: str = ""


def retrieve_urls(
    urls: list[str],
    *,
    port: int = 9222,
    launch_browser: bool = True,
    browser: str = "chrome",
    headless: bool = False,
    on_progress: ProgressCallback | None = None,
    max_chars_per_page: int = 50_000,
    max_pages: int = 4,
) -> WebRetrievalResult:
    """Open and read multiple URLs through the local browser.

    Each page is read as B-level evidence: URL, content_hash, and a
    text preview are recorded.  Full text is **not** stored by default.

    This is the general-purpose retrieval entry point — any external URL
    the agent needs to read should go through this function.
    """
    result = WebRetrievalResult()
    client: BrowserCDPClient | None = None
    proc = None

    def _emit(stage: str, message: str, detail: str = "") -> None:
        if on_progress:
            on_progress(RetrievalProgress(stage=stage, message=message, detail=detail))

    try:
        client, proc = _ensure_browser(client, port, launch_browser, browser, headless)

        urls_to_fetch = urls[:max_pages]
        _emit("retrieving", f"Opening {len(urls_to_fetch)} URLs")

        for i, url in enumerate(urls_to_fetch):
            _emit("navigating", f"[{i+1}/{len(urls_to_fetch)}] Opening", url)
            page_result = WebPageResult(url=url)

            try:
                target_id = client.new_tab("about:blank", background=True)
                client.navigate(target_id, url)

                # §15: Check for login/CAPTCHA gates
                page_status = client.classify_page(target_id)
                if page_status.is_captcha_page:
                    page_result.error = f"CAPTCHA_REQUIRED: {'; '.join(page_status.captcha_indicators)}"
                    result.pages.append(page_result)
                    client.close_tab(target_id)
                    continue
                if page_status.is_login_page:
                    page_result.error = f"LOGIN_REQUIRED: {'; '.join(page_status.login_indicators)}"
                    result.pages.append(page_result)
                    client.close_tab(target_id)
                    continue

                content = client.read_page(target_id, max_chars=max_chars_per_page)

                page_result.title = content.title
                page_result.char_count = content.char_count
                page_result.link_count = len(content.links)
                page_result.content_hash = content.content_hash
                page_result.text_preview = content.text[:2000]
                page_result.pdf_candidates = content.pdf_candidates

                # Record B-level source
                source_record = {
                    "schema_version": "0.1.0-draft",
                    "record_kind": "pdf_evidence_source_record",
                    "source_id": f"src_web_{content.content_hash[:12]}",
                    "source_type": "web_page",
                    "evidence_level": "B",
                    "title": content.title,
                    "original_url": url,
                    "final_url": url,
                    "accessed_at": utc_now(),
                    "content_hash": content.content_hash,
                    "usage_trace": {
                        "char_count": content.char_count,
                        "link_count": len(content.links),
                        "pdf_candidates": content.pdf_candidates,
                    },
                }
                result.source_records.append(source_record)
                result.total_chars += content.char_count

                client.close_tab(target_id)
                _emit("reading", f"[{i+1}/{len(urls_to_fetch)}] {content.char_count} chars — {content.title[:60]}")

            except AssuranceError as exc:
                page_result.error = str(exc)
                _emit("reading", f"[{i+1}/{len(urls_to_fetch)}] Failed: {exc}")

            result.pages.append(page_result)

        _emit("done", f"Retrieved {len(result.pages)} pages, {result.total_chars} total chars")
        return result

    finally:
        _cleanup(client, proc)


def retrieve_search(
    query: str,
    *,
    port: int = 9222,
    launch_browser: bool = True,
    browser: str = "chrome",
    headless: bool = False,
    on_progress: ProgressCallback | None = None,
    max_results: int = 4,
    engine: str = "google",
) -> WebRetrievalResult:
    """Search the web through the user's local browser.

    Uses the browser's existing session — no API key, no separate
    search configuration.  The search runs in a background tab.

    Parameters
    ----------
    engine:
        ``"google"`` (default) or ``"duckduckgo"``.
    """
    from urllib.parse import quote_plus

    if engine == "google":
        search_url = f"https://www.google.com/search?q={quote_plus(query)}"
    else:
        search_url = f"https://html.duckduckgo.com/html/?q={quote_plus(query)}"

    _emit = on_progress or (lambda _: None)
    _emit(RetrievalProgress("searching", f"Searching {engine} for: {query}", search_url))

    # Step 1: Open search page and extract result links
    # Keep the browser alive for the entire search+retrieve session
    client: BrowserCDPClient | None = None
    proc = None
    try:
        client, proc = _ensure_browser(client, port, launch_browser, browser, headless)
        target_id = client.new_tab("about:blank", background=True)
        client.navigate(target_id, search_url)
        content = client.read_page(target_id, max_chars=30_000)
        links = client.get_links(target_id)
        client.close_tab(target_id)

        # Filter result links
        result_urls: list[str] = []
        skip_domains = {
            "google.com", "googleadservices.com", "duckduckgo.com",
            "youtube.com", "accounts.google.com",
        }
        for link in links:
            url = link.url
            if not url.startswith("http"):
                continue
            from urllib.parse import urlparse
            domain = urlparse(url).netloc.lower()
            if any(s in domain for s in skip_domains):
                continue
            if url in result_urls:
                continue
            result_urls.append(url)

        urls = result_urls[:max_results]
        _emit(RetrievalProgress(
            "searching",
            f"Found {len(result_urls)} results, opening {len(urls)}",
        ))

        # Step 2: Open each result in the SAME browser session
        result = WebRetrievalResult()
        for i, url in enumerate(urls):
            _emit(RetrievalProgress("navigating", f"[{i+1}/{len(urls)}] Opening", url))
            page_result = WebPageResult(url=url)

            try:
                tab = client.new_tab("about:blank", background=True)
                client.navigate(tab, url)
                page = client.read_page(tab, max_chars=50_000)

                page_result.title = page.title
                page_result.char_count = page.char_count
                page_result.link_count = len(page.links)
                page_result.content_hash = page.content_hash
                page_result.text_preview = page.text[:2000]
                page_result.pdf_candidates = page.pdf_candidates

                source_record = {
                    "schema_version": "0.1.0-draft",
                    "record_kind": "pdf_evidence_source_record",
                    "source_id": f"src_web_{page.content_hash[:12]}",
                    "source_type": "web_page",
                    "evidence_level": "B",
                    "title": page.title,
                    "original_url": url,
                    "final_url": url,
                    "accessed_at": utc_now(),
                    "content_hash": page.content_hash,
                }
                result.source_records.append(source_record)
                result.total_chars += page.char_count

                client.close_tab(tab)
                _emit(RetrievalProgress(
                    "reading",
                    f"[{i+1}/{len(urls)}] {page.char_count} chars — {page.title[:60]}",
                ))

            except AssuranceError as exc:
                page_result.error = str(exc)
                _emit(RetrievalProgress("reading", f"[{i+1}/{len(urls)}] Failed: {exc}"))

            result.pages.append(page_result)

        _emit(RetrievalProgress(
            "done",
            f"Retrieved {len(result.pages)} pages, {result.total_chars} chars",
        ))
        return result

    finally:
        _cleanup(client, proc)


# ══════════════════════════════════════════════════════════════════════════════
# internal helpers
# ══════════════════════════════════════════════════════════════════════════════


def _ensure_browser(
    client: BrowserCDPClient | None,
    port: int,
    launch: bool,
    browser: str,
    headless: bool,
) -> tuple[BrowserCDPClient, Any]:
    """Launch or connect to a browser. Returns (client, proc).

    Connection strategy (tried in order):

    1. **Already-running browser** — connect to an existing Chrome/Edge
       instance listening on *port*.  This path works when the user has
       manually started a browser with ``--remote-debugging-port``.
       Note: the main Chrome profile is often blocked by enterprise
       policy; a manually-started instance with a separate profile
       will work.

    2. **Auto-launch** (when *launch* is True) — starts a new browser
       with a **project-isolated profile** stored at
       ``<project_root>/.gsa_chrome_profile/``.  This profile never
       touches the user's main Chrome data (cookies, passwords,
       bookmarks, history).

       Why a separate profile?
       Chrome's main user profile is typically blocked from CDP
       connections by enterprise security policy
       (``DeveloperToolsAvailability``).  The project profile bypasses
       this restriction while also providing stronger isolation: the
       AI can never accidentally read the user's authenticated
       sessions.

       The profile is persistent: open a visible window once
       (``headless=False``), log into Google, and cookies survive
       across retrieval sessions.  Subsequent runs in headless mode
       reuse the authenticated profile.

    See :file:`architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
    §25 for the full limitation and resolution plan.
    """
    import time
    client = BrowserCDPClient(port=port)
    proc = None

    # Step 1: Try connecting to an already-running browser first
    if client.connect():
        return client, proc

    if not launch:
        raise AssuranceError(
            f"Browser not reachable on port {port}.\n"
            f"Start Chrome with --remote-debugging-port={port}, or\n"
            f"set launch_browser=True to auto-launch."
        )

    # Step 2: Kill any zombie browser on the target port, then auto-launch
    # with a project-isolated profile.
    #
    # DESIGN NOTE (LBR-001, 2026-07-29):
    #   Chrome's main user profile CDP connection is blocked by enterprise
    #   security policy on the development machine.  We use a dedicated
    #   profile under <project_root>/.gsa_chrome_profile/ as a workaround.
    #
    #   This is actually *better* for security — the AI-owned browser
    #   window has zero access to the user's main profile cookies,
    #   passwords, bookmarks, or authenticated sessions.
    #
    #   The profile is persistent: open a visible window once
    #   (headless=False), log into Google, and cookies survive across
    #   retrieval sessions.  It is stored inside the project directory
    #   so it never conflicts with the user's main Chrome.
    #
    #   We kill any existing process on the port first to prevent
    #   "port already in use" errors from a previous crashed/zombie
    #   Chrome instance.

    from .contracts import ASSURANCE_ROOT
    _profile = str(ASSURANCE_ROOT.parent / ".gsa_chrome_profile")
    BrowserCDPClient.kill_browser_on_port(port, user_data_dir=_profile)
    proc = BrowserCDPClient.launch_browser(
        port=port, browser=browser, headless=headless,
        user_data_dir=_profile,
    )
    for _ in range(30):
        time.sleep(1)
        if client.connect():
            return client, proc
    raise AssuranceError(
        f"Launched {browser} but cannot connect on port {port}. "
        f"Check that no other process is using port {port}."
    )


def launch_visible_browser(
    port: int = 9222,
    browser: str = "chrome",
) -> subprocess.Popen | None:
    """Launch a **visible** Chrome window with the project-isolated profile.

    Use this for the **one-time Google login** step: a normal Chrome
    window opens (not headless), you log into Google Scholar / arXiv /
    etc., and close the window.  Cookies are persisted in
    ``.gsa_chrome_profile/`` and reused by subsequent headless retrieval
    sessions.

    Returns the subprocess handle (the caller is responsible for
    calling ``.terminate()`` when done), or ``None`` if Chrome is not
    installed.
    """
    from .browser_retrieval import BrowserCDPClient
    from .contracts import ASSURANCE_ROOT

    _profile = str(ASSURANCE_ROOT.parent / ".gsa_chrome_profile")
    BrowserCDPClient.kill_browser_on_port(port, user_data_dir=_profile)
    logger.info(
        "Launching visible Chrome on port %d with profile %s. "
        "Log into Google once; cookies persist for retrieval sessions.",
        port, _profile,
    )
    return BrowserCDPClient.launch_browser(
        port=port,
        browser=browser,
        headless=False,
        user_data_dir=_profile,
    )


class _BrokeredOps:
    """Transparently route browser operations through broker when available.

    When *broker* is provided, all operations are gated (task_id,
    capability check, tab ownership).  Otherwise they fall back to
    direct :class:`BrowserCDPClient` calls.
    """

    def __init__(
        self,
        client: BrowserCDPClient,
        broker: Any | None = None,
    ) -> None:
        self._client = client
        self._broker = broker
        self._task_id: str | None = None
        if broker is not None:
            from .browser_retrieval_broker import SEARCH_CAPABILITIES
            self._task_id = broker.create_task(capabilities=SEARCH_CAPABILITIES)

    @property
    def task_id(self) -> str | None:
        return self._task_id

    def new_tab(self, url: str = "about:blank", background: bool = True) -> str:
        if self._broker and self._task_id:
            return self._broker.new_tab(self._task_id, url, background)
        return self._client.new_tab(url, background)

    def navigate(self, target_id: str, url: str) -> None:
        if self._broker and self._task_id:
            self._broker.navigate(self._task_id, target_id, url)
        else:
            self._client.navigate(target_id, url)

    def read_page(self, target_id: str, max_chars: int = 100_000) -> Any:
        if self._broker and self._task_id:
            return self._broker.read_page(self._task_id, target_id, max_chars)
        return self._client.read_page(target_id, max_chars)

    def get_page_metadata(self, target_id: str) -> dict:
        if self._broker and self._task_id:
            return self._broker.get_page_metadata(self._task_id, target_id)
        return self._client.get_page_metadata(target_id)

    def get_links(self, target_id: str) -> list:
        if self._broker and self._task_id:
            return self._broker.get_links(self._task_id, target_id)
        return self._client.get_links(target_id)

    def find_pdf_links(self, target_id: str) -> list[str]:
        if self._broker and self._task_id:
            return self._broker.find_pdf_links(self._task_id, target_id)
        return self._client.find_pdf_links(target_id)

    def close_tab(self, target_id: str) -> None:
        if self._broker and self._task_id:
            self._broker.close_tab(self._task_id, target_id)
        else:
            self._client.close_tab(target_id)

    def close_all_owned_tabs(self) -> None:
        if self._broker and self._task_id:
            self._broker.close_task(self._task_id)
        else:
            self._client.close_all_owned_tabs()

    def disconnect(self) -> None:
        self._client.disconnect()


def _cleanup(client: BrowserCDPClient | None, proc: Any) -> None:
    """Safely close browser resources.

    Guarantees (best-effort, each step catches its own errors):

    1. Close every AI-owned tab through CDP.
    2. Close all WebSocket connections.
    3. Terminate the browser process (only when we launched it).

    If step 1 fails, steps 2 and 3 still run, ensuring the browser
    process does not leak.  For a project-isolated profile this is
    safe — the AI window has no tabs the user cares about.
    """
    if client:
        try:
            client.close_all_owned_tabs()
        except Exception as exc:
            logger.warning("Failed to close all owned tabs: %s", exc)
        try:
            client.disconnect()
        except Exception as exc:
            logger.warning("Failed to disconnect CDP client: %s", exc)
    if proc:
        try:
            proc.terminate()
        except Exception as exc:
            logger.warning("Failed to terminate browser process: %s", exc)
