from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .execution_lock import verify_disposable_reproduction_execution_lock
from .journal_lock import exclusive_journal_lock
from .utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    exclusive_create_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
)


ROOT = ASSURANCE_ROOT.parent
RUNTIME_ROOT = ROOT / "runtime"
RUNNER_RECEIPT_SCHEMA = "gsa-no-model-runner-receipt-v0.1.schema.json"
RUNNER_RECOVERY_INSPECTION_SCHEMA = (
    "gsa-runner-journal-recovery-inspection-v0.1.schema.json"
)
RUNNER_RECOVERY_RECEIPT_SCHEMA = "gsa-runner-journal-recovery-receipt-v0.1.schema.json"
RUNNER_LIFECYCLE_REPAIR_POLICY_SCHEMA = (
    "gsa-runner-lifecycle-repair-policy-v0.1.schema.json"
)
PROOF_SCHEMA = "disposable-reproduction-run-proof-v0.1.schema.json"
PROJECTION_SCHEMA = "gsa-runtime-preflight-projection-v0.1.schema.json"
EXECUTION_LOCK_SCHEMA = "gsa-execution-lock-v0.1.schema.json"
TERMINAL_EVENTS = {"run_finished", "run_failed", "run_cancelled", "run_invalidated"}
FORBIDDEN_MODEL_TOOL_EVENTS = {
    "model_request",
    "model_output",
    "tool_proposal",
    "permission_decision",
    "tool_started",
    "tool_completed",
}


def _validate_runtime_contract(value: dict[str, Any], schema_name: str, *, label: str) -> None:
    schema = load_json(RUNTIME_ROOT / schema_name)
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda item: list(item.absolute_path),
    )
    if errors:
        rendered = [
            f"{label}#/{'/'.join(map(str, item.absolute_path))}: {item.message}"
            for item in errors
        ]
        raise AssuranceError("; ".join(rendered))


def _event_hash(event: dict[str, Any]) -> str:
    projection = dict(event)
    projection.pop("event_sha256", None)
    return sha256_bytes(canonical_bytes(projection))


def _event_line(event: dict[str, Any]) -> bytes:
    return (
        json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
    )


def replay_gsa_runner_journal(
    *,
    run_manifest_path: Path,
    journal_path: Path,
    require_terminal: bool = True,
) -> dict[str, Any]:
    run_manifest = load_json(run_manifest_path)
    _validate_runtime_contract(
        run_manifest,
        "run-manifest-v0.1.schema.json",
        label="runner run manifest",
    )
    expected_run_id = run_manifest["run_id"]
    expected_manifest_sha256 = sha256_file(run_manifest_path)
    errors: list[str] = []
    events: list[dict[str, Any]] = []
    if not journal_path.is_file():
        raise AssuranceError(f"runner journal does not exist: {journal_path}")
    for line_number, line in enumerate(journal_path.read_text(encoding="utf-8").splitlines(), 1):
        if not line:
            errors.append(f"blank journal line {line_number}")
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as exc:
            errors.append(f"invalid JSON at line {line_number}: {exc}")
            continue
        if not isinstance(event, dict):
            errors.append(f"journal line {line_number} is not an object")
            continue
        try:
            _validate_runtime_contract(
                event,
                "run-event-v0.1.schema.json",
                label=f"runner journal line {line_number}",
            )
        except AssuranceError as exc:
            errors.append(str(exc))
        events.append(event)

    previous_event_sha256: str | None = None
    terminal_events: list[str] = []
    for expected_sequence, event in enumerate(events):
        if event.get("sequence") != expected_sequence:
            errors.append(f"sequence mismatch at event {expected_sequence}")
        if event.get("run_id") != expected_run_id:
            errors.append(f"run ID mismatch at event {expected_sequence}")
        if event.get("run_manifest_sha256") != expected_manifest_sha256:
            errors.append(f"manifest digest mismatch at event {expected_sequence}")
        if event.get("previous_event_sha256") != previous_event_sha256:
            errors.append(f"previous-event digest mismatch at event {expected_sequence}")
        payload = event.get("payload")
        if isinstance(payload, dict) and event.get("payload_sha256") != sha256_bytes(
            canonical_bytes(payload)
        ):
            errors.append(f"payload digest mismatch at event {expected_sequence}")
        if event.get("event_sha256") != _event_hash(event):
            errors.append(f"event digest mismatch at event {expected_sequence}")
        if event.get("event_type") in FORBIDDEN_MODEL_TOOL_EVENTS:
            errors.append(f"model/tool event is forbidden at event {expected_sequence}")
        if event.get("event_type") in TERMINAL_EVENTS:
            terminal_events.append(event["event_type"])
            if expected_sequence != len(events) - 1:
                errors.append("terminal event is not last")
        previous_event_sha256 = event.get("event_sha256")
    if require_terminal and len(terminal_events) != 1:
        errors.append(f"expected exactly one terminal event, found {len(terminal_events)}")
    if len(terminal_events) > 1:
        errors.append("multiple terminal events")

    return {
        "schema_version": "0.1.0-draft",
        "replay_kind": "gsa_runner_journal_replay",
        "valid": not errors,
        "run_id": expected_run_id,
        "run_manifest_sha256": expected_manifest_sha256,
        "journal_sha256": sha256_file(journal_path),
        "event_count": len(events),
        "last_event_sha256": previous_event_sha256,
        "terminal_event": terminal_events[-1] if terminal_events else None,
        "errors": errors,
        "limitations": [
            "Replay verifies runner mechanics and does not score scientific correctness."
        ],
    }


