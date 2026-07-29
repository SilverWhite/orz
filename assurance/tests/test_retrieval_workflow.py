"""Tests for retrieval workflow — LBR-001 Phase 3.

Covers :mod:`assurance.retrieval_workflow`: dataclass construction,
DOI/author/year extraction, download validation, browser connection
logic, and cleanup safety.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

from assurance.errors import AssuranceError


# ══════════════════════════════════════════════════════════════════════════════
# Dataclass tests
# ══════════════════════════════════════════════════════════════════════════════


class RetrievalProgressTests(unittest.TestCase):
    """LBR-001: RetrievalProgress dataclass construction."""

    def test_minimal_construction(self) -> None:
        from assurance.retrieval_workflow import RetrievalProgress
        rp = RetrievalProgress(stage="searching", message="Looking up...")
        self.assertEqual(rp.stage, "searching")
        self.assertEqual(rp.message, "Looking up...")
        self.assertEqual(rp.detail, "")
        self.assertGreater(rp.timestamp, 0)

    def test_with_detail(self) -> None:
        from assurance.retrieval_workflow import RetrievalProgress
        rp = RetrievalProgress(stage="downloading", message="Fetching PDF", detail="https://example.com/paper.pdf")
        self.assertEqual(rp.detail, "https://example.com/paper.pdf")


class RetrievalResultTests(unittest.TestCase):
    """LBR-001: RetrievalResult dataclass construction."""

    def test_defaults(self) -> None:
        from assurance.retrieval_workflow import RetrievalResult
        rr = RetrievalResult(source_url="https://example.com")
        self.assertEqual(rr.source_url, "https://example.com")
        self.assertEqual(rr.document_id, "")
        self.assertEqual(rr.error, "")
        self.assertEqual(rr.page_count, 0)
        self.assertEqual(rr.total_chars, 0)
        self.assertEqual(rr.authors, [])

    def test_error_result(self) -> None:
        from assurance.retrieval_workflow import RetrievalResult
        rr = RetrievalResult(source_url="https://example.com", error="PDF not found")
        self.assertEqual(rr.error, "PDF not found")


class WebRetrievalResultTests(unittest.TestCase):
    """LBR-001: WebPageResult and WebRetrievalResult construction."""

    def test_web_page_result_defaults(self) -> None:
        from assurance.retrieval_workflow import WebPageResult
        pr = WebPageResult(url="https://example.com")
        self.assertEqual(pr.url, "https://example.com")
        self.assertEqual(pr.char_count, 0)
        self.assertEqual(pr.link_count, 0)
        self.assertEqual(pr.error, "")

    def test_web_page_result_with_error(self) -> None:
        from assurance.retrieval_workflow import WebPageResult
        pr = WebPageResult(url="https://blocked.local", error="Host blocked")
        self.assertEqual(pr.error, "Host blocked")

    def test_web_retrieval_result_defaults(self) -> None:
        from assurance.retrieval_workflow import WebRetrievalResult
        wrr = WebRetrievalResult()
        self.assertEqual(wrr.pages, [])
        self.assertEqual(wrr.total_chars, 0)
        self.assertEqual(wrr.error, "")


# ══════════════════════════════════════════════════════════════════════════════
# DOI / author / year extraction tests
# ══════════════════════════════════════════════════════════════════════════════


class ExtractDoiTests(unittest.TestCase):
    """LBR-001: DOI extraction from various sources."""

    def _extract(self, url: str = "https://example.com", text: str = "", meta: dict | None = None) -> str:
        from assurance.retrieval_workflow import _extract_doi
        from assurance.browser_retrieval import PageContent
        content = PageContent(url=url, text=text)
        return _extract_doi(url, content, meta or {})

    def test_doi_from_url(self) -> None:
        doi = self._extract(url="https://doi.org/10.1234/example.paper")
        self.assertIn("10.1234/example.paper", doi)

    def test_doi_from_url_with_query(self) -> None:
        doi = self._extract(url="https://arxiv.org/abs/2605.15058")
        self.assertEqual(doi, "")  # no DOI in arXiv URL

    def test_doi_from_page_text(self) -> None:
        doi = self._extract(text="DOI: 10.5678/foo.bar Methods section...")
        self.assertIn("10.5678/foo.bar", doi)

    def test_doi_from_meta_tags(self) -> None:
        doi = self._extract(meta={
            "meta_tags": [
                {"name": "citation_doi", "content": "10.9999/test.1"},
            ],
        })
        self.assertEqual(doi, "10.9999/test.1")

    def test_doi_from_citation_metadata(self) -> None:
        doi = self._extract(meta={
            "citation": [
                {"name": "citation_doi", "content": "10.8888/test.2"},
            ],
        })
        self.assertEqual(doi, "10.8888/test.2")

    def test_no_doi_returns_empty(self) -> None:
        doi = self._extract(url="https://example.com/not-a-paper")
        self.assertEqual(doi, "")


class ExtractAuthorsTests(unittest.TestCase):
    """LBR-001: Author extraction from metadata."""

    def test_authors_from_citation_metadata(self) -> None:
        from assurance.retrieval_workflow import _extract_authors
        authors = _extract_authors({
            "citation": [
                {"name": "citation_author", "content": "Alice Smith"},
                {"name": "citation_author", "content": "Bob Jones"},
            ],
        })
        self.assertEqual(authors, ["Alice Smith", "Bob Jones"])

    def test_authors_from_meta_tags(self) -> None:
        from assurance.retrieval_workflow import _extract_authors
        authors = _extract_authors({
            "meta_tags": [
                {"name": "citation_author", "content": "Carol Wang"},
                {"name": "citation_author", "content": "Carol Wang"},  # duplicate
            ],
        })
        self.assertEqual(authors, ["Carol Wang"])

    def test_no_authors_returns_empty(self) -> None:
        from assurance.retrieval_workflow import _extract_authors
        authors = _extract_authors({})
        self.assertEqual(authors, [])


class ExtractYearTests(unittest.TestCase):
    """LBR-001: Publication year extraction from metadata."""

    def test_year_from_date_tag(self) -> None:
        from assurance.retrieval_workflow import _extract_year
        year = _extract_year({
            "meta_tags": [
                {"name": "citation_publication_date", "content": "2025-06-15"},
            ],
        })
        self.assertEqual(year, 2025)

    def test_year_from_year_tag(self) -> None:
        from assurance.retrieval_workflow import _extract_year
        year = _extract_year({
            "meta_tags": [
                {"name": "citation_year", "content": "2024"},
            ],
        })
        self.assertEqual(year, 2024)

    def test_no_year_returns_none(self) -> None:
        from assurance.retrieval_workflow import _extract_year
        year = _extract_year({})
        self.assertIsNone(year)

    def test_invalid_year_returns_none(self) -> None:
        from assurance.retrieval_workflow import _extract_year
        year = _extract_year({
            "meta_tags": [
                {"name": "citation_year", "content": "not-a-year"},
            ],
        })
        self.assertIsNone(year)


# ══════════════════════════════════════════════════════════════════════════════
# Download validation tests
# ══════════════════════════════════════════════════════════════════════════════


class DownloadFileTests(unittest.TestCase):
    """LBR-001: _download_file validation and safety checks."""

    def test_blocked_endpoint_raises(self) -> None:
        """Non-HTTP schemes are rejected by the endpoint canonicalizer."""
        from assurance.retrieval_workflow import _download_file
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            dest = Path(f.name)
        try:
            with self.assertRaises(AssuranceError) as ctx:
                _download_file("file:///C:/windows/system32/not-a-file.pdf", dest)
            # Either "Endpoint rejected" (canonicalizer) or "Download failed" (urlopen on file://)
            err = str(ctx.exception)
            self.assertTrue(
                "Endpoint" in err or "Download failed" in err or "Blocked" in err,
                f"Unexpected error message: {err}",
            )
        finally:
            dest.unlink(missing_ok=True)

    def test_file_too_small_raises(self) -> None:
        """Content-Length < 256 bytes should be rejected."""
        from assurance.retrieval_workflow import _download_file
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            dest = Path(f.name)
        try:
            # Mock urlopen to return a tiny response
            with patch("assurance.retrieval_workflow.urlopen") as mock_open:
                mock_resp = MagicMock()
                mock_resp.headers = {"Content-Type": "application/pdf", "Content-Length": "100"}
                mock_resp.read.return_value = b"%PDF-1.4\n%tiny"
                mock_resp.geturl.return_value = "https://example.com/tiny.pdf"
                mock_open.return_value = mock_resp

                with self.assertRaises(AssuranceError) as ctx:
                    _download_file("https://example.com/tiny.pdf", dest)
                self.assertIn("too small", str(ctx.exception))
        finally:
            dest.unlink(missing_ok=True)

    def test_non_pdf_content_raises(self) -> None:
        """Response that doesn't start with %PDF should be rejected."""
        from assurance.retrieval_workflow import _download_file
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            dest = Path(f.name)
        try:
            with patch("assurance.retrieval_workflow.urlopen") as mock_open:
                mock_resp = MagicMock()
                mock_resp.headers = {"Content-Type": "text/html"}
                mock_resp.read.return_value = b"<html><body>Login page</body></html>"
                mock_resp.geturl.return_value = "https://example.com/fake.pdf"
                mock_open.return_value = mock_resp

                with self.assertRaises(AssuranceError) as ctx:
                    _download_file("https://example.com/fake.pdf", dest)
                self.assertIn("PDF signature", str(ctx.exception))
        finally:
            dest.unlink(missing_ok=True)

    def test_file_too_large_raises(self) -> None:
        """Content-Length exceeding max_bytes should be rejected."""
        from assurance.retrieval_workflow import _download_file
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            dest = Path(f.name)
        try:
            with patch("assurance.retrieval_workflow.urlopen") as mock_open:
                mock_resp = MagicMock()
                mock_resp.headers = {"Content-Type": "application/pdf", "Content-Length": "200000000"}
                mock_resp.geturl.return_value = "https://example.com/huge.pdf"
                mock_open.return_value = mock_resp

                with self.assertRaises(AssuranceError) as ctx:
                    _download_file("https://example.com/huge.pdf", dest, max_bytes=50_000_000)
                self.assertIn("too large", str(ctx.exception))
        finally:
            dest.unlink(missing_ok=True)

    def test_valid_pdf_download_succeeds(self) -> None:
        """A valid PDF response should write to disk."""
        from assurance.retrieval_workflow import _download_file
        with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
            dest = Path(f.name)
        try:
            pdf_bytes = b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\n%%EOF"
            with patch("assurance.retrieval_workflow.urlopen") as mock_open:
                mock_resp = MagicMock()
                mock_resp.headers = {"Content-Type": "application/pdf"}
                mock_resp.read.return_value = pdf_bytes
                mock_resp.geturl.return_value = "https://arxiv.org/pdf/2605.15058.pdf"
                mock_open.return_value = mock_resp

                _download_file("https://arxiv.org/pdf/2605.15058.pdf", dest)
                self.assertTrue(dest.exists())
                self.assertEqual(dest.read_bytes(), pdf_bytes)
        finally:
            dest.unlink(missing_ok=True)


