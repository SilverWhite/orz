"""Plan mode and process usage monitor — GAK-PLAN-001.

Design reference: :file:`architecture/PLAN_MODE_AND_PROCESS_USAGE_MONITOR_v0.1.md`.

Provides two stable CLI capabilities:

1. **Plan mode** — a pre-execution planning phase that produces a structured,
   approvable, replayable plan artifact before any side-effecting actions.
   Plan approval is separated from individual action approval.

2. **Process usage monitor** — lightweight, continuous sampling of the current
   run's root PID and its child process tree (CPU, memory, elapsed time).
   Designed for anomaly awareness, not performance profiling.

Evidence categories (aligned with :mod:`~.audit`):
  - Plan artifact proves a plan was formed and approved/rejected.
  - Usage sample proves resource occupancy at a point in time.
  - Neither proves task correctness, model reliability, or claim truth.

Integration points:
  - :class:`PlanStateMachine` plugs into :mod:`~.canonical_cli` before gate chain.
  - :class:`ProcessUsageSampler` runs as a background thread during live runs.
  - Sampler output formats for system terminal and VS Code terminal title.
  - Plan artifacts and usage telemetry feed the run journal via
    :mod:`~.canonical_cli` event stream.
"""

from __future__ import annotations

import os
import re
import threading
import time as _time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Callable

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .keystore import InstallationKeyStore, MemoryInstallationKeyStore
from .utils import (
    atomic_write_json,
    canonical_bytes,
    load_json,
    sha256_bytes,
    utc_now,
)

# ── safe identifiers ─────────────────────────────────────────────────────────

_SAFE_ID: re.Pattern[str] = re.compile(r"^[A-Z][A-Z0-9._-]{2,127}$")
_SAFE_PATH: re.Pattern[str] = re.compile(r"^[a-zA-Z0-9._/-]{1,512}$")

# ── schema names ─────────────────────────────────────────────────────────────

PLAN_ARTIFACT_SCHEMA = "plan-mode-result-v0.1.schema.json"

# ── output paths ─────────────────────────────────────────────────────────────

PLAN_ROOT: Path = ASSURANCE_ROOT.parent / ".gsa_plans"


# ══════════════════════════════════════════════════════════════════════════════
# Planning & Approval policy enums
# ══════════════════════════════════════════════════════════════════════════════


class PlanningPolicy(str, Enum):
    """Controls whether a planning phase is required before execution.

    Aligned with :file:`architecture/PLAN_MODE_AND_PROCESS_USAGE_MONITOR_v0.1.md` §3.4.
    """

    NONE = "none"          # execute directly, no plan needed
    SUGGESTED = "suggested"  # high-complexity tasks suggest a plan
    REQUIRED = "required"    # must plan before any side-effecting action


class ApprovalPolicy(str, Enum):
    """Controls how actions are approved after a plan is accepted.

    Separate from :class:`PlanningPolicy` — plan approval only authorises
    *entering the execution phase*, not every future action within it.
    """

    MANUAL = "manual"  # every sensitive action requires human confirmation
    AUTO = "auto"      # low-risk actions auto-approved within the plan scope
    MIXED = "mixed"    # classifier decides per action; novel/high-risk → manual


class PlanApprovalDecision(str, Enum):
    """Outcome of a plan approval request."""

    APPROVE = "approve"  # enter execution phase under specified approval policy
    REVISE = "revise"    # continue planning, keep plan version history
    REJECT = "reject"    # stop task or return to ordinary conversation


class PlanState(str, Enum):
    """States of the planning state machine (§3.3)."""

    IDLE = "idle"
    PLANNING = "planning"
    AWAITING_PLAN_APPROVAL = "awaiting_plan_approval"
    EXECUTING = "executing"
    REVISING = "revising"
    REJECTED = "rejected"
    COMPLETED = "completed"
    FAILED = "failed"


# ══════════════════════════════════════════════════════════════════════════════
# Plan artifact
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class PlanSection:
    """One of the four required plan sections.

    Each section has a fixed semantic role (§3.2) but flexible internal format.
    """

    title: str                          # "前期调查" | "具体计划" | "具体设计" | "实施方案"
    content_md: str                     # free-form markdown; no fixed bullet count
    evidence_status: str = "observed"   # "observed" | "inferred" | "unverified"
    source_refs: list[str] = field(default_factory=list)


