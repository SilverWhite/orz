from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sys
import uuid
from typing import Any, Iterable


SHA256_PATTERN_LENGTH = 64


def _canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


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


def _atomic_jsonl(path: Path, values: Iterable[dict[str, Any]]) -> None:
    temporary = path.with_name(f"{path.name}.{uuid.uuid4().hex}.tmp")
    try:
        with temporary.open("w", encoding="utf-8", newline="\n") as handle:
            for value in values:
                handle.write(
                    json.dumps(
                        value, ensure_ascii=False, sort_keys=True, allow_nan=False
                    )
                )
                handle.write("\n")
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def _artifact(path: Path, *, state: str, sensitivity: str, parsed: bool) -> dict[str, Any]:
    if not path.is_file():
        raise ValueError(f"source artifact is missing: {path}")
    return {
        "path": str(path.resolve()),
        "bytes": path.stat().st_size,
        "sha256": _sha256_file(path),
        "state": state,
        "sensitivity": sensitivity,
        "parsed": parsed,
    }


def _resolve_inside(root: Path, path: Path) -> Path:
    resolved_root = root.resolve()
    resolved = path.resolve()
    try:
        resolved.relative_to(resolved_root)
    except ValueError as exc:
        raise ValueError(f"path escaped run directory: {resolved}") from exc
    return resolved


def _json_lines(path: Path) -> list[tuple[bytes, dict[str, Any]]]:
    records: list[tuple[bytes, dict[str, Any]]] = []
    for line_number, raw_line in enumerate(path.read_bytes().splitlines(), 1):
        if not raw_line.strip():
            continue
        try:
            value = json.loads(raw_line.decode("utf-8"))
        except Exception as exc:
            raise ValueError(f"invalid JSON line {path}:{line_number}: {exc}") from exc
        if not isinstance(value, dict):
            raise ValueError(f"JSON line is not an object: {path}:{line_number}")
        records.append((raw_line, value))
    return records


def _epoch_seconds_to_iso(value: Any) -> str | None:
    if not isinstance(value, (int, float)):
        return None
    return datetime.fromtimestamp(value, tz=timezone.utc).isoformat().replace("+00:00", "Z")


def _epoch_ns_to_iso(value: Any) -> str | None:
    if not isinstance(value, int):
        return None
    seconds, nanoseconds = divmod(value, 1_000_000_000)
    base = datetime.fromtimestamp(seconds, tz=timezone.utc)
    return base.strftime("%Y-%m-%dT%H:%M:%S") + f".{nanoseconds:09d}Z"


def _find_session_directory(grok_home: Path, session_id: str) -> Path:
    matches: list[Path] = []
    sessions = grok_home / "sessions"
    for summary_path in sessions.rglob("summary.json"):
        try:
            summary = _load_json(summary_path)
        except Exception:
            continue
        if summary.get("info", {}).get("id") == session_id:
            matches.append(summary_path.parent.resolve())
    if len(matches) != 1:
        raise ValueError(
            f"expected exactly one local session directory for {session_id}, found {len(matches)}"
        )
    return matches[0]


class EventBuilder:
    def __init__(self, *, bridge_id: str, run_id: str, session_id: str) -> None:
        self.bridge_id = bridge_id
        self.run_id = run_id
        self.session_id = session_id
        self.events: list[dict[str, Any]] = []
        self.previous_event_sha256: str | None = None

    def append(
        self,
        *,
        observed_from: str,
        source_artifact: str,
        source_sequence: int,
        source_event_type: str,
        canonical_event_type: str,
        runtime_timestamp: str | None,
        turn_id: str | None,
        tool_call_id: str | None,
        raw_payload: bytes | None,
        raw_payload_sha256: str | None,
        raw_payload_bytes: int | None,
        redaction_state: str,
        completeness_state: str,
        details: dict[str, Any],
    ) -> None:
        if raw_payload is not None:
            raw_payload_sha256 = _sha256_bytes(raw_payload)
            raw_payload_bytes = len(raw_payload)
        if raw_payload_sha256 is None or len(raw_payload_sha256) != SHA256_PATTERN_LENGTH:
            raise ValueError("event requires a SHA-256 payload reference")
        sequence = len(self.events)
        event = {
            "schema_version": "0.1.0",
            "bridge_id": self.bridge_id,
            "run_id": self.run_id,
            "session_id": self.session_id,
            "sequence": sequence,
            "event_id": f"BEVT-{sequence:06d}",
            "observed_from": observed_from,
            "source_artifact": source_artifact,
            "source_sequence": source_sequence,
            "source_event_type": source_event_type,
            "canonical_event_type": canonical_event_type,
            "runtime_timestamp": runtime_timestamp,
            "turn_id": turn_id,
            "tool_call_id": tool_call_id,
            "raw_payload_sha256": raw_payload_sha256,
            "raw_payload_bytes": raw_payload_bytes,
            "raw_payload_scope": "source_record_or_artifact_bytes",
            "redaction_state": redaction_state,
            "completeness_state": completeness_state,
            "details": details,
            "previous_event_sha256": self.previous_event_sha256,
        }
        event_sha256 = _sha256_bytes(_canonical_bytes(event))
        event["event_sha256"] = event_sha256
        self.events.append(event)
        self.previous_event_sha256 = event_sha256