# ══════════════════════════════════════════════════════════════════════════════
# Browser connection and cleanup tests
# ══════════════════════════════════════════════════════════════════════════════


class EnsureBrowserTests(unittest.TestCase):
    """LBR-001: _ensure_browser connection strategy."""

    def test_connects_to_running_browser_first(self) -> None:
        """When a browser is already running, connect without launching."""
        from assurance.retrieval_workflow import _ensure_browser
        with patch("assurance.retrieval_workflow.BrowserCDPClient") as MockClient:
            mock_instance = MockClient.return_value
            mock_instance.connect.return_value = True  # already running

            client, proc = _ensure_browser(None, 9222, True, "chrome", False)

            mock_instance.connect.assert_called_once()
            MockClient.assert_called_once_with(port=9222)
            self.assertIsNone(proc)  # no new process

    def test_raises_when_no_browser_and_launch_false(self) -> None:
        from assurance.retrieval_workflow import _ensure_browser
        with patch("assurance.retrieval_workflow.BrowserCDPClient") as MockClient:
            mock_instance = MockClient.return_value
            mock_instance.connect.return_value = False  # nothing running

            with self.assertRaises(AssuranceError) as ctx:
                _ensure_browser(None, 9222, launch=False, browser="chrome", headless=False)
            self.assertIn("not reachable", str(ctx.exception))


