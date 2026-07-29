"""Event source abstraction — GAK-UI-001 Phase 2.

Defines the :class:`EventSource` protocol and provides three implementations:

* :class:`FakeEventSource` — in-process queue fed from tests or demo scenarios
* :class:`JsonlFileSource` — reads events from a canonical JSONL journal file
  (now uses the bridge layer for typed event construction)
* :class:`LiveRunEventSource` — wraps a running canonical CLI in a background
  thread, streaming events into the TUI drain loop

Pre-built demo scenarios (:func:`build_gate_chain_demo`, etc.) produce
realistic synthetic event sequences for interactive demonstration.
"""

from __future__ import annotations

import queue
import threading
from abc import ABC, abstractmethod
from typing import Any, Callable

from .events import (
    ArtifactRegisteredEvent,
    ErrorEvent,
    GateDecisionEvent,
    ModelOutputEvent,
    ModelRequestEvent,
    RunFailedEvent,
    RunFinishedEvent,
    RunPreflightEvent,
    RunStartedEvent,
    SourceVisibilityEvent,
    TuiEvent,
    TuiEventKind,
    is_terminal,
)


# ── abstract source ──────────────────────────────────────────────────────────


class EventSource(ABC):
    """Abstract source of :class:`TuiEvent` instances.

    Implementations may be push-based (queue), pull-based (file tail),
    or bridge-based (real assurance-core pipe).  The TUI compositor
    drains events via :meth:`poll` from a background async coroutine.
    """

    @abstractmethod
    def poll(self) -> list[TuiEvent]:
        """Return newly available events (non-blocking).

        Returns an empty list when no events are ready.  Must not block
        or raise on transient unavailability.
        """

    @abstractmethod
    def close(self) -> None:
        """Signal the source to stop producing events."""

    def is_active(self) -> bool:
        """Return ``True`` while the source may still produce events.

        The default implementation returns ``True``; subclasses override
        to signal end-of-stream.
        """
        return True


# ── fake (in-process queue) source ───────────────────────────────────────────


class FakeEventSource(EventSource):
    """In-process event source backed by a thread-safe :class:`queue.Queue`.

    Tests and demo code push events synchronously via :meth:`push`.
    The TUI drain loop pulls them via :meth:`poll`.

    When *preload* events are supplied at construction time they are
    enqueued immediately, so the first :meth:`poll` picks them up
    without any external push.
    """

    def __init__(self, preload: list[TuiEvent] | None = None) -> None:
        self._queue: queue.Queue[TuiEvent | None] = queue.Queue()
        self._closed = False
        if preload:
            for evt in preload:
                self._queue.put_nowait(evt)

    # ── producer API (sync — called from tests / demo) ────────────────────

    def push(self, event: TuiEvent) -> None:
        """Push a single event into the source (non-blocking)."""
        if not self._closed:
            self._queue.put_nowait(event)

    def push_all(self, events: list[TuiEvent]) -> None:
        """Push a batch of events in order."""
        for evt in events:
            self.push(evt)

    # ── consumer API (sync — called from drain coroutine) ─────────────────

    def poll(self) -> list[TuiEvent]:
        """Drain all currently queued events without blocking."""
        results: list[TuiEvent] = []
        while True:
            try:
                item = self._queue.get_nowait()
            except queue.Empty:
                break
            if item is not None:
                results.append(item)
        return results

    def close(self) -> None:
        self._closed = True

    def is_active(self) -> bool:
        # Return True until explicitly closed — this matches the base class
        # default and supports streaming scenarios where events may arrive
        # after an idle period.  For test scenarios that need drain-then-stop
        # behaviour, call close() after the last push.
        return not self._closed


# ── JSONL file source ────────────────────────────────────────────────────────


