"""TUI-to-Assurance bridge — GAK-UI-001 Phase 2.

**This is the ONLY module in the TUI package allowed to import from**
``assurance.*``.  It lives at the application boundary and is imported
lazily by :class:`JsonlFileSource` and :class:`LiveRunEventSource`.

Responsibilities:

1. Map canonical CLI journal JSONL lines → typed :class:`TuiEvent` subclasses.
2. Provide an ``on_event`` callback factory for live streaming from
   ``_build_events_and_receipt()``.
3. (Future) Build TuiEvents directly from gate receipt dicts without
   going through the journal.
"""

from __future__ import annotations

import json
import queue
from dataclasses import dataclass, field
from typing import Any, Callable

# ── TUI-owned types (no assurance imports needed for these) ──────────────────
from .events import (
    ArtifactRegisteredEvent,
    ChecklistItemStatusEvent,
    ErrorEvent,
    GateDecisionEvent,
    InstructionProvenanceGateEvent,
    ModelOutputEvent,
    ModelRequestEvent,
    OrientationCheckpointEvent,
    PlanApprovalDecisionEvent,
    PlanPhaseEnteredEvent,
    PlanPhaseSubmittedEvent,
    RetrievalOutcome,
    RetrievalProgress,
    RunFailedEvent,
    RunFinishedEvent,
    RunPreflightEvent,
    RunStartedEvent,
    SourceVisibilityEvent,
    StatusUpdateEvent,
    TaskChecklistEvent,
    ToolAvailabilityEvent,
    TuiEvent,
    TuiEventKind,
    UsageSampleEvent,
)

# ══════════════════════════════════════════════════════════════════════════════
# JSONL line → TuiEvent factory
# ══════════════════════════════════════════════════════════════════════════════


def build_event_from_jsonl_line(line: dict[str, Any]) -> TuiEvent:
    """Map a single JSONL journal line to the correct :class:`TuiEvent` subclass.

    Parameters
    ----------
    line:
        A dict with ``event_type``, ``payload``, ``timestamp``, and
        optional ``payload_schema`` / ``event_sha256`` fields — exactly
        as written by :func:`assurance.canonical_cli._build_event`.

    Returns
    -------
    :
        A typed :class:`TuiEvent` subclass with all available fields
        populated from the payload.  Falls back to a bare
        :class:`TuiEvent` with the mapped ``kind`` when the event_type
        is unrecognised.
    """
    event_type = line.get("event_type", "")
    payload: dict[str, Any] = line.get("payload", {}) or {}
    timestamp = line.get("timestamp", "")

    factory = _EVENT_FACTORY.get(event_type)
    if factory is None:
        # Unknown event_type — produce a bare TuiEvent with best-guess kind
        kind = _kind_for_event_type(event_type)
        return TuiEvent(kind=kind, timestamp=timestamp)

    return factory(payload, timestamp)


def _kind_for_event_type(event_type: str) -> TuiEventKind:
    """Best-effort mapping from journal ``event_type`` to a TuiEventKind."""
    return _KIND_MAP.get(event_type, TuiEventKind.STATUS_UPDATE)


# ══════════════════════════════════════════════════════════════════════════════
# LiveRunEventSource wiring (imports from assurance.* — allowed in bridge.py)
# ══════════════════════════════════════════════════════════════════════════════


def build_live_run_fn(
    *,
    run_root: str,
    ask: str | None = None,
    task_contract_path: str | None = None,
    real_adapter: bool = False,
    **kwargs: Any,
) -> Callable[[Callable[[dict[str, Any]], None]], dict[str, Any]]:
    """Build a ``run_fn`` for :class:`.event_source.LiveRunEventSource`.

    Returns a callable that accepts an ``on_event`` callback and executes
    the canonical CLI.  The *on_event* callback receives each journal
    event dict as it is built.

    This function is the **only** place the bridge imports from
    ``assurance.canonical_cli`` — everything else in the TUI layer
    stays free of assurance imports.
    """
    import json
    import os
    import tempfile
    from pathlib import Path

    from assurance.canonical_cli import (
        run_canonical_guarded_cli,
        run_canonical_guarded_cli_real,
    )

    _run_root = Path(run_root)
    _task_contract = Path(task_contract_path) if task_contract_path else None

    # When a plain ask string is provided without a source ledger or task
    # contract, reuse the project's mixed-visibility fixture.  This is
    # schema-valid and exercises the source visibility gate correctly.
    _source_ledger: Path | None = None
    if ask is not None and task_contract_path is None:
        import assurance
        _fixture_dir = Path(assurance.__file__).parent / "fixtures" / "source_visibility"
        _candidate = _fixture_dir / "mixed-visibility-ledger.json"
        if _candidate.is_file():
            _source_ledger = _candidate

    if real_adapter:
        def _run(on_event: Callable[[dict[str, Any]], None]) -> dict[str, Any]:
            return run_canonical_guarded_cli_real(
                run_root=_run_root,
                ask=ask,
                source_ledger_path=_source_ledger,
                task_contract_path=_task_contract,
                on_event=on_event,
                audit=True,  # GAK-EVT-001: enable audit ledger
                **kwargs,
            )
    else:
        def _run(on_event: Callable[[dict[str, Any]], None]) -> dict[str, Any]:
            return run_canonical_guarded_cli(
                run_root=_run_root,
                ask=ask,
                source_ledger_path=_source_ledger,
                task_contract_path=_task_contract,
                on_event=on_event,
                **kwargs,
            )

    return _run