@dataclass
class PlanArtifact:
    """A structured plan with the four required sections (§3.1).

    Fields match the schema in :file:`assurance/plan-mode-result-v0.1.schema.json`.
    """

    plan_id: str
    task_id: str
    run_id: str
    workspace_root: str
    created_at: str                          # ISO-8601 UTC
    planning_policy: str                     # PlanningPolicy value
    sections: list[PlanSection]              # exactly 4 sections expected
    version: int = 1
    previous_plan_sha256: str = ""           # empty for v1
    deferred_decisions: list[str] = field(default_factory=list)
    # ── approval fields (populated after plan review) ──
    approval_decision: str = ""              # PlanApprovalDecision value
    approval_timestamp: str = ""
    approval_authority: str = ""             # "user" | "policy:auto" | ...
    selected_approval_policy: str = ""       # ApprovalPolicy for execution phase
    # ── internal ──
    _sha256: str = field(default="", repr=False)

    def compute_sha256(self) -> str:
        """Return deterministic SHA-256 of the plan contents (excluding hash)."""
        payload = {
            "plan_id": self.plan_id,
            "task_id": self.task_id,
            "run_id": self.run_id,
            "workspace_root": self.workspace_root,
            "created_at": self.created_at,
            "planning_policy": self.planning_policy,
            "version": self.version,
            "previous_plan_sha256": self.previous_plan_sha256,
            "sections": [
                {
                    "title": s.title,
                    "content_md": s.content_md,
                    "evidence_status": s.evidence_status,
                    "source_refs": sorted(s.source_refs),
                }
                for s in self.sections
            ],
            "deferred_decisions": sorted(self.deferred_decisions),
            "approval_decision": self.approval_decision,
            "approval_timestamp": self.approval_timestamp,
            "approval_authority": self.approval_authority,
            "selected_approval_policy": self.selected_approval_policy,
        }
        return sha256_bytes(canonical_bytes(payload))

    def to_dict(self) -> dict[str, Any]:
        """Serialize to a JSON-compatible dict matching the schema."""
        h = self.compute_sha256()
        return {
            "schema_version": "0.1.0-draft",
            "plan_id": self.plan_id,
            "task_id": self.task_id,
            "run_id": self.run_id,
            "workspace_root": self.workspace_root,
            "created_at": self.created_at,
            "planning_policy": self.planning_policy,
            "version": self.version,
            "previous_plan_sha256": self.previous_plan_sha256,
            "sections": [
                {
                    "title": s.title,
                    "content_md": s.content_md,
                    "evidence_status": s.evidence_status,
                    "source_refs": sorted(s.source_refs),
                }
                for s in self.sections
            ],
            "deferred_decisions": sorted(self.deferred_decisions),
            "approval_decision": self.approval_decision,
            "approval_timestamp": self.approval_timestamp,
            "approval_authority": self.approval_authority,
            "selected_approval_policy": self.selected_approval_policy,
            "plan_sha256": h,
        }

    def to_markdown(self) -> str:
        """Render the plan as a human-readable markdown document."""
        lines: list[str] = [
            f"# Plan: {self.plan_id}",
            "",
            f"- **Task**: {self.task_id}",
            f"- **Run**: {self.run_id}",
            f"- **Workspace**: {self.workspace_root}",
            f"- **Created**: {self.created_at}",
            f"- **Planning Policy**: {self.planning_policy}",
            f"- **Version**: {self.version}",
        ]
        if self.previous_plan_sha256:
            lines.append(f"- **Previous Plan**: {self.previous_plan_sha256}")
        if self.deferred_decisions:
            lines.append("- **Deferred Decisions**:")
            for d in self.deferred_decisions:
                lines.append(f"  - {d}")
        if self.approval_decision:
            lines.extend([
                "",
                "## Approval",
                "",
                f"- **Decision**: {self.approval_decision}",
                f"- **Authority**: {self.approval_authority}",
                f"- **Timestamp**: {self.approval_timestamp}",
                f"- **Execution Approval Policy**: {self.selected_approval_policy}",
            ])
        for s in self.sections:
            lines.extend([
                "",
                f"## {s.title}",
                "",
                f"*Evidence: {s.evidence_status}*",
                "",
                s.content_md,
            ])
        if self._sha256:
            lines.extend(["", "---", "", f"`sha256:{self._sha256}`"])
        return "\n".join(lines)


# ── plan construction ────────────────────────────────────────────────────────

_REQUIRED_SECTIONS = ["前期调查", "具体计划", "具体设计", "实施方案"]


