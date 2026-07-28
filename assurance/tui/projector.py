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
    ModelOutputEvent,
    ModelRequestEvent,
    PermissionDecisionEvent,
    RunCancelledEvent,
    RunFailedEvent,
    RunFinishedEvent,
    RunPreflightEvent,
    RunStartedEvent,
    SourceVisibilityEvent,
    StatusUpdateEvent,
    ToolCompletedEvent,
    ToolProposalEvent,
    ToolStartedEvent,
    TuiEvent,
    TuiEventKind,
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
    app.status_bar.update_item("PREFLIGHT", True)
    app.status_bar.update_item("IDLE", False)
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
    app.status_bar.update_item("RUNNING", True)
    app.explorer_pane.add_event_entry(
        "Run", f"started: {event.task_id}"
    )
    app.content_pane.set_disposition("RUNNING", f"task: {event.task_id}")
    return [f"Run started: {event.task_id}"]


@_register(TuiEventKind.RUN_FINISHED)
def _on_run_finished(app: Any, event: RunFinishedEvent) -> list[str]:
    app.running = False
    app.status_bar.update_item("RUNNING", False)
    app.status_bar.update_item("IDLE", True)
    app.explorer_pane.add_event_entry("Run", f"finished ({event.status})")
    app.content_pane.set_disposition("COMPLETED", "")
    return [f"Run finished: {event.status}"]


@_register(TuiEventKind.RUN_FAILED)
def _on_run_failed(app: Any, event: RunFailedEvent) -> list[str]:
    app.running = False
    app.status_bar.update_item("RUNNING", False)
    app.status_bar.update_item("FAILED", True)
    app.explorer_pane.add_event_entry("Run", f"FAILED: {event.reason}")
    app.explorer_pane.add_event_entry("Errors", event.reason)
    app.content_pane.set_disposition("FAILED", event.reason)
    return [f"Run failed: {event.reason}"]


@_register(TuiEventKind.RUN_CANCELLED)
def _on_run_cancelled(app: Any, event: RunCancelledEvent) -> list[str]:
    app.running = False
    app.status_bar.update_item("RUNNING", False)
    app.status_bar.update_item("IDLE", True)
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
    """Handle instruction_provenance_gate journal events."""
    app.explorer_pane.add_event_entry("Decisions", "IPG: evaluated")
    return []


@_register(TuiEventKind.TOOL_AVAILABILITY_CHECK)
def _on_tool_availability(app: Any, event: TuiEvent) -> list[str]:
    """Handle tool_availability_check journal events."""
    app.explorer_pane.add_event_entry("Decisions", "Tools: probed")
    return []


@_register(TuiEventKind.ORIENTATION_CHECKPOINT)
def _on_orientation(app: Any, event: TuiEvent) -> list[str]:
    """Handle orientation_checkpoint journal events."""
    app.explorer_pane.add_event_entry("Run", "checkpoint: saved")
    return []


# ── model handlers ───────────────────────────────────────────────────────────


@_register(TuiEventKind.MODEL_REQUEST)
def _on_model_request(app: Any, event: ModelRequestEvent) -> list[str]:
    app.status_bar.update_item("MODEL", True)
    app.status_bar.update_item("RUNNING", False)
    app.explorer_pane.add_event_entry(
        "Run", f"model request: {event.provider}/{event.model_id}"
    )
    return [f"Model request: {event.model_id}"]


@_register(TuiEventKind.MODEL_OUTPUT)
def _on_model_output(app: Any, event: ModelOutputEvent) -> list[str]:
    app.status_bar.update_item("MODEL", False)
    app.status_bar.update_item("RUNNING", True)
    valid_str = "valid" if event.structured_output_valid else "INVALID"
    app.explorer_pane.add_event_entry(
        "Run", f"model output ({valid_str})"
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
    return [f"Tool {event.tool_name}: {event.status}"]


# ── permission / artifact handlers ───────────────────────────────────────────


@_register(TuiEventKind.PERMISSION_DECISION)
def _on_permission(app: Any, event: PermissionDecisionEvent) -> list[str]:
    app.explorer_pane.add_event_entry(
        "Permissions", f"{event.permission}: {event.decision}"
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
    app.status_bar.update_item("ERROR", False)
    app.explorer_pane.add_event_entry(
        "Errors", f"[{event.source}] {event.message}"
    )
    return [f"Error [{event.source}]: {event.message}"]


@_register(TuiEventKind.STATUS_UPDATE)
def _on_status_update(app: Any, event: StatusUpdateEvent) -> list[str]:
    app.status_bar.update_item(event.label, event.ok)
    return [f"Status: {event.label}={event.ok}"]
