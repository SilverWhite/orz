from __future__ import annotations

from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .orientation_runtime_integration import (
    run_orientation_stagnation_integration_fixture,
    verify_orientation_stagnation_integration_fixture,
)
from .orientation_runtime_journal import (
    verify_orientation_stagnation_runtime_journal,
    write_orientation_stagnation_runtime_journal,
)
from .runner_public_output import run_runner_public_output_extraction_fixture
from .utils import atomic_write_json, canonical_bytes, load_json, sha256_bytes, sha256_file


FIXTURE_SCHEMA = "deepseek-stream-observation-fixture-v0.1.schema.json"
RECEIPT_SCHEMA = "deepseek-stream-observation-pipeline-receipt-v0.1.schema.json"

FIXTURE_NAME = "fixture-input.json"
STREAM_NAME = "runner-output-stream.input.json"
INTEGRATION_INPUT_NAME = "orientation-stagnation-input.json"
RECEIPT_NAME = "deepseek-stream-observation-pipeline-receipt.json"


def _load_and_validate_fixture(path: Path) -> dict[str, Any]:
    fixture = load_json(path)
    validate_contract(fixture, FIXTURE_SCHEMA, label="DeepSeek stream observation fixture")
    for expected_sequence, chunk in enumerate(fixture["chunks"]):
        if chunk["sequence"] != expected_sequence:
            raise AssuranceError("DeepSeek stream chunks must be contiguous")
    return fixture


def _build_runner_stream(fixture: dict[str, Any]) -> dict[str, Any]:
    records: list[dict[str, Any]] = []
    for chunk in fixture["chunks"]:
        delta = chunk.get("delta", {})
        private_digest = delta.get("reasoning_content_sha256")
        if private_digest is not None:
            records.append(
                {
                    "record_id": f"REC-DS-PRIVATE-{len(records):03d}",
                    "sequence": len(records),
                    "source": "assistant",
                    "channel": "reasoning_private",
                    "visibility": "private_hidden",
                    "content_sha256": private_digest,
                }
            )
        public_text = delta.get("content")
        if public_text is not None:
            records.append(
                {
                    "record_id": f"REC-DS-PUBLIC-{len(records):03d}",
                    "sequence": len(records),
                    "source": "assistant",
                    "channel": "assistant_delta",
                    "visibility": "public",
                    "text": public_text,
                }
            )
        if "finish_reason" in chunk or "usage" in chunk:
            metadata = {
                "chunk_id": chunk["chunk_id"],
                "sequence": chunk["sequence"],
                "finish_reason": chunk.get("finish_reason"),
                "usage": chunk.get("usage"),
            }
            records.append(
                {
                    "record_id": f"REC-DS-METADATA-{len(records):03d}",
                    "sequence": len(records),
                    "source": "runner",
                    "channel": "metadata_event",
                    "visibility": "redacted_metadata",
                    "content_sha256": sha256_bytes(canonical_bytes(metadata)),
                }
            )
    if not records:
        raise AssuranceError("DeepSeek stream fixture produced no runner records")
    return {
        "schema_version": "0.1.0-draft",
        "stream_kind": "runner_public_output_stream_fixture",
        "stream_id": fixture["stream_id"],
        "records": records,
        "extraction_policy": {
            "public_assistant_channels": ["assistant_delta", "assistant_final"],
            "private_channels_forbidden": True,
            "restart_packet_source_policy": {
                "included_state": [
                    "task_contract",
                    "verified_artifact_ledger",
                    "unresolved_questions",
                    "last_valid_checkpoint_digest",
                ],
                "runaway_suffix_allowed": False,
                "hidden_reasoning_allowed": False,
            },
        },
    }


def _build_integration_fixture(
    *,
    fixture: dict[str, Any],
    public_outputs: list[str],
) -> dict[str, Any]:
    return {
        "schema_version": "0.1.0-draft",
        "fixture_kind": "orientation_stagnation_integration_fixture",
        "task_id": fixture["task_id"],
        "task_contract_sha256": fixture["task_contract_sha256"],
        "step_index": fixture["step_index"],
        "public_outputs": public_outputs,
        "retry_count": fixture["retry_count"],
        "retry_budget": fixture["retry_budget"],
        "thresholds": fixture["thresholds"],
        "progress_markers": fixture["progress_markers"],
    }