def build_plan_artifact(
    *,
    task_id: str,
    run_id: str,
    workspace_root: str | Path,
    sections: list[PlanSection],
    planning_policy: str | PlanningPolicy = "required",
    version: int = 1,
    previous_plan_sha256: str = "",
    deferred_decisions: list[str] | None = None,
) -> PlanArtifact:
    """Create a new plan artifact with the four required sections.

    Parameters
    ----------
    task_id:
        Task identifier (e.g. ``TASK-...``).
    run_id:
        Run identifier (e.g. ``RUN-...``).
    workspace_root:
        Canonical workspace path.
    sections:
        Exactly 4 :class:`PlanSection` entries with the required titles.
    planning_policy:
        Policy that triggered this plan.
    version:
        Plan version number (≥1).
    previous_plan_sha256:
        SHA-256 of the previous plan version (empty for v1).
    deferred_decisions:
        Decisions explicitly deferred to a later phase.

    Returns
    -------
    :
        A populated :class:`PlanArtifact` ready for validation and storage.

    Raises
    ------
    AssuranceError
        If the sections do not include all four required titles.
    """
    if isinstance(planning_policy, PlanningPolicy):
        planning_policy = planning_policy.value
    plan_id = f"PLAN-{uuid.uuid4().hex[:12].upper()}"
    present = {s.title for s in sections}
    missing = [t for t in _REQUIRED_SECTIONS if t not in present]
    if missing:
        raise AssuranceError(
            f"Plan {plan_id} missing required sections: {', '.join(missing)}"
        )
    if len(sections) != 4:
        raise AssuranceError(
            f"Plan {plan_id} has {len(sections)} sections; exactly 4 required"
        )
    artifact = PlanArtifact(
        plan_id=plan_id,
        task_id=task_id,
        run_id=run_id,
        workspace_root=str(workspace_root),
        created_at=utc_now(),
        planning_policy=planning_policy,
        sections=list(sections),
        version=version,
        previous_plan_sha256=previous_plan_sha256,
        deferred_decisions=sorted(deferred_decisions or []),
    )
    artifact._sha256 = artifact.compute_sha256()
    return artifact


def write_plan_artifact(artifact: PlanArtifact, *, store_root: Path | None = None) -> Path:
    """Persist a plan artifact as JSON and Markdown.

    Parameters
    ----------
    artifact:
        The plan to write.
    store_root:
        Root directory for plan storage (defaults to ``.gsa_plans/``).

    Returns
    -------
    :
        Path to the written JSON file.
    """
    root = store_root or PLAN_ROOT
    plan_dir = root / artifact.plan_id
    plan_dir.mkdir(parents=True, exist_ok=True)
    json_path = plan_dir / "plan-mode-result.json"
    md_path = plan_dir / "plan-mode-result.md"
    data = artifact.to_dict()
    validate_contract(data, PLAN_ARTIFACT_SCHEMA, label=f"plan:{artifact.plan_id}")
    atomic_write_json(json_path, data, overwrite=True)
    md_path.write_text(artifact.to_markdown(), encoding="utf-8")
    return json_path


def load_plan_artifact(plan_dir: Path) -> PlanArtifact:
    """Read a plan artifact from disk.

    Parameters
    ----------
    plan_dir:
        Directory containing ``plan-mode-result.json``.

    Returns
    -------
    :
        A reconstructed :class:`PlanArtifact`.
    """
    data = load_json(plan_dir / "plan-mode-result.json")
    validate_contract(data, PLAN_ARTIFACT_SCHEMA, label=f"plan:{data.get('plan_id', '?')}")
    sections = [
        PlanSection(
            title=s["title"],
            content_md=s["content_md"],
            evidence_status=s.get("evidence_status", "observed"),
            source_refs=s.get("source_refs", []),
        )
        for s in data["sections"]
    ]
    artifact = PlanArtifact(
        plan_id=data["plan_id"],
        task_id=data["task_id"],
        run_id=data["run_id"],
        workspace_root=data["workspace_root"],
        created_at=data["created_at"],
        planning_policy=data["planning_policy"],
        sections=sections,
        version=data.get("version", 1),
        previous_plan_sha256=data.get("previous_plan_sha256", ""),
        deferred_decisions=data.get("deferred_decisions", []),
        approval_decision=data.get("approval_decision", ""),
        approval_timestamp=data.get("approval_timestamp", ""),
        approval_authority=data.get("approval_authority", ""),
        selected_approval_policy=data.get("selected_approval_policy", ""),
    )
    artifact._sha256 = data.get("plan_sha256", "")
    return artifact


# ══════════════════════════════════════════════════════════════════════════════
# Plan verifier
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class PlanVerificationResult:
    """Output of :func:`verify_plan_artifact`."""

    valid: bool
    plan_id: str
    checks: list[dict[str, Any]]   # {check, passed, detail}
    errors: list[str]


