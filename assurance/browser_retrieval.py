"""Browser retrieval via Chrome DevTools Protocol — LBR-001 Phase 2.

Provides a :class:`BrowserCDPClient` that controls a local Chrome or Edge
browser through the Chrome DevTools Protocol (CDP).  No browser extension
is required — CDP gives direct, authenticated access to a browser instance
that the user launched with ``--remote-debugging-port``.

Design constraints (from :file:`architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`):
    - Do not steal focus or switch the user's active tab.
    - AI-owned tabs are created in a dedicated window by default.
    - Never return cookies, passwords, auth headers, or localStorage.
    - Reject navigation to localhost, private IPs, and file:// URLs.
"""

from __future__ import annotations

import json
import logging
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.request import urlopen

from .errors import AssuranceError
from .endpoint_canonicalizer import canonicalize_network_endpoint

logger = logging.getLogger(__name__)

# ── CDP constants ────────────────────────────────────────────────────────────

DEFAULT_CDP_HOST = "localhost"
DEFAULT_CDP_PORT = 9222
_CDP_VERSION_ENDPOINT = "/json/version"
_CDP_LIST_ENDPOINT = "/json/list"
_CDP_NEW_ENDPOINT = "/json/new"

# URL block list — never navigate to these
_BLOCKED_URL_PREFIXES: tuple[str, ...] = (
    "file://",
    "chrome://",
    "edge://",
    "about:blank",  # allowed for new tabs but blocked for navigate()
    "chrome-extension://",
)
_BLOCKED_HOSTS: tuple[str, ...] = (
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "[::1]",
    "169.254.",    # link-local
    "10.",         # private
    "172.16.", "172.17.", "172.18.", "172.19.",
    "172.20.", "172.21.", "172.22.", "172.23.",
    "172.24.", "172.25.", "172.26.", "172.27.",
    "172.28.", "172.29.", "172.30.", "172.31.",
    "192.168.",    # private
    "metadata.google.internal",  # cloud metadata
)


# ── data types ───────────────────────────────────────────────────────────────


@dataclass
class PageLink:
    """A single ``<a href=...>`` element found on a page."""

    url: str
    text: str = ""
    title: str = ""
    rel: str = ""          # nofollow, noopener, etc.


@dataclass
class PageContent:
    """The extracted content of a web page."""

    url: str
    title: str = ""
    text: str = ""
    links: list[PageLink] = field(default_factory=list)
    pdf_candidates: list[str] = field(default_factory=list)
    content_hash: str = ""       # SHA-256 of the text
    char_count: int = 0


@dataclass
class TabInfo:
    """Metadata about an AI-owned tab."""

    target_id: str
    url: str = ""
    title: str = ""
    websocket_url: str = ""
    created_at: float = 0.0


# ── CDP client ───────────────────────────────────────────────────────────────


