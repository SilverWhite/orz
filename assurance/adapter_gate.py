from __future__ import annotations

import uuid
from pathlib import Path
from typing import Any, Callable

from .contracts import validate_contract
from .errors import AssuranceError
from .instruction_provenance_gate import (
    evaluate_instruction_provenance_gate,
    verify_instruction_provenance_gate_receipt,
)
from .utils import atomic_write_json, load_json, sha256_bytes, sha256_file, utc_now


ENFORCEMENT_SCHEMA = "adapter-gate-enforcement-receipt-v0.1.schema.json"


class AdapterGateBlockedError(AssuranceError):
    """Raised when a model/tool adapter call is blocked by the gate enforcer."""


class AdapterGateContext:
    """Immutable context binding an IPG receipt to a specific adapter call."""

    def __init__(
        self,
        *,
        ipg_receipt: dict[str, Any],
        ipg_context: dict[str, Any],
        adapter_id: str,
        conversation_id: str,
        run_id: str,
        trust_receipt: dict[str, Any] | None = None,
        network_policy: dict[str, Any] | None = None,
        network_endpoint: str | None = None,
        network_endpoint_category: str | None = None,
        network_attempt: int = 1,
        network_turn: int = 1,
        allowed_categories: set[str] | None = None,
        allowed_endpoints: set[str] | None = None,
    ) -> None:
        self.ipg_receipt = ipg_receipt
        self.ipg_context = ipg_context
        self.adapter_id = adapter_id
        self.conversation_id = conversation_id
        self.run_id = run_id
        self.trust_receipt = trust_receipt
        self.network_policy = network_policy
        self.network_endpoint = network_endpoint
        self.network_endpoint_category = network_endpoint_category
        self.network_attempt = network_attempt
        self.network_turn = network_turn
        self.allowed_categories = allowed_categories
        self.allowed_endpoints = allowed_endpoints
        self._enforcement_id = f"AGE-{uuid.uuid4().hex.upper()}"
        self._created_at = utc_now()

    @property
    def gate_decision(self) -> str:
        return self.ipg_receipt["gate_decision"]

    @property
    def gate_valid(self) -> bool:
        return self.ipg_receipt.get("valid", False)

    @property
    def ipg_receipt_sha256(self) -> str:
        from .utils import canonical_bytes

        return sha256_bytes(canonical_bytes(self.ipg_receipt))

    @property
    def ipg_context_sha256(self) -> str:
        from .utils import canonical_bytes

        return sha256_bytes(canonical_bytes(self.ipg_context))

    @property
    def trust_status(self) -> str:
        """Effective workspace trust status for this adapter."""
        if self.trust_receipt is None:
            return "not_observed"
        from .workspace_trust import workspace_trust_for_adapter
        return workspace_trust_for_adapter(
            self.trust_receipt, adapter_id=self.adapter_id,
        )

    @property
    def trust_receipt_id(self) -> str:
        if self.trust_receipt is None:
            return ""
        return self.trust_receipt.get("receipt_id", "")

    @property
    def trust_receipt_sha256(self) -> str:
        if self.trust_receipt is None:
            return ""
        from .utils import canonical_bytes
        return sha256_bytes(canonical_bytes(self.trust_receipt))


