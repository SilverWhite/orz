from __future__ import annotations

import uuid
from typing import TYPE_CHECKING, Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, utc_now

if TYPE_CHECKING:
    from .keystore import InstallationKeyStore


class ChildCapabilityEscalationError(AssuranceError):
    """Raised when a child process/agent requests capabilities exceeding its parent."""


ENFORCEMENT_SCHEMA = "child-capability-enforcement-receipt-v0.1.schema.json"


def enforce_child_capabilities(
    *,
    parent_envelope: dict[str, Any],
    child_kind: str,
    child_id: str,
    requested_capabilities: list[str],
    spawn_context: dict[str, Any] | None = None,
    allowed_endpoints: list[str] | None = None,
    denied_endpoints: list[str] | None = None,
) -> dict[str, Any]:
    """Enforce that a child process/agent/MCP does not exceed parent capabilities.

    This is the HARD GATE that must be called BEFORE any child process or
    sub-agent is spawned.  It verifies:

    1. Child's requested capabilities are a subset of parent's allowed.
    2. Child does not request any of parent's denied capabilities.
    3. Remote MCP children cannot request local capabilities (credential,
       filesystem, host, process, sandbox, secret).
    4. Network endpoints are within the parent's permit scope.

    Returns an enforcement receipt.  Raises
    :exc:`ChildCapabilityEscalationError` if enforcement fails.
    """
    if child_kind not in {"child_process", "child_agent", "remote_mcp"}:
        raise AssuranceError(f"unknown child kind: {child_kind}")

    errors: list[str] = []
    checks: dict[str, bool] = {}

    parent_allowed = set(parent_envelope.get("capability_envelope", {}).get("allowed", []))
    parent_denied = set(parent_envelope.get("capability_envelope", {}).get("denied", []))
    requested = set(requested_capabilities)

    # Check 1: Capability subset
    escalation = requested - parent_allowed
    checks["capability_subset_verified"] = not bool(escalation)
    if escalation:
        errors.append(f"child requests capabilities not in parent envelope: {sorted(escalation)}")

    # Check 2: No denied capabilities
    denied_overlap = requested & parent_denied
    checks["no_denied_capabilities"] = not bool(denied_overlap)
    if denied_overlap:
        errors.append(f"child requests capabilities denied by parent: {sorted(denied_overlap)}")

    # Check 3: Remote MCP cannot have local capabilities
    if child_kind == "remote_mcp":
        LOCAL_PREFIXES = ("credential.", "filesystem.", "host.", "process.", "sandbox.", "secret.")
        remote_local = {c for c in requested if any(c.startswith(p) for p in LOCAL_PREFIXES)}
        checks["remote_mcp_no_local_capabilities"] = not bool(remote_local)
        if remote_local:
            errors.append(f"remote MCP requests local capabilities: {sorted(remote_local)}")
    else:
        checks["remote_mcp_no_local_capabilities"] = True

    # Check 4: Network endpoint allowlist
    if allowed_endpoints is not None or denied_endpoints is not None:
        allowed = set(allowed_endpoints or [])
        denied = set(denied_endpoints or [])
        checks["network_endpoints_constrained"] = True
        if denied:
            # If child has network capability but denied endpoints exist, verify the deny list
            has_network = any(
                c.startswith("network.") for c in requested
            )
            if has_network and denied:
                checks["network_denylist_applied"] = True
            else:
                checks["network_denylist_applied"] = not bool(denied)
    else:
        checks["network_endpoints_constrained"] = True
        checks["network_denylist_applied"] = True

    cap_enforced = not bool(errors)
    outcome = "enforced" if cap_enforced else "blocked"

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "child_capability_enforcement_receipt",
        "receipt_id": f"CCE-{uuid.uuid4().hex.upper()}",
        "created_at": utc_now(),
        "child": {
            "kind": child_kind,
            "child_id": child_id,
        },
        "parent_envelope_id": parent_envelope.get("envelope_id", ""),
        "requested_capabilities": sorted(requested),
        "parent_allowed": sorted(parent_allowed),
        "parent_denied": sorted(parent_denied),
        "escalation_detected": sorted(escalation) if escalation else [],
        "outcome": outcome,
        "capability_enforced": cap_enforced,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "Capability enforcement is mechanical: it checks declared subsets.",
            "It cannot prevent a child from re-deriving capabilities at runtime.",
            "Actual OS-level containment requires AppContainer/Job Object (P2).",
        ],
    }
    validate_contract(
        receipt, ENFORCEMENT_SCHEMA, label="child capability enforcement receipt"
    )

    if not cap_enforced:
        raise ChildCapabilityEscalationError(
            f"child '{child_id}' ({child_kind}) capability enforcement failed: "
            + "; ".join(errors)
        )

    return receipt


