from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import subprocess
from typing import Any, Iterable
import uuid

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import sha256_file, utc_now


ROOT = Path(__file__).resolve().parents[1]
RECEIPT_SCHEMA = "global-review-mode-receipt-v0.1.schema.json"


@dataclass(frozen=True)
class GlobalReviewPathFinding:
    path: str
    exists: bool
    sha256: str | None
    tags: list[str]
    risk_flags: list[str]


REVIEW_DIMENSIONS: list[dict[str, Any]] = [
    {
        "dimension_id": "design_intent_alignment",
        "question": "Does the current work still match the original project purpose, user intent, and design constraints?",
        "source_anchor": "CLI_PROJECT_INDEX.md#Global-Review-Mode",
        "flag_hints": [
            "design_reference_changed",
            "critical_design_surface_changed",
            "doc_or_index_changed",
        ],
    },
    {
        "dimension_id": "current_progress_judgment",
        "question": "Is the current progress state being judged honestly as pending, in progress, completed, blocked, or carried limitation?",
        "source_anchor": "CLI_PROJECT_INDEX.md#Pending-In-Progress",
        "flag_hints": [
            "progress_status_changed",
            "todo_order_changed",
            "doc_or_index_changed",
            "test_surface_changed",
        ],
    },
    {
        "dimension_id": "implementation_content_positioning",
        "question": "Is each changed implementation correctly positioned as product capability, adapter, fixture, test, document, prototype, or temporary scaffold?",
        "source_anchor": "CLI_PROJECT_INDEX.md#Term-Quick-Reference",
        "flag_hints": [
            "implementation_surface_changed",
            "adapter_or_integration_changed",
            "fixture_surface_changed",
            "prototype_surface_changed",
            "test_surface_changed",
            "doc_or_index_changed",
        ],
    },
    {
        "dimension_id": "critical_design_preservation",
        "question": "Were the task's key design constraints and required user-facing or assurance behaviours preserved rather than silently removed or degraded?",
        "source_anchor": "CLI_PROJECT_INDEX.md#Usage-Redlines",
        "flag_hints": [
            "critical_design_surface_changed",
            "ui_surface_changed",
            "retrieval_surface_changed",
            "inquiry_surface_changed",
            "progress_surface_changed",
        ],
    },
    {
        "dimension_id": "project_task_boundary",
        "question": "Is the project task boundary clear, with in-scope work, out-of-scope work, dependencies, and follow-up work kept separate?",
        "source_anchor": "CLI_PROJECT_INDEX.md#Pending-In-Progress",
        "flag_hints": [
            "project_task_boundary_changed",
            "scope_boundary_changed",
            "dependency_boundary_changed",
            "todo_order_changed",
        ],
    },
]


def _git_status_paths(repo_root: Path) -> tuple[list[str], bool, int | None]:
    try:
        completed = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=repo_root,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except Exception:
        return [], False, None
    paths: list[str] = []
    if completed.returncode == 0:
        for line in completed.stdout.splitlines():
            if not line.strip():
                continue
            raw = line[3:].strip()
            if " -> " in raw:
                raw = raw.split(" -> ", 1)[1]
            if raw:
                paths.append(raw.replace("\\", "/"))
    return paths, True, completed.returncode


def _normalize_relative_path(path: Path | str, repo_root: Path) -> str:
    value = str(path).replace("\\", "/")
    candidate = Path(path)
    if candidate.is_absolute():
        try:
            value = candidate.resolve(strict=False).relative_to(
                repo_root.resolve(strict=False)
            ).as_posix()
        except ValueError as exc:
            raise AssuranceError(f"path is outside workspace root: {path}") from exc
    if not value or value.startswith("../") or value == "..":
        raise AssuranceError(f"unsafe review path: {path}")
    return value


