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


API_RESULT_SCHEMA = "deepseek-api-observation-result-v0.1.schema.json"
PIPELINE_RECEIPT_SCHEMA = "deepseek-api-observation-pipeline-receipt-v0.1.schema.json"

API_RESULT_NAME = "api-observation-result.json"
STREAM_INPUT_NAME = "runner-output-stream.input.json"
PIPELINE_RECEIPT_NAME = "deepseek-api-observation-pipeline-receipt.json"


def _validate_api_result(result: dict[str, Any]) -> None:
    validate_contract(result, API_RESULT_SCHEMA, label="DeepSeek API observation result")
    if result["request_count"] != 1 or result["retry_count"] != 0:
        raise AssuranceError("DeepSeek API observation must be exactly one request")
    if result["credential_value_recorded"] or result["raw_response_recorded"]:
        raise AssuranceError("DeepSeek API observation recorded a forbidden raw surface")


def _build_stream(result: dict[str, Any]) -> dict[str, Any]:
    _validate_api_result(result)
    records: list[dict[str, Any]] = []
    records.append(
        {
            "record_id": "REC-DEEPSEEK-METADATA-000",
            "sequence": 0,
            "source": "runner",
            "channel": "metadata_event",
            "visibility": "redacted_metadata",
            "content_sha256": sha256_bytes(canonical_bytes(result["usage"])),
        }
    )
    private_digest = result.get("private_reasoning_content_sha256")
    if private_digest:
        records.append(
            {
                "record_id": "REC-DEEPSEEK-PRIVATE-001",
                "sequence": 1,
                "source": "assistant",
                "channel": "reasoning_private",
                "visibility": "private_hidden",
                "content_sha256": private_digest,
            }
        )
    records.append(
        {
            "record_id": f"REC-DEEPSEEK-PUBLIC-{len(records):03d}",
            "sequence": len(records),
            "source": "assistant",
            "channel": "assistant_final",
            "visibility": "public",
            "text": result["public_assistant_text"],
        }
    )
    return {
        "schema_version": "0.1.0-draft",
        "stream_kind": "runner_public_output_stream_fixture",
        "stream_id": "STREAM-DEEPSEEK-API-OBSERVATION-001",
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
    api_result: dict[str, Any],
    public_outputs: list[str],
    task_id: str,
    task_contract_sha256: str,
    step_index: int,
) -> dict[str, Any]:
    progress_markers = [
        f"deepseek-api-status-{api_result['http_status_code']}",
        f"deepseek-model-{api_result['model']}",
    ]
    return {
        "schema_version": "0.1.0-draft",
        "fixture_kind": "orientation_stagnation_integration_fixture",
        "task_id": task_id,
        "task_contract_sha256": task_contract_sha256,
        "step_index": step_index,
        "public_outputs": public_outputs,
        "retry_count": 0,
        "retry_budget": 1,
        "thresholds": {
            "repeated_content_threshold": 10,
            "ngram_repeat_threshold": 10,
        },
        "progress_markers": progress_markers,
    }


def _build_pipeline_receipt(
    *,
    api_result_path: Path,
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
        "receipt_kind": "deepseek_api_observation_pipeline_receipt",
        "valid": True,
        "api_result": {
            "path": API_RESULT_NAME,
            "sha256": sha256_file(api_result_path),
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
            "stagnation_decision": integration["decisions"]["stagnation_decision"],
            "stagnation_action": integration["decisions"]["stagnation_action"],
            "terminal_event_type": journal["terminal_event"],
        },
        "checks": {
            "model_invoked": True,
            "network_requested": True,
            "request_count_one": True,
            "retry_count_zero": True,
            "credential_value_recorded": False,
            "raw_response_recorded": False,
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
            "This is one controlled API observation, not a production runner adapter.",
            "The API response is projected to public-output fixtures; raw provider response and credentials are not recorded.",
            "A non-repeating one-shot response can verify the plumbing but cannot exercise the restart branch by itself.",
        ],
    }
    validate_contract(
        receipt,
        PIPELINE_RECEIPT_SCHEMA,
        label="DeepSeek API observation pipeline receipt",
    )
    return receipt


def run_deepseek_api_observation_pipeline(
    *,
    api_result_path: Path,
    output_root: Path,
    task_id: str,
    task_contract_sha256: str,
    step_index: int,
) -> dict[str, Any]:
    if output_root.exists() and any(output_root.iterdir()):
        raise AssuranceError(f"output root must be empty or absent: {output_root}")
    api_result = load_json(api_result_path)
    _validate_api_result(api_result)

    output_root.mkdir(parents=True, exist_ok=True)
    api_result_copy = output_root / API_RESULT_NAME
    stream_input_path = output_root / STREAM_INPUT_NAME
    integration_input_path = output_root / "orientation-stagnation-input.json"
    public_output_root = output_root / "runner-public-output"
    integration_root = output_root / "orientation-stagnation"
    journal_root = output_root / "runtime-journal"

    atomic_write_json(api_result_copy, api_result)
    stream = _build_stream(api_result)
    atomic_write_json(stream_input_path, stream)
    extraction = run_runner_public_output_extraction_fixture(
        stream_path=stream_input_path,
        output_root=public_output_root,
    )
    integration_fixture = _build_integration_fixture(
        api_result=api_result,
        public_outputs=extraction["public_outputs"],
        task_id=task_id,
        task_contract_sha256=task_contract_sha256,
        step_index=step_index,
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
    receipt = _build_pipeline_receipt(
        api_result_path=api_result_copy,
        output_root=output_root,
        public_output_root=public_output_root,
        integration_root=integration_root,
        journal_root=journal_root,
        extraction=extraction,
        integration=integration,
        journal=journal,
    )
    atomic_write_json(output_root / PIPELINE_RECEIPT_NAME, receipt)
    return receipt


def verify_deepseek_api_observation_pipeline(*, output_root: Path) -> dict[str, Any]:
    api_result_path = output_root / API_RESULT_NAME
    api_result = load_json(api_result_path)
    _validate_api_result(api_result)
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
    expected = _build_pipeline_receipt(
        api_result_path=api_result_path,
        output_root=output_root,
        public_output_root=output_root / "runner-public-output",
        integration_root=output_root / "orientation-stagnation",
        journal_root=output_root / "runtime-journal",
        extraction=extraction,
        integration=integration,
        journal=journal,
    )
    observed = load_json(output_root / PIPELINE_RECEIPT_NAME)
    validate_contract(
        observed,
        PIPELINE_RECEIPT_SCHEMA,
        label="DeepSeek API observation pipeline receipt",
    )
    if canonical_bytes(observed) != canonical_bytes(expected):
        raise AssuranceError("DeepSeek API observation pipeline receipt does not rebuild")
    return expected
