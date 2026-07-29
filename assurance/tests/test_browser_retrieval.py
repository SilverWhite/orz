"""Tests for browser retrieval — LBR-001 Phase 2.

Tests for :mod:`assurance.browser_retrieval`: URL validation, link
parsing, PDF detection, and CDP client construction.  Tests that
require a live Chrome instance are skipped when Chrome is unavailable.
"""

from __future__ import annotations

import os
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import MagicMock, patch

from assurance.errors import AssuranceError


# ══════════════════════════════════════════════════════════════════════════════
# URL validation tests (no browser required)
# ══════════════════════════════════════════════════════════════════════════════


class UrlValidationTests(unittest.TestCase):
    """LBR-001: navigation URL blocking policy."""

    def _validate(self, url: str) -> None:
        from assurance.browser_retrieval import _validate_navigation_url
        _validate_navigation_url(url)

    def test_https_allowed(self) -> None:
        self._validate("https://example.com/page")

    def test_http_allowed(self) -> None:
        self._validate("http://example.org")

    def test_file_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("file:///C:/windows/system32")

    def test_localhost_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("http://localhost:8080/admin")

    def test_127_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("http://127.0.0.1/api")

    def test_private_ip_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("http://192.168.1.1/")
        with self.assertRaises(AssuranceError):
            self._validate("http://10.0.0.1/")

    def test_chrome_internal_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("chrome://version")

    def test_edge_internal_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("edge://settings")

    def test_cloud_metadata_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("http://metadata.google.internal/computeMetadata")

    def test_doi_https_allowed(self) -> None:
        self._validate("https://doi.org/10.1234/example")

    def test_raw_ipv4_blocked(self) -> None:
        with self.assertRaises(AssuranceError):
            self._validate("http://54.231.123.456/path")


# ══════════════════════════════════════════════════════════════════════════════
# Data type tests (no browser required)
# ══════════════════════════════════════════════════════════════════════════════


class PageLinkTests(unittest.TestCase):
    """LBR-001: PageLink dataclass."""

    def test_construction(self) -> None:
        from assurance.browser_retrieval import PageLink
        link = PageLink(url="https://example.com", text="Example", title="Go to Example")
        self.assertEqual(link.url, "https://example.com")
        self.assertEqual(link.text, "Example")

    def test_defaults(self) -> None:
        from assurance.browser_retrieval import PageLink
        link = PageLink(url="https://x.com")
        self.assertEqual(link.text, "")
        self.assertEqual(link.rel, "")


class PageContentTests(unittest.TestCase):
    """LBR-001: PageContent dataclass."""

    def test_construction(self) -> None:
        from assurance.browser_retrieval import PageContent
        pc = PageContent(
            url="https://example.com",
            title="Test Page",
            text="Some text content.",
            char_count=17,
        )
        self.assertEqual(pc.title, "Test Page")
        self.assertEqual(pc.char_count, 17)


class TabInfoTests(unittest.TestCase):
    """LBR-001: TabInfo dataclass."""

    def test_construction(self) -> None:
        from assurance.browser_retrieval import TabInfo
        import time
        t = TabInfo(target_id="abc123", url="about:blank", created_at=time.time())
        self.assertEqual(t.target_id, "abc123")
        self.assertGreater(t.created_at, 0)


# ══════════════════════════════════════════════════════════════════════════════
# PDF detection logic tests (no browser required)
# ══════════════════════════════════════════════════════════════════════════════


class PdfDetectionTests(unittest.TestCase):
    """LBR-001: PDF link detection heuristics."""

    def _detect(self, links: list[dict]) -> list[str]:
        from assurance.browser_retrieval import BrowserCDPClient, PageLink
        client = BrowserCDPClient.__new__(BrowserCDPClient)
        page_links = [
            PageLink(
                url=l.get("url", ""),
                text=l.get("text", ""),
                title=l.get("title", ""),
            )
            for l in links
        ]
        return client._find_pdf_links_from_links(page_links, "fake-target")

    def test_direct_pdf_extension_detected(self) -> None:
        candidates = self._detect([
            {"url": "https://example.com/paper.pdf", "text": "Download"},
        ])
        self.assertIn("https://example.com/paper.pdf", candidates)

    def test_pdf_with_query_params_detected(self) -> None:
        candidates = self._detect([
            {"url": "https://publisher.com/article.pdf?download=1", "text": "PDF"},
        ])
        self.assertEqual(len(candidates), 1)

    def test_pdf_link_text_detected(self) -> None:
        candidates = self._detect([
            {"url": "https://publisher.com/article", "text": "Download PDF", "title": "Full Text PDF"},
        ])
        self.assertEqual(len(candidates), 1)

    def test_non_pdf_links_ignored(self) -> None:
        candidates = self._detect([
            {"url": "https://example.com/page", "text": "Read more"},
            {"url": "https://example.com/about", "text": "About us"},
        ])
        self.assertEqual(candidates, [])

    def test_javascript_links_ignored(self) -> None:
        candidates = self._detect([
            {"url": "javascript:void(0)", "text": "Download PDF"},
            {"url": "#section", "text": "PDF version"},
        ])
        self.assertEqual(candidates, [])

    def test_mixed_links(self) -> None:
        candidates = self._detect([
            {"url": "https://example.com/paper.pdf", "text": "Download"},
            {"url": "https://example.com/supplement.pdf", "text": "Supplementary Material"},
            {"url": "https://example.com/abstract", "text": "Abstract"},
            {"url": "https://arxiv.org/pdf/2501.12345", "text": "View PDF"},
        ])
        self.assertEqual(len(candidates), 3)


