from __future__ import annotations

from copy import deepcopy
import ssl
from typing import Any, Callable

from .credentials import CredentialProvider, UnavailableCredentialProvider
from .deepseek_https import DeepSeekHttpsTransport
from .errors import PrototypeError
from .model_transport import (
    TransportAttemptContext,
    TransportControl,
    TransportResponse,
)
from .network_broker import (
    PermitBroker,
    build_network_confirmation_summary,
    confirmation_summary_sha256,
)


class BrokeredDeepSeekHttpsTransport:
    """Acquire a fresh digest-bound permit for every actual HTTP attempt."""

    def __init__(
        self,
        *,
        permit_broker: PermitBroker,
        credential_provider: CredentialProvider | None = None,
        connection_factory: Callable[[str, int, float, ssl.SSLContext], Any] | None = None,
    ) -> None:
        self._permit_broker = permit_broker
        self._credential_provider = credential_provider or UnavailableCredentialProvider()
        self._connection_factory = connection_factory
        self._in_process_fake = bool(
            connection_factory is not None
            and getattr(connection_factory, "is_in_process_fake_provider", False) is True
        )
        if (
            getattr(permit_broker, "test_only_auto_approve", False) is True
            and not self._in_process_fake
        ):
            raise PrototypeError(
                "scripted permit broker is restricted to the in-process fake provider"
            )
        if (
            getattr(permit_broker, "fake_provider_only", False) is True
            and not self._in_process_fake
        ):
            raise PrototypeError(
                "this interactive permit broker is restricted to the in-process fake provider"
            )
        self.real_network = not self._in_process_fake
        self.transport_id = (
            "deepseek-brokered-fake-https-v0.1"
            if self._in_process_fake
            else "deepseek-brokered-direct-https-v0.1"
        )

    def send(
        self,
        request: dict[str, Any],
        control: TransportControl | None = None,
        attempt_context: TransportAttemptContext | None = None,
    ) -> TransportResponse:
        if attempt_context is None:
            raise PrototypeError("brokered HTTPS transport requires attempt context")
        selected = control or TransportControl(
            connect_seconds=30,
            first_semantic_seconds=600,
            total_seconds=1800,
        )
        summary = build_network_confirmation_summary(
            request=request,
            control=selected,
            attempt_context=attempt_context,
        )
        permit = self._permit_broker.authorize(summary)
        direct = DeepSeekHttpsTransport(
            permit=permit,
            credential_provider=self._credential_provider,
            connection_factory=self._connection_factory,
        )
        response = direct.send(request, selected, attempt_context)
        metadata = deepcopy(response.metadata)
        metadata.update(
            {
                "external_network": self.real_network,
                "connection_factory": (
                    "in-process-fake-provider"
                    if self._in_process_fake
                    else metadata["connection_factory"]
                ),
                "confirmation_summary": summary,
                "confirmation_summary_sha256": confirmation_summary_sha256(summary),
                "permit_brokered_per_attempt": True,
            }
        )
        return TransportResponse(
            status=response.status,
            lines=response.lines,
            headers=response.headers,
            metadata=metadata,
        )
