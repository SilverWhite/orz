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
    AcpInitializeEvent,
    AcpSessionCreatedEvent,
    ArtifactRegisteredEvent,
    ChecklistItemStatusEvent,
    ErrorEvent,
    GateDecisionEvent,
    InstructionProvenanceGateEvent,
    ModelOutputEvent,
    ModelRequestEvent,
    OrientationCheckpointEvent,
    PermissionDecisionEvent,
    PermissionRequestedEvent,
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
    TextDeltaEvent,
    ToolAvailabilityEvent,
    ToolCompletedEvent,
    ToolProposalEvent,
    TuiEvent,
    TuiEventKind,
    UserMessageEvent,
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


def build_grok_live_run_fn(
    *,
    run_root: str,
    workspace: str | None = None,
    run_id: str | None = None,
    retrieval_mode: str = "off",
    retrieval_mode_explicit: bool = False,
) -> Callable[[Callable[[dict[str, Any]], None]], dict[str, Any]]:
    """Build a Grok ``version-smoke`` run function for the TUI event source.

    The first Grok slice runs only the locked binary ``--version`` smoke.
    It writes a Grok runtime receipt plus normalized ``events.jsonl``; this
    bridge then streams those already-normalized runtime events into the TUI.
    """
    from pathlib import Path

    from assurance.grok_runtime_adapter import run_grok_version_smoke

    _run_root = Path(run_root)
    _workspace = Path(workspace) if workspace else None

    def _run(on_event: Callable[[dict[str, Any]], None]) -> dict[str, Any]:
        receipt = run_grok_version_smoke(
            run_root=_run_root,
            workspace_path=_workspace,
            run_id=run_id,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
        )
        events_path = Path(str(receipt["artifacts"]["events_path"]))
        with events_path.open("r", encoding="utf-8") as handle:
            for line in handle:
                if line.strip():
                    on_event(json.loads(line))
        return receipt

    return _run