# ══════════════════════════════════════════════════════════════════════════════
# Client construction tests (no browser required)
# ══════════════════════════════════════════════════════════════════════════════


class BrowserCDPClientConstructionTests(unittest.TestCase):
    """LBR-001: BrowserCDPClient construction and state."""

    def test_default_construction(self) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        client = BrowserCDPClient()
        self.assertFalse(client.connected)
        self.assertEqual(client._port, 9222)

    def test_custom_port(self) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        client = BrowserCDPClient(port=9223)
        self.assertEqual(client._port, 9223)

    def test_connect_without_browser_returns_false(self) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        client = BrowserCDPClient(port=19222)  # unlikely to be in use
        self.assertFalse(client.connect())

    def test_require_connected_raises(self) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        client = BrowserCDPClient()
        with self.assertRaises(AssuranceError):
            client._require_connected()

    def test_new_tab_without_connect_raises(self) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        client = BrowserCDPClient()
        with self.assertRaises(AssuranceError):
            client.new_tab("https://example.com")

    def test_disconnect_when_not_connected_is_safe(self) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        client = BrowserCDPClient()
        client.disconnect()  # should not raise


class BrowserPortCleanupTests(unittest.TestCase):
    """LBR-001: stale CDP browser cleanup is narrowly scoped."""

    def _process(
        self,
        *,
        name: str,
        exe: str,
        cmdline: list[str],
    ) -> MagicMock:
        process = MagicMock()
        process.name.return_value = name
        process.exe.return_value = exe
        process.cmdline.return_value = cmdline
        return process

    def test_browser_debug_process_requires_browser_and_debug_port(self) -> None:
        from assurance.browser_retrieval import _is_browser_debug_process

        browser = self._process(
            name="chrome.exe",
            exe=r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            cmdline=["chrome.exe", "--remote-debugging-port=9222"],
        )
        self.assertTrue(_is_browser_debug_process(browser, port=9222))

        no_debug_port = self._process(
            name="chrome.exe",
            exe=r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            cmdline=["chrome.exe"],
        )
        self.assertFalse(_is_browser_debug_process(no_debug_port, port=9222))

        non_browser = self._process(
            name="python.exe",
            exe=r"C:\Python312\python.exe",
            cmdline=["python.exe", "--remote-debugging-port=9222"],
        )
        self.assertFalse(_is_browser_debug_process(non_browser, port=9222))

    def test_browser_debug_process_requires_matching_profile_when_provided(self) -> None:
        from assurance.browser_retrieval import _is_browser_debug_process

        profile = r"D:\CLI\.gsa_chrome_profile"
        browser = self._process(
            name="msedge.exe",
            exe=r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            cmdline=[
                "msedge.exe",
                "--remote-debugging-port=9222",
                f"--user-data-dir={profile}",
            ],
        )
        self.assertTrue(
            _is_browser_debug_process(
                browser,
                port=9222,
                user_data_dir=profile,
            )
        )

        other_profile = self._process(
            name="msedge.exe",
            exe=r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            cmdline=[
                "msedge.exe",
                "--remote-debugging-port=9222",
                r"--user-data-dir=D:\CLI\other_profile",
            ],
        )
        self.assertFalse(
            _is_browser_debug_process(
                other_profile,
                port=9222,
                user_data_dir=profile,
            )
        )

    def test_kill_browser_on_port_only_terminates_owned_browser(self) -> None:
        from assurance import browser_retrieval
        from assurance.browser_retrieval import BrowserCDPClient

        profile = r"D:\CLI\.gsa_chrome_profile"
        owned = self._process(
            name="chrome.exe",
            exe=r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            cmdline=[
                "chrome.exe",
                "--remote-debugging-port=9222",
                f"--user-data-dir={profile}",
            ],
        )
        other_browser = self._process(
            name="chrome.exe",
            exe=r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            cmdline=[
                "chrome.exe",
                "--remote-debugging-port=9222",
                r"--user-data-dir=D:\CLI\other_profile",
            ],
        )
        service = self._process(
            name="python.exe",
            exe=r"C:\Python312\python.exe",
            cmdline=["python.exe", "-m", "http.server"],
        )
        processes = {
            1001: owned,
            1002: other_browser,
            1003: service,
        }

        fake_psutil = MagicMock()
        fake_psutil.CONN_LISTEN = "LISTEN"
        fake_psutil.net_connections.return_value = [
            SimpleNamespace(
                laddr=SimpleNamespace(port=9222),
                status="LISTEN",
                pid=1001,
            ),
            SimpleNamespace(
                laddr=SimpleNamespace(port=9222),
                status="LISTEN",
                pid=1002,
            ),
            SimpleNamespace(
                laddr=SimpleNamespace(port=9222),
                status="LISTEN",
                pid=1003,
            ),
        ]
        fake_psutil.Process.side_effect = lambda pid: processes[pid]

        with (
            patch.object(browser_retrieval, "psutil", fake_psutil),
            patch("assurance.browser_retrieval.time.sleep"),
        ):
            BrowserCDPClient.kill_browser_on_port(9222, user_data_dir=profile)

        owned.terminate.assert_called_once()
        owned.kill.assert_not_called()
        other_browser.terminate.assert_not_called()
        service.terminate.assert_not_called()


