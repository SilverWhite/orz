from __future__ import annotations

from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .orientation_runtime_guard import (
    build_orientation_checkpoint,
    evaluate_runtime_stagnation_guard,
)
from .utils import atomic_write_json, canonical_bytes, load_json, sha256_file


FIXTURE_SCHEMA = "orientation-stagnation-integration-fixture-v0.1.schema.json"
RECEIPT_SCHEMA = "orientation-stagnation-integration-receipt-v0.1.schema.json"

INPUT_NAME = "fixture-input.json"
ORIENTATION_NAME = "orientation-checkpoint.json"
STAGNATION_NAME = "runtime-stagnation-guard-receipt.json"
RECEIPT_NAME = "orientation-stagnation-integration-receipt.json"


def _load_and_validate_fixture(path: Path) -> dict[str, Any]:
    fixture = load_json(path)
    validate_contract(
        fixture,
        FIXTURE_SCHEMA,
        label="orientation/stagnation integration fixture",
    )
    return fixture


def _build_artifacts(fixture: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    thresholds = fixture.get("thresholds", {})
    orientation = build_orientation_checkpoint(
        task_id=fixture["task_id"],
        trigger_step=fixture["step_index"],
        task_contract_sha256=fixture["task_contract_sha256"],
    )
    stagnation = evaluate_runtime_stagnation_guard(
        public_outputs=fixture["public_outputs"],
        retry_count=fixture["retry_count"],
        retry_budget=fixture["retry_budget"],
        repeated_content_threshold=thresholds.get("repeated_content_threshold", 10),
        ngram_repeat_threshold=thresholds.get("ngram_repeat_threshold", 10),
        progress_markers=fixture.get("progress_markers", []),
    )
    return orientation, stagnation


def _build_receipt(
    *,
    fixture: dict[str, Any],
    fixture_path: Path,
    output_root: Path,
    orientation_path: Path,
    stagnation_path: Path,
) -> dict[str, Any]:
    orientation = load_json(orientation_path)
    stagnation = load_json(stagnation_path)
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "orientation_stagnation_integration_receipt",
        "valid": True,
        "fixture_sha256": sha256_file(fixture_path),
        "task_id": fixture["task_id"],
        "step_index": fixture["step_index"],
        "output_root": str(output_root),
        "artifacts": {
            "orientation_checkpoint": {
                "path": ORIENTATION_NAME,
                "sha256": sha256_file(orientation_path),
            },
            "runtime_stagnation_guard_receipt": {
                "path": STAGNATION_NAME,
                "sha256": sha256_file(stagnation_path),
            },
        },
        "decisions": {
            "stagnation_decision": stagnation["decision"],
            "stagnation_action": stagnation["action"],
            "reason_codes": stagnation["reason_codes"],
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
        },
        "limitations": [
            "No runner was attached.",
            "No real model, network, or hidden chain-of-thought was used.",
            "Counterexample queue remains outside this integration fixture.",
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
    orientation, stagnation = _build_artifacts(fixture)

    output_root.mkdir(parents=True, exist_ok=True)
    fixture_copy = output_root / INPUT_NAME
    orientation_path = output_root / ORIENTATION_NAME
    stagnation_path = output_root / STAGNATION_NAME
    receipt_path = output_root / RECEIPT_NAME

    atomic_write_json(fixture_copy, fixture)
    atomic_write_json(orientation_path, orientation)
    atomic_write_json(stagnation_path, stagnation)
    receipt = _build_receipt(
        fixture=fixture,
        fixture_path=fixture_copy,
        output_root=output_root,
        orientation_path=orientation_path,
        stagnation_path=stagnation_path,
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
    receipt_path = output_root / RECEIPT_NAME

    fixture = _load_and_validate_fixture(fixture_path)
    expected_orientation, expected_stagnation = _build_artifacts(fixture)
    observed_orientation = load_json(orientation_path)
    observed_stagnation = load_json(stagnation_path)
    if canonical_bytes(observed_orientation) != canonical_bytes(expected_orientation):
        raise AssuranceError("orientation checkpoint artifact does not rebuild")
    if canonical_bytes(observed_stagnation) != canonical_bytes(expected_stagnation):
        raise AssuranceError("stagnation guard receipt artifact does not rebuild")

    expected_receipt = _build_receipt(
        fixture=fixture,
        fixture_path=fixture_path,
        output_root=output_root,
        orientation_path=orientation_path,
        stagnation_path=stagnation_path,
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