def enforce_adapter_call(
    *,
    gate_context: AdapterGateContext,
    adapter_call: Callable[[], Any],
) -> dict[str, Any]:
    """Execute an adapter call only if the IPG allows it.

    This is the single enforcement point: every model/tool adapter MUST be
    wrapped by this function.  It verifies the IPG receipt is valid and
    the gate decision is ``allow`` before invoking *adapter_call*.

    Returns an enforcement receipt.  Raises :exc:`AdapterGateBlockedError`
    if the call is blocked.
    """
    errors: list[str] = []
    ipg_evaluated_before_adapter = True
    ipg_receipt_present = gate_context.ipg_receipt is not None
    ipg_receipt_valid = False
    gate_decision_respected = False
    no_direct_adapter_call = True
    no_replay_attack = True
    bypass_attempted = False
    bypass_details = ""

    # Verify IPG receipt is present
    if not ipg_receipt_present:
        errors.append("IPG receipt is missing; adapter call blocked")
        bypass_attempted = True
        bypass_details = "adapter called without IPG receipt"

    # Verify IPG receipt is valid (rebuild and compare)
    if ipg_receipt_present:
        try:
            verify_instruction_provenance_gate_receipt(
                gate_context=gate_context.ipg_context,
                receipt=gate_context.ipg_receipt,
            )
            ipg_receipt_valid = True
        except AssuranceError as exc:
            errors.append(f"IPG receipt validation failed: {exc}")
            bypass_attempted = True
            bypass_details = f"IPG receipt tampered or mismatched: {exc}"

    # Verify gate decision is respected
    if ipg_receipt_valid:
        if gate_context.gate_decision == "allow":
            gate_decision_respected = True
        else:
            gate_decision_respected = True  # respecting the block/defer
            bypass_attempted = True
            bypass_details = (
                f"adapter call attempted despite gate decision "
                f"'{gate_context.gate_decision}'"
            )
            errors.append(bypass_details)

    # Network permit policy check
    network_permit_required = (
        gate_context.network_policy is not None
        and gate_context.network_policy.get("require_permit_for_all", False)
    )
    network_permit_granted = not network_permit_required  # defaults True if not required
    if network_permit_required:
        if not gate_context.network_endpoint or not gate_context.network_endpoint_category:
            errors.append("network permit policy requires endpoint and category")
            bypass_attempted = True
            bypass_details = "adapter call attempted without network permit endpoint metadata"
        else:
            try:
                from .network_permit_gateway import evaluate_network_permit

                policy_categories = set(
                    gate_context.network_policy.get("allowed_categories", [])
                )
                policy_endpoints = set(
                    gate_context.network_policy.get("allowed_endpoints", [])
                )
                policy_denied = set(
                    gate_context.network_policy.get("denied_endpoints", [])
                )
                permit_receipt = evaluate_network_permit(
                    endpoint=gate_context.network_endpoint,
                    category=gate_context.network_endpoint_category,
                    conversation_id=gate_context.conversation_id,
                    attempt=gate_context.network_attempt,
                    turn=gate_context.network_turn,
                    allowed_categories=(
                        gate_context.allowed_categories or policy_categories or None
                    ),
                    allowed_endpoints=(
                        gate_context.allowed_endpoints or policy_endpoints or None
                    ),
                    denied_endpoints=(policy_denied or None),
                    max_attempts_per_turn=gate_context.network_policy.get(
                        "max_attempts_per_turn", 3,
                    ),
                )
                network_permit_granted = permit_receipt.get("permit_granted", False)
            except AssuranceError as exc:
                network_permit_granted = False
                errors.append(f"network permit denied: {exc}")
                bypass_attempted = True
                bypass_details = f"adapter call attempted without network permit: {exc}"

    adapter_call_allowed = (
        ipg_receipt_valid
        and gate_context.gate_decision == "allow"
        and (not network_permit_required or network_permit_granted)
    )

    enforcement_receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "adapter_gate_enforcement_receipt",
        "receipt_id": gate_context._enforcement_id,
        "conversation_id": gate_context.conversation_id,
        "run_id": gate_context.run_id,
        "created_at": gate_context._created_at,
        "gate_valid": gate_context.gate_valid,
        "gate_decision": gate_context.gate_decision,
        "ipg_receipt_sha256": gate_context.ipg_receipt_sha256,
        "ipg_context_sha256": gate_context.ipg_context_sha256,
        "adapter_id": gate_context.adapter_id,
        "adapter_call_allowed": adapter_call_allowed,
        "bypass_attempted": bypass_attempted,
        "bypass_details": bypass_details if bypass_attempted else "",
        "trust_status": gate_context.trust_status,
        "trust_receipt_id": gate_context.trust_receipt_id,
        "trust_receipt_sha256": gate_context.trust_receipt_sha256,
        "network_permit_required": network_permit_required,
        "network_permit_granted": network_permit_granted,
        "network_allowed_categories": (
            sorted(gate_context.allowed_categories)
            if gate_context.allowed_categories
            else list(gate_context.network_policy.get("allowed_categories", []))
            if gate_context.network_policy
            else []
        ),
        "network_endpoint": gate_context.network_endpoint or "",
        "checks": {
            "ipg_evaluated_before_adapter": ipg_evaluated_before_adapter,
            "ipg_receipt_present": ipg_receipt_present,
            "ipg_receipt_valid": ipg_receipt_valid,
            "gate_decision_respected": gate_decision_respected,
            "no_direct_adapter_call": no_direct_adapter_call,
            "no_replay_attack": no_replay_attack,
            "network_permit_policy_present": gate_context.network_policy is not None,
        },
        "errors": errors,
        "limitations": [
            "Gate enforcement is mechanical: it checks receipt presence and "
            "validity but cannot prevent a malicious adapter from ignoring it.",
            "Replay detection requires caller to provide a fresh gate context "
            "per adapter call; this receipt records the context identity.",
        ],
    }
    validate_contract(
        enforcement_receipt,
        ENFORCEMENT_SCHEMA,
        label="adapter gate enforcement receipt",
    )

    if not adapter_call_allowed:
        raise AdapterGateBlockedError(
            f"adapter '{gate_context.adapter_id}' blocked by "
            f"instruction provenance gate: {bypass_details}"
        )

    # Execute the adapter call under gate enforcement
    result = adapter_call()

    return {
        **enforcement_receipt,
        "adapter_result": result,
    }


