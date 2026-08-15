"""Probe-accuracy audit for v0.2 run journals.

ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条7): the tool probe's
target is "real changes flip, misjudgments are minimal"; flips are
journaled as `tool_availability_check` and the actual request header as
`request_header_change`. This module turns the journal into OBSERVATIONS:

- `false_complete_candidates`: a tool was complete at the last probe, then a
  main-lane call for it ended in ToolCompleted(error) without a prior probe
  flip to incomplete. Call-time gate codes (permission denials, candidate
  caps, injection budget, …) are excluded — design invariant 2 makes the
  call-time mechanical gate the final backstop, so those are NOT probe
  misjudgments; the remaining codes are candidates for a probe-chain bug.
- `false_incomplete_candidates`: a tool was incomplete at the last probe,
  yet a main-lane ToolStarted for it arrived without a prior flip to
  complete (a probe snapshot went stale — usually benign, but audit-worthy).
- `flip_without_header_change`: a probe flip was NOT followed by a
  main-lane `request_header_change(reason=change)` before the next
  `model_output` — the projected list changed without a real request-header
  change (stale flip / probe bug).

This is an audit, not a hard gate: races between probe time and call time
are legal, so callers classify candidates against real runs.
"""

from __future__ import annotations

from typing import Any


# The 23 main-agent work tools — single source of truth is the reference
# verifier's `_WORK_TOOLS` (2026-08-15 review fix: duplicated lists drift).
try:  # package context (pytest / check_repository)
    from .run_event_journal_validation import _WORK_TOOLS as WORK_TOOLS
except ImportError:  # pragma: no cover — direct script execution
    from run_event_journal_validation import _WORK_TOOLS as WORK_TOOLS

# ToolCompleted(error) codes that represent the call-time mechanical gate,
# NOT a probe misjudgment (design invariant 2 — the probe never promises
# call success). A false-complete candidate whose code is here is expected.
GATE_ONLY_CODES = frozenset(
    {
        "permission_denied",
        "missing_test_runner",
        "retrieval_mode_off",
        "retrieval_role_write_denied",
        "retrieval_role_shell_denied",
        "retrieval_role_execution_denied",
        "nested_subagent_dispatch_refused",
        "control_tool_lane_denied",
        "web_fetch_candidate_cap_exceeded",
        "web_fetch_candidate_url_missing",
        "web_fetch_candidate_count_unbound",
        "browser_read_candidate_cap_exceeded",
        "browser_read_candidate_url_missing",
        "browser_read_candidate_count_unbound",
        "round_inject_budget_exceeded",
    }
)


def _is_gate_only(error: str) -> bool:
    """Exact gate-only codes plus the fail-closed control-ticket prefix.
    `refuse_ticketed_tool` emits `control_ticket_rejected:<reject_code>`
    (e.g. missing_goal_context / missing_target_argument /
    missing_snapshot_store) BEFORE ToolStarted, so every such completion is
    a call-time mechanical gate, never a probe misjudgment (2026-08-15
    review fix — the prefix keeps the audit in sync with the whole
    fail-closed reject-code surface without re-listing each code)."""
    return error in GATE_ONLY_CODES or error.startswith("control_ticket_rejected:")


def _payload(event: dict[str, Any]) -> dict[str, Any]:
    return event.get("payload", {})


def audit_probe_accuracy(events: list[dict[str, Any]]) -> dict[str, Any]:
    """Scan a v0.2 journal (loaded event dicts) and return observations.

    Only main-lane tool events are considered: `tool_availability_check` is
    main-lane by construction, and main-lane `tool_started`/`tool_completed`
    carry no `target` (retrieval-lane tool events carry `target`).
    """
    observations: dict[str, Any] = {
        "flips": [],
        "false_complete_candidates": [],
        "false_incomplete_candidates": [],
        "flip_without_header_change": [],
    }
    last_complete: frozenset[str] | None = None
    last_incomplete: frozenset[str] = frozenset()
    pending_flip: tuple[int, frozenset[str], frozenset[str]] | None = None

    for index, event in enumerate(events):
        if event.get("payload_schema") != "run-event-v0.2.schema.json":
            continue
        event_type = event.get("event_type")
        payload = _payload(event)

        if event_type == "tool_availability_check":
            complete = frozenset(payload.get("complete", []))
            incomplete = frozenset(
                item.get("tool") for item in payload.get("incomplete", [])
            )
            if last_complete is not None and complete != last_complete:
                flip: tuple[int, frozenset[str], frozenset[str]] = (
                    index,
                    last_complete,
                    complete,
                )
                observations["flips"].append(flip)
                pending_flip = flip
            last_complete = complete
            last_incomplete = incomplete
            continue

        if event_type == "request_header_change":
            # Note (2026-08-15 review): a main-lane `initial` deliberately
            # does NOT clear a pending flip — currently unreachable (an
            # `initial` only precedes the first flip of a loop and the seed
            # flip is ignored above), so no false positive exists.
            if (
                pending_flip is not None
                and payload.get("agent_role") == "main"
                and payload.get("reason") == "change"
            ):
                pending_flip = None
            continue

        if event_type == "tool_started" and payload.get("target") is None:
            tool = payload.get("tool")
            if (
                tool in WORK_TOOLS
                and tool in last_incomplete
                and tool not in (last_complete or frozenset())
            ):
                observations["false_incomplete_candidates"].append((index, tool))
            continue

        if event_type == "tool_completed" and payload.get("target") is None:
            tool = payload.get("tool")
            if tool not in WORK_TOOLS or payload.get("status") != "error":
                continue
            error = payload.get("error") or ""
            if _is_gate_only(error):
                continue
            if (
                last_complete is not None
                and tool in last_complete
                and tool not in last_incomplete
            ):
                observations["false_complete_candidates"].append(
                    (index, tool, error)
                )
            continue

        if event_type == "model_output" and pending_flip is not None:
            observations["flip_without_header_change"].append(
                (pending_flip[0], index)
            )
            pending_flip = None

    return observations
