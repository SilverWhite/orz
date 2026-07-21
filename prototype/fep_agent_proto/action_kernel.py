from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any, Sequence
import uuid

from .errors import PrototypeError
from .io_utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    load_json,
    safe_relative_path,
    sha256_bytes,
    sha256_file,
    utc_now,
)
from .journal import append_event, replay_journal
from .layout import DESIGN_ROOT, PROTOCOL_ROOT, RUNTIME_ROOT
from .schema import validate_instance
from .session_validator import validate_session_record
from .windows_process import run_windows_process


ACTION_ID = "ACT-LOCAL-PROCESS-001"
ARTIFACT_ID = "ART-PROCESS-RESULT-001"
AGENT_ID = "AGENT-PROTOTYPE-KERNEL"
TOOL_NAME = "local_process_smoke"
TERMINAL_RUN_EVENTS = {
    "succeeded": "run_finished",
    "failed": "run_failed",
    "cancelled": "run_cancelled",
    "unknown": "run_invalidated",
}


def _write_run_manifest(
    *, export_root: Path, output_dir: Path, run_id: str
) -> tuple[dict[str, Any], Path]:
    export_manifest_path = export_root / "scenario-export-manifest.json"
    export_manifest = load_json(export_manifest_path)
    validate_instance(
        export_manifest,
        RUNTIME_ROOT / "scenario-export-manifest-v0.1.schema.json",
        label="scenario export manifest",
    )
    if export_manifest["status"] != "ready":
        raise PrototypeError("action kernel requires a ready development export")

    system_prompt = output_dir / "system-prompt.txt"
    project_rules = output_dir / "project-rules.txt"
    generated_context = output_dir / "generated-context.json"
    redaction_policy = output_dir / "redaction-policy.json"
    atomic_write_bytes(system_prompt, b"NO_MODEL_ACTION_KERNEL_SMOKE\n")
    atomic_write_bytes(project_rules, b"FIXED_LOCAL_PROCESS_ALLOWLIST_ONLY\n")
    atomic_write_json(
        generated_context,
        {
            "model_invoked": False,
            "purpose": "Windows local-process action-kernel integration smoke",
        },
    )
    atomic_write_json(
        redaction_policy,
        {
            "version": "0.1.0-prototype",
            "policy": "arguments and process output are represented by metadata and digests",
        },
    )
    tool_policy = {
        "allowlist": [TOOL_NAME],
        "arbitrary_cli_command_exposed": False,
        "shell_allowed": False,
    }
    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "immutable_precommit",
        "run_id": run_id,
        "created_at": utc_now(),
        "execution_mode": "development",
        "scenario_export": {
            "export_id": export_manifest["export_id"],
            "manifest_sha256": sha256_file(export_manifest_path),
            "bundle_sha256": export_manifest["bundle_sha256"],
            "leak_scan_report_sha256": export_manifest["leak_scan"]["report_sha256"],
        },
        "adapter": {
            "provider": "prototype",
            "model_id": "no-model-action-kernel",
            "adapter_id": "no-model-action-kernel-adapter",
            "adapter_version": "0.1.0-prototype",
            "capabilities": {
                "structured_output": False,
                "tool_calling": True,
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
            "allowlist": [TOOL_NAME],
            "policy_sha256": sha256_bytes(canonical_bytes(tool_policy)),
            "permission_mode": "preapproved_narrow",
            "filesystem_profile": "custom",
            "network_profile": "disabled",
        },
        "isolation": {
            "memory": "session_only",
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
            "max_tool_calls": 1,
        },
        "randomness": {"seed": None, "temperature": 0, "retry_policy": "none"},
        "protocol_digests": {
            "agent_protocol": sha256_file(PROTOCOL_ROOT / "agent-protocol-v0.1.schema.json"),
            "reason_codes": sha256_file(PROTOCOL_ROOT / "reason-codes-v0.1.yaml"),
            "gate_matrix": sha256_file(PROTOCOL_ROOT / "gate-matrix-v0.1.yaml"),
            "scoring_protocol": sha256_file(
                DESIGN_ROOT / "evaluation/SCORING_PROTOCOL_v0.1.md"
            ),
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
            "Development-only no-model Windows action-kernel smoke; custom filesystem profile is not a sandbox claim."
        ],
    }
    validate_instance(
        manifest,
        RUNTIME_ROOT / "run-manifest-v0.1.schema.json",
        label="action-kernel run manifest",
    )
    path = output_dir / "run-manifest.json"
    atomic_write_json(path, manifest)
    return manifest, path