# ══════════════════════════════════════════════════════════════════════════════
# on_event callback factory (for live streaming)
# ══════════════════════════════════════════════════════════════════════════════


def build_on_event_callback(
    target_queue: queue.Queue[TuiEvent],
) -> Callable[[dict[str, Any]], None]:
    """Build a callback suitable for passing to ``_build_events_and_receipt(on_event=...)``.

    Each journal event dict is mapped to a :class:`TuiEvent` and pushed
    onto *target_queue* for consumption by the TUI drain loop.
    """
    def _on_event(journal_event: dict[str, Any]) -> None:
        tui_evt = build_event_from_jsonl_line(journal_event)
        target_queue.put(tui_evt)

    return _on_event


# ══════════════════════════════════════════════════════════════════════════════
# Event factory implementations
# ══════════════════════════════════════════════════════════════════════════════


def _make_run_preflight(payload: dict[str, Any], timestamp: str) -> RunPreflightEvent:
    return RunPreflightEvent(
        timestamp=timestamp,
        model_id=payload.get("model_id", ""),
        adapter_id=payload.get("adapter_id", ""),
        real_network_allowed=payload.get("real_network_allowed", False),
    )


def _make_ipg_event(payload: dict[str, Any], timestamp: str) -> InstructionProvenanceGateEvent:
    """Map the ``instruction_provenance_gate`` journal event.

    The journal payload only carries receipt_sha256 and context_sha256.
    The actual gate decision lives in the full IPG receipt file on disk
    (too expensive to load during replay).  We surface the receipt_sha256
    as evidence that the gate was evaluated.
    """
    receipt = payload.get("receipt_sha256", "")
    return InstructionProvenanceGateEvent(
        timestamp=timestamp,
        receipt_sha256=receipt,
        decision="evaluated",  # actual decision is in the receipt file
    )


def _make_tool_avail_event(payload: dict[str, Any], timestamp: str) -> ToolAvailabilityEvent:
    return ToolAvailabilityEvent(
        timestamp=timestamp,
        available=payload.get("available_count", 0),
        unavailable=payload.get("unavailable_count", 0),
        unprobed=payload.get("unprobed_count", 0),
        degraded=payload.get("degraded_count", 0),
    )


def _make_orientation_event(payload: dict[str, Any], timestamp: str) -> OrientationCheckpointEvent:
    return OrientationCheckpointEvent(
        timestamp=timestamp,
        checkpoint_sha256=payload.get("checkpoint_sha256", ""),
        trigger_step=payload.get("trigger_step", 0),
    )


def _make_run_started(payload: dict[str, Any], timestamp: str) -> RunStartedEvent:
    return RunStartedEvent(
        timestamp=timestamp,
        task_id=payload.get("task_id", ""),
        run_root=payload.get("run_root", ""),
    )


def _make_source_vis_events(
    payload: dict[str, Any], timestamp: str,
) -> list[TuiEvent]:
    """Map the ``gate_decision`` journal event (source visibility gate).

    Produces a :class:`GateDecisionEvent` for the overall decision AND
    one :class:`SourceVisibilityEvent` per per-reference decision when
    available in the payload.
    """
    events: list[TuiEvent] = []

    decision = payload.get("decision", "")
    reference_count = payload.get("reference_count", 0)

    events.append(GateDecisionEvent(
        timestamp=timestamp,
        gate_name="source_visibility",
        decision=decision,
        reason=f"{reference_count} references",
        reference_count=reference_count,
    ))

    # Per-reference decisions (if present in payload)
    per_ref: list[dict[str, Any]] = payload.get("reference_decisions", []) or []
    for ref in per_ref:
        events.append(SourceVisibilityEvent(
            timestamp=timestamp,
            source_id=ref.get("ref_id", ref.get("source_id", "")),
            observed=_visibility_label(ref.get("observed_visibility", "")),
            required=_visibility_label(ref.get("required_visibility", "")),
            decision=ref.get("decision", "").upper(),
            claim=ref.get("claim_allowed", ""),
        ))

    return events


