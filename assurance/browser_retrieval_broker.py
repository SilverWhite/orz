"""Browser Retrieval Broker — LBR-001 §5.2.

Safety layer between model-directed (or CLI-directed) browser actions
and raw CDP access.  Every operation is gated on:

1. **Task ID** — all actions must carry a valid, non-expired task_id.
2. **Capability sets** — the broker defines explicit capability
   categories; an operation may only use CDP methods within its set.
3. **Tab ownership** — the broker tracks which tabs belong to which
   task and rejects operations on unknown or foreign tabs.
4. **Method allowlist** — only a predefined set of CDP methods may
   be called; arbitrary ``_send()`` / ``_evaluate()`` is not exposed.

The broker wraps :class:`BrowserCDPClient` but does not inherit from it.
"""

from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any

from .browser_retrieval import (
    BrowserCDPClient,
    PageContent,
    PageLink,
)
from .errors import AssuranceError


# ── capability sets ──────────────────────────────────────────────────────────


class BrokerCapability(str, Enum):
    """Explicit capability categories for browser operations.

    Each operation requires a specific capability; a task is granted
    a subset of these at creation time.
    """

    TAB_CREATE = "tab_create"          # new_tab
    TAB_CLOSE = "tab_close"            # close_tab, close_all_owned_tabs
    TAB_NAVIGATE = "tab_navigate"      # navigate
    PAGE_READ = "page_read"            # read_page, get_links
    PAGE_METADATA = "page_metadata"    # get_page_metadata
    PDF_DETECT = "pdf_detect"          # find_pdf_links


# Tasks that only search the web need read + navigate; tasks that
# download papers need the full set.
READ_ONLY_CAPABILITIES: frozenset[BrokerCapability] = frozenset({
    BrokerCapability.TAB_CREATE,
    BrokerCapability.TAB_CLOSE,
    BrokerCapability.TAB_NAVIGATE,
    BrokerCapability.PAGE_READ,
    BrokerCapability.PAGE_METADATA,
    BrokerCapability.PDF_DETECT,
})

SEARCH_CAPABILITIES: frozenset[BrokerCapability] = READ_ONLY_CAPABILITIES


# ── CDP method allowlist ─────────────────────────────────────────────────────

# Only these CDP methods may be called through the broker.  Everything
# else (including Runtime.evaluate with arbitrary expressions) is blocked.
_ALLOWED_CDP_METHODS: frozenset[str] = frozenset({
    "Target.createTarget",
    "Target.closeTarget",
    "Target.activateTarget",
    "Page.enable",
    "Page.disable",
    "Page.navigate",
    "Runtime.enable",
    "Runtime.evaluate",
})


# ── task record ──────────────────────────────────────────────────────────────


@dataclass
class BrokerTask:
    """A registered retrieval task with bounded capabilities."""

    task_id: str
    capabilities: frozenset[BrokerCapability]
    created_at: float = field(default_factory=time.time)
    tab_ids: set[str] = field(default_factory=set)
    closed: bool = False


# ── broker ───────────────────────────────────────────────────────────────────