def _append(
    journal_path: Path,
    *,
    run_id: str,
    manifest_sha256: str,
    event_type: str,
    payload: dict[str, Any],
) -> dict[str, Any]:
    return append_event(
        journal_path,
        run_id=run_id,
        run_manifest_sha256=manifest_sha256,
        event_type=event_type,
        payload_schema=f"prototype/action-kernel/{event_type}-v0.1",
        payload=payload,
        redaction="metadata_only",
    )


def _source_record(source_id: str, path: Path) -> dict[str, Any]:
    return {
        "source_id": source_id,
        "kind": "workspace_file",
        "path_or_uri": str(path),
        "content_hash": sha256_file(path),
        "accessed_at": utc_now(),
        "usage": "read",
        "epistemic_role": "source_grounded",
    }


def _build_session_record(
    *,
    output_dir: Path,
    process_report: dict[str, Any],
    process_result_sha256: str,
    journal_head: str,
) -> dict[str, Any]:
    state = process_report["process"]["terminal_state"]
    reason_codes = ["PROC-EXIT-UNKNOWN-001"] if state == "unknown" else []
    process_artifact_path = output_dir / "windows-process-result.json"
    sources = [
        _source_record("SRC-PROTOCOL", PROTOCOL_ROOT / "PROTOCOL_DRAFT_v0.1.md"),
        _source_record(
            "SRC-WINDOWS-RUNTIME",
            DESIGN_ROOT / "architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md",
        ),
        _source_record(
            "SRC-ACTION-KERNEL",
            DESIGN_ROOT / "architecture/ACTION_KERNEL_CONTRACT_v0.1.md",
        ),
    ]
    action_arguments = {
        "executable": process_report["command"]["executable"],
        "argument_count": process_report["command"]["argument_count"],
        "arguments_sha256": process_report["command"]["arguments_sha256"],
        "command_sha256": process_report["command"]["command_sha256"],
        "cwd": process_report["command"]["cwd"],
        "shell": False,
    }
    idempotency_key = sha256_bytes(canonical_bytes(action_arguments))
    session = {
        "protocol_version": "0.1.0-draft",
        "session": {
            "session_id": f"SESSION-{process_report['run_id'][6:]}",
            "workspace_root": str(output_dir.resolve()),
            "created_at": process_report["started_at"],
            "updated_at": process_report["completed_at"],
            "task_state": "reviewing",
            "mode": "guarded",
            "agent_profile_id": AGENT_ID,
            "model_adapter_id": None,
            "journal_head": journal_head,
        },
        "task_contract": {
            "task_id": "TASK-ACTION-KERNEL-SMOKE",
            "intent": "exercise one deterministic Windows local-process action and record replayable mechanics",
            "mode": "implementation",
            "must": [
                "execute without a shell",
                "record exactly one terminal action event",
                "atomically persist and hash the process result",
            ],
            "must_not": [
                "invoke a model",
                "treat process success as scientific evidence",
                "claim filesystem or network sandboxing",
            ],
            "source_of_truth": [
                "SRC-PROTOCOL",
                "SRC-WINDOWS-RUNTIME",
                "SRC-ACTION-KERNEL",
            ],
            "acceptance": [
                "session schema and mechanical invariants validate",
                "journal hash chain replays with one terminal run event",
                "process artifact digest matches the registered artifact",
            ],
            "risk_class": "workspace_mutation",
        },
        "reasoning_precommitment": None,
        "sources": sources,
        "actions": [
            {
                "action_id": ACTION_ID,
                "action_type": "local_process",
                "requested_by": AGENT_ID,
                "state": state,
                "arguments": action_arguments,
                "risk_class": "reversible_write",
                "expected_outputs": [str(process_artifact_path)],
                "required_gate_ids": [],
                "idempotency_key": idempotency_key,
                "events": [
                    {
                        "event_id": "EVT-ACTION-PROCESS-STARTED",
                        "sequence": 0,
                        "timestamp": process_report["started_at"],
                        "action_id": ACTION_ID,
                        "event_kind": "progress",
                        "event_type": "process.started",
                        "payload": {
                            "pid": process_report["process"]["pid"],
                            "job_object_assigned": process_report["containment"][
                                "job_object_assigned"
                            ],
                        },
                        "causal_parent": None,
                        "exit_code": None,
                        "terminal_state": None,
                        "produced_artifact_ids": [],
                        "reason_codes": [],
                    },
                    {
                        "event_id": "EVT-ACTION-PROCESS-TERMINAL",
                        "sequence": 1,
                        "timestamp": process_report["completed_at"],
                        "action_id": ACTION_ID,
                        "event_kind": "terminal",
                        "event_type": f"process.{state}",
                        "payload": {
                            "runtime_ready": process_report["runtime_ready"],
                            "timed_out": process_report["process"]["timed_out"],
                            "process_result_sha256": process_result_sha256,
                        },
                        "causal_parent": "EVT-ACTION-PROCESS-STARTED",
                        "exit_code": process_report["process"]["exit_code"],
                        "terminal_state": state,
                        "produced_artifact_ids": [ARTIFACT_ID],
                        "reason_codes": reason_codes,
                    },
                ],
            }
        ],
        "artifacts": [
            {
                "artifact_id": ARTIFACT_ID,
                "path_or_uri": str(process_artifact_path),
                "sha256": process_result_sha256,
                "producer_action_id": ACTION_ID,
                "media_type": "application/json",
                "schema_id": "runtime/windows-process-result-v0.1.schema.json",
                "checks": {
                    "atomic_write": "pass",
                    "parse": "pass",
                    "finite": "not_applicable",
                    "schema": "pass",
                    "sidecar": "not_applicable",
                },
                "planned_count": 1,
                "completed_count": 1,
                "record_count": 1,
                "manifest_artifact_id": None,
            }
        ],
        "evidence": [],
        "gate_decisions": [],
        "case_retrievals": [],
        "claims": [],
    }
    return session


