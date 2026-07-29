"""View-model projector — GAK-UI-001 Phase 1.

Translates typed :class:`~tui.events.TuiEvent` instances into widget
state mutations on :class:`~tui.app.TuiPrototype`.  This is the bridge
between the event stream and the visual layer.

Architecture rule (from :file:`architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`):
    *Core modules emit structured events.  The UI decides how to display them.*

Every handler is a pure function that reads event fields and mutates
widget dataclass fields in place.  Widgets re-read their fields on the
next :meth:`render` call, so no invalidation or notification is needed.
"""

from __future__ import annotations

from typing import Any, Callable

from .events import (
    ArtifactRegisteredEvent,
    ErrorEvent,
    GateDecisionEvent,
    InstructionProvenanceGateEvent,
    ModelOutputEvent,
    ModelRequestEvent,
    OrientationCheckpointEvent,
    PermissionDecisionEvent,
    PlanApprovalDecisionEvent,
    PlanPhaseEnteredEvent,
    PlanPhaseSubmittedEvent,
    RunCancelledEvent,
    RunFailedEvent,
    RunFinishedEvent,
    RunPreflightEvent,
    RunStartedEvent,
    SourceVisibilityEvent,
    StatusUpdateEvent,
    TaskChecklistEvent,
    ToolAvailabilityEvent,
    ToolCompletedEvent,
    ToolProposalEvent,
    ToolStartedEvent,
    TuiEvent,
    TuiEventKind,
    UsageSampleEvent,
    is_terminal,
)


# ── public API ───────────────────────────────────────────────────────────────


def apply_event(app: Any, event: TuiEvent) -> list[str]:
    """Project *event* onto *app*'s widgets and return status messages.

    Parameters
    ----------
    app:
        A :class:`TuiPrototype` instance.  (Typed as ``Any`` to avoid a
        circular import between ``projector`` and ``app``.)
    event:
        The event to project.

    Returns
    -------
    :
        Human-readable status messages describing what changed (may be
        empty if the event produced no visible mutation).
    """
    kind = event.kind
    handler = _DISPATCH.get(kind)
    if handler is not None:
        try:
            return handler(app, event)
        except AttributeError:
            # Base TuiEvent lacks subclass-specific fields — safe no-op
            return []
    return []


# ── dispatch table ───────────────────────────────────────────────────────────

_DISPATCH: dict[TuiEventKind, Callable[[Any, TuiEvent], list[str]]] = {}


def _register(kind: TuiEventKind):
    """Decorator that registers a handler for *kind* in the dispatch table."""
    def decorator(fn):
        _DISPATCH[kind] = fn
        return fn
    return decorator


# ── lifecycle handlers ───────────────────────────────────────────────────────


@_register(TuiEventKind.RUN_PREFLIGHT)
def _on_run_preflight(app: Any, event: RunPreflightEvent) -> list[str]:
    model_id = getattr(event, "model_id", "")
    adapter_id = getattr(event, "adapter_id", "")
    app.status_bar.update_item("预检", True)
    app.status_bar.update_item("空闲", False)
    if model_id:
        app.status_bar.update_item(model_id, True)
    app.explorer_pane.add_event_entry(
        "Run", f"preflight: {adapter_id} → {model_id}"
    )
    return [f"Preflight: {model_id}"]


@_register(TuiEventKind.RUN_STARTED)
def _on_run_started(app: Any, event: RunStartedEvent) -> list[str]:
    app.running = True
    app.status_bar.update_item("PREFLIGHT", False)
    app.status_bar.update_item("运行中", True)
    app.explorer_pane.add_event_entry(
        "Run", f"started: {event.task_id}"
    )
    app.content_pane.set_disposition("RUNNING", f"task: {event.task_id}")
    return [f"Run started: {event.task_id}"]


@_register(TuiEventKind.RUN_FINISHED)
def _on_run_finished(app: Any, event: RunFinishedEvent) -> list[str]:
    app.running = False
    app.status_bar.update_item("运行中", False)
    app.status_bar.update_item("空闲", True)
    app.explorer_pane.add_event_entry("Run", f"finished ({event.status})")
    app.content_pane.set_disposition("COMPLETED", "")
    app.content_pane.collapse_non_warnings()
    return [f"Run finished: {event.status}"]


@_register(TuiEventKind.RUN_FAILED)
def _on_run_failed(app: Any, event: RunFailedEvent) -> list[str]:
    app.running = False
    app.status_bar.update_item("RUNNING", False)
    app.status_bar.update_item("失败", True)
    app.explorer_pane.add_event_entry("Run", f"FAILED: {event.reason}")
    app.explorer_pane.add_event_entry("Errors", event.reason)
    app.content_pane.set_disposition("FAILED", event.reason)
    app.content_pane.collapse_non_warnings()
    return [f"Run failed: {event.reason}"]


