from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sys
import uuid
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


def _load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _atomic_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f"{path.name}.{uuid.uuid4().hex}.tmp")
    try:
        temporary.write_text(
            json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n",
            encoding="utf-8",
        )
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def verify(bridge_root: Path, schema_root: Path) -> dict[str, Any]:
    manifest_path = bridge_root / "manifest.json"
    events_path = bridge_root / "events.bridge.jsonl"
    manifest = _load_json(manifest_path)
    manifest_schema = _load_json(schema_root / "grok-event-bridge-manifest-v0.1.schema.json")
    event_schema = _load_json(schema_root / "grok-event-bridge-event-v0.1.schema.json")
    format_checker = FormatChecker()
    errors: list[str] = []
    manifest_errors = sorted(
        Draft202012Validator(
            manifest_schema, format_checker=format_checker
        ).iter_errors(manifest),
        key=lambda error: list(error.absolute_path),
    )
    errors.extend(
        "manifest#/" + "/".join(map(str, error.absolute_path)) + f": {error.message}"
        for error in manifest_errors
    )

    source_artifacts_match = True
    for name, artifact in manifest.get("source_artifacts", {}).items():
        path = Path(artifact.get("path", ""))
        if (
            not path.is_file()
            or path.stat().st_size != artifact.get("bytes")
            or _sha256_file(path) != artifact.get("sha256")
        ):
            source_artifacts_match = False
            errors.append(f"source artifact mismatch: {name}")

    journal_artifact = manifest.get("event_journal", {})
    journal_matches = (
        events_path.is_file()
        and str(events_path.resolve()) == journal_artifact.get("path")
        and events_path.stat().st_size == journal_artifact.get("bytes")
        and _sha256_file(events_path) == journal_artifact.get("sha256")
    )
    if not journal_matches:
        errors.append("event journal artifact mismatch")

    event_validator = Draft202012Validator(event_schema, format_checker=format_checker)
    events: list[dict[str, Any]] = []
    previous: str | None = None
    hash_chain_valid = True
    sequence_contiguous = True
    raw_fields_omitted = True
    forbidden = {"content", "data", "rawInput", "rawOutput", "reasoning_content"}

    def contains_forbidden(value: Any) -> bool:
        if isinstance(value, dict):
            return any(
                key in forbidden or contains_forbidden(child)
                for key, child in value.items()
            )
        if isinstance(value, list):
            return any(contains_forbidden(child) for child in value)
        return False

    for line_number, raw_line in enumerate(events_path.read_bytes().splitlines(), 1):
        if not raw_line.strip():
            continue
        try:
            event = json.loads(raw_line.decode("utf-8"))
        except Exception as exc:
            errors.append(f"event line {line_number}: {exc}")
            continue
        event_errors = sorted(
            event_validator.iter_errors(event), key=lambda error: list(error.absolute_path)
        )
        errors.extend(
            f"event[{line_number}]#/"
            + "/".join(map(str, error.absolute_path))
            + f": {error.message}"
            for error in event_errors
        )
        sequence = len(events)
        if event.get("sequence") != sequence:
            sequence_contiguous = False
        candidate = dict(event)
        claimed = candidate.pop("event_sha256", None)
        actual = hashlib.sha256(_canonical_bytes(candidate)).hexdigest()
        if claimed != actual or event.get("previous_event_sha256") != previous:
            hash_chain_valid = False
        previous = claimed
        details = event.get("details", {})
        if contains_forbidden(details):
            raw_fields_omitted = False
        events.append(event)

    counts_match = (
        len(events) == manifest.get("counts", {}).get("events")
        and dict(sorted(Counter(event.get("observed_from") for event in events).items()))
        == manifest.get("counts", {}).get("by_source")
        and dict(
            sorted(Counter(event.get("canonical_event_type") for event in events).items())
        )
        == manifest.get("counts", {}).get("by_canonical_type")
    )
    if not counts_match:
        errors.append("event counts do not match manifest")
    if not sequence_contiguous:
        errors.append("bridge sequence is not contiguous")
    if not hash_chain_valid:
        errors.append("event hash chain mismatch")
    if not raw_fields_omitted:
        errors.append("forbidden raw content field found in bridge details")

    checks = {
        "manifest_schema_valid": not manifest_errors,
        "source_artifacts_match": source_artifacts_match,
        "event_journal_artifact_matches": journal_matches,
        "event_schemas_valid": not any(error.startswith("event[") for error in errors),
        "event_count_and_breakdown_match": counts_match,
        "bridge_sequence_contiguous": sequence_contiguous,
        "event_hash_chain_valid": hash_chain_valid,
        "raw_content_fields_omitted": raw_fields_omitted,
        "manifest_claims_valid": manifest.get("valid") is True,
        "completeness_not_promoted": manifest.get("completeness", {}).get("overall")
        == "partial",
    }
    valid = all(checks.values()) and not errors
    return {
        "schema_version": "0.1.0",
        "verification_kind": "grok-event-bridge-replay",
        "checked_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "bridge_id": manifest.get("bridge_id"),
        "run_id": manifest.get("run_id"),
        "session_id": manifest.get("session_id"),
        "manifest": {
            "path": str(manifest_path.resolve()),
            "bytes": manifest_path.stat().st_size,
            "sha256": _sha256_file(manifest_path),
        },
        "event_journal": {
            "path": str(events_path.resolve()),
            "bytes": events_path.stat().st_size,
            "sha256": _sha256_file(events_path),
        },
        "source_artifact_count": len(manifest.get("source_artifacts", {})),
        "event_count": len(events),
        "checks": checks,
        "valid": valid,
        "errors": errors,
        "limitations": [
            "Replay validation checks recorded bytes and structure, not missing unrecorded events.",
            "A valid partial bridge is not a complete runtime trace or scientific proof.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Independently replay and verify a Grok event bridge.")
    parser.add_argument("--bridge-directory", required=True)
    parser.add_argument("--schema-directory", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    output = Path(args.output).resolve()
    if output.exists():
        print(f"output already exists; refusing to overwrite: {output}", file=sys.stderr)
        return 1
    try:
        report = verify(
            Path(args.bridge_directory).resolve(), Path(args.schema_directory).resolve()
        )
        output.parent.mkdir(parents=True, exist_ok=True)
        _atomic_json(output, report)
    except Exception as exc:
        print(f"bridge verification failed: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