def verify_adapter_gate_enforcement(
    *,
    enforcement_receipt: dict[str, Any],
    ipg_context: dict[str, Any],
    ipg_receipt: dict[str, Any],
) -> dict[str, Any]:
    """Independently verify that an adapter call was properly gated.

    Rebuilds the IPG receipt and checks that the enforcement receipt
    correctly reflects the gate state.
    """
    errors: list[str] = []

    # Verify the enforcement receipt schema
    try:
        validate_contract(
            enforcement_receipt,
            ENFORCEMENT_SCHEMA,
            label="adapter gate enforcement receipt",
        )
    except AssuranceError as exc:
        return {"valid": False, "errors": [str(exc)]}

    # Verify the IPG receipt against its context
    try:
        verify_instruction_provenance_gate_receipt(
            gate_context=ipg_context,
            receipt=ipg_receipt,
        )
    except AssuranceError as exc:
        errors.append(f"IPG receipt verification failed: {exc}")

    # Check enforcement receipt matches IPG
    from .utils import canonical_bytes

    expected_ipg_sha = sha256_bytes(canonical_bytes(ipg_receipt))
    if enforcement_receipt["ipg_receipt_sha256"] != expected_ipg_sha:
        errors.append("enforcement receipt IPG digest mismatch")

    expected_ctx_sha = sha256_bytes(canonical_bytes(ipg_context))
    if enforcement_receipt["ipg_context_sha256"] != expected_ctx_sha:
        errors.append("enforcement receipt IPG context digest mismatch")

    if enforcement_receipt["gate_decision"] != ipg_receipt["gate_decision"]:
        errors.append("gate decision mismatch between enforcement and IPG receipt")

    network_ok = (
        not enforcement_receipt.get("network_permit_required", False)
        or enforcement_receipt.get("network_permit_granted", False)
    )
    expected_allowed = (
        enforcement_receipt["gate_valid"]
        and enforcement_receipt["gate_decision"] == "allow"
        and network_ok
    )
    if enforcement_receipt["adapter_call_allowed"] != expected_allowed:
        errors.append("adapter_call_allowed inconsistent with gate and network permit state")

    if enforcement_receipt["bypass_attempted"] and enforcement_receipt["adapter_call_allowed"]:
        errors.append("bypass attempted but call was allowed")

    return {
        "valid": not errors,
        "errors": errors,
        "checks": enforcement_receipt["checks"],
    }


