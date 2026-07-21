from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass
import json
import ssl
import threading
from typing import Any, Callable, Sequence

from .deepseek_https import DEEPSEEK_CHAT_PATH, DEEPSEEK_HOST, DEEPSEEK_PORT
from .errors import PrototypeError
from .io_utils import sha256_bytes


@dataclass(frozen=True)
class FakeHttpsExchange:
    status: int
    lines: tuple[str, ...] = ()
    request_validator: Callable[[dict[str, Any]], None] | None = None

    def __post_init__(self) -> None:
        if not 100 <= self.status <= 599:
            raise PrototypeError("fake HTTPS status is invalid")


class _FakeSocket:
    def __init__(self) -> None:
        self.timeout: float | None = None

    def settimeout(self, value: float) -> None:
        self.timeout = value


class _FakeResponse:
    def __init__(self, exchange: FakeHttpsExchange) -> None:
        self.status = exchange.status
        if exchange.status == 200:
            lines = exchange.lines
            self._headers = [("Content-Type", "text/event-stream; charset=utf-8")]
        else:
            lines = (
                json.dumps(
                    {"error": {"type": "fake_status", "status": exchange.status}},
                    sort_keys=True,
                    separators=(",", ":"),
                ),
            )
            self._headers = [("Content-Type", "application/json")]
        self._lines = iter([line.encode("utf-8") + b"\r\n" for line in lines] + [b""])
        self.closed = False

    def getheaders(self) -> list[tuple[str, str]]:
        return list(self._headers)

    def readline(self) -> bytes:
        return next(self._lines)

    def close(self) -> None:
        self.closed = True


class _FakeConnection:
    def __init__(
        self,
        *,
        exchange: FakeHttpsExchange,
        record: Callable[[dict[str, Any]], None],
    ) -> None:
        self._exchange = exchange
        self._record = record
        self._response = _FakeResponse(exchange)
        self.sock = _FakeSocket()
        self.connected = False
        self.closed = False

    def connect(self) -> None:
        self.connected = True

    def request(
        self,
        method: str,
        path: str,
        *,
        body: bytes,
        headers: dict[str, str],
    ) -> None:
        if not self.connected:
            raise PrototypeError("fake HTTPS request occurred before connect")
        if method != "POST" or path != DEEPSEEK_CHAT_PATH:
            raise PrototypeError("fake HTTPS request target differs from pinned endpoint")
        authorization = headers.get("Authorization", "")
        if not authorization.startswith("Bearer ") or len(authorization) <= len("Bearer "):
            raise PrototypeError("fake HTTPS request lacks Bearer authorization")
        try:
            request = json.loads(body.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise PrototypeError("fake HTTPS request is not strict UTF-8 JSON") from exc
        if not isinstance(request, dict):
            raise PrototypeError("fake HTTPS request body is not an object")
        if self._exchange.request_validator is not None:
            self._exchange.request_validator(request)
        self._record(
            {
                "method": method,
                "path": path,
                "request_body_bytes": len(body),
                "request_body_sha256": sha256_bytes(body),
                "authorization_present": True,
                "authorization_recorded": False,
                "message_count": len(request.get("messages", [])),
                "private_reasoning_message_count": sum(
                    bool(message.get("reasoning_content"))
                    for message in request.get("messages", [])
                    if isinstance(message, dict)
                ),
            }
        )

    def getresponse(self) -> _FakeResponse:
        return self._response

    def close(self) -> None:
        self.closed = True


class InProcessFakeDeepSeekConnectionFactory:
    """Explicit no-socket provider fixture recognized by brokered transport."""

    is_in_process_fake_provider = True

    def __init__(self, exchanges: Sequence[FakeHttpsExchange]) -> None:
        if not exchanges:
            raise PrototypeError("fake HTTPS provider requires exchanges")
        self._exchanges = list(exchanges)
        self._records: list[dict[str, Any]] = []
        self._lock = threading.Lock()

    def __call__(
        self, host: str, port: int, timeout: float, context: ssl.SSLContext
    ) -> _FakeConnection:
        if host != DEEPSEEK_HOST or port != DEEPSEEK_PORT:
            raise PrototypeError("fake HTTPS factory received an unpinned endpoint")
        if timeout <= 0:
            raise PrototypeError("fake HTTPS factory received an invalid timeout")
        if not context.check_hostname or context.verify_mode != ssl.CERT_REQUIRED:
            raise PrototypeError("fake HTTPS factory received an unsafe TLS context")
        with self._lock:
            if not self._exchanges:
                raise PrototypeError("fake HTTPS provider exchange sequence is exhausted")
            exchange = self._exchanges.pop(0)
        return _FakeConnection(exchange=exchange, record=self._record)

    def _record(self, record: dict[str, Any]) -> None:
        with self._lock:
            self._records.append(deepcopy(record))

    @property
    def records(self) -> list[dict[str, Any]]:
        with self._lock:
            return deepcopy(self._records)

    @property
    def remaining_exchange_count(self) -> int:
        with self._lock:
            return len(self._exchanges)