def _build_receipt(
    *,
    fixture_path: Path,
    output_root: Path,
    public_output_root: Path,
    integration_root: Path,
    journal_root: Path,
    extraction: dict[str, Any],
    integration: dict[str, Any],
    journal: dict[str, Any],
) -> dict[str, Any]:
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "deepseek_stream_observation_pipeline_receipt",
        "valid": True,
        "source_fixture": {
            "path": FIXTURE_NAME,
            "sha256": sha256_file(fixture_path),
        },
        "artifacts": {
            "runner_public_output_extraction": {
                "path": str(public_output_root.relative_to(output_root)),
                "receipt_sha256": sha256_file(
                    public_output_root / "runner-public-output-extraction-receipt.json"
                ),
            },
            "orientation_stagnation_integration": {
                "path": str(integration_root.relative_to(output_root)),
                "receipt_sha256": sha256_file(
                    integration_root / "orientation-stagnation-integration-receipt.json"
                ),
            },
            "orientation_stagnation_journal": {
                "path": str(journal_root.relative_to(output_root)),
                "receipt_sha256": sha256_file(
                    journal_root / "orientation-stagnation-journal-receipt.json"
                ),
            },
        },
        "decisions": {
            "public_output_count": extraction["counts"]["public_output_count"],
            "private_hidden_excluded_count": extraction["counts"][
                "private_hidden_excluded_count"
            ],
            "stagnation_decision": integration["decisions"]["stagnation_decision"],
            "stagnation_action": integration["decisions"]["stagnation_action"],
            "terminal_event_type": journal["terminal_event"],
        },
        "checks": {
            "provider_stream_shape": "deepseek-chat-completions-stream",
            "real_stream_captured": False,
            "model_invoked": False,
            "network_requested": False,
            "stream_sequence_contiguous": True,
            "private_reasoning_text_recorded": False,
            "hidden_chain_of_thought_saved": False,
            "public_output_only_for_restart": (
                not extraction["restart_packet_source_policy"]["hidden_reasoning_allowed"]
                and not extraction["restart_packet_source_policy"]["runaway_suffix_allowed"]
            ),
            "orientation_counterexample_disabled": integration["checks"][
                "orientation_counterexample_disabled"
            ],
        },
        "limitations": [
            "This fixture uses DeepSeek-shaped stream chunks; it does not capture a real streaming API response.",
            "Private reasoning is represented by digest only before runner public-output extraction.",
            "The repeated-output fixture exercises restart projection, not production stream cancellation.",
        ],
    }
    validate_contract(
        receipt,
        RECEIPT_SCHEMA,
        label="DeepSeek stream observation pipeline receipt",
    )
    return receipt


def run_deepseek_stream_observation_fixture(
    *,
    fixture_path: Path,
    output_root: Path,
) -> dict[str, Any]:
    if output_root.exists() and any(output_root.iterdir()):
        raise AssuranceError(f"output root must be empty or absent: {output_root}")
    fixture = _load_and_validate_fixture(fixture_path)
    output_root.mkdir(parents=True, exist_ok=True)
    fixture_copy = output_root / FIXTURE_NAME
    stream_path = output_root / STREAM_NAME
    integration_input_path = output_root / INTEGRATION_INPUT_NAME
    public_output_root = output_root / "runner-public-output"
    integration_root = output_root / "orientation-stagnation"
    journal_root = output_root / "runtime-journal"

    atomic_write_json(fixture_copy, fixture)
    stream = _build_runner_stream(fixture)
    atomic_write_json(stream_path, stream)
    extraction = run_runner_public_output_extraction_fixture(
        stream_path=stream_path,
        output_root=public_output_root,
    )
    integration_fixture = _build_integration_fixture(
        fixture=fixture,
        public_outputs=extraction["public_outputs"],
    )
    atomic_write_json(integration_input_path, integration_fixture)
    integration = run_orientation_stagnation_integration_fixture(
        fixture_path=integration_input_path,
        output_root=integration_root,
    )
    journal = write_orientation_stagnation_runtime_journal(
        integration_root=integration_root,
        journal_root=journal_root,
    )
    receipt = _build_receipt(
        fixture_path=fixture_copy,
        output_root=output_root,
        public_output_root=public_output_root,
        integration_root=integration_root,
        journal_root=journal_root,
        extraction=extraction,
        integration=integration,
        journal=journal,
    )
    atomic_write_json(output_root / RECEIPT_NAME, receipt)
    return receipt


def verify_deepseek_stream_observation_fixture(*, output_root: Path) -> dict[str, Any]:
    fixture_path = output_root / FIXTURE_NAME
    fixture = _load_and_validate_fixture(fixture_path)
    expected_stream = _build_runner_stream(fixture)
    observed_stream = load_json(output_root / STREAM_NAME)
    if canonical_bytes(observed_stream) != canonical_bytes(expected_stream):
        raise AssuranceError("DeepSeek stream runner-output projection does not rebuild")
    extraction = load_json(
        output_root
        / "runner-public-output"
        / "runner-public-output-extraction-receipt.json"
    )
    integration = verify_orientation_stagnation_integration_fixture(
        output_root=output_root / "orientation-stagnation",
    )
    journal = verify_orientation_stagnation_runtime_journal(
        integration_root=output_root / "orientation-stagnation",
        journal_root=output_root / "runtime-journal",
    )
    expected = _build_receipt(
        fixture_path=fixture_path,
        output_root=output_root,
        public_output_root=output_root / "runner-public-output",
        integration_root=output_root / "orientation-stagnation",
        journal_root=output_root / "runtime-journal",
        extraction=extraction,
        integration=integration,
        journal=journal,
    )
    observed = load_json(output_root / RECEIPT_NAME)
    validate_contract(
        observed,
        RECEIPT_SCHEMA,
        label="DeepSeek stream observation pipeline receipt",
    )
    if canonical_bytes(observed) != canonical_bytes(expected):
        raise AssuranceError("DeepSeek stream observation receipt does not rebuild")
    return expected
