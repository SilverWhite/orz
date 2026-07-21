from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import threading
import time
from typing import Any, Sequence

from .errors import PrototypeError


@dataclass(frozen=True)
class MockHttpExchange:
    status: int
    lines: tuple[str, ...] = ()
    initial_delay_seconds: float = 0
    line_delay_seconds: float = 0

    def __post_init__(self) -> None:
        if not 100 <= self.status <= 599:
            raise PrototypeError("mock HTTP status is invalid")
        if self.initial_delay_seconds < 0 or self.line_delay_seconds < 0:
            raise PrototypeError("mock HTTP delays cannot be negative")


class _MockServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = False

    def __init__(self, exchanges: Sequence[MockHttpExchange]) -> None:
        super().__init__(("127.0.0.1", 0), _MockHandler)
        self._exchanges = list(exchanges)
        self._requests: list[dict[str, Any]] = []
        self._lock = threading.Lock()

    def take_exchange(self) -> MockHttpExchange:
        with self._lock:
            if not self._exchanges:
                return MockHttpExchange(status=500)
            return self._exchanges.pop(0)

    def record_request(self, request: dict[str, Any]) -> None:
        with self._lock:
            self._requests.append(deepcopy(request))

    def request_snapshot(self) -> list[dict[str, Any]]:
        with self._lock:
            return deepcopy(self._requests)


class _MockHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "FEPModelLoopMock/0.1"

    @property
    def mock_server(self) -> _MockServer:
        return self.server  # type: ignore[return-value]

    def do_POST(self) -> None:  # noqa: N802 - stdlib handler API
        if self.path != "/chat/completions":
            self.send_error(404)
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            self.send_error(400)
            return
        if length <= 0 or length > 8 * 1024 * 1024:
            self.send_error(413)
            return
        raw = self.rfile.read(length)
        try:
            request = json.loads(
                raw.decode("utf-8"),
                parse_constant=lambda value: (_ for _ in ()).throw(
                    ValueError(f"non-standard JSON constant {value}")
                ),
            )
        except (UnicodeDecodeError, json.JSONDecodeError, ValueError):
            self.send_error(400)
            return
        if not isinstance(request, dict):
            self.send_error(400)
            return
        self.mock_server.record_request(request)
        exchange = self.mock_server.take_exchange()
        if exchange.initial_delay_seconds:
            time.sleep(exchange.initial_delay_seconds)
        try:
            if exchange.status != 200:
                body = json.dumps(
                    {"error": {"type": "mock_http_status", "status": exchange.status}},
                    separators=(",", ":"),
                ).encode("utf-8")
                self.send_response(exchange.status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Connection", "close")
                self.end_headers()
                self.wfile.write(body)
                self.wfile.flush()
                self.close_connection = True
                return

            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Connection", "close")
            self.end_headers()
            for line in exchange.lines:
                self.wfile.write(line.encode("utf-8") + b"\n\n")
                self.wfile.flush()
                if exchange.line_delay_seconds:
                    time.sleep(exchange.line_delay_seconds)
            self.close_connection = True
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            self.close_connection = True

    def log_message(self, format: str, *args: object) -> None:
        return


class LoopbackMockServer:
    """Context-managed HTTP server bound only to 127.0.0.1 and an ephemeral port."""

    def __init__(self, exchanges: Sequence[MockHttpExchange]) -> None:
        if not exchanges:
            raise PrototypeError("loopback mock server requires at least one exchange")
        self._server = _MockServer(exchanges)
        self._thread = threading.Thread(
            target=self._server.serve_forever,
            kwargs={"poll_interval": 0.05},
            name="fep-loopback-mock-server",
            daemon=True,
        )
        self._started = False

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self._server.server_port}"

    @property
    def requests(self) -> list[dict[str, Any]]:
        return self._server.request_snapshot()

    def __enter__(self) -> "LoopbackMockServer":
        self._thread.start()
        self._started = True
        return self

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> None:
        if self._started:
            self._server.shutdown()
            self._server.server_close()
            self._thread.join(timeout=5)
            self._started = False
