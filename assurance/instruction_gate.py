from __future__ import annotations

from copy import deepcopy
from datetime import datetime, timezone
import re
from typing import Any
import uuid

from .contracts import validate_contract
from .envelope import create_security_envelope, verify_security_envelope
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .conversation import ConversationNamespace
from .permit import (
    consume_sensitive_action_permit,
    verify_sensitive_action_permit,
)
from .utils import canonical_bytes, sha256_bytes, utc_now


SOURCE_TYPES = {
    "platform",
    "user",
    "trusted_project",
    "untrusted_project",
    "external_content",
    "tool_output",
    "recalled_memory",
    "derived_summary",
}
ROUTABLE_SOURCES = {"platform", "user", "trusted_project"}
DATA_ONLY_SOURCES = SOURCE_TYPES - ROUTABLE_SOURCES
REMOTE_MCP_LOCAL_PREFIXES = (
    "credential.",
    "filesystem.",
    "host.",
    "process.",
    "sandbox.",
    "secret.",
)
CAPABILITY_PATTERN = re.compile(r"^[a-z][a-z0-9_.:-]+$")
SHA256_PATTERN = re.compile(r"^[a-f0-9]{64}$")
ACTION_ID_PATTERN = re.compile(r"^ACT-[A-Z0-9._-]+$")
ACTION_TYPE_PATTERN = re.compile(r"^[a-z][a-z0-9_.:-]{0,127}$")


def _timestamp(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _parse_timestamp(value: str) -> datetime:
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise AssuranceError(f"invalid timestamp: {value}") from exc


def _sign_body(
    body: dict[str, Any], *, key_store: InstallationKeyStore
) -> dict[str, Any]:
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": key_store.key_id,
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": key_store.sign(payload),
        },
    }


