from __future__ import annotations

from copy import deepcopy
from pathlib import Path
from typing import Any
import uuid

from .archive import ArchiveController
from .archive_verifier import verify_archive
from .audit import AuditLedger, verify_audit_seal
from .contracts import ASSURANCE_ROOT, validate_contract
from .conversation import ConversationNamespace
from .errors import AssuranceError
from .guarded_execution import workspace_content_sha256
from .instruction_gate import (
    authorize_action_candidate,
    record_instruction_provenance,
    verify_action_authorization,
    verify_instruction_provenance,
)
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_json,
    canonical_bytes,
    exclusive_create_bytes,
    is_link_or_reparse,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)


BACKEND_POLICY_NAME = "execution-backend-policy-v0.1.json"
WORKSPACE_MARKER = ".assurance-p45-workspace.json"
PROBE_FILE = ".p4-5-workspace-roundtrip-probe"
PROBE_PAYLOAD = b"p4.5-workspace-guarded-ok\n"
ACTION_CONTENT = b"Execute the fixed no-model workspace roundtrip."
ACTION_KIND = "fixed_workspace_roundtrip"
ACTION_CAPABILITIES = [
    "action.fixed_no_model",
    "filesystem.workspace_read",
    "filesystem.workspace_write",
]
DENIED_CAPABILITIES = [
    "credential.raw_read",
    "host.mount",
    "network.unrestricted",
    "process.spawn",
    "secret.raw_read",
]
MARKER_VALUE = {
    "schema_version": "0.1.0-draft",
    "purpose": "workspace-guarded-standard",
    "allow_fixed_roundtrip": True,
    "acknowledge_logical_boundary_only": True,
}


def load_execution_backend_policy(
    path: Path | None = None,
) -> dict[str, Any]:
    selected = path or ASSURANCE_ROOT / BACKEND_POLICY_NAME
    policy = load_json(selected)
    validate_contract(
        policy,
        "execution-backend-policy-v0.1.schema.json",
        label="execution backend policy",
    )
    return policy


def select_execution_backend(
    mode: str,
    *,
    policy_path: Path | None = None,
) -> dict[str, Any]:
    policy = load_execution_backend_policy(policy_path)
    if mode == "standard":
        return deepcopy(policy["modes"]["standard"])
    if mode == "strict":
        raise AssuranceError(
            "strict mode requires an explicit, observed P2 Docker selection; "
            "policy forbids implicit Docker start and fallback to standard"
        )
    raise AssuranceError("execution mode must be standard or strict")


def initialize_workspace_marker(workspace: Path) -> Path:
    if not workspace.is_dir() or is_link_or_reparse(workspace):
        raise AssuranceError(
            "P4.5 workspace must be a non-linked existing directory"
        )
    marker = workspace.resolve(strict=True) / WORKSPACE_MARKER
    atomic_write_json(marker, MARKER_VALUE)
    return marker


def _validate_workspace(workspace: Path) -> Path:
    if not workspace.is_dir() or is_link_or_reparse(workspace):
        raise AssuranceError(
            "P4.5 workspace must be a non-linked existing directory"
        )
    resolved = workspace.resolve(strict=True)
    marker = resolved / WORKSPACE_MARKER
    if not marker.is_file() or is_link_or_reparse(marker):
        raise AssuranceError("P4.5 workspace marker is missing or invalid")
    if load_json(marker) != MARKER_VALUE:
        raise AssuranceError("P4.5 workspace marker is missing or invalid")
    if (resolved / PROBE_FILE).exists():
        raise AssuranceError("refusing to overwrite an existing P4.5 probe")
    return resolved


def _evidenced(
    value: str, *, source: str, status: str = "derived"
) -> dict[str, Any]:
    return {
        "value": value,
        "evidence_status": status,
        "source_refs": [source],
    }


