from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import ssl
from typing import Any, Callable

from .approval_ledger import (
    ApprovalLedger,
    ConfirmationIO,
    InteractiveRealNetworkPermitBroker,
    verify_approval_ledger,
)
from .brokered_transport import BrokeredDeepSeekHttpsTransport
from .credentials import (
    CredentialProvider,
    WINDOWS_DEEPSEEK_CREDENTIAL_TARGET,
    WindowsCredentialManagerProvider,
)
from .deepseek_adapter import classify_sse_line
from .deepseek_client import consume_deepseek_sse
from .errors import PrototypeError
from .io_utils import atomic_write_json, load_json, sha256_bytes, utc_now
from .layout import RUNTIME_ROOT
from .model_transport import TransportAttemptContext, TransportControl
from .network_broker import (
    build_network_confirmation_summary,
    confirmation_summary_sha256,
)
from .schema import validate_instance


PROBE_MODEL = "deepseek-v4-pro"
PROBE_MARKER = "LIF_REAL_DEEPSEEK_OK"
PLAN_FILENAME = "plan.json"
RESULT_FILENAME = "result.json"
LEDGER_FILENAME = "network-approvals.jsonl"


def _fixed_request() -> dict[str, Any]:
    return {
        "model": PROBE_MODEL,
        "messages": [
            {
                "role": "system",
                "content": "Return only the exact marker requested by the user.",
            },
            {
                "role": "user",
                "content": f"Reply with exactly {PROBE_MARKER}",
            },
        ],
        "thinking": {"type": "disabled"},
        "stream": True,
        "stream_options": {"include_usage": True},
        "max_tokens": 16,
    }


def _fixed_control() -> TransportControl:
    return TransportControl(
        connect_seconds=15,
        first_semantic_seconds=120,
        total_seconds=180,
        max_request_bytes=16 * 1024,
        max_response_bytes=2 * 1024 * 1024,
    )


def _resolved_response_model(lines: tuple[str, ...]) -> str:
    models: set[str] = set()
    for line in lines:
        classified = classify_sse_line(line)
        payload = classified.get("payload")
        if not isinstance(payload, dict) or "model" not in payload:
            continue
        model = payload["model"]
        if not isinstance(model, str) or not model:
            raise PrototypeError("DeepSeek SSE response model is invalid")
        models.add(model)
    if len(models) != 1:
        raise PrototypeError("DeepSeek SSE response must identify exactly one resolved model")
    return next(iter(models))


def create_real_development_plan(*, output_dir: Path) -> dict[str, Any]:
    if output_dir.exists():
        raise PrototypeError(f"refusing to overwrite existing probe root: {output_dir}")
    request = _fixed_request()
    control = _fixed_control()
    attempt_context = TransportAttemptContext(turn=1, attempt=1)
    summary = build_network_confirmation_summary(
        request=request,
        control=control,
        attempt_context=attempt_context,
    )
    plan = {
        "schema_version": "0.1.0-development",
        "plan_kind": "deepseek-real-development-one-shot",
        "created_at": utc_now(),
        "provider": "deepseek",
        "model": PROBE_MODEL,
        "credential": {
            "provider": "windows-credential-manager-current-user",
            "target": WINDOWS_DEEPSEEK_CREDENTIAL_TARGET,
            "value_recorded": False,
        },
        "request": {
            "fixed_prompt": True,
            "thinking": "disabled",
            "tool_definition_count": 0,
            "max_tokens": 16,
            "retry_count": 0,
            "expected_marker_sha256": sha256_bytes(PROBE_MARKER.encode("utf-8")),
        },
        "confirmation_summary": summary,
        "confirmation_summary_sha256": confirmation_summary_sha256(summary),
        "execution": {
            "network_attempted": False,
            "credential_read": False,
            "billable_request_made": False,
        },
        "limitations": [
            "This plan is not evidence that credentials, DNS, TLS, provider access, or billing are ready.",
            "A successful provider response is development conformance only and is not scientific evidence.",
        ],
    }
    validate_instance(
        plan,
        RUNTIME_ROOT / "deepseek-real-development-plan-v0.1.schema.json",
        label="DeepSeek real development plan",
    )
    output_dir.mkdir(parents=True, exist_ok=False)
    atomic_write_json(output_dir / PLAN_FILENAME, plan)
    return plan