def _unique_paths(paths: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for path in paths:
        if path not in seen:
            seen.add(path)
            result.append(path)
    return result


def _tags_and_flags(relative_path: str) -> tuple[list[str], list[str]]:
    tags: set[str] = set()
    flags: set[str] = set()
    path = relative_path

    if path == "CLI_PROJECT_INDEX.md" or path.startswith("architecture/") or path.startswith("docs/"):
        tags.add("design_reference")
        flags.update({"design_reference_changed", "doc_or_index_changed"})
    if path == "CLI_PROJECT_INDEX.md":
        tags.add("progress_index")
        flags.update({"progress_status_changed", "todo_order_changed", "project_task_boundary_changed"})
    if "global_review_mode" in path or "global-review-mode" in path:
        tags.add("global_review_surface")
        flags.add("critical_design_surface_changed")
    if path.startswith("upstream/"):
        tags.add("dependency_reference")
        flags.add("dependency_boundary_changed")
    lowered_path = path.lower()
    if (
        path.startswith("integration/")
        or "adapter" in lowered_path
        or "normalizer" in lowered_path
        or "bridge" in lowered_path
    ):
        tags.add("adapter_or_integration")
        flags.add("adapter_or_integration_changed")
    if path.startswith("assurance/tui/"):
        tags.add("ui_surface")
        flags.add("ui_surface_changed")
    if path.startswith("assurance/retrieval") or path.startswith("assurance/browser_retrieval") or path.startswith("assurance/pdf_evidence") or path.startswith("assurance/evidence_store"):
        tags.add("retrieval_surface")
        flags.add("retrieval_surface_changed")
    if "orientation" in path or "counterexample" in path or "diagnostic" in path:
        tags.add("inquiry_surface")
        flags.add("inquiry_surface_changed")
    if "global-progress" in path or "global_progress" in path or path.startswith("assurance/runner"):
        tags.add("progress_surface")
        flags.add("progress_surface_changed")
    if path.startswith("assurance/") or path.startswith("scripts/"):
        tags.add("implementation_surface")
        flags.add("implementation_surface_changed")
    if path.startswith("assurance/tests/") or path.startswith("tests/"):
        tags.add("test_surface")
        flags.add("test_surface_changed")
    if path.startswith("fixtures/") or "/fixtures/" in path:
        tags.add("fixture_surface")
        flags.add("fixture_surface_changed")
    if path.startswith("prototype/"):
        tags.add("prototype_surface")
        flags.add("prototype_surface_changed")
    if "task" in path.lower() or "scope" in path.lower() or "boundary" in path.lower():
        tags.add("task_boundary_surface")
        flags.add("scope_boundary_changed")
    if not tags:
        tags.add("unclassified")
    return sorted(tags), sorted(flags)


def _path_finding(relative_path: str, repo_root: Path) -> GlobalReviewPathFinding:
    full_path = repo_root / relative_path
    exists = full_path.is_file()
    digest = sha256_file(full_path) if exists else None
    tags, flags = _tags_and_flags(relative_path)
    return GlobalReviewPathFinding(
        path=relative_path,
        exists=exists,
        sha256=digest,
        tags=tags,
        risk_flags=flags,
    )


def build_global_review_mode_receipt(
    *,
    paths: Iterable[Path | str] | None = None,
    include_git_status: bool = True,
    repo_root: Path = ROOT,
    review_id: str | None = None,
    created_at: str | None = None,
) -> dict[str, Any]:
    explicit_paths = [
        _normalize_relative_path(path, repo_root)
        for path in (paths or [])
    ]
    git_paths: list[str] = []
    git_available = False
    git_exit_code: int | None = None
    if include_git_status:
        git_paths, git_available, git_exit_code = _git_status_paths(repo_root)
        git_paths = [
            _normalize_relative_path(path, repo_root)
            for path in git_paths
        ]
    review_paths = _unique_paths([*explicit_paths, *git_paths])

    if explicit_paths and git_paths:
        source = "explicit_paths_plus_git_status"
    elif explicit_paths:
        source = "explicit_paths"
    elif git_paths:
        source = "git_status"
    else:
        source = "empty_scope"

    findings = [_path_finding(path, repo_root) for path in review_paths]
    observed_flags = {
        flag
        for finding in findings
        for flag in finding.risk_flags
    }
    dimensions = []
    for dimension in REVIEW_DIMENSIONS:
        dimensions.append(
            {
                "dimension_id": dimension["dimension_id"],
                "required": True,
                "question": dimension["question"],
                "triggered_by_risk_flags": sorted(
                    observed_flags & set(dimension["flag_hints"])
                ),
                "source_anchor": dimension["source_anchor"],
            }
        )

    receipt = {
        "schema_version": "0.1.0",
        "receipt_kind": "global_review_mode_receipt",
        "review_id": review_id or f"GRM-{uuid.uuid4().hex[:12].upper()}",
        "created_at": created_at or utc_now(),
        "valid": True,
        "mode_activation": {
            "activation": "explicit_global_review_mode",
            "explicit_only": True,
            "ordinary_review_scope": "local_engineering_review",
            "source_policy": (
                "Only explicit global-review / review-mode requests activate this receipt; "
                "ordinary code review remains local engineering review."
            ),
        },
        "scope": {
            "workspace_root": str(repo_root.resolve()),
            "source": source,
            "path_count": len(findings),
            "git_status_available": git_available,
            "git_status_exit_code": git_exit_code,
        },
        "dimensions": dimensions,
        "path_findings": [
            {
                "path": finding.path,
                "exists": finding.exists,
                "sha256": finding.sha256,
                "tags": finding.tags,
                "risk_flags": finding.risk_flags,
            }
            for finding in findings
        ],
        "decision": (
            "ready_for_global_review"
            if findings else "ready_for_global_review_no_paths"
        ),
        "safety": {
            "model_invoked": False,
            "network_attempted": False,
            "files_modified_by_review": False,
            "ordinary_review_replaced": False,
        },
        "limitations": [
            "This receipt activates and scopes global review obligations; it is not itself the final human or model review conclusion.",
            "Path tags are deterministic routing hints and may over-report risk; reviewers must inspect source documents and code before making claim-bearing judgments.",
            "The mode is explicit-only, project-generic, and must not expand ordinary local code review by default.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="Global review mode receipt")
    return receipt