class JsonlFileSource(EventSource):
    """Reads events from a canonical JSONL journal file.

    Intended for post-hoc replay of a completed run.  The file is read
    line-by-line; each line is parsed as a JSON object whose
    ``event_type`` field is mapped to the corresponding :class:`TuiEvent`
    subclass.

    .. note::

        This is a **Phase 1 stub** — the mapping from raw dict fields to
        typed event dataclasses is intentionally minimal.  A full
        canonical→TUI event bridge belongs in Phase 2.
    """

    _KIND_MAP: dict[str, TuiEventKind] = {
        "run_preflight": TuiEventKind.RUN_PREFLIGHT,
        "instruction_provenance_gate": TuiEventKind.INSTRUCTION_PROVENANCE_GATE,
        "tool_availability_check": TuiEventKind.TOOL_AVAILABILITY_CHECK,
        "orientation_checkpoint": TuiEventKind.ORIENTATION_CHECKPOINT,
        "run_started": TuiEventKind.RUN_STARTED,
        "gate_decision": TuiEventKind.GATE_DECISION,
        "model_request": TuiEventKind.MODEL_REQUEST,
        "model_output": TuiEventKind.MODEL_OUTPUT,
        "tool_proposal": TuiEventKind.TOOL_PROPOSAL,
        "permission_decision": TuiEventKind.PERMISSION_DECISION,
        "tool_started": TuiEventKind.TOOL_STARTED,
        "tool_completed": TuiEventKind.TOOL_COMPLETED,
        "artifact_registered": TuiEventKind.ARTIFACT_REGISTERED,
        "run_finished": TuiEventKind.RUN_FINISHED,
        "run_failed": TuiEventKind.RUN_FAILED,
        "run_cancelled": TuiEventKind.RUN_CANCELLED,
        "error_event": TuiEventKind.ERROR_EVENT,
        "status_update": TuiEventKind.STATUS_UPDATE,
    }

    def __init__(self, journal_path: str) -> None:
        import os
        self._path = journal_path
        self._lines: list[str] = []
        self._index = 0
        self._loaded = False
        self._exists = os.path.isfile(journal_path)

    def _ensure_loaded(self) -> None:
        if self._loaded:
            return
        self._loaded = True
        if not self._exists:
            return
        try:
            with open(self._path, "r", encoding="utf-8") as fh:
                self._lines = [line.rstrip("\n") for line in fh if line.strip()]
        except OSError:
            pass

    def poll(self) -> list[TuiEvent]:
        """Return the next unread event as a typed :class:`TuiEvent` subclass.

        Uses :func:`.bridge.build_event_from_jsonl_line` to map each
        JSONL line to the correct typed event with payload fields populated.
        """
        import json

        self._ensure_loaded()
        if self._index >= len(self._lines):
            return []
        line = self._lines[self._index]
        self._index += 1
        try:
            raw = json.loads(line)
        except json.JSONDecodeError:
            return []

        # Use the bridge layer to construct a typed TuiEvent subclass.
        # bridge.py is the ONLY module allowed to import from assurance.*.
        from .bridge import build_event_from_jsonl_line
        return [build_event_from_jsonl_line(raw)]

    def close(self) -> None:
        self._lines.clear()
        self._index = 0

    def is_active(self) -> bool:
        self._ensure_loaded()
        return self._index < len(self._lines)


# ── live run source (GAK-UI-001 Phase 2) ──────────────────────────────────────


class LiveRunEventSource(EventSource):
    """Event source that wraps a canonical CLI run in a background thread.

    Accepts a *run_fn* callable (provided by :mod:`.bridge`) that takes
    an ``on_event`` callback and executes the CLI.  This keeps
    ``event_source.py`` free of ``assurance.*`` imports — the bridge
    layer handles the wiring.

    Usage::

        from .bridge import build_live_run_fn
        source = LiveRunEventSource(
            run_fn=build_live_run_fn(run_root=..., ask="..."),
            on_event=build_on_event_callback(queue),
        )
        source.start()
    """

    def __init__(
        self,
        run_fn: Callable[[Callable[[dict[str, Any]], None]], Any],
    ) -> None:
        self._queue: queue.Queue[TuiEvent] = queue.Queue()
        self._thread: threading.Thread | None = None
        self._run_fn = run_fn
        self._started = False
        self._receipt: dict[str, Any] | None = None
        self._error: str | None = None
        self._closed = False

    def start(self) -> None:
        """Launch the canonical CLI run in a background daemon thread.

        Idempotent — calling :meth:`start` on an already-started source
        is a no-op.
        """
        if self._started:
            return
        self._started = True

        from .bridge import build_on_event_callback

        on_event = build_on_event_callback(self._queue)

        def _run() -> None:
            try:
                receipt = self._run_fn(on_event)
                self._receipt = receipt
            except Exception as exc:
                self._error = str(exc)

        self._thread = threading.Thread(target=_run, daemon=True)
        self._thread.start()

    def poll(self) -> list[TuiEvent]:
        """Drain all currently queued events (non-blocking)."""
        results: list[TuiEvent] = []
        while True:
            try:
                item = self._queue.get_nowait()
            except queue.Empty:
                break
            if item is not None:
                results.append(item)
        return results

    def close(self) -> None:
        """Signal the source to stop."""
        self._closed = True

    def is_active(self) -> bool:
        """Return ``True`` while the CLI thread is alive or events remain.

        Once the thread has exited AND the queue is drained, the drain
        loop will exit naturally.
        """
        if self._closed:
            return False
        thread_alive = self._thread is not None and self._thread.is_alive()
        queue_has_items = not self._queue.empty()
        return thread_alive or queue_has_items

    @property
    def receipt(self) -> dict[str, Any] | None:
        """The run receipt, populated after the CLI run completes."""
        return self._receipt

    @property
    def error(self) -> str | None:
        """Exception message if the CLI run crashed."""
        return self._error


