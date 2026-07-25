from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from .errors import PrototypeError
from .io_utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)
from .layout import DESIGN_ROOT, RUNTIME_ROOT
from .journal_lock import JournalLockError, exclusive_journal_lock
from .schema import validate_instance


TERMINAL_EVENTS = {"run_finished", "run_failed", "run_cancelled", "run_invalidated"}


def _event_hash(event: dict[str, Any]) -> str:
    projection = dict(event)
    projection.pop("event_sha256", None)
    return sha256_bytes(canonical_bytes(projection))


def _append_event_unlocked(
    journal_path: Path,
    *,
    run_id: str,
    run_manifest_sha256: str,
    event_type: str,
    payload_schema: str,
    payload: dict[str, Any],
    redaction: str = "none",
) -> dict[str, Any]:
    previous: str | None = None
    sequence = 0
    if journal_path.exists():
        replay = _replay_journal_unlocked(
            run_manifest_path=None,
            journal_path=journal_path,
            expected_manifest_sha256=run_manifest_sha256,
            expected_run_id=run_id,
            require_terminal=False,
        )
        if not replay["valid"]:
            raise PrototypeError("refusing to append to an invalid journal")
        sequence = replay["event_count"]
        previous = replay["last_event_sha256"]
        if replay["terminal_event"] is not None:
            raise PrototypeError("refusing to append after terminal event")

    event = {
        "schema_version": "0.1.0-draft",
        "run_id": run_id,
        "event_id": f"EVT-{run_id[4:]}-{sequence:06d}",
        "sequence": sequence,
        "timestamp": utc_now(),
        "event_type": event_type,
        "run_manifest_sha256": run_manifest_sha256,
        "previous_event_sha256": previous,
        "payload_schema": payload_schema,
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": redaction,
        "event_sha256": "0" * 64,
    }
    event["event_sha256"] = _event_hash(event)
    validate_instance(
        event,
        RUNTIME_ROOT / "run-event-v0.1.schema.json",
        label=f"run event {sequence}",
    )
    line = json.dumps(
        event, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode("utf-8") + b"\n"
    journal_path.parent.mkdir(parents=True, exist_ok=True)
    with journal_path.open("ab") as handle:
        handle.write(line)
        handle.flush()
        os.fsync(handle.fileno())
    return event


def append_event(
    journal_path: Path,
    *,
    run_id: str,
    run_manifest_sha256: str,
    event_type: str,
    payload_schema: str,
    payload: dict[str, Any],
    redaction: str = "none",
) -> dict[str, Any]:
    try:
        with exclusive_journal_lock(journal_path):
            return _append_event_unlocked(
                journal_path,
                run_id=run_id,
                run_manifest_sha256=run_manifest_sha256,
                event_type=event_type,
                payload_schema=payload_schema,
                payload=payload,
                redaction=redaction,
            )
    except JournalLockError as exc:
        raise PrototypeError(str(exc)) from exc


def _replay_journal_unlocked(
    *,
    run_manifest_path: Path | None,
    journal_path: Path,
    expected_manifest_sha256: str | None = None,
    expected_run_id: str | None = None,
    require_terminal: bool = True,
) -> dict[str, Any]:
    errors: list[str] = []
    if run_manifest_path is not None:
        manifest = load_json(run_manifest_path)
        validate_instance(
            manifest,
            RUNTIME_ROOT / "run-manifest-v0.1.schema.json",
            label="run manifest",
        )
        expected_manifest_sha256 = sha256_file(run_manifest_path)
        expected_run_id = manifest["run_id"]
    if not expected_manifest_sha256 or not expected_run_id:
        raise PrototypeError("replay requires a run manifest or explicit expected identifiers")
    if not journal_path.is_file():
        raise PrototypeError(f"journal does not exist: {journal_path}")

    events: list[dict[str, Any]] = []
    with journal_path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            if not line.strip():
                errors.append(f"blank journal line {line_number}")
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError as exc:
                errors.append(f"invalid JSON at line {line_number}: {exc}")
                continue
            try:
                validate_instance(
                    event,
                    RUNTIME_ROOT / "run-event-v0.1.schema.json",
                    label=f"journal line {line_number}",
                )
            except PrototypeError as exc:
                errors.append(str(exc))
            events.append(event)

    previous: str | None = None
    terminal_events: list[str] = []
    for expected_sequence, event in enumerate(events):
        if event.get("sequence") != expected_sequence:
            errors.append(f"sequence mismatch at event {expected_sequence}")
        if event.get("run_id") != expected_run_id:
            errors.append(f"run ID mismatch at event {expected_sequence}")
        if event.get("run_manifest_sha256") != expected_manifest_sha256:
            errors.append(f"manifest digest mismatch at event {expected_sequence}")
        if event.get("previous_event_sha256") != previous:
            errors.append(f"previous-event digest mismatch at event {expected_sequence}")
        payload = event.get("payload")
        if isinstance(payload, dict):
            if event.get("payload_sha256") != sha256_bytes(canonical_bytes(payload)):
                errors.append(f"payload digest mismatch at event {expected_sequence}")
        if event.get("event_sha256") != _event_hash(event):
            errors.append(f"event digest mismatch at event {expected_sequence}")
        previous = event.get("event_sha256")
        if event.get("event_type") in TERMINAL_EVENTS:
            terminal_events.append(event["event_type"])
            if expected_sequence != len(events) - 1:
                errors.append("terminal event is not last")

    if require_terminal and len(terminal_events) != 1:
        errors.append(f"expected exactly one terminal event, found {len(terminal_events)}")
    if len(terminal_events) > 1:
        errors.append("multiple terminal events")
    return {
        "schema_version": "0.1.0-prototype",
        "valid": not errors,
        "run_id": expected_run_id,
        "run_manifest_sha256": expected_manifest_sha256,
        "journal_path": str(journal_path),
        "event_count": len(events),
        "last_event_sha256": previous,
        "terminal_event": terminal_events[-1] if terminal_events else None,
        "errors": errors,
        "checked_at": utc_now(),
        "limitations": ["Replay verifies mechanics and does not score model correctness."],
    }


def replay_journal(
    *,
    run_manifest_path: Path | None,
    journal_path: Path,
    expected_manifest_sha256: str | None = None,
    expected_run_id: str | None = None,
    require_terminal: bool = True,
) -> dict[str, Any]:
    if not journal_path.is_file():
        return _replay_journal_unlocked(
            run_manifest_path=run_manifest_path,
            journal_path=journal_path,
            expected_manifest_sha256=expected_manifest_sha256,
            expected_run_id=expected_run_id,
            require_terminal=require_terminal,
        )
    try:
        with exclusive_journal_lock(journal_path):
            return _replay_journal_unlocked(
                run_manifest_path=run_manifest_path,
                journal_path=journal_path,
                expected_manifest_sha256=expected_manifest_sha256,
                expected_run_id=expected_run_id,
                require_terminal=require_terminal,
            )
    except JournalLockError as exc:
        raise PrototypeError(str(exc)) from exc


def create_journal_smoke(*, export_root: Path, output_dir: Path) -> dict[str, Any]:
    if output_dir.exists():
        raise PrototypeError(f"refusing to overwrite existing smoke root: {output_dir}")
    manifest_path = export_root / "scenario-export-manifest.json"
    export_manifest = load_json(manifest_path)
    validate_instance(
        export_manifest,
        RUNTIME_ROOT / "scenario-export-manifest-v0.1.schema.json",
        label="scenario export manifest",
    )
    if export_manifest["status"] != "ready":
        raise PrototypeError("journal smoke requires a ready development export")
    output_dir.mkdir(parents=True)
    try:
        system_prompt = output_dir / "system-prompt.txt"
        project_rules = output_dir / "project-rules.txt"
        generated_context = output_dir / "generated-context.json"
        redaction_policy = output_dir / "redaction-policy.json"
        atomic_write_bytes(system_prompt, b"NO_MODEL_SMOKE\n")
        atomic_write_bytes(project_rules, b"DEVELOPMENT_ONLY_NO_ORACLE\n")
        atomic_write_json(generated_context, {"model_invoked": False, "purpose": "journal smoke"})
        atomic_write_json(
            redaction_policy,
            {"version": "0.1.0-prototype", "policy": "no secrets accepted by smoke"},
        )

        run_id = f"RUN-SMOKE-{export_manifest['bundle_sha256'][:12].upper()}"
        run_manifest = {
            "schema_version": "0.1.0-draft",
            "manifest_kind": "immutable_precommit",
            "run_id": run_id,
            "created_at": utc_now(),
            "execution_mode": "development",
            "scenario_export": {
                "export_id": export_manifest["export_id"],
                "manifest_sha256": sha256_file(manifest_path),
                "bundle_sha256": export_manifest["bundle_sha256"],
                "leak_scan_report_sha256": export_manifest["leak_scan"]["report_sha256"],
            },
            "adapter": {
                "provider": "prototype",
                "model_id": "no-model-smoke",
                "adapter_id": "no-model-adapter",
                "adapter_version": "0.1.0-prototype",
                "capabilities": {
                    "structured_output": False,
                    "tool_calling": False,
                    "streaming": False,
                    "seed_control": False,
                    "cancellation": True,
                    "context_limit_tokens": 1,
                },
            },
            "prompts": {
                "system_prompt_sha256": sha256_file(system_prompt),
                "project_rules_sha256": sha256_file(project_rules),
                "generated_context_sha256": sha256_file(generated_context),
            },
            "tools": {
                "allowlist": [],
                "policy_sha256": sha256_bytes(canonical_bytes({"allowlist": []})),
                "permission_mode": "deny_by_default",
                "filesystem_profile": "strict_ephemeral",
                "network_profile": "disabled",
            },
            "isolation": {
                "memory": "disabled",
                "historical_case_retrieval": "disabled",
                "oracle_mounted": False,
                "reviewer_fixtures_mounted": False,
                "inherited_user_config": False,
                "writable_roots": [str(output_dir.resolve())],
            },
            "budgets": {
                "wall_time_seconds": 30,
                "max_turns": 1,
                "max_input_tokens": 1,
                "max_output_tokens": 1,
                "max_tool_calls": 0,
            },
            "randomness": {"seed": None, "temperature": 0, "retry_policy": "none"},
            "protocol_digests": {
                "agent_protocol": sha256_file(DESIGN_ROOT / "protocol/agent-protocol-v0.1.schema.json"),
                "reason_codes": sha256_file(DESIGN_ROOT / "protocol/reason-codes-v0.1.yaml"),
                "gate_matrix": sha256_file(DESIGN_ROOT / "protocol/gate-matrix-v0.1.yaml"),
                "scoring_protocol": sha256_file(DESIGN_ROOT / "evaluation/SCORING_PROTOCOL_v0.1.md"),
                "output_schema": sha256_file(
                    DESIGN_ROOT / "evaluation/evaluation-result-v0.1.schema.json"
                ),
            },
            "journal_policy": {
                "format": "jsonl",
                "event_schema_version": "0.1.0-draft",
                "canonicalization": "RFC8785",
                "hash_chain": "sha256",
                "redaction_policy_sha256": sha256_file(redaction_policy),
            },
            "notes": [
                "Development-only no-model smoke; this is not an Agent evaluation result."
            ],
        }
        validate_instance(
            run_manifest,
            RUNTIME_ROOT / "run-manifest-v0.1.schema.json",
            label="smoke run manifest",
        )
        run_manifest_path = output_dir / "run-manifest.json"
        atomic_write_json(run_manifest_path, run_manifest)
        manifest_sha = sha256_file(run_manifest_path)
        journal_path = output_dir / "events.jsonl"
        append_event(
            journal_path,
            run_id=run_id,
            run_manifest_sha256=manifest_sha,
            event_type="run_preflight",
            payload_schema="prototype/preflight-v0.1",
            payload={
                "export_status": export_manifest["status"],
                "leak_verdict": export_manifest["leak_scan"]["verdict"],
                "model_invoked": False,
            },
        )
        append_event(
            journal_path,
            run_id=run_id,
            run_manifest_sha256=manifest_sha,
            event_type="run_finished",
            payload_schema="prototype/no-model-finish-v0.1",
            payload={
                "model_invoked": False,
                "result": "journal_replay_smoke_completed",
                "claim_eligibility": "not_applicable",
            },
        )
        report = replay_journal(
            run_manifest_path=run_manifest_path,
            journal_path=journal_path,
        )
        atomic_write_json(output_dir / "replay-report.json", report)
        if not report["valid"]:
            raise PrototypeError(f"journal smoke replay failed: {report['errors']}")
        return report
    except Exception as exc:
        try:
            atomic_write_json(
                output_dir / "_INCOMPLETE.json",
                {"status": "incomplete", "error": str(exc), "recorded_at": utc_now()},
            )
        except Exception:
            pass
        raise
