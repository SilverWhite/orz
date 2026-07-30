from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


RUNTIME_ROOT = Path(__file__).resolve().parents[1] / "runtime"
TRANSITION_SCHEMA_NAME = "global-progress-transition-receipt-v0.1.schema.json"
TRANSITION_SCHEMA = RUNTIME_ROOT / TRANSITION_SCHEMA_NAME
INITIAL_STATE = "executing"


def _transition_errors(payload: dict[str, Any]) -> list[str]:
    schema = json.loads(TRANSITION_SCHEMA.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(
            schema,
            format_checker=FormatChecker(),
        ).iter_errors(payload),
        key=lambda item: list(item.absolute_path),
    )
    return [
        f"{'/'.join(map(str, item.absolute_path)) or '<root>'}: {item.message}"
        for item in errors
    ]


def _transition_semantics(payload: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    expected_pair = {
        "step_boundary": ("executing", "reviewing"),
        "task_completion": ("reviewing", "completed"),
    }.get(payload["transition_kind"])
    allowed_pair = expected_pair == (
        payload["state_before"],
        payload["requested_state"],
    )
    applied = payload["transition_applied"]
    if applied:
        if not allowed_pair:
            errors.append("applied transition kind/state pair is not allowed")
        if (
            payload["decision"] != "pass"
            or payload["control_codes"] != ["GPS-TRANSITION-PASS"]
            or payload["state_after"] != payload["requested_state"]
        ):
            errors.append("applied transition has inconsistent pass semantics")
    elif (
        payload["decision"] != "block"
        or "GPS-TRANSITION-PASS" in payload["control_codes"]
        or payload["state_after"] != payload["state_before"]
    ):
        errors.append("blocked transition has inconsistent state semantics")
    elif (
        not allowed_pair
        and payload["control_codes"] != ["GPS-TRANSITION-NOT-ALLOWED"]
    ):
        errors.append("disallowed transition lacks the not-allowed control code")
    elif (
        allowed_pair
        and payload["control_codes"] == ["GPS-TRANSITION-NOT-ALLOWED"]
    ):
        errors.append("allowed transition incorrectly uses the not-allowed control code")
    return errors


def reduce_global_progress_events(
    events: list[dict[str, Any]],
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> dict[str, Any]:
    errors: list[str] = []
    checks = {
        "journal_binding_valid": True,
        "run_started_anchor_valid": True,
        "transition_receipts_valid": True,
        "task_continuity_valid": True,
        "state_continuity_valid": True,
        "terminal_state_valid": True,
    }
    for index, event in enumerate(events):
        if (
            event.get("run_id") != expected_run_id
            or event.get("run_manifest_sha256") != expected_manifest_sha256
        ):
            checks["journal_binding_valid"] = False
            errors.append(f"journal binding mismatch at sequence {index}")

    anchors = [
        event for event in events if event.get("event_type") == "run_started"
    ]
    anchor = anchors[0] if len(anchors) == 1 else None
    if anchor is None:
        checks["run_started_anchor_valid"] = False
        errors.append(
            f"expected exactly one run_started state anchor, found {len(anchors)}"
        )

    current_state: str | None = INITIAL_STATE if anchor is not None else None
    task_id: str | None = None
    transition_count = 0
    applied_transition_count = 0
    last_transition: dict[str, Any] | None = None
    last_applied_checkpoint: dict[str, Any] | None = None
    completed_sequence: int | None = None

    for event in events:
        if event.get("payload_schema") != TRANSITION_SCHEMA_NAME:
            continue
        sequence = event.get("sequence")
        if event.get("event_type") != "gate_decision":
            checks["transition_receipts_valid"] = False
            errors.append(
                f"GPS transition payload uses non-gate event at sequence {sequence}"
            )
            continue
        if anchor is None or sequence <= anchor["sequence"]:
            checks["run_started_anchor_valid"] = False
            errors.append(
                f"GPS transition precedes run_started anchor at sequence {sequence}"
            )
        payload = event.get("payload")
        if not isinstance(payload, dict):
            checks["transition_receipts_valid"] = False
            errors.append(f"GPS transition payload is not an object at sequence {sequence}")
            continue
        payload_errors = _transition_errors(payload)
        semantic_errors = [] if payload_errors else _transition_semantics(payload)
        if payload_errors or semantic_errors:
            checks["transition_receipts_valid"] = False
            for detail in payload_errors + semantic_errors:
                errors.append(f"GPS transition invalid at sequence {sequence}: {detail}")
            continue

        transition_count += 1
        if task_id is None:
            task_id = payload["task_id"]
        elif task_id != payload["task_id"]:
            checks["task_continuity_valid"] = False
            errors.append(f"GPS task changed at sequence {sequence}")
        if current_state != payload["state_before"]:
            checks["state_continuity_valid"] = False
            errors.append(
                "GPS state discontinuity at sequence "
                f"{sequence}: derived {current_state}, event declares "
                f"{payload['state_before']}"
            )
        if completed_sequence is not None:
            checks["terminal_state_valid"] = False
            errors.append(
                f"GPS transition follows completed state at sequence {sequence}"
            )
        current_state = payload["state_after"]
        last_transition = {
            "transition_id": payload["transition_id"],
            "event_id": event["event_id"],
            "event_sha256": event["event_sha256"],
            "sequence": sequence,
            "decision": payload["decision"],
            "state_after": payload["state_after"],
            "checkpoint_id": payload["source_bindings"]["checkpoint_id"],
        }
        if payload["transition_applied"]:
            applied_transition_count += 1
            binding = payload["source_bindings"]
            last_applied_checkpoint = {
                "checkpoint_id": binding["checkpoint_id"],
                "checkpoint_file_sha256": binding["checkpoint_file_sha256"],
                "checkpoint_verification_sha256": (
                    binding["checkpoint_verification_sha256"]
                ),
                "transition_id": payload["transition_id"],
                "event_sha256": event["event_sha256"],
            }
        if current_state == "completed":
            completed_sequence = sequence

    head = events[-1]["event_sha256"] if events else None
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-state-reduction",
        "valid": not errors,
        "run_id": expected_run_id,
        "run_manifest_sha256": expected_manifest_sha256,
        "journal_event_count": len(events),
        "journal_head_sha256": head,
        "state_anchor": (
            {
                "event_id": anchor["event_id"],
                "event_sha256": anchor["event_sha256"],
                "sequence": anchor["sequence"],
                "state": INITIAL_STATE,
            }
            if anchor is not None
            else None
        ),
        "task_id": task_id,
        "current_state": current_state,
        "transition_count": transition_count,
        "applied_transition_count": applied_transition_count,
        "last_transition": last_transition,
        "last_applied_checkpoint": last_applied_checkpoint,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "run_started is the controller-state anchor and deterministically maps to executing.",
            "The last applied checkpoint is gate-accepted, not a scientific correctness claim.",
            "Reduction proves journal mechanics and declared transition continuity only.",
        ],
    }
