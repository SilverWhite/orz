from __future__ import annotations

from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .orientation_runtime_guard import (
    build_orientation_checkpoint,
    evaluate_runtime_stagnation_guard,
    evaluate_tool_belief_stagnation,
)
from .tool_availability_gate import (
    build_tool_availability_gate_receipt,
    probe_tool_availability,
)
from .utils import atomic_write_json, canonical_bytes, load_json, sha256_bytes, sha256_file


FIXTURE_SCHEMA = "orientation-stagnation-integration-fixture-v0.1.schema.json"
RECEIPT_SCHEMA = "orientation-stagnation-integration-receipt-v0.1.schema.json"

INPUT_NAME = "fixture-input.json"
ORIENTATION_NAME = "orientation-checkpoint.json"
STAGNATION_NAME = "runtime-stagnation-guard-receipt.json"
TOOL_AVAIL_REPORT_NAME = "tool-availability-report.json"
TOOL_AVAIL_GATE_NAME = "tool-availability-gate-receipt.json"
TOOL_BELIEF_STAGNATION_NAME = "tool-belief-stagnation-receipt.json"
RECEIPT_NAME = "orientation-stagnation-integration-receipt.json"


def _load_and_validate_fixture(path: Path) -> dict[str, Any]:
    fixture = load_json(path)
    validate_contract(
        fixture,
        FIXTURE_SCHEMA,
        label="orientation/stagnation integration fixture",
    )
    return fixture


def _build_artifacts(fixture: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]]:
    thresholds = fixture.get("thresholds", {})
    tool_specs = fixture.get("tool_specs", [])
    probe_registry = fixture.get("probe_registry")

    if tool_specs and probe_registry is not None:
        tool_report = probe_tool_availability(
            tool_specs=tool_specs,
            runtime_id=fixture["task_id"],
            probe_registry=probe_registry,
        )
        tool_gate_receipt = build_tool_availability_gate_receipt(tool_report)
        tool_availability_sha256 = sha256_bytes(canonical_bytes(tool_report))
    else:
        tool_report = {
            "schema_version": "0.1.0-draft",
            "report_kind": "tool_availability_report",
            "report_id": f"TOOL-AVAIL-{fixture['task_id']}",
            "runtime_id": fixture["task_id"],
            "probe_timestamp": "2026-07-27T00:00:00Z",
            "available": [],
            "unavailable": [],
            "unprobed": [],
            "degraded": [],
            "context_block": "[TOOL_AVAILABILITY v0.1]\nAVAILABLE: (none declared)\nUNAVAILABLE: (none declared)\n[/TOOL_AVAILABILITY]",
            "notes": ["No tool specs provided; tool availability gate skipped."],
        }
        tool_gate_receipt = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "tool_availability_gate_receipt",
            "valid": True,
            "report_id": tool_report["report_id"],
            "report_sha256": sha256_bytes(canonical_bytes(tool_report)),
            "available_count": 0,
            "unavailable_count": 0,
            "unprobed_count": 0,
            "degraded_count": 0,
            "context_block_sha256": sha256_bytes(tool_report["context_block"].encode("utf-8")),
            "decisions": {
                "gate_decision": "defer",
                "context_injected": False,
                "model_must_not_guess": True,
            },
            "checks": {
                "all_declared_tools_probed": False,
                "unavailable_tools_listed": False,
                "no_capability_guessing_permitted": True,
                "mechanical_probe_only": True,
                "no_model_invoked": True,
                "no_network_requested": True,
            },
            "limitations": ["No tool specs provided; tool availability gate skipped."],
        }
        tool_availability_sha256 = None

    tool_belief_receipt = evaluate_tool_belief_stagnation(
        public_outputs=fixture["public_outputs"],
        available_tool_names=fixture.get("available_tool_names", []),
        unavailable_tool_names=fixture.get("unavailable_tool_names", []),
    )

    orientation = build_orientation_checkpoint(
        task_id=fixture["task_id"],
        trigger_step=fixture["step_index"],
        task_contract_sha256=fixture["task_contract_sha256"],
        tool_availability_sha256=tool_availability_sha256,
    )
    stagnation = evaluate_runtime_stagnation_guard(
        public_outputs=fixture["public_outputs"],
        retry_count=fixture["retry_count"],
        retry_budget=fixture["retry_budget"],
        repeated_content_threshold=thresholds.get("repeated_content_threshold", 10),
        ngram_repeat_threshold=thresholds.get("ngram_repeat_threshold", 10),
        progress_markers=fixture.get("progress_markers", []),
    )
    return orientation, stagnation, tool_report, tool_gate_receipt, tool_belief_receipt


