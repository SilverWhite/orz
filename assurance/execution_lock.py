from __future__ import annotations

from importlib import metadata
from pathlib import Path
import platform
import sys
from typing import Any

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, sha256_file
from .runtime_preflight import verify_gsa_runtime_preflight_journal


ROOT = ASSURANCE_ROOT.parent
EXECUTION_LOCK_SCHEMA = "gsa-execution-lock-v0.1.schema.json"
PROOF_SCHEMA = "disposable-reproduction-run-proof-v0.1.schema.json"
PROJECTION_SCHEMA = "gsa-runtime-preflight-projection-v0.1.schema.json"
JOURNAL_RECEIPT_SCHEMA = "gsa-runtime-journal-receipt-v0.1.schema.json"

LOCKED_SOURCE_PATHS = (
    "assurance/disposable_reproduction.py",
    "assurance/runtime_preflight.py",
    "assurance/execution_lock.py",
    "assurance/runner.py",
    "assurance/journal_lock.py",
    "assurance/contracts.py",
    "assurance/utils.py",
    "assurance/disposable-reproduction-manifest-v0.1.schema.json",
    "assurance/disposable-reproduction-receipt-v0.1.schema.json",
    "assurance/disposable-reproduction-verification-v0.1.schema.json",
    "assurance/disposable-reproduction-run-proof-v0.1.schema.json",
    "assurance/gsa-runtime-preflight-projection-v0.1.schema.json",
    "assurance/gsa-runtime-journal-receipt-v0.1.schema.json",
    "assurance/gsa-execution-lock-v0.1.schema.json",
    "assurance/gsa-no-model-runner-receipt-v0.1.schema.json",
    "assurance/gsa-runner-journal-recovery-inspection-v0.1.schema.json",
    "assurance/gsa-runner-journal-recovery-receipt-v0.1.schema.json",
    "assurance/gsa-runner-lifecycle-repair-policy-v0.1.schema.json",
    "runtime/run-manifest-v0.1.schema.json",
    "runtime/run-event-v0.1.schema.json",
)

RUNTIME_DISTRIBUTIONS = ("jsonschema", "rfc8785")


def _relative_file_digest(relative_path: str) -> dict[str, str]:
    path = ROOT / relative_path
    if not path.is_file():
        raise AssuranceError(f"locked source file is missing: {relative_path}")
    return {
        "path": relative_path,
        "sha256": sha256_file(path),
    }


def _selected_source_files() -> list[dict[str, str]]:
    return [_relative_file_digest(path) for path in LOCKED_SOURCE_PATHS]


def _distribution_versions() -> list[dict[str, str]]:
    distributions: list[dict[str, str]] = []
    for name in RUNTIME_DISTRIBUTIONS:
        try:
            version = metadata.version(name)
        except metadata.PackageNotFoundError as exc:
            raise AssuranceError(f"required Python distribution is missing: {name}") from exc
        distributions.append({"name": name, "version": version})
    return sorted(distributions, key=lambda item: item["name"].lower())


def _environment_lock() -> dict[str, Any]:
    return {
        "python_version": platform.python_version(),
        "python_implementation": platform.python_implementation(),
        "platform_system": platform.system() or "unknown",
        "platform_release": platform.release() or "unknown",
        "filesystem_encoding": sys.getfilesystemencoding() or "unknown",
        "dependency_lock_status": "python_distribution_snapshot_only",
        "python_distributions": _distribution_versions(),
    }


def _packed_ref(ref_name: str, packed_refs_path: Path) -> str | None:
    if not packed_refs_path.is_file():
        return None
    for line in packed_refs_path.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#") or line.startswith("^"):
            continue
        parts = line.split(" ", 1)
        if len(parts) == 2 and parts[1] == ref_name:
            return parts[0]
    return None


