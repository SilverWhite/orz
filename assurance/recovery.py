from __future__ import annotations

from datetime import datetime, timezone
import re
from typing import Any
import uuid

from .audit import verify_audit_seal
from .contracts import validate_contract
from .conversation import ConversationNamespace
from .envelope import verify_security_envelope
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .permit import (
    consume_sensitive_action_permit,
    verify_sensitive_action_permit,
)
from .utils import (
    atomic_write_json,
    canonical_bytes,
    sha256_bytes,
)


SHA256_PATTERN = re.compile(r"^[a-f0-9]{64}$")


def _timestamp(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _validate_sha256(value: str, *, label: str) -> None:
    if not isinstance(value, str) or not SHA256_PATTERN.fullmatch(value):
        raise AssuranceError(f"{label} must be a SHA-256 digest")


def _sign(
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


def _verify_signature(
    receipt: dict[str, Any],
    *,
    key_store: InstallationKeyStore,
    label: str,
) -> list[str]:
    errors: list[str] = []
    integrity = receipt["integrity"]
    body = {key: value for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    if integrity["key_id"] != key_store.key_id:
        errors.append(f"{label} key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(payload):
        errors.append(f"{label} signed payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        errors.append(f"{label} signature verification failed")
    return errors


def _action_binding(
    candidate: dict[str, Any], *, attempt: int
) -> dict[str, Any]:
    projection = {
        "action_type": "recovery.apply_candidate",
        "candidate_sha256": sha256_bytes(canonical_bytes(candidate)),
        "target_workspace_sha256": candidate["destination"][
            "target_workspace_sha256"
        ],
        "snapshot_sha256": candidate["source"]["snapshot_sha256"],
        "attempt": attempt,
    }
    return {
        "action_sha256": sha256_bytes(canonical_bytes(projection)),
        "target_workspace_sha256": projection["target_workspace_sha256"],
        "snapshot_sha256": projection["snapshot_sha256"],
        "attempt": attempt,
    }


def create_recovery_candidate(
    *,
    source_namespace: ConversationNamespace,
    destination_namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    audit_seal: dict[str, Any],
    snapshot_relative_path: str,
    target_workspace_sha256: str,
    now: datetime | None = None,
) -> dict[str, Any]:
    _validate_sha256(target_workspace_sha256, label="target workspace")
    audit_verification = verify_audit_seal(
        audit_seal,
        namespace=source_namespace,
        key_store=key_store,
        require_archive_complete=True,
    )
    if not audit_verification["valid"]:
        raise AssuranceError(
            "recovery source audit is not verified after archive: "
            + "; ".join(audit_verification["errors"])
        )
    envelope = destination_namespace.load_active_envelope()
    verify_security_envelope(envelope, key_store=key_store)
    snapshot = source_namespace.read_artifact(
        source_namespace.conversation_id,
        "user_pinned_snapshot",
        snapshot_relative_path,
    )
    if not snapshot:
        raise AssuranceError("recovery snapshot must be non-empty")
    selected_now = now or datetime.now(timezone.utc)
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "recovery_candidate",
        "receipt_id": f"RCR-{uuid.uuid4().hex.upper()}",
        "candidate_id": f"RCV-{uuid.uuid4().hex.upper()}",
        "created_at": _timestamp(selected_now),
        "source": {
            "conversation_id": source_namespace.conversation_id,
            "audit_seal_sha256": sha256_bytes(canonical_bytes(audit_seal)),
            "snapshot_relative_path": (
                "artifacts/user_pinned_snapshot/" + snapshot_relative_path
            ),
            "snapshot_sha256": sha256_bytes(snapshot),
            "snapshot_bytes": len(snapshot),
            "classification": "untrusted_recovery_candidate",
        },
        "destination": {
            "conversation_id": destination_namespace.conversation_id,
            "envelope_id": envelope["envelope_id"],
            "target_workspace_sha256": target_workspace_sha256,
        },
        "decision": {
            "state": "candidate_untrusted",
            "authority_effect": "none",
            "action_authorized": False,
            "permit_required": True,
            "restoration_performed": False,
            "reason_codes": [
                "RECOVERY_CANDIDATE_REQUIRES_SEPARATE_AUTHORIZATION"
            ],
        },
    }
    receipt = _sign(body, key_store=key_store)
    validate_contract(
        receipt,
        "recovery-candidate-receipt-v0.1.schema.json",
        label="recovery candidate receipt",
    )
    verification = verify_recovery_candidate(
        receipt,
        source_namespace=source_namespace,
        destination_namespace=destination_namespace,
        key_store=key_store,
        audit_seal=audit_seal,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "recovery candidate failed verification: "
            + "; ".join(verification["errors"])
        )
    destination_namespace.write_artifact(
        "temporary_lifecycle_receipt",
        f"{receipt['receipt_id']}.json",
        canonical_bytes(receipt),
    )
    return receipt


def verify_recovery_candidate(
    receipt: dict[str, Any],
    *,
    source_namespace: ConversationNamespace,
    destination_namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    audit_seal: dict[str, Any],
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "recovery-candidate-receipt-v0.1.schema.json",
            label="recovery candidate receipt",
        )
        errors.extend(
            _verify_signature(
                receipt, key_store=key_store, label="recovery candidate"
            )
        )
        audit_verification = verify_audit_seal(
            audit_seal,
            namespace=source_namespace,
            key_store=key_store,
            require_archive_complete=True,
        )
        if not audit_verification["valid"]:
            errors.extend(
                f"audit: {item}" for item in audit_verification["errors"]
            )
        envelope = destination_namespace.load_active_envelope()
        verify_security_envelope(envelope, key_store=key_store)
        prefix = "artifacts/user_pinned_snapshot/"
        relative = receipt["source"]["snapshot_relative_path"]
        if not relative.startswith(prefix):
            raise AssuranceError("recovery snapshot path is outside pinned storage")
        snapshot = source_namespace.read_artifact(
            source_namespace.conversation_id,
            "user_pinned_snapshot",
            relative[len(prefix) :],
        )
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}
    if receipt["source"]["conversation_id"] != source_namespace.conversation_id:
        errors.append("recovery source conversation mismatch")
    if receipt["source"]["audit_seal_sha256"] != sha256_bytes(
        canonical_bytes(audit_seal)
    ):
        errors.append("recovery audit seal digest mismatch")
    if (
        receipt["source"]["snapshot_sha256"] != sha256_bytes(snapshot)
        or receipt["source"]["snapshot_bytes"] != len(snapshot)
    ):
        errors.append("recovery snapshot digest mismatch")
    if (
        receipt["destination"]["conversation_id"]
        != destination_namespace.conversation_id
    ):
        errors.append("recovery destination conversation mismatch")
    if receipt["destination"]["envelope_id"] != envelope["envelope_id"]:
        errors.append("recovery destination envelope mismatch")
    if (
        receipt["decision"]["action_authorized"]
        or receipt["decision"]["restoration_performed"]
        or receipt["decision"]["authority_effect"] != "none"
    ):
        errors.append("recovery candidate improperly carries authority")
    return {"valid": not errors, "errors": errors}


def authorize_recovery_candidate(
    *,
    source_namespace: ConversationNamespace,
    destination_namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    audit_seal: dict[str, Any],
    candidate: dict[str, Any],
    attempt: int,
    issued_permit: dict[str, Any] | None,
    now: datetime | None = None,
) -> tuple[dict[str, Any], dict[str, Any] | None]:
    if attempt < 1:
        raise AssuranceError("recovery authorization attempt must be positive")
    candidate_verification = verify_recovery_candidate(
        candidate,
        source_namespace=source_namespace,
        destination_namespace=destination_namespace,
        key_store=key_store,
        audit_seal=audit_seal,
    )
    if not candidate_verification["valid"]:
        raise AssuranceError(
            "cannot authorize an invalid recovery candidate: "
            + "; ".join(candidate_verification["errors"])
        )
    envelope = destination_namespace.load_active_envelope()
    binding = _action_binding(candidate, attempt=attempt)
    selected_now = now or datetime.now(timezone.utc)
    consumed: dict[str, Any] | None = None
    if issued_permit is None:
        outcome = "deny"
        authority = "none"
        reasons = ["RECOVERY_ONE_SHOT_PERMIT_REQUIRED"]
    else:
        try:
            consumed = consume_sensitive_action_permit(
                namespace=destination_namespace,
                key_store=key_store,
                issued_receipt=issued_permit,
                action_sha256=binding["action_sha256"],
                target_sha256=binding["target_workspace_sha256"],
                impact_scope_sha256=binding["snapshot_sha256"],
                attempt=attempt,
                now=selected_now,
            )
        except AssuranceError:
            outcome = "deny"
            authority = "none"
            reasons = ["RECOVERY_ONE_SHOT_PERMIT_INVALID_OR_REPLAYED"]
        else:
            outcome = "allow"
            authority = "envelope_and_one_shot_permit"
            reasons = ["RECOVERY_EXACT_ONE_SHOT_PERMIT_CONSUMED"]
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "recovery_authorization",
        "receipt_id": f"RAR-{uuid.uuid4().hex.upper()}",
        "candidate_id": candidate["candidate_id"],
        "conversation_id": destination_namespace.conversation_id,
        "envelope_id": envelope["envelope_id"],
        "created_at": _timestamp(selected_now),
        "candidate_sha256": sha256_bytes(canonical_bytes(candidate)),
        "binding": binding,
        "permit_consumption_sha256": (
            sha256_bytes(canonical_bytes(consumed))
            if consumed is not None
            else None
        ),
        "decision": {
            "outcome": outcome,
            "authority_source": authority,
            "restoration_performed": False,
            "reason_codes": reasons,
        },
    }
    receipt = _sign(body, key_store=key_store)
    validate_contract(
        receipt,
        "recovery-authorization-receipt-v0.1.schema.json",
        label="recovery authorization receipt",
    )
    verification = verify_recovery_authorization(
        receipt,
        source_namespace=source_namespace,
        destination_namespace=destination_namespace,
        key_store=key_store,
        audit_seal=audit_seal,
        candidate=candidate,
        issued_permit=issued_permit,
        consumed_permit=consumed,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "recovery authorization failed verification: "
            + "; ".join(verification["errors"])
        )
    destination_namespace.write_artifact(
        "temporary_lifecycle_receipt",
        f"{receipt['receipt_id']}.json",
        canonical_bytes(receipt),
    )
    return receipt, consumed


def verify_recovery_authorization(
    receipt: dict[str, Any],
    *,
    source_namespace: ConversationNamespace,
    destination_namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    audit_seal: dict[str, Any],
    candidate: dict[str, Any],
    issued_permit: dict[str, Any] | None,
    consumed_permit: dict[str, Any] | None,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "recovery-authorization-receipt-v0.1.schema.json",
            label="recovery authorization receipt",
        )
        errors.extend(
            _verify_signature(
                receipt, key_store=key_store, label="recovery authorization"
            )
        )
        candidate_verification = verify_recovery_candidate(
            candidate,
            source_namespace=source_namespace,
            destination_namespace=destination_namespace,
            key_store=key_store,
            audit_seal=audit_seal,
        )
        if not candidate_verification["valid"]:
            errors.extend(
                f"candidate: {item}"
                for item in candidate_verification["errors"]
            )
        envelope = destination_namespace.load_active_envelope()
        verify_security_envelope(envelope, key_store=key_store)
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}
    expected_binding = _action_binding(
        candidate, attempt=receipt["binding"]["attempt"]
    )
    if receipt["binding"] != expected_binding:
        errors.append("recovery authorization binding mismatch")
    if receipt["candidate_id"] != candidate["candidate_id"]:
        errors.append("recovery authorization candidate ID mismatch")
    if receipt["candidate_sha256"] != sha256_bytes(canonical_bytes(candidate)):
        errors.append("recovery authorization candidate digest mismatch")
    if (
        receipt["conversation_id"] != destination_namespace.conversation_id
        or receipt["envelope_id"] != envelope["envelope_id"]
    ):
        errors.append("recovery authorization destination mismatch")
    if consumed_permit is None:
        expected_outcome = "deny"
        expected_authority = "none"
        expected_reasons = (
            ["RECOVERY_ONE_SHOT_PERMIT_INVALID_OR_REPLAYED"]
            if issued_permit is not None
            else ["RECOVERY_ONE_SHOT_PERMIT_REQUIRED"]
        )
        expected_permit_sha = None
    else:
        if issued_permit is None:
            errors.append("consumed recovery permit lacks issued receipt")
        else:
            permit_verification = verify_sensitive_action_permit(
                consumed_permit,
                key_store=key_store,
                envelope=envelope,
                issued_receipt=issued_permit,
            )
            if not permit_verification["valid"]:
                errors.extend(
                    f"permit: {item}"
                    for item in permit_verification["errors"]
                )
            if consumed_permit["binding"] != {
                "action_sha256": expected_binding["action_sha256"],
                "target_sha256": expected_binding[
                    "target_workspace_sha256"
                ],
                "impact_scope_sha256": expected_binding["snapshot_sha256"],
                "attempt": expected_binding["attempt"],
            }:
                errors.append("recovery permit binding mismatch")
        expected_outcome = "allow"
        expected_authority = "envelope_and_one_shot_permit"
        expected_reasons = ["RECOVERY_EXACT_ONE_SHOT_PERMIT_CONSUMED"]
        expected_permit_sha = sha256_bytes(canonical_bytes(consumed_permit))
    if receipt["decision"]["outcome"] != expected_outcome:
        errors.append("recovery authorization outcome mismatch")
    if receipt["decision"]["authority_source"] != expected_authority:
        errors.append("recovery authorization authority mismatch")
    if receipt["decision"]["reason_codes"] != expected_reasons:
        errors.append("recovery authorization reason mismatch")
    if receipt["permit_consumption_sha256"] != expected_permit_sha:
        errors.append("recovery authorization permit digest mismatch")
    if receipt["decision"]["restoration_performed"]:
        errors.append("authorization receipt must not perform restoration")
    return {"valid": not errors, "errors": errors}


def recovery_permit_binding(
    candidate: dict[str, Any], *, attempt: int
) -> dict[str, Any]:
    """Return the exact public binding required to issue a recovery permit."""
    return _action_binding(candidate, attempt=attempt)
