from __future__ import annotations

from dataclasses import dataclass
import http.client
import json
import socket
import ssl
import time
from typing import Any, Callable

from .credentials import CredentialProvider, UnavailableCredentialProvider
from .errors import PrototypeError
from .external_network import (
    DEEPSEEK_CHAT_ENDPOINT,
    DEEPSEEK_PROVIDER,
    OneShotNetworkPermit,
)
from .io_utils import sha256_bytes
from .model_transport import (
    TransportCancelled,
    TransportAttemptContext,
    TransportControl,
    TransportResponse,
    TransportTimeout,
)


DEEPSEEK_HOST = "api.deepseek.com"
DEEPSEEK_PORT = 443
DEEPSEEK_CHAT_PATH = "/chat/completions"


class SanitizedDeepSeekTransportError(PrototypeError):
    """External transport failure carrying only non-secret diagnostic fields."""

    def __init__(
        self,
        *,
        stage: str,
        error_type: str,
        error_number: int | None,
    ) -> None:
        self.stage = stage
        self.error_type = error_type
        self.error_number = error_number
        number_suffix = (
            f", error_number={error_number}" if error_number is not None else ""
        )
        super().__init__(
            f"DeepSeek HTTPS transport failed at {stage} ({error_type}{number_suffix})"
        )


@dataclass(frozen=True)
class PreparedDeepSeekRequest:
    body: bytes
    body_sha256: str


def prepare_deepseek_https_request(request: dict[str, Any]) -> PreparedDeepSeekRequest:
    try:
        body = json.dumps(
            request,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError) as exc:
        raise PrototypeError(f"cannot encode DeepSeek request as strict JSON: {exc}") from exc
    return PreparedDeepSeekRequest(body=body, body_sha256=sha256_bytes(body))


def create_deepseek_tls_context() -> ssl.SSLContext:
    context = ssl.create_default_context(purpose=ssl.Purpose.SERVER_AUTH)
    context.check_hostname = True
    context.verify_mode = ssl.CERT_REQUIRED
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    return context


def _default_connection_factory(
    host: str, port: int, timeout: float, context: ssl.SSLContext
) -> http.client.HTTPSConnection:
    return http.client.HTTPSConnection(host, port, timeout=timeout, context=context)