def build_grok_acp_live_run_fn(
    *,
    run_root: str,
    workspace: str | None = None,
    run_id: str | None = None,
    prompt_text: str,
    model_id: str = "lif-fake-deepseek",
    retrieval_mode: str = "off",
    retrieval_mode_explicit: bool = False,
    interactive: bool = False,
    fake_provider: bool = False,
    mcp_servers: list[dict[str, Any]] | None = None,
) -> (
    Callable[[Callable[[dict[str, Any]], None]], dict[str, Any]]
    | tuple[Callable[[Callable[[dict[str, Any]], None]], dict[str, Any]], "queue.Queue[str]", "queue.Queue[str]"]
):
    """Build a Grok ACP live run function for the TUI event source.

    Launches ``grok agent stdio`` in ACP JSON-RPC mode and keeps the
    session alive for multiple prompts (D1.10 multi-prompt loop).
    The TUI sends new prompts via the *prompt_queue*.

    When *interactive* is ``True``, permission requests are routed to
    the TUI for user approval.  Returns ``(run_fn, permission_queue,
    prompt_queue)``.
    """
    from pathlib import Path

    from assurance.grok_runtime_adapter import GrokAcpSession, GrokRunRequest

    _run_root = Path(run_root)
    _workspace = Path(workspace) if workspace else None
    _permission_queue: queue.Queue[str] | None = queue.Queue() if interactive else None
    _prompt_queue: queue.Queue[str | None] = queue.Queue()

    def _run(on_event: Callable[[dict[str, Any]], None]) -> dict[str, Any]:
        import json as _json
        _live_emitted_types: set[str] = set()

        # ── P4.1: Known-safe Grok tool names (inline gate) ──────────────
        _KNOWN_TOOLS: frozenset[str] = frozenset({
            "read_file", "write_file", "edit_file",
            "list_directory", "search_file_content",
            "search_files", "glob", "grep",
            "run_terminal_command", "execute_command",
            "web_search", "web_fetch",
            "read_lints", "replace_in_file",
            "task", "enter_plan_mode", "exit_plan_mode",
            "think", "todo_write",
        })

        def _check_tool_availability(tool_name: str) -> str:
            """Return 'allow' if *tool_name* is known, 'defer' otherwise."""
            if not tool_name:
                return "defer"
            return "allow" if tool_name in _KNOWN_TOOLS else "defer"

        def _acp_event_handler(event: dict[str, Any]) -> str | None:
            _live_emitted_types.add(str(event.get("event_type", "")))
            # ── P4.1: Inline tool availability gate ──
            if event["event_type"] == "tool_proposal":
                payload = event.get("payload", {})
                tool_name = payload.get("tool_name", "")
                # Emit a gate-decision event alongside the tool proposal.
                gate_decision = _check_tool_availability(tool_name)
                on_event({
                    "event_type": "gate_decision", "timestamp": event.get("timestamp", ""),
                    "payload": {
                        "gate_name": "tool_availability",
                        "decision": gate_decision,
                        "reason": f"inline check: {tool_name}",
                        "reference_count": 1,
                    },
                    "redaction": "metadata_only",
                })

            if event["event_type"] == "permission_requested" and _permission_queue is not None:
                on_event(event)
                try:
                    decision: str = _permission_queue.get(timeout=300)
                except queue.Empty:
                    decision = "cancelled"
                return decision
            on_event(event)
            return None

        request = GrokRunRequest(
            run_root=_run_root,
            workspace_path=_workspace or (_run_root / "workspace"),
            mode="acp_smoke",
            run_id=run_id or "RUN-GROK-ACP-TUI-001",
            prompt_text=prompt_text,
            model_id=model_id,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
            fake_provider=fake_provider,
            mcp_servers=mcp_servers or [],
        )

        with GrokAcpSession(request) as session:
            # Send initial prompt.
            session.send_prompt(prompt_text, on_acp_event=_acp_event_handler)

            # D1.10: Multi-prompt loop — wait for follow-up prompts from
            # the TUI.  A ``None`` sentinel in the queue ends the session.
            while interactive:
                try:
                    next_prompt = _prompt_queue.get(timeout=0.5)
                except queue.Empty:
                    # No prompt yet — check if TUI still alive
                    continue
                if next_prompt is None:
                    break  # Session ended by TUI
                session.send_prompt(next_prompt, on_acp_event=_acp_event_handler)

        receipt = session.receipt if session.receipt is not None else {}

        # ── Write minimal session marker alongside Grok artifacts ─────────
        # Grok owns session persistence.  We write one tiny summary file
        # so the TUI session list can discover past runs without a
        # separate index.  This is a read-only view; Grok is the authority.
        try:
            _marker = {
                "session_id": request.run_id,
                "created_at": receipt.get("created_at", ""),
                "first_prompt": prompt_text,
                "turn_count": session.turn_count,
            }
            _marker_path = request.run_root / "session.json"
            _marker_path.write_text(
                _json.dumps(_marker, ensure_ascii=False, sort_keys=True, allow_nan=False),
                encoding="utf-8",
            )
        except Exception:
            pass  # Non-fatal — session list degrades gracefully.

        # Replay normalized events (lifecycle events only).
        _LIVE_EMITTED = {
            "acp_initialize", "acp_session_created",
            "tool_proposal", "tool_completed",
            "permission_requested", "permission_decision",
            "model_output", "text_delta",
        }
        events_path = Path(str(receipt.get("artifacts", {}).get("events_path", "")))
        if events_path.is_file():
            with events_path.open("r", encoding="utf-8") as handle:
                for line in handle:
                    if line.strip():
                        evt = _json.loads(line)
                        event_type = str(evt.get("event_type", ""))
                        if (
                            event_type not in _LIVE_EMITTED
                            or event_type not in _live_emitted_types
                        ):
                            on_event(evt)
        return receipt

    if interactive:
        assert _permission_queue is not None
        return _run, _permission_queue, _prompt_queue
    return _run


def build_deepseek_live_run_fn(
    *,
    run_root: str,
    run_id: str | None = None,
    prompt_text: str,
    credential_target: str = "orz-deepseek/agent",
    timeout_seconds: int = 60,
) -> Callable[[Callable[[dict[str, Any]], None]], dict[str, Any]]:
    """Build a direct DeepSeek run function for the TUI event source."""
    from pathlib import Path

    from assurance.deepseek_runtime_adapter import run_deepseek_direct_smoke

    _run_root = Path(run_root)

    def _run(on_event: Callable[[dict[str, Any]], None]) -> dict[str, Any]:
        receipt = run_deepseek_direct_smoke(
            run_root=_run_root,
            prompt_text=prompt_text,
            credential_target=credential_target,
            run_id=run_id,
            timeout_seconds=timeout_seconds,
        )
        events_path = Path(str(receipt["artifacts"]["events_path"]))
        with events_path.open("r", encoding="utf-8") as handle:
            for line in handle:
                if line.strip():
                    on_event(json.loads(line))
        return receipt

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
        text=payload.get("text", ""),
        turn=payload.get("turn", 0),
    )


