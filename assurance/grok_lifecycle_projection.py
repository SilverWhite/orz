from __future__ import annotations

from collections import Counter
from copy import deepcopy
import json
from pathlib import Path
from typing import Any, Iterable

from jsonschema import Draft202012Validator, FormatChecker

from .contracts import validate_contract
from .errors import AssuranceError
from .orientation_runtime_guard import ALLOWED_ORIENTATION_FIELDS, FORBIDDEN_ORIENTATION_FIELDS
from .utils import canonical_bytes, sha256_bytes, utc_now


ROOT = Path(__file__).resolve().parents[1]
RUNTIME_ROOT = ROOT / "runtime"
GLOBAL_PROGRESS_INPUT_SCHEMA = RUNTIME_ROOT / "global-progress-input-v0.1.schema.json"
RECEIPT_SCHEMA = "grok-lifecycle-projection-receipt-v0.1.schema.json"

START_EVENTS = {
    "workflow_started",
    "workflow_step_started",
    "subagent_started",
}
SUCCESS_EVENTS = {
    "workflow_completed",
    "workflow_step_completed",
    "subagent_completed",
}
FAILED_EVENTS = {
    "workflow_failed",
    "workflow_step_failed",
    "subagent_failed",
}
CANCELLED_EVENTS = {
    "workflow_cancelled",
    "workflow_step_cancelled",
    "subagent_cancelled",
}
VERIFICATION_EVENTS = {
    "workflow_verified",
    "workflow_step_verified",
    "subagent_verified",
}
PLAN_EVENTS = {
    "workflow_plan_revised",
}
SUPPORTED_LIFECYCLE_EVENTS = (
    START_EVENTS
    | SUCCESS_EVENTS
    | FAILED_EVENTS
    | CANCELLED_EVENTS
    | VERIFICATION_EVENTS
    | PLAN_EVENTS
)
FORBIDDEN_SOURCE_KEYS = {
    "authorization",
    "hidden_reasoning",
    "messages",
    "prompt",
    "raw_output",
    "raw_payload",
    "tool_input",
    "tool_output",
}


def _schema_errors(instance: dict[str, Any], schema_path: Path) -> list[str]:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(instance), key=lambda item: list(item.path))
    return [
        f"{'/'.join(map(str, error.absolute_path)) or '<root>'}: {error.message}"
        for error in errors
    ]


def _validate_global_progress_input(instance: dict[str, Any]) -> list[str]:
    return _schema_errors(instance, GLOBAL_PROGRESS_INPUT_SCHEMA)


def _next_sequence(journal: list[dict[str, Any]]) -> int:
    if not journal:
        return 0
    return max(int(event["sequence"]) for event in journal) + 1


def _event_id(sequence: int) -> str:
    return f"EVT-GROK-{sequence:06d}"


def _metadata_only(event: dict[str, Any]) -> bool:
    lower_keys = {str(key).casefold() for key in event}
    if lower_keys & FORBIDDEN_SOURCE_KEYS:
        return False
    details = event.get("details")
    if isinstance(details, dict):
        return not ({str(key).casefold() for key in details} & FORBIDDEN_SOURCE_KEYS)
    return True


def _project_one_event(
    source_event: dict[str, Any],
    *,
    sequence: int,
) -> dict[str, Any]:
    event_type = source_event.get("event_type")
    step_id = source_event.get("step_id")
    direction_id = source_event.get("direction_id")
    if event_type in START_EVENTS:
        gps_type = "artifact_registered"
        terminal_state = None
        verification_state = None
    elif event_type in SUCCESS_EVENTS:
        gps_type = "action_terminal"
        terminal_state = "succeeded"
        verification_state = None
    elif event_type in FAILED_EVENTS:
        gps_type = "action_terminal"
        terminal_state = "failed"
        verification_state = None
    elif event_type in CANCELLED_EVENTS:
        gps_type = "action_terminal"
        terminal_state = "cancelled"
        verification_state = None
    elif event_type in VERIFICATION_EVENTS:
        gps_type = "verification_result"
        terminal_state = None
        verification_state = source_event.get("verification_state") or "pass"
    elif event_type in PLAN_EVENTS:
        gps_type = "plan_revised"
        terminal_state = None
        verification_state = None
        step_id = None
        direction_id = None
    else:
        raise AssuranceError(f"unsupported Grok lifecycle event type: {event_type}")

    return {
        "sequence": sequence,
        "event_id": _event_id(sequence),
        "event_type": gps_type,
        "step_id": step_id,
        "direction_id": direction_id,
        "terminal_state": terminal_state,
        "write_effect": bool(source_event.get("write_effect", False)),
        "verification_state": verification_state,
    }