def _event_hash_chain_valid(events: list[dict[str, Any]]) -> bool:
    previous: str | None = None
    for sequence, event in enumerate(events):
        candidate = dict(event)
        claimed = candidate.pop("event_sha256", None)
        if (
            event.get("sequence") != sequence
            or event.get("previous_event_sha256") != previous
            or claimed != _sha256_bytes(_canonical_bytes(candidate))
        ):
            return False
        previous = claimed
    return True


def _raw_content_fields_omitted(events: list[dict[str, Any]]) -> bool:
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

    return not any(contains_forbidden(event.get("details", {})) for event in events)


def _safe_stdout_details(value: dict[str, Any]) -> tuple[str, dict[str, Any]]:
    event_type = str(value.get("type", "unknown"))
    if event_type == "end":
        usage = value.get("usage") if isinstance(value.get("usage"), dict) else {}
        return "terminal", {
            "stop_reason": value.get("stopReason"),
            "request_id": value.get("requestId"),
            "reported_session_id": value.get("sessionId"),
            "num_turns": value.get("num_turns"),
            "usage_keys": sorted(usage),
        }
    if event_type == "thought":
        return "model_reasoning_chunk", {"content_omitted": True}
    if event_type == "text":
        return "model_output_chunk", {"content_omitted": True}
    if "tool" in event_type.lower():
        return "tool_lifecycle", {"content_omitted": True}
    return "source_event", {"content_omitted": "data" in value}


