from __future__ import annotations

import uuid
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, sha256_file, utc_now


UNIFIED_TRUST_SCHEMA = "workspace-trust-receipt-v0.1.schema.json"


def establish_workspace_trust(
    *,
    workspace_root: Path,
    adapter_id: str,
    conversation_id: str,
    policy_sha256: str | None = None,
    required_files_digests: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Establish trust in a workspace before any adapter discovery or execution.

    This is the unified entry point that ALL adapters must use before
    reading project rules, hooks, MCP configs, or external content.

    Returns a trust receipt that must be consumed by the instruction
    provenance gate as evidence of ``workspace_trust=observed_trusted``.
    """
    errors: list[str] = []
    checks: dict[str, bool] = {}

    # Verify workspace exists and is a directory
    if not workspace_root.is_dir():
        errors.append("workspace root does not exist or is not a directory")
        checks["workspace_exists"] = False
    else:
        checks["workspace_exists"] = True

    # Count files and compute aggregate digest
    file_count = 0
    aggregate_parts: list[str] = []
    try:
        for path in sorted(workspace_root.rglob("*")):
            if path.is_file() and not path.is_symlink():
                file_count += 1
                aggregate_parts.append(sha256_file(path))
    except OSError as exc:
        errors.append(f"workspace scan failed: {exc}")
        checks["workspace_scanned"] = False
    else:
        checks["workspace_scanned"] = True

    aggregate_sha256 = sha256_bytes(
        canonical_bytes(sorted(aggregate_parts))
    )

    # Verify required files match expected digests
    if required_files_digests:
        for rel_path, expected_digest in required_files_digests.items():
            target = workspace_root / rel_path
            if not target.is_file():
                errors.append(f"required file missing: {rel_path}")
                checks[f"file_present:{rel_path}"] = False
            else:
                actual = sha256_file(target)
                if actual != expected_digest:
                    errors.append(f"file digest mismatch: {rel_path}")
                    checks[f"file_digest:{rel_path}"] = False
                else:
                    checks[f"file_digest:{rel_path}"] = True

    trust_established = not errors

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "workspace_trust_receipt",
        "receipt_id": f"WTR-{uuid.uuid4().hex.upper()}",
        "conversation_id": conversation_id,
        "adapter_id": adapter_id,
        "created_at": utc_now(),
        "workspace": {
            "root_path": str(workspace_root.resolve()),
            "aggregate_sha256": aggregate_sha256,
            "file_count": file_count,
            "policy_sha256": policy_sha256 or "",
        },
        "trust_established": trust_established,
        "trust_granted": trust_established,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "Workspace trust is established at a point in time; file changes invalidate trust.",
            "Trust covers declared workspace files only, not external dependencies or environment.",
            "Trust receipt does not grant execution permission; it only records observed state.",
        ],
    }
    validate_contract(receipt, UNIFIED_TRUST_SCHEMA, label="workspace trust receipt")
    return receipt


def verify_workspace_trust(
    receipt: dict[str, Any],
    *,
    workspace_root: Path,
) -> dict[str, Any]:
    """Independently verify that workspace trust is still valid.

    Re-scans the workspace and compares the aggregate digest with the
    receipt.  Trust is invalidated if any file has changed.
    """
    errors: list[str] = []
    validate_contract(receipt, UNIFIED_TRUST_SCHEMA, label="workspace trust receipt")

    if not workspace_root.is_dir():
        return {"valid": False, "trust_still_valid": False, "errors": ["workspace root missing"]}

    aggregate_parts: list[str] = []
    try:
        for path in sorted(workspace_root.rglob("*")):
            if path.is_file() and not path.is_symlink():
                aggregate_parts.append(sha256_file(path))
    except OSError as exc:
        return {"valid": False, "trust_still_valid": False, "errors": [str(exc)]}

    current_aggregate = sha256_bytes(canonical_bytes(sorted(aggregate_parts)))
    recorded_aggregate = receipt["workspace"]["aggregate_sha256"]

    trust_still_valid = current_aggregate == recorded_aggregate
    if not trust_still_valid:
        errors.append(
            f"workspace aggregate digest changed: "
            f"recorded={recorded_aggregate[:16]}..., current={current_aggregate[:16]}..."
        )

    return {
        "valid": not errors,
        "trust_still_valid": trust_still_valid,
        "recorded_aggregate_sha256": recorded_aggregate,
        "current_aggregate_sha256": current_aggregate,
        "errors": errors,
    }


def workspace_trust_for_adapter(
    receipt: dict[str, Any],
    *,
    adapter_id: str,
) -> str:
    """Return the effective workspace_trust status for an adapter.

    Returns ``"observed_trusted"`` if the receipt is valid and applies
    to the given adapter, ``"not_observed"`` otherwise.
    """
    if receipt.get("adapter_id") != adapter_id:
        return "not_observed"
    if not receipt.get("trust_established", False):
        return "not_observed"
    return "observed_trusted"