def _build_receipt(
    *,
    fixture: dict[str, Any],
    fixture_path: Path,
    output_root: Path,
    orientation_path: Path,
    stagnation_path: Path,
    tool_avail_report_path: Path | None,
    tool_avail_gate_path: Path | None,
    tool_belief_stagnation_path: Path | None,
) -> dict[str, Any]:
    orientation = load_json(orientation_path)
    stagnation = load_json(stagnation_path)
    artifacts: dict[str, Any] = {
        "orientation_checkpoint": {
            "path": ORIENTATION_NAME,
            "sha256": sha256_file(orientation_path),
        },
        "runtime_stagnation_guard_receipt": {
            "path": STAGNATION_NAME,
            "sha256": sha256_file(stagnation_path),
        },
    }
    if tool_avail_report_path is not None and tool_avail_report_path.exists():
        artifacts["tool_availability_report"] = {
            "path": TOOL_AVAIL_REPORT_NAME,
            "sha256": sha256_file(tool_avail_report_path),
        }
    if tool_avail_gate_path is not None and tool_avail_gate_path.exists():
        artifacts["tool_availability_gate_receipt"] = {
            "path": TOOL_AVAIL_GATE_NAME,
            "sha256": sha256_file(tool_avail_gate_path),
        }
    if tool_belief_stagnation_path is not None and tool_belief_stagnation_path.exists():
        artifacts["tool_belief_stagnation_receipt"] = {
            "path": TOOL_BELIEF_STAGNATION_NAME,
            "sha256": sha256_file(tool_belief_stagnation_path),
        }

    reason_codes = list(stagnation["reason_codes"])
    if tool_belief_stagnation_path is not None and tool_belief_stagnation_path.exists():
        belief = load_json(tool_belief_stagnation_path)
        reason_codes.extend(belief["reason_codes"])

    tool_gate_completed = tool_avail_gate_path is not None and tool_avail_gate_path.exists()
    tool_belief_completed = tool_belief_stagnation_path is not None and tool_belief_stagnation_path.exists()

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "orientation_stagnation_integration_receipt",
        "valid": True,
        "fixture_sha256": sha256_file(fixture_path),
        "task_id": fixture["task_id"],
        "step_index": fixture["step_index"],
        "output_root": str(output_root),
        "artifacts": artifacts,
        "decisions": {
            "stagnation_decision": stagnation["decision"],
            "stagnation_action": stagnation["action"],
            "reason_codes": sorted(set(reason_codes)),
        },
        "checks": {
            "orientation_neutral_checkpoint": (
                "[ORIENTATION_CHECKPOINT v0.1]" in orientation["message_block"]
            ),
            "orientation_counterexample_disabled": not orientation["claim_policy"][
                "may_generate_counterexample_candidate"
            ],
            "orientation_claim_disposition_disabled": not orientation["claim_policy"][
                "may_set_claim_disposition"
            ],
            "orientation_no_claim_strength_effect": (
                orientation["claim_policy"]["claim_strength_effect"] == "none"
            ),
            "mechanisms_decoupled": True,
            "stagnation_public_output_only": True,
            "hidden_chain_of_thought_saved": False,
            "runner_attached": False,
            "model_invoked": False,
            "network_requested": False,
            "counterexample_queue_invoked": False,
            "retry_budget_bounded": stagnation["checks"]["retry_budget_bounded"],
            "tool_availability_gate_completed": tool_gate_completed,
            "tool_belief_stagnation_completed": tool_belief_completed,
        },
        "limitations": [
            "No runner was attached.",
            "No real model, network, or hidden chain-of-thought was used.",
            "Counterexample queue remains outside this integration fixture.",
            "Tool availability probe uses static fixture registry, not live runtime hooks.",
        ],
    }
    validate_contract(
        receipt,
        RECEIPT_SCHEMA,
        label="orientation/stagnation integration receipt",
    )
    return receipt


