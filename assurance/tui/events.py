"""TUI event protocol — GAK-UI-001 Phase 1.

Type-safe event dataclasses that mirror the canonical run-event schema
(``runtime/run-event-v0.1.schema.json``) but are **completely independent**
of the assurance core.  No imports from ``assurance.*`` are permitted.

Every event the TUI can consume is represented as a concrete
:class:`TuiEvent` subclass.  The :class:`TuiEventKind` enum enables
pattern-matching dispatch in the projector layer.
"""

from __future__ import annotations

import time as _time

from dataclasses import dataclass, field
from enum import Enum


# ── event kind enum ──────────────────────────────────────────────────────────


class TuiEventKind(str, Enum):
    """Canonical event types the TUI recognises.

    Values match the ``event_type`` strings in
    ``runtime/run-event-v0.1.schema.json``.
    """

    RUN_PREFLIGHT = "run_preflight"
    INSTRUCTION_PROVENANCE_GATE = "instruction_provenance_gate"
    TOOL_AVAILABILITY_CHECK = "tool_availability_check"
    ORIENTATION_CHECKPOINT = "orientation_checkpoint"
    RUN_STARTED = "run_started"
    GATE_DECISION = "gate_decision"
    MODEL_REQUEST = "model_request"
    MODEL_OUTPUT = "model_output"
    TOOL_PROPOSAL = "tool_proposal"
    PERMISSION_DECISION = "permission_decision"
    TOOL_STARTED = "tool_started"
    TOOL_COMPLETED = "tool_completed"
    ARTIFACT_REGISTERED = "artifact_registered"
    RUN_FINISHED = "run_finished"
    RUN_FAILED = "run_failed"
    RUN_CANCELLED = "run_cancelled"
    ERROR_EVENT = "error_event"
    STATUS_UPDATE = "status_update"


# ── base event ───────────────────────────────────────────────────────────────


@dataclass
class TuiEvent:
    """Base for all TUI-consumable events.

    Every subclass carries a :attr:`kind` discriminator so the projector
    can dispatch without ``isinstance`` chains.
    """

    kind: TuiEventKind
    timestamp: str = ""


# ── lifecycle events ─────────────────────────────────────────────────────────


@dataclass
class RunPreflightEvent(TuiEvent):
    """Emitted after gate setup, before the run officially starts."""

    kind: TuiEventKind = field(default=TuiEventKind.RUN_PREFLIGHT, init=False)
    model_id: str = ""
    adapter_id: str = ""
    real_network_allowed: bool = False


@dataclass
class RunStartedEvent(TuiEvent):
    """Emitted when the run begins (all gates passed)."""

    kind: TuiEventKind = field(default=TuiEventKind.RUN_STARTED, init=False)
    task_id: str = ""
    run_root: str = ""


@dataclass
class RunFinishedEvent(TuiEvent):
    """Emitted on successful completion."""

    kind: TuiEventKind = field(default=TuiEventKind.RUN_FINISHED, init=False)
    status: str = "completed"


@dataclass
class RunFailedEvent(TuiEvent):
    """Emitted when the run terminates with an error."""

    kind: TuiEventKind = field(default=TuiEventKind.RUN_FAILED, init=False)
    reason: str = ""


@dataclass
class RunCancelledEvent(TuiEvent):
    """Emitted when the user cancels the run (Ctrl+Z in TUI)."""

    kind: TuiEventKind = field(default=TuiEventKind.RUN_CANCELLED, init=False)
    reason: str = ""


# ── gate events ──────────────────────────────────────────────────────────────


@dataclass
class GateDecisionEvent(TuiEvent):
    """A single gate has produced a decision.

    This covers IPG, tool-availability, source-visibility, and adapter-gate
    decisions.  The :attr:`gate_name` field distinguishes which gate.
    """

    kind: TuiEventKind = field(default=TuiEventKind.GATE_DECISION, init=False)
    gate_name: str = ""       # "instruction_provenance" | "tool_availability" | ...
    decision: str = ""        # "allow" | "defer" | "block"
    reason: str = ""
    reference_count: int = 0


@dataclass
class SourceVisibilityEvent(TuiEvent):
    """A per-reference source-visibility decision.

    One event is emitted for each reference in the source ledger.
    """

    kind: TuiEventKind = field(default=TuiEventKind.GATE_DECISION, init=False)
    source_id: str = ""
    observed: str = ""        # "FULL TEXT" | "PARTIAL" | "NONE"
    required: str = ""        # "FULL TEXT" | "METADATA ONLY"
    decision: str = ""        # "ALLOW" | "DEFER" | "BLOCK"
    claim: str = ""           # "full_text" | "fragment" | "none"


# ── model events ─────────────────────────────────────────────────────────────


@dataclass
class ModelRequestEvent(TuiEvent):
    """Emitted when the adapter sends a request to the model."""

    kind: TuiEventKind = field(default=TuiEventKind.MODEL_REQUEST, init=False)
    provider: str = ""
    model_id: str = ""


@dataclass
class ModelOutputEvent(TuiEvent):
    """Emitted when the model response is received."""

    kind: TuiEventKind = field(default=TuiEventKind.MODEL_OUTPUT, init=False)
    answer_packet_sha256: str = ""
    structured_output_valid: bool = True