class BrowserCDPClient:
    """Control a local Chrome / Edge browser via CDP.

    Usage::

        client = BrowserCDPClient(port=9222)
        client.connect()
        tab = client.new_tab("https://example.com")
        content = client.read_page(tab)
        pdfs = client.find_pdf_links(tab)
        client.close_tab(tab)
        client.disconnect()

    The browser must be started with ``--remote-debugging-port=<port>``.
    Use :meth:`launch_browser` to start a compatible browser automatically,
    or :meth:`connect` to attach to an already-running instance.
    """

    def __init__(self, host: str = DEFAULT_CDP_HOST, port: int = DEFAULT_CDP_PORT) -> None:
        self._host = host
        self._port = port
        self._base = f"http://{host}:{port}"
        self._ws_connections: dict[str, Any] = {}  # target_id → websocket
        self._owned_tabs: dict[str, TabInfo] = {}
        self._msg_id: int = 0
        self._connected = False

    # ── connection management ──────────────────────────────────────────────

    @property
    def connected(self) -> bool:
        return self._connected

    def connect(self) -> bool:
        """Connect to an already-running browser's CDP endpoint.

        Returns ``True`` on success, ``False`` if the browser is not reachable.
        """
        try:
            resp = urlopen(f"{self._base}{_CDP_VERSION_ENDPOINT}", timeout=5)
            data = json.loads(resp.read().decode())
            logger.info(
                "Connected to %s %s on ws://%s:%s",
                data.get("Browser", "browser"),
                data.get("Browser-Version", ""),
                self._host, self._port,
            )
            self._connected = True
            return True
        except (URLError, OSError, json.JSONDecodeError) as exc:
            logger.warning("Cannot connect to browser on port %s: %s", self._port, exc)
            return False

    def disconnect(self) -> None:
        """Close all WebSocket connections and clean up."""
        for ws in self._ws_connections.values():
            try:
                ws.close()
            except Exception:
                pass
        self._ws_connections.clear()
        self._connected = False

    @staticmethod
    def launch_browser(
        port: int = DEFAULT_CDP_PORT,
        browser: str = "chrome",
        headless: bool = False,
        user_data_dir: str | None = None,
    ) -> subprocess.Popen | None:
        """Launch a browser instance with remote debugging enabled.

        Parameters
        ----------
        port:
            CDP debugging port.
        browser:
            ``"chrome"`` or ``"edge"``.  Falls back to the other if not found.
        headless:
            If ``True``, launch in headless mode (no visible window).
        user_data_dir:
            Optional path to a dedicated profile directory.  Created if absent.

        Returns
        -------
        :
            The :class:`subprocess.Popen` handle, or ``None`` if no browser
            executable was found.
        """
        import shutil

        # Resolve browser executable
        if browser == "chrome":
            candidates = _find_chrome()
        elif browser == "edge":
            candidates = _find_edge()
        else:
            raise AssuranceError(f"Unknown browser: {browser}. Use 'chrome' or 'edge'.")

        exe_path = None
        for c in candidates:
            if Path(c).is_file():
                exe_path = c
                break

        if exe_path is None:
            raise AssuranceError(
                f"Cannot find {browser} executable. "
                f"Searched: {', '.join(candidates)}"
            )

        args = [
            exe_path,
            f"--remote-debugging-port={port}",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-extensions",
            "--disable-background-networking",
            "--disable-sync",
            "--disable-translate",
            "--disable-features=TranslateUI",
            "--metrics-recording-only",
        ]

        if headless:
            args.append("--headless=new")

        if user_data_dir:
            udd = Path(user_data_dir)
            udd.mkdir(parents=True, exist_ok=True)
            args.append(f"--user-data-dir={udd}")

        logger.info("Launching browser: %s", exe_path)
        proc = subprocess.Popen(
            args,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL,
        )
        # Give the browser a moment to start
        time.sleep(1.5)
        return proc

    # ── tab management ─────────────────────────────────────────────────────

    def new_tab(self, url: str = "about:blank", background: bool = True) -> str:
        """Create a new AI-owned tab.

        Parameters
        ----------
        url:
            Initial URL to load (default: blank page).
        background:
            If ``True`` (default), the tab is created in the background and
            does not steal focus.

        Returns
        -------
        :
            The CDP ``targetId`` for the new tab.
        """
        self._require_connected()
        new_url = f"{self._base}{_CDP_NEW_ENDPOINT}/{url}"
        try:
            resp = urlopen(new_url, timeout=10)
            data = json.loads(resp.read().decode())
        except (URLError, OSError) as exc:
            raise AssuranceError(f"Failed to create tab: {exc}") from exc

        target_id = data.get("id", "")
        ws_url = data.get("webSocketDebuggerUrl", "")

        tab = TabInfo(
            target_id=target_id,
            url=url,
            created_at=time.time(),
            websocket_url=ws_url,
        )
        self._owned_tabs[target_id] = tab

        # Connect WebSocket to this target
        if ws_url:
            self._connect_ws(target_id, ws_url)

        # If background, move focus back to the original tab
        if background and target_id:
            self._activate_target(target_id, in_background=True)

        logger.info("New tab %s → %s", target_id[:20], url)
        return target_id

    def navigate(self, target_id: str, url: str) -> None:
        """Navigate an AI-owned tab to *url*.

        Raises :class:`AssuranceError` if the URL is blocked.
        """
        self._require_connected()
        _validate_navigation_url(url)
        self._send(target_id, "Page.enable")
        self._send(target_id, "Page.navigate", {"url": url})
        # Wait for page load
        self._wait_for_load(target_id)
        tab = self._owned_tabs.get(target_id)
        if tab:
            tab.url = url

    def close_tab(self, target_id: str) -> None:
        """Close an AI-owned tab."""
        self._require_connected()
        self._send(target_id, "Page.disable")
        self._send(None, "Target.closeTarget", {"targetId": target_id})
        if target_id in self._ws_connections:
            try:
                self._ws_connections[target_id].close()
            except Exception:
                pass
            del self._ws_connections[target_id]
        self._owned_tabs.pop(target_id, None)

    def close_all_owned_tabs(self) -> None:
        """Close every tab created through this client."""
        for target_id in list(self._owned_tabs.keys()):
            self.close_tab(target_id)

    # ── page reading ───────────────────────────────────────────────────────

    def read_page(self, target_id: str, max_chars: int = 100_000) -> PageContent:
        """Extract the visible text content of a page.

        Uses ``document.body.innerText`` for plain-text extraction.
        Does **not** return DOM markup, hidden elements, or script content.
        """
        self._require_connected()
        self._wait_for_load(target_id)

        # Extract title
        title_result = self._evaluate(target_id, "document.title")
        title = title_result.get("value", "") if isinstance(title_result, dict) else ""

        # Extract visible text
        text_result = self._evaluate(
            target_id,
            "document.body ? document.body.innerText : ''",
        )
        text = ""
        if isinstance(text_result, dict):
            text = text_result.get("value", "") or ""
        text = text[:max_chars]

        # Extract links
        links = self._get_all_links(target_id)

        # Find PDF candidates
        pdf_candidates = self._find_pdf_links_from_links(links, target_id)

        # Content hash
        from .utils import sha256_bytes
        content_hash = sha256_bytes(text.encode("utf-8"))

        return PageContent(
            url=self._owned_tabs.get(target_id, TabInfo(target_id="")).url,
            title=title,
            text=text,
            links=links,
            pdf_candidates=pdf_candidates,
            content_hash=content_hash,
            char_count=len(text),
        )

    def get_page_metadata(self, target_id: str) -> dict:
        """Extract structured metadata from the page.

        Returns JSON-LD, meta tags, and citation metadata when available.
        """
        self._require_connected()
        self._wait_for_load(target_id)

        meta: dict[str, Any] = {}

        # JSON-LD
        jsonld = self._evaluate(
            target_id,
            "Array.from(document.querySelectorAll('script[type=\"application/ld+json\"]')).map(s => { try { return JSON.parse(s.textContent) } catch(e) { return null } }).filter(Boolean)",
        )
        if isinstance(jsonld, dict) and jsonld.get("value"):
            meta["json_ld"] = jsonld["value"]

        # Standard meta tags
        meta_tags = self._evaluate(
            target_id,
            "Array.from(document.querySelectorAll('meta[name], meta[property]')).map(m => ({name: m.getAttribute('name') || m.getAttribute('property'), content: m.getAttribute('content')}))",
        )
        if isinstance(meta_tags, dict) and meta_tags.get("value"):
            meta["meta_tags"] = meta_tags["value"]

        # Citation metadata
        citation = self._evaluate(
            target_id,
            "Array.from(document.querySelectorAll('meta[name^=\"citation_\"]')).map(m => ({name: m.getAttribute('name'), content: m.getAttribute('content')}))",
        )
        if isinstance(citation, dict) and citation.get("value"):
            meta["citation"] = citation["value"]

        return meta

    # ── link & PDF detection ────────────────────────────────────────────────

    def get_links(self, target_id: str) -> list[PageLink]:
        """Return all links (``<a href>``) on the page."""
        self._require_connected()
        return self._get_all_links(target_id)

    def find_pdf_links(self, target_id: str) -> list[str]:
        """Return URLs that appear to be PDF download candidates."""
        self._require_connected()
        links = self._get_all_links(target_id)
        return self._find_pdf_links_from_links(links, target_id)

    # ── internal: WebSocket connection ─────────────────────────────────────

    def _require_connected(self) -> None:
        if not self._connected:
            raise AssuranceError("Not connected to browser. Call connect() first.")

    def _connect_ws(self, target_id: str, ws_url: str) -> None:
        import websocket
        try:
            ws = websocket.create_connection(ws_url, timeout=10)
            self._ws_connections[target_id] = ws
        except Exception as exc:
            raise AssuranceError(
                f"WebSocket connection failed for {target_id}: {exc}"
            ) from exc

    def _send(
        self, target_id: str | None, method: str, params: dict | None = None,
    ) -> dict:
        """Send a CDP command and return the result.

        If *target_id* is ``None``, use the browser-level connection
        (Target domain methods like ``closeTarget``).
        """
        if target_id is None:
            # Browser-level commands use the first available WS connection
            ws = next(iter(self._ws_connections.values()), None)
        else:
            ws = self._ws_connections.get(target_id)

        if ws is None:
            raise AssuranceError(f"No WebSocket connection for target {target_id}")

        self._msg_id += 1
        msg = {
            "id": self._msg_id,
            "method": method,
        }
        if params:
            msg["params"] = params

        ws.send(json.dumps(msg))

        # Read responses until we get a matching id or an error
        while True:
            try:
                raw = ws.recv()
            except Exception as exc:
                raise AssuranceError(f"WebSocket recv error: {exc}") from exc

            response = json.loads(raw)
            if response.get("id") == self._msg_id:
                if "error" in response:
                    err = response["error"]
                    raise AssuranceError(
                        f"CDP error: {err.get('message', str(err))}"
                    )
                return response.get("result", {})
            # Otherwise it's an event — ignore for sync mode

    def _evaluate(self, target_id: str, expression: str) -> dict:
        """Execute JavaScript in the page context and return the result."""
        self._send(target_id, "Runtime.enable")
        result = self._send(
            target_id,
            "Runtime.evaluate",
            {
                "expression": expression,
                "returnByValue": True,
                "awaitPromise": True,
            },
        )
        return result.get("result", {})

    def _wait_for_load(self, target_id: str, timeout: float = 30.0) -> None:
        """Wait for the page to finish loading."""
        self._send(target_id, "Page.enable")
        ws = self._ws_connections.get(target_id)
        if ws is None:
            return

        deadline = time.time() + timeout
        while time.time() < deadline:
            ws.settimeout(1.0)
            try:
                raw = ws.recv()
                msg = json.loads(raw)
                method = msg.get("method", "")
                if method == "Page.loadEventFired":
                    return
            except Exception:
                # Timeout or non-JSON — continue waiting
                pass
        logger.warning("Page load timeout for target %s", target_id[:20])

    def _get_all_links(self, target_id: str) -> list[PageLink]:
        """Extract all links from the current page."""
        raw = self._evaluate(
            target_id,
            "Array.from(document.querySelectorAll('a[href]')).map(a => ({url: a.href, text: (a.textContent || '').trim().substring(0, 200), title: (a.title || '').substring(0, 100), rel: (a.rel || '')}))",
        )
        links_data = raw.get("value", []) if isinstance(raw, dict) else []
        links: list[PageLink] = []
        for item in links_data or []:
            links.append(PageLink(
                url=item.get("url", ""),
                text=item.get("text", ""),
                title=item.get("title", ""),
                rel=item.get("rel", ""),
            ))
        return links

    def _find_pdf_links_from_links(
        self, links: list[PageLink], target_id: str,
    ) -> list[str]:
        """Identify PDF download candidates from a list of links."""
        candidates: list[str] = []

        for link in links:
            url_lower = link.url.lower()

            # Direct PDF links
            if url_lower.endswith(".pdf") or ".pdf?" in url_lower:
                candidates.append(link.url)
                continue

            # Common publisher PDF patterns
            text_lower = (link.text + " " + link.title).lower()
            if any(kw in text_lower for kw in ("pdf", "download", "full text", "view pdf")):
                if not link.url.startswith("javascript:") and not link.url.startswith("#"):
                    candidates.append(link.url)

        return candidates

    def _activate_target(self, target_id: str, in_background: bool = True) -> None:
        """Bring a target into focus or send it to background.

        When *in_background* is ``True``, the browser's active tab is not
        changed (the new target stays in the background).
        """
        # CDP: Target.activateTarget brings the tab to the front.
        # We deliberately do NOT call this for background tabs.
        # Instead we just send Page.enable to start receiving events.
        self._send(target_id, "Page.enable")
        if not in_background:
            try:
                self._send(None, "Target.activateTarget", {"targetId": target_id})
            except AssuranceError:
                pass  # best-effort