# ── demo scenario builders ───────────────────────────────────────────────────


def build_gate_chain_demo() -> list[TuiEvent]:
    """Return the canonical 10-event gate chain — all gates pass.

    This mirrors the standard ``run_canonical_guarded_cli`` event sequence.
    """
    return [
        RunPreflightEvent(
            model_id="deepseek-v4-pro",
            adapter_id="fake",
            real_network_allowed=False,
        ),
        GateDecisionEvent(
            gate_name="instruction_provenance",
            decision="allow",
            reason="all sources classified, no injection",
            reference_count=1,
        ),
        GateDecisionEvent(
            gate_name="tool_availability",
            decision="allow",
            reason="3/3 tools available",
            reference_count=3,
        ),
        RunStartedEvent(
            task_id="TASK-DEMO-001",
            run_root="runtime/demo-run",
        ),
        GateDecisionEvent(
            gate_name="source_visibility",
            decision="allow",
            reason="all sources have full text",
            reference_count=3,
        ),
        SourceVisibilityEvent(
            source_id="MAP_MEM.md", observed="FULL TEXT",
            required="FULL TEXT", decision="ALLOW", claim="full_text",
        ),
        SourceVisibilityEvent(
            source_id="R211.md", observed="FULL TEXT",
            required="FULL TEXT", decision="ALLOW", claim="full_text",
        ),
        SourceVisibilityEvent(
            source_id="run_2026_06_26.json", observed="FULL TEXT",
            required="FULL TEXT", decision="ALLOW", claim="full_text",
        ),
        ModelRequestEvent(provider="deepseek", model_id="deepseek-v4-pro"),
        ModelOutputEvent(
            answer_packet_sha256="a1b2c3d4e5f6...",
            structured_output_valid=True,
        ),
        ArtifactRegisteredEvent(
            artifact_path="runtime/demo-run/answer-packet.json",
            artifact_sha256="a1b2c3d4e5f6...",
        ),
        RunFinishedEvent(status="completed"),
    ]


def build_gate_defer_demo() -> list[TuiEvent]:
    """Return a gate chain where source visibility returns ``defer``.

    One reference (R211.md) has only partial text, so the gate defers
    and the answer is marked as incomplete.
    """
    return [
        RunPreflightEvent(
            model_id="deepseek-v4-pro",
            adapter_id="fake",
            real_network_allowed=False,
        ),
        GateDecisionEvent(
            gate_name="instruction_provenance",
            decision="allow",
            reason="all sources classified",
            reference_count=1,
        ),
        RunStartedEvent(task_id="TASK-DEMO-002", run_root="runtime/demo-run"),
        GateDecisionEvent(
            gate_name="source_visibility",
            decision="defer",
            reason="SOURCE_FULLTEXT_MISSING",
            reference_count=3,
        ),
        SourceVisibilityEvent(
            source_id="MAP_MEM.md", observed="FULL TEXT",
            required="FULL TEXT", decision="ALLOW", claim="full_text",
        ),
        SourceVisibilityEvent(
            source_id="R211.md", observed="PARTIAL",
            required="FULL TEXT", decision="DEFER", claim="fragment",
        ),
        SourceVisibilityEvent(
            source_id="run_2026_06_26.json", observed="FULL TEXT",
            required="FULL TEXT", decision="ALLOW", claim="full_text",
        ),
        ModelRequestEvent(provider="deepseek", model_id="deepseek-v4-pro"),
        ModelOutputEvent(
            answer_packet_sha256="b2c3d4e5f6a1...",
            structured_output_valid=True,
        ),
        ArtifactRegisteredEvent(
            artifact_path="runtime/demo-run/answer-packet.json",
            artifact_sha256="b2c3d4e5f6a1...",
        ),
        RunFinishedEvent(status="completed"),
    ]


def build_run_failed_demo() -> list[TuiEvent]:
    """Return a gate chain where IPG blocks — the run fails before model call."""
    return [
        RunPreflightEvent(
            model_id="deepseek-v4-pro",
            adapter_id="fake",
            real_network_allowed=False,
        ),
        GateDecisionEvent(
            gate_name="instruction_provenance",
            decision="block",
            reason="injection detected: obfuscated directive in untrusted source",
            reference_count=1,
        ),
        ErrorEvent(
            message="IPG blocked — injection alert in source #3",
            source="gate",
        ),
        RunFailedEvent(reason="instruction_provenance_gate: block"),
    ]