class CleanupTests(unittest.TestCase):
    """LBR-001: _cleanup handles all edge cases safely."""

    def test_cleanup_with_none_client_and_proc(self) -> None:
        from assurance.retrieval_workflow import _cleanup
        # Should not raise
        _cleanup(None, None)

    def test_cleanup_with_client_only(self) -> None:
        from assurance.retrieval_workflow import _cleanup
        mock_client = MagicMock()
        _cleanup(mock_client, None)
        mock_client.close_all_owned_tabs.assert_called_once()
        mock_client.disconnect.assert_called_once()

    def test_cleanup_with_proc_only(self) -> None:
        from assurance.retrieval_workflow import _cleanup
        mock_proc = MagicMock()
        _cleanup(None, mock_proc)
        mock_proc.terminate.assert_called_once()

    def test_cleanup_client_error_does_not_prevent_proc_terminate(self) -> None:
        """If close_all_owned_tabs raises, disconnect and terminate still run."""
        from assurance.retrieval_workflow import _cleanup
        mock_client = MagicMock()
        mock_client.close_all_owned_tabs.side_effect = RuntimeError("CDP gone")
        mock_proc = MagicMock()
        _cleanup(mock_client, mock_proc)
        # disconnect should still be called
        mock_client.disconnect.assert_called_once()
        # proc should still be terminated
        mock_proc.terminate.assert_called_once()

    def test_cleanup_all_errors_handled(self) -> None:
        """Even if everything fails, _cleanup should not raise."""
        from assurance.retrieval_workflow import _cleanup
        mock_client = MagicMock()
        mock_client.close_all_owned_tabs.side_effect = RuntimeError("fail1")
        mock_client.disconnect.side_effect = RuntimeError("fail2")
        mock_proc = MagicMock()
        mock_proc.terminate.side_effect = RuntimeError("fail3")
        # Should not raise
        _cleanup(mock_client, mock_proc)


