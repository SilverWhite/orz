from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass, field
import threading
from typing import Any, Protocol, Sequence

from .errors import PrototypeError


@dataclass(frozen=True)
class TransportResponse:
    status: int
    lines: tuple[str, ...]
    headers: dict[str, str]
    metadata: dict[str, Any] = field(default_factory=dict)


class TransportCancelled(PrototypeError):
    pass


class TransportTimeout(PrototypeError):
    def __init__(self, phase: str) -> None:
        super().__init__(f"transport timeout during {phase}")
        self.phase = phase


@dataclass(frozen=True)
class TransportAttemptContext:
    turn: int
    attempt: int
    previous_status: int | None = None

    def __post_init__(self) -> None:
        if self.turn < 1 or self.attempt < 1:
            raise PrototypeError("transport turn and attempt must be positive")
        if self.attempt == 1 and self.previous_status is not None:
            raise PrototypeError("first transport attempt cannot have a previous status")
        if self.attempt > 1 and self.previous_status is None:
            raise PrototypeError("retry transport attempt requires a previous status")
        if self.previous_status is not None and not 100 <= self.previous_status <= 599:
            raise PrototypeError("transport previous status is not a valid HTTP status")


@dataclass
class TransportControl:
    connect_seconds: float
    first_semantic_seconds: float
    total_seconds: float
    max_request_bytes: int = 8 * 1024 * 1024
    max_response_bytes: int = 8 * 1024 * 1024
    cancel_event: threading.Event = field(default_factory=threading.Event)

    def __post_init__(self) -> None:
        if self.connect_seconds <= 0:
            raise PrototypeError("transport connect timeout must be positive")
        if self.first_semantic_seconds <= 0:
            raise PrototypeError("transport first-semantic timeout must be positive")
        if self.total_seconds <= 0:
            raise PrototypeError("transport total timeout must be positive")
        if self.total_seconds < self.connect_seconds:
            raise PrototypeError("transport total timeout is shorter than connect timeout")
        if self.total_seconds < self.first_semantic_seconds:
            raise PrototypeError(
                "transport total timeout is shorter than first-semantic timeout"
            )
        if self.max_request_bytes <= 0 or self.max_response_bytes <= 0:
            raise PrototypeError("transport request and response limits must be positive")

    def cancel(self) -> None:
        self.cancel_event.set()

    @property
    def cancelled(self) -> bool:
        return self.cancel_event.is_set()


class StreamingTransport(Protocol):
    """Provider-neutral streaming transport boundary.

    The request is an already normalized provider body. Authentication and
    socket ownership belong to a future concrete transport, not to adapters.
    """

    transport_id: str
    real_network: bool

    def send(
        self,
        request: dict[str, Any],
        control: TransportControl | None = None,
        attempt_context: TransportAttemptContext | None = None,
    ) -> TransportResponse:
        ...


class ScriptedTransport:
    """Deterministic no-network transport used by contract tests and smoke runs."""

    transport_id = "scripted-no-network-v0.1"
    real_network = False

    def __init__(self, responses: Sequence[TransportResponse]) -> None:
        if not responses:
            raise PrototypeError("scripted transport requires at least one response")
        self._responses = list(responses)
        self.sent_requests: list[dict[str, Any]] = []

    def send(
        self,
        request: dict[str, Any],
        control: TransportControl | None = None,
        attempt_context: TransportAttemptContext | None = None,
    ) -> TransportResponse:
        if control is not None and control.cancelled:
            raise TransportCancelled("scripted transport cancelled before send")
        if not self._responses:
            raise PrototypeError("scripted transport response sequence is exhausted")
        self.sent_requests.append(deepcopy(request))
        return self._responses.pop(0)

    @property
    def remaining_response_count(self) -> int:
        return len(self._responses)