def _verify_integrity(
    receipt: dict[str, Any], *, key_store: InstallationKeyStore
) -> list[str]:
    errors: list[str] = []
    integrity = receipt["integrity"]
    body = {key: value for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    if integrity["key_id"] != key_store.key_id:
        errors.append("receipt integrity key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(payload):
        errors.append("receipt signed payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        errors.append("receipt signature verification failed")
    return errors


def _effective_source(
    declared_type: str, *, workspace_trust: str
) -> str:
    if declared_type == "trusted_project" and workspace_trust != "observed_trusted":
        return "untrusted_project"
    return declared_type


def _normalize_action(value: dict[str, Any] | None) -> dict[str, Any] | None:
    if value is None:
        return None
    expected = {
        "action_id",
        "action_type",
        "risk_class",
        "target_sha256",
        "parameters_sha256",
        "requested_capabilities",
    }
    if set(value) != expected:
        raise AssuranceError("structured action has an invalid field set")
    if not isinstance(value["action_id"], str) or not ACTION_ID_PATTERN.fullmatch(
        value["action_id"]
    ):
        raise AssuranceError("structured action ID is invalid")
    if not isinstance(value["action_type"], str) or not ACTION_TYPE_PATTERN.fullmatch(
        value["action_type"]
    ):
        raise AssuranceError("structured action type is invalid")
    if value["risk_class"] not in {
        "normal",
        "sensitive",
        "external_side_effect",
    }:
        raise AssuranceError("structured action risk class is invalid")
    for field in ("target_sha256", "parameters_sha256"):
        if not isinstance(value[field], str) or not SHA256_PATTERN.fullmatch(
            value[field]
        ):
            raise AssuranceError(f"structured action {field} is invalid")
    capabilities = value["requested_capabilities"]
    if (
        not isinstance(capabilities, list)
        or len(capabilities) != len(set(capabilities))
        or any(
            not isinstance(item, str) or not CAPABILITY_PATTERN.fullmatch(item)
            for item in capabilities
        )
    ):
        raise AssuranceError("structured action capabilities are invalid")
    return {
        **deepcopy(value),
        "requested_capabilities": sorted(capabilities),
    }


def _instruction_decision(
    *,
    effective_source: str,
    action: dict[str, Any] | None,
    allowed_capabilities: set[str],
) -> dict[str, Any]:
    if action is None:
        return {
            "routing": "no_action",
            "authority_effect": "none",
            "action_authorized": False,
            "embedded_directive_ignored": True,
            "capability_subset": None,
            "permit_required": False,
            "reason_codes": ["NO_STRUCTURED_ACTION"],
        }
    requested = set(action["requested_capabilities"])
    is_subset = requested <= allowed_capabilities
    permit_required = action["risk_class"] in {
        "sensitive",
        "external_side_effect",
    }
    if effective_source in DATA_ONLY_SOURCES:
        return {
            "routing": "data_only",
            "authority_effect": "none",
            "action_authorized": False,
            "embedded_directive_ignored": True,
            "capability_subset": is_subset,
            "permit_required": permit_required,
            "reason_codes": ["UNTRUSTED_SOURCE_DATA_ONLY"],
        }
    if not is_subset:
        return {
            "routing": "rejected",
            "authority_effect": "none",
            "action_authorized": False,
            "embedded_directive_ignored": True,
            "capability_subset": False,
            "permit_required": permit_required,
            "reason_codes": ["CAPABILITY_EXCEEDS_ENVELOPE"],
        }
    return {
        "routing": "candidate",
        "authority_effect": "none",
        "action_authorized": False,
        "embedded_directive_ignored": True,
        "capability_subset": True,
        "permit_required": permit_required,
        "reason_codes": ["STRUCTURED_CANDIDATE_REQUIRES_ACTION_GATE"],
    }


def record_instruction_provenance(
    *,
    key_store: InstallationKeyStore,
    envelope: dict[str, Any],
    declared_source_type: str,
    source_id: str,
    content: bytes,
    structured_action: dict[str, Any] | None,
    workspace_trust: str = "not_applicable",
    origin_conversation_id: str | None = None,
    now: datetime | None = None,
) -> dict[str, Any]:
    verify_security_envelope(envelope, key_store=key_store)
    if declared_source_type not in SOURCE_TYPES:
        raise AssuranceError("instruction source type is invalid")
    if not source_id or not re.fullmatch(r"[A-Za-z0-9._:-]+", source_id):
        raise AssuranceError("instruction source ID is invalid")
    if declared_source_type in {"trusted_project", "untrusted_project"}:
        if workspace_trust not in {"observed_trusted", "not_observed"}:
            raise AssuranceError("project instruction requires workspace trust status")
    elif workspace_trust != "not_applicable":
        raise AssuranceError("non-project instruction cannot assert workspace trust")
    if declared_source_type == "recalled_memory" and origin_conversation_id is None:
        raise AssuranceError("recalled memory requires an origin conversation")
    selected_now = now or datetime.now(timezone.utc)
    if envelope["lifecycle_state"] != "active":
        raise AssuranceError("instruction routing requires an active envelope")
    if selected_now >= _parse_timestamp(envelope["expires_at"]):
        raise AssuranceError("instruction routing envelope is expired")
    action = _normalize_action(structured_action)
    effective_source = _effective_source(
        declared_source_type, workspace_trust=workspace_trust
    )
    decision = _instruction_decision(
        effective_source=effective_source,
        action=action,
        allowed_capabilities=set(envelope["capability_envelope"]["allowed"]),
    )
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "instruction_provenance_decision",
        "receipt_id": f"IPR-{uuid.uuid4().hex.upper()}",
        "conversation_id": envelope["conversation_id"],
        "envelope_id": envelope["envelope_id"],
        "created_at": _timestamp(selected_now),
        "source": {
            "declared_type": declared_source_type,
            "effective_type": effective_source,
            "source_id": source_id,
            "origin_conversation_id": origin_conversation_id,
            "workspace_trust": workspace_trust,
        },
        "content": {
            "sha256": sha256_bytes(content),
            "bytes": len(content),
            "raw_content_recorded": False,
        },
        "structured_action": action,
        "decision": decision,
    }
    receipt = _sign_body(body, key_store=key_store)
    validate_contract(
        receipt,
        "instruction-provenance-receipt-v0.1.schema.json",
        label="instruction provenance receipt",
    )
    verification = verify_instruction_provenance(
        receipt, key_store=key_store, envelope=envelope, content=content
    )
    if not verification["valid"]:
        raise AssuranceError(
            "instruction provenance receipt failed verification: "
            + "; ".join(verification["errors"])
        )
    return receipt


def verify_instruction_provenance(
    receipt: dict[str, Any],
    *,
    key_store: InstallationKeyStore,
    envelope: dict[str, Any],
    content: bytes | None = None,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "instruction-provenance-receipt-v0.1.schema.json",
            label="instruction provenance receipt",
        )
        verify_security_envelope(envelope, key_store=key_store)
        errors.extend(_verify_integrity(receipt, key_store=key_store))
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}
    if receipt["conversation_id"] != envelope["conversation_id"]:
        errors.append("instruction receipt conversation mismatch")
    if receipt["envelope_id"] != envelope["envelope_id"]:
        errors.append("instruction receipt envelope mismatch")
    source = receipt["source"]
    expected_effective = _effective_source(
        source["declared_type"], workspace_trust=source["workspace_trust"]
    )
    if source["effective_type"] != expected_effective:
        errors.append("instruction effective source mismatch")
    if source["declared_type"] == "recalled_memory" and source[
        "origin_conversation_id"
    ] is None:
        errors.append("recalled memory lacks origin conversation")
    action = receipt["structured_action"]
    expected_decision = _instruction_decision(
        effective_source=expected_effective,
        action=action,
        allowed_capabilities=set(envelope["capability_envelope"]["allowed"]),
    )
    if receipt["decision"] != expected_decision:
        errors.append("instruction decision projection mismatch")
    if content is not None and (
        receipt["content"]["sha256"] != sha256_bytes(content)
        or receipt["content"]["bytes"] != len(content)
    ):
        errors.append("instruction content digest mismatch")
    if receipt["decision"]["action_authorized"]:
        errors.append("instruction content must never directly authorize an action")
    return {"valid": not errors, "errors": errors}


def _remote_mcp_forbidden(capabilities: set[str]) -> set[str]:
    return {
        item
        for item in capabilities
        if item.startswith(REMOTE_MCP_LOCAL_PREFIXES)
    }


def delegate_capabilities(
    *,
    key_store: InstallationKeyStore,
    parent_envelope: dict[str, Any],
    subject_kind: str,
    subject_id: str,
    requested_allowed: list[str],
    ttl_seconds: int,
    now: datetime | None = None,
) -> tuple[dict[str, Any] | None, dict[str, Any]]:
    verify_security_envelope(parent_envelope, key_store=key_store)
    if subject_kind not in {"child_process", "child_agent", "remote_mcp"}:
        raise AssuranceError("delegation subject kind is invalid")
    if not source_id_valid(subject_id):
        raise AssuranceError("delegation subject ID is invalid")
    if ttl_seconds < 1:
        raise AssuranceError("delegation TTL must be positive")
    if not isinstance(requested_allowed, list) or len(requested_allowed) != len(
        set(requested_allowed)
    ) or any(
        not isinstance(item, str) or not CAPABILITY_PATTERN.fullmatch(item)
        for item in requested_allowed
    ):
        raise AssuranceError("delegated capabilities are invalid")
    selected_now = now or datetime.now(timezone.utc)
    parent_expires = _parse_timestamp(parent_envelope["expires_at"])
    if (
        parent_envelope["lifecycle_state"] != "active"
        or selected_now >= parent_expires
    ):
        raise AssuranceError("delegation requires an unexpired active parent envelope")
    parent_allowed = set(parent_envelope["capability_envelope"]["allowed"])
    parent_denied = set(parent_envelope["capability_envelope"]["denied"])
    requested = set(requested_allowed)
    escalation = requested - parent_allowed
    remote_forbidden = (
        _remote_mcp_forbidden(requested) if subject_kind == "remote_mcp" else set()
    )
    child: dict[str, Any] | None = None
    if escalation:
        outcome = "denied"
        reasons = ["CAPABILITY_ESCALATION_DENIED"]
    elif remote_forbidden:
        outcome = "denied"
        reasons = ["REMOTE_MCP_LOCAL_CAPABILITY_DENIED"]
    else:
        outcome = "delegated"
        reasons = ["CAPABILITY_SUBSET_VERIFIED"]
        remaining_seconds = int((parent_expires - selected_now).total_seconds())
        if remaining_seconds < 1:
            raise AssuranceError("parent envelope expires before child creation")
        child = create_security_envelope(
            key_store=key_store,
            conversation_id=parent_envelope["conversation_id"],
            frozen_context=deepcopy(parent_envelope["frozen_context"]),
            allowed_capabilities=sorted(requested),
            denied_capabilities=sorted(
                parent_denied | (parent_allowed - requested)
            ),
            parent_envelope_id=parent_envelope["envelope_id"],
            now=selected_now,
            ttl_seconds=min(ttl_seconds, remaining_seconds),
        )
    if child is None:
        effective_allowed: list[str] = []
        effective_denied = sorted(parent_allowed | parent_denied | requested)
        child_id = None
        child_sha = None
    else:
        effective_allowed = child["capability_envelope"]["allowed"]
        effective_denied = child["capability_envelope"]["denied"]
        child_id = child["envelope_id"]
        child_sha = sha256_bytes(canonical_bytes(child))
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "capability_delegation",
        "receipt_id": f"CDR-{uuid.uuid4().hex.upper()}",
        "conversation_id": parent_envelope["conversation_id"],
        "parent_envelope_id": parent_envelope["envelope_id"],
        "created_at": _timestamp(selected_now),
        "subject": {"kind": subject_kind, "subject_id": subject_id},
        "requested": {
            "allowed": sorted(requested),
            "ttl_seconds": ttl_seconds,
        },
        "decision": {
            "outcome": outcome,
            "child_envelope_id": child_id,
            "child_envelope_sha256": child_sha,
            "effective_allowed": effective_allowed,
            "effective_denied": effective_denied,
            "reason_codes": reasons,
        },
    }
    receipt = _sign_body(body, key_store=key_store)
    validate_contract(
        receipt,
        "capability-delegation-receipt-v0.1.schema.json",
        label="capability delegation receipt",
    )
    verification = verify_capability_delegation(
        receipt,
        key_store=key_store,
        parent_envelope=parent_envelope,
        child_envelope=child,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "capability delegation failed verification: "
            + "; ".join(verification["errors"])
        )
    return child, receipt


