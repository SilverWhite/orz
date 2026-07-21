from __future__ import annotations

import json
from typing import Any

from .errors import PrototypeError
from .layout import RUNTIME_ROOT
from .schema import validate_instance


CURRENT_MODELS = {"deepseek-v4-pro", "deepseek-v4-flash"}
RETIRED_ALIASES = {"deepseek-chat", "deepseek-reasoner"}
RETRYABLE_HTTP_STATUSES = {429, 500, 503}


def _reject_json_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant {value}")


def validate_deepseek_profile(profile: dict[str, Any]) -> dict[str, Any]:
    validate_instance(
        profile,
        RUNTIME_ROOT / "deepseek-adapter-profile-v0.1.schema.json",
        label="DeepSeek adapter profile",
    )
    issues: list[dict[str, str]] = []

    def add(code: str, location: str, message: str) -> None:
        issues.append({"code": code, "location": location, "message": message})

    model = profile["model"]
    if model in RETIRED_ALIASES:
        add(
            "DS-MODEL-RETIRED-001",
            "/model",
            "legacy DeepSeek model aliases are not accepted by this adapter",
        )
    elif model not in CURRENT_MODELS:
        add(
            "DS-MODEL-UNKNOWN-001",
            "/model",
            "model is not in the pinned 2026-07-21 capability snapshot",
        )

    if profile["timeouts"]["total_seconds"] < profile["timeouts"]["connect_seconds"]:
        add(
            "DS-TIMEOUT-001",
            "/timeouts/total_seconds",
            "total timeout is shorter than connect timeout",
        )
    if profile["timeouts"]["total_seconds"] < profile["timeouts"]["first_inference_seconds"]:
        add(
            "DS-TIMEOUT-001",
            "/timeouts/total_seconds",
            "total timeout is shorter than first-inference timeout",
        )

    retry_statuses = set(profile["retry"]["retry_http_statuses"])
    if retry_statuses - RETRYABLE_HTTP_STATUSES:
        add(
            "DS-RETRY-001",
            "/retry/retry_http_statuses",
            "retry policy contains a status not classified as retryable",
        )

    issues.sort(key=lambda item: (item["location"], item["code"]))
    return {
        "schema_version": "0.1.0-prototype",
        "valid": not issues,
        "model_requested": model,
        "model_resolved": model if model in CURRENT_MODELS else None,
        "transport": profile["transport"],
        "thinking": profile["thinking"],
        "issue_count": len(issues),
        "issues": issues,
        "capability_snapshot": {
            "checked_at": "2026-07-21",
            "models": sorted(CURRENT_MODELS),
            "source": "DeepSeek official API documentation",
        },
        "limitations": [
            "No network request or /models capability discovery was performed.",
            "This preflight does not establish model correctness, evidence sufficiency, or evaluation readiness.",
        ],
    }


def validate_tool_history(
    messages: list[dict[str, Any]], *, thinking_enabled: bool
) -> dict[str, Any]:
    issues: list[dict[str, str]] = []
    tool_call_ids: set[str] = set()

    def add(code: str, location: str, message: str) -> None:
        issues.append({"code": code, "location": location, "message": message})

    for message_index, message in enumerate(messages):
        role = message.get("role")
        location = f"/messages/{message_index}"
        if role == "assistant" and message.get("tool_calls"):
            if thinking_enabled and not message.get("reasoning_content"):
                add(
                    "DS-REASONING-CONTINUITY-001",
                    f"{location}/reasoning_content",
                    "thinking-mode assistant tool calls require reasoning_content",
                )
            for tool_index, tool_call in enumerate(message["tool_calls"]):
                call_location = f"{location}/tool_calls/{tool_index}"
                call_id = tool_call.get("id")
                if not isinstance(call_id, str) or not call_id:
                    add("DS-TOOL-ID-001", f"{call_location}/id", "tool call ID is missing")
                elif call_id in tool_call_ids:
                    add("DS-TOOL-ID-001", f"{call_location}/id", "tool call ID is duplicated")
                else:
                    tool_call_ids.add(call_id)
                arguments = tool_call.get("function", {}).get("arguments")
                if not isinstance(arguments, str):
                    add(
                        "DS-TOOL-ARGS-001",
                        f"{call_location}/function/arguments",
                        "tool arguments must be a JSON string",
                    )
                else:
                    try:
                        json.loads(
                            arguments,
                            parse_constant=_reject_json_constant,
                        )
                    except (json.JSONDecodeError, ValueError) as exc:
                        add(
                            "DS-TOOL-ARGS-001",
                            f"{call_location}/function/arguments",
                            f"tool arguments are not strict JSON: {exc}",
                        )
        elif role == "tool":
            call_id = message.get("tool_call_id")
            if call_id not in tool_call_ids:
                add(
                    "DS-TOOL-ID-001",
                    f"{location}/tool_call_id",
                    "tool result references an unknown or future tool call",
                )

    issues.sort(key=lambda item: (item["location"], item["code"]))
    return {
        "schema_version": "0.1.0-prototype",
        "valid": not issues,
        "message_count": len(messages),
        "tool_call_count": len(tool_call_ids),
        "issue_count": len(issues),
        "issues": issues,
        "limitations": [
            "The validator checks transcript mechanics and never emits reasoning_content.",
            "Tool execution receipts and scientific evidence remain outside this adapter check.",
        ],
    }


def classify_sse_line(line: str) -> dict[str, Any]:
    stripped = line.strip()
    if not stripped:
        return {"kind": "separator", "payload": None}
    if stripped.startswith(":"):
        return {"kind": "keep_alive", "payload": None}
    if not stripped.startswith("data:"):
        raise PrototypeError("unsupported SSE line; expected comment or data field")
    data = stripped[5:].strip()
    if data == "[DONE]":
        return {"kind": "done", "payload": None}
    try:
        payload = json.loads(
            data,
            parse_constant=_reject_json_constant,
        )
    except (json.JSONDecodeError, ValueError) as exc:
        raise PrototypeError(f"invalid strict JSON in SSE data: {exc}") from exc
    if not isinstance(payload, dict):
        raise PrototypeError("SSE data payload must be a JSON object")
    return {"kind": "data", "payload": payload}


def classify_http_status(status: int) -> dict[str, Any]:
    categories = {
        400: "invalid_format",
        401: "authentication",
        402: "insufficient_balance",
        422: "invalid_parameters",
        429: "rate_limit",
        500: "server_error",
        503: "server_overloaded",
    }
    return {
        "status": status,
        "category": categories.get(status, "unknown"),
        "retryable": status in RETRYABLE_HTTP_STATUSES,
    }
