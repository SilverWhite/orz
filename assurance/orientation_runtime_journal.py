from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Sequence

from jsonschema import Draft202012Validator, FormatChecker

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .orientation_runtime_integration import (
    ORIENTATION_NAME,
    RECEIPT_NAME as INTEGRATION_RECEIPT_NAME,
    STAGNATION_NAME,
    verify_orientation_stagnation_integration_fixture,
)
from .utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
)


ROOT = ASSURANCE_ROOT.parent
RUNTIME_ROOT = ROOT / "runtime"
JOURNAL_RECEIPT_SCHEMA = "orientation-stagnation-journal-receipt-v0.1.schema.json"
ORIENTATION_PAYLOAD_SCHEMA = "orientation-checkpoint-event-payload-v0.1.schema.json"
STAGNATION_PAYLOAD_SCHEMA = "runtime-stagnation-guard-event-payload-v0.1.schema.json"
RUN_ID = "RUN-ORIENTATION-STAGNATION-JOURNAL"
CREATED_AT = "2026-07-26T00:00:00Z"
RUN_MANIFEST_NAME = "run-manifest.json"
JOURNAL_NAME = "events.jsonl"
JOURNAL_RECEIPT_NAME = "orientation-stagnation-journal-receipt.json"
MODEL_OR_TOOL_EVENTS = {
    "model_request",
    "model_output",
    "tool_proposal",
    "permission_decision",
    "tool_started",
    "tool_completed",
}


def _validate_runtime_contract(value: dict[str, Any], schema_name: str, *, label: str) -> None:
    schema = load_json(RUNTIME_ROOT / schema_name)
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda item: list(item.absolute_path),
    )
    if not errors:
        return
    rendered = [
        f"{label}#/{'/'.join(map(str, item.absolute_path))}: {item.message}"
        for item in errors
    ]
    raise AssuranceError("; ".join(rendered))


def _event_hash(event: dict[str, Any]) -> str:
    return sha256_bytes(
        canonical_bytes({key: value for key, value in event.items() if key != "event_sha256"})
    )


def _event_line(event: dict[str, Any]) -> bytes:
    return (
        json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
    )


def _journal_bytes(events: Sequence[dict[str, Any]]) -> bytes:
    return b"".join(_event_line(event) for event in events)


def _zero_digest(label: str) -> str:
    return sha256_bytes(label.encode("utf-8"))


def _runtime_manifest(integration_receipt: dict[str, Any]) -> dict[str, Any]:
    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "immutable_precommit",
        "run_id": RUN_ID,
        "created_at": CREATED_AT,
        "execution_mode": "development",
        "scenario_export": {
            "export_id": "EXP-ORIENTATION-STAGNATION-JOURNAL",
            "manifest_sha256": integration_receipt["fixture_sha256"],
            "bundle_sha256": sha256_bytes(canonical_bytes(integration_receipt["artifacts"])),
            "leak_scan_report_sha256": sha256_bytes(canonical_bytes(integration_receipt["checks"])),
        },
        "adapter": {
            "provider": "none",
            "model_id": "no-model",
            "adapter_id": "orientation-stagnation-journal-fixture",
            "adapter_version": "0.1.0-draft",
            "capabilities": {
                "structured_output": True,
                "tool_calling": False,
                "streaming": False,
                "seed_control": False,
                "cancellation": False,
                "context_limit_tokens": 1,
            },
        },
        "prompts": {
            "system_prompt_sha256": _zero_digest("no model prompt"),
            "project_rules_sha256": sha256_file(ROOT / "README.md"),
            "generated_context_sha256": sha256_bytes(canonical_bytes(integration_receipt)),
        },
        "tools": {
            "allowlist": [],
            "policy_sha256": _zero_digest("no tools"),
            "permission_mode": "deny_by_default",
            "filesystem_profile": "custom",
            "network_profile": "disabled",
        },
        "isolation": {
            "memory": "disabled",
            "historical_case_retrieval": "disabled",
            "oracle_mounted": False,
            "reviewer_fixtures_mounted": False,
            "inherited_user_config": False,
            "writable_roots": ["journal-output-root"],
        },
        "budgets": {
            "wall_time_seconds": 1,
            "max_turns": 1,
            "max_input_tokens": 1,
            "max_output_tokens": 1,
            "max_tool_calls": 0,
        },
        "randomness": {
            "seed": None,
            "temperature": 0,
            "retry_policy": "none",
        },
        "protocol_digests": {
            "agent_protocol": sha256_file(ROOT / "protocol" / "PROTOCOL_DRAFT_v0.1.md"),
            "reason_codes": sha256_file(ROOT / "protocol" / "reason-codes-v0.1.yaml"),
            "gate_matrix": sha256_file(ROOT / "protocol" / "gate-matrix-v0.1.yaml"),
            "scoring_protocol": sha256_file(ROOT / "evaluation" / "SCORING_PROTOCOL_v0.1.md"),
            "output_schema": sha256_file(ASSURANCE_ROOT / JOURNAL_RECEIPT_SCHEMA),
        },
        "journal_policy": {
            "format": "jsonl",
            "event_schema_version": "0.1.0-draft",
            "canonicalization": "RFC8785",
            "hash_chain": "sha256",
            "redaction_policy_sha256": _zero_digest("metadata only"),
        },
        "notes": [
            "No-model journal projection for orientation/stagnation runner event shape.",
            "No real runner, model, tool, network, or counterexample queue is attached.",
        ],
    }
    _validate_runtime_contract(manifest, "run-manifest-v0.1.schema.json", label="orientation runtime manifest")
    return manifest


