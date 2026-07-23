from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
import time
from typing import Any

from fake_deepseek_provider import (
    COMPACTION_POST_RESPONSE_MARKER,
    COMPACTION_SOURCE_OMITTED_MARKER,
    COMPACTION_SOURCE_RETAINED_MARKER,
    COMPACTION_SUMMARY_MARKER,
)


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    values: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError(f"expected JSON objects in {path}")
            values.append(value)
    return values


def _atomic_write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _artifact_valid(record: Any) -> bool:
    if not isinstance(record, dict):
        return False
    path_value = record.get("path")
    if not isinstance(path_value, str):
        return False
    path = Path(path_value)
    if not path.is_file():
        return False
    content = path.read_bytes()
    return (
        record.get("bytes") == len(content)
        and record.get("sha256") == _sha256_bytes(content)
    )


def verify(result_path: Path, lock_path: Path) -> dict[str, Any]:
    result_bytes = result_path.read_bytes()
    result = _read_json(result_path)
    lock = _read_json(lock_path)
    binary_release = lock.get("binary_release")
    if not isinstance(binary_release, dict):
        raise ValueError("lock has no binary_release object")
    artifacts = result.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("result has no artifacts object")

    request_record = artifacts.get("compaction_request")
    checkpoint_record = artifacts.get("compaction_checkpoint")
    hook_record = artifacts.get("hook_receipts")
    source_record = artifacts.get("source_snapshot")
    provider_record = artifacts.get("provider_capture")
    provider_result_record = artifacts.get("provider_result")
    required_artifacts = {
        key: value
        for key, value in artifacts.items()
        if key
        in {
            "config",
            "hook_config",
            "hook_receipts",
            "source_snapshot",
            "compaction_request",
            "compaction_checkpoint",
            "session_events",
            "session_updates",
            "provider_capture",
            "provider_result",
        }
    }
    artifact_hashes_valid = len(required_artifacts) == 10 and all(
        _artifact_valid(value) for value in required_artifacts.values()
    )
    request = (
        _read_json(Path(str(request_record["path"])))
        if isinstance(request_record, dict) and artifact_hashes_valid
        else {}
    )
    checkpoint = (
        _read_json(Path(str(checkpoint_record["path"])))
        if isinstance(checkpoint_record, dict) and artifact_hashes_valid
        else {}
    )
    hooks = (
        _read_jsonl(Path(str(hook_record["path"])))
        if isinstance(hook_record, dict) and artifact_hashes_valid
        else []
    )
    source_snapshot = (
        _read_json(Path(str(source_record["path"])))
        if isinstance(source_record, dict) and artifact_hashes_valid
        else {}
    )
    provider_rows = (
        _read_jsonl(Path(str(provider_record["path"])))
        if isinstance(provider_record, dict) and artifact_hashes_valid
        else []
    )
    primary_provider_rows = [
        row
        for row in provider_rows
        if isinstance(row.get("body"), dict)
        and row["body"].get("model") == "deepseek-v4-pro"
    ]
    provider_result = (
        _read_json(Path(str(provider_result_record["path"])))
        if isinstance(provider_result_record, dict) and artifact_hashes_valid
        else {}
    )
    history = request.get("chat_history")
    summary = request.get("summary")
    history_text = (
        json.dumps(history, ensure_ascii=False, sort_keys=True)
        if isinstance(history, list)
        else ""
    )
    checkpoint_text = json.dumps(checkpoint, ensure_ascii=False, sort_keys=True)
    source_text = json.dumps(source_snapshot, ensure_ascii=False, sort_keys=True)
    lifecycle = result.get("lifecycle", {})
    provenance = result.get("provenance", {})
    source_span = (
        provenance.get("source_span", {}) if isinstance(provenance, dict) else {}
    )
    summary_record = (
        provenance.get("summary", {}) if isinstance(provenance, dict) else {}
    )
    boundary = (
        provenance.get("boundary", {}) if isinstance(provenance, dict) else {}
    )
    ranges = provenance.get("ranges", {}) if isinstance(provenance, dict) else {}
    unknown = ranges.get("unknown", []) if isinstance(ranges, dict) else []
    containment = result.get("containment", {})
    release = result.get("release", {})
    checks = {
        "terminal_state_succeeded": result.get("terminal_state") == "succeeded",
        "locked_binary_matches": (
            isinstance(release, dict)
            and release.get("inspection_valid") is True
            and release.get("binary_sha256") == binary_release.get("sha256")
            and release.get("version") == binary_release.get("version")
            and release.get("build_id") == binary_release.get("build_id")
        ),
        "artifact_hashes_valid": artifact_hashes_valid,
        "manual_request_recorded": request.get("trigger") == "manual",
        "source_span_nonempty": isinstance(history, list) and len(history) > 0,
        "source_span_digest_replays": (
            isinstance(history, list)
            and isinstance(source_span, dict)
            and source_span.get("item_count") == len(history)
            and source_span.get("first_index") == 0
            and source_span.get("last_index") == len(history) - 1
            and source_span.get("canonical_sha256")
            == _sha256_bytes(_canonical_bytes(history))
        ),
        "source_canaries_recorded": (
            COMPACTION_SOURCE_RETAINED_MARKER in history_text
            and COMPACTION_SOURCE_OMITTED_MARKER in history_text
            and COMPACTION_SOURCE_RETAINED_MARKER in source_text
            and COMPACTION_SOURCE_OMITTED_MARKER in source_text
        ),
        "derived_summary_boundary": (
            isinstance(summary, str)
            and COMPACTION_SUMMARY_MARKER in summary
            and COMPACTION_SOURCE_RETAINED_MARKER in summary
            and COMPACTION_SOURCE_OMITTED_MARKER not in summary
            and isinstance(provenance, dict)
            and provenance.get("summary", {}).get("status")
            == "derived_unverified"
            and provenance.get("summary", {}).get("may_replace_source_evidence")
            is False
        ),
        "summary_digest_replays": (
            isinstance(summary, str)
            and isinstance(summary_record, dict)
            and summary_record.get("chars") == len(summary)
            and summary_record.get("sha256")
            == _sha256_bytes(summary.encode("utf-8"))
        ),
        "boundary_replays": (
            isinstance(boundary, dict)
            and boundary.get("request_id") == request.get("request_id")
            and boundary.get("trigger") == request.get("trigger")
            and boundary.get("checkpoint_id") == checkpoint.get("checkpoint_id")
            and boundary.get("prompt_index_at_compaction")
            == checkpoint.get("prompt_index_at_compaction")
        ),
        "checkpoint_contains_summary": COMPACTION_SUMMARY_MARKER in checkpoint_text,
        "hook_lifecycle_ordered": (
            [row.get("hook_event_name") for row in hooks]
            == ["pre_compact", "post_compact"]
            and all(row.get("source") == "manual" for row in hooks)
            and isinstance(lifecycle, dict)
            and lifecycle.get("session_ids_match") is True
        ),
        "unknown_mapping_explicit": (
            isinstance(unknown, list)
            and len(unknown) == 1
            and ranges.get("retained") == []
            and ranges.get("discarded") == []
        ),
        "three_request_fixture_complete": (
            len(primary_provider_rows) == 3
            and provider_result.get("response", {}).get("marker")
            == COMPACTION_POST_RESPONSE_MARKER
        ),
        "source_snapshot_matches_first_request": (
            len(primary_provider_rows) >= 1
            and source_snapshot == primary_provider_rows[0]
        ),
        "containment_declared": (
            isinstance(containment, dict)
            and containment.get("loopback_only_provider") is True
            and containment.get("firewall_added") is True
            and containment.get("job_created") is True
            and containment.get("job_assigned") is True
            and containment.get("real_model_invoked") is False
        ),
    }
    return {
        "schema_version": "0.1.0",
        "verifier": "verify_grok_compaction_provenance_probe.py",
        "verified_at_unix_ns": time.time_ns(),
        "result_path": str(result_path.resolve()),
        "result_sha256": _sha256_bytes(result_bytes),
        "lock_path": str(lock_path.resolve()),
        "lock_sha256": _sha256_bytes(lock_path.read_bytes()),
        "checks": checks,
        "passed": all(checks.values()),
        "limitations": [
            "Verification establishes fixed-fixture provenance mechanics only.",
            "It does not validate the semantic truth of the derived summary.",
            "It does not convert unrecorded retained/discarded mappings into claims.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Independently verify a Grok compaction provenance probe result"
    )
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--lock", type=Path, required=True)
    args = parser.parse_args()
    try:
        verification = verify(args.result.resolve(), args.lock.resolve())
        _atomic_write_json(args.output.resolve(), verification)
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(verification, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0 if verification["passed"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
