from __future__ import annotations

from datetime import datetime, timedelta, timezone
import re
from typing import Any
import uuid

from .contracts import validate_contract
from .conversation import ConversationNamespace
from .envelope import verify_security_envelope
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import canonical_bytes, load_json, sha256_bytes


SHA256_PATTERN = re.compile(r"^[a-f0-9]{64}$")


def _timestamp(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _parse_timestamp(value: str) -> datetime:
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise AssuranceError(f"invalid permit timestamp: {value}") from exc


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


def verify_sensitive_action_permit(
    receipt: dict[str, Any],
    *,
    key_store: InstallationKeyStore,
    envelope: dict[str, Any],
    issued_receipt: dict[str, Any] | None = None,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "sensitive-action-permit-v0.1.schema.json",
            label="sensitive action permit",
        )
        verify_security_envelope(envelope, key_store=key_store)
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}
    integrity = receipt["integrity"]
    body = {key: value for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    if integrity["key_id"] != key_store.key_id:
        errors.append("permit integrity key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(payload):
        errors.append("permit signed payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        errors.append("permit signature verification failed")
    if receipt["conversation_id"] != envelope["conversation_id"]:
        errors.append("permit conversation mismatch")
    if receipt["envelope_id"] != envelope["envelope_id"]:
        errors.append("permit envelope mismatch")
    if _parse_timestamp(receipt["expires_at"]) > _parse_timestamp(
        envelope["expires_at"]
    ):
        errors.append("permit outlives security envelope")
    if receipt["state"] == "issued":
        if issued_receipt is not None:
            errors.append("issued permit unexpectedly references another receipt")
    else:
        if issued_receipt is None:
            errors.append("consumed permit lacks issued receipt")
        else:
            issued_verification = verify_sensitive_action_permit(
                issued_receipt,
                key_store=key_store,
                envelope=envelope,
            )
            if not issued_verification["valid"]:
                errors.extend(
                    f"issued permit: {item}"
                    for item in issued_verification["errors"]
                )
            if receipt["permit_id"] != issued_receipt["permit_id"]:
                errors.append("consumed permit ID mismatch")
            if receipt["binding"] != issued_receipt["binding"]:
                errors.append("consumed permit binding mismatch")
            if receipt["confirmation_sha256"] != issued_receipt[
                "confirmation_sha256"
            ]:
                errors.append("consumed permit confirmation mismatch")
            if receipt["expires_at"] != issued_receipt["expires_at"]:
                errors.append("consumed permit expiry mismatch")
            if receipt["issued_permit_sha256"] != sha256_bytes(
                canonical_bytes(issued_receipt)
            ):
                errors.append("consumed permit issued-receipt digest mismatch")
    return {"valid": not errors, "errors": errors}


def issue_sensitive_action_permit(
    *,
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    confirmation_sha256: str,
    action_sha256: str,
    target_sha256: str,
    impact_scope_sha256: str,
    attempt: int,
    ttl_seconds: int = 300,
    now: datetime | None = None,
) -> dict[str, Any]:
    for label, value in (
        ("confirmation", confirmation_sha256),
        ("action", action_sha256),
        ("target", target_sha256),
        ("impact scope", impact_scope_sha256),
    ):
        _validate_sha256(value, label=label)
    if attempt < 1:
        raise AssuranceError("permit attempt must be positive")
    if ttl_seconds < 1:
        raise AssuranceError("permit TTL must be positive")
    envelope = namespace.load_active_envelope()
    verify_security_envelope(envelope, key_store=key_store)
    selected_now = now or datetime.now(timezone.utc)
    envelope_expiry = _parse_timestamp(envelope["expires_at"])
    if selected_now >= envelope_expiry:
        raise AssuranceError("cannot issue a permit from an expired envelope")
    expires_at = min(
        selected_now + timedelta(seconds=ttl_seconds),
        envelope_expiry,
    )
    permit_id = f"PERMIT-{uuid.uuid4().hex.upper()}"
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "sensitive_action_permit",
        "receipt_id": f"SAPR-{uuid.uuid4().hex.upper()}",
        "permit_id": permit_id,
        "state": "issued",
        "conversation_id": namespace.conversation_id,
        "envelope_id": envelope["envelope_id"],
        "created_at": _timestamp(selected_now),
        "expires_at": _timestamp(expires_at),
        "authority": "local-interactive-user",
        "confirmation_sha256": confirmation_sha256,
        "binding": {
            "action_sha256": action_sha256,
            "target_sha256": target_sha256,
            "impact_scope_sha256": impact_scope_sha256,
            "attempt": attempt,
        },
        "issued_permit_sha256": None,
        "raw_confirmation_recorded": False,
        "raw_action_recorded": False,
    }
    receipt = _sign(body, key_store=key_store)
    validate_contract(
        receipt,
        "sensitive-action-permit-v0.1.schema.json",
        label="issued sensitive action permit",
    )
    verification = verify_sensitive_action_permit(
        receipt, key_store=key_store, envelope=envelope
    )
    if not verification["valid"]:
        raise AssuranceError(
            "issued sensitive action permit failed verification: "
            + "; ".join(verification["errors"])
        )
    namespace.write_artifact(
        "one_shot_permit",
        f"{permit_id}.issued.json",
        canonical_bytes(receipt),
    )
    return receipt


def consume_sensitive_action_permit(
    *,
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    issued_receipt: dict[str, Any],
    action_sha256: str,
    target_sha256: str,
    impact_scope_sha256: str,
    attempt: int,
    now: datetime | None = None,
) -> dict[str, Any]:
    envelope = namespace.load_active_envelope()
    verification = verify_sensitive_action_permit(
        issued_receipt, key_store=key_store, envelope=envelope
    )
    if not verification["valid"]:
        raise AssuranceError(
            "issued permit verification failed: "
            + "; ".join(verification["errors"])
        )
    supplied = {
        "action_sha256": action_sha256,
        "target_sha256": target_sha256,
        "impact_scope_sha256": impact_scope_sha256,
        "attempt": attempt,
    }
    if supplied != issued_receipt["binding"]:
        raise AssuranceError("sensitive action does not match permit binding")
    selected_now = now or datetime.now(timezone.utc)
    if selected_now >= _parse_timestamp(issued_receipt["expires_at"]):
        raise AssuranceError("sensitive action permit is expired")
    permit_id = issued_receipt["permit_id"]
    consumed_path = (
        namespace.artifacts_root
        / "one_shot_permit"
        / f"{permit_id}.consumed.json"
    )
    claim_path = (
        namespace.artifacts_root
        / "one_shot_permit"
        / f"{permit_id}.consumption.claim"
    )
    if consumed_path.exists():
        existing = load_json(consumed_path)
        raise AssuranceError(
            "sensitive action permit already consumed by "
            f"{existing.get('receipt_id', 'unknown receipt')}"
        )
    if claim_path.exists():
        raise AssuranceError("sensitive action permit consumption is already claimed")
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "sensitive_action_permit",
        "receipt_id": f"SAPR-{uuid.uuid4().hex.upper()}",
        "permit_id": permit_id,
        "state": "consumed",
        "conversation_id": namespace.conversation_id,
        "envelope_id": envelope["envelope_id"],
        "created_at": _timestamp(selected_now),
        "expires_at": issued_receipt["expires_at"],
        "authority": "local-interactive-user",
        "confirmation_sha256": issued_receipt["confirmation_sha256"],
        "binding": deepcopy_binding(issued_receipt["binding"]),
        "issued_permit_sha256": sha256_bytes(canonical_bytes(issued_receipt)),
        "raw_confirmation_recorded": False,
        "raw_action_recorded": False,
    }
    receipt = _sign(body, key_store=key_store)
    validate_contract(
        receipt,
        "sensitive-action-permit-v0.1.schema.json",
        label="consumed sensitive action permit",
    )
    consumed_verification = verify_sensitive_action_permit(
        receipt,
        key_store=key_store,
        envelope=envelope,
        issued_receipt=issued_receipt,
    )
    if not consumed_verification["valid"]:
        raise AssuranceError(
            "consumed sensitive action permit failed verification: "
            + "; ".join(consumed_verification["errors"])
        )
    namespace.claim_artifact(
        "one_shot_permit",
        f"{permit_id}.consumption.claim",
        canonical_bytes(
            {
                "permit_id": permit_id,
                "issued_permit_sha256": sha256_bytes(
                    canonical_bytes(issued_receipt)
                ),
                "consumed_receipt_sha256": sha256_bytes(
                    canonical_bytes(receipt)
                ),
            }
        ),
    )
    namespace.write_artifact(
        "one_shot_permit",
        f"{permit_id}.consumed.json",
        canonical_bytes(receipt),
    )
    return receipt


def deepcopy_binding(value: dict[str, Any]) -> dict[str, Any]:
    return {
        "action_sha256": value["action_sha256"],
        "target_sha256": value["target_sha256"],
        "impact_scope_sha256": value["impact_scope_sha256"],
        "attempt": value["attempt"],
    }