# ══════════════════════════════════════════════════════════════════════════════
# Live browser tests (opt-in auto-launch of headless Chrome/Edge)
# ══════════════════════════════════════════════════════════════════════════════

_LIVE_PORT = 19223  # off the default 9222 to avoid conflicts


def _live_browser_tests_enabled() -> bool:
    """Live CDP tests are opt-in because CI runners vary by browser support."""
    return os.environ.get("GSA_RUN_LIVE_BROWSER_TESTS") == "1"


def _chrome_installed() -> bool:
    """Return True if Chrome or Edge is installed on this system."""
    from assurance.browser_retrieval import _find_chrome, _find_edge
    for path in (*_find_chrome(), *_find_edge()):
        if Path(path).is_file():
            return True
    return False


@unittest.skipUnless(
    _live_browser_tests_enabled() and _chrome_installed(),
    "live browser tests require GSA_RUN_LIVE_BROWSER_TESTS=1 and Chrome/Edge",
)
class LiveBrowserTests(unittest.TestCase):
    """LBR-001: live Chrome CDP integration tests.

    Auto-launches headless Chrome with a project-isolated profile on
    :data:`_LIVE_PORT`.  No manual browser setup required.
    """

    _proc: object = None  # subprocess.Popen | None

    @classmethod
    def setUpClass(cls) -> None:
        from assurance.browser_retrieval import BrowserCDPClient
        from assurance.retrieval_workflow import _ensure_browser

        # Auto-launch headless Chrome with the project-isolated profile.
        # _ensure_browser handles the "try connect first, then launch"
        # strategy and waits up to 30s for CDP to be ready.
        client = BrowserCDPClient(port=_LIVE_PORT)
        cls._client, cls._proc = _ensure_browser(
            client=client,
            port=_LIVE_PORT,
            launch=True,
            browser="chrome",
            headless=True,
        )
        cls._created_tabs: list[str] = []

    @classmethod
    def tearDownClass(cls) -> None:
        for target_id in cls._created_tabs:
            try:
                cls._client.close_tab(target_id)
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

    def test_new_tab_creates_target(self) -> None:
        target_id = self._client.new_tab("about:blank")
        self.assertIsNotNone(target_id)
        self.assertNotEqual(target_id, "")
        self.__class__._created_tabs.append(target_id)

    def test_navigate_and_read_page(self) -> None:
        target_id = self._client.new_tab("about:blank")
        self.__class__._created_tabs.append(target_id)

        # example.com is the most reliable page on the internet;
        # httpbin.org has intermittent timeouts in headless Chrome
        self._client.navigate(target_id, "https://example.com")
        content = self._client.read_page(target_id, max_chars=5000)
        self.assertGreater(len(content.text), 0)
        self.assertGreater(content.char_count, 0)

    def test_get_links(self) -> None:
        target_id = self._client.new_tab("about:blank")
        self.__class__._created_tabs.append(target_id)

        # example.com is fast, always available, and contains <a> elements
        self._client.navigate(target_id, "https://example.com")
        links = self._client.get_links(target_id)
        self.assertGreater(len(links), 0)
        for link in links:
            self.assertIsInstance(link.url, str)

    def test_close_tab(self) -> None:
        target_id = self._client.new_tab("about:blank")
        self._client.close_tab(target_id)
        # Should not be in owned tabs anymore
        self.assertNotIn(target_id, self._client._owned_tabs)


if __name__ == "__main__":
    unittest.main()