def verify_plan_artifact(
    artifact: PlanArtifact,
    *,
    require_approval: bool = False,
) -> PlanVerificationResult:
    """Verify the structural integrity of a plan artifact.

    Checks performed (§3.5):

    1. All four required sections present with non-empty content
    2. Section titles match the canonical set
    3. Version ≥ 1 and previous_plan_sha256 consistency
    4. Plan SHA-256 matches recomputed value
    5. (Optional) Approval chain: decision, authority, and timestamp present

    This verifier checks **structure, version, and approval chain**, not plan
    quality.  Plan quality is determined by the user, subsequent execution,
    and test outcomes.

    Parameters
    ----------
    artifact:
        The plan to verify.
    require_approval:
        If ``True``, require a non-empty ``approval_decision``.

    Returns
    -------
    :
        A structured verification result.
    """
    checks: list[dict[str, Any]] = []
    errors: list[str] = []

    # 1. Section presence
    titles = {s.title for s in artifact.sections}
    for required in _REQUIRED_SECTIONS:
        ok = required in titles
        checks.append({
            "check": f"section_present:{required}",
            "passed": ok,
            "detail": "found" if ok else "missing",
        })
        if not ok:
            errors.append(f"Missing required section: {required}")

    # 2. Section count
    count_ok = len(artifact.sections) == 4
    checks.append({
        "check": "section_count",
        "passed": count_ok,
        "detail": f"{len(artifact.sections)} sections (expected 4)",
    })
    if not count_ok:
        errors.append(f"Expected 4 sections, got {len(artifact.sections)}")

    # 3. Non-empty content
    for s in artifact.sections:
        content_ok = bool(s.content_md.strip())
        checks.append({
            "check": f"content_nonempty:{s.title}",
            "passed": content_ok,
            "detail": f"{len(s.content_md)} chars",
        })
        if not content_ok:
            errors.append(f"Section '{s.title}' has empty content")

    # 4. Version
    version_ok = artifact.version >= 1
    checks.append({
        "check": "version_positive",
        "passed": version_ok,
        "detail": f"version={artifact.version}",
    })
    if not version_ok:
        errors.append(f"Version must be ≥1, got {artifact.version}")

    # 5. Previous plan hash consistency
    if artifact.version == 1:
        hash_ok = artifact.previous_plan_sha256 == ""
        checks.append({
            "check": "previous_plan_sha256_v1_empty",
            "passed": hash_ok,
            "detail": (
                "empty" if hash_ok
                else f"non-empty: {artifact.previous_plan_sha256[:16]}..."
            ),
        })
        if not hash_ok:
            errors.append("v1 plan must have empty previous_plan_sha256")
    else:
        hash_ok = bool(artifact.previous_plan_sha256) and len(artifact.previous_plan_sha256) == 64
        checks.append({
            "check": "previous_plan_sha256_present",
            "passed": hash_ok,
            "detail": (
                f"{artifact.previous_plan_sha256[:16]}..."
                if hash_ok else "missing or malformed"
            ),
        })
        if not hash_ok:
            errors.append(f"v{artifact.version} plan requires previous_plan_sha256")

    # 6. SHA-256 integrity
    expected = artifact.compute_sha256()
    integrity_ok = expected == artifact._sha256 if artifact._sha256 else True
    checks.append({
        "check": "sha256_integrity",
        "passed": integrity_ok,
        "detail": f"expected={expected[:16]}..." if not integrity_ok else "matches",
    })
    if not integrity_ok:
        errors.append(
            f"Plan SHA-256 mismatch: stored={artifact._sha256[:16]}... "
            f"computed={expected[:16]}..."
        )

    # 7. Approval chain (optional)
    if require_approval:
        approval_ok = artifact.approval_decision in {"approve", "revise", "reject"}
        checks.append({
            "check": "approval_decision_valid",
            "passed": approval_ok,
            "detail": artifact.approval_decision or "empty",
        })
        if not approval_ok:
            errors.append(
                f"Invalid or missing approval_decision: {artifact.approval_decision!r}"
            )
        ts_ok = bool(artifact.approval_timestamp)
        checks.append({
            "check": "approval_timestamp_present",
            "passed": ts_ok,
            "detail": artifact.approval_timestamp or "empty",
        })
        if not ts_ok:
            errors.append("Approval timestamp is missing")

    return PlanVerificationResult(
        valid=len(errors) == 0,
        plan_id=artifact.plan_id,
        checks=checks,
        errors=errors,
    )


