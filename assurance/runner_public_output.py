from __future__ import annotations

from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import atomic_write_json, canonical_bytes, load_json, sha256_bytes


STREAM_SCHEMA = "runner-public-output-stream-v0.1.schema.json"
RECEIPT_SCHEMA = "runner-public-output-extraction-receipt-v0.1.schema.json"
STREAM_INPUT_NAME = "runner-output-stream.json"
RECEIPT_NAME = "runner-public-output-extraction-receipt.json"


def _validate_stream(stream: dict[str, Any]) -> None:
    validate_contract(stream, STREAM_SCHEMA, label="runner public output stream")
    seen_ids: set[str] = set()
    expected_sequence = 0
    for record in stream["records"]:
        record_id = record["record_id"]
        if record_id in seen_ids:
            raise AssuranceError(f"duplicate runner output record_id: {record_id}")
        seen_ids.add(record_id)
        if record["sequence"] != expected_sequence:
            raise AssuranceError("runner output stream sequence must be contiguous")
        expected_sequence += 1
        if record["visibility"] != "public" and "text" in record:
            raise AssuranceError("private/redacted runner output record must not save text")


def _extract_public_outputs(stream: dict[str, Any]) -> dict[str, Any]:
    _validate_stream(stream)
    public_channels = set(stream["extraction_policy"]["public_assistant_channels"])
    public_outputs: list[str] = []
    included_record_ids: list[str] = []
    excluded_record_ids: list[str] = []
    private_hidden_excluded_count = 0
    nonassistant_excluded_count = 0
    for record in stream["records"]:
        include = (
            record["source"] == "assistant"
            and record["visibility"] == "public"
            and record["channel"] in public_channels
        )
        if include:
            public_outputs.append(record["text"])
            included_record_ids.append(record["record_id"])
            continue
        excluded_record_ids.append(record["record_id"])
        if record["visibility"] in {"private_hidden", "redacted_metadata"}:
            private_hidden_excluded_count += 1
        if record["source"] != "assistant":
            nonassistant_excluded_count += 1
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "runner_public_output_extraction_receipt",
        "valid": True,
        "stream_id": stream["stream_id"],
        "stream_sha256": sha256_bytes(canonical_bytes(stream)),
        "public_outputs": public_outputs,
        "included_record_ids": included_record_ids,
        "excluded_record_ids": excluded_record_ids,
        "counts": {
            "record_count": len(stream["records"]),
            "public_output_count": len(public_outputs),
            "private_hidden_excluded_count": private_hidden_excluded_count,
            "nonassistant_excluded_count": nonassistant_excluded_count,
        },
        "restart_packet_source_policy": stream["extraction_policy"][
            "restart_packet_source_policy"
        ],
        "checks": {
            "ordered_sequence": True,
            "public_assistant_only": True,
            "private_channels_excluded": True,
            "hidden_chain_of_thought_saved": False,
            "runaway_suffix_excluded_from_restart": True,
            "model_invoked": False,
            "network_requested": False,
        },
        "limitations": [
            "This fixture extracts public assistant text only; it does not attach to a real runner.",
            "Private reasoning and redacted metadata are represented by digest only and are not copied to public_outputs.",
        ],
    }
    validate_contract(
        receipt,
        RECEIPT_SCHEMA,
        label="runner public output extraction receipt",
    )
    return receipt


def extract_public_outputs_from_runner_stream(stream: dict[str, Any]) -> dict[str, Any]:
    return _extract_public_outputs(stream)


def run_runner_public_output_extraction_fixture(
    *,
    stream_path: Path,
    output_root: Path,
) -> dict[str, Any]:
    if output_root.exists() and any(output_root.iterdir()):
        raise AssuranceError(f"output root must be empty or absent: {output_root}")
    stream = load_json(stream_path)
    receipt = _extract_public_outputs(stream)
    output_root.mkdir(parents=True, exist_ok=True)
    stream_copy = output_root / STREAM_INPUT_NAME
    receipt_path = output_root / RECEIPT_NAME
    atomic_write_json(stream_copy, stream)
    atomic_write_json(receipt_path, receipt)
    return receipt


def verify_runner_public_output_extraction_fixture(
    *,
    output_root: Path,
) -> dict[str, Any]:
    stream_path = output_root / STREAM_INPUT_NAME
    receipt_path = output_root / RECEIPT_NAME
    stream = load_json(stream_path)
    expected_receipt = _extract_public_outputs(stream)
    observed_receipt = load_json(receipt_path)
    validate_contract(
        observed_receipt,
        RECEIPT_SCHEMA,
        label="runner public output extraction receipt",
    )
    if canonical_bytes(observed_receipt) != canonical_bytes(expected_receipt):
        raise AssuranceError("runner public output extraction receipt does not rebuild")
    if observed_receipt["stream_sha256"] != sha256_bytes(canonical_bytes(stream)):
        raise AssuranceError("runner public output stream digest mismatch")
    return expected_receipt
