from __future__ import annotations

import http.client
import json
import socket
import time
from typing import Any
from urllib.parse import urlsplit

from .errors import PrototypeError
from .io_utils import sha256_bytes
from .model_transport import (
    TransportCancelled,
    TransportAttemptContext,
    TransportControl,
    TransportResponse,
    TransportTimeout,
)


class LoopbackHttpTransport:
    """HTTP/1.1 transport restricted to a literal IPv4 loopback endpoint."""

    transport_id = "loopback-http-v0.1"
    real_network = False

    def __init__(self, base_url: str) -> None:
        parsed = urlsplit(base_url)
        if parsed.scheme != "http":
            raise PrototypeError("loopback transport requires plain HTTP for local mock use")
        if parsed.hostname != "127.0.0.1":
            raise PrototypeError("loopback transport requires literal host 127.0.0.1")
        if parsed.username or parsed.password or parsed.query or parsed.fragment:
            raise PrototypeError("loopback transport URL cannot contain credentials or suffixes")
        if parsed.path not in {"", "/"}:
            raise PrototypeError("loopback transport base URL cannot contain a path")
        try:
            port = parsed.port
        except ValueError as exc:
            raise PrototypeError(f"invalid loopback transport port: {exc}") from exc
        if port is None or not 1 <= port <= 65535:
            raise PrototypeError("loopback transport requires an explicit valid port")
        self._port = port
        self._path = "/chat/completions"

    def send(
        self,
        request: dict[str, Any],
        control: TransportControl | None = None,
        attempt_context: TransportAttemptContext | None = None,
    ) -> TransportResponse:
        selected = control or TransportControl(
            connect_seconds=5,
            first_semantic_seconds=30,
            total_seconds=60,
        )
        if selected.cancelled:
            raise TransportCancelled("loopback transport cancelled before connect")
        try:
            body = json.dumps(
                request,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
                allow_nan=False,
            ).encode("utf-8")
        except (TypeError, ValueError) as exc:
            raise PrototypeError(f"cannot encode loopback request as strict JSON: {exc}") from exc
        if len(body) > selected.max_request_bytes:
            raise PrototypeError("loopback request exceeds configured byte limit")

        started = time.monotonic()
        connection = http.client.HTTPConnection(
            "127.0.0.1",
            self._port,
            timeout=selected.connect_seconds,
        )
        response: http.client.HTTPResponse | None = None
        try:
            connection.connect()
            if selected.cancelled:
                raise TransportCancelled("loopback transport cancelled after connect")
            if time.monotonic() - started > selected.connect_seconds:
                raise TransportTimeout("connect")
            try:
                connection.request(
                    "POST",
                    self._path,
                    body=body,
                    headers={
                        "Accept": "text/event-stream",
                        "Content-Type": "application/json",
                        "User-Agent": "fep-agent-proto-loopback/0.1",
                    },
                )
                response = connection.getresponse()
            except socket.timeout as exc:
                raise TransportTimeout("response_headers") from exc
            headers = {name.lower(): value for name, value in response.getheaders()}
            if headers.get("content-encoding", "identity").lower() not in {
                "",
                "identity",
            }:
                raise PrototypeError("loopback transport rejects compressed responses")
            if response.status == 200 and not headers.get("content-type", "").lower().startswith(
                "text/event-stream"
            ):
                raise PrototypeError("successful loopback response must be text/event-stream")

            if connection.sock is not None:
                connection.sock.settimeout(
                    max(
                        0.01,
                        min(
                            0.25,
                            selected.first_semantic_seconds,
                            selected.total_seconds,
                        ),
                    )
                )
            request_sent_at = time.monotonic()
            first_semantic_at: float | None = None
            total_bytes = 0
            lines: list[str] = []
            while True:
                now = time.monotonic()
                if selected.cancelled:
                    raise TransportCancelled("loopback transport cancelled while reading")
                if now - started >= selected.total_seconds:
                    raise TransportTimeout("total")
                if (
                    first_semantic_at is None
                    and now - request_sent_at >= selected.first_semantic_seconds
                ):
                    raise TransportTimeout("first_semantic")
                try:
                    raw = response.readline()
                except socket.timeout as exc:
                    phase = "first_semantic" if first_semantic_at is None else "read_idle"
                    raise TransportTimeout(phase) from exc
                if not raw:
                    break
                total_bytes += len(raw)
                if total_bytes > selected.max_response_bytes:
                    raise PrototypeError("loopback response exceeds configured byte limit")
                try:
                    line = raw.decode("utf-8").rstrip("\r\n")
                except UnicodeDecodeError as exc:
                    raise PrototypeError("loopback response is not valid UTF-8") from exc
                lines.append(line)
                stripped = line.strip()
                if stripped and not stripped.startswith(":") and first_semantic_at is None:
                    first_semantic_at = time.monotonic()

            completed = time.monotonic()
            return TransportResponse(
                status=response.status,
                lines=tuple(lines),
                headers=headers,
                metadata={
                    "request_body_bytes": len(body),
                    "request_body_sha256": sha256_bytes(body),
                    "response_body_bytes": total_bytes,
                    "first_semantic_observed": first_semantic_at is not None,
                    "first_semantic_ms": (
                        round((first_semantic_at - request_sent_at) * 1000, 3)
                        if first_semantic_at is not None
                        else None
                    ),
                    "duration_ms": round((completed - started) * 1000, 3),
                    "cancelled": False,
                    "external_network": False,
                },
            )
        except (ConnectionError, http.client.HTTPException, OSError) as exc:
            if isinstance(exc, (TransportCancelled, TransportTimeout)):
                raise
            raise PrototypeError(f"loopback HTTP transport failed: {exc}") from exc
        finally:
            if response is not None:
                response.close()
            connection.close()