def _event(
    *,
    run_manifest_sha256: str,
    sequence: int,
    event_type: str,
    payload_schema: str,
    payload: dict[str, Any],
    previous_event_sha256: str | None,
) -> dict[str, Any]:
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": RUN_ID,
        "event_id": f"EVT-ORIENTATION-STAGNATION-{sequence:03d}",
        "sequence": sequence,
        "timestamp": CREATED_AT,
        "event_type": event_type,
        "run_manifest_sha256": run_manifest_sha256,
        "previous_event_sha256": previous_event_sha256,
        "payload_schema": payload_schema,
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": "metadata_only",
        "event_sha256": "",
    }
    event["event_sha256"] = _event_hash(event)
    _validate_runtime_contract(event, "run-event-v0.1.schema.json", label=f"orientation runtime event {sequence}")
    return event


def _build_events(
    *,
    integration_root: Path,
    integration_receipt: dict[str, Any],
    run_manifest_sha256: str,
) -> list[dict[str, Any]]:
    orientation_path = integration_root / ORIENTATION_NAME
    stagnation_path = integration_root / STAGNATION_NAME
    orientation = load_json(orientation_path)
    stagnation = load_json(stagnation_path)
    orientation_payload = {
        "checkpoint_id": orientation["checkpoint_id"],
        "orientation_checkpoint_sha256": sha256_file(orientation_path),
        "task_id": integration_receipt["task_id"],
        "step_index": integration_receipt["step_index"],
        "neutral_orientation_only": True,
        "counterexample_queue_invoked": False,
        "claim_strength_effect": "none",
    }
    validate_contract(
        orientation_payload,
        ORIENTATION_PAYLOAD_SCHEMA,
        label="orientation checkpoint event payload",
    )
    restart_packet = stagnation.get("restart_packet")
    stagnation_payload = {
        "runtime_stagnation_guard_receipt_sha256": sha256_file(stagnation_path),
        "decision": stagnation["decision"],
        "action": stagnation["action"],
        "reason_codes": stagnation["reason_codes"],
        "public_output_count": stagnation["metrics"]["public_output_count"],
        "retry_count": stagnation["thresholds"]["retry_count"],
        "retry_budget": stagnation["thresholds"]["retry_budget"],
        "restart_packet_sha256": (
            sha256_bytes(canonical_bytes(restart_packet))
            if restart_packet is not None
            else None
        ),
        "public_output_only": True,
        "asks_model_if_stuck": False,
        "hidden_chain_of_thought_saved": False,
    }
    validate_contract(
        stagnation_payload,
        STAGNATION_PAYLOAD_SCHEMA,
        label="runtime stagnation guard event payload",
    )
    terminal_type = (
        "run_finished" if stagnation["decision"] == "continue" else "run_invalidated"
    )
    terminal_payload = {
        "status": "completed" if terminal_type == "run_finished" else "invalidated",
        "stagnation_decision": stagnation["decision"],
        "formal_runner_claimed": False,
        "restart_packet_retains_runaway_suffix": False,
    }
    specs = [
        (
            "run_preflight",
            "orientation-stagnation-preflight-v0.1",
            {
                "integration_receipt_sha256": sha256_file(
                    integration_root / INTEGRATION_RECEIPT_NAME
                ),
                "runner_attached": False,
                "model_invoked": False,
                "network_requested": False,
            },
        ),
        (
            "run_started",
            "orientation-stagnation-run-started-v0.1",
            {
                "task_id": integration_receipt["task_id"],
                "step_index": integration_receipt["step_index"],
            },
        ),
        (
            "orientation_checkpoint",
            ORIENTATION_PAYLOAD_SCHEMA,
            orientation_payload,
        ),
        (
            "runtime_stagnation_guard",
            STAGNATION_PAYLOAD_SCHEMA,
            stagnation_payload,
        ),
        (terminal_type, "orientation-stagnation-terminal-v0.1", terminal_payload),
    ]
    events: list[dict[str, Any]] = []
    previous: str | None = None
    for sequence, (event_type, payload_schema, payload) in enumerate(specs):
        item = _event(
            run_manifest_sha256=run_manifest_sha256,
            sequence=sequence,
            event_type=event_type,
            payload_schema=payload_schema,
            payload=payload,
            previous_event_sha256=previous,
        )
        events.append(item)
        previous = item["event_sha256"]
    return events