# ══════════════════════════════════════════════════════════════════════════════
# Progress callback tests
# ══════════════════════════════════════════════════════════════════════════════


class ProgressCallbackTests(unittest.TestCase):
    """LBR-001: progress callback wiring."""

    def test_callback_receives_progress_for_search(self) -> None:
        from assurance.retrieval_workflow import (
            RetrievalProgress,
            retrieve_search,
        )
        events: list[RetrievalProgress] = []

        # This will fail because there's no real browser, but the callback
        # should fire for the "searching" stage at least (before connection).
        try:
            retrieve_search(
                "test query",
                launch_browser=False,  # don't auto-launch
                on_progress=lambda evt: events.append(evt),
                max_results=1,
            )
        except AssuranceError:
            pass  # expected — no browser available

        # At least one progress event should have fired
        self.assertGreater(len(events), 0)
        self.assertEqual(events[0].stage, "searching")

    def test_callback_receives_progress_for_retrieval(self) -> None:
        from assurance.retrieval_workflow import (
            RetrievalProgress,
            run_retrieval,
        )
        events: list[RetrievalProgress] = []

        result = run_retrieval(
            "https://arxiv.org/abs/2605.15058",
            launch_browser=False,
            on_progress=lambda evt: events.append(evt),
        )

        # Should fail with connection error (no browser)
        self.assertNotEqual(result.error, "")
        # Should have at least "launching" and "failed" events
        self.assertGreater(len(events), 0)
        stages = [e.stage for e in events]
        self.assertIn("launching", stages)


if __name__ == "__main__":
    unittest.main()