# ══════════════════════════════════════════════════════════════════════════════
# Plan state machine
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class PlanStateMachine:
    """Tracks the planning lifecycle for a single run (§3.3).

    Valid transitions::

        IDLE → PLANNING
        PLANNING → AWAITING_PLAN_APPROVAL | FAILED
        AWAITING_PLAN_APPROVAL → EXECUTING | REVISING | REJECTED
        REVISING → PLANNING
        EXECUTING → COMPLETED | FAILED
    """

    state: PlanState = PlanState.IDLE
    planning_policy: PlanningPolicy = PlanningPolicy.NONE
    approval_policy: ApprovalPolicy = ApprovalPolicy.MANUAL
    plan_versions: list[str] = field(default_factory=list)  # SHA-256 chain
    current_plan: PlanArtifact | None = None
    approval_history: list[dict[str, Any]] = field(default_factory=list)
    _transcript: list[dict[str, Any]] = field(default_factory=list)

    def enter_planning(self, *, policy: PlanningPolicy | None = None) -> None:
        """Transition to PLANNING state.

        During planning the model may read workspace files, query local state,
        perform read-only analysis, generate/update the plan artifact, and ask
        the user for clarification (§3.3).
        """
        if policy is not None:
            self.planning_policy = policy
        if self.state not in {PlanState.IDLE, PlanState.REVISING}:
            raise AssuranceError(
                f"Cannot enter planning from state {self.state.value}"
            )
        self.state = PlanState.PLANNING
        self._record("enter_planning", {"policy": self.planning_policy.value})

    def submit_plan(self, artifact: PlanArtifact) -> None:
        """Submit a completed plan for approval."""
        if self.state != PlanState.PLANNING:
            raise AssuranceError(
                f"Cannot submit plan from state {self.state.value}"
            )
        self.current_plan = artifact
        self.plan_versions.append(artifact._sha256 or artifact.compute_sha256())
        self.state = PlanState.AWAITING_PLAN_APPROVAL
        self._record("submit_plan", {"plan_id": artifact.plan_id})

    def approve(
        self,
        *,
        authority: str = "user",
        execution_policy: ApprovalPolicy = ApprovalPolicy.MANUAL,
    ) -> PlanApprovalRecord:
        """Approve the plan and transition to EXECUTING."""
        if self.state != PlanState.AWAITING_PLAN_APPROVAL:
            raise AssuranceError(
                f"Cannot approve plan from state {self.state.value}"
            )
        if self.current_plan is None:
            raise AssuranceError("No plan to approve")
        record = PlanApprovalRecord(
            plan_id=self.current_plan.plan_id,
            decision=PlanApprovalDecision.APPROVE,
            authority=authority,
            execution_policy=execution_policy,
            timestamp=utc_now(),
        )
        self.current_plan.approval_decision = "approve"
        self.current_plan.approval_timestamp = record.timestamp
        self.current_plan.approval_authority = authority
        self.current_plan.selected_approval_policy = execution_policy.value
        self.approval_policy = execution_policy
        self.approval_history.append(record.to_dict())
        self.state = PlanState.EXECUTING
        self._record("approve", record.to_dict())
        return record

    def revise(self, *, authority: str = "user") -> PlanApprovalRecord:
        """Request plan revision, returning to PLANNING."""
        if self.state != PlanState.AWAITING_PLAN_APPROVAL:
            raise AssuranceError(
                f"Cannot revise plan from state {self.state.value}"
            )
        if self.current_plan is None:
            raise AssuranceError("No plan to revise")
        record = PlanApprovalRecord(
            plan_id=self.current_plan.plan_id,
            decision=PlanApprovalDecision.REVISE,
            authority=authority,
            timestamp=utc_now(),
        )
        self.current_plan.approval_decision = "revise"
        self.current_plan.approval_timestamp = record.timestamp
        self.current_plan.approval_authority = authority
        self.approval_history.append(record.to_dict())
        self.state = PlanState.REVISING
        self._record("revise", record.to_dict())
        self.state = PlanState.PLANNING  # immediate transition per §3.3
        self._record("reenter_planning", {})
        return record

    def reject(self, *, authority: str = "user") -> PlanApprovalRecord:
        """Reject the plan and stop the task."""
        if self.state != PlanState.AWAITING_PLAN_APPROVAL:
            raise AssuranceError(
                f"Cannot reject plan from state {self.state.value}"
            )
        if self.current_plan is None:
            raise AssuranceError("No plan to reject")
        record = PlanApprovalRecord(
            plan_id=self.current_plan.plan_id,
            decision=PlanApprovalDecision.REJECT,
            authority=authority,
            timestamp=utc_now(),
        )
        self.current_plan.approval_decision = "reject"
        self.current_plan.approval_timestamp = record.timestamp
        self.current_plan.approval_authority = authority
        self.approval_history.append(record.to_dict())
        self.state = PlanState.REJECTED
        self._record("reject", record.to_dict())
        return record

    def complete(self) -> None:
        """Mark execution as successfully completed."""
        if self.state != PlanState.EXECUTING:
            raise AssuranceError(
                f"Cannot complete from state {self.state.value}"
            )
        self.state = PlanState.COMPLETED
        self._record("complete", {})

    def fail(self, reason: str = "") -> None:
        """Mark execution as failed."""
        self.state = PlanState.FAILED
        self._record("fail", {"reason": reason})

    @property
    def is_planning(self) -> bool:
        return self.state == PlanState.PLANNING

    @property
    def is_awaiting_approval(self) -> bool:
        return self.state == PlanState.AWAITING_PLAN_APPROVAL

    @property
    def is_executing(self) -> bool:
        return self.state == PlanState.EXECUTING

    def to_dict(self) -> dict[str, Any]:
        return {
            "state": self.state.value,
            "planning_policy": self.planning_policy.value,
            "approval_policy": self.approval_policy.value,
            "plan_versions": list(self.plan_versions),
            "current_plan_id": self.current_plan.plan_id if self.current_plan else None,
            "approval_count": len(self.approval_history),
        }

    def _record(self, event: str, detail: dict[str, Any]) -> None:
        self._transcript.append({
            "event": event,
            "timestamp": utc_now(),
            "from_state": self.state.value if event not in {"enter_planning"} else "idle",
            **detail,
        })


