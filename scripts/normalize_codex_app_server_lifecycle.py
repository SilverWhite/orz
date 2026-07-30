#!/usr/bin/env python3
"""Normalize an offline ordered Codex app-server capture into lifecycle observations."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from assurance.codex_app_server_lifecycle import (
    ADAPTER_ID,
    NORMALIZATION_SCHEMA,
    NORMALIZER_ID,
    RUNTIME_FAMILY,
    CodexLifecycleNormalizationError,
    normalize_capture,
    observations_bytes,
    sha256_bytes,
)
from assurance.errors import AssuranceError as PrototypeError
from assurance.io_utils import atomic_write_bytes, atomic_write_json
from assurance.schema import validate_instance


LIMITATIONS = [
    "The normalizer consumes an offline supervisor capture and does not start or control Codex app-server.",
    "Receive order proves transport observation order, not hidden model-internal causality.",
    "Ignored item and delta records are digest-bound only; prompt, reasoning, tool, and output content is not copied.",
    "An active or in-turn capture remains partial; EOF is never promoted to a session terminal.",
    "The observations file and receipt are atomically replaced individually, not as one cross-file transaction.",
]


def _artifact(path: Path, data: bytes) -> dict[str, Any]:
    return {
        "path": str(path.resolve()),
        "bytes": len(data),
        "sha256": sha256_bytes(data),
    }


def _base_receipt(
    *,
    capture: Path,
    runtime_version: str,
    run_id: str,
    run_manifest_sha256: str,
) -> dict[str, Any]:
    source_bytes = capture.read_bytes()
    source_sha256 = sha256_bytes(source_bytes)
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-lifecycle-normalization",
        "normalization_id": f"CLINORM-CODEX-{source_sha256[:24].upper()}",
        "normalizer_id": NORMALIZER_ID,
        "adapter_id": ADAPTER_ID,
        "valid": False,
        "normalization_status": "rejected",
        "runtime_family": RUNTIME_FAMILY,
        "runtime_version": runtime_version,
        "run_id": run_id,
        "run_manifest_sha256": run_manifest_sha256,
        "session_id": None,
        "source_stream_id": None,
        "source_artifact": _artifact(capture, source_bytes),
        "record_count": 0,
        "ignored_record_count": 0,
        "observation_count": 0,
        "observation_breakdown": {},
        "lifecycle_state": "unknown",
        "active_turn_id": None,
        "observations_artifact": None,
        "errors": [],
        "limitations": LIMITATIONS,
    }


def normalize_to_files(
    *,
    capture: Path,
    runtime_version: str,
    run_id: str,
    run_manifest_sha256: str,
    observations_path: Path,
    receipt_path: Path,
) -> dict[str, Any]:
    if observations_path.exists():
        raise PrototypeError(
            f"refusing to overwrite existing observations: {observations_path}"
        )
    if receipt_path.exists():
        raise PrototypeError(f"refusing to overwrite existing receipt: {receipt_path}")
    resolved = {capture.resolve(), observations_path.resolve(), receipt_path.resolve()}
    if len(resolved) != 3:
        raise PrototypeError("capture, observations, and receipt paths must be distinct")
    if not re.fullmatch(r"RUN-[A-Z0-9._-]+", run_id):
        raise PrototypeError("run id is invalid")
    if not re.fullmatch(r"[a-f0-9]{64}", run_manifest_sha256):
        raise PrototypeError("run manifest SHA-256 is invalid")
    if not runtime_version:
        raise PrototypeError("runtime version is required")

    receipt = _base_receipt(
        capture=capture,
        runtime_version=runtime_version,
        run_id=run_id,
        run_manifest_sha256=run_manifest_sha256,
    )
    try:
        result = normalize_capture(
            capture,
            runtime_version=runtime_version,
            run_id=run_id,
            run_manifest_sha256=run_manifest_sha256,
        )
        observations = result["observations"]
        observation_bytes = observations_bytes(observations)
        receipt.update(
            {
                "valid": True,
                "normalization_status": result["normalization_status"],
                "session_id": result["session_id"],
                "source_stream_id": result["source_stream_id"],
                "record_count": result["record_count"],
                "ignored_record_count": result["ignored_record_count"],
                "observation_count": len(observations),
                "observation_breakdown": result["observation_breakdown"],
                "lifecycle_state": result["lifecycle_state"],
                "active_turn_id": result["active_turn_id"],
                "observations_artifact": _artifact(
                    observations_path, observation_bytes
                ),
            }
        )
        validate_instance(receipt, NORMALIZATION_SCHEMA, label="normalization receipt")
        atomic_write_bytes(observations_path, observation_bytes)
        atomic_write_json(receipt_path, receipt)
        return receipt
    except CodexLifecycleNormalizationError as exc:
        receipt["errors"] = [str(exc)]
        validate_instance(
            receipt,
            NORMALIZATION_SCHEMA,
            label="rejected normalization receipt",
        )
        atomic_write_json(receipt_path, receipt)
        return receipt


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Normalize an offline ordered Codex app-server capture without "
            "starting or controlling Codex."
        )
    )
    parser.add_argument("--capture", required=True)
    parser.add_argument("--runtime-version", required=True)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--run-manifest-sha256", required=True)
    parser.add_argument("--observations", required=True)
    parser.add_argument("--receipt", required=True)
    args = parser.parse_args()

    capture = Path(args.capture).resolve()
    if not capture.is_file():
        print(f"capture does not exist: {capture}", file=sys.stderr)
        return 1
    try:
        receipt = normalize_to_files(
            capture=capture,
            runtime_version=args.runtime_version,
            run_id=args.run_id,
            run_manifest_sha256=args.run_manifest_sha256,
            observations_path=Path(args.observations).resolve(),
            receipt_path=Path(args.receipt).resolve(),
        )
    except (OSError, PrototypeError, ValueError) as exc:
        print(f"normalization failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, ensure_ascii=False, indent=2, allow_nan=False))
    return 0 if receipt["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