def _step_bindings_valid(
    gps_input: dict[str, Any],
    projected_events: Iterable[dict[str, Any]],
) -> tuple[bool, list[str]]:
    plan_steps = {
        step["step_id"]: step["direction_id"]
        for step in gps_input.get("plan", {}).get("steps", [])
        if isinstance(step, dict)
    }
    errors: list[str] = []
    for event in projected_events:
        step_id = event.get("step_id")
        if step_id is None:
            continue
        if step_id not in plan_steps:
            errors.append(f"projected event {event['event_id']} references unknown step {step_id}")
        elif event.get("direction_id") != plan_steps[step_id]:
            errors.append(
                f"projected event {event['event_id']} direction does not match step {step_id}"
            )
    return not errors, errors


def _orientation_context_block(
    *,
    source_events: list[dict[str, Any]],
    projected_events: list[dict[str, Any]],
) -> str:
    latest = source_events[-1] if source_events else {}
    latest_type = str(latest.get("event_type") or "none")
    latest_subject = str(latest.get("subject_id") or latest.get("step_id") or "none")
    counts = Counter(str(event.get("event_type") or "unknown") for event in source_events)
    count_text = ", ".join(f"{key}={counts[key]}" for key in sorted(counts)) or "none"
    return (
        "[GROK_LIFECYCLE_CONTEXT v0.1]\n"
        f"latest_event_type: {latest_type}\n"
        f"latest_subject: {latest_subject}\n"
        f"source_event_counts: {count_text}\n"
        f"projected_gps_events: {len(projected_events)}\n"
        "claim_policy: neutral_context_only\n"
        "[/GROK_LIFECYCLE_CONTEXT]"
    )


