from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass
from typing import Any, Protocol, Sequence

from .deepseek_https import prepare_deepseek_https_request
from .errors import PrototypeError
from .external_network import DEEPSEEK_CHAT_ENDPOINT, OneShotNetworkPermit
from .io_utils import canonical_bytes, sha256_bytes
from .layout import RUNTIME_ROOT
from .model_transport import TransportAttemptContext, TransportControl
from .schema import validate_instance


def build_network_confirmation_summary(
    *,
    request: dict[str, Any],
    control: TransportControl,
    attempt_context: TransportAttemptContext,
) -> dict[str, Any]:
    prepared = prepare_deepseek_https_request(request)
    messages = request.get("messages")
    tools = request.get("tools", [])
    if not isinstance(messages, list) or not all(
        isinstance(message, dict) for message in messages
    ):
        raise PrototypeError("network confirmation requires normalized message objects")
    if not isinstance(tools, list):
        raise PrototypeError("network confirmation requires normalized tool definitions")
    model = request.get("model")
    if not isinstance(model, str) or not model:
        raise PrototypeError("network confirmation requires a model name")
    roles = [message.get("role") for message in messages]
    if not all(isinstance(role, str) and role for role in roles):
        raise PrototypeError("network confirmation message role is invalid")
    response_format = request.get("response_format")
    response_format_type = (
        response_format.get("type") if isinstance(response_format, dict) else None
    )
    summary = {
        "schema_version": "0.1.0-prototype",
        "provider": "deepseek",
        "endpoint": DEEPSEEK_CHAT_ENDPOINT,
        "billable_external_request": True,
        "turn": attempt_context.turn,
        "attempt": attempt_context.attempt,
        "is_retry": attempt_context.attempt > 1,
        "previous_status": attempt_context.previous_status,
        "model": model,
        "request_body_bytes": len(prepared.body),
        "request_body_sha256": prepared.body_sha256,
        "message_count": len(messages),
        "message_roles": roles,
        "tool_definition_count": len(tools),
        "tool_result_message_count": sum(role == "tool" for role in roles),
        "private_reasoning_message_count": sum(
            bool(message.get("reasoning_content")) for message in messages
        ),
        "stream": request.get("stream") is True,
        "response_format": response_format_type,
        "max_tokens": request.get("max_tokens"),
        "disclosure": {
            "sends_message_content": bool(messages),
            "sends_tool_definitions": bool(tools),
            "sends_provider_private_reasoning": any(
                bool(message.get("reasoning_content")) for message in messages
            ),
            "raw_message_content_recorded_in_summary": False,
            "raw_reasoning_recorded_in_summary": False,
            "authorization_recorded_in_summary": False,
        },
        "bounds": {
            "connect_seconds": control.connect_seconds,
            "first_semantic_seconds": control.first_semantic_seconds,
            "total_seconds": control.total_seconds,
            "max_request_bytes": control.max_request_bytes,
            "max_response_bytes": control.max_response_bytes,
        },
    }
    validate_instance(
        summary,
        RUNTIME_ROOT / "network-confirmation-summary-v0.1.schema.json",
        label="network confirmation summary",
    )
    return summary


def confirmation_summary_sha256(summary: dict[str, Any]) -> str:
    validate_instance(
        summary,
        RUNTIME_ROOT / "network-confirmation-summary-v0.1.schema.json",
        label="network confirmation summary",
    )
    return sha256_bytes(canonical_bytes(summary))


def render_network_confirmation(summary: dict[str, Any]) -> str:
    digest = confirmation_summary_sha256(summary)
    previous = (
        f" after HTTP {summary['previous_status']}"
        if summary["previous_status"] is not None
        else ""
    )
    disclosure = summary["disclosure"]
    return "\n".join(
        [
            f"DeepSeek external-request approval: turn {summary['turn']} attempt {summary['attempt']}{previous}",
            "Exposure if actually sent: billable request and message/tool disclosure",
            f"Endpoint: {summary['endpoint']}",
            f"Model: {summary['model']}",
            (
                "Payload: "
                f"{summary['request_body_bytes']} bytes, sha256={summary['request_body_sha256']}"
            ),
            (
                "Messages/tools: "
                f"{summary['message_count']} messages, "
                f"{summary['tool_definition_count']} tool definitions, "
                f"{summary['private_reasoning_message_count']} private-reasoning messages"
            ),
            (
                "Disclosure: message_content="
                f"{str(disclosure['sends_message_content']).lower()}, "
                "tool_definitions="
                f"{str(disclosure['sends_tool_definitions']).lower()}, "
                "provider_private_reasoning="
                f"{str(disclosure['sends_provider_private_reasoning']).lower()}"
            ),
            f"Confirmation summary sha256: {digest}",
        ]
    )


class PermitBroker(Protocol):
    def authorize(self, summary: dict[str, Any]) -> OneShotNetworkPermit:
        ...


class DenyAllPermitBroker:
    """Production-safe default: no implicit approval path exists."""

    def authorize(self, summary: dict[str, Any]) -> OneShotNetworkPermit:
        confirmation_summary_sha256(summary)
        raise PrototypeError("external network permit broker denied the request")


@dataclass(frozen=True)
class ScriptedPermitDecision:
    allow: bool
    expected_turn: int
    expected_attempt: int


class ScriptedPermitBroker:
    """Deterministic test broker; never use as a user-approval implementation."""

    test_only_auto_approve = True

    def __init__(self, decisions: Sequence[ScriptedPermitDecision]) -> None:
        if not decisions:
            raise PrototypeError("scripted permit broker requires decisions")
        self._decisions = list(decisions)
        self._summaries: list[dict[str, Any]] = []

    def authorize(self, summary: dict[str, Any]) -> OneShotNetworkPermit:
        confirmation_summary_sha256(summary)
        if not self._decisions:
            raise PrototypeError("scripted permit broker decision sequence is exhausted")
        decision = self._decisions.pop(0)
        if summary["turn"] != decision.expected_turn:
            raise PrototypeError("scripted permit broker turn mismatch")
        if summary["attempt"] != decision.expected_attempt:
            raise PrototypeError("scripted permit broker attempt mismatch")
        self._summaries.append(deepcopy(summary))
        if not decision.allow:
            raise PrototypeError("scripted permit broker denied the request")
        return OneShotNetworkPermit.issue_for_deepseek_chat(
            request_sha256=summary["request_body_sha256"]
        )

    @property
    def summaries(self) -> list[dict[str, Any]]:
        return deepcopy(self._summaries)

    @property
    def remaining_decision_count(self) -> int:
        return len(self._decisions)
