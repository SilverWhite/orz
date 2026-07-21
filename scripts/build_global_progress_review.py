#!/usr/bin/env python3
"""Build a deterministic no-model Global Progress Sentinel review artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
INPUT_SCHEMA = ROOT / "runtime" / "global-progress-input-v0.1.schema.json"
REVIEW_SCHEMA = ROOT / "runtime" / "global-progress-review-v0.1.schema.json"


class ReviewError(RuntimeError):
    pass


def _canonical_json(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ReviewError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ReviewError(f"expected JSON object: {path}")
    return value


def _validate(schema_path: Path, value: dict[str, Any]) -> None:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        detail = "; ".join(
            f"{'/'.join(map(str, error.absolute_path)) or '<root>'}: {error.message}"
            for error in errors[:8]
        )
        raise ReviewError(f"schema validation failed for {schema_path.name}: {detail}")


def _unique(values: list[str], label: str) -> None:
    if len(values) != len(set(values)):
        raise ReviewError(f"duplicate {label}")


def _semantic_validate(source: dict[str, Any]) -> None:
    contract_material = {
        "objective": source["task_contract"]["objective"],
        "acceptance": source["task_contract"]["acceptance"],
    }
    if _sha256(_canonical_json(contract_material)) != source["task_contract"]["sha256"]:
        raise ReviewError("task_contract sha256 does not match embedded objective and acceptance")
    acceptance_ids = [item["acceptance_ref"] for item in source["task_contract"]["acceptance"]]
    step_ids = [item["step_id"] for item in source["plan"]["steps"]]
    event_ids = [item["event_id"] for item in source["journal"]]
    _unique(acceptance_ids, "acceptance_ref")
    _unique(step_ids, "step_id")
    _unique(event_ids, "event_id")

    acceptance_set = set(acceptance_ids)
    step_by_id = {step["step_id"]: step for step in source["plan"]["steps"]}
    for step in source["plan"]["steps"]:
        unknown = set(step["acceptance_refs"]) - acceptance_set
        if unknown:
            raise ReviewError(f"step {step['step_id']} references unknown acceptance: {sorted(unknown)}")
        if step["state"] == "deferred" and not step["defer_reason"]:
            raise ReviewError(f"deferred step lacks defer_reason: {step['step_id']}")
        if step["state"] != "deferred" and step["defer_reason"] is not None:
            raise ReviewError(f"non-deferred step has defer_reason: {step['step_id']}")

    sequences = [event["sequence"] for event in source["journal"]]
    if sequences != sorted(sequences) or len(sequences) != len(set(sequences)):
        raise ReviewError("journal sequences must be strictly increasing and unique")
    for event in source["journal"]:
        step_id = event["step_id"]
        if step_id is not None:
            if step_id not in step_by_id:
                raise ReviewError(f"event {event['event_id']} references unknown step {step_id}")
            if event["direction_id"] != step_by_id[step_id]["direction_id"]:
                raise ReviewError(f"event {event['event_id']} direction does not match its step")
        if event["event_type"] == "action_terminal":
            if step_id is None or event["terminal_state"] is None:
                raise ReviewError(f"action_terminal lacks step/terminal state: {event['event_id']}")
        elif event["terminal_state"] is not None:
            raise ReviewError(f"non-terminal event has terminal_state: {event['event_id']}")
        if event["event_type"] == "verification_result":
            if step_id is None or event["verification_state"] is None:
                raise ReviewError(f"verification_result lacks step/state: {event['event_id']}")
        elif event["verification_state"] is not None:
            raise ReviewError(f"non-verification event has verification_state: {event['event_id']}")


def _warning(
    source: dict[str, Any],
    kind: str,
    message: str,
    subject_refs: list[str],
    source_refs: list[str],
) -> dict[str, Any]:
    head = source["journal"][-1]["event_id"]
    material = {
        "task_contract_sha256": source["task_contract"]["sha256"],
        "plan_revision": source["plan"]["revision"],
        "journal_head": head,
        "kind": kind,
        "subject_refs": sorted(subject_refs),
    }
    dedupe_key = _sha256(_canonical_json(material))
    return {
        "warning_id": f"GPW-{dedupe_key[:16]}",
        "kind": kind,
        "severity": "warn",
        "message": message,
        "subject_refs": sorted(subject_refs),
        "source_refs": sorted(source_refs),
        "dedupe_key": dedupe_key,
        "review_required": True,
    }


def build_review(source: dict[str, Any]) -> dict[str, Any]:
    _validate(INPUT_SCHEMA, source)
    _semantic_validate(source)
    source_digest = _sha256(_canonical_json(source))
    steps = sorted(source["plan"]["steps"], key=lambda item: item["step_id"])
    journal = source["journal"]
    events_by_step: dict[str, list[dict[str, Any]]] = {step["step_id"]: [] for step in steps}
    for event in journal:
        if event["step_id"] is not None:
            events_by_step[event["step_id"]].append(event)

    progress = []
    for step in steps:
        progress.append(
            {
                "step_id": step["step_id"],
                "direction_id": step["direction_id"],
                "state": step["state"],
                "acceptance_refs": sorted(step["acceptance_refs"]),
                "source_refs": [event["event_id"] for event in events_by_step[step["step_id"]]],
            }
        )

    direction_coverage = []
    for direction in sorted({step["direction_id"] for step in steps}):
        matching = [step for step in steps if step["direction_id"] == direction]
        counts = {state: 0 for state in ("pending", "in_progress", "completed", "deferred", "failed")}
        refs: list[str] = []
        for step in matching:
            counts[step["state"]] += 1
            refs.append(step["step_id"])
        direction_coverage.append(
            {"direction_id": direction, **counts, "source_refs": sorted(refs)}
        )

    latest_verification: dict[str, dict[str, Any]] = {}
    for event in journal:
        if event["event_type"] == "verification_result" and event["step_id"] is not None:
            latest_verification[event["step_id"]] = event

    acceptance_coverage = []
    for acceptance in sorted(
        source["task_contract"]["acceptance"], key=lambda item: item["acceptance_ref"]
    ):
        acceptance_ref = acceptance["acceptance_ref"]
        matching = [step for step in steps if acceptance_ref in step["acceptance_refs"]]
        if not matching:
            state = "uncovered"
        elif all(step["state"] == "deferred" for step in matching):
            state = "deferred"
        elif any(step["state"] == "in_progress" for step in matching):
            state = "in_progress"
        elif any(step["state"] == "pending" for step in matching):
            state = "planned"
        elif all(step["state"] == "completed" for step in matching):
            verified = all(
                latest_verification.get(step["step_id"], {}).get("verification_state") == "pass"
                for step in matching
            )
            state = "verified" if verified else "completed_unverified"
        else:
            state = "completed_unverified"
        step_ids = sorted(step["step_id"] for step in matching)
        source_refs = step_ids or [f"ACCEPTANCE:{acceptance_ref}"]
        acceptance_coverage.append(
            {
                "acceptance_ref": acceptance_ref,
                "state": state,
                "step_ids": step_ids,
                "source_refs": source_refs,
            }
        )

    candidates: list[dict[str, Any]] = []
    threshold = source["thresholds"]["direction_concentration_actions"]
    successful_actions = [
        event
        for event in journal
        if event["event_type"] == "action_terminal" and event["terminal_state"] == "succeeded"
    ]
    if len(successful_actions) >= threshold:
        recent = successful_actions[-threshold:]
        directions = {event["direction_id"] for event in recent}
        if len(directions) == 1:
            direction = next(iter(directions))
            other_active = sorted(
                {
                    step["direction_id"]
                    for step in steps
                    if step["direction_id"] != direction and step["state"] in {"pending", "in_progress"}
                }
            )
            if other_active:
                candidates.append(
                    _warning(
                        source,
                        "direction_concentration",
                        f"The last {threshold} successful actions all served {direction}; other active directions remain.",
                        [direction, *other_active],
                        [event["event_id"] for event in recent],
                    )
                )

    for item in acceptance_coverage:
        if item["state"] == "uncovered":
            candidates.append(
                _warning(
                    source,
                    "acceptance_uncovered",
                    f"Acceptance {item['acceptance_ref']} has no mapped plan step.",
                    [item["acceptance_ref"]],
                    item["source_refs"],
                )
            )

    for step in steps:
        step_events = events_by_step[step["step_id"]]
        write_events = [
            event
            for event in step_events
            if event["event_type"] == "action_terminal" and event["write_effect"]
        ]
        verification = latest_verification.get(step["step_id"])
        verification_state = verification["verification_state"] if verification else "missing"
        if step["state"] == "completed" and write_events and verification_state != "pass":
            refs = [event["event_id"] for event in write_events]
            if verification:
                refs.append(verification["event_id"])
            candidates.append(
                _warning(
                    source,
                    "verification_debt",
                    f"Completed write step {step['step_id']} has verification state {verification_state}.",
                    [step["step_id"], step["direction_id"]],
                    refs,
                )
            )

    prior = set(source["previous_warning_dedupe_keys"])
    warnings = sorted(
        [item for item in candidates if item["dedupe_key"] not in prior],
        key=lambda item: (item["kind"], item["warning_id"]),
    )
    suppressed = sorted(item["dedupe_key"] for item in candidates if item["dedupe_key"] in prior)
    review_material = {
        "source_input_sha256": source_digest,
        "task_id": source["task_id"],
        "plan_revision": source["plan"]["revision"],
        "journal_head": journal[-1]["event_id"],
    }
    review = {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-review",
        "review_id": f"GPR-{_sha256(_canonical_json(review_material))[:16]}",
        "source_input_sha256": source_digest,
        "task_id": source["task_id"],
        "task_contract_sha256": source["task_contract"]["sha256"],
        "plan_revision": source["plan"]["revision"],
        "review_cycle": source["plan"]["review_cycle"],
        "journal_span": {
            "first_sequence": journal[0]["sequence"],
            "last_sequence": journal[-1]["sequence"],
            "head_event_id": journal[-1]["event_id"],
        },
        "objective": {"text": source["task_contract"]["objective"], "state": "contract_derived"},
        "progress": progress,
        "direction_coverage": direction_coverage,
        "acceptance_coverage": acceptance_coverage,
        "unresolved": sorted(source["unresolved_user_constraints"], key=lambda item: item["constraint_id"]),
        "warnings": warnings,
        "suppressed_warning_dedupe_keys": suppressed,
        "next_review_trigger": "step_boundary_or_before_external_costly_or_destructive_action",
        "generator": {
            "name": "build_global_progress_review.py",
            "version": "0.1.0",
            "summary_state": "deterministic_structured_projection",
        },
    }
    _validate(REVIEW_SCHEMA, review)
    return review


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise ReviewError(f"refusing to overwrite output: {path}") from exc
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except BaseException:
        path.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        source = _load_json(Path(args.input))
        review = build_review(source)
        _write_new(Path(args.output), review)
    except ReviewError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(json.dumps({"valid": True, "review_id": review["review_id"], "warnings": len(review["warnings"])}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