# ══════════════════════════════════════════════════════════════════════════════
# Plan approval record
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class PlanApprovalRecord:
    """A single plan approval decision (§3.3-3.4).

    This is separate from individual action approval — approving a plan
    only authorises *entering the execution phase*, not every future action
    within it.
    """

    plan_id: str
    decision: PlanApprovalDecision
    authority: str                       # "user" | "policy:auto" | ...
    execution_policy: ApprovalPolicy = ApprovalPolicy.MANUAL
    timestamp: str = field(default_factory=utc_now)

    def to_dict(self) -> dict[str, Any]:
        return {
            "plan_id": self.plan_id,
            "decision": self.decision.value,
            "authority": self.authority,
            "execution_policy": self.execution_policy.value,
            "timestamp": self.timestamp,
        }


# ══════════════════════════════════════════════════════════════════════════════
# Process usage sampler
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class UsageSample:
    """A single resource-usage snapshot of the run's process tree (§4.2)."""

    timestamp: str
    root_pid: int
    cpu_percent: float
    memory_bytes: int
    elapsed_seconds: float
    process_count: int
    completeness: str = "complete"  # "complete" | "partial" | "unavailable"
    reason_unavailable: str = ""
    # optional extras
    private_bytes: int = 0
    io_bytes: int = 0
    handle_count: int = 0
    thread_count: int = 0

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "timestamp": self.timestamp,
            "root_pid": self.root_pid,
            "cpu_percent": round(self.cpu_percent, 1),
            "memory_bytes": self.memory_bytes,
            "elapsed_seconds": round(self.elapsed_seconds, 3),
            "process_count": self.process_count,
            "completeness": self.completeness,
        }
        if self.reason_unavailable:
            d["reason_unavailable"] = self.reason_unavailable
        if self.private_bytes:
            d["private_bytes"] = self.private_bytes
        return d

    def format_status_line(self) -> str:
        """Format as a single-line status string for system terminal display.

        Example: ``CPU 185% MEM 2.1G 14m 12p``
        """
        cpu = f"CPU {self.cpu_percent:.0f}%"
        mem = f"MEM {_format_memory(self.memory_bytes)}"
        elapsed = _format_elapsed(self.elapsed_seconds)
        return f"{cpu} {mem} {elapsed} {self.process_count}p"

    def format_anomaly_line(self) -> str:
        """Format with anomaly prefix if thresholds are exceeded (§4.6).

        Returns empty string when no anomaly is detected.
        """
        prefix = ""
        if self.cpu_percent > 300:
            prefix = "HIGH CPU "
        elif self.memory_bytes > 4 * 1024**3:
            prefix = "HIGH MEM "
        elif self.cpu_percent < 1.0 and self.elapsed_seconds > 30:
            prefix = "IDLE? "
        if prefix:
            return prefix + self.format_status_line()
        return ""


