from __future__ import annotations

import uuid
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import utc_now

PREFLIGHT_SCHEMA = "adapter-preflight-receipt-v0.1.schema.json"


def run_adapter_preflight(
    *,
    adapter_id: str,
    provider: str,
    model_id: str,
    endpoint: str,
    conversation_id: str,
    run_id: str,
    timeout_seconds: int = 60,
    credential_target: str | None = None,
    require_structured_output: bool = True,
    require_streaming: bool = False,
    ipg_receipt_valid: bool = False,
    gate_chain_complete: bool = False,
    output_schema_known: bool = False,
) -> dict[str, Any]:
    """Run a preflight check on an adapter before any model call.

    This is a mechanical readiness check — it does not call the model,
    read credentials from disk, or make network requests.  It validates
    that the adapter configuration is consistent with the manifest and
    that all prerequisite gates have been evaluated.

    Callers must supply the factual state of each check; this function
    only records and schema-validates them.
    """
    errors: list[str] = []
    checks: dict[str, bool] = {}

    # Endpoint canonicalization
    try:
        from .endpoint_canonicalizer import canonicalize_network_endpoint

        canonical_endpoint = canonicalize_network_endpoint(endpoint)
        checks["endpoint_canonicalized"] = True
    except AssuranceError as exc:
        checks["endpoint_canonicalized"] = False
        errors.append(f"endpoint canonicalization failed: {exc}")

    # Credential availability (probe only — never read value)
    checks["credential_readable"] = credential_target is not None
    checks["credential_not_persisted"] = True  # enforced by architecture

    # Gate chain
    checks["gate_chain_complete"] = gate_chain_complete
    checks["ipg_receipt_valid"] = ipg_receipt_valid

    if not gate_chain_complete:
        errors.append("gate chain incomplete: IPG, tool availability, and source visibility must all be evaluated before adapter call")

    if not ipg_receipt_valid:
        errors.append("IPG receipt is not valid; adapter call would be blocked")

    # Adapter capability alignment
    adapter_caps_ok = True
    if require_structured_output and not output_schema_known:
        adapter_caps_ok = False
        errors.append("adapter requires structured output but output schema is not known")
    checks["adapter_capabilities_match_manifest"] = adapter_caps_ok
    checks["output_schema_known"] = output_schema_known

    preflight_passed = not errors

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "adapter_preflight_receipt",
        "receipt_id": f"APF-{uuid.uuid4().hex.upper()}",
        "conversation_id": conversation_id,
        "run_id": run_id,
        "created_at": utc_now(),
        "adapter_id": adapter_id,
        "preflight_passed": preflight_passed,
        "checks": checks,
        "manifest_checks": {
            "provider": provider,
            "model_id": model_id,
            "endpoint": endpoint,
            "timeout_seconds": timeout_seconds,
            "structured_output": require_structured_output,
            "streaming": require_streaming,
        },
        "errors": errors,
        "limitations": [
            "Preflight is a mechanical configuration check, not a live connectivity test.",
            "Credential readability is probed via expectation, not actual read.",
            "It cannot detect provider outages, model deprecation, or rate-limit status.",
        ],
    }
    validate_contract(receipt, PREFLIGHT_SCHEMA, label="adapter preflight receipt")
    return receipt


def verify_adapter_preflight(receipt: dict[str, Any]) -> dict[str, Any]:
    """Independently verify a preflight receipt for structural validity."""
    errors: list[str] = []
    try:
        validate_contract(receipt, PREFLIGHT_SCHEMA, label="adapter preflight receipt")
    except AssuranceError as exc:
        return {"valid": False, "errors": [str(exc)]}

    # Logical consistency checks
    if receipt["preflight_passed"] and receipt["errors"]:
        errors.append("preflight passed but errors are present")
    if not receipt["preflight_passed"] and not receipt["errors"]:
        errors.append("preflight failed but no errors recorded")
    if receipt["checks"]["ipg_receipt_valid"] and not receipt["checks"]["gate_chain_complete"]:
        errors.append("IPG valid but gate chain incomplete — inconsistent")

    return {
        "valid": not errors,
        "preflight_passed": receipt["preflight_passed"],
        "errors": errors,
        "checks": receipt["checks"],
    }