def _git_snapshot() -> dict[str, Any]:
    git_root = ROOT / ".git"
    head_path = git_root / "HEAD"
    if not head_path.is_file():
        return {
            "repository_detected": False,
            "head_ref": None,
            "head_sha256": None,
            "status_scope": "selected_files_hashed_only",
        }
    head = head_path.read_text(encoding="utf-8").strip()
    head_ref: str | None = None
    head_sha: str | None = None
    if head.startswith("ref: "):
        head_ref = head.removeprefix("ref: ").strip()
        ref_path = git_root / head_ref
        if ref_path.is_file():
            head_sha = ref_path.read_text(encoding="utf-8").strip()
        else:
            head_sha = _packed_ref(head_ref, git_root / "packed-refs")
    else:
        head_ref = "detached"
        head_sha = head
    return {
        "repository_detected": True,
        "head_ref": head_ref,
        "head_sha256": head_sha,
        "status_scope": "selected_files_hashed_only",
    }


def build_disposable_reproduction_execution_lock(
    proof: dict[str, Any],
    projection: dict[str, Any],
    journal_receipt: dict[str, Any],
    *,
    journal_path: Path,
) -> dict[str, Any]:
    validate_contract(
        proof,
        PROOF_SCHEMA,
        label="disposable reproduction run proof",
    )
    validate_contract(
        projection,
        PROJECTION_SCHEMA,
        label="GSA runtime preflight projection",
    )
    validate_contract(
        journal_receipt,
        JOURNAL_RECEIPT_SCHEMA,
        label="GSA runtime preflight journal receipt",
    )
    verify_gsa_runtime_preflight_journal(
        journal_receipt,
        projection,
        journal_path=journal_path,
    )
    files = _selected_source_files()
    execution_lock = {
        "schema_version": "0.1.0-draft",
        "lock_kind": "gsa_disposable_reproduction_execution_lock",
        "reproduction_id": proof["reproduction_id"],
        "proof_sha256": sha256_bytes(canonical_bytes(proof)),
        "projection_sha256": sha256_bytes(canonical_bytes(projection)),
        "journal_receipt_sha256": sha256_bytes(canonical_bytes(journal_receipt)),
        "selected_source_tree_sha256": sha256_bytes(canonical_bytes(files)),
        "selected_source_files": files,
        "git": _git_snapshot(),
        "environment": _environment_lock(),
        "checks": {
            "proof_schema_valid": True,
            "projection_schema_valid": True,
            "journal_receipt_schema_valid": True,
            "journal_replayed": True,
            "selected_files_exist": True,
            "selected_source_tree_hashed": True,
            "dependency_versions_recorded": True,
            "no_dependency_lock_claimed": True,
            "no_claim_promotion": True,
        },
        "evidence_boundary": {
            "runner_status": "development_code_environment_lock_only",
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
        },
        "valid": True,
    }
    validate_contract(
        execution_lock,
        EXECUTION_LOCK_SCHEMA,
        label="GSA execution lock",
    )
    return execution_lock


def verify_disposable_reproduction_execution_lock(
    execution_lock: dict[str, Any],
    proof: dict[str, Any],
    projection: dict[str, Any],
    journal_receipt: dict[str, Any],
    *,
    journal_path: Path,
) -> dict[str, Any]:
    validate_contract(
        execution_lock,
        EXECUTION_LOCK_SCHEMA,
        label="GSA execution lock",
    )
    expected = build_disposable_reproduction_execution_lock(
        proof,
        projection,
        journal_receipt,
        journal_path=journal_path,
    )
    if canonical_bytes(execution_lock) != canonical_bytes(expected):
        raise AssuranceError("GSA execution lock mismatch")
    return {
        "schema_version": "0.1.0-draft",
        "verification_kind": "gsa_disposable_reproduction_execution_lock_verification",
        "reproduction_id": execution_lock["reproduction_id"],
        "valid": True,
        "selected_source_tree_sha256": execution_lock["selected_source_tree_sha256"],
        "locked_source_file_count": len(execution_lock["selected_source_files"]),
        "dependency_lock_status": execution_lock["environment"]["dependency_lock_status"],
        "evidence_boundary": execution_lock["evidence_boundary"],
    }