# ── URL validation ───────────────────────────────────────────────────────────


def _validate_navigation_url(url: str) -> None:
    """Raise :class:`AssuranceError` if *url* is blocked by policy."""
    import re
    from urllib.parse import urlparse

    parsed = urlparse(url)
    scheme = parsed.scheme.lower()

    if scheme not in ("http", "https"):
        raise AssuranceError(f"Blocked URL scheme: {scheme} — {url}")

    for prefix in _BLOCKED_URL_PREFIXES:
        if url.lower().startswith(prefix) and url != "about:blank":
            raise AssuranceError(f"Blocked URL prefix: {prefix} — {url}")

    hostname = (parsed.hostname or "").lower()
    for blocked in _BLOCKED_HOSTS:
        if hostname == blocked or hostname.startswith(blocked):
            raise AssuranceError(f"Blocked host: {hostname} — {url}")

    # Canonicalize and check for IP addresses that resolve to private ranges
    try:
        canonicalize_network_endpoint(url)
    except AssuranceError:
        raise AssuranceError(f"Endpoint canonicalization rejected: {url}")

    # No raw IP addresses (IPv4 or IPv6)
    if re.match(r"^\d+\.\d+\.\d+\.\d+$", hostname):
        raise AssuranceError(f"Blocked raw IPv4 address: {url}")
    if hostname.startswith("[") or "::" in hostname:
        raise AssuranceError(f"Blocked raw IPv6 address: {url}")


# ── browser discovery ────────────────────────────────────────────────────────


def _find_chrome() -> list[str]:
    """Return candidate paths for Google Chrome."""
    candidates = []
    if sys.platform == "win32":
        candidates = [
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
            "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
            str(Path.home() / "AppData/Local/Google/Chrome/Application/chrome.exe"),
        ]
    elif sys.platform == "darwin":
        candidates = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ]
    else:
        candidates = [
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
        ]
    return candidates


def _find_edge() -> list[str]:
    """Return candidate paths for Microsoft Edge."""
    candidates = []
    if sys.platform == "win32":
        candidates = [
            "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
            "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
        ]
    elif sys.platform == "darwin":
        candidates = [
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ]
    else:
        candidates = [
            "/usr/bin/microsoft-edge",
        ]
    return candidates