def source_id_valid(value: str) -> bool:
    return bool(
        isinstance(value, str)
        and value
        and re.fullmatch(r"[A-Za-z0-9._:-]+", value)
    )


def verify_capability_delegation(
    receipt: dict[str, Any],
    *,
    key_store: InstallationKeyStore,
    parent_envelope: dict[str, Any],
    child_envelope: dict[str, Any] | None,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "capability-delegation-receipt-v0.1.schema.json",
            label="capability delegation receipt",
        )
        verify_security_envelope(parent_envelope, key_store=key_store)
        errors.extend(_verify_integrity(receipt, key_store=key_store))
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}
    if receipt["conversation_id"] != parent_envelope["conversation_id"]:
        errors.append("delegation conversation mismatch")
    if receipt["parent_envelope_id"] != parent_envelope["envelope_id"]:
        errors.append("delegation parent envelope mismatch")
    requested = set(receipt["requested"]["allowed"])
    parent_allowed = set(parent_envelope["capability_envelope"]["allowed"])
    parent_denied = set(parent_envelope["capability_envelope"]["denied"])
    escalation = requested - parent_allowed
    remote_forbidden = (
        _remote_mcp_forbidden(requested)
        if receipt["subject"]["kind"] == "remote_mcp"
        else set()
    )
    should_delegate = not escalation and not remote_forbidden
    decision = receipt["decision"]
    expected_reasons = (
        ["CAPABILITY_ESCALATION_DENIED"]
        if escalation
        else (
            ["REMOTE_MCP_LOCAL_CAPABILITY_DENIED"]
            if remote_forbidden
            else ["CAPABILITY_SUBSET_VERIFIED"]
        )
    )
    if should_delegate != (decision["outcome"] == "delegated"):
        errors.append("delegation outcome does not match subset policy")
    if decision["reason_codes"] != expected_reasons:
        errors.append("delegation reason codes do not match subset policy")
    if decision["outcome"] == "denied":
        if child_envelope is not None:
            errors.append("denied delegation returned a child envelope")
        expected_denied = sorted(parent_allowed | parent_denied | requested)
        if decision["effective_allowed"] or decision["effective_denied"] != expected_denied:
            errors.append("denied delegation capability projection mismatch")
    else:
        if child_envelope is None:
            errors.append("delegated receipt lacks child envelope")
        else:
            try:
                verify_security_envelope(child_envelope, key_store=key_store)
            except AssuranceError as exc:
                errors.append(str(exc))
            expected_denied = sorted(parent_denied | (parent_allowed - requested))
            if child_envelope["conversation_id"] != parent_envelope["conversation_id"]:
                errors.append("child envelope conversation mismatch")
            if child_envelope["parent_envelope_id"] != parent_envelope["envelope_id"]:
                errors.append("child envelope parent link mismatch")
            if child_envelope["frozen_context"] != parent_envelope["frozen_context"]:
                errors.append("child envelope changed frozen context")
            if child_envelope["capability_envelope"]["allowed"] != sorted(requested):
                errors.append("child allowed capabilities mismatch")
            if child_envelope["capability_envelope"]["denied"] != expected_denied:
                errors.append("child denied capabilities mismatch")
            if _parse_timestamp(child_envelope["expires_at"]) > _parse_timestamp(
                parent_envelope["expires_at"]
            ):
                errors.append("child envelope outlives parent")
            if _parse_timestamp(child_envelope["created_at"]) != _parse_timestamp(
                receipt["created_at"]
            ):
                errors.append("child envelope creation time mismatch")
            if decision["child_envelope_id"] != child_envelope["envelope_id"]:
                errors.append("delegation child envelope ID mismatch")
            if decision["child_envelope_sha256"] != sha256_bytes(
                canonical_bytes(child_envelope)
            ):
                errors.append("delegation child envelope digest mismatch")
            if decision["effective_allowed"] != sorted(requested):
                errors.append("delegation effective allowed mismatch")
            if decision["effective_denied"] != expected_denied:
                errors.append("delegation effective denied mismatch")
    return {"valid": not errors, "errors": errors}