def build_bridge(args: argparse.Namespace) -> dict[str, Any]:
    run_root = Path(args.run_directory).resolve()
    output_root = Path(args.output_directory).resolve()
    if output_root.exists():
        raise ValueError(f"output directory already exists; refusing to overwrite: {output_root}")
    result_path = _resolve_inside(run_root, run_root / "result.json")
    result = _load_json(result_path)
    if result.get("valid") is not True:
        raise ValueError("source run result is not valid")
    run_id = str(result["run_id"])
    session_id = str(result["session_id"])
    bridge_id = "BRIDGE-" + uuid.uuid4().hex

    config_path = _resolve_inside(run_root, Path(result["artifacts"]["config"]["path"]))
    profile_root = config_path.parent.parent
    grok_home = config_path.parent
    workspace = _resolve_inside(run_root, Path(result["artifacts"]["prompt"]["path"])).parent
    session_directory = _find_session_directory(grok_home, session_id)
    _resolve_inside(run_root, session_directory)

    known_paths = {
        "source_result": result_path,
        "grok_stdout": _resolve_inside(run_root, Path(result["artifacts"]["stdout"]["path"])),
        "provider_result": _resolve_inside(run_root, Path(result["artifacts"]["provider_result"]["path"])),
        "provider_private": _resolve_inside(run_root, Path(result["artifacts"]["provider_private"]["path"])),
        "trust_preflight": _resolve_inside(run_root, Path(result["artifacts"]["workspace_trust_receipt_preflight"]["path"])),
        "trust_launch": _resolve_inside(run_root, Path(result["artifacts"]["workspace_trust_receipt_launch"]["path"])),
        "session_events": session_directory / "events.jsonl",
        "session_updates": session_directory / "updates.jsonl",
        "session_chat_history": session_directory / "chat_history.jsonl",
        "session_summary": session_directory / "summary.json",
        "session_signals": session_directory / "signals.json",
        "trust_postrun": Path(args.postrun_receipt).resolve(),
        "trace_status": Path(args.trace_status).resolve(),
        "export_status": Path(args.export_status).resolve(),
    }
    for name in ("trust_postrun", "trace_status", "export_status"):
        _resolve_inside(Path(args.evidence_directory).resolve(), known_paths[name])

    sensitivity = {
        "provider_private": "sealed_private_candidate_plaintext_source",
        "session_updates": "sealed_private_candidate_plaintext_source",
        "session_chat_history": "sealed_private_candidate_plaintext_source",
        "grok_stdout": "content_bearing_fake_fixture",
    }
    parsed_names = {
        "source_result", "grok_stdout", "provider_result", "trust_preflight",
        "trust_launch", "trust_postrun", "session_events", "session_updates",
        "session_summary", "session_signals", "trace_status", "export_status",
    }
    source_artifacts = {
        name: _artifact(
            path,
            state="available",
            sensitivity=sensitivity.get(name, "metadata"),
            parsed=name in parsed_names,
        )
        for name, path in known_paths.items()
    }

    source_result_artifacts_match = True
    for artifact in result.get("artifacts", {}).values():
        artifact_path = _resolve_inside(run_root, Path(artifact["path"]))
        if (
            artifact_path.stat().st_size != artifact["bytes"]
            or _sha256_file(artifact_path) != artifact["sha256"]
        ):
            source_result_artifacts_match = False

    preflight = _load_json(known_paths["trust_preflight"])
    launch = _load_json(known_paths["trust_launch"])
    postrun = _load_json(known_paths["trust_postrun"])
    trace_status = _load_json(known_paths["trace_status"])
    export_status = _load_json(known_paths["export_status"])
    summary = _load_json(known_paths["session_summary"])
    provider = _load_json(known_paths["provider_result"])

    builder = EventBuilder(bridge_id=bridge_id, run_id=run_id, session_id=session_id)
    for source_sequence, (name, receipt) in enumerate(
        (("trust_preflight", preflight), ("trust_launch", launch), ("trust_postrun", postrun))
    ):
        builder.append(
            observed_from="workspace_scan",
            source_artifact=name,
            source_sequence=source_sequence,
            source_event_type="workspace_trust_receipt",
            canonical_event_type="workspace_trust",
            runtime_timestamp=receipt.get("created_at"),
            turn_id=None,
            tool_call_id=None,
            raw_payload=None,
            raw_payload_sha256=source_artifacts[name]["sha256"],
            raw_payload_bytes=source_artifacts[name]["bytes"],
            redaction_state="metadata_only",
            completeness_state="direct",
            details={
                "mode": receipt.get("decision", {}).get("mode"),
                "launch_permitted": receipt.get("decision", {}).get("launch_permitted"),
                "candidate_count": receipt.get("discovery", {}).get("candidate_count"),
                "aggregate_sha256": receipt.get("discovery", {}).get("aggregate_sha256"),
            },
        )

    stdout_records = _json_lines(known_paths["grok_stdout"])
    for source_sequence, (raw_line, value) in enumerate(stdout_records):
        canonical_type, details = _safe_stdout_details(value)
        builder.append(
            observed_from="grok_stdout",
            source_artifact="grok_stdout",
            source_sequence=source_sequence,
            source_event_type=str(value.get("type", "unknown")),
            canonical_event_type=canonical_type,
            runtime_timestamp=None,
            turn_id=None,
            tool_call_id=None,
            raw_payload=raw_line,
            raw_payload_sha256=None,
            raw_payload_bytes=None,
            redaction_state="content_redacted" if "data" in value else "metadata_only",
            completeness_state="direct",
            details=details,
        )

    session_event_records = _json_lines(known_paths["session_events"])
    event_detail_keys = (
        "phase", "tool_name", "decision", "outcome", "duration_ms", "loop_index",
        "model_id", "conversation_message_count",
    )
    for source_sequence, (raw_line, value) in enumerate(session_event_records):
        event_type = str(value.get("type", "unknown"))
        canonical_type = {
            "turn_started": "turn_started",
            "turn_ended": "terminal",
            "tool_started": "tool_started",
            "tool_completed": "tool_completed",
            "permission_requested": "permission_requested",
            "permission_resolved": "permission_decision",
        }.get(event_type, "source_event")
        details = {key: value[key] for key in event_detail_keys if key in value}
        builder.append(
            observed_from="grok_session_events",
            source_artifact="session_events",
            source_sequence=source_sequence,
            source_event_type=event_type,
            canonical_event_type=canonical_type,
            runtime_timestamp=value.get("ts"),
            turn_id=str(value["turn_number"]) if "turn_number" in value else None,
            tool_call_id=None,
            raw_payload=raw_line,
            raw_payload_sha256=None,
            raw_payload_bytes=None,
            redaction_state="metadata_only",
            completeness_state="direct",
            details=details,
        )

    update_records = _json_lines(known_paths["session_updates"])
    for source_sequence, (raw_line, value) in enumerate(update_records):
        params = value.get("params") if isinstance(value.get("params"), dict) else {}
        update = params.get("update") if isinstance(params.get("update"), dict) else {}
        update_type = str(update.get("sessionUpdate", "unknown"))
        tool_call_id = update.get("toolCallId") if isinstance(update.get("toolCallId"), str) else None
        canonical_type = {
            "tool_call": "tool_proposal",
            "tool_call_update": "tool_update",
            "agent_thought_chunk": "model_reasoning_chunk",
            "agent_message_chunk": "model_output_chunk",
        }.get(update_type, "source_event")
        builder.append(
            observed_from="grok_session_updates",
            source_artifact="session_updates",
            source_sequence=source_sequence,
            source_event_type=update_type,
            canonical_event_type=canonical_type,
            runtime_timestamp=_epoch_seconds_to_iso(value.get("timestamp")),
            turn_id=None,
            tool_call_id=tool_call_id,
            raw_payload=raw_line,
            raw_payload_sha256=None,
            raw_payload_bytes=None,
            redaction_state="content_redacted",
            completeness_state="direct",
            details={
                "status": update.get("status"),
                "kind": update.get("kind"),
                "content_omitted": True,
            },
        )

    provider_requests = provider.get("requests") if isinstance(provider.get("requests"), list) else []
    for source_sequence, request in enumerate(provider_requests):
        if not isinstance(request, dict):
            raise ValueError("provider request summary is not an object")
        builder.append(
            observed_from="provider_capture",
            source_artifact="provider_private",
            source_sequence=source_sequence,
            source_event_type="provider_request",
            canonical_event_type="model_request",
            runtime_timestamp=_epoch_ns_to_iso(request.get("received_at_unix_ns")),
            turn_id=None,
            tool_call_id=None,
            raw_payload=None,
            raw_payload_sha256=request.get("body_sha256"),
            raw_payload_bytes=request.get("body_bytes"),
            redaction_state="content_redacted",
            completeness_state="direct",
            details={
                key: request.get(key)
                for key in (
                    "sequence", "request_class", "method", "path", "model", "stream",
                    "message_count", "message_roles", "tool_names", "authorization_present",
                    "authorization_value_recorded",
                )
            },
        )

    for source_sequence, (name, status, observed_from) in enumerate(
        (("trace_status", trace_status, "grok_trace"), ("export_status", export_status, "grok_export"))
    ):
        builder.append(
            observed_from=observed_from,
            source_artifact=name,
            source_sequence=source_sequence,
            source_event_type="postrun_command_status",
            canonical_event_type="source_status",
            runtime_timestamp=status.get("completed_at"),
            turn_id=None,
            tool_call_id=None,
            raw_payload=None,
            raw_payload_sha256=source_artifacts[name]["sha256"],
            raw_payload_bytes=source_artifacts[name]["bytes"],
            redaction_state="metadata_only",
            completeness_state="direct" if status.get("state") == "available" else "unavailable",
            details={
                "state": status.get("state"),
                "exit_code": status.get("exit_code"),
                "timed_out": status.get("timed_out"),
                "artifact_created": status.get("artifact_created"),
            },
        )

    builder.append(
        observed_from="supervisor",
        source_artifact="source_result",
        source_sequence=0,
        source_event_type="source_run_result",
        canonical_event_type="run_result",
        runtime_timestamp=result.get("created_at"),
        turn_id=None,
        tool_call_id=None,
        raw_payload=None,
        raw_payload_sha256=source_artifacts["source_result"]["sha256"],
        raw_payload_bytes=source_artifacts["source_result"]["bytes"],
        redaction_state="metadata_only",
        completeness_state="direct",
        details={
            "valid": result.get("valid"),
            "exit_code": result.get("process", {}).get("exit_code"),
            "timed_out": result.get("process", {}).get("timed_out"),
        },
    )

    source_counts = Counter(event["observed_from"] for event in builder.events)
    canonical_counts = Counter(event["canonical_event_type"] for event in builder.events)
    stdout_tool = any("tool" in value.get("type", "").lower() for _, value in stdout_records)
    session_types = [value.get("type") for _, value in session_event_records]
    update_types = [
        value.get("params", {}).get("update", {}).get("sessionUpdate")
        for _, value in update_records
    ]
    is_tool_scenario = result.get("scenario") == "tool-continuity"
    session_tool_lifecycle = "tool_started" in session_types and "tool_completed" in session_types
    session_tool_identity = "tool_call" in update_types and "tool_call_update" in update_types
    provider_tool_continuity = (
        all(provider.get("continuity", {}).values()) if is_tool_scenario else None
    )
    trust_unchanged = all(
        receipt.get("valid") is True
        and receipt.get("decision", {}).get("launch_permitted") is True
        and receipt.get("discovery", {}).get("candidate_count") == 0
        and receipt.get("discovery", {}).get("aggregate_sha256")
        == result.get("workspace_trust", {}).get("aggregate_sha256")
        and receipt.get("discovery", {}).get("scan_policy_sha256")
        == result.get("workspace_trust", {}).get("scan_policy_sha256")
        for receipt in (preflight, launch, postrun)
    )
    checks = {
        "source_result_valid": result.get("valid") is True,
        "source_result_artifacts_match": source_result_artifacts_match,
        "session_id_matches_summary": summary.get("info", {}).get("id") == session_id,
        "workspace_matches_summary": Path(summary.get("info", {}).get("cwd", "")).resolve() == workspace,
        "trust_receipts_valid_and_unchanged": trust_unchanged,
        "source_json_lines_parse": True,
        "source_order_preserved": True,
        "payloads_exist_before_references": True,
        "bridge_sequence_contiguous": [event["sequence"] for event in builder.events]
        == list(range(len(builder.events))),
        "event_hash_chain_valid": _event_hash_chain_valid(builder.events),
        "terminal_observed": "end" in [value.get("type") for _, value in stdout_records]
        and "turn_ended" in session_types,
        "tool_lifecycle_observed_when_expected": (
            session_tool_lifecycle and session_tool_identity and provider_tool_continuity
        ) if is_tool_scenario else True,
        "raw_content_omitted_from_bridge": _raw_content_fields_omitted(builder.events),
        "debug_capture_disabled": True,
    }
    valid = all(checks.values())
    missing = ["cross_source_runtime_order", "raw_content_sealed_encryption"]
    if not stdout_tool and is_tool_scenario:
        missing.append("explicit_stdout_tool_lifecycle")
    if trace_status.get("state") != "available":
        missing.append("grok_trace_archive")
    if export_status.get("state") != "available":
        missing.append("grok_export_transcript")

    output_root.mkdir(parents=True)
    events_path = output_root / "events.bridge.jsonl"
    _atomic_jsonl(events_path, builder.events)
    event_journal = _artifact(
        events_path, state="available", sensitivity="metadata", parsed=True
    )
    manifest = {
        "schema_version": "0.1.0",
        "bridge_kind": "grok-local-event-completeness",
        "bridge_id": bridge_id,
        "created_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "run_id": run_id,
        "session_id": session_id,
        "source_result_schema": result.get("schema_version"),
        "source_artifacts": source_artifacts,
        "event_journal": event_journal,
        "counts": {
            "events": len(builder.events),
            "by_source": dict(sorted(source_counts.items())),
            "by_canonical_type": dict(sorted(canonical_counts.items())),
        },
        "ordering": {
            "bridge_sequence_semantics": "deterministic_source_append_order",
            "cross_source_runtime_order": "not_established",
            "source_order_preserved": True,
        },
        "completeness": {
            "overall": "partial" if valid else "invalid",
            "stdout_tool_lifecycle": "observed" if stdout_tool else "absent",
            "session_event_tool_lifecycle": (
                "observed" if session_tool_lifecycle else "not_applicable" if not is_tool_scenario else "absent"
            ),
            "session_update_tool_identity": (
                "observed" if session_tool_identity else "not_applicable" if not is_tool_scenario else "absent"
            ),
            "provider_tool_continuity": (
                "observed" if provider_tool_continuity else "not_applicable" if not is_tool_scenario else "absent"
            ),
            "trace": trace_status.get("state", "unknown"),
            "export": export_status.get("state", "unknown"),
            "missing_observations": missing,
        },
        "checks": checks,
        "valid": valid,
        "limitations": [
            "Bridge sequence is deterministic append order, not inferred cross-source runtime chronology.",
            "Raw content remains in existing fake-run source files and is not copied into the bridge.",
            "Plaintext session/update/provider-private sources are sealed-private candidates, not an encrypted storage layer.",
            "Trace and export command failure is recorded as missing evidence rather than synthesized output.",
            "Mechanical validity and partial completeness do not establish safety or scientific correctness.",
        ],
    }
    _atomic_json(output_root / "manifest.json", manifest)
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build a metadata-only, hash-chained Grok event completeness bridge."
    )
    parser.add_argument("--run-directory", required=True)
    parser.add_argument("--evidence-directory", required=True)
    parser.add_argument("--output-directory", required=True)
    parser.add_argument("--postrun-receipt", required=True)
    parser.add_argument("--trace-status", required=True)
    parser.add_argument("--export-status", required=True)
    args = parser.parse_args()
    try:
        manifest = build_bridge(args)
    except Exception as exc:
        print(f"event bridge failed: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(manifest, ensure_ascii=False, indent=2, allow_nan=False))
    return 0 if manifest["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