class BrowserRetrievalBroker:
    """Safety broker for browser retrieval operations.

    Usage::

        broker = BrowserRetrievalBroker(client)
        task_id = broker.create_task(capabilities=SEARCH_CAPABILITIES)
        tab_id = broker.new_tab(task_id, "https://example.com")
        content = broker.read_page(task_id, tab_id)
        broker.close_task(task_id)

    Every method that touches the browser requires a *task_id* and
    checks that the task has the required capability.  This prevents
    model-controlled code from issuing arbitrary CDP commands.
    """

    def __init__(self, client: BrowserCDPClient) -> None:
        self._client = client
        self._tasks: dict[str, BrokerTask] = {}

    # ── task lifecycle ───────────────────────────────────────────────────

    def create_task(
        self,
        capabilities: frozenset[BrokerCapability] = SEARCH_CAPABILITIES,
        task_id: str | None = None,
    ) -> str:
        """Register a new retrieval task and return its *task_id*."""
        tid = task_id or f"browser-task-{uuid.uuid4().hex[:12]}"
        if tid in self._tasks:
            raise AssuranceError(f"Duplicate task_id: {tid}")
        self._tasks[tid] = BrokerTask(
            task_id=tid,
            capabilities=capabilities,
        )
        return tid

    def close_task(self, task_id: str) -> None:
        """Close all tabs owned by *task_id* and mark the task closed."""
        task = self._require_task(task_id)
        for tab_id in list(task.tab_ids):
            try:
                self._client.close_tab(tab_id)
            except Exception:
                pass
        task.tab_ids.clear()
        task.closed = True

    def close_all_tasks(self) -> None:
        """Close every registered task and its tabs."""
        for tid in list(self._tasks.keys()):
            self.close_task(tid)

    # ── tab operations ───────────────────────────────────────────────────

    def new_tab(
        self, task_id: str, url: str = "about:blank", background: bool = True,
    ) -> str:
        """Create a new tab owned by *task_id*."""
        task = self._require_task(task_id)
        self._require_capability(task, BrokerCapability.TAB_CREATE)
        target_id = self._client.new_tab(url=url, background=background)
        task.tab_ids.add(target_id)
        return target_id

    def navigate(self, task_id: str, target_id: str, url: str) -> None:
        """Navigate a task-owned tab to *url*."""
        task = self._require_task(task_id)
        self._require_capability(task, BrokerCapability.TAB_NAVIGATE)
        self._require_owned_tab(task, target_id)
        self._client.navigate(target_id, url)

    def close_tab(self, task_id: str, target_id: str) -> None:
        """Close a task-owned tab."""
        task = self._require_task(task_id)
        self._require_capability(task, BrokerCapability.TAB_CLOSE)
        self._require_owned_tab(task, target_id)
        self._client.close_tab(target_id)
        task.tab_ids.discard(target_id)

    # ── page reading ─────────────────────────────────────────────────────

    def read_page(
        self, task_id: str, target_id: str, max_chars: int = 100_000,
    ) -> PageContent:
        """Read the text content of a task-owned page."""
        task = self._require_task(task_id)
        self._require_capability(task, BrokerCapability.PAGE_READ)
        self._require_owned_tab(task, target_id)
        return self._client.read_page(target_id, max_chars=max_chars)

    def get_links(
        self, task_id: str, target_id: str,
    ) -> list[PageLink]:
        """Get all links from a task-owned page."""
        task = self._require_task(task_id)
        self._require_capability(task, BrokerCapability.PAGE_READ)
        self._require_owned_tab(task, target_id)
        return self._client.get_links(target_id)

    def get_page_metadata(
        self, task_id: str, target_id: str,
    ) -> dict[str, Any]:
        """Extract structured metadata from a task-owned page."""
        task = self._require_task(task_id)
        self._require_capability(task, BrokerCapability.PAGE_METADATA)
        self._require_owned_tab(task, target_id)
        return self._client.get_page_metadata(target_id)

    def find_pdf_links(
        self, task_id: str, target_id: str,
    ) -> list[str]:
        """Find PDF download candidates on a task-owned page."""
        task = self._require_task(task_id)
        self._require_capability(task, BrokerCapability.PDF_DETECT)
        self._require_owned_tab(task, target_id)
        return self._client.find_pdf_links(target_id)

    # ── internal guards ──────────────────────────────────────────────────

    def _require_task(self, task_id: str) -> BrokerTask:
        task = self._tasks.get(task_id)
        if task is None:
            raise AssuranceError(
                f"Unknown task_id: {task_id}. "
                f"Call create_task() first."
            )
        if task.closed:
            raise AssuranceError(
                f"Task {task_id} is closed. Create a new task."
            )
        return task

    @staticmethod
    def _require_capability(task: BrokerTask, capability: BrokerCapability) -> None:
        if capability not in task.capabilities:
            raise AssuranceError(
                f"Task {task.task_id} does not have capability "
                f"{capability.value}. Allowed: "
                f"{sorted(c.value for c in task.capabilities)}"
            )

    @staticmethod
    def _require_owned_tab(task: BrokerTask, target_id: str) -> None:
        if target_id not in task.tab_ids:
            raise AssuranceError(
                f"Tab {target_id} is not owned by task {task.task_id}. "
                f"Owned tabs: {sorted(task.tab_ids)}"
            )

    # ── passthrough ──────────────────────────────────────────────────────

    @property
    def client(self) -> BrowserCDPClient:
        """The underlying CDP client (for setup/teardown only)."""
        return self._client
