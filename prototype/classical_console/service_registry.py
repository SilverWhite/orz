"""HA-style service registry: domain.service -> input schema -> handler -> response schema."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable

import jsonschema


class ConsoleError(Exception):
    code = "console_error"

    def __init__(
        self,
        message: str = "",
        *,
        step: str = "execute",
        upstream: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.step = step
        self.upstream = upstream


class UnknownService(ConsoleError):
    code = "unknown_service"


class InvalidArguments(ConsoleError):
    code = "invalid_arguments"


class InvalidResponse(ConsoleError):
    code = "invalid_response"


class ProtocolError(ConsoleError):
    code = "protocol_error"


class IntentUnavailable(ConsoleError):
    code = "intent_unavailable"


class UnknownIntent(ConsoleError):
    code = "unknown_intent"


@dataclass(frozen=True)
class Service:
    name: str
    input_schema: dict[str, Any]
    handler: Callable[[dict[str, Any], str], dict[str, Any]]
    response_schema: dict[str, Any]
    description: str = ""


class ServiceRegistry:
    """Deterministic, name-addressed service table (the console's button panel)."""

    def __init__(self) -> None:
        self._services: dict[str, Service] = {}

    def register(self, service: Service) -> None:
        if service.name in self._services:
            raise ValueError(f"duplicate service: {service.name}")
        self._services[service.name] = service

    def names(self) -> list[str]:
        return sorted(self._services)

    def call(
        self,
        name: str,
        data: dict[str, Any],
        allow_root: str,
        trace: Any = None,
    ) -> dict[str, Any]:
        service = self._services.get(name)
        if service is None:
            raise UnknownService(f"unknown service: {name}", step="registry")
        if trace is not None:
            trace.add(step="registry", action=name, ok=True)
        try:
            jsonschema.validate(instance=data, schema=service.input_schema)
        except jsonschema.ValidationError as exc:
            raise InvalidArguments(f"{name}: {exc.message}", step="contract") from exc
        if trace is not None:
            trace.add(step="contract", action=name, ok=True)
        response = service.handler(data, allow_root)
        if trace is not None:
            trace.add(step="execute", action=name, ok=True)
        try:
            jsonschema.validate(instance=response, schema=service.response_schema)
        except jsonschema.ValidationError as exc:
            raise InvalidResponse(
                f"{name}: response failed verification: {exc.message}",
                step="verify",
            ) from exc
        if trace is not None:
            trace.add(step="verify", action=name, ok=True)
        return response
