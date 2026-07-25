#!/usr/bin/env python3
"""Independently replay a Codex app-server lifecycle normalization receipt."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from prototype.fep_agent_proto.codex_app_server_lifecycle import (
    NORMALIZATION_SCHEMA,
    CodexLifecycleNormalizationError,
    normalize_capture,
    observations_bytes,
    sha256_bytes,
)
from prototype.fep_agent_proto.errors import PrototypeError
from prototype.fep_agent_proto.io_utils import atomic_write_json
from prototype.fep_agent_proto.layout import RUNTIME_ROOT
from prototype.fep_agent_proto.schema import validate_instance


VERIFICATION_SCHEMA = (
    RUNTIME_ROOT / "codex-app-server-lifecycle-verification-v0.1.schema.json"
)
FORBIDDEN_KEYS = {
    "content",
    "reasoning",
    "reasoning_content",
    "text",
    "command",
    "output",
    "arguments",
    "result",
}


def _load_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PrototypeError(f"cannot read JSON object {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise PrototypeError(f"expected JSON object: {path}")
    return value


def _contains_forbidden(value: Any) -> bool:
    if isinstance(value, dict):
        return any(
            key in FORBIDDEN_KEYS or _contains_forbidden(child)
            for key, child in value.items()
        )
    if isinstance(value, list):
        return any(_contains_forbidden(child) for child in value)
    return False


def verify(
    *,
    capture: Path,
    observations_path: Path,
    receipt_path: Path,
) -> dict[str, Any]:
    errors: list[str] = []
    receipt = _load_object(receipt_path)
    receipt_schema_valid = True
    try:
        validate_instance(receipt, NORMALIZATION_SCHEMA, label="normalization receipt")
    except PrototypeError as exc:
        receipt_schema_valid = False
        errors.append(str(exc))

    source_bytes = capture.read_bytes()
    source_artifact = receipt.get("source_artifact", {})
    source_artifact_matches = (
        isinstance(source_artifact, dict)
        and source_artifact.get("path") == str(capture.resolve())
        and source_artifact.get("bytes") == len(source_bytes)
        and source_artifact.get("sha256") == sha256_bytes(source_bytes)
    )
    if not source_artifact_matches:
        errors.append("source artifact does not match receipt")

    replayed = None
    normalization_replayed = False
    try:
        replayed = normalize_capture(
            capture,
            runtime_version=receipt["runtime_version"],
            run_id=receipt["run_id"],
            run_manifest_sha256=receipt["run_manifest_sha256"],
        )
        normalization_replayed = True
    except (KeyError, CodexLifecycleNormalizationError) as exc:
        errors.append(f"normalization replay failed: {exc}")

    receipt_projection_matches = False
    observations_artifact_matches = False
    observation_content_matches = False
    raw_content_omitted = False
    if replayed is not None:
        expected_observation_bytes = observations_bytes(replayed["observations"])
        expected_projection = {
            "normalization_status": replayed["normalization_status"],
            "session_id": replayed["session_id"],
            "source_stream_id": replayed["source_stream_id"],
            "record_count": replayed["record_count"],
            "ignored_record_count": replayed["ignored_record_count"],
            "observation_count": len(replayed["observations"]),
            "observation_breakdown": replayed["observation_breakdown"],
            "lifecycle_state": replayed["lifecycle_state"],
            "active_turn_id": replayed["active_turn_id"],
        }
        receipt_projection_matches = receipt.get("valid") is True and all(
            receipt.get(key) == value for key, value in expected_projection.items()
        )
        if not receipt_projection_matches:
            errors.append("receipt lifecycle projection does not match replay")

        if observations_path.is_file():
            actual_observation_bytes = observations_path.read_bytes()
            artifact = receipt.get("observations_artifact")
            observations_artifact_matches = (
                isinstance(artifact, dict)
                and artifact.get("path") == str(observations_path.resolve())
                and artifact.get("bytes") == len(actual_observation_bytes)
                and artifact.get("sha256") == sha256_bytes(actual_observation_bytes)
            )
            observation_content_matches = (
                actual_observation_bytes == expected_observation_bytes
            )
            raw_content_omitted = not any(
                _contains_forbidden(item) for item in replayed["observations"]
            )
        if not observations_artifact_matches:
            errors.append("observations artifact does not match receipt")
        if not observation_content_matches:
            errors.append("observation content does not match replay")
        if not raw_content_omitted:
            errors.append("normalized observations contain forbidden raw content")

    checks = {
        "receipt_schema_valid": receipt_schema_valid,
        "source_artifact_matches": source_artifact_matches,
        "normalization_replayed": normalization_replayed,
        "receipt_projection_matches": receipt_projection_matches,
        "observations_artifact_matches": observations_artifact_matches,
        "observation_content_matches": observation_content_matches,
        "raw_content_omitted": raw_content_omitted,
    }
    result = {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-lifecycle-verification",
        "valid": all(checks.values()) and not errors,
        "normalization_id": receipt.get("normalization_id"),
        "run_id": receipt.get("run_id"),
        "session_id": receipt.get("session_id"),
        "source_stream_id": receipt.get("source_stream_id"),
        "checks": checks,
        "errors": errors,
        "limitations": [
            "Replay proves deterministic normalization of the supplied capture, not completeness of uncaptured runtime events.",
            "Verification does not establish model quality, task completion, or scientific correctness.",
        ],
    }
    validate_instance(result, VERIFICATION_SCHEMA, label="normalization verification")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Replay and verify a Codex app-server lifecycle normalization."
    )
    parser.add_argument("--capture", required=True)
    parser.add_argument("--observations", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    output = Path(args.output).resolve()
    if output.exists():
        print(f"output already exists; refusing to overwrite: {output}", file=sys.stderr)
        return 1
    try:
        result = verify(
            capture=Path(args.capture).resolve(),
            observations_path=Path(args.observations).resolve(),
            receipt_path=Path(args.receipt).resolve(),
        )
        atomic_write_json(output, result)
    except (OSError, PrototypeError, ValueError) as exc:
        print(f"verification failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False))
    return 0 if result["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