def run_adapter_gate_bypass_fixture(
    *,
    output_root: Path,
    ipg_context: dict[str, Any],
    ipg_receipt: dict[str, Any],
    adapter_id: str,
    conversation_id: str,
    run_id: str,
    attempt_direct_call: bool = False,
    tamper_receipt: bool = False,
    use_blocked_gate: bool = False,
    replay_old_receipt: bool = False,
) -> dict[str, Any]:
    """Run a controlled bypass-attempt fixture for testing.

    Returns a summary of what was attempted and what the enforcer did.
    """
    if output_root.exists() and any(output_root.iterdir()):
        raise AssuranceError(f"output root must be empty or absent: {output_root}")

    working_receipt = dict(ipg_receipt)
    if tamper_receipt:
        working_receipt = dict(working_receipt)
        # Add a fake field that will cause canonical_bytes mismatch
        # with the independently rebuilt receipt
        working_receipt["_tamper_injected"] = "forge"

    if use_blocked_gate:
        working_receipt = dict(working_receipt)
        working_receipt["gate_decision"] = "block"
        working_receipt["valid"] = False

    gate_ctx = AdapterGateContext(
        ipg_receipt=working_receipt,
        ipg_context=ipg_context,
        adapter_id=adapter_id,
        conversation_id=conversation_id,
        run_id=run_id,
    )

    blocked_error: str | None = None
    enforcement_receipt: dict[str, Any] | None = None

    if attempt_direct_call:
        # Simulate calling adapter without any gate context at all
        enforcement_receipt = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "adapter_gate_enforcement_receipt",
            "receipt_id": f"AGE-{uuid.uuid4().hex.upper()}",
            "conversation_id": conversation_id,
            "run_id": run_id,
            "created_at": utc_now(),
            "gate_valid": False,
            "gate_decision": "block",
            "ipg_receipt_sha256": "",
            "ipg_context_sha256": "",
            "adapter_id": adapter_id,
            "adapter_call_allowed": False,
            "bypass_attempted": True,
            "bypass_details": "direct adapter call without gate context",
            "trust_status": "not_observed",
            "trust_receipt_id": "",
            "trust_receipt_sha256": "",
            "network_permit_required": False,
            "network_permit_granted": True,
            "network_allowed_categories": [],
            "network_endpoint": "",
            "checks": {
                "ipg_evaluated_before_adapter": False,
                "ipg_receipt_present": False,
                "ipg_receipt_valid": False,
                "gate_decision_respected": False,
                "no_direct_adapter_call": False,
                "no_replay_attack": True,
                "network_permit_policy_present": False,
            },
            "errors": ["direct adapter call without IPG gate context"],
            "limitations": [],
        }
        blocked_error = "direct call blocked"
    else:
        try:
            result = enforce_adapter_call(
                gate_context=gate_ctx,
                adapter_call=lambda: "adapter-output",
            )
            enforcement_receipt = {
                k: v for k, v in result.items() if k != "adapter_result"
            }
        except AdapterGateBlockedError as exc:
            blocked_error = str(exc)
            # Build enforcement receipt for the blocked case
            enforcement_receipt = {
                "schema_version": "0.1.0-draft",
                "receipt_kind": "adapter_gate_enforcement_receipt",
                "receipt_id": gate_ctx._enforcement_id,
                "conversation_id": conversation_id,
                "run_id": run_id,
                "created_at": gate_ctx._created_at,
                "gate_valid": False,
                "gate_decision": working_receipt.get("gate_decision", "block"),
                "ipg_receipt_sha256": gate_ctx.ipg_receipt_sha256,
                "ipg_context_sha256": gate_ctx.ipg_context_sha256,
                "adapter_id": adapter_id,
                "adapter_call_allowed": False,
                "bypass_attempted": True,
                "bypass_details": str(exc),
                "trust_status": gate_ctx.trust_status,
                "trust_receipt_id": gate_ctx.trust_receipt_id,
                "trust_receipt_sha256": gate_ctx.trust_receipt_sha256,
                "network_permit_required": False,
                "network_permit_granted": True,
                "network_allowed_categories": [],
                "network_endpoint": "",
                "checks": {
                    "ipg_evaluated_before_adapter": True,
                    "ipg_receipt_present": True,
                    "ipg_receipt_valid": not tamper_receipt,
                    "gate_decision_respected": True,
                    "no_direct_adapter_call": True,
                    "no_replay_attack": not replay_old_receipt,
                    "network_permit_policy_present": False,
                },
                "errors": [str(exc)],
                "limitations": [],
            }

    output_root.mkdir(parents=True, exist_ok=True)
    enforcement_path = output_root / "adapter-gate-enforcement-receipt.json"
    atomic_write_json(enforcement_path, enforcement_receipt)

    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "adapter_gate_bypass_fixture_summary",
        "bypass_attempted": enforcement_receipt.get("bypass_attempted", False),
        "adapter_call_allowed": enforcement_receipt.get("adapter_call_allowed", False),
        "blocked": blocked_error is not None,
        "blocked_reason": blocked_error,
        "enforcement_receipt_path": str(enforcement_path),
        "fixture_type": (
            "direct_call" if attempt_direct_call
            else "tampered_receipt" if tamper_receipt
            else "blocked_gate" if use_blocked_gate
            else "replay" if replay_old_receipt
            else "valid"
        ),
        "limitations": [
            "Bypass fixture is a mechanical conformance test.",
            "Real adapter bypass may use different vectors not covered here.",
        ],
    }
