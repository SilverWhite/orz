from __future__ import annotations

from copy import deepcopy
from datetime import datetime, timedelta, timezone
import secrets
from typing import Any
import uuid

from .contracts import validate_contract
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import canonical_bytes, sha256_bytes


def new_conversation_id() -> str:
    return f"CONV-{uuid.uuid4().hex.upper()}"


def new_envelope_id() -> str:
    return f"ENV-{uuid.uuid4().hex.upper()}"


def _timestamp(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _evidenced_string(value: str, *, source_ref: str) -> dict[str, object]:
    return {
        "value": value,
        "evidence_status": "observed",
        "source_refs": [source_ref],
    }


def _evidenced_sha256(value: str, *, source_ref: str) -> dict[str, object]:
    return {
        "value": value,
        "evidence_status": "derived",
        "source_refs": [source_ref],
    }


def create_security_envelope(
    *,
    key_store: InstallationKeyStore,
    conversation_id: str,
    frozen_context: dict[str, Any],
    allowed_capabilities: list[str],
    denied_capabilities: list[str],
    parent_envelope_id: str | None = None,
    resumed_from_conversation_id: str | None = None,
    now: datetime | None = None,
    ttl_seconds: int = 3600,
) -> dict[str, Any]:
    if ttl_seconds < 1:
        raise AssuranceError("security envelope TTL must be positive")
    if len(set(allowed_capabilities)) != len(allowed_capabilities):
        raise AssuranceError("allowed capabilities must be unique")
    if not denied_capabilities or len(set(denied_capabilities)) != len(
        denied_capabilities
    ):
        raise AssuranceError("denied capabilities must be non-empty and unique")
    overlap = sorted(set(allowed_capabilities) & set(denied_capabilities))
    if overlap:
        raise AssuranceError(f"capabilities cannot be both allowed and denied: {overlap}")

    selected_now = now or datetime.now(timezone.utc)
    envelope_id = new_envelope_id()
    nonce = secrets.token_hex(24)
    capability_projection = {
        "conversation_id": conversation_id,
        "envelope_id": envelope_id,
        "nonce": nonce,
        "allowed": sorted(allowed_capabilities),
        "denied": sorted(denied_capabilities),
    }
    capability_digest = sha256_bytes(canonical_bytes(capability_projection))
    body: dict[str, Any] = {
        "schema_version": "0.1.0-draft",
        "envelope_kind": "effective_security_envelope",
        "envelope_id": envelope_id,
        "installation_key_id": key_store.key_id,
        "conversation_id": conversation_id,
        "resumed_from_conversation_id": resumed_from_conversation_id,
        "parent_envelope_id": parent_envelope_id,
        "created_at": _timestamp(selected_now),
        "expires_at": _timestamp(selected_now + timedelta(seconds=ttl_seconds)),
        "lifecycle_state": "active",
        "runtime_selection_policy": "capability_gated",
        "frozen_context": deepcopy(frozen_context),
        "capability_envelope": {
            "sequence": 0,
            "nonce": nonce,
            "allowed": sorted(allowed_capabilities),
            "denied": sorted(denied_capabilities),
            "capability_digest": _evidenced_sha256(
                capability_digest,
                source_ref="assurance.envelope:capability-projection",
            ),
        },
    }
    signed_payload = canonical_bytes(body)
    signed_digest = sha256_bytes(signed_payload)
    envelope = {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_store": _evidenced_string(
                key_store.storage_id,
                source_ref=f"installation-key-metadata:{key_store.key_id}",
            ),
            "signature_algorithm": "hmac-sha256",
            "signature_verification": {
                "value": True,
                "evidence_status": "observed",
                "source_refs": ["assurance.envelope:create-and-verify"],
            },
            "signed_payload_digest": _evidenced_sha256(
                signed_digest,
                source_ref="assurance.envelope:RFC8785",
            ),
            "signature": key_store.sign(signed_payload),
        },
    }
    validate_contract(
        envelope,
        "effective-security-envelope-v0.1.schema.json",
        label="effective security envelope",
    )
    verify_security_envelope(envelope, key_store=key_store)
    return envelope


def verify_security_envelope(
    envelope: dict[str, Any], *, key_store: InstallationKeyStore
) -> dict[str, Any]:
    validate_contract(
        envelope,
        "effective-security-envelope-v0.1.schema.json",
        label="effective security envelope",
    )
    if envelope["installation_key_id"] != key_store.key_id:
        raise AssuranceError("security envelope installation key ID mismatch")
    body = {key: deepcopy(value) for key, value in envelope.items() if key != "integrity"}
    signed_payload = canonical_bytes(body)
    actual_digest = sha256_bytes(signed_payload)
    expected_digest = envelope["integrity"]["signed_payload_digest"]["value"]
    if actual_digest != expected_digest:
        raise AssuranceError("security envelope signed payload digest mismatch")
    if not key_store.verify(signed_payload, envelope["integrity"]["signature"]):
        raise AssuranceError("security envelope signature verification failed")

    capability = envelope["capability_envelope"]
    capability_projection = {
        "conversation_id": envelope["conversation_id"],
        "envelope_id": envelope["envelope_id"],
        "nonce": capability["nonce"],
        "allowed": capability["allowed"],
        "denied": capability["denied"],
    }
    if (
        sha256_bytes(canonical_bytes(capability_projection))
        != capability["capability_digest"]["value"]
    ):
        raise AssuranceError("security envelope capability digest mismatch")
    return {
        "valid": True,
        "conversation_id": envelope["conversation_id"],
        "envelope_id": envelope["envelope_id"],
        "installation_key_id": envelope["installation_key_id"],
        "signed_payload_sha256": actual_digest,
    }
