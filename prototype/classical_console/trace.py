"""Bounded, read-only execution trace for the Classical Console."""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from typing import Any

MAX_TRACES = 50
MAX_EVENTS = 200


@dataclass
class Trace:
    trace_id: str
    request_id: str | None
    events: list[dict[str, Any]] = field(default_factory=list)

    def add(self, **event: Any) -> None:
        if len(self.events) >= MAX_EVENTS:
            return
        event.setdefault("seq", len(self.events) + 1)
        self.events.append(event)

    def tail(self, n: int) -> tuple[list[dict[str, Any]], bool]:
        events = self.events[-n:]
        return events, len(self.events) > n


class TraceStore:
    """In-memory bounded store (last MAX_TRACES traces)."""

    def __init__(self, max_traces: int = MAX_TRACES) -> None:
        self._traces: deque[Trace] = deque(maxlen=max_traces)
        self._seq = 0

    def new(self, request_id: str | None) -> Trace:
        self._seq += 1
        trace = Trace(trace_id=f"t{self._seq:06d}", request_id=request_id)
        self._traces.append(trace)
        return trace

    def get(self, trace_id: str) -> Trace | None:
        for trace in self._traces:
            if trace.trace_id == trace_id:
                return trace
        return None