class ProcessUsageSampler:
    """Lightweight background sampler for the current run's process tree (§4).

    Uses ``psutil`` to sample root PID + child tree CPU, memory, and elapsed
    time.  Designed to be near-zero overhead in normal operation (§4.5).

    Parameters
    ----------
    root_pid:
        The root process ID to monitor (typically the launcher/shell PID).
    interval:
        Sample interval in seconds (default 1.0 for system terminal).
    max_samples:
        Maximum samples to retain in-memory (bounded queue).

    Usage::

        sampler = ProcessUsageSampler(root_pid=os.getpid())
        sampler.start()
        # ... run the task ...
        samples = sampler.stop()
    """

    def __init__(
        self,
        root_pid: int,
        *,
        interval: float = 1.0,
        max_samples: int = 3600,  # 1 hour at 1s
    ) -> None:
        self.root_pid = root_pid
        self.interval = interval
        self.max_samples = max_samples
        self._samples: list[UsageSample] = []
        self._lock = threading.Lock()
        self._thread: threading.Thread | None = None
        self._stop_event = threading.Event()
        self._start_time: float = 0.0
        self._running = False

    @property
    def running(self) -> bool:
        return self._running

    @property
    def samples(self) -> list[UsageSample]:
        with self._lock:
            return list(self._samples)

    @property
    def latest(self) -> UsageSample | None:
        with self._lock:
            return self._samples[-1] if self._samples else None

    def start(self) -> None:
        """Begin background sampling.  Idempotent if already running."""
        if self._running:
            return
        self._stop_event.clear()
        self._start_time = _time.monotonic()
        self._running = True
        self._thread = threading.Thread(
            target=self._sample_loop,
            name="gsa-usage-sampler",
            daemon=True,
        )
        self._thread.start()

    def stop(self) -> list[UsageSample]:
        """Stop sampling and return all collected samples."""
        self._stop_event.set()
        if self._thread is not None:
            self._thread.join(timeout=5.0)
        self._running = False
        return self.samples

    def sample_once(self) -> UsageSample:
        """Take a single synchronous sample (useful for testing)."""
        return self._collect_sample()

    def _sample_loop(self) -> None:
        backoff = self.interval
        idle_count = 0
        while not self._stop_event.is_set():
            try:
                sample = self._collect_sample()
                with self._lock:
                    self._samples.append(sample)
                    if len(self._samples) > self.max_samples:
                        self._samples = self._samples[-self.max_samples:]
                # Adaptive backoff: 2s when idle for >10 consecutive samples
                if sample.cpu_percent < 1.0:
                    idle_count += 1
                    backoff = 2.0 if idle_count > 10 else self.interval
                else:
                    idle_count = 0
                    backoff = self.interval
            except Exception:
                # Monitor failure must not interrupt the main task (§4.5)
                with self._lock:
                    self._samples.append(UsageSample(
                        timestamp=utc_now(),
                        root_pid=self.root_pid,
                        cpu_percent=0.0,
                        memory_bytes=0,
                        elapsed_seconds=_time.monotonic() - self._start_time,
                        process_count=0,
                        completeness="unavailable",
                        reason_unavailable="sampler_error",
                    ))
            self._stop_event.wait(timeout=backoff)

    def _collect_sample(self) -> UsageSample:
        """Collect a single sample using psutil."""
        try:
            import psutil
        except ImportError:
            return UsageSample(
                timestamp=utc_now(),
                root_pid=self.root_pid,
                cpu_percent=0.0,
                memory_bytes=0,
                elapsed_seconds=_time.monotonic() - self._start_time,
                process_count=0,
                completeness="unavailable",
                reason_unavailable="psutil_not_installed",
            )
        try:
            root = psutil.Process(self.root_pid)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            return UsageSample(
                timestamp=utc_now(),
                root_pid=self.root_pid,
                cpu_percent=0.0,
                memory_bytes=0,
                elapsed_seconds=_time.monotonic() - self._start_time,
                process_count=0,
                completeness="partial",
                reason_unavailable="root_process_gone",
            )
        try:
            children = root.children(recursive=True)
            all_procs = [root] + children
            total_cpu = sum(p.cpu_percent() for p in all_procs)
            mem_info = root.memory_info()
            total_mem = mem_info.rss
            for c in children:
                try:
                    total_mem += c.memory_info().rss
                except (psutil.NoSuchProcess, psutil.AccessDenied):
                    pass
            elapsed = _time.monotonic() - self._start_time
            # Optional extras
            private_bytes = 0
            try:
                pmem = root.memory_full_info()
                private_bytes = getattr(pmem, "private", 0) or getattr(pmem, "uss", 0)
            except (psutil.NoSuchProcess, psutil.AccessDenied, AttributeError):
                pass
            return UsageSample(
                timestamp=utc_now(),
                root_pid=self.root_pid,
                cpu_percent=total_cpu,
                memory_bytes=total_mem,
                elapsed_seconds=elapsed,
                process_count=len(all_procs),
                completeness="complete",
                private_bytes=private_bytes,
                thread_count=root.num_threads() if hasattr(root, 'num_threads') else 0,
            )
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            return UsageSample(
                timestamp=utc_now(),
                root_pid=self.root_pid,
                cpu_percent=0.0,
                memory_bytes=0,
                elapsed_seconds=_time.monotonic() - self._start_time,
                process_count=0,
                completeness="partial",
                reason_unavailable="child_enumeration_failed",
            )


# ══════════════════════════════════════════════════════════════════════════════
# VS Code terminal title adapter
# ══════════════════════════════════════════════════════════════════════════════


