from __future__ import annotations

from typing import Any

from .utils import utc_now


FAILURE_CATEGORIES = {
    "network_timeout": {
        "retryable": True,
        "description": "Network request timed out before receiving a response.",
        "recovery_action": "retry_with_backoff",
    },
    "network_unreachable": {
        "retryable": False,
        "description": "Cannot reach the API endpoint — DNS, routing, or firewall issue.",
        "recovery_action": "check_connectivity_and_retry",
    },
    "auth_failure": {
        "retryable": False,
        "description": "API key is invalid, expired, or has insufficient permissions.",
        "recovery_action": "refresh_credential_and_retry",
    },
    "rate_limited": {
        "retryable": True,
        "description": "Provider returned a rate-limit response (HTTP 429).",
        "recovery_action": "backoff_and_retry",
    },
    "server_error": {
        "retryable": True,
        "description": "Provider returned a server error (HTTP 5xx).",
        "recovery_action": "retry_with_backoff",
    },
    "content_filtered": {
        "retryable": False,
        "description": "Request was blocked by the provider's content filter.",
        "recovery_action": "review_prompt_and_retry",
    },
    "invalid_output": {
        "retryable": True,
        "description": "Adapter response failed output validation (schema, claim boundaries, credential leak).",
        "recovery_action": "retry_with_output_constraints",
    },
    "model_unavailable": {
        "retryable": False,
        "description": "Requested model ID is not available, deprecated, or access is denied.",
        "recovery_action": "switch_model_or_abort",
    },
    "context_length_exceeded": {
        "retryable": False,
        "description": "Input exceeds the model's context window.",
        "recovery_action": "truncate_input_and_retry",
    },
    "unknown": {
        "retryable": False,
        "description": "Unclassified adapter failure.",
        "recovery_action": "investigate_and_decide",
    },
}


def classify_adapter_error(
    exception: Exception | None = None,
    *,
    http_status: int | None = None,
    error_message: str = "",
    timeout: bool = False,
    connection_error: bool = False,
    output_validation_failed: bool = False,
) -> dict[str, Any]:
    """Classify an adapter error into a structured failure category.

    Returns a dict with ``category``, ``retryable``, ``description``,
    ``recovery_action``, and the original diagnostic context.
    """
    if output_validation_failed:
        category = "invalid_output"
    elif timeout:
        category = "network_timeout"
    elif connection_error:
        category = "network_unreachable"
    elif http_status == 401 or http_status == 403:
        category = "auth_failure"
    elif http_status == 429:
        category = "rate_limited"
    elif http_status is not None and http_status >= 500:
        category = "server_error"
    elif http_status == 400:
        if "context" in error_message.lower() or "length" in error_message.lower() or "token" in error_message.lower():
            category = "context_length_exceeded"
        elif "content" in error_message.lower() or "policy" in error_message.lower() or "safety" in error_message.lower():
            category = "content_filtered"
        elif "model" in error_message.lower():
            category = "model_unavailable"
        else:
            category = "unknown"
    elif http_status == 404:
        category = "model_unavailable"
    elif exception is not None:
        exc_name = type(exception).__name__
        exc_msg = str(exception).lower()
        if "timeout" in exc_name.lower() or "timeout" in exc_msg:
            category = "network_timeout"
        elif "connection" in exc_name.lower() or "refused" in exc_msg or "reset" in exc_msg:
            category = "network_unreachable"
        elif "auth" in exc_msg or "credential" in exc_msg or "unauthorized" in exc_msg:
            category = "auth_failure"
        else:
            category = "unknown"
    else:
        category = "unknown"

    info = FAILURE_CATEGORIES.get(category, FAILURE_CATEGORIES["unknown"])

    return {
        "schema_version": "0.1.0-draft",
        "classification_kind": "adapter_failure_classification",
        "classified_at": utc_now(),
        "category": category,
        "retryable": info["retryable"],
        "description": info["description"],
        "recovery_action": info["recovery_action"],
        "diagnostic": {
            "exception_type": type(exception).__name__ if exception else None,
            "exception_message": str(exception) if exception else error_message,
            "http_status": http_status,
            "timeout_detected": timeout,
            "connection_error_detected": connection_error,
            "output_validation_failed": output_validation_failed,
        },
        "limitations": [
            "Classification is heuristic based on error surface signals.",
            "Providers may change error formats; heuristics may need updating.",
            "Not all failure modes are distinguishable from available diagnostics.",
        ],
    }


def build_failure_recovery_plan(
    classification: dict[str, Any],
    *,
    max_retries: int = 3,
    current_retry: int = 0,
) -> dict[str, Any]:
    """Build a recovery plan based on a failure classification.

    Returns a dict with ``should_retry``, ``retry_delay_seconds``,
    ``action``, and ``reason``.
    """
    category = classification["category"]
    retryable = classification["retryable"]
    recovery_action = classification["recovery_action"]

    if not retryable:
        return {
            "should_retry": False,
            "retry_delay_seconds": 0,
            "action": recovery_action,
            "reason": f"failure category '{category}' is not retryable",
        }

    if current_retry >= max_retries:
        return {
            "should_retry": False,
            "retry_delay_seconds": 0,
            "action": "abort",
            "reason": f"max retries ({max_retries}) exhausted",
        }

    # Exponential backoff
    base_delays = {
        "network_timeout": 2.0,
        "rate_limited": 5.0,
        "server_error": 1.0,
        "invalid_output": 0.5,
    }
    base = base_delays.get(category, 1.0)
    delay = base * (2 ** current_retry)

    return {
        "should_retry": True,
        "retry_delay_seconds": delay,
        "action": recovery_action,
        "reason": f"retryable failure category '{category}', attempt {current_retry + 1}/{max_retries}",
    }
