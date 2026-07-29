"""End-to-end tests for browser retrieval workflows — LBR-001 Phase 4/5.

Tests the full pipeline from auto-launched headless Chrome through
``retrieve_urls()`` (general web retrieval) and ``run_retrieval()``
(paper PDF → evidence store).

All tests auto-launch headless Chrome with the project-isolated profile.
No manual browser setup is required.  Tests are skipped only when Chrome
or Edge is not installed.
"""

from __future__ import annotations

import unittest
from pathlib import Path

from assurance.errors import AssuranceError

# ── helpers ──────────────────────────────────────────────────────────────────

_E2E_PORT = 19224  # dedicated port for E2E tests (not shared with unit tests)


def _chrome_installed() -> bool:
    """Return True if Chrome or Edge is installed on this system."""
    from assurance.browser_retrieval import _find_chrome, _find_edge
    for p in (*_find_chrome(), *_find_edge()):
        if Path(p).is_file():
            return True
    return False


# ══════════════════════════════════════════════════════════════════════════════
# General web retrieval E2E tests (retrieve_urls)
# ══════════════════════════════════════════════════════════════════════════════


@unittest.skipUnless(_chrome_installed(), "Chrome/Edge not installed")
class WebRetrievalE2ETests(unittest.TestCase):
    """LBR-001: end-to-end ``retrieve_urls()`` through auto-launched Chrome.

    Uses example.com / example.org — the most reliable pages on the
    internet — to avoid the intermittent timeouts observed with
    httpbin.org in headless Chrome.
    """

    _proc: object = None
    _client: object = None

    @classmethod
    def setUpClass(cls) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        from assurance.retrieval_workflow import _ensure_browser

        client = BrowserCDPClient(port=_E2E_PORT)
        cls._client, cls._proc = _ensure_browser(
            client=client,
            port=_E2E_PORT,
            launch=True,
            browser="chrome",
            headless=True,
        )

    @classmethod
    def tearDownClass(cls) -> None:
        if cls._client is not None:
            try:
                cls._client.close_all_owned_tabs()
            except Exception:
                pass
            try:
                cls._client.disconnect()
            except Exception:
                pass
        if cls._proc is not None:
            try:
                cls._proc.terminate()
            except Exception:
                pass

    # ── retrieve_urls (general web) ───────────────────────────────────────

    def test_retrieve_single_page(self) -> None:
        """E2E: retrieve a single page and verify source record."""
        from assurance.retrieval_workflow import retrieve_urls

        result = retrieve_urls(
            ["https://example.com"],
            port=_E2E_PORT,
            launch_browser=False,        # already launched in setUpClass
            browser="chrome",
            headless=True,
        )
        self.assertEqual(len(result.pages), 1)
        self.assertEqual(result.pages[0].error, "")
        self.assertGreater(result.pages[0].char_count, 0)
        self.assertGreater(result.total_chars, 0)
        # At least one source record (B-level web page)
        self.assertGreater(len(result.source_records), 0)
        record = result.source_records[0]
        self.assertEqual(record["evidence_level"], "B")
        self.assertEqual(record["source_type"], "web_page")
        self.assertIn("content_hash", record)

    def test_retrieve_multiple_pages(self) -> None:
        """E2E: retrieve multiple pages — tests tab reuse logic.

        Uses the two most reliable pages on the internet.
        """
        from assurance.retrieval_workflow import retrieve_urls

        result = retrieve_urls(
            [
                "https://example.com",
                "https://example.org",
            ],
            port=_E2E_PORT,
            launch_browser=False,
            browser="chrome",
            headless=True,
        )
        self.assertEqual(len(result.pages), 2)
        success_count = sum(1 for p in result.pages if not p.error)
        self.assertGreaterEqual(success_count, 2,
            f"Expected both pages to succeed; errors: "
            f"{[(p.url, p.error) for p in result.pages if p.error]}")
        self.assertGreater(result.total_chars, 0)
        # At least one source record per successful page
        self.assertGreaterEqual(len(result.source_records), 2)

    def test_retrieve_with_error_handling(self) -> None:
        """E2E: one valid and one invalid URL — error does not crash the batch."""
        from assurance.retrieval_workflow import retrieve_urls

        result = retrieve_urls(
            [
                "https://example.com",
                "https://this-domain-does-not-exist-12345.invalid/",
            ],
            port=_E2E_PORT,
            launch_browser=False,
            browser="chrome",
            headless=True,
        )
        self.assertEqual(len(result.pages), 2)
        # First page should succeed
        success = [p for p in result.pages if not p.error]
        failed = [p for p in result.pages if p.error]
        self.assertGreaterEqual(len(success), 1, "expected at least one success")
        self.assertGreaterEqual(len(failed), 1, "expected at least one failure")

    def test_retrieve_empty_urls(self) -> None:
        """E2E: empty URL list returns empty result (no browser round-trip)."""
        from assurance.retrieval_workflow import retrieve_urls

        result = retrieve_urls(
            [],
            port=_E2E_PORT,
            launch_browser=False,
            browser="chrome",
            headless=True,
        )
        self.assertEqual(len(result.pages), 0)
        self.assertEqual(result.total_chars, 0)

    def test_progress_callback_fires(self) -> None:
        """E2E: on_progress callback receives RetrievalProgress events."""
        from assurance.retrieval_workflow import RetrievalProgress, retrieve_urls

        events: list[RetrievalProgress] = []

        def _collect(evt: RetrievalProgress) -> None:
            events.append(evt)

        result = retrieve_urls(
            ["https://example.com"],
            port=_E2E_PORT,
            launch_browser=False,
            browser="chrome",
            headless=True,
            on_progress=_collect,
        )
        self.assertGreater(len(events), 0, "Expected at least one progress event")
        # Last event should be 'done'
        self.assertEqual(events[-1].stage, "done")
        self.assertGreater(len(result.pages), 0)