def _make_model_request(payload: dict[str, Any], timestamp: str) -> ModelRequestEvent:
    return ModelRequestEvent(
        timestamp=timestamp,
        provider=payload.get("provider", ""),
        model_id=payload.get("model_id", ""),
    )


def _make_model_output(payload: dict[str, Any], timestamp: str) -> ModelOutputEvent:
    return ModelOutputEvent(
        timestamp=timestamp,
        answer_packet_sha256=payload.get("answer_packet_sha256", ""),
        structured_output_valid=payload.get("structured_output_valid", True),
    )


def _make_artifact_registered(payload: dict[str, Any], timestamp: str) -> ArtifactRegisteredEvent:
    return ArtifactRegisteredEvent(
        timestamp=timestamp,
        artifact_path=payload.get("artifact_path", ""),
        artifact_sha256=payload.get("artifact_sha256", ""),
    )


def _make_run_finished(payload: dict[str, Any], timestamp: str) -> RunFinishedEvent:
    return RunFinishedEvent(
        timestamp=timestamp,
        status=payload.get("status", "completed"),
    )


def _make_run_failed(payload: dict[str, Any], timestamp: str) -> RunFailedEvent:
    return RunFailedEvent(
        timestamp=timestamp,
        reason=payload.get("status", "unknown error"),
    )


def _make_error_event(payload: dict[str, Any], timestamp: str) -> ErrorEvent:
    return ErrorEvent(
        timestamp=timestamp,
        message=payload.get("message", payload.get("status", "")),
        source=payload.get("source", "unknown"),
    )


# ── plan mode factories (GAK-PLAN-001) ──────────────────────────────────────


def _make_plan_phase_entered(payload: dict[str, Any], timestamp: str) -> PlanPhaseEnteredEvent:
    return PlanPhaseEnteredEvent(
        timestamp=timestamp,
        planning_policy=payload.get("planning_policy", ""),
        task_id=payload.get("task_id", ""),
        run_id=payload.get("run_id", ""),
    )


def _make_plan_phase_submitted(payload: dict[str, Any], timestamp: str) -> PlanPhaseSubmittedEvent:
    return PlanPhaseSubmittedEvent(
        timestamp=timestamp,
        plan_id=payload.get("plan_id", ""),
        plan_sha256=payload.get("plan_sha256", ""),
        section_count=payload.get("section_count", 4),
        version=payload.get("version", 1),
    )


def _make_plan_approval_decision(payload: dict[str, Any], timestamp: str) -> PlanApprovalDecisionEvent:
    return PlanApprovalDecisionEvent(
        timestamp=timestamp,
        plan_id=payload.get("plan_id", ""),
        decision=payload.get("decision", ""),
        authority=payload.get("authority", ""),
        execution_policy=payload.get("execution_policy", ""),
    )


# ── process usage factory ───────────────────────────────────────────────────


def _make_usage_sample(payload: dict[str, Any], timestamp: str) -> UsageSampleEvent:
    return UsageSampleEvent(
        timestamp=timestamp,
        cpu_percent=payload.get("cpu_percent", 0.0),
        memory_bytes=payload.get("memory_bytes", 0),
        elapsed_seconds=payload.get("elapsed_seconds", 0.0),
        process_count=payload.get("process_count", 0),
        completeness=payload.get("completeness", "complete"),
        status_line=payload.get("status_line", ""),
        anomaly_line=payload.get("anomaly_line", ""),
    )


# ── checklist factories ──────────────────────────────────────────────────────


def _make_checklist_derived(payload: dict[str, Any], timestamp: str) -> TaskChecklistEvent:
    return TaskChecklistEvent(
        timestamp=timestamp,
        plan_id=payload.get("plan_id", ""),
        task_id=payload.get("task_id", ""),
        run_id=payload.get("run_id", ""),
        items=payload.get("items", []),
        created_at=payload.get("created_at", ""),
    )