def project_grok_lifecycle_events_to_global_progress(
    *,
    global_progress_input: dict[str, Any],
    lifecycle_events: list[dict[str, Any]],
    observed_at: str | None = None,
) -> dict[str, Any]:
    source_input = deepcopy(global_progress_input)
    projected_input = deepcopy(global_progress_input)
    errors: list[str] = []
    checks = {
        "source_events_metadata_only": True,
        "source_event_types_supported": True,
        "gps_events_schema_valid": True,
        "gps_step_bindings_valid": True,
        "global_progress_input_schema_valid": True,
        "orientation_context_neutral": True,
        "no_model_invoked": True,
        "no_network_requested": True,
        "no_claim_promotion": True,
        "prompt_tool_promotion_blocked": True,
    }

    source_schema_errors = _validate_global_progress_input(source_input)
    if source_schema_errors:
        checks["global_progress_input_schema_valid"] = False
        errors.extend(f"source input invalid: {error}" for error in source_schema_errors)

    projected_events: list[dict[str, Any]] = []
    sequence = _next_sequence(projected_input.get("journal", []))
    for source_event in lifecycle_events:
        if not isinstance(source_event, dict):
            checks["source_event_types_supported"] = False
            errors.append("source lifecycle event is not an object")
            continue
        event_type = source_event.get("event_type")
        if event_type not in SUPPORTED_LIFECYCLE_EVENTS:
            checks["source_event_types_supported"] = False
            errors.append(f"unsupported Grok lifecycle event type: {event_type}")
            continue
        if not _metadata_only(source_event):
            checks["source_events_metadata_only"] = False
            errors.append(f"source lifecycle event contains content-bearing fields: {event_type}")
            continue
        try:
            projected_events.append(_project_one_event(source_event, sequence=sequence))
            sequence += 1
        except AssuranceError as exc:
            checks["source_event_types_supported"] = False
            errors.append(str(exc))

    bindings_valid, binding_errors = _step_bindings_valid(source_input, projected_events)
    if not bindings_valid:
        checks["gps_step_bindings_valid"] = False
        errors.extend(binding_errors)

    projected_input.setdefault("journal", []).extend(projected_events)
    projected_schema_errors = _validate_global_progress_input(projected_input)
    if projected_schema_errors:
        checks["gps_events_schema_valid"] = False
        checks["global_progress_input_schema_valid"] = False
        errors.extend(f"projected input invalid: {error}" for error in projected_schema_errors)

    context_block = _orientation_context_block(
        source_events=lifecycle_events,
        projected_events=projected_events,
    )
    if any(token in context_block for token in FORBIDDEN_ORIENTATION_FIELDS):
        checks["orientation_context_neutral"] = False
        errors.append("orientation context contains forbidden orientation field names")
    valid = all(checks.values()) and not errors
    material = {
        "source": source_input,
        "events": lifecycle_events,
        "projected_events": projected_events,
    }
    projection_id = sha256_bytes(canonical_bytes(material))[:16].upper()
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "grok_lifecycle_projection_receipt",
        "projection_id": f"GROK-LIFE-{projection_id}",
        "valid": valid,
        "decision": "allow" if valid else "block",
        "observed_at": observed_at or utc_now(),
        "task_id": str(source_input.get("task_id") or "TASK-UNKNOWN"),
        "source_global_progress_input_sha256": sha256_bytes(canonical_bytes(source_input)),
        "projected_global_progress_input_sha256": sha256_bytes(
            canonical_bytes(projected_input)
        ),
        "source_event_count": len(lifecycle_events),
        "projected_gps_event_count": len(projected_events),
        "source_event_type_counts": dict(
            sorted(Counter(str(event.get("event_type")) for event in lifecycle_events).items())
        ),
        "gps_journal_events": projected_events,
        "orientation_context_block": context_block,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "This projection maps lifecycle metadata only; it does not infer task correctness.",
            "Grok remains the owner of workflow/subagent execution. GPS and Orientation consume projected metadata only.",
            "Prompt/tool execution remains blocked until containment and promotion gates are satisfied.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="grok lifecycle projection receipt")
    return {
        "global_progress_input": projected_input,
        "receipt": receipt,
        "orientation_context_block": context_block,
    }


def build_grok_lifecycle_orientation_checkpoint(
    *,
    projection_receipt: dict[str, Any],
    task_id: str,
    trigger_step: int,
    task_contract_sha256: str | None = None,
) -> dict[str, Any]:
    validate_contract(
        projection_receipt,
        RECEIPT_SCHEMA,
        label="grok lifecycle projection receipt",
    )
    if trigger_step < 0:
        raise AssuranceError("orientation trigger_step must be non-negative")
    message_block = (
        projection_receipt["orientation_context_block"]
        + "\n\n[ORIENTATION_CHECKPOINT v0.1]\n"
        + "当前正在做什么？\n"
        + "当前任务定位是什么？\n"
        + "下一步输出应该服务哪个用户目标？\n"
        + "[/ORIENTATION_CHECKPOINT]"
    )
    checkpoint = {
        "schema_version": "0.1.0-draft",
        "checkpoint_kind": "orientation_checkpoint",
        "checkpoint_id": f"ORIENT-{task_id}-{trigger_step:04d}",
        "task_id": task_id,
        "trigger": {
            "trigger_type": "fixed_step_interval",
            "step_index": trigger_step,
            "task_contract_sha256": task_contract_sha256,
        },
        "message_block": message_block,
        "allowed_response_fields": ALLOWED_ORIENTATION_FIELDS,
        "forbidden_response_fields": FORBIDDEN_ORIENTATION_FIELDS,
        "claim_policy": {
            "may_generate_counterexample_candidate": False,
            "may_set_claim_disposition": False,
            "claim_strength_effect": "none",
        },
        "notes": [
            "Neutral task orientation with Grok lifecycle metadata.",
            "Lifecycle metadata may describe position and status only; it must not promote claims.",
            "Prompt/tool execution remains blocked by runtime-first promotion gates.",
        ],
    }
    validate_contract(checkpoint, "orientation-checkpoint-v0.1.schema.json", label="orientation checkpoint")
    return checkpoint
