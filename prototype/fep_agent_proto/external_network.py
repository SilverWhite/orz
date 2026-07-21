from __future__ import annotations

from dataclasses import dataclass
import hashlib
import secrets
import threading
import time
from typing import Callable

from .errors import PrototypeError


DEEPSEEK_PROVIDER = "deepseek"
DEEPSEEK_CHAT_ENDPOINT = "https://api.deepseek.com/chat/completions"


@dataclass(frozen=True)
class NetworkPermitReceipt:
    permit_id_sha256: str
    provider: str
    endpoint: str
    request_sha256: str


class OneShotNetworkPermit:
    """Short-lived capability bound to one exact external request.

    This object is deliberately not serializable.  The caller that owns the
    user-interaction boundary is responsible for deciding whether to issue it;
    the permit only makes that decision narrow, expiring, and one-shot.
    """

    def __init__(
        self,
        *,
        provider: str,
        endpoint: str,
        request_sha256: str,
        ttl_seconds: float,
        clock: Callable[[], float] = time.monotonic,
    ) -> None:
        if provider != DEEPSEEK_PROVIDER:
            raise PrototypeError("network permit provider is not DeepSeek")
        if endpoint != DEEPSEEK_CHAT_ENDPOINT:
            raise PrototypeError("network permit endpoint is not the pinned DeepSeek chat endpoint")
        if not _is_sha256(request_sha256):
            raise PrototypeError("network permit request digest is not SHA-256")
        if ttl_seconds <= 0 or ttl_seconds > 300:
            raise PrototypeError("network permit TTL must be within (0, 300] seconds")
        self._provider = provider
        self._endpoint = endpoint
        self._request_sha256 = request_sha256
        self._clock = clock
        self._deadline = clock() + ttl_seconds
        self._permit_id = secrets.token_bytes(32)
        self._consumed = False
        self._lock = threading.Lock()

    @classmethod
    def issue_for_deepseek_chat(
        cls,
        *,
        request_sha256: str,
        ttl_seconds: float = 60,
        clock: Callable[[], float] = time.monotonic,
    ) -> OneShotNetworkPermit:
        return cls(
            provider=DEEPSEEK_PROVIDER,
            endpoint=DEEPSEEK_CHAT_ENDPOINT,
            request_sha256=request_sha256,
            ttl_seconds=ttl_seconds,
            clock=clock,
        )

    def consume(
        self, *, provider: str, endpoint: str, request_sha256: str
    ) -> NetworkPermitReceipt:
        with self._lock:
            if self._consumed:
                raise PrototypeError("external network permit was already consumed")
            if self._clock() > self._deadline:
                self._consumed = True
                raise PrototypeError("external network permit expired")
            if provider != self._provider:
                raise PrototypeError("external network permit provider mismatch")
            if endpoint != self._endpoint:
                raise PrototypeError("external network permit endpoint mismatch")
            if request_sha256 != self._request_sha256:
                raise PrototypeError("external network permit request digest mismatch")
            self._consumed = True
            return NetworkPermitReceipt(
                permit_id_sha256=hashlib.sha256(self._permit_id).hexdigest(),
                provider=self._provider,
                endpoint=self._endpoint,
                request_sha256=self._request_sha256,
            )

    @property
    def consumed(self) -> bool:
        with self._lock:
            return self._consumed

    def __repr__(self) -> str:
        return (
            "OneShotNetworkPermit(provider='deepseek', endpoint="
            "'https://api.deepseek.com/chat/completions', secret_id=<redacted>)"
        )


def _is_sha256(value: str) -> bool:
    return len(value) == 64 and all(character in "0123456789abcdef" for character in value)