def _authorization_projection(
    *,
    provenance_receipt: dict[str, Any],
    attempt: int,
) -> dict[str, Any]:
    action = provenance_receipt["structured_action"]
    if action is None:
        raise AssuranceError("action authorization requires a structured action")
    return {
        "action_id": action["action_id"],
        "action_type": action["action_type"],
        "risk_class": action["risk_class"],
        "action_sha256": sha256_bytes(canonical_bytes(action)),
        "target_sha256": action["target_sha256"],
        "impact_scope_sha256": action["parameters_sha256"],
        "requested_capabilities": action["requested_capabilities"],
        "attempt": attempt,
    }


def _no_action_projection(*, attempt: int) -> dict[str, Any]:
    placeholder = {
        "action_id": "ACT-NONE",
        "action_type": "none",
        "risk_class": "normal",
        "target_sha256": "0" * 64,
        "parameters_sha256": "0" * 64,
        "requested_capabilities": [],
    }
    return _authorization_projection(
        provenance_receipt={"structured_action": placeholder},
        attempt=attempt,
    )


def authorize_action_candidate(
    *,
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    provenance_receipt: dict[str, Any],
    content: bytes | None,
    attempt: int,
    issued_permit: dict[str, Any] | None = None,
    now: datetime | None = None,
) -> tuple[dict[str, Any], dict[str, Any] | None]:
    if attempt < 1:
        raise AssuranceError("action authorization attempt must be positive")
    envelope = namespace.load_active_envelope()
    provenance_verification = verify_instruction_provenance(
        provenance_receipt,
        key_store=key_store,
        envelope=envelope,
        content=content,
    )
    selected_now = now or datetime.now(timezone.utc)
    if (
        envelope["lifecycle_state"] != "active"
        or selected_now >= _parse_timestamp(envelope["expires_at"])
    ):
        raise AssuranceError(
            "action authorization requires an unexpired active envelope"
        )
    consumed: dict[str, Any] | None = None
    action = provenance_receipt.get("structured_action")
    if not provenance_verification["valid"]:
        outcome = "deny"
        authority = "none"
        reasons = ["PROVENANCE_RECEIPT_INVALID"]
    elif action is None:
        outcome = "deny"
        authority = "none"
        reasons = ["NO_STRUCTURED_ACTION"]
    elif provenance_receipt["decision"]["routing"] != "candidate":
        outcome = "deny"
        authority = "none"
        reasons = ["INSTRUCTION_NOT_ROUTED_AS_CANDIDATE"]
    elif action["risk_class"] == "normal":
        outcome = "allow"
        authority = "envelope"
        reasons = ["ENVELOPE_CAPABILITY_AUTHORIZED"]
    elif issued_permit is None:
        outcome = "deny"
        authority = "none"
        reasons = ["ONE_SHOT_PERMIT_REQUIRED"]
    else:
        projection = _authorization_projection(
            provenance_receipt=provenance_receipt,
            attempt=attempt,
        )
        try:
            consumed = consume_sensitive_action_permit(
                namespace=namespace,
                key_store=key_store,
                issued_receipt=issued_permit,
                action_sha256=projection["action_sha256"],
                target_sha256=projection["target_sha256"],
                impact_scope_sha256=projection["impact_scope_sha256"],
                attempt=attempt,
                now=selected_now,
            )
        except AssuranceError:
            outcome = "deny"
            authority = "none"
            reasons = ["ONE_SHOT_PERMIT_INVALID_OR_REPLAYED"]
        else:
            outcome = "allow"
            authority = "envelope_and_one_shot_permit"
            reasons = ["EXACT_ONE_SHOT_PERMIT_CONSUMED"]
    if action is None:
        action_receipt = _no_action_projection(attempt=attempt)
    else:
        action_receipt = _authorization_projection(
            provenance_receipt=provenance_receipt,
            attempt=attempt,
        )
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "kernel_action_authorization",
        "receipt_id": f"AAR-{uuid.uuid4().hex.upper()}",
        "conversation_id": namespace.conversation_id,
        "envelope_id": envelope["envelope_id"],
        "created_at": _timestamp(selected_now),
        "provenance_receipt_sha256": sha256_bytes(
            canonical_bytes(provenance_receipt)
        ),
        "action": action_receipt,
        "permit_consumption_sha256": (
            sha256_bytes(canonical_bytes(consumed))
            if consumed is not None
            else None
        ),
        "decision": {
            "outcome": outcome,
            "authority_source": authority,
            "model_assertion_ignored": True,
            "reason_codes": reasons,
        },
    }
    receipt = _sign_body(body, key_store=key_store)
    validate_contract(
        receipt,
        "action-authorization-receipt-v0.1.schema.json",
        label="action authorization receipt",
    )
    verification = verify_action_authorization(
        receipt,
        key_store=key_store,
        envelope=envelope,
        provenance_receipt=provenance_receipt,
        content=content,
        consumed_permit=consumed,
        issued_permit=issued_permit,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "action authorization receipt failed verification: "
            + "; ".join(verification["errors"])
        )
    namespace.write_artifact(
        "temporary_lifecycle_receipt",
        f"{receipt['receipt_id']}.json",
        canonical_bytes(receipt),
    )
    return receipt, consumed