def build_workspace_first_frozen_context(
    workspace: Path,
    *,
    policy_path: Path | None = None,
) -> dict[str, Any]:
    selected_policy = policy_path or ASSURANCE_ROOT / BACKEND_POLICY_NAME
    policy = load_execution_backend_policy(selected_policy)
    mode = policy["modes"]["standard"]
    resolved = _validate_workspace(workspace)
    runtime_projection = {
        "action_kind": ACTION_KIND,
        "docker_invoked": False,
        "model_invoked": False,
        "network_requested": False,
        "process_spawned": False,
    }
    return {
        "workspace_canonical_path_digest": _evidenced(
            sha256_bytes(str(resolved).encode("utf-8")),
            source="assurance.integrated_run:canonical-workspace",
        ),
        "workspace_content_digest": _evidenced(
            workspace_content_sha256(resolved),
            source="assurance.integrated_run:workspace-manifest",
        ),
        "workspace_policy_digest": _evidenced(
            sha256_file(resolved / WORKSPACE_MARKER),
            source="assurance.integrated_run:workspace-marker",
        ),
        "runtime_family": _evidenced(
            "runtime-neutral-no-model",
            source="assurance.integrated_run:fixed-action",
            status="observed",
        ),
        "runtime_adapter_id": _evidenced(
            "assurance-workspace-guarded",
            source="assurance.integrated_run:fixed-action",
            status="observed",
        ),
        "runtime_binary_digest": _evidenced(
            sha256_file(Path(__file__).resolve()),
            source="assurance.integrated_run:module",
        ),
        "runtime_capabilities_digest": _evidenced(
            sha256_bytes(canonical_bytes(runtime_projection)),
            source="assurance.integrated_run:capability-projection",
        ),
        "assurance_config_digest": _evidenced(
            sha256_file(selected_policy),
            source="assurance.integrated_run:backend-policy",
        ),
        "sandbox_backend": _evidenced(
            mode["backend"],
            source="assurance.integrated_run:backend-policy",
            status="observed",
        ),
        "sandbox_backend_digest": _evidenced(
            sha256_bytes(canonical_bytes(mode)),
            source="assurance.integrated_run:backend-mode",
        ),
    }