# ── tool events ──────────────────────────────────────────────────────────────


@dataclass
class ToolProposalEvent(TuiEvent):
    """The model has proposed a tool call."""

    kind: TuiEventKind = field(default=TuiEventKind.TOOL_PROPOSAL, init=False)
    tool_name: str = ""
    input_summary: str = ""


@dataclass
class ToolStartedEvent(TuiEvent):
    """A tool invocation has begun."""

    kind: TuiEventKind = field(default=TuiEventKind.TOOL_STARTED, init=False)
    tool_name: str = ""


@dataclass
class ToolCompletedEvent(TuiEvent):
    """A tool invocation has finished."""

    kind: TuiEventKind = field(default=TuiEventKind.TOOL_COMPLETED, init=False)
    tool_name: str = ""
    status: str = "success"   # "success" | "error" | "timeout"


# ── permission / artifact events ─────────────────────────────────────────────


@dataclass
class PermissionDecisionEvent(TuiEvent):
    """A permission gate has been evaluated."""

    kind: TuiEventKind = field(default=TuiEventKind.PERMISSION_DECISION, init=False)
    permission: str = ""
    decision: str = ""        # "granted" | "denied" | "deferred"


@dataclass
class ArtifactRegisteredEvent(TuiEvent):
    """An answer packet or other artifact has been persisted."""

    kind: TuiEventKind = field(default=TuiEventKind.ARTIFACT_REGISTERED, init=False)
    artifact_path: str = ""
    artifact_sha256: str = ""


# ── auxiliary events ─────────────────────────────────────────────────────────


@dataclass
class ErrorEvent(TuiEvent):
    """A non-fatal error occurred during the run."""

    kind: TuiEventKind = field(default=TuiEventKind.ERROR_EVENT, init=False)
    message: str = ""
    source: str = ""          # "network" | "gate" | "adapter" | "sandbox" | "unknown"


@dataclass
class StatusUpdateEvent(TuiEvent):
    """Generic status-bar label update.

    Used for fine-grained control over individual status indicators
    without coupling the projector to StatusBar internals.
    """

    kind: TuiEventKind = field(default=TuiEventKind.STATUS_UPDATE, init=False)
    label: str = ""
    ok: bool = True


# ── gate-specific typed events (GAK-UI-001 Phase 2) ──────────────────────────


@dataclass
class InstructionProvenanceGateEvent(TuiEvent):
    """IPG evaluation result — typed fields for the previously-bare event kind."""

    kind: TuiEventKind = field(default=TuiEventKind.INSTRUCTION_PROVENANCE_GATE, init=False)
    decision: str = ""            # "allow" | "defer" | "block" | "evaluated"
    reason: str = ""
    receipt_sha256: str = ""
    routing_count: int = 0
    blocked_count: int = 0


@dataclass
class ToolAvailabilityEvent(TuiEvent):
    """Tool availability check result — typed fields."""

    kind: TuiEventKind = field(default=TuiEventKind.TOOL_AVAILABILITY_CHECK, init=False)
    decision: str = ""            # "allow" | "defer" | "block" | "evaluated"
    available: int = 0
    unavailable: int = 0
    unprobed: int = 0
    degraded: int = 0


@dataclass
class OrientationCheckpointEvent(TuiEvent):
    """Orientation checkpoint created — typed fields."""

    kind: TuiEventKind = field(default=TuiEventKind.ORIENTATION_CHECKPOINT, init=False)
    checkpoint_sha256: str = ""
    trigger_step: int = 0


# ── retrieval events (LBR-001) ────────────────────────────────────────────────


@dataclass
class RetrievalProgress:
    """Progress update emitted during a browser retrieval workflow.

    TUI-owned type — mirrors :class:`assurance.retrieval_workflow.RetrievalProgress`
    without importing from the assurance core.  The wiring layer at
    :mod:`assurance.tui.main` bridges the two.
    """

    stage: str       # "launching", "connecting", "navigating", "reading",
                     # "finding_pdf", "downloading", "validating",
                     # "storing", "indexing", "done", "failed",
                     # "searching", "retrieving"
    message: str
    detail: str = ""
    timestamp: float = field(default_factory=_time.time)


@dataclass
class RetrievalOutcome:
    """Final result of a retrieval workflow, TUI-owned.

    The wiring layer maps :class:`assurance.retrieval_workflow.RetrievalResult`
    and :class:`assurance.retrieval_workflow.WebRetrievalResult` into this
    single type so the TUI never touches assurance types.
    """

    ok: bool = True
    title: str = ""
    summary: str = ""   # one-line summary for content pane
    detail: str = ""    # detailed info for content pane
    error: str = ""


# ── known terminal event kinds ───────────────────────────────────────────────

TERMINAL_EVENT_KINDS: frozenset[TuiEventKind] = frozenset({
    TuiEventKind.RUN_FINISHED,
    TuiEventKind.RUN_FAILED,
    TuiEventKind.RUN_CANCELLED,
})


def is_terminal(event: TuiEvent) -> bool:
    """Return True if *event* represents a terminal run state."""
    return event.kind in TERMINAL_EVENT_KINDS