def execute_local_process_action(
    *,
    export_root: Path,
    output_dir: Path,
    command: Sequence[str],
    timeout_seconds: float,
    cancel_grace_seconds: float = 1.0,
) -> dict[str, Any]:
    if output_dir.exists():
        raise PrototypeError(f"refusing to overwrite existing action-kernel root: {output_dir}")
    output_dir.mkdir(parents=True)
    run_id = f"RUN-ACTION-{uuid.uuid4().hex[:16].upper()}"
    try:
        _, manifest_path = _write_run_manifest(
            export_root=export_root,
            output_dir=output_dir,
            run_id=run_id,
        )
        manifest_sha256 = sha256_file(manifest_path)
        journal_path = output_dir / "events.jsonl"
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="run_preflight",
            payload={
                "model_invoked": False,
                "tool_allowlisted": True,
                "arbitrary_cli_command_exposed": False,
                "network_requested": False,
            },
        )
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="run_started",
            payload={"action_count": 1, "tool": TOOL_NAME},
        )
        command_digest = sha256_bytes(canonical_bytes(list(command)))
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="tool_proposal",
            payload={
                "action_id": ACTION_ID,
                "tool": TOOL_NAME,
                "command_sha256": command_digest,
                "argument_count": max(len(command) - 1, 0),
            },
        )
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="permission_decision",
            payload={
                "action_id": ACTION_ID,
                "decision": "allow",
                "policy": "fixed_smoke_allowlist",
                "shell": False,
            },
        )
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="tool_started",
            payload={"action_id": ACTION_ID, "command_sha256": command_digest},
        )

        process_report = run_windows_process(
            command,
            cwd=output_dir,
            timeout_seconds=timeout_seconds,
            cancel_grace_seconds=cancel_grace_seconds,
        )
        process_path = output_dir / "windows-process-result.json"
        atomic_write_json(process_path, process_report)
        process_sha256 = sha256_file(process_path)
        tool_completed = _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="tool_completed",
            payload={
                "action_id": ACTION_ID,
                "terminal_state": process_report["process"]["terminal_state"],
                "exit_code": process_report["process"]["exit_code"],
                "timed_out": process_report["process"]["timed_out"],
                "runtime_ready": process_report["runtime_ready"],
                "process_result_sha256": process_sha256,
            },
        )

        session = _build_session_record(
            output_dir=output_dir,
            process_report=process_report,
            process_result_sha256=process_sha256,
            journal_head=tool_completed["event_id"],
        )
        session_validation = validate_session_record(session)
        if not session_validation["valid"]:
            raise PrototypeError(
                f"action-kernel session validation failed: {session_validation['violations']}"
            )
        session_path = output_dir / "session-record.json"
        atomic_write_json(session_path, session)
        session_sha256 = sha256_file(session_path)
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="artifact_registered",
            payload={
                "action_id": ACTION_ID,
                "artifacts": [
                    {
                        "artifact_id": ARTIFACT_ID,
                        "path": process_path.name,
                        "sha256": process_sha256,
                    },
                    {
                        "artifact_id": "ART-SESSION-SNAPSHOT-001",
                        "path": session_path.name,
                        "sha256": session_sha256,
                    },
                ],
            },
        )
        state = process_report["process"]["terminal_state"]
        terminal_run_event = TERMINAL_RUN_EVENTS[state]
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type=terminal_run_event,
            payload={
                "action_id": ACTION_ID,
                "action_state": state,
                "model_invoked": False,
                "claim_eligibility": "not_assessed",
            },
        )
        replay = replay_journal(
            run_manifest_path=manifest_path,
            journal_path=journal_path,
        )
        result = {
            "schema_version": "0.1.0-prototype",
            "kernel": "windows-local-process-action-spike",
            "run_id": run_id,
            "valid": session_validation["valid"] and replay["valid"],
            "runtime_ready": process_report["runtime_ready"],
            "action_state": state,
            "terminal_run_event": terminal_run_event,
            "session_validation": {
                "valid": session_validation["valid"],
                "violation_count": session_validation["violation_count"],
            },
            "journal_replay": {
                "valid": replay["valid"],
                "event_count": replay["event_count"],
                "terminal_event": replay["terminal_event"],
            },
            "artifacts": {
                "run_manifest": {
                    "path": manifest_path.name,
                    "sha256": sha256_file(manifest_path),
                },
                "journal": {
                    "path": journal_path.name,
                    "sha256": sha256_file(journal_path),
                },
                "process_result": {
                    "path": process_path.name,
                    "sha256": process_sha256,
                },
                "session_record": {
                    "path": session_path.name,
                    "sha256": session_sha256,
                },
            },
            "limitations": [
                "The fixed permission decision is not a general interactive tool broker.",
                "Job Object containment is not filesystem or network sandboxing.",
                "The session is a derived snapshot and the append-only journal remains the replay source.",
            ],
        }
        validate_instance(
            result,
            RUNTIME_ROOT / "action-kernel-result-v0.1.schema.json",
            label="action-kernel result",
        )
        result_path = output_dir / "action-kernel-result.json"
        atomic_write_json(result_path, result)
        verification = verify_action_kernel(output_dir=output_dir)
        if not verification["valid"]:
            raise PrototypeError(
                f"action-kernel cross-file verification failed: {verification['errors']}"
            )
        return result
    except Exception as exc:
        try:
            atomic_write_json(
                output_dir / "_INCOMPLETE.json",
                {"status": "incomplete", "error": str(exc), "recorded_at": utc_now()},
            )
        except Exception:
            pass
        raise