def _sign_receipt(
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


def _verify_signed_receipt(
    receipt: dict[str, Any], *, key_store: InstallationKeyStore
) -> list[str]:
    errors: list[str] = []
    integrity = receipt["integrity"]
    body = {key: value for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    if integrity["key_id"] != key_store.key_id:
        errors.append("receipt key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(payload):
        errors.append("receipt signed payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        errors.append("receipt signature verification failed")
    return errors


def _fixed_workspace_roundtrip(workspace: Path) -> dict[str, Any]:
    probe = workspace / PROBE_FILE
    if probe.exists():
        raise AssuranceError("refusing to overwrite an existing P4.5 probe")
    cleaned = False
    try:
        exclusive_create_bytes(probe, PROBE_PAYLOAD)
        if is_link_or_reparse(probe) or not probe.is_file():
            raise AssuranceError("P4.5 probe is not a regular non-linked file")
        observed = probe.read_bytes()
        if observed != PROBE_PAYLOAD:
            raise AssuranceError("P4.5 probe roundtrip mismatch")
        return {
            "action_kind": ACTION_KIND,
            "outcome": "succeeded",
            "probe_payload_sha256": sha256_bytes(observed),
            "probe_bytes": len(observed),
        }
    finally:
        try:
            probe.unlink(missing_ok=True)
        finally:
            cleaned = not probe.exists()
        if not cleaned:
            raise AssuranceError("P4.5 probe cleanup failed")


def execute_workspace_first_integrated_run(
    *,
    workspace: Path,
    conversation_repository: Path,
    key_store: InstallationKeyStore,
    policy_path: Path | None = None,
) -> dict[str, Any]:
    selected_policy = policy_path or ASSURANCE_ROOT / BACKEND_POLICY_NAME
    policy = load_execution_backend_policy(selected_policy)
    backend = select_execution_backend("standard", policy_path=selected_policy)
    resolved = _validate_workspace(workspace)
    before_sha256 = workspace_content_sha256(resolved)
    frozen_context = build_workspace_first_frozen_context(
        resolved, policy_path=selected_policy
    )
    namespace = ConversationNamespace.create(
        conversation_repository,
        key_store=key_store,
        frozen_context=frozen_context,
        allowed_capabilities=ACTION_CAPABILITIES,
        denied_capabilities=DENIED_CAPABILITIES,
    )
    envelope = namespace.load_active_envelope()
    action = {
        "action_id": "ACT-P45-FIXED",
        "action_type": "filesystem.fixed_roundtrip",
        "risk_class": "normal",
        "target_sha256": sha256_bytes(str(resolved).encode("utf-8")),
        "parameters_sha256": sha256_bytes(
            canonical_bytes(
                {
                    "action_kind": ACTION_KIND,
                    "probe_file": PROBE_FILE,
                    "probe_payload_sha256": sha256_bytes(PROBE_PAYLOAD),
                }
            )
        ),
        "requested_capabilities": ACTION_CAPABILITIES,
    }
    provenance = record_instruction_provenance(
        key_store=key_store,
        envelope=envelope,
        declared_source_type="user",
        source_id="user:local-p45",
        content=ACTION_CONTENT,
        structured_action=action,
    )
    provenance_verification = verify_instruction_provenance(
        provenance,
        key_store=key_store,
        envelope=envelope,
        content=ACTION_CONTENT,
    )
    authorization, consumed = authorize_action_candidate(
        namespace=namespace,
        key_store=key_store,
        provenance_receipt=provenance,
        content=ACTION_CONTENT,
        attempt=1,
    )
    authorization_verification = verify_action_authorization(
        authorization,
        key_store=key_store,
        envelope=envelope,
        provenance_receipt=provenance,
        content=ACTION_CONTENT,
        consumed_permit=consumed,
        issued_permit=None,
    )
    if (
        not provenance_verification["valid"]
        or not authorization_verification["valid"]
        or authorization["decision"]["outcome"] != "allow"
    ):
        raise AssuranceError("P3 action authorization failed closed")

    ledger = AuditLedger(namespace=namespace, key_store=key_store)
    ledger.append(
        source_kind="kernel",
        source_id="kernel:p45",
        source_sequence=0,
        source_completeness="complete",
        canonical_event_type="action.authorized",
        raw_payload=canonical_bytes(authorization),
        raw_retention_category="raw_tool_result",
        provenance_status="direct",
        source_refs=["assurance.integrated_run:p3-authorization"],
        facts={"attempt": 1, "allowed": True},
    )
    result = _fixed_workspace_roundtrip(resolved)
    after_sha256 = workspace_content_sha256(resolved)
    if before_sha256 != after_sha256:
        raise AssuranceError("workspace changed after fixed action cleanup")
    ledger.append(
        source_kind="runtime",
        source_id="runtime:p45",
        source_sequence=0,
        source_completeness="complete",
        canonical_event_type="action.completed",
        raw_payload=canonical_bytes(result),
        raw_retention_category="full_stdout_stderr",
        provenance_status="direct",
        source_refs=["assurance.integrated_run:fixed-roundtrip"],
        facts={
            "status": "succeeded",
            "probe_bytes": result["probe_bytes"],
            "cleanup_complete": True,
        },
    )
    ledger.append(
        source_kind="supervisor",
        source_id="supervisor:p45",
        source_sequence=0,
        source_completeness="complete",
        canonical_event_type="run.terminal",
        raw_payload=canonical_bytes(
            {
                "status": "succeeded",
                "model_invoked": False,
                "network_requested": False,
                "process_spawned": False,
                "docker_invoked": False,
            }
        ),
        raw_retention_category="full_stdout_stderr",
        provenance_status="direct",
        source_refs=["assurance.integrated_run:supervisor"],
        facts={"status": "succeeded", "residue_count": 0},
        terminal_outcome="succeeded",
    )
    audit_seal = ledger.seal(
        source_completeness={
            "acp": "unknown",
            "session": "unknown",
            "provider": "unknown",
            "supervisor": "complete",
            "kernel": "complete",
            "runtime": "complete",
        }
    )
    audit_prearchive = verify_audit_seal(
        audit_seal, namespace=namespace, key_store=key_store
    )
    if not audit_prearchive["valid"]:
        raise AssuranceError("P4 audit seal failed before archive")

    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "integrated_assurance_run",
        "receipt_id": f"IAR-{uuid.uuid4().hex.upper()}",
        "conversation_id": namespace.conversation_id,
        "envelope_id": envelope["envelope_id"],
        "created_at": utc_now(),
        "mode": "standard",
        "backend": backend["backend"],
        "security_claim": backend["security_claim"],
        "bindings": {
            "policy_sha256": sha256_file(selected_policy),
            "envelope_sha256": sha256_bytes(canonical_bytes(envelope)),
            "provenance_receipt_sha256": sha256_bytes(
                canonical_bytes(provenance)
            ),
            "action_authorization_receipt_sha256": sha256_bytes(
                canonical_bytes(authorization)
            ),
            "audit_seal_sha256": sha256_bytes(canonical_bytes(audit_seal)),
            "workspace_path_sha256": sha256_bytes(
                str(resolved).encode("utf-8")
            ),
            "workspace_before_sha256": before_sha256,
            "workspace_after_sha256": after_sha256,
        },
        "execution": {
            **result,
            "model_invoked": False,
            "network_requested": False,
            "process_spawned": False,
            "docker_invoked": False,
            "cleanup_complete": not (resolved / PROBE_FILE).exists(),
        },
        "verification": {
            "provenance_prearchive_valid": True,
            "authorization_prearchive_valid": True,
            "audit_prearchive_valid": True,
            "archive_complete_required": True,
        },
    }
    receipt = _sign_receipt(body, key_store=key_store)
    validate_contract(
        receipt,
        "integrated-assurance-run-receipt-v0.1.schema.json",
        label="P4.5 integrated assurance run receipt",
    )
    retained = {
        "provenance.json": provenance,
        "action-authorization.json": authorization,
        "audit-seal.json": audit_seal,
        "integrated-run-receipt.json": receipt,
    }
    for name, value in retained.items():
        namespace.write_artifact(
            "redacted_conversation",
            f"p4-5/{name}",
            canonical_bytes(value),
        )
    archive_result = ArchiveController(key_store=key_store).archive(namespace)
    if not archive_result["archive_complete"]:
        raise AssuranceError("P4.5 archive did not complete")
    verification = verify_workspace_first_integrated_run(
        receipt,
        namespace=namespace,
        key_store=key_store,
        workspace=resolved,
        policy_path=selected_policy,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "P4.5 post-archive verification failed: "
            + "; ".join(verification["errors"])
        )
    return {
        "namespace": namespace,
        "receipt": receipt,
        "archive": archive_result,
        "verification": verification,
        "policy": policy,
    }


def verify_workspace_first_integrated_run(
    receipt: dict[str, Any],
    *,
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    workspace: Path,
    policy_path: Path | None = None,
) -> dict[str, Any]:
    errors: list[str] = []
    selected_policy = policy_path or ASSURANCE_ROOT / BACKEND_POLICY_NAME
    try:
        validate_contract(
            receipt,
            "integrated-assurance-run-receipt-v0.1.schema.json",
            label="P4.5 integrated assurance run receipt",
        )
        errors.extend(_verify_signed_receipt(receipt, key_store=key_store))
        policy = load_execution_backend_policy(selected_policy)
        resolved = _validate_workspace(workspace)
        provenance = load_json(
            namespace.artifacts_root
            / "redacted_conversation"
            / "p4-5"
            / "provenance.json"
        )
        authorization = load_json(
            namespace.artifacts_root
            / "redacted_conversation"
            / "p4-5"
            / "action-authorization.json"
        )
        audit_seal = load_json(
            namespace.artifacts_root
            / "redacted_conversation"
            / "p4-5"
            / "audit-seal.json"
        )
        validate_contract(
            provenance,
            "instruction-provenance-receipt-v0.1.schema.json",
            label="retained P4.5 provenance receipt",
        )
        validate_contract(
            authorization,
            "action-authorization-receipt-v0.1.schema.json",
            label="retained P4.5 authorization receipt",
        )
        errors.extend(
            f"provenance: {item}"
            for item in _verify_signed_receipt(
                provenance, key_store=key_store
            )
        )
        errors.extend(
            f"authorization: {item}"
            for item in _verify_signed_receipt(
                authorization, key_store=key_store
            )
        )
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}

    bindings = receipt["bindings"]
    expected = {
        "policy_sha256": sha256_file(selected_policy),
        "provenance_receipt_sha256": sha256_bytes(canonical_bytes(provenance)),
        "action_authorization_receipt_sha256": sha256_bytes(
            canonical_bytes(authorization)
        ),
        "audit_seal_sha256": sha256_bytes(canonical_bytes(audit_seal)),
        "workspace_path_sha256": sha256_bytes(str(resolved).encode("utf-8")),
        "workspace_after_sha256": workspace_content_sha256(resolved),
    }
    for name, value in expected.items():
        if bindings[name] != value:
            errors.append(f"{name} mismatch")
    if bindings["workspace_before_sha256"] != bindings["workspace_after_sha256"]:
        errors.append("workspace was not restored after fixed action")
    if receipt["conversation_id"] != namespace.conversation_id:
        errors.append("conversation mismatch")
    if authorization["provenance_receipt_sha256"] != bindings[
        "provenance_receipt_sha256"
    ]:
        errors.append("authorization-to-provenance binding mismatch")
    if authorization["decision"]["outcome"] != "allow":
        errors.append("retained authorization is not allow")
    if authorization["decision"]["authority_source"] != "envelope":
        errors.append("retained authorization authority is not envelope")
    if provenance["decision"]["routing"] != "candidate":
        errors.append("retained provenance was not routed as candidate")
    if provenance["structured_action"]["requested_capabilities"] != (
        ACTION_CAPABILITIES
    ):
        errors.append("retained action capability projection mismatch")
    if policy["modes"]["strict"]["fallback_to_standard"]:
        errors.append("strict policy unexpectedly permits fallback")
    if (resolved / PROBE_FILE).exists():
        errors.append("fixed action probe residue remains")

    archive_verification = verify_archive(namespace, key_store=key_store)
    if (
        not archive_verification["valid"]
        or not archive_verification["archive_complete"]
    ):
        errors.append("conversation archive is not independently complete")
    audit_verification = verify_audit_seal(
        audit_seal,
        namespace=namespace,
        key_store=key_store,
        require_archive_complete=True,
    )
    if not audit_verification["valid"]:
        errors.extend(
            f"audit: {item}" for item in audit_verification["errors"]
        )
    if not audit_verification["raw_payload_absence_proven"]:
        errors.append("audit raw payload absence was not proven")
    return {
        "valid": not errors,
        "errors": errors,
        "archive_complete": archive_verification.get(
            "archive_complete", False
        ),
        "audit_valid": audit_verification["valid"],
        "raw_payload_absence_proven": audit_verification[
            "raw_payload_absence_proven"
        ],
        "docker_invoked": receipt["execution"]["docker_invoked"],
        "process_spawned": receipt["execution"]["process_spawned"],
        "model_invoked": receipt["execution"]["model_invoked"],
        "network_requested": receipt["execution"]["network_requested"],
    }