def _make_user_message(payload: dict[str, Any], timestamp: str) -> UserMessageEvent:
    return UserMessageEvent(
        timestamp=timestamp,
        text=payload.get("text", ""),
        turn=payload.get("turn", 0),
    )


def _make_text_delta(payload: dict[str, Any], timestamp: str) -> TextDeltaEvent:
    return TextDeltaEvent(
        timestamp=timestamp,
        text=payload.get("text", ""),
        turn=payload.get("turn", 0),
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
        severity=payload.get("severity", "error"),
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


def _make_checklist_item_status(payload: dict[str, Any], timestamp: str) -> ChecklistItemStatusEvent:
    """Factory for ``checklist_item_status_changed`` events (GAK-PLAN-001)."""
    return ChecklistItemStatusEvent(
        timestamp=timestamp,
        step_id=payload.get("step_id", ""),
        status=payload.get("status", "todo"),
    )


# ── ACP / Grok runtime factories ──────────────────────────────────────────────


def _make_acp_initialize(payload: dict[str, Any], timestamp: str) -> AcpInitializeEvent:
    return AcpInitializeEvent(
        timestamp=timestamp,
        protocol_version=payload.get("protocol_version", 0),
    )


def _make_acp_session_created(payload: dict[str, Any], timestamp: str) -> AcpSessionCreatedEvent:
    return AcpSessionCreatedEvent(
        timestamp=timestamp,
        session_id_hash=payload.get("session_id_hash", ""),
    )


def _make_tool_proposal(payload: dict[str, Any], timestamp: str) -> ToolProposalEvent:
    return ToolProposalEvent(
        timestamp=timestamp,
        tool_name=payload.get("tool_name", ""),
        tool_call_id=payload.get("tool_call_id", ""),
        input_summary=payload.get("input_summary", ""),
    )


def _make_tool_completed(payload: dict[str, Any], timestamp: str) -> ToolCompletedEvent:
    return ToolCompletedEvent(
        timestamp=timestamp,
        tool_name=payload.get("tool_name", ""),
        status=payload.get("status", "success"),
    )


def _make_permission_requested(payload: dict[str, Any], timestamp: str) -> PermissionRequestedEvent:
    return PermissionRequestedEvent(
        timestamp=timestamp,
        permission=payload.get("permission", payload.get("tool_name", "")),
        options=payload.get("options", []),
    )


def _make_permission_decision(payload: dict[str, Any], timestamp: str) -> PermissionDecisionEvent:
    return PermissionDecisionEvent(
        timestamp=timestamp,
        permission=payload.get("permission", ""),
        decision=payload.get("decision", payload.get("outcome", "")),
        decision_source=payload.get("decision_source", "adapter"),
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
    "user_message": _make_user_message,
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
    "checklist_item_status_changed": _make_checklist_item_status,
    # ── Grok / ACP events ──
    "acp_initialize": _make_acp_initialize,
    "acp_session_created": _make_acp_session_created,
    "tool_proposal": _make_tool_proposal,
    "tool_completed": _make_tool_completed,
    "permission_requested": _make_permission_requested,
    "permission_decision": _make_permission_decision,
    "text_delta": _make_text_delta,
}


_KIND_MAP: dict[str, TuiEventKind] = {
    "run_preflight": TuiEventKind.RUN_PREFLIGHT,
    "instruction_provenance_gate": TuiEventKind.INSTRUCTION_PROVENANCE_GATE,
    "tool_availability_check": TuiEventKind.TOOL_AVAILABILITY_CHECK,
    "orientation_checkpoint": TuiEventKind.ORIENTATION_CHECKPOINT,
    "run_started": TuiEventKind.RUN_STARTED,
    "gate_decision": TuiEventKind.GATE_DECISION,
    "model_request": TuiEventKind.MODEL_REQUEST,
    "user_message": TuiEventKind.USER_MESSAGE,
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
    # ── Grok / ACP events ──
    "acp_initialize": TuiEventKind.ACP_INITIALIZE,
    "acp_session_created": TuiEventKind.ACP_SESSION_CREATED,
    "tool_proposal": TuiEventKind.TOOL_PROPOSAL,
    "tool_completed": TuiEventKind.TOOL_COMPLETED,
    "permission_requested": TuiEventKind.PERMISSION_REQUESTED,
    "permission_decision": TuiEventKind.PERMISSION_DECISION,
    "text_delta": TuiEventKind.TEXT_DELTA,
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
