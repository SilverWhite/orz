"""Task Checklist — soft workboard derived from an approved plan.

Design reference: :file:`architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md`.

The task checklist is a user-facing and model-facing soft workboard.  It is
**not** a hard constraint engine.  Plan approval produces checklist items plus
plan annotations; the checklist helps the AI orient itself without overriding
system policy, user instructions, permissions, evidence gates, or
source-visibility rules.

Status vocabulary (§5):
  todo | doing | done | blocked | deferred | replanned

Step IDs (§4):  ``<task-part-code>-<two-digit-sequence>``, e.g. ``GAK-01``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

# ── status vocabulary ───────────────────────────────────────────────────────────


class ChecklistStatus(str, Enum):
    """Six-value status vocabulary (§5).

    The checklist state is a work coordination signal.  It is **not** evidence
    of final correctness, verification, or claim strength.
    """

    TODO = "todo"
    DOING = "doing"
    DONE = "done"
    BLOCKED = "blocked"
    DEFERRED = "deferred"
    REPLANNED = "replanned"


# Display symbols for the compact L1 bar.
STATUS_DISPLAY: dict[ChecklistStatus, str] = {
    ChecklistStatus.TODO: ".",
    ChecklistStatus.DOING: ">",
    ChecklistStatus.DONE: "done",
    ChecklistStatus.BLOCKED: "!",
    ChecklistStatus.DEFERRED: "~",
    ChecklistStatus.REPLANNED: "*",
}


# ── step-id pattern ─────────────────────────────────────────────────────────────

# Must match the spec: task-part-code (upper-case alphanumeric + dash/underscore)
# followed by a hyphen and a two-digit sequence.
_STEP_ID_RE: re.Pattern[str] = re.compile(
    r"^[A-Z][A-Z0-9_-]{1,31}-[0-9]{2}$"
)


def validate_step_id(step_id: str) -> bool:
    """Return ``True`` if *step_id* matches the required format."""
    return bool(_STEP_ID_RE.fullmatch(step_id))


def make_step_id(task_code: str, sequence: int) -> str:
    """Build a stable step id from a task-area code and a sequence number.

    >>> make_step_id("GAK", 1)
    'GAK-01'
    """
    return f"{task_code}-{sequence:02d}"


# ── data structures ─────────────────────────────────────────────────────────────


@dataclass
class ChecklistAnnotation:
    """Soft task memory attached to a checklist item (§6).

    Annotations help the AI orient itself.  They do **not** override system
    policy, user instructions, permissions, evidence gates, or source-visibility
    rules.
    """

    plan_revision: str = ""               # e.g. "v3"
    source_section: str = ""              # which plan section this came from
    acceptance_refs: list[str] = field(default_factory=list)
    soft_constraints: list[str] = field(default_factory=list)
    assumptions: list[str] = field(default_factory=list)
    deferred_decisions: list[str] = field(default_factory=list)
    runtime_records: list[str] = field(default_factory=list)
    next_review_trigger: str = ""


@dataclass
class ChecklistItem:
    """A single step in the task checklist.

    Stability rule: if a step is split or replaced, create a new plan revision
    rather than silently reusing the old step id for a different meaning.
    """

    step_id: str                          # e.g. "GAK-01"
    title: str
    status: ChecklistStatus = ChecklistStatus.TODO
    annotations: ChecklistAnnotation = field(default_factory=ChecklistAnnotation)
    created_at: str = ""
    updated_at: str = ""

    def to_dict(self) -> dict[str, Any]:
        """Serialize to a plain dict suitable for JSON/journal events."""
        return {
            "step_id": self.step_id,
            "title": self.title,
            "status": self.status.value if isinstance(self.status, ChecklistStatus) else self.status,
            "annotations": {
                "plan_revision": self.annotations.plan_revision,
                "source_section": self.annotations.source_section,
                "acceptance_refs": list(self.annotations.acceptance_refs),
                "soft_constraints": list(self.annotations.soft_constraints),
                "assumptions": list(self.annotations.assumptions),
                "deferred_decisions": list(self.annotations.deferred_decisions),
                "runtime_records": list(self.annotations.runtime_records),
                "next_review_trigger": self.annotations.next_review_trigger,
            },
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "ChecklistItem":
        """Restore from a plain dict."""
        ann = d.get("annotations", {})
        return cls(
            step_id=d.get("step_id", ""),
            title=d.get("title", ""),
            status=ChecklistStatus(d.get("status", "todo")),
            annotations=ChecklistAnnotation(
                plan_revision=ann.get("plan_revision", ""),
                source_section=ann.get("source_section", ""),
                acceptance_refs=list(ann.get("acceptance_refs", [])),
                soft_constraints=list(ann.get("soft_constraints", [])),
                assumptions=list(ann.get("assumptions", [])),
                deferred_decisions=list(ann.get("deferred_decisions", [])),
                runtime_records=list(ann.get("runtime_records", [])),
                next_review_trigger=ann.get("next_review_trigger", ""),
            ),
            created_at=d.get("created_at", ""),
            updated_at=d.get("updated_at", ""),
        )


@dataclass
class TaskChecklist:
    """The complete task checklist for an active run.

    Two synchronized views (§3):
      1. Compact announcement view (L1) — always visible near the top of the
         conversation window.
      2. Expanded checklist page (L2/L3) — opened by user action.
    """

    plan_id: str
    task_id: str
    run_id: str
    items: list[ChecklistItem] = field(default_factory=list)
    created_at: str = ""
    updated_at: str = ""

    @property
    def current_index(self) -> int:
        """Index of the 'doing' item, or first 'todo' after a 'done'.

        Falls back to the last item if nothing is in progress.
        """
        for i, item in enumerate(self.items):
            if item.status == ChecklistStatus.DOING:
                return i
        for i, item in enumerate(self.items):
            if item.status == ChecklistStatus.TODO:
                return i
        return max(0, len(self.items) - 1)

    @property
    def current_item(self) -> ChecklistItem | None:
        """The item that is currently in progress (or next todo)."""
        if not self.items:
            return None
        return self.items[self.current_index]

    def to_dict(self) -> dict[str, Any]:
        """Serialize to a plain dict suitable for JSON/journal events."""
        return {
            "plan_id": self.plan_id,
            "task_id": self.task_id,
            "run_id": self.run_id,
            "items": [item.to_dict() for item in self.items],
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "TaskChecklist":
        """Restore from a plain dict."""
        return cls(
            plan_id=d.get("plan_id", ""),
            task_id=d.get("task_id", ""),
            run_id=d.get("run_id", ""),
            items=[ChecklistItem.from_dict(i) for i in d.get("items", [])],
            created_at=d.get("created_at", ""),
            updated_at=d.get("updated_at", ""),
        )


# ── plan → checklist derivation ─────────────────────────────────────────────────


# Patterns used to detect step-like lines inside the "具体计划" section.
# Ordered from most structured to loosest.
_STEP_PATTERNS: list[re.Pattern[str]] = [
    # ## GAK-01 Some title
    re.compile(
        r"^#{1,4}\s+(?P<id>[A-Z][A-Z0-9_-]{0,31}-\d{2})\s*[：:]\s*(?P<title>.+)$",
        re.IGNORECASE,
    ),
    # ## GAK-01 Title (no colon)
    re.compile(
        r"^#{1,4}\s+(?P<id>[A-Z][A-Z0-9_-]{0,31}-\d{2})\s+(?P<title>.+)$",
        re.IGNORECASE,
    ),
    # - GAK-01: Title  or  * GAK-01: Title
    re.compile(
        r"^[-*]\s+(?P<id>[A-Z][A-Z0-9_-]{0,31}-\d{2})\s*[：:]\s*(?P<title>.+)$",
        re.IGNORECASE,
    ),
    # 1. GAK-01: Title  or  1) GAK-01: Title
    re.compile(
        r"^\d+[.)]\s+(?P<id>[A-Z][A-Z0-9_-]{0,31}-\d{2})\s*[：:]\s*(?P<title>.+)$",
        re.IGNORECASE,
    ),
    # 1. Some title (no step ID — we'll synthesize one)
    re.compile(r"^\d+[.)]\s+(?P<title>.+)$"),
    # - Some title (bullet, no step ID — synthesize)
    re.compile(r"^[-*]\s+(?P<title>.+)$"),
]

# Already-used step IDs to avoid duplicate synthetic IDs.
_SEEN_IDS: set[str] = set()


def _synthesize_step_id(
    prefix: str, sequence: int, title: str
) -> tuple[str, str]:
    """Try to guess a meaningful step id from the title, or fall back to
    ``<prefix>-<NN>``."""
    # Look for an embedded ID pattern anywhere in the title
    m = re.search(
        r"\b(?P<id>[A-Z][A-Z0-9_-]{0,31}-\d{2})\b", title, re.IGNORECASE
    )
    if m:
        return m.group("id").upper(), title
    return make_step_id(prefix, sequence), title


def derive_checklist_from_plan(
    artifact: Any,   # PlanArtifact (lazy import to avoid circular dep)
    task_code: str = "TASK",
) -> TaskChecklist:
    """Derive a :class:`TaskChecklist` from an approved :class:`PlanArtifact`.

    Parses the **具体计划** section looking for step-like lines (§4).  Each
    detected step becomes a :class:`ChecklistItem` with ``status=todo``.
    Annotations are populated from the plan's deferred decisions and section
    evidence statuses.

    If no steps are detected, the entire section content is treated as a single
    step so the checklist is never empty when a plan exists.

    Returns a :class:`TaskChecklist` ready for journal emission or TUI display.
    """
    # Locate the "具体计划" section.
    plan_section = None
    for sec in artifact.sections:
        if sec.title == "具体计划":
            plan_section = sec
            break

    now = ""
    try:
        from .utils import utc_now as _utc
        now = _utc()
    except Exception:
        pass

    items: list[ChecklistItem] = []
    _SEEN_IDS.clear()
    seq = 0

    if plan_section and plan_section.content_md.strip():
        lines = plan_section.content_md.splitlines()
        for line in lines:
            stripped = line.strip()
            if not stripped:
                continue

            matched = False
            for pat in _STEP_PATTERNS:
                m = pat.match(stripped)
                if m is None:
                    continue
                matched = True

                sid = m.groupdict().get("id")
                title = m.group("title").strip()

                if sid:
                    sid = sid.upper()
                    if sid in _SEEN_IDS:
                        # Duplicate — skip (already captured)
                        break
                else:
                    seq += 1
                    sid, title = _synthesize_step_id(task_code, seq, title)

                if not validate_step_id(sid):
                    # If the embedded ID doesn't validate, synthesize one.
                    seq += 1
                    sid = make_step_id(task_code, seq)

                _SEEN_IDS.add(sid)
                items.append(ChecklistItem(
                    step_id=sid,
                    title=title,
                    status=ChecklistStatus.TODO,
                    created_at=now,
                    updated_at=now,
                ))
                break  # first matching pattern wins

            if not matched:
                # Non-matching line — could be a continuation of the previous
                # item's description.  We don't capture continuations here
                # because L1/L2/L3 have limited space; the full plan remains
                # available in the plan artifact for detailed reading.
                pass

    # If no steps were detected, create a single synthetic step from the
    # whole section content (or the plan title as last resort).
    if not items:
        title = ""
        if plan_section and plan_section.content_md.strip():
            # Use the first non-empty line as the title
            for line in plan_section.content_md.splitlines():
                stripped = line.strip()
                if stripped and not stripped.startswith("#"):
                    title = stripped.lstrip("-* 0123456789.) ")[:80]
                    break
        if not title:
            title = plan_section.title if plan_section else "Execute plan"
        items.append(ChecklistItem(
            step_id=make_step_id(task_code, 1),
            title=title,
            status=ChecklistStatus.TODO,
            created_at=now,
            updated_at=now,
        ))

    # Build annotations shared across all items (plan-level context).
    shared_annotation = ChecklistAnnotation(
        plan_revision=f"v{artifact.version}",
        source_section="具体计划",
        deferred_decisions=list(artifact.deferred_decisions),
    )

    # Attach annotations to every item.
    for item in items:
        ann = ChecklistAnnotation(
            plan_revision=shared_annotation.plan_revision,
            source_section=shared_annotation.source_section,
            deferred_decisions=list(shared_annotation.deferred_decisions),
        )
        item.annotations = ann

    return TaskChecklist(
        plan_id=artifact.plan_id,
        task_id=artifact.task_id,
        run_id=artifact.run_id,
        items=items,
        created_at=now,
        updated_at=now,
    )


# ── B2 — journal registration ─────────────────────────────────────────────────


CHECKLIST_ARTIFACT_SCHEMA = "task-checklist-v0.1.schema.json"


def checklist_to_journal_payload(checklist: TaskChecklist) -> dict[str, Any]:
    """Serialize a :class:`TaskChecklist` into a journal-event payload.

    The payload is suitable for a ``checklist_updated`` event in the
    canonical CLI journal.  Status values are serialised as plain strings.
    """
    return {
        "schema_version": "0.1.0-draft",
        "checklist_sha256": _checklist_sha256(checklist),
        "plan_id": checklist.plan_id,
        "task_id": checklist.task_id,
        "run_id": checklist.run_id,
        "item_count": len(checklist.items),
        "current_step": (
            checklist.current_item.step_id if checklist.current_item else ""
        ),
        "items": [item.to_dict() for item in checklist.items],
    }


def _checklist_sha256(checklist: TaskChecklist) -> str:
    """Stable SHA-256 of the checklist for journal references."""
    try:
        from .utils import canonical_bytes, sha256_bytes
        return sha256_bytes(canonical_bytes(checklist.to_dict()))
    except Exception:
        return ""


# ── B3 — GPS mapping ──────────────────────────────────────────────────────────


def map_checklist_to_gps_plan(
    checklist: TaskChecklist,
) -> list[dict[str, Any]]:
    """Map checklist items to GPS ``plan.steps[]`` entries.

    Each :class:`ChecklistItem` is projected into a GPS plan step with
    ``step_id``, ``direction_id`` (derived from the step-id task-part code),
    ``state``, and ``acceptance_refs``.
    """
    steps: list[dict[str, Any]] = []
    for item in checklist.items:
        # Derive direction_id from the task-part code of the step_id.
        # e.g. "GAK-01" → "gak"
        parts = item.step_id.rsplit("-", 1)
        direction_id = parts[0].lower() if len(parts) >= 1 else "task"

        # Map ChecklistStatus → GPS state
        state_map: dict[ChecklistStatus, str] = {
            ChecklistStatus.TODO: "pending",
            ChecklistStatus.DOING: "in_progress",
            ChecklistStatus.DONE: "completed",
            ChecklistStatus.BLOCKED: "pending",
            ChecklistStatus.DEFERRED: "deferred",
            ChecklistStatus.REPLANNED: "pending",
        }
        gps_state = state_map.get(item.status, "pending")

        steps.append({
            "step_id": item.step_id,
            "direction_id": direction_id,
            "state": gps_state,
            "acceptance_refs": (
                list(item.annotations.acceptance_refs)
                if item.annotations.acceptance_refs
                else []
            ),
            "defer_reason": (
                "blocked" if item.status == ChecklistStatus.BLOCKED
                else "deferred" if item.status == ChecklistStatus.DEFERRED
                else None
            ),
            "last_activity_review_cycle": None,
        })
    return steps


def map_checklist_to_gps_journal(
    checklist: TaskChecklist,
) -> list[dict[str, Any]]:
    """Map checklist status changes to GPS ``journal[]`` entries.

    Only items that have moved past ``todo`` produce journal events.
    """
    journal: list[dict[str, Any]] = []
    for seq, item in enumerate(checklist.items):
        parts = item.step_id.rsplit("-", 1)
        direction_id = parts[0].lower() if len(parts) >= 1 else "task"

        event_type = "action_terminal"
        terminal_state = None
        write_effect = False
        verification_state = None

        if item.status == ChecklistStatus.DONE:
            terminal_state = "succeeded"
        elif item.status == ChecklistStatus.BLOCKED:
            terminal_state = "failed"
        elif item.status == ChecklistStatus.DEFERRED:
            terminal_state = "cancelled"
        elif item.status == ChecklistStatus.REPLANNED:
            event_type = "plan_revised"
        elif item.status == ChecklistStatus.DOING:
            terminal_state = "unknown"
        else:
            continue  # TODO — no journal entry

        journal.append({
            "sequence": seq,
            "event_id": f"EVT-CHK-{checklist.plan_id[5:]}-{seq:03d}",
            "event_type": event_type,
            "step_id": item.step_id,
            "direction_id": direction_id,
            "terminal_state": terminal_state,
            "write_effect": write_effect,
            "verification_state": verification_state,
        })
    return journal


def map_checklist_to_gps_plan_revision(checklist: TaskChecklist) -> int:
    """Extract GPS plan revision from checklist annotations.

    Returns 1 if no revision can be determined.
    """
    for item in checklist.items:
        rev = item.annotations.plan_revision
        if rev and rev.startswith("v"):
            try:
                return int(rev[1:])
            except ValueError:
                pass
    return 1