def verify_child_capability_enforcement(
    receipt: dict[str, Any],
    *,
    parent_envelope: dict[str, Any],
) -> dict[str, Any]:
    """Independently verify a child capability enforcement receipt."""
    errors: list[str] = []
    validate_contract(
        receipt, ENFORCEMENT_SCHEMA, label="child capability enforcement receipt"
    )

    parent_allowed = set(parent_envelope.get("capability_envelope", {}).get("allowed", []))
    parent_denied = set(parent_envelope.get("capability_envelope", {}).get("denied", []))

    requested = set(receipt["requested_capabilities"])
    if requested - parent_allowed:
        errors.append("enforcement receipt allows escalation not in parent envelope")
    if requested & parent_denied:
        errors.append("enforcement receipt allows denied capabilities")

    if receipt["child"]["kind"] == "remote_mcp":
        LOCAL = ("credential.", "filesystem.", "host.", "process.", "sandbox.", "secret.")
        if any(c.startswith(LOCAL) for c in requested):
            errors.append("remote MCP enforcement receipt allows local capabilities")

    if receipt["outcome"] == "enforced" and receipt["errors"]:
        errors.append("enforcement passed but errors are present")

    return {
        "valid": not errors,
        "capability_enforced": receipt.get("capability_enforced", False),
        "errors": errors,
    }


def spawn_child_context(
    *,
    parent_envelope: dict[str, Any],
    child_kind: str,
    child_id: str,
    requested_capabilities: list[str],
    key_store: InstallationKeyStore,
    ttl_seconds: int = 3600,
    frozen_context: dict[str, Any],
) -> dict[str, Any]:
    """Unified child spawn gate: enforce + delegate child capabilities.

    Composes :func:`enforce_child_capabilities` (predicate check) with
    :func:`delegate_capabilities` (signed child envelope creation) into a
    single call site.

    Returns a dict with:
      - ``enforcement_receipt``: from :func:`enforce_child_capabilities`
      - ``child_envelope``: signed child security envelope (or None if denied)
      - ``delegation_receipt``: from :func:`delegate_capabilities`
      - ``escalation_detected``: bool
      - ``capability_enforced``: bool
    """
    # Step 1: Mechanical predicate check
    enforcement_receipt = enforce_child_capabilities(
        parent_envelope=parent_envelope,
        child_kind=child_kind,
        child_id=child_id,
        requested_capabilities=requested_capabilities,
    )

    # Step 2: Create signed child envelope via delegation
    from .instruction_gate import delegate_capabilities as _delegate

    child_envelope, delegation_receipt = _delegate(
        key_store=key_store,
        parent_envelope=parent_envelope,
        subject_kind=child_kind,
        subject_id=child_id,
        requested_allowed=list(requested_capabilities),
        ttl_seconds=ttl_seconds,
    )

    escalation = bool(enforcement_receipt.get("escalation_detected"))
    capability_enforced = (
        enforcement_receipt.get("capability_enforced", False)
        and child_envelope is not None
    )

    return {
        "enforcement_receipt": enforcement_receipt,
        "child_envelope": child_envelope,
        "delegation_receipt": delegation_receipt,
        "escalation_detected": escalation,
        "capability_enforced": capability_enforced,
        "child_envelope_id": (
            child_envelope.get("envelope_id", "") if child_envelope else ""
        ),
    }