def format_vscode_title(sample: UsageSample) -> str:
    """Format a usage sample for VS Code terminal title (§4.3).

    Default format: ``CPU 185% MEM 2.1G 14m``

    With anomaly prefix (empty string if no anomaly)::

        HIGH CPU 420% MEM 3.8G 22m
        HIGH MEM 7.6G 31m
        IDLE? CPU 0% MEM 2.2G 18m

    Parameters
    ----------
    sample:
        A usage sample from :class:`ProcessUsageSampler`.

    Returns
    -------
    :
        A short string suitable for a terminal title (≤ 60 chars).
    """
    anomaly = sample.format_anomaly_line()
    if anomaly:
        return anomaly
    cpu = f"CPU {sample.cpu_percent:.0f}%"
    mem = f"MEM {_format_memory(sample.memory_bytes)}"
    elapsed = _format_elapsed(sample.elapsed_seconds)
    return f"{cpu} {mem} {elapsed}"


def set_vscode_terminal_title(title: str) -> bool:
    """Attempt to set the VS Code terminal title.

    Uses the OSC 0 escape sequence.  Falls back gracefully — if the
    terminal doesn't support it or writing fails, returns ``False``.

    Parameters
    ----------
    title:
        The title string to set.

    Returns
    -------
    :
        ``True`` if the escape sequence was written, ``False`` otherwise.
    """
    try:
        # OSC 0 ; <title> ST
        print(f"\x1b]0;{title}\x07", end="", flush=True)
        return True
    except (OSError, BlockingIOError):
        return False


class VSCodeTitleUpdater:
    """Background thread that periodically updates the VS Code terminal title.

    Consumes samples from a :class:`ProcessUsageSampler` and writes the
    formatted title at the configured interval (§4.3: default 2s).

    Parameters
    ----------
    sampler:
        The sampler to read latest samples from.
    interval:
        Title update interval in seconds (default 2.0).
    """

    def __init__(
        self,
        sampler: ProcessUsageSampler,
        *,
        interval: float = 2.0,
    ) -> None:
        self.sampler = sampler
        self.interval = interval
        self._thread: threading.Thread | None = None
        self._stop_event = threading.Event()
        self._running = False

    @property
    def running(self) -> bool:
        return self._running

    def start(self) -> None:
        """Begin periodic title updates."""
        if self._running:
            return
        self._stop_event.clear()
        self._running = True
        self._thread = threading.Thread(
            target=self._update_loop,
            name="gsa-vscode-title",
            daemon=True,
        )
        self._thread.start()

    def stop(self) -> None:
        """Stop title updates."""
        self._stop_event.set()
        if self._thread is not None:
            self._thread.join(timeout=3.0)
        self._running = False

    def _update_loop(self) -> None:
        while not self._stop_event.is_set():
            sample = self.sampler.latest
            if sample is not None:
                title = format_vscode_title(sample)
                set_vscode_terminal_title(title)
            self._stop_event.wait(timeout=self.interval)


# ══════════════════════════════════════════════════════════════════════════════
# Helpers
# ══════════════════════════════════════════════════════════════════════════════


def _format_memory(bytes_: int) -> str:
    """Format bytes as human-readable memory string."""
    if bytes_ >= 1024**3:
        return f"{bytes_ / 1024**3:.1f}G"
    elif bytes_ >= 1024**2:
        return f"{bytes_ / 1024**2:.0f}M"
    elif bytes_ >= 1024:
        return f"{bytes_ / 1024:.0f}K"
    return f"{bytes_}B"


def _format_elapsed(seconds: float) -> str:
    """Format elapsed seconds as a short time string."""
    if seconds < 60:
        return f"{seconds:.0f}s"
    elif seconds < 3600:
        m = int(seconds // 60)
        s = int(seconds % 60)
        return f"{m}m{s:02d}s"
    else:
        h = int(seconds // 3600)
        m = int((seconds % 3600) // 60)
        return f"{h}h{m:02d}m"


def _resolve_planning_policy(
    *,
    explicit: PlanningPolicy | str | None,
    task_complexity: str = "normal",
) -> PlanningPolicy:
    """Resolve the effective planning policy from explicit and heuristic inputs.

    Parameters
    ----------
    explicit:
        User-requested policy override.
    task_complexity:
        Heuristic complexity label: "trivial", "normal", "complex".

    Returns
    -------
    :
        The resolved :class:`PlanningPolicy`.
    """
    if explicit is not None:
        if isinstance(explicit, str):
            return PlanningPolicy(explicit)
        return explicit
    if task_complexity == "complex":
        return PlanningPolicy.SUGGESTED
    return PlanningPolicy.NONE