def _load_events(journal_path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line_number, line in enumerate(journal_path.read_text(encoding="utf-8").splitlines(), 1):
        if not line:
            raise AssuranceError(f"orientation runtime journal has empty line: {line_number}")
        try:
            event = json.loads(line)
        except json.JSONDecodeError as exc:
            raise AssuranceError(f"orientation runtime journal line {line_number} is invalid JSON: {exc}") from exc
        _validate_runtime_contract(event, "run-event-v0.1.schema.json", label=f"orientation runtime journal line {line_number}")
        events.append(event)
    return events


def _verify_events(events: list[dict[str, Any]], *, run_manifest_sha256: str) -> None:
    expected = [
        "run_preflight",
        "run_started",
        "orientation_checkpoint",
        "runtime_stagnation_guard",
    ]
    if len(events) != 5:
        raise AssuranceError("orientation runtime journal event count mismatch")
    event_types = [event["event_type"] for event in events]
    if event_types[:4] != expected:
        raise AssuranceError("orientation runtime journal event order mismatch")
    if event_types[-1] not in {"run_finished", "run_invalidated"}:
        raise AssuranceError("orientation runtime journal terminal event mismatch")
    previous: str | None = None
    for sequence, event in enumerate(events):
        if event["sequence"] != sequence:
            raise AssuranceError("orientation runtime journal sequence mismatch")
        if event["run_id"] != RUN_ID:
            raise AssuranceError("orientation runtime journal run_id mismatch")
        if event["run_manifest_sha256"] != run_manifest_sha256:
            raise AssuranceError("orientation runtime journal manifest digest mismatch")
        if event["previous_event_sha256"] != previous:
            raise AssuranceError("orientation runtime journal hash chain mismatch")
        if event["payload_sha256"] != sha256_bytes(canonical_bytes(event["payload"])):
            raise AssuranceError("orientation runtime journal payload digest mismatch")
        if event["event_sha256"] != _event_hash(event):
            raise AssuranceError("orientation runtime journal event digest mismatch")
        if event["redaction"] != "metadata_only":
            raise AssuranceError("orientation runtime journal redaction mismatch")
        if event["event_type"] in MODEL_OR_TOOL_EVENTS:
            raise AssuranceError("orientation runtime journal model/tool event forbidden")
        previous = event["event_sha256"]
    if event_types.index("orientation_checkpoint") > event_types.index("runtime_stagnation_guard"):
        raise AssuranceError("orientation checkpoint must precede stagnation guard")
    validate_contract(
        events[event_types.index("orientation_checkpoint")]["payload"],
        ORIENTATION_PAYLOAD_SCHEMA,
        label="orientation checkpoint event payload",
    )
    validate_contract(
        events[event_types.index("runtime_stagnation_guard")]["payload"],
        STAGNATION_PAYLOAD_SCHEMA,
        label="runtime stagnation guard event payload",
    )


def _build_receipt(
    *,
    integration_root: Path,
    run_manifest_path: Path,
    journal_path: Path,
    events: list[dict[str, Any]],
) -> dict[str, Any]:
    integration_receipt_path = integration_root / INTEGRATION_RECEIPT_NAME
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "orientation_stagnation_runtime_journal_receipt",
        "valid": True,
        "run_id": RUN_ID,
        "integration_receipt_sha256": sha256_file(integration_receipt_path),
        "run_manifest_sha256": sha256_file(run_manifest_path),
        "journal_sha256": sha256_file(journal_path),
        "event_count": len(events),
        "event_types": [event["event_type"] for event in events],
        "terminal_event": events[-1]["event_type"],
        "checks": {
            "journal_hash_chain_valid": True,
            "orientation_before_stagnation": True,
            "payload_schemas_valid": True,
            "no_model_or_tool_events": True,
            "metadata_only": True,
            "runner_attached": False,
            "counterexample_queue_invoked": False,
        },
        "limitations": [
            "This is a no-model journal projection, not a real runner attachment.",
            "No model, tool, network, hidden chain-of-thought, or counterexample queue was used.",
        ],
    }
    validate_contract(
        receipt,
        JOURNAL_RECEIPT_SCHEMA,
        label="orientation/stagnation runtime journal receipt",
    )
    return receipt


