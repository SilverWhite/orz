from __future__ import annotations

from pathlib import Path
from typing import Any
import uuid

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_bytes,
    canonical_bytes,
    is_link_or_reparse,
    load_json,
    require_no_linked_ancestors,
    require_within,
    safe_relative_path,
    sha256_bytes,
    sha256_file,
    utc_now,
)


DEFAULT_PROJECTION = (
    ASSURANCE_ROOT
    / "fixtures"
    / "p5"
    / "lif-r211-readonly-projection-v0.1.json"
)
PROJECTION_SCHEMA = "readonly-task-projection-v0.1.schema.json"
RECEIPT_SCHEMA = "readonly-task-projection-receipt-v0.1.schema.json"


def load_readonly_task_projection(
    path: Path | None = None,
) -> dict[str, Any]:
    selected = path or DEFAULT_PROJECTION
    projection = load_json(selected)
    validate_contract(
        projection,
        PROJECTION_SCHEMA,
        label="P5 read-only task projection",
    )
    relative_paths = [item["relative_path"] for item in projection["files"]]
    if len(relative_paths) != len(set(relative_paths)):
        raise AssuranceError("P5 projection paths must be unique")
    roles = [item["role"] for item in projection["files"]]
    required_roles = {
        "claim_router",
        "environment_router",
        "self_check",
        "current_map",
        "discussion",
        "analysis_code",
        "result_manifest",
        "result_summary",
    }
    if set(roles) != required_roles:
        raise AssuranceError("P5 projection must include every required role")
    return projection


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
    receipt: dict[str, Any], *, key_store: InstallationKeyStore
) -> list[str]:
    errors: list[str] = []
    body = {key: value for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    integrity = receipt["integrity"]
    if integrity["key_id"] != key_store.key_id:
        errors.append("read-only projection key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(payload):
        errors.append("read-only projection signed payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        errors.append("read-only projection signature verification failed")
    return errors


def _source_entry(
    source_root: Path, item: dict[str, str]
) -> tuple[Path, dict[str, Any]]:
    relative = safe_relative_path(item["relative_path"])
    path = source_root / Path(*relative.parts)
    require_within(path, source_root, must_exist=True)
    require_no_linked_ancestors(path, source_root)
    if is_link_or_reparse(path) or not path.is_file():
        raise AssuranceError(
            f"P5 source projection requires a regular non-linked file: {path}"
        )
    size = path.stat().st_size
    if size < 1:
        raise AssuranceError(f"P5 source projection file is empty: {path}")
    return path, {
        "relative_path": item["relative_path"],
        "role": item["role"],
        "bytes": size,
        "sha256": sha256_file(path),
    }


def _aggregate(entries: list[dict[str, Any]]) -> str:
    projection = [
        {
            "relative_path": item["relative_path"],
            "role": item["role"],
            "bytes": item["bytes"],
            "sha256": item["sha256"],
        }
        for item in entries
    ]
    return sha256_bytes(canonical_bytes(projection))


def project_complex_task_readonly(
    *,
    source_root: Path,
    snapshot_root: Path,
    key_store: InstallationKeyStore,
    projection_path: Path | None = None,
) -> dict[str, Any]:
    projection_file = projection_path or DEFAULT_PROJECTION
    projection = load_readonly_task_projection(projection_file)
    if not source_root.is_dir() or is_link_or_reparse(source_root):
        raise AssuranceError(
            "P5 source root must be an existing non-linked directory"
        )
    resolved_source = source_root.resolve(strict=True)
    if snapshot_root.exists():
        raise AssuranceError("refusing to reuse a P5 snapshot root")
    snapshot_root.mkdir(parents=True)
    if is_link_or_reparse(snapshot_root):
        raise AssuranceError("P5 snapshot root cannot be linked")

    source_paths: dict[str, Path] = {}
    before: list[dict[str, Any]] = []
    for item in projection["files"]:
        source, entry = _source_entry(resolved_source, item)
        source_paths[item["relative_path"]] = source
        before.append(entry)

    snapshot_entries: list[dict[str, Any]] = []
    for item in projection["files"]:
        relative = safe_relative_path(item["relative_path"])
        destination = snapshot_root / Path(*relative.parts)
        payload = source_paths[item["relative_path"]].read_bytes()
        atomic_write_bytes(destination, payload)
        snapshot_entries.append(
            {
                "relative_path": item["relative_path"],
                "role": item["role"],
                "bytes": len(payload),
                "sha256": sha256_file(destination),
            }
        )

    after: list[dict[str, Any]] = []
    for item in projection["files"]:
        _, entry = _source_entry(resolved_source, item)
        after.append(entry)
    if before != after:
        raise AssuranceError(
            "source content changed while building P5 read-only projection"
        )
    if before != snapshot_entries:
        raise AssuranceError("P5 snapshot content does not match source")

    file_receipts = [
        {
            "relative_path": source_item["relative_path"],
            "role": source_item["role"],
            "bytes": source_item["bytes"],
            "source_before_sha256": source_item["sha256"],
            "source_after_sha256": after_item["sha256"],
            "snapshot_sha256": snapshot_item["sha256"],
            "content_match": (
                source_item["sha256"]
                == after_item["sha256"]
                == snapshot_item["sha256"]
            ),
        }
        for source_item, after_item, snapshot_item in zip(
            before, after, snapshot_entries, strict=True
        )
    ]
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "readonly_complex_task_projection_receipt",
        "receipt_id": f"RTP-{uuid.uuid4().hex.upper()}",
        "created_at": utc_now(),
        "projection_id": projection["projection_id"],
        "bindings": {
            "projection_sha256": sha256_bytes(canonical_bytes(projection)),
            "source_root_path_sha256": sha256_bytes(
                str(resolved_source).encode("utf-8")
            ),
            "source_before_aggregate_sha256": _aggregate(before),
            "source_after_aggregate_sha256": _aggregate(after),
            "snapshot_aggregate_sha256": _aggregate(snapshot_entries),
        },
        "safety": {
            "source_opened_read_only": True,
            "source_write_attempted": False,
            "source_content_unchanged": True,
            "snapshot_is_untrusted_copy": True,
            "training_executed": False,
            "analysis_code_executed": False,
            "model_invoked": False,
            "network_requested": False,
            "docker_invoked": False,
            "child_process_spawned": False,
        },
        "files": file_receipts,
        "result": {
            "outcome": "projected_readonly",
            "projected_file_count": len(file_receipts),
            "source_mutation_detected": False,
        },
    }
    receipt = _sign(body, key_store=key_store)
    validate_contract(
        receipt,
        RECEIPT_SCHEMA,
        label="P5 read-only task projection receipt",
    )
    verification = verify_complex_task_readonly_projection(
        receipt,
        source_root=resolved_source,
        snapshot_root=snapshot_root,
        key_store=key_store,
        projection_path=projection_file,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "P5 read-only projection failed verification: "
            + "; ".join(verification["errors"])
        )
    return {
        "receipt": receipt,
        "verification": verification,
        "projection": projection,
    }


def verify_complex_task_readonly_projection(
    receipt: dict[str, Any],
    *,
    source_root: Path,
    snapshot_root: Path,
    key_store: InstallationKeyStore,
    projection_path: Path | None = None,
) -> dict[str, Any]:
    errors: list[str] = []
    projection_file = projection_path or DEFAULT_PROJECTION
    try:
        validate_contract(
            receipt,
            RECEIPT_SCHEMA,
            label="P5 read-only task projection receipt",
        )
        errors.extend(_verify_signature(receipt, key_store=key_store))
        projection = load_readonly_task_projection(projection_file)
        resolved_source = source_root.resolve(strict=True)
        resolved_snapshot = snapshot_root.resolve(strict=True)
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}

    if receipt["projection_id"] != projection["projection_id"]:
        errors.append("read-only projection identity mismatch")
    if receipt["bindings"]["projection_sha256"] != sha256_bytes(
        canonical_bytes(projection)
    ):
        errors.append("read-only projection manifest digest mismatch")
    if receipt["bindings"]["source_root_path_sha256"] != sha256_bytes(
        str(resolved_source).encode("utf-8")
    ):
        errors.append("read-only projection source-root digest mismatch")

    current_source: list[dict[str, Any]] = []
    current_snapshot: list[dict[str, Any]] = []
    for item in projection["files"]:
        try:
            _, source_entry = _source_entry(resolved_source, item)
            relative = safe_relative_path(item["relative_path"])
            snapshot = resolved_snapshot / Path(*relative.parts)
            require_within(snapshot, resolved_snapshot, must_exist=True)
            require_no_linked_ancestors(snapshot, resolved_snapshot)
            if is_link_or_reparse(snapshot) or not snapshot.is_file():
                raise AssuranceError("snapshot entry is not a regular file")
            snapshot_entry = {
                "relative_path": item["relative_path"],
                "role": item["role"],
                "bytes": snapshot.stat().st_size,
                "sha256": sha256_file(snapshot),
            }
            current_source.append(source_entry)
            current_snapshot.append(snapshot_entry)
        except Exception as exc:
            errors.append(f"{item['relative_path']}: {exc}")

    if len(current_source) == len(projection["files"]):
        source_aggregate = _aggregate(current_source)
        snapshot_aggregate = _aggregate(current_snapshot)
        bindings = receipt["bindings"]
        if source_aggregate != bindings["source_before_aggregate_sha256"]:
            errors.append("source differs from pre-projection aggregate")
        if source_aggregate != bindings["source_after_aggregate_sha256"]:
            errors.append("source differs from post-projection aggregate")
        if snapshot_aggregate != bindings["snapshot_aggregate_sha256"]:
            errors.append("snapshot aggregate mismatch")
        if current_source != current_snapshot:
            errors.append("snapshot bytes differ from current source")

    if len(receipt["files"]) != len(projection["files"]):
        errors.append("read-only projection file count mismatch")
    if receipt["result"]["projected_file_count"] != len(projection["files"]):
        errors.append("read-only projection result count mismatch")
    return {
        "valid": not errors,
        "errors": errors,
        "projection_id": projection["projection_id"],
        "projected_file_count": len(projection["files"]),
        "source_content_unchanged": not errors,
        "source_write_attempted": False,
        "training_executed": False,
        "analysis_code_executed": False,
        "model_invoked": False,
        "network_requested": False,
        "docker_invoked": False,
        "child_process_spawned": False,
    }