def create_action_kernel_smoke(
    *, export_root: Path, output_dir: Path
) -> dict[str, Any]:
    command = [
        str(Path(sys.executable).resolve()),
        "-c",
        (
            "import sys; "
            "print('action-kernel-smoke'); "
            "print('action-kernel-stderr', file=sys.stderr)"
        ),
    ]
    return execute_local_process_action(
        export_root=export_root,
        output_dir=output_dir,
        command=command,
        timeout_seconds=15,
    )


def verify_action_kernel(*, output_dir: Path) -> dict[str, Any]:
    errors: list[str] = []
    result: dict[str, Any] | None = None
    manifest: dict[str, Any] | None = None
    process_report: dict[str, Any] | None = None
    session: dict[str, Any] | None = None
    replay: dict[str, Any] | None = None
    journal_events: list[dict[str, Any]] = []
    paths = {
        "result": output_dir / "action-kernel-result.json",
        "manifest": output_dir / "run-manifest.json",
        "journal": output_dir / "events.jsonl",
        "process": output_dir / "windows-process-result.json",
        "session": output_dir / "session-record.json",
    }

    try:
        result = load_json(paths["result"])
        validate_instance(
            result,
            RUNTIME_ROOT / "action-kernel-result-v0.1.schema.json",
            label="action-kernel result",
        )
    except PrototypeError as exc:
        errors.append(str(exc))
    try:
        manifest = load_json(paths["manifest"])
        validate_instance(
            manifest,
            RUNTIME_ROOT / "run-manifest-v0.1.schema.json",
            label="action-kernel run manifest",
        )
    except PrototypeError as exc:
        errors.append(str(exc))
    try:
        process_report = load_json(paths["process"])
        validate_instance(
            process_report,
            RUNTIME_ROOT / "windows-process-result-v0.1.schema.json",
            label="Windows process result",
        )
    except PrototypeError as exc:
        errors.append(str(exc))
    try:
        session = load_json(paths["session"])
        session_report = validate_session_record(session)
        if not session_report["valid"]:
            errors.extend(
                f"session invariant: {item['code']} at {item['location']}"
                for item in session_report["violations"]
            )
    except PrototypeError as exc:
        errors.append(str(exc))
    try:
        replay = replay_journal(
            run_manifest_path=paths["manifest"],
            journal_path=paths["journal"],
        )
        errors.extend(f"journal: {item}" for item in replay["errors"])
    except PrototypeError as exc:
        errors.append(str(exc))

    try:
        with paths["journal"].open("r", encoding="utf-8") as handle:
            journal_events = [json.loads(line) for line in handle if line.strip()]
    except (OSError, json.JSONDecodeError) as exc:
        errors.append(f"cannot inspect journal payloads: {exc}")

    actual_process_sha256: str | None = None
    actual_session_sha256: str | None = None
    try:
        actual_process_sha256 = sha256_file(paths["process"])
    except OSError as exc:
        errors.append(f"cannot hash process result: {exc}")
    try:
        actual_session_sha256 = sha256_file(paths["session"])
    except OSError as exc:
        errors.append(f"cannot hash session record: {exc}")

    if result is not None:
        for name, record in result["artifacts"].items():
            try:
                relative = safe_relative_path(record["path"])
                candidate = output_dir.joinpath(*relative.parts)
                actual = sha256_file(candidate)
                if actual != record["sha256"]:
                    errors.append(f"artifact digest mismatch: {name}")
            except (OSError, PrototypeError) as exc:
                errors.append(f"artifact verification failed for {name}: {exc}")

    if result is not None and manifest is not None:
        if result["run_id"] != manifest["run_id"]:
            errors.append("result run_id differs from manifest")
    if result is not None and process_report is not None:
        if result["action_state"] != process_report["process"]["terminal_state"]:
            errors.append("result action state differs from process result")
        if result["runtime_ready"] != process_report["runtime_ready"]:
            errors.append("result runtime readiness differs from process result")
    if result is not None and session is not None:
        if len(session["actions"]) != 1:
            errors.append("session must contain exactly one action")
        else:
            action = session["actions"][0]
            if action["state"] != result["action_state"]:
                errors.append("session action state differs from result")
        matching = [
            item for item in session["artifacts"] if item["artifact_id"] == ARTIFACT_ID
        ]
        if len(matching) != 1:
            errors.append("session must register exactly one process-result artifact")
        elif (
            actual_process_sha256 is not None
            and matching[0]["sha256"] != actual_process_sha256
        ):
            errors.append("session process artifact digest mismatch")
    if result is not None and replay is not None:
        if result["terminal_run_event"] != replay["terminal_event"]:
            errors.append("result terminal run event differs from journal")
        if result["journal_replay"]["event_count"] != replay["event_count"]:
            errors.append("result journal event count differs from replay")

    tool_completed_events = [
        event for event in journal_events if event.get("event_type") == "tool_completed"
    ]
    if len(tool_completed_events) != 1:
        errors.append(
            f"journal must contain exactly one tool_completed event; found {len(tool_completed_events)}"
        )
    else:
        completed = tool_completed_events[0]
        if (
            actual_process_sha256 is not None
            and completed.get("payload", {}).get("process_result_sha256")
            != actual_process_sha256
        ):
            errors.append("journal tool_completed process digest mismatch")
        if session is not None and session["session"].get("journal_head") != completed.get(
            "event_id"
        ):
            errors.append("session journal_head does not reference tool_completed")

    registration_events = [
        event
        for event in journal_events
        if event.get("event_type") == "artifact_registered"
    ]
    if len(registration_events) != 1:
        errors.append(
            "journal must contain exactly one artifact_registered event; "
            f"found {len(registration_events)}"
        )
    else:
        registrations = {
            item.get("artifact_id"): item
            for item in registration_events[0].get("payload", {}).get("artifacts", [])
            if isinstance(item, dict)
        }
        registered_process = registrations.get(ARTIFACT_ID)
        if registered_process is None:
            errors.append("journal does not register the process-result artifact")
        elif (
            actual_process_sha256 is not None
            and registered_process.get("sha256") != actual_process_sha256
        ):
            errors.append("journal registered process digest mismatch")
        registered_session = registrations.get("ART-SESSION-SNAPSHOT-001")
        if registered_session is None:
            errors.append("journal does not register the session snapshot")
        elif (
            actual_session_sha256 is not None
            and registered_session.get("sha256") != actual_session_sha256
        ):
            errors.append("journal registered session digest mismatch")

    return {
        "schema_version": "0.1.0-prototype",
        "valid": not errors,
        "output_dir": str(output_dir),
        "errors": sorted(set(errors)),
        "checked_at": utc_now(),
        "limitations": [
            "Verification establishes cross-file mechanics only, not scientific evidence or sandbox strength."
        ],
    }