def _append_projected_event_unlocked(
    *,
    journal_path: Path,
    event: dict[str, Any],
    run_manifest_path: Path,
    lifecycle_repair_policy: dict[str, Any] | None = None,
) -> None:
    run_manifest = load_json(run_manifest_path)
    expected_run_id = run_manifest["run_id"]
    expected_manifest_sha256 = sha256_file(run_manifest_path)
    previous_event_sha256: str | None = None
    expected_sequence = 0
    if journal_path.exists():
        replay = replay_gsa_runner_journal(
            run_manifest_path=run_manifest_path,
            journal_path=journal_path,
            require_terminal=False,
        )
        if replay["errors"]:
            if lifecycle_repair_policy is None:
                raise AssuranceError("refusing to append to invalid runner journal")
            _apply_lifecycle_repair_policy_unlocked(
                run_manifest_path=run_manifest_path,
                journal_path=journal_path,
                policy=lifecycle_repair_policy,
            )
            replay = replay_gsa_runner_journal(
                run_manifest_path=run_manifest_path,
                journal_path=journal_path,
                require_terminal=False,
            )
            if replay["errors"]:
                raise AssuranceError("refusing to append to invalid runner journal")
        expected_sequence = replay["event_count"]
        previous_event_sha256 = replay["last_event_sha256"]
        if replay["terminal_event"] is not None:
            raise AssuranceError("refusing to append after terminal runner event")

    if event["sequence"] != expected_sequence:
        raise AssuranceError("projected event sequence does not extend runner journal")
    if event["run_id"] != expected_run_id:
        raise AssuranceError("projected event run_id mismatch")
    if event["run_manifest_sha256"] != expected_manifest_sha256:
        raise AssuranceError("projected event manifest digest mismatch")
    if event["previous_event_sha256"] != previous_event_sha256:
        raise AssuranceError("projected event hash chain mismatch")
    if event["event_sha256"] != _event_hash(event):
        raise AssuranceError("projected event digest mismatch")
    _validate_runtime_contract(
        event,
        "run-event-v0.1.schema.json",
        label=f"projected runner event {event['sequence']}",
    )
    journal_path.parent.mkdir(parents=True, exist_ok=True)
    with journal_path.open("ab") as handle:
        handle.write(_event_line(event))
        handle.flush()
        os.fsync(handle.fileno())