def write_orientation_stagnation_runtime_journal(
    *,
    integration_root: Path,
    journal_root: Path,
) -> dict[str, Any]:
    if journal_root.exists() and any(journal_root.iterdir()):
        raise AssuranceError(f"journal root must be empty or absent: {journal_root}")
    integration_receipt = verify_orientation_stagnation_integration_fixture(
        output_root=integration_root
    )
    manifest = _runtime_manifest(integration_receipt)
    journal_root.mkdir(parents=True, exist_ok=True)
    manifest_path = journal_root / RUN_MANIFEST_NAME
    journal_path = journal_root / JOURNAL_NAME
    receipt_path = journal_root / JOURNAL_RECEIPT_NAME
    atomic_write_json(manifest_path, manifest)
    run_manifest_sha256 = sha256_file(manifest_path)
    events = _build_events(
        integration_root=integration_root,
        integration_receipt=integration_receipt,
        run_manifest_sha256=run_manifest_sha256,
    )
    atomic_write_bytes(journal_path, _journal_bytes(events))
    receipt = _build_receipt(
        integration_root=integration_root,
        run_manifest_path=manifest_path,
        journal_path=journal_path,
        events=events,
    )
    atomic_write_json(receipt_path, receipt)
    return receipt


def verify_orientation_stagnation_runtime_journal(
    *,
    integration_root: Path,
    journal_root: Path,
) -> dict[str, Any]:
    integration_receipt = verify_orientation_stagnation_integration_fixture(
        output_root=integration_root
    )
    manifest_path = journal_root / RUN_MANIFEST_NAME
    journal_path = journal_root / JOURNAL_NAME
    receipt_path = journal_root / JOURNAL_RECEIPT_NAME
    observed_manifest = load_json(manifest_path)
    expected_manifest = _runtime_manifest(integration_receipt)
    if canonical_bytes(observed_manifest) != canonical_bytes(expected_manifest):
        raise AssuranceError("orientation runtime manifest does not rebuild")
    run_manifest_sha256 = sha256_file(manifest_path)
    events = _load_events(journal_path)
    _verify_events(events, run_manifest_sha256=run_manifest_sha256)
    expected_receipt = _build_receipt(
        integration_root=integration_root,
        run_manifest_path=manifest_path,
        journal_path=journal_path,
        events=events,
    )
    observed_receipt = load_json(receipt_path)
    validate_contract(
        observed_receipt,
        JOURNAL_RECEIPT_SCHEMA,
        label="orientation/stagnation runtime journal receipt",
    )
    if canonical_bytes(observed_receipt) != canonical_bytes(expected_receipt):
        raise AssuranceError("orientation runtime journal receipt does not rebuild")
    return expected_receipt
