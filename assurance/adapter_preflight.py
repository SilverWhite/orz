from __future__ import annotations

import uuid
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import utc_now

PREFLIGHT_SCHEMA = "adapter-preflight-receipt-v0.1.schema.json"

# Retired 2026-07-24 15:59 UTC per DeepSeek official announcement.
# https://api-docs.deepseek.com/guides/model_deprecation/
_DEPRECATED_DEEPSEEK_ALIASES: frozenset[str] = frozenset({
    "deepseek-chat",
    "deepseek-reasoner",
})

# Thinking mode ignores these sampling parameters silently on the
# DeepSeek API side — local preflight must reject declaring both
# so we never record unexecuted sampling settings as provenance.
_THINKING_IGNORED_PARAMS: frozenset[str] = frozenset({
    "temperature",
    "top_p",
    "presence_penalty",
    "frequency_penalty",
})


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
    thinking_mode: bool = False,
    sampling_params: list[str] | None = None,
    probed_model_available: bool | None = None,
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

    # GAK-07: reject deprecated DeepSeek model aliases.
    # deepseek-chat / deepseek-reasoner retired 2026-07-24 15:59 UTC.
    model_lower = model_id.lower()
    if provider == "deepseek" and model_lower in _DEPRECATED_DEEPSEEK_ALIASES:
        checks["model_id_not_deprecated"] = False
        errors.append(
            f"model_id {model_id!r} is a deprecated alias that was retired "
            "on 2026-07-24; use deepseek-v4-pro or deepseek-v4-flash instead"
        )
    else:
        checks["model_id_not_deprecated"] = True

    # GAK-08: thinking mode silently ignores sampling parameters on
    # the DeepSeek API.  Reject profiles that declare both so we
    # never record unexecuted sampling settings as provenance.
    if thinking_mode and sampling_params:
        conflicting = [p for p in sampling_params if p in _THINKING_IGNORED_PARAMS]
        if conflicting:
            checks["thinking_sampling_exclusive"] = False
            errors.append(
                f"thinking mode ignores sampling parameters "
                f"{sorted(conflicting)}; remove them or disable thinking mode"
            )
        else:
            checks["thinking_sampling_exclusive"] = True
    else:
        checks["thinking_sampling_exclusive"] = True

    # GAK-09: optional /models capability discovery.
    # probed_model_available is None when network is unavailable
    # (trinary: True=known-available, False=known-unavailable,
    #  None=not-probed). Only fail when we probed and the model
    # was not found.
    if probed_model_available is not None:
        checks["model_id_known_by_provider"] = probed_model_available
        if not probed_model_available:
            errors.append(
                f"model_id {model_id!r} was not found in provider /models "
                "response; it may be deprecated, misspelled, or unavailable"
            )
    # When not probed the key is absent — schema does not require it.

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
            "thinking_mode": thinking_mode,
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
    # GAK-07: deprecated model and pass cannot coexist
    if (
        not receipt["checks"]["model_id_not_deprecated"]
        and receipt["preflight_passed"]
    ):
        errors.append("deprecated model but preflight passed — inconsistent")
    # GAK-08: thinking+sampling conflict and pass cannot coexist
    if (
        not receipt["checks"]["thinking_sampling_exclusive"]
        and receipt["preflight_passed"]
    ):
        errors.append(
            "thinking/sampling conflict but preflight passed — inconsistent"
        )
    # GAK-09: probed model unavailable and pass cannot coexist
    model_known = receipt["checks"].get("model_id_known_by_provider")
    if model_known is False and receipt["preflight_passed"]:
        errors.append(
            "model_id not found at provider but preflight passed — inconsistent"
        )

    return {
        "valid": not errors,
        "preflight_passed": receipt["preflight_passed"],
        "errors": errors,
        "checks": receipt["checks"],
    }


def probe_model_availability(
    *,
    model_id: str,
    provider: str = "deepseek",
    timeout_seconds: int = 15,
) -> bool | None:
    """Check whether *model_id* is present in the provider's /models list.

    Returns ``True`` if the model was found, ``False`` if the model was
    not found, or ``None`` if the provider endpoint is unreachable
    (network unavailable — this is NOT treated as a failure).

    Only supports DeepSeek currently.  Raises :exc:`AssuranceError` for
    unsupported providers.
    """
    if provider != "deepseek":
        raise AssuranceError(
            f"model availability probing is only supported for deepseek, "
            f"not {provider!r}"
        )

    import json as _json
    import urllib.error as _urllib_error
    import urllib.request as _urllib_request

    models_url = "https://api.deepseek.com/models"
    try:
        req = _urllib_request.Request(
            models_url,
            method="GET",
            headers={"Accept": "application/json"},
        )
        with _urllib_request.urlopen(req, timeout=timeout_seconds) as resp:
            data = _json.loads(resp.read().decode("utf-8"))
    except (_urllib_error.URLError, OSError, ValueError):
        # Network unavailable or response unparseable — not a failure.
        return None

    if not isinstance(data, dict):
        return None

    models = data.get("data")
    if not isinstance(models, list):
        return None

    model_ids = {
        item["id"]
        for item in models
        if isinstance(item, dict) and isinstance(item.get("id"), str)
    }
    return model_id in model_ids