# ══════════════════════════════════════════════════════════════════════════════
# Dispatch tables
# ══════════════════════════════════════════════════════════════════════════════


_EVENT_FACTORY: dict[str, Callable[[dict[str, Any], str], TuiEvent | list[TuiEvent]]] = {
    "run_preflight": _make_run_preflight,
    "instruction_provenance_gate": _make_ipg_event,
    "tool_availability_check": _make_tool_avail_event,
    "orientation_checkpoint": _make_orientation_event,
    "run_started": _make_run_started,
    "gate_decision": _make_source_vis_events,
    "model_request": _make_model_request,
    "model_output": _make_model_output,
    "artifact_registered": _make_artifact_registered,
    "run_finished": _make_run_finished,
    "run_failed": _make_run_failed,
    "run_cancelled": _make_run_failed,   # same shape
    "error_event": _make_error_event,
    "plan_phase_entered": _make_plan_phase_entered,
    "plan_phase_submitted": _make_plan_phase_submitted,
    "plan_approval_decision": _make_plan_approval_decision,
    "usage_sample": _make_usage_sample,
    "checklist_derived": _make_checklist_derived,
}


_KIND_MAP: dict[str, TuiEventKind] = {
    "run_preflight": TuiEventKind.RUN_PREFLIGHT,
    "instruction_provenance_gate": TuiEventKind.INSTRUCTION_PROVENANCE_GATE,
    "tool_availability_check": TuiEventKind.TOOL_AVAILABILITY_CHECK,
    "orientation_checkpoint": TuiEventKind.ORIENTATION_CHECKPOINT,
    "run_started": TuiEventKind.RUN_STARTED,
    "gate_decision": TuiEventKind.GATE_DECISION,
    "model_request": TuiEventKind.MODEL_REQUEST,
    "model_output": TuiEventKind.MODEL_OUTPUT,
    "artifact_registered": TuiEventKind.ARTIFACT_REGISTERED,
    "run_finished": TuiEventKind.RUN_FINISHED,
    "run_failed": TuiEventKind.RUN_FAILED,
    "run_cancelled": TuiEventKind.RUN_CANCELLED,
    "error_event": TuiEventKind.ERROR_EVENT,
    "status_update": TuiEventKind.STATUS_UPDATE,
    "plan_phase_entered": TuiEventKind.PLAN_PHASE_ENTERED,
    "plan_phase_submitted": TuiEventKind.PLAN_PHASE_SUBMITTED,
    "plan_approval_decision": TuiEventKind.PLAN_APPROVAL_DECISION,
    "usage_sample": TuiEventKind.USAGE_SAMPLE,
    "checklist_derived": TuiEventKind.CHECKLIST_DERIVED,
    "checklist_item_status_changed": TuiEventKind.CHECKLIST_ITEM_STATUS_CHANGED,
}


# ══════════════════════════════════════════════════════════════════════════════
# Helpers
# ══════════════════════════════════════════════════════════════════════════════


def _visibility_label(raw: str) -> str:
    """Normalise a visibility string for TUI display."""
    mapping: dict[str, str] = {
        "full_text_observed": "FULL TEXT",
        "partial_text_observed": "PARTIAL",
        "metadata_only": "METADATA ONLY",
        "unavailable": "NONE",
    }
    return mapping.get(raw, raw.upper())


# ══════════════════════════════════════════════════════════════════════════════
# Checklist bridge (GAK-PLAN-001 extension — Part B)
# ══════════════════════════════════════════════════════════════════════════════


def build_checklist_from_plan_id(plan_id: str) -> list[dict[str, object]] | None:
    """Load a :class:`PlanArtifact` from disk and derive checklist items.

    This is the **only** TUI-side function allowed to import from
    ``assurance.task_checklist`` and ``assurance.plan_mode``.  It is called
    by the projector when a plan is approved, so the announcement strip can
    be populated immediately without waiting for a journal event.

    Returns ``None`` if the plan cannot be loaded (e.g. demo/replay mode
    where no plan directory exists on disk).
    """
    try:
        from assurance.plan_mode import load_plan_artifact
        from assurance.task_checklist import derive_checklist_from_plan
        from pathlib import Path

        plan_dir = Path(".gsa_plans") / plan_id
        if not plan_dir.is_dir():
            return None

        artifact = load_plan_artifact(plan_dir)
        checklist = derive_checklist_from_plan(artifact)
        return [item.to_dict() for item in checklist.items]
    except Exception:
        return None