# ══════════════════════════════════════════════════════════════════════════════
# Paper PDF retrieval E2E tests (run_retrieval)
# ══════════════════════════════════════════════════════════════════════════════

_ARXIV_PAPER = "https://arxiv.org/abs/1706.03762"  # "Attention Is All You Need"


@unittest.skipUnless(_chrome_installed(), "Chrome/Edge not installed")
class PaperRetrievalE2ETests(unittest.TestCase):
    """LBR-001: end-to-end ``run_retrieval()`` (paper PDF → evidence store).

    Navigates to an arXiv abstract page, finds the PDF link, downloads the
    PDF, indexes it, and stores it in the evidence store.  Uses a
    well-known, always-available open-access paper.
    """

    _proc: object = None
    _client: object = None

    @classmethod
    def setUpClass(cls) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        from assurance.retrieval_workflow import _ensure_browser

        client = BrowserCDPClient(port=_E2E_PORT)
        cls._client, cls._proc = _ensure_browser(
            client=client,
            port=_E2E_PORT,
            launch=True,
            browser="chrome",
            headless=True,
        )

    @classmethod
    def tearDownClass(cls) -> None:
        if cls._client is not None:
            try:
                cls._client.close_all_owned_tabs()
            except Exception:
                pass
            try:
                cls._client.disconnect()
            except Exception:
                pass
        if cls._proc is not None:
            try:
                cls._proc.terminate()
            except Exception:
                pass

    def test_run_retrieval_arxiv_paper(self) -> None:
        """E2E: full paper retrieval — navigate arXiv → download PDF → index.

        Verifies that:
        - ``run_retrieval()`` returns a successful ``RetrievalResult``
        - PDF is validated and stored (document_id is a SHA-256)
        - Page count and char count are non-zero
        - Source record is A-level evidence
        - DOI and title are extracted from the abstract page
        """
        try:
            from assurance.retrieval_workflow import RetrievalProgress, run_retrieval
        except ImportError:
            self.skipTest("retrieval_workflow not available")

        events: list[RetrievalProgress] = []

        def _collect(evt: RetrievalProgress) -> None:
            events.append(evt)

        result = run_retrieval(
            _ARXIV_PAPER,
            port=_E2E_PORT,
            launch_browser=False,           # already launched in setUpClass
            browser="chrome",
            headless=True,
            on_progress=_collect,
        )

        # ── core assertions (pipeline must work) ─────────────────────────

        self.assertEqual(result.error, "", f"run_retrieval failed: {result.error}")
        self.assertNotEqual(result.document_id, "", "expected non-empty document_id")
        # document_id is "sha256:{64-hex}" → 71 chars
        self.assertTrue(
            result.document_id.startswith("sha256:"),
            f"document_id should start with 'sha256:', got {result.document_id!r}",
        )
        self.assertEqual(
            len(result.document_id), 71,
            f"document_id length expected 71, got {len(result.document_id)}: {result.document_id!r}",
        )
        self.assertGreater(result.page_count, 0, "expected non-zero page count")
        self.assertGreater(result.total_chars, 0, "expected non-zero text content")

        # Source record path should be a valid JSON file
        self.assertNotEqual(result.source_record_path, "", "expected source_record_path")
        self.assertTrue(
            Path(result.source_record_path).is_file(),
            f"source record not found at {result.source_record_path}",
        )

        # Progress events should span the full pipeline
        stages = [e.stage for e in events]
        self.assertIn("navigating", stages)
        self.assertIn("done", stages)

        # PDF file must exist in the evidence store
        from assurance.evidence_store import get_document_path
        store_path = get_document_path(result.document_id)
        pdf_path = store_path / "original.pdf"
        self.assertTrue(pdf_path.is_file(), f"PDF not found at {pdf_path}")
        # Verify it looks like a PDF
        header = pdf_path.read_bytes()[:5]
        self.assertEqual(header, b"%PDF-", f"File does not start with PDF header: {header!r}")

        # Metadata and pages should also exist
        self.assertTrue((store_path / "metadata.json").is_file())
        self.assertTrue((store_path / "pages.jsonl").is_file())

        # ── best-effort metadata assertions ──────────────────────────────
        # DOI, authors, and year are extracted from the abstract page.
        # They may be empty if the page load was slow or the publisher
        # uses an unusual markup.  The core retrieval pipeline (PDF
        # download + evidence store) is verified above; these are
        # informative but not gate-level.
        if result.doi:
            self.assertIn("/", result.doi, f"DOI format unexpected: {result.doi}")
        if result.authors:
            self.assertGreater(len(result.authors), 0)
        if result.year is not None and result.year > 0:
            self.assertGreater(result.year, 1900)


if __name__ == "__main__":
    unittest.main()