def execute_real_development_probe(
    *,
    plan_path: Path,
    confirmation_io: ConfirmationIO,
    credential_provider: CredentialProvider | None = None,
    connection_factory: Callable[[str, int, float, ssl.SSLContext], Any] | None = None,
) -> dict[str, Any]:
    plan = load_json(plan_path)
    validate_instance(
        plan,
        RUNTIME_ROOT / "deepseek-real-development-plan-v0.1.schema.json",
        label="DeepSeek real development plan",
    )
    output_dir = plan_path.parent
    result_path = output_dir / RESULT_FILENAME
    if result_path.exists():
        raise PrototypeError(f"refusing to overwrite existing probe result: {result_path}")
    request = _fixed_request()
    control = _fixed_control()
    attempt_context = TransportAttemptContext(turn=1, attempt=1)
    summary = build_network_confirmation_summary(
        request=request,
        control=control,
        attempt_context=attempt_context,
    )
    summary_sha256 = confirmation_summary_sha256(summary)
    if plan["confirmation_summary"] != summary:
        raise PrototypeError("probe plan confirmation summary no longer matches fixed request")
    if plan["confirmation_summary_sha256"] != summary_sha256:
        raise PrototypeError("probe plan confirmation digest no longer matches fixed request")

    ledger_path = output_dir / LEDGER_FILENAME
    broker = InteractiveRealNetworkPermitBroker(
        ledger=ApprovalLedger(ledger_path),
        confirmation_io=confirmation_io,
    )
    transport = BrokeredDeepSeekHttpsTransport(
        permit_broker=broker,
        credential_provider=credential_provider or WindowsCredentialManagerProvider(),
        connection_factory=connection_factory,
    )
    if not transport.real_network:
        raise PrototypeError("real development probe requires the real-network transport")

    response = transport.send(
        request,
        control,
        attempt_context,
    )
    if response.status != 200:
        raise PrototypeError(f"DeepSeek real development probe returned HTTP {response.status}")
    resolved_model = _resolved_response_model(response.lines)
    private_message, public_response = consume_deepseek_sse(response.lines)
    content = private_message.get("content")
    if not isinstance(content, str):
        raise PrototypeError("DeepSeek real development probe content is not text")
    marker_matched = content == PROBE_MARKER
    approval = verify_approval_ledger(ledger_path)
    result = {
        "schema_version": "0.1.0-development",
        "result_kind": "deepseek-real-development-one-shot",
        "completed_at": utc_now(),
        "valid": bool(
            marker_matched
            and resolved_model == PROBE_MODEL
            and public_response["finish_reason"] == "stop"
            and approval["valid"]
            and approval["event_count"] == 1
            and approval["allow_count"] == 1
        ),
        "provider": "deepseek",
        "model": PROBE_MODEL,
        "resolved_model": resolved_model,
        "endpoint": response.metadata["endpoint"],
        "request_count": 1,
        "retry_count": 0,
        "http_status": response.status,
        "marker_matched": marker_matched,
        "response": {
            **deepcopy(public_response),
            "raw_content_recorded": False,
            "raw_reasoning_recorded": False,
        },
        "transport": {
            "request_body_bytes": response.metadata["request_body_bytes"],
            "request_body_sha256": response.metadata["request_body_sha256"],
            "response_body_bytes": response.metadata["response_body_bytes"],
            "first_semantic_observed": response.metadata["first_semantic_observed"],
            "duration_ms": response.metadata["duration_ms"],
            "tls_verification": response.metadata["tls_verification"],
            "credential_source_id": response.metadata["credential_source_id"],
            "authorization_recorded": False,
        },
        "approval": {
            "confirmation_summary_sha256": summary_sha256,
            "ledger_event_count": approval["event_count"],
            "ledger_terminal_event_sha256": approval["terminal_event_sha256"],
            "confirmation_token_recorded": False,
        },
        "claim_eligibility": "not_assessed",
        "limitations": [
            "This is a single development transport probe, not a reliability or quality evaluation.",
            "No prompt, response content, provider-private reasoning, or credential value is persisted.",
        ],
    }
    validate_instance(
        result,
        RUNTIME_ROOT / "deepseek-real-development-result-v0.1.schema.json",
        label="DeepSeek real development result",
    )
    atomic_write_json(result_path, result)
    return result