class DeepSeekHttpsTransport:
    """Pinned direct HTTPS transport gated by a one-shot request capability."""

    transport_id = "deepseek-direct-https-v0.1"
    real_network = True

    def __init__(
        self,
        *,
        permit: OneShotNetworkPermit,
        credential_provider: CredentialProvider | None = None,
        connection_factory: Callable[[str, int, float, ssl.SSLContext], Any] | None = None,
    ) -> None:
        self._permit = permit
        self._credential_provider = credential_provider or UnavailableCredentialProvider()
        self._connection_factory = connection_factory or _default_connection_factory
        self._injected_connection_factory = connection_factory is not None

    def send(
        self,
        request: dict[str, Any],
        control: TransportControl | None = None,
        attempt_context: TransportAttemptContext | None = None,
    ) -> TransportResponse:
        selected = control or TransportControl(
            connect_seconds=30,
            first_semantic_seconds=600,
            total_seconds=1800,
        )
        if selected.cancelled:
            raise TransportCancelled("DeepSeek HTTPS transport cancelled before approval")

        prepared = prepare_deepseek_https_request(request)
        if len(prepared.body) > selected.max_request_bytes:
            raise PrototypeError("DeepSeek request exceeds configured byte limit")
        receipt = self._permit.consume(
            provider=DEEPSEEK_PROVIDER,
            endpoint=DEEPSEEK_CHAT_ENDPOINT,
            request_sha256=prepared.body_sha256,
        )
        if selected.cancelled:
            raise TransportCancelled("DeepSeek HTTPS transport cancelled after approval")

        started = time.monotonic()
        context = create_deepseek_tls_context()
        connection: Any | None = None
        response: Any | None = None
        credential_source_id: str | None = None
        failure_stage = "credential_acquire"
        try:
            with self._credential_provider.acquire() as credential:
                credential_source_id = credential.source_id
                authorization = credential.authorization_value()
                headers = {
                    "Accept": "text/event-stream",
                    "Authorization": authorization,
                    "Content-Length": str(len(prepared.body)),
                    "Content-Type": "application/json",
                    "User-Agent": "fep-agent-proto-deepseek/0.1",
                }
                connection = self._connection_factory(
                    DEEPSEEK_HOST,
                    DEEPSEEK_PORT,
                    selected.connect_seconds,
                    context,
                )
                if getattr(connection, "debuglevel", 0) != 0:
                    raise PrototypeError("DeepSeek HTTPS debug output must be disabled")
                try:
                    failure_stage = "connect"
                    connection.connect()
                    if selected.cancelled:
                        raise TransportCancelled(
                            "DeepSeek HTTPS transport cancelled after connect"
                        )
                    if time.monotonic() - started > selected.connect_seconds:
                        raise TransportTimeout("connect")
                    failure_stage = "request"
                    connection.request(
                        "POST",
                        DEEPSEEK_CHAT_PATH,
                        body=prepared.body,
                        headers=headers,
                    )
                    failure_stage = "response_headers"
                    response = connection.getresponse()
                except socket.timeout as exc:
                    raise TransportTimeout("response_headers") from exc
                finally:
                    authorization = "<cleared>"
                    headers.clear()

            failure_stage = "response_headers_validate"
            response_headers = {
                str(name).lower(): str(value) for name, value in response.getheaders()
            }
            if response_headers.get("content-encoding", "identity").lower() not in {
                "",
                "identity",
            }:
                raise PrototypeError("DeepSeek HTTPS transport rejects compressed responses")
            if response.status == 200 and not response_headers.get(
                "content-type", ""
            ).lower().startswith("text/event-stream"):
                raise PrototypeError("successful DeepSeek response must be text/event-stream")

            sock = getattr(connection, "sock", None)
            if sock is not None:
                sock.settimeout(
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
            failure_stage = "response_body"
            while True:
                now = time.monotonic()
                if selected.cancelled:
                    raise TransportCancelled("DeepSeek HTTPS transport cancelled while reading")
                if now - started >= selected.total_seconds:
                    raise TransportTimeout("total")
                if (
                    first_semantic_at is None
                    and now - request_sent_at >= selected.first_semantic_seconds
                ):
                    raise TransportTimeout("first_semantic")
                try:
                    raw = response.readline()
                except socket.timeout:
                    # The short socket timeout is a cancellation/deadline polling
                    # interval, not the semantic or total deadline itself.
                    continue
                if not raw:
                    break
                total_bytes += len(raw)
                if total_bytes > selected.max_response_bytes:
                    raise PrototypeError("DeepSeek response exceeds configured byte limit")
                try:
                    line = raw.decode("utf-8").rstrip("\r\n")
                except UnicodeDecodeError as exc:
                    raise PrototypeError("DeepSeek response is not valid UTF-8") from exc
                lines.append(line)
                stripped = line.strip()
                if stripped and not stripped.startswith(":") and first_semantic_at is None:
                    first_semantic_at = time.monotonic()

            completed = time.monotonic()
            return TransportResponse(
                status=int(response.status),
                lines=tuple(lines),
                headers=response_headers,
                metadata={
                    "approval_permit_id_sha256": receipt.permit_id_sha256,
                    "credential_source_id": credential_source_id,
                    "request_body_bytes": len(prepared.body),
                    "request_body_sha256": prepared.body_sha256,
                    "response_body_bytes": total_bytes,
                    "first_semantic_observed": first_semantic_at is not None,
                    "first_semantic_ms": (
                        round((first_semantic_at - request_sent_at) * 1000, 3)
                        if first_semantic_at is not None
                        else None
                    ),
                    "duration_ms": round((completed - started) * 1000, 3),
                    "cancelled": False,
                    "external_network": True,
                    "endpoint": DEEPSEEK_CHAT_ENDPOINT,
                    "tls_verification": "system-trust-hostname-required-min-tls1.2",
                    "connection_factory": (
                        "injected" if self._injected_connection_factory else "stdlib-direct"
                    ),
                    "endpoint_pinned": True,
                    "proxy_environment_used": False,
                    "redirects_followed": False,
                    "http_debug_output": False,
                    "authorization_recorded": False,
                },
            )
        except (ConnectionError, http.client.HTTPException, OSError, ssl.SSLError) as exc:
            if isinstance(exc, (TransportCancelled, TransportTimeout)):
                raise
            error_number = getattr(exc, "winerror", None)
            if error_number is None:
                error_number = getattr(exc, "errno", None)
            raise SanitizedDeepSeekTransportError(
                stage=failure_stage,
                error_type=type(exc).__name__,
                error_number=error_number,
            ) from None
        finally:
            if response is not None:
                response.close()
            if connection is not None:
                connection.close()