def verify_action_authorization(
    receipt: dict[str, Any],
    *,
    key_store: InstallationKeyStore,
    envelope: dict[str, Any],
    provenance_receipt: dict[str, Any],
    content: bytes | None,
    consumed_permit: dict[str, Any] | None,
    issued_permit: dict[str, Any] | None,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "action-authorization-receipt-v0.1.schema.json",
            label="action authorization receipt",
        )
        verify_security_envelope(envelope, key_store=key_store)
        errors.extend(_verify_integrity(receipt, key_store=key_store))
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}
    provenance_verification = verify_instruction_provenance(
        provenance_receipt,
        key_store=key_store,
        envelope=envelope,
        content=content,
    )
    if not provenance_verification["valid"]:
        errors.extend(
            f"provenance: {item}" for item in provenance_verification["errors"]
        )
    if receipt["conversation_id"] != envelope["conversation_id"]:
        errors.append("action authorization conversation mismatch")
    if receipt["envelope_id"] != envelope["envelope_id"]:
        errors.append("action authorization envelope mismatch")
    if receipt["provenance_receipt_sha256"] != sha256_bytes(
        canonical_bytes(provenance_receipt)
    ):
        errors.append("action authorization provenance digest mismatch")
    provenance_action = provenance_receipt["structured_action"]
    expected_action = (
        _no_action_projection(attempt=receipt["action"]["attempt"])
        if provenance_action is None
        else _authorization_projection(
            provenance_receipt=provenance_receipt,
            attempt=receipt["action"]["attempt"],
        )
    )
    if receipt["action"] != expected_action:
        errors.append("action authorization projection mismatch")

    routing = provenance_receipt["decision"]["routing"]
    risk = provenance_action["risk_class"] if provenance_action is not None else None
    if not provenance_verification["valid"]:
        expected_outcome = "deny"
        expected_authority = "none"
        expected_reasons = ["PROVENANCE_RECEIPT_INVALID"]
    elif provenance_action is None:
        expected_outcome = "deny"
        expected_authority = "none"
        expected_reasons = ["NO_STRUCTURED_ACTION"]
    elif routing != "candidate":
        expected_outcome = "deny"
        expected_authority = "none"
        expected_reasons = ["INSTRUCTION_NOT_ROUTED_AS_CANDIDATE"]
    elif risk == "normal":
        expected_outcome = "allow"
        expected_authority = "envelope"
        expected_reasons = ["ENVELOPE_CAPABILITY_AUTHORIZED"]
        if consumed_permit is not None:
            errors.append("normal action unexpectedly consumed a permit")
    elif consumed_permit is None or issued_permit is None:
        expected_outcome = "deny"
        expected_authority = "none"
        expected_reasons = (
            ["ONE_SHOT_PERMIT_INVALID_OR_REPLAYED"]
            if issued_permit is not None
            else ["ONE_SHOT_PERMIT_REQUIRED"]
        )
    else:
        permit_verification = verify_sensitive_action_permit(
            consumed_permit,
            key_store=key_store,
            envelope=envelope,
            issued_receipt=issued_permit,
        )
        if not permit_verification["valid"]:
            errors.extend(
                f"permit: {item}" for item in permit_verification["errors"]
            )
            expected_outcome = "deny"
            expected_authority = "none"
            expected_reasons = ["ONE_SHOT_PERMIT_INVALID_OR_REPLAYED"]
        else:
            expected_binding = {
                "action_sha256": expected_action["action_sha256"],
                "target_sha256": expected_action["target_sha256"],
                "impact_scope_sha256": expected_action[
                    "impact_scope_sha256"
                ],
                "attempt": expected_action["attempt"],
            }
            if consumed_permit["binding"] != expected_binding:
                errors.append("action authorization permit binding mismatch")
            expected_outcome = "allow"
            expected_authority = "envelope_and_one_shot_permit"
            expected_reasons = ["EXACT_ONE_SHOT_PERMIT_CONSUMED"]
    if receipt["decision"]["outcome"] != expected_outcome:
        errors.append("action authorization outcome mismatch")
    if receipt["decision"]["authority_source"] != expected_authority:
        errors.append("action authorization authority source mismatch")
    if receipt["decision"]["reason_codes"] != expected_reasons:
        errors.append("action authorization reason codes mismatch")
    expected_permit_sha = (
        sha256_bytes(canonical_bytes(consumed_permit))
        if consumed_permit is not None
        else None
    )
    if receipt["permit_consumption_sha256"] != expected_permit_sha:
        errors.append("action authorization permit digest mismatch")
    return {"valid": not errors, "errors": errors}