@_register(TuiEventKind.RUN_CANCELLED)
def _on_run_cancelled(app: Any, event: RunCancelledEvent) -> list[str]:
    app.running = False
    app.status_bar.update_item("运行中", False)
    app.status_bar.update_item("空闲", True)
    app.explorer_pane.add_event_entry("Run", "cancelled")
    return ["Run cancelled"]


# ── gate handlers ────────────────────────────────────────────────────────────


@_register(TuiEventKind.GATE_DECISION)
def _on_gate_decision(app: Any, event: GateDecisionEvent) -> list[str]:
    """Handle both ``gate_decision`` and per-source-visibility events."""
    gate_name = getattr(event, "gate_name", "")
    decision = getattr(event, "decision", "?")
    reason = getattr(event, "reason", "")
    reference_count = getattr(event, "reference_count", 0)

    # If it's a per-source visibility event, update ContentPane
    source_id = getattr(event, "source_id", "")
    if source_id:
        observed = getattr(event, "observed", "?")
        required = getattr(event, "required", "?")
        claim = getattr(event, "claim", "?")
        app.content_pane.add_or_update_source_row(
            source_id, observed, required, decision, claim,
        )
        return [f"Source {source_id}: {decision}"]

    if gate_name:
        label = gate_name.replace("_", " ").title()
        app.explorer_pane.add_event_entry(
            "Decisions",
            f"{label}: {decision}" + (f" — {reason}" if reason else ""),
        )
        return [f"Gate {gate_name}: {decision}"]
    return []


@_register(TuiEventKind.INSTRUCTION_PROVENANCE_GATE)
def _on_ipg(app: Any, event: TuiEvent) -> list[str]:
    """Handle instruction_provenance_gate events — now with typed fields."""
    decision = getattr(event, "decision", "") or "evaluated"
    receipt = getattr(event, "receipt_sha256", "")
    reason = getattr(event, "reason", "")
    text = f"IPG: {decision}"
    if reason:
        text += f" — {reason}"
    elif receipt:
        text += f" (receipt: {receipt[:12]}…)"
    app.explorer_pane.add_event_entry("Decisions", text)
    return [text]


@_register(TuiEventKind.TOOL_AVAILABILITY_CHECK)
def _on_tool_availability(app: Any, event: TuiEvent) -> list[str]:
    """Handle tool_availability_check events — now with typed fields."""
    avail = getattr(event, "available", 0)
    unavail = getattr(event, "unavailable", 0)
    unprobed = getattr(event, "unprobed", 0)
    degraded = getattr(event, "degraded", 0)
    parts = [f"{avail} avail"]
    if unavail:
        parts.append(f"{unavail} unavail")
    if unprobed:
        parts.append(f"{unprobed} unprobed")
    if degraded:
        parts.append(f"{degraded} degraded")
    text = f"Tools: {', '.join(parts)}"
    app.explorer_pane.add_event_entry("Decisions", text)
    return [text]


@_register(TuiEventKind.ORIENTATION_CHECKPOINT)
def _on_orientation(app: Any, event: TuiEvent) -> list[str]:
    """Handle orientation_checkpoint events — now with typed fields."""
    cid = getattr(event, "checkpoint_sha256", "")
    step = getattr(event, "trigger_step", 0)
    if cid:
        text = f"Orientation: step {step} ({cid[:12]}…)"
    else:
        text = "checkpoint: saved"
    app.explorer_pane.add_event_entry("Run", text)
    return []


# ── model handlers ───────────────────────────────────────────────────────────


@_register(TuiEventKind.MODEL_REQUEST)
def _on_model_request(app: Any, event: ModelRequestEvent) -> list[str]:
    app.status_bar.update_item("模型", True)
    app.status_bar.update_item("RUNNING", False)
    app.explorer_pane.add_event_entry(
        "Run", f"model request: {event.provider}/{event.model_id}"
    )
    return [f"Model request: {event.model_id}"]


@_register(TuiEventKind.MODEL_OUTPUT)
def _on_model_output(app: Any, event: ModelOutputEvent) -> list[str]:
    app.status_bar.update_item("模型", False)
    app.status_bar.update_item("运行中", True)
    valid_str = "valid" if event.structured_output_valid else "INVALID"
    app.explorer_pane.add_event_entry(
        "Run", f"model output ({valid_str})"
    )
    app.content_pane.add_message(
        "模型输出", f"sha256: {event.answer_packet_sha256[:20]}...",
        warning=not event.structured_output_valid,
    )
    return [f"Model output received ({valid_str})"]


# ── tool handlers ────────────────────────────────────────────────────────────


@_register(TuiEventKind.TOOL_PROPOSAL)
def _on_tool_proposal(app: Any, event: ToolProposalEvent) -> list[str]:
    app.explorer_pane.add_event_entry(
        "Tool Calls", f"proposed: {event.tool_name}"
    )
    return [f"Tool proposed: {event.tool_name}"]


@_register(TuiEventKind.TOOL_STARTED)
def _on_tool_started(app: Any, event: ToolStartedEvent) -> list[str]:
    app.explorer_pane.add_event_entry(
        "Tool Calls", f"started: {event.tool_name}"
    )
    return [f"Tool started: {event.tool_name}"]