def run_orientation_stagnation_integration_fixture(
    *,
    fixture_path: Path,
    output_root: Path,
) -> dict[str, Any]:
    if output_root.exists() and any(output_root.iterdir()):
        raise AssuranceError(f"output root must be empty or absent: {output_root}")
    fixture = _load_and_validate_fixture(fixture_path)
    orientation, stagnation, tool_report, tool_gate_receipt, tool_belief_receipt = _build_artifacts(fixture)

    output_root.mkdir(parents=True, exist_ok=True)
    fixture_copy = output_root / INPUT_NAME
    orientation_path = output_root / ORIENTATION_NAME
    stagnation_path = output_root / STAGNATION_NAME
    tool_avail_report_path: Path | None = None
    tool_avail_gate_path: Path | None = None
    tool_belief_stagnation_path: Path | None = None
    receipt_path = output_root / RECEIPT_NAME

    atomic_write_json(fixture_copy, fixture)
    atomic_write_json(orientation_path, orientation)
    atomic_write_json(stagnation_path, stagnation)

    tool_specs = fixture.get("tool_specs", [])
    if tool_specs:
        tool_avail_report_path = output_root / TOOL_AVAIL_REPORT_NAME
        tool_avail_gate_path = output_root / TOOL_AVAIL_GATE_NAME
        tool_belief_stagnation_path = output_root / TOOL_BELIEF_STAGNATION_NAME
        atomic_write_json(tool_avail_report_path, tool_report)
        atomic_write_json(tool_avail_gate_path, tool_gate_receipt)
        atomic_write_json(tool_belief_stagnation_path, tool_belief_receipt)

    receipt = _build_receipt(
        fixture=fixture,
        fixture_path=fixture_copy,
        output_root=output_root,
        orientation_path=orientation_path,
        stagnation_path=stagnation_path,
        tool_avail_report_path=tool_avail_report_path,
        tool_avail_gate_path=tool_avail_gate_path,
        tool_belief_stagnation_path=tool_belief_stagnation_path,
    )
    atomic_write_json(receipt_path, receipt)
    return receipt


def verify_orientation_stagnation_integration_fixture(
    *,
    output_root: Path,
) -> dict[str, Any]:
    fixture_path = output_root / INPUT_NAME
    orientation_path = output_root / ORIENTATION_NAME
    stagnation_path = output_root / STAGNATION_NAME
    tool_avail_report_path = output_root / TOOL_AVAIL_REPORT_NAME
    tool_avail_gate_path = output_root / TOOL_AVAIL_GATE_NAME
    tool_belief_stagnation_path = output_root / TOOL_BELIEF_STAGNATION_NAME
    receipt_path = output_root / RECEIPT_NAME

    fixture = _load_and_validate_fixture(fixture_path)
    expected_orientation, expected_stagnation, expected_tool_report, expected_tool_gate, expected_tool_belief = _build_artifacts(fixture)
    observed_orientation = load_json(orientation_path)
    observed_stagnation = load_json(stagnation_path)
    if canonical_bytes(observed_orientation) != canonical_bytes(expected_orientation):
        raise AssuranceError("orientation checkpoint artifact does not rebuild")
    if canonical_bytes(observed_stagnation) != canonical_bytes(expected_stagnation):
        raise AssuranceError("stagnation guard receipt artifact does not rebuild")

    tool_specs = fixture.get("tool_specs", [])
    if tool_specs:
        observed_tool_report = load_json(tool_avail_report_path)
        observed_tool_gate = load_json(tool_avail_gate_path)
        observed_tool_belief = load_json(tool_belief_stagnation_path)
        if canonical_bytes(observed_tool_report) != canonical_bytes(expected_tool_report):
            raise AssuranceError("tool availability report artifact does not rebuild")
        if canonical_bytes(observed_tool_gate) != canonical_bytes(expected_tool_gate):
            raise AssuranceError("tool availability gate receipt artifact does not rebuild")
        if canonical_bytes(observed_tool_belief) != canonical_bytes(expected_tool_belief):
            raise AssuranceError("tool belief stagnation receipt artifact does not rebuild")

    expected_receipt = _build_receipt(
        fixture=fixture,
        fixture_path=fixture_path,
        output_root=output_root,
        orientation_path=orientation_path,
        stagnation_path=stagnation_path,
        tool_avail_report_path=tool_avail_report_path if tool_specs else None,
        tool_avail_gate_path=tool_avail_gate_path if tool_specs else None,
        tool_belief_stagnation_path=tool_belief_stagnation_path if tool_specs else None,
    )
    observed_receipt = load_json(receipt_path)
    validate_contract(
        observed_receipt,
        RECEIPT_SCHEMA,
        label="orientation/stagnation integration receipt",
    )
    if canonical_bytes(observed_receipt) != canonical_bytes(expected_receipt):
        raise AssuranceError("orientation/stagnation integration receipt does not rebuild")
    return expected_receipt