def _append_projected_event(
    *,
    journal_path: Path,
    event: dict[str, Any],
    run_manifest_path: Path,
    lifecycle_repair_policy: dict[str, Any] | None = None,
) -> None:
    with exclusive_journal_lock(journal_path):
        _append_projected_event_unlocked(
            journal_path=journal_path,
            event=event,
            run_manifest_path=run_manifest_path,
            lifecycle_repair_policy=lifecycle_repair_policy,
        )


def _validate_journal_bytes(
    data: bytes,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> tuple[list[dict[str, Any]], list[str]]:
    errors: list[str] = []
    if not data or not data.endswith(b"\n"):
        return [], ["candidate journal is empty or lacks a final newline"]
    events: list[dict[str, Any]] = []
    previous_event_sha256: str | None = None
    for expected_sequence, line in enumerate(data.splitlines()):
        try:
            event = json.loads(line.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            errors.append(f"invalid JSON at line {expected_sequence + 1}: {exc}")
            continue
        if not isinstance(event, dict):
            errors.append(f"journal line {expected_sequence + 1} is not an object")
            continue
        try:
            _validate_runtime_contract(
                event,
                "run-event-v0.1.schema.json",
                label=f"runner journal line {expected_sequence + 1}",
            )
        except AssuranceError as exc:
            errors.append(str(exc))
        if event.get("sequence") != expected_sequence:
            errors.append(f"sequence mismatch at event {expected_sequence}")
        if event.get("run_id") != expected_run_id:
            errors.append(f"run ID mismatch at event {expected_sequence}")
        if event.get("run_manifest_sha256") != expected_manifest_sha256:
            errors.append(f"manifest digest mismatch at event {expected_sequence}")
        if event.get("previous_event_sha256") != previous_event_sha256:
            errors.append(f"previous-event digest mismatch at event {expected_sequence}")
        payload = event.get("payload")
        if not isinstance(payload, dict):
            errors.append(f"payload is not an object at event {expected_sequence}")
        elif event.get("payload_sha256") != sha256_bytes(canonical_bytes(payload)):
            errors.append(f"payload digest mismatch at event {expected_sequence}")
        if event.get("event_sha256") != _event_hash(event):
            errors.append(f"event digest mismatch at event {expected_sequence}")
        if (
            event.get("event_type") in TERMINAL_EVENTS
            and expected_sequence != len(data.splitlines()) - 1
        ):
            errors.append("terminal event is not last")
        previous_event_sha256 = event.get("event_sha256")
        events.append(event)
    return events, errors


def _inspect_journal_bytes(
    raw: bytes,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> dict[str, Any]:
    retained = raw
    discarded = b""
    events: list[dict[str, Any]] = []
    errors: list[str] = []
    if not raw:
        status = "unrecoverable"
        classification = "empty_journal"
        retained = b""
        errors = ["empty journal has no independently valid prefix"]
    elif raw.endswith(b"\n"):
        events, errors = _validate_journal_bytes(
            raw,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        )
        status = "valid" if not errors else "unrecoverable"
        classification = "valid_journal" if not errors else "history_corruption"
    else:
        normalized = raw + b"\n"
        events, errors = _validate_journal_bytes(
            normalized,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        )
        if not errors:
            status = "recoverable"
            classification = "missing_newline_normalized"
            retained = normalized
            discarded = b""
        else:
            last_newline = raw.rfind(b"\n")
            if last_newline < 0:
                status = "unrecoverable"
                classification = "no_valid_prefix"
                retained = b""
                discarded = raw
                events = []
            else:
                retained = raw[: last_newline + 1]
                discarded = raw[last_newline + 1 :]
                events, prefix_errors = _validate_journal_bytes(
                    retained,
                    expected_run_id=expected_run_id,
                    expected_manifest_sha256=expected_manifest_sha256,
                )
                if prefix_errors or not events or not discarded:
                    status = "unrecoverable"
                    classification = "history_corruption"
                    errors = [*prefix_errors, *errors]
                else:
                    status = "recoverable"
                    classification = "torn_tail_removed"
    return {
        "status": status,
        "classification": classification,
        "original": raw,
        "retained": retained,
        "discarded": discarded,
        "events": events,
        "errors": errors,
    }


def _inspection_public(
    inspection: dict[str, Any],
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> dict[str, Any]:
    raw = inspection["original"]
    retained = inspection["retained"]
    discarded = inspection["discarded"]
    events = inspection["events"]
    last_event = events[-1] if events else {}
    return {
        "schema_version": "0.1.0-draft",
        "inspection_kind": "gsa_runner_journal_recovery_inspection",
        "status": inspection["status"],
        "classification": inspection["classification"],
        "run_id": expected_run_id,
        "run_manifest_sha256": expected_manifest_sha256,
        "original_journal_sha256": sha256_bytes(raw),
        "original_size_bytes": len(raw),
        "retained_size_bytes": len(retained),
        "discarded_offset_bytes": len(retained) if discarded else len(raw),
        "discarded_size_bytes": len(discarded),
        "discarded_sha256": sha256_bytes(discarded) if discarded else None,
        "prefix_event_count": len(events),
        "prefix_last_event_sha256": last_event.get("event_sha256"),
        "prefix_terminal_event": (
            last_event.get("event_type")
            if last_event.get("event_type") in TERMINAL_EVENTS
            else None
        ),
        "errors": inspection["errors"],
        "limitations": [
            "Inspection is read-only and does not repair the journal.",
            "Advisory locking cannot constrain non-cooperating or multi-host writers.",
        ],
    }


def inspect_gsa_runner_journal_recovery(
    *,
    run_manifest_path: Path,
    journal_path: Path,
) -> dict[str, Any]:
    run_manifest = load_json(run_manifest_path)
    _validate_runtime_contract(
        run_manifest,
        "run-manifest-v0.1.schema.json",
        label="runner run manifest",
    )
    expected_run_id = run_manifest["run_id"]
    expected_manifest_sha256 = sha256_file(run_manifest_path)
    with exclusive_journal_lock(journal_path):
        raw = journal_path.read_bytes()
    inspection = _inspection_public(
        _inspect_journal_bytes(
            raw,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        ),
        expected_run_id=expected_run_id,
        expected_manifest_sha256=expected_manifest_sha256,
    )
    validate_contract(
        inspection,
        RUNNER_RECOVERY_INSPECTION_SCHEMA,
        label="GSA runner journal recovery inspection",
    )
    return inspection


def repair_gsa_runner_journal(
    *,
    run_manifest_path: Path,
    journal_path: Path,
    receipt_path: Path,
    quarantine_path: Path | None = None,
) -> dict[str, Any]:
    targets = [journal_path.resolve(), receipt_path.resolve()]
    if quarantine_path is not None:
        targets.append(quarantine_path.resolve())
    if len(targets) != len(set(targets)):
        raise AssuranceError("journal, receipt, and quarantine paths must be distinct")
    if receipt_path.exists():
        raise AssuranceError(f"refusing to overwrite recovery receipt: {receipt_path}")
    if quarantine_path is not None and quarantine_path.exists():
        raise AssuranceError(f"refusing to overwrite quarantine: {quarantine_path}")

    run_manifest = load_json(run_manifest_path)
    _validate_runtime_contract(
        run_manifest,
        "run-manifest-v0.1.schema.json",
        label="runner run manifest",
    )
    expected_run_id = run_manifest["run_id"]
    expected_manifest_sha256 = sha256_file(run_manifest_path)
    with exclusive_journal_lock(journal_path):
        return _repair_gsa_runner_journal_unlocked(
            run_manifest_path=run_manifest_path,
            journal_path=journal_path,
            receipt_path=receipt_path,
            quarantine_path=quarantine_path,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        )


def _repair_gsa_runner_journal_unlocked(
    *,
    run_manifest_path: Path,
    journal_path: Path,
    receipt_path: Path,
    quarantine_path: Path | None,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> dict[str, Any]:
        targets = [journal_path.resolve(), receipt_path.resolve()]
        if quarantine_path is not None:
            targets.append(quarantine_path.resolve())
        if len(targets) != len(set(targets)):
            raise AssuranceError("journal, receipt, and quarantine paths must be distinct")
        if receipt_path.exists():
            raise AssuranceError(f"refusing to overwrite recovery receipt: {receipt_path}")
        if quarantine_path is not None and quarantine_path.exists():
            raise AssuranceError(f"refusing to overwrite quarantine: {quarantine_path}")
        raw = journal_path.read_bytes()
        inspection = _inspect_journal_bytes(
            raw,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        )
        public = _inspection_public(
            inspection,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        )
        if inspection["status"] != "recoverable":
            raise AssuranceError(
                f"runner journal repair refused: {inspection['classification']}"
            )
        if inspection["classification"] == "torn_tail_removed" and quarantine_path is None:
            raise AssuranceError("torn-tail repair requires an explicit quarantine path")

        quarantine_sha256: str | None = None
        if inspection["discarded"]:
            assert quarantine_path is not None
            atomic_write_bytes(quarantine_path, inspection["discarded"])
            quarantine_sha256 = sha256_file(quarantine_path)

        atomic_write_bytes(journal_path, inspection["retained"], overwrite=True)
        replay = replay_gsa_runner_journal(
            run_manifest_path=run_manifest_path,
            journal_path=journal_path,
            require_terminal=public["prefix_terminal_event"] is not None,
        )
        if not replay["valid"]:
            atomic_write_bytes(journal_path, raw, overwrite=True)
            raise AssuranceError(
                f"repaired runner journal failed replay and was restored: {replay['errors']}"
            )

        receipt = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "gsa_runner_journal_recovery_receipt",
            "valid": True,
            "recovery_id": f"JRR-{public['original_journal_sha256'][:16].upper()}",
            "classification": inspection["classification"],
            "run_id": expected_run_id,
            "run_manifest_sha256": expected_manifest_sha256,
            "original_journal_sha256": public["original_journal_sha256"],
            "original_size_bytes": public["original_size_bytes"],
            "retained_size_bytes": public["retained_size_bytes"],
            "discarded_offset_bytes": public["discarded_offset_bytes"],
            "discarded_size_bytes": public["discarded_size_bytes"],
            "discarded_sha256": public["discarded_sha256"],
            "quarantine_path": str(quarantine_path) if quarantine_sha256 else None,
            "quarantine_sha256": quarantine_sha256,
            "receipt_path": str(receipt_path),
            "final_journal_sha256": sha256_file(journal_path),
            "final_event_count": replay["event_count"],
            "final_last_event_sha256": replay["last_event_sha256"],
            "final_terminal_event": replay["terminal_event"],
            "repaired_under_lock": True,
            "limitations": [
                "Repair proves mechanical prefix continuity, not semantic event truth.",
                "Advisory locking cannot constrain non-cooperating or multi-host writers.",
            ],
        }
        validate_contract(
            receipt,
            RUNNER_RECOVERY_RECEIPT_SCHEMA,
            label="GSA runner journal recovery receipt",
        )
        atomic_write_json(receipt_path, receipt)
        return receipt


def _apply_lifecycle_repair_policy_unlocked(
    *,
    run_manifest_path: Path,
    journal_path: Path,
    policy: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(
        policy,
        RUNNER_LIFECYCLE_REPAIR_POLICY_SCHEMA,
        label="GSA runner lifecycle repair policy",
    )
    run_manifest = load_json(run_manifest_path)
    expected_run_id = run_manifest["run_id"]
    expected_manifest_sha256 = sha256_file(run_manifest_path)
    raw = journal_path.read_bytes()
    inspection = _inspect_journal_bytes(
        raw,
        expected_run_id=expected_run_id,
        expected_manifest_sha256=expected_manifest_sha256,
    )
    if inspection["status"] != "recoverable":
        raise AssuranceError(
            f"runner lifecycle repair refused: {inspection['classification']}"
        )
    if policy["mode"] != "repair_recoverable":
        raise AssuranceError("runner lifecycle repair policy is inspect-only")
    if (
        inspection["classification"] == "missing_newline_normalized"
        and not policy["allow_missing_newline_normalization"]
    ):
        raise AssuranceError("lifecycle policy forbids missing-newline repair")
    if inspection["classification"] == "torn_tail_removed":
        if not policy["allow_torn_tail_quarantine"]:
            raise AssuranceError("lifecycle policy forbids torn-tail repair")
        if policy["quarantine_path"] is None:
            raise AssuranceError("torn-tail lifecycle repair requires quarantine_path")
    receipt_path = Path(policy["receipt_path"])
    quarantine_path = (
        Path(policy["quarantine_path"])
        if policy["quarantine_path"] is not None
        else None
    )
    return _repair_gsa_runner_journal_unlocked(
        run_manifest_path=run_manifest_path,
        journal_path=journal_path,
        receipt_path=receipt_path,
        quarantine_path=quarantine_path,
        expected_run_id=expected_run_id,
        expected_manifest_sha256=expected_manifest_sha256,
    )


def _verify_runner_inputs(
    proof: dict[str, Any],
    projection: dict[str, Any],
    execution_lock: dict[str, Any],
    *,
    execution_lock_journal_path: Path,
) -> None:
    validate_contract(
        proof,
        PROOF_SCHEMA,
        label="disposable reproduction run proof",
    )
    validate_contract(
        projection,
        PROJECTION_SCHEMA,
        label="GSA runtime preflight projection",
    )
    validate_contract(
        execution_lock,
        EXECUTION_LOCK_SCHEMA,
        label="GSA execution lock",
    )
    verify_disposable_reproduction_execution_lock(
        execution_lock,
        proof,
        projection,
        _journal_receipt_from_execution_lock(
            execution_lock,
            projection,
            execution_lock_journal_path,
        ),
        journal_path=execution_lock_journal_path,
    )


def _runner_receipt(
    proof: dict[str, Any],
    projection: dict[str, Any],
    execution_lock: dict[str, Any],
    *,
    run_manifest_path: Path,
    journal_path: Path,
    replay: dict[str, Any],
) -> dict[str, Any]:
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "gsa_no_model_runner_skeleton_receipt",
        "reproduction_id": projection["reproduction_id"],
        "run_id": projection["runtime_manifest"]["run_id"],
        "proof_sha256": sha256_bytes(canonical_bytes(proof)),
        "projection_sha256": sha256_bytes(canonical_bytes(projection)),
        "execution_lock_sha256": sha256_bytes(canonical_bytes(execution_lock)),
        "run_manifest_path": str(run_manifest_path),
        "run_manifest_sha256": sha256_file(run_manifest_path),
        "journal_path": str(journal_path),
        "journal_sha256": sha256_file(journal_path),
        "event_count": replay["event_count"],
        "terminal_event": replay["terminal_event"],
        "last_event_sha256": replay["last_event_sha256"],
        "checks": {
            "projection_schema_valid": True,
            "execution_lock_schema_valid": True,
            "execution_lock_matches_inputs": True,
            "manifest_written_canonical": True,
            "journal_appended_incrementally": True,
            "journal_replayed": True,
            "terminal_event_valid": True,
            "no_model_or_tool_events": True,
            "no_claim_promotion": True,
        },
        "evidence_boundary": {
            "runner_status": "no_model_runner_skeleton_only",
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
        },
        "valid": True,
    }
    validate_contract(
        receipt,
        RUNNER_RECEIPT_SCHEMA,
        label="GSA no-model runner receipt",
    )
    return receipt


def run_gsa_no_model_runner_skeleton(
    proof: dict[str, Any],
    projection: dict[str, Any],
    execution_lock: dict[str, Any],
    *,
    output_root: Path,
    execution_lock_journal_path: Path,
    lifecycle_repair_policy: dict[str, Any] | None = None,
) -> dict[str, Any]:
    _verify_runner_inputs(
        proof,
        projection,
        execution_lock,
        execution_lock_journal_path=execution_lock_journal_path,
    )
    if output_root.exists():
        raise AssuranceError(f"refusing to overwrite existing runner root: {output_root}")
    output_root.mkdir(parents=True)
    run_manifest_path = output_root / "run-manifest.json"
    journal_path = output_root / "run-journal.jsonl"
    exclusive_create_bytes(run_manifest_path, canonical_bytes(projection["runtime_manifest"]))
    for event in projection["runtime_events"]:
        _append_projected_event(
            journal_path=journal_path,
            event=event,
            run_manifest_path=run_manifest_path,
            lifecycle_repair_policy=lifecycle_repair_policy,
        )
    replay = replay_gsa_runner_journal(
        run_manifest_path=run_manifest_path,
        journal_path=journal_path,
    )
    if not replay["valid"]:
        raise AssuranceError(f"runner journal replay failed: {replay['errors']}")
    return _runner_receipt(
        proof,
        projection,
        execution_lock,
        run_manifest_path=run_manifest_path,
        journal_path=journal_path,
        replay=replay,
    )


def resume_gsa_no_model_runner_skeleton(
    proof: dict[str, Any],
    projection: dict[str, Any],
    execution_lock: dict[str, Any],
    *,
    runner_root: Path,
    execution_lock_journal_path: Path,
    lifecycle_repair_policy: dict[str, Any] | None = None,
) -> dict[str, Any]:
    _verify_runner_inputs(
        proof,
        projection,
        execution_lock,
        execution_lock_journal_path=execution_lock_journal_path,
    )
    if not runner_root.is_dir():
        raise AssuranceError(f"runner root does not exist: {runner_root}")
    run_manifest_path = runner_root / "run-manifest.json"
    journal_path = runner_root / "run-journal.jsonl"
    if canonical_bytes(load_json(run_manifest_path)) != canonical_bytes(
        projection["runtime_manifest"]
    ):
        raise AssuranceError("runner manifest does not match projection")
    if journal_path.exists():
        replay = replay_gsa_runner_journal(
            run_manifest_path=run_manifest_path,
            journal_path=journal_path,
            require_terminal=False,
        )
        if replay["errors"]:
            if lifecycle_repair_policy is None:
                raise AssuranceError("runner lifecycle found invalid journal")
            with exclusive_journal_lock(journal_path):
                _apply_lifecycle_repair_policy_unlocked(
                    run_manifest_path=run_manifest_path,
                    journal_path=journal_path,
                    policy=lifecycle_repair_policy,
                )
            replay = replay_gsa_runner_journal(
                run_manifest_path=run_manifest_path,
                journal_path=journal_path,
                require_terminal=False,
            )
            if replay["errors"]:
                raise AssuranceError("runner lifecycle found invalid journal")
        if replay["terminal_event"] is not None:
            if replay["event_count"] != len(projection["runtime_events"]):
                raise AssuranceError("runner lifecycle found premature terminal event")
            if replay["terminal_event"] != "run_finished":
                raise AssuranceError("runner lifecycle found non-success terminal event")
            start_sequence = replay["event_count"]
        else:
            start_sequence = replay["event_count"]
    else:
        start_sequence = 0
    for event in projection["runtime_events"][start_sequence:]:
        _append_projected_event(
            journal_path=journal_path,
            event=event,
            run_manifest_path=run_manifest_path,
            lifecycle_repair_policy=lifecycle_repair_policy,
        )
    replay = replay_gsa_runner_journal(
        run_manifest_path=run_manifest_path,
        journal_path=journal_path,
    )
    if not replay["valid"]:
        raise AssuranceError(f"runner journal replay failed: {replay['errors']}")
    return _runner_receipt(
        proof,
        projection,
        execution_lock,
        run_manifest_path=run_manifest_path,
        journal_path=journal_path,
        replay=replay,
    )


def verify_gsa_no_model_runner_skeleton(
    receipt: dict[str, Any],
    proof: dict[str, Any],
    projection: dict[str, Any],
    execution_lock: dict[str, Any],
    *,
    execution_lock_journal_path: Path,
) -> dict[str, Any]:
    validate_contract(
        receipt,
        RUNNER_RECEIPT_SCHEMA,
        label="GSA no-model runner receipt",
    )
    run_manifest_path = Path(receipt["run_manifest_path"])
    journal_path = Path(receipt["journal_path"])
    replay = replay_gsa_runner_journal(
        run_manifest_path=run_manifest_path,
        journal_path=journal_path,
    )
    if not replay["valid"]:
        raise AssuranceError(f"runner journal replay failed: {replay['errors']}")
    expected_manifest = load_json(run_manifest_path)
    if canonical_bytes(expected_manifest) != canonical_bytes(projection["runtime_manifest"]):
        raise AssuranceError("runner manifest does not match projection")
    expected_events = [
        json.loads(line)
        for line in journal_path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    if canonical_bytes(expected_events) != canonical_bytes(projection["runtime_events"]):
        raise AssuranceError("runner journal does not match projection")
    verify_disposable_reproduction_execution_lock(
        execution_lock,
        proof,
        projection,
        _journal_receipt_from_execution_lock(
            execution_lock,
            projection,
            execution_lock_journal_path,
        ),
        journal_path=execution_lock_journal_path,
    )
    expected = {
        "proof_sha256": sha256_bytes(canonical_bytes(proof)),
        "projection_sha256": sha256_bytes(canonical_bytes(projection)),
        "execution_lock_sha256": sha256_bytes(canonical_bytes(execution_lock)),
        "run_manifest_sha256": sha256_file(run_manifest_path),
        "journal_sha256": sha256_file(journal_path),
        "event_count": replay["event_count"],
        "terminal_event": replay["terminal_event"],
        "last_event_sha256": replay["last_event_sha256"],
    }
    for key, value in expected.items():
        if receipt[key] != value:
            raise AssuranceError(f"GSA no-model runner receipt mismatch: {key}")
    return {
        "schema_version": "0.1.0-draft",
        "verification_kind": "gsa_no_model_runner_skeleton_verification",
        "reproduction_id": receipt["reproduction_id"],
        "valid": True,
        "event_count": replay["event_count"],
        "terminal_event": replay["terminal_event"],
        "evidence_boundary": receipt["evidence_boundary"],
    }


def _journal_receipt_from_execution_lock(
    execution_lock: dict[str, Any],
    projection: dict[str, Any],
    journal_path: Path,
) -> dict[str, Any]:
    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "gsa_runtime_preflight_journal_receipt",
        "reproduction_id": projection["reproduction_id"],
        "projection_sha256": execution_lock["projection_sha256"],
        "run_id": projection["runtime_manifest"]["run_id"],
        "run_manifest_sha256": projection["run_manifest_sha256"],
        "journal_path": str(journal_path),
        "journal_sha256": sha256_file(journal_path),
        "event_count": 3,
        "first_event_sha256": projection["runtime_events"][0]["event_sha256"],
        "last_event_sha256": projection["runtime_events"][-1]["event_sha256"],
        "terminal_event": "run_finished",
        "checks": {
            "projection_schema_valid": True,
            "runtime_events_schema_valid": True,
            "hash_chain_valid": True,
            "journal_written_once": True,
            "journal_events_match_projection": True,
            "metadata_only": True,
            "no_model_or_tool_events": True,
            "formal_runner_not_claimed": True,
        },
        "evidence_boundary": {
            "runner_status": "runtime_journal_disposable_write_replay_only",
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
        },
        "valid": True,
    }