@_register(TuiEventKind.TOOL_COMPLETED)
def _on_tool_completed(app: Any, event: ToolCompletedEvent) -> list[str]:
    ok = event.status == "success"
    app.status_bar.update_item(event.tool_name, ok)
    app.explorer_pane.add_event_entry(
        "Tool Calls", f"completed: {event.tool_name} ({event.status})"
    )
    app.content_pane.add_message(
        "工具调用", f"{event.tool_name}: {event.status}",
        collapsible=True,
        warning=event.status != "success",
    )
    return [f"Tool {event.tool_name}: {event.status}"]


# ── permission / artifact handlers ───────────────────────────────────────────


@_register(TuiEventKind.PERMISSION_DECISION)
def _on_permission(app: Any, event: PermissionDecisionEvent) -> list[str]:
    app.explorer_pane.add_event_entry(
        "Permissions", f"{event.permission}: {event.decision}"
    )
    app.content_pane.add_message(
        "权限", f"{event.permission}: {event.decision}",
        warning=event.decision not in ("granted", "approved"),
    )
    return [f"Permission {event.permission}: {event.decision}"]


@_register(TuiEventKind.ARTIFACT_REGISTERED)
def _on_artifact(app: Any, event: ArtifactRegisteredEvent) -> list[str]:
    short_path = event.artifact_path
    if len(short_path) > 40:
        short_path = "…" + short_path[-39:]
    app.explorer_pane.add_event_entry(
        "Artifacts", f"registered: {short_path}"
    )
    return [f"Artifact: {short_path}"]


# ── auxiliary handlers ───────────────────────────────────────────────────────


@_register(TuiEventKind.ERROR_EVENT)
def _on_error(app: Any, event: ErrorEvent) -> list[str]:
    app.status_bar.update_item("错误", False)
    app.explorer_pane.add_event_entry(
        "Errors", f"[{event.source}] {event.message}"
    )
    app.content_pane.add_message(
        "错误", f"[{event.source}] {event.message}",
        warning=True,   # errors are never collapsed
    )
    return [f"Error [{event.source}]: {event.message}"]


@_register(TuiEventKind.STATUS_UPDATE)
def _on_status_update(app: Any, event: StatusUpdateEvent) -> list[str]:
    app.status_bar.update_item(event.label, event.ok)
    return [f"Status: {event.label}={event.ok}"]


# ── plan mode handlers (GAK-PLAN-001) ───────────────────────────────────────


@_register(TuiEventKind.PLAN_PHASE_ENTERED)
def _on_plan_phase_entered(app: Any, event: PlanPhaseEnteredEvent) -> list[str]:
    app.status_bar.update_item("计划", True)
    app.status_bar.update_item("空闲", False)
    app.content_pane.add_message("计划阶段", f"policy={event.planning_policy}")
    return [f"计划阶段: policy={event.planning_policy}"]


@_register(TuiEventKind.PLAN_PHASE_SUBMITTED)
def _on_plan_phase_submitted(app: Any, event: PlanPhaseSubmittedEvent) -> list[str]:
    app.status_bar.update_item("等待", True)
    app.content_pane.add_message(
        "计划提交", f"{event.plan_id} v{event.version} ({event.section_count} sections)",
    )
    return [f"计划已提交: {event.plan_id} v{event.version}"]


@_register(TuiEventKind.PLAN_APPROVAL_DECISION)
def _on_plan_approval_decision(app: Any, event: PlanApprovalDecisionEvent) -> list[str]:
    app.status_bar.update_item("计划", False)
    app.status_bar.update_item("等待", False)
    decision = event.decision
    if decision == "approve":
        app.status_bar.update_item("执行", True)
        # Derive checklist from plan and populate the announcement strip.
        try:
            from .bridge import build_checklist_from_plan_id
            items = build_checklist_from_plan_id(event.plan_id)
            if items:
                app.announcement_strip.load_checklist(
                    plan_id=event.plan_id,
                    task_id="",
                    items=items,
                )
        except Exception:
            pass
    app.content_pane.add_message(
        "计划审批", f"Plan {event.plan_id}: {decision}",
        warning=decision not in ("approve",),
    )
    return [f"计划{decision}: {event.plan_id}"]


@_register(TuiEventKind.CHECKLIST_DERIVED)
def _on_checklist_derived(app: Any, event: TaskChecklistEvent) -> list[str]:
    app.announcement_strip.load_checklist(
        plan_id=event.plan_id,
        task_id=event.task_id,
        items=event.items,
    )
    return [f"Checklist ready: {len(event.items)} items"]


# ── process usage monitor handler ────────────────────────────────────────────


@_register(TuiEventKind.USAGE_SAMPLE)
def _on_usage_sample(app: Any, event: UsageSampleEvent) -> list[str]:
    line = event.anomaly_line or event.status_line
    if line:
        app.status_bar.update_item("资源", True)
    return [line] if line else []
