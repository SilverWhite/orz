from __future__ import annotations

import uuid
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, utc_now


class NetworkPermitBlockedError(AssuranceError):
    """Raised when a network request is blocked by the permit gateway."""


ENDPOINT_CATEGORIES = {
    "llm_provider": "Model API endpoints (DeepSeek, Anthropic, OpenAI, etc.)",
    "web_fetch": "General web page retrieval",
    "mcp_server": "MCP tool server endpoints",
    "git_remote": "Git remote operations (clone, fetch, push)",
    "package_registry": "Package registries (PyPI, npm, crates.io)",
    "other": "Any network endpoint not in the above categories",
}


def evaluate_network_permit(
    *,
    endpoint: str,
    category: str,
    conversation_id: str,
    attempt: int,
    turn: int,
    previous_http_status: int | None = None,
    request_body_sha256: str | None = None,
    allowed_categories: set[str] | None = None,
    allowed_endpoints: set[str] | None = None,
    max_attempts_per_turn: int = 3,
) -> dict[str, Any]:
    """Evaluate a single network permit request.

    This is the unified gate for ALL network requests: model APIs,
    web fetches, MCP servers, Git operations, and package registries.

    Returns a permit receipt.  Raises :exc:`NetworkPermitBlockedError`
    if the request is denied.
    """
    errors: list[str] = []
    checks: dict[str, bool] = {}

    # Category validation
    if category not in ENDPOINT_CATEGORIES:
        errors.append(f"unknown endpoint category: {category}")
        checks["category_valid"] = False
    else:
        checks["category_valid"] = True

    # Attempt tracking
    checks["first_attempt_has_no_previous_status"] = (
        attempt > 1 or previous_http_status is None
    )
    if attempt == 1 and previous_http_status is not None:
        errors.append("first attempt must not carry a previous HTTP status")

    checks["retry_has_previous_status"] = (
        attempt == 1 or previous_http_status is not None
    )
    if attempt > 1 and previous_http_status is None:
        errors.append("retry attempt must carry previous HTTP status")

    # Attempt budget
    checks["within_attempt_budget"] = attempt <= max_attempts_per_turn
    if attempt > max_attempts_per_turn:
        errors.append(f"attempt {attempt} exceeds max {max_attempts_per_turn} per turn")

    # Endpoint canonicalization
    try:
        from .endpoint_canonicalizer import canonicalize_network_endpoint

        canonical = canonicalize_network_endpoint(endpoint)
        checks["endpoint_canonicalized"] = True
    except AssuranceError as exc:
        errors.append(f"endpoint rejected: {exc}")
        checks["endpoint_canonicalized"] = False
        canonical = endpoint

    # Category allowlist
    if allowed_categories is not None:
        checks["category_allowed"] = category in allowed_categories
        if category not in allowed_categories:
            errors.append(f"category '{category}' not in allowed set")
    else:
        checks["category_allowed"] = True

    # Endpoint allowlist
    if allowed_endpoints is not None and canonical != endpoint:
        # Check if the canonical endpoint or any prefix matches
        endpoint_allowed = any(
            canonical.startswith(allowed) for allowed in allowed_endpoints
        )
        checks["endpoint_allowed"] = endpoint_allowed
        if not endpoint_allowed:
            errors.append(f"endpoint '{canonical}' not in allowed set")
    else:
        checks["endpoint_allowed"] = True

    permit_granted = not errors
    decision = "allow" if permit_granted else "block"

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "network_permit_receipt",
        "permit_id": f"NET-{uuid.uuid4().hex.upper()}",
        "conversation_id": conversation_id,
        "created_at": utc_now(),
        "request": {
            "endpoint": endpoint,
            "canonical_endpoint": canonical,
            "category": category,
            "turn": turn,
            "attempt": attempt,
            "previous_http_status": previous_http_status,
            "request_body_sha256": request_body_sha256 or "",
        },
        "decision": decision,
        "permit_granted": permit_granted,
        "one_shot": True,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "Network permit is a mechanical allowlist check, not a runtime firewall.",
            "It cannot detect protocol-level bypasses, DNS rebinding, or TLS interception.",
            "One-shot permit: each attempt requires a new evaluation.",
        ],
    }
    validate_contract(
        receipt,
        "network-permit-receipt-v0.1.schema.json",
        label="network permit receipt",
    )

    if not permit_granted:
        raise NetworkPermitBlockedError(
            f"network permit denied for {category} endpoint '{canonical}': "
            + "; ".join(errors)
        )

    return receipt


def build_network_permit_policy(
    *,
    policy_name: str = "default",
    mode: str = "discussion",
    allowed_categories: set[str] | None = None,
    allowed_endpoints: set[str] | None = None,
    denied_endpoints: set[str] | None = None,
    max_attempts_per_turn: int = 3,
    require_permit_for_all: bool = True,
) -> dict[str, Any]:
    """Build a network permit policy for a conversation.

    Three standard modes:
      - ``discussion``: read-only web fetch + Git clone, no LLM API, no MCP
      - ``guarded``: LLM API (allowlisted endpoints only), web fetch, Git
      - ``strict``: LLM API (single fixed endpoint), no web, no Git, no MCP
    """
    if mode == "discussion":
        default_categories = {"web_fetch", "git_remote"}
    elif mode == "guarded":
        default_categories = {"llm_provider", "web_fetch", "git_remote"}
    elif mode == "strict":
        default_categories = {"llm_provider"}
    else:
        raise AssuranceError(f"unknown network permit mode: {mode}")

    policy = {
        "schema_version": "0.1.0-draft",
        "policy_kind": "network_permit_policy",
        "policy_name": policy_name,
        "mode": mode,
        "created_at": utc_now(),
        "allowed_categories": sorted(allowed_categories or default_categories),
        "allowed_endpoints": sorted(allowed_endpoints or []),
        "denied_endpoints": sorted(denied_endpoints or []),
        "max_attempts_per_turn": max_attempts_per_turn,
        "require_permit_for_all": require_permit_for_all,
        "endpoint_categories": {
            k: v for k, v in ENDPOINT_CATEGORIES.items()
        },
        "notes": [
            "Each network request consumes one one-shot permit.",
            "Retries require new permits with previous HTTP status.",
            "First attempt must not carry a previous status.",
        ],
    }
    return policy


def verify_network_permit_receipt(
    receipt: dict[str, Any],
    *,
    policy: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Independently verify a network permit receipt against policy."""
    errors: list[str] = []
    validate_contract(
        receipt, "network-permit-receipt-v0.1.schema.json", label="network permit receipt"
    )

    if not receipt["permit_granted"] and not receipt["errors"]:
        errors.append("permit denied but no errors recorded")

    if receipt["request"]["attempt"] == 1 and receipt["request"]["previous_http_status"] is not None:
        errors.append("first attempt has previous HTTP status")

    if policy is not None:
        category = receipt["request"]["category"]
        if category not in policy.get("allowed_categories", []):
            errors.append(f"category '{category}' not in policy allowed set")

    return {
        "valid": not errors,
        "permit_granted": receipt["permit_granted"],
        "errors": errors,
    }
