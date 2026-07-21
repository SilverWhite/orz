#!/usr/bin/env python3
"""Independently rebuild and verify a Global Progress Sentinel review/disposition."""

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
DISPOSITION_SCHEMA = ROOT / "runtime" / "global-progress-disposition-v0.1.schema.json"
VERIFICATION_SCHEMA = ROOT / "runtime" / "global-progress-verification-v0.1.schema.json"


class VerificationError(RuntimeError):
    pass


def _canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")


def _digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _digest_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _read(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise VerificationError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise VerificationError(f"expected JSON object: {path}")
    return value


def _schema_errors(schema_path: Path, value: dict[str, Any], label: str) -> list[str]:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    failures = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda error: list(error.absolute_path),
    )
    return [
        f"{label}#/{'/'.join(map(str, failure.absolute_path))}: {failure.message}"
        for failure in failures
    ]


def _semantic_errors(source: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    contract_material = {
        "objective": source["task_contract"]["objective"],
        "acceptance": source["task_contract"]["acceptance"],
    }
    if _digest_bytes(_canonical(contract_material)) != source["task_contract"]["sha256"]:
        errors.append("task_contract sha256 does not match embedded objective and acceptance")
    acceptance_ids = [item["acceptance_ref"] for item in source["task_contract"]["acceptance"]]
    steps = source["plan"]["steps"]
    step_ids = [item["step_id"] for item in steps]
    event_ids = [item["event_id"] for item in source["journal"]]
    for values, label in ((acceptance_ids, "acceptance_ref"), (step_ids, "step_id"), (event_ids, "event_id")):
        if len(values) != len(set(values)):
            errors.append(f"duplicate {label}")
    acceptance_set = set(acceptance_ids)
    step_map = {item["step_id"]: item for item in steps}
    for step in steps:
        unknown = set(step["acceptance_refs"]) - acceptance_set
        if unknown:
            errors.append(f"step {step['step_id']} references unknown acceptance")
        if (step["state"] == "deferred") != bool(step["defer_reason"]):
            errors.append(f"step {step['step_id']} defer state/reason mismatch")
    sequences = [item["sequence"] for item in source["journal"]]
    if sequences != sorted(sequences) or len(sequences) != len(set(sequences)):
        errors.append("journal sequences are not strictly increasing and unique")
    for event in source["journal"]:
        step_id = event["step_id"]
        if step_id is not None:
            if step_id not in step_map:
                errors.append(f"event {event['event_id']} references unknown step")
                continue
            if event["direction_id"] != step_map[step_id]["direction_id"]:
                errors.append(f"event {event['event_id']} direction mismatch")
        if event["event_type"] == "action_terminal":
            if step_id is None or event["terminal_state"] is None:
                errors.append(f"event {event['event_id']} lacks terminal semantics")
        elif event["terminal_state"] is not None:
            errors.append(f"event {event['event_id']} has unexpected terminal state")
        if event["event_type"] == "verification_result":
            if step_id is None or event["verification_state"] is None:
                errors.append(f"event {event['event_id']} lacks verification semantics")
        elif event["verification_state"] is not None:
            errors.append(f"event {event['event_id']} has unexpected verification state")
    return errors


def _make_warning(
    source: dict[str, Any], kind: str, message: str, subjects: list[str], sources: list[str]
) -> dict[str, Any]:
    material = {
        "task_contract_sha256": source["task_contract"]["sha256"],
        "plan_revision": source["plan"]["revision"],
        "journal_head": source["journal"][-1]["event_id"],
        "kind": kind,
        "subject_refs": sorted(subjects),
    }
    key = _digest_bytes(_canonical(material))
    return {
        "warning_id": f"GPW-{key[:16]}",
        "kind": kind,
        "severity": "warn",
        "message": message,
        "subject_refs": sorted(subjects),
        "source_refs": sorted(sources),
        "dedupe_key": key,
        "review_required": True,
    }


def _rebuild(source: dict[str, Any]) -> dict[str, Any]:
    steps = sorted(source["plan"]["steps"], key=lambda item: item["step_id"])
    journal = source["journal"]
    by_step: dict[str, list[dict[str, Any]]] = {step["step_id"]: [] for step in steps}
    latest_verification: dict[str, dict[str, Any]] = {}
    for event in journal:
        step_id = event["step_id"]
        if step_id is not None:
            by_step[step_id].append(event)
        if event["event_type"] == "verification_result" and step_id is not None:
            latest_verification[step_id] = event

    progress = [
        {
            "step_id": step["step_id"],
            "direction_id": step["direction_id"],
            "state": step["state"],
            "acceptance_refs": sorted(step["acceptance_refs"]),
            "source_refs": [event["event_id"] for event in by_step[step["step_id"]]],
        }
        for step in steps
    ]
    direction_coverage = []
    for direction in sorted({step["direction_id"] for step in steps}):
        members = [step for step in steps if step["direction_id"] == direction]
        counts = {
            state: sum(step["state"] == state for step in members)
            for state in ("pending", "in_progress", "completed", "deferred", "failed")
        }
        direction_coverage.append(
            {
                "direction_id": direction,
                **counts,
                "source_refs": sorted(step["step_id"] for step in members),
            }
        )

    acceptance_coverage = []
    for acceptance in sorted(source["task_contract"]["acceptance"], key=lambda item: item["acceptance_ref"]):
        ref = acceptance["acceptance_ref"]
        members = [step for step in steps if ref in step["acceptance_refs"]]
        if not members:
            coverage_state = "uncovered"
        elif all(step["state"] == "deferred" for step in members):
            coverage_state = "deferred"
        elif any(step["state"] == "in_progress" for step in members):
            coverage_state = "in_progress"
        elif any(step["state"] == "pending" for step in members):
            coverage_state = "planned"
        elif all(step["state"] == "completed" for step in members):
            all_pass = all(
                latest_verification.get(step["step_id"], {}).get("verification_state") == "pass"
                for step in members
            )
            coverage_state = "verified" if all_pass else "completed_unverified"
        else:
            coverage_state = "completed_unverified"
        member_ids = sorted(step["step_id"] for step in members)
        acceptance_coverage.append(
            {
                "acceptance_ref": ref,
                "state": coverage_state,
                "step_ids": member_ids,
                "source_refs": member_ids or [f"ACCEPTANCE:{ref}"],
            }
        )

    candidates: list[dict[str, Any]] = []
    threshold = source["thresholds"]["direction_concentration_actions"]
    successes = [
        event
        for event in journal
        if event["event_type"] == "action_terminal" and event["terminal_state"] == "succeeded"
    ]
    if len(successes) >= threshold:
        recent = successes[-threshold:]
        recent_directions = {event["direction_id"] for event in recent}
        if len(recent_directions) == 1:
            direction = next(iter(recent_directions))
            alternatives = sorted(
                {
                    step["direction_id"]
                    for step in steps
                    if step["direction_id"] != direction and step["state"] in {"pending", "in_progress"}
                }
            )
            if alternatives:
                candidates.append(
                    _make_warning(
                        source,
                        "direction_concentration",
                        f"The last {threshold} successful actions all served {direction}; other active directions remain.",
                        [direction, *alternatives],
                        [event["event_id"] for event in recent],
                    )
                )
    for coverage in acceptance_coverage:
        if coverage["state"] == "uncovered":
            candidates.append(
                _make_warning(
                    source,
                    "acceptance_uncovered",
                    f"Acceptance {coverage['acceptance_ref']} has no mapped plan step.",
                    [coverage["acceptance_ref"]],
                    coverage["source_refs"],
                )
            )
    for step in steps:
        events = by_step[step["step_id"]]
        writes = [
            event
            for event in events
            if event["event_type"] == "action_terminal" and event["write_effect"]
        ]
        verification = latest_verification.get(step["step_id"])
        state = verification["verification_state"] if verification else "missing"
        if step["state"] == "completed" and writes and state != "pass":
            refs = [event["event_id"] for event in writes]
            if verification:
                refs.append(verification["event_id"])
            candidates.append(
                _make_warning(
                    source,
                    "verification_debt",
                    f"Completed write step {step['step_id']} has verification state {state}.",
                    [step["step_id"], step["direction_id"]],
                    refs,
                )
            )

    previous = set(source["previous_warning_dedupe_keys"])
    active = sorted(
        [warning for warning in candidates if warning["dedupe_key"] not in previous],
        key=lambda warning: (warning["kind"], warning["warning_id"]),
    )
    suppressed = sorted(warning["dedupe_key"] for warning in candidates if warning["dedupe_key"] in previous)
    source_digest = _digest_bytes(_canonical(source))
    identity = {
        "source_input_sha256": source_digest,
        "task_id": source["task_id"],
        "plan_revision": source["plan"]["revision"],
        "journal_head": journal[-1]["event_id"],
    }
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-review",
        "review_id": f"GPR-{_digest_bytes(_canonical(identity))[:16]}",
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
        "warnings": active,
        "suppressed_warning_dedupe_keys": suppressed,
        "next_review_trigger": "step_boundary_or_before_external_costly_or_destructive_action",
        "generator": {
            "name": "build_global_progress_review.py",
            "version": "0.1.0",
            "summary_state": "deterministic_structured_projection",
        },
    }


def _decision_errors(source: dict[str, Any], review: dict[str, Any], disposition: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    expected_warning_ids = {warning["warning_id"] for warning in review.get("warnings", [])}
    actual = disposition.get("warning_dispositions", [])
    actual_ids = [item.get("warning_id") for item in actual if isinstance(item, dict)]
    if len(actual_ids) != len(set(actual_ids)) or set(actual_ids) != expected_warning_ids:
        errors.append("warning dispositions do not exactly cover active warnings")

    step_map = {step["step_id"]: step for step in source["plan"]["steps"]}
    selected = disposition.get("selected_step_id")
    decision = disposition.get("decision")
    if decision in {"continue", "pivot"}:
        if selected not in step_map or step_map[selected]["state"] not in {"pending", "in_progress"}:
            errors.append("continue/pivot requires an active selected step")
        else:
            acceptance_refs = set(disposition.get("acceptance_refs", []))
            if not acceptance_refs or not acceptance_refs.issubset(set(step_map[selected]["acceptance_refs"])):
                errors.append("selected step does not support the disposition acceptance refs")
    elif selected is not None and selected not in step_map:
        errors.append("selected_step_id is unknown")

    warning_by_id = {warning["warning_id"]: warning for warning in review.get("warnings", [])}
    for item in actual:
        if item.get("status") != "reasoned_continue":
            continue
        if decision != "continue" or selected not in step_map:
            errors.append("reasoned_continue requires decision=continue and an active selected step")
            continue
        warning = warning_by_id.get(item.get("warning_id"))
        if warning and warning["kind"] == "direction_concentration":
            if step_map[selected]["direction_id"] not in warning["subject_refs"]:
                errors.append("reasoned_continue selected step is unrelated to direction warning")
    return errors


def verify(input_path: Path, review_path: Path, disposition_path: Path) -> dict[str, Any]:
    source = _read(input_path)
    review = _read(review_path)
    disposition = _read(disposition_path)
    errors: list[str] = []
    checks = {
        "input_schema_valid": False,
        "review_schema_valid": False,
        "review_rebuilt_exactly": False,
        "disposition_schema_valid": False,
        "review_linkage_valid": False,
        "warning_dispositions_complete": False,
        "decision_semantics_valid": False,
    }
    input_failures = _schema_errors(INPUT_SCHEMA, source, "input")
    semantic_failures = [] if input_failures else _semantic_errors(source)
    errors.extend(input_failures)
    errors.extend(semantic_failures)
    checks["input_schema_valid"] = not input_failures and not semantic_failures

    review_failures = _schema_errors(REVIEW_SCHEMA, review, "review")
    errors.extend(review_failures)
    checks["review_schema_valid"] = not review_failures
    expected: dict[str, Any] | None = None
    if checks["input_schema_valid"]:
        expected = _rebuild(source)
        checks["review_rebuilt_exactly"] = _canonical(review) == _canonical(expected)
        if not checks["review_rebuilt_exactly"]:
            errors.append("review does not exactly match independent reconstruction")

    disposition_failures = _schema_errors(DISPOSITION_SCHEMA, disposition, "disposition")
    errors.extend(disposition_failures)
    checks["disposition_schema_valid"] = not disposition_failures
    review_digest = _digest_file(review_path)
    checks["review_linkage_valid"] = (
        disposition.get("review_id") == review.get("review_id")
        and disposition.get("review_sha256") == review_digest
    )
    if not checks["review_linkage_valid"]:
        errors.append("disposition review linkage is invalid")

    if not disposition_failures and checks["input_schema_valid"]:
        semantic = _decision_errors(source, review, disposition)
        errors.extend(semantic)
        checks["warning_dispositions_complete"] = not any(
            error == "warning dispositions do not exactly cover active warnings" for error in semantic
        )
        checks["decision_semantics_valid"] = not any(
            error != "warning dispositions do not exactly cover active warnings" for error in semantic
        )

    valid = all(checks.values()) and not errors
    report = {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-verification",
        "valid": valid,
        "review_id": review.get("review_id") if isinstance(review.get("review_id"), str) else None,
        "input_sha256": _digest_file(input_path),
        "review_sha256": review_digest,
        "disposition_sha256": _digest_file(disposition_path),
        "warning_count": len(review.get("warnings", [])) if isinstance(review.get("warnings"), list) else 0,
        "checks": checks,
        "errors": errors,
    }
    verification_failures = _schema_errors(VERIFICATION_SCHEMA, report, "verification")
    if verification_failures:
        raise VerificationError("generated verification report failed its schema: " + "; ".join(verification_failures))
    return report


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise VerificationError(f"refusing to overwrite output: {path}") from exc
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
    parser.add_argument("--review", required=True)
    parser.add_argument("--disposition", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        report = verify(
            Path(args.input), Path(args.review), Path(args.disposition)
        )
        _write_new(Path(args.output), report)
    except VerificationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps({"valid": report["valid"], "checks": report["checks"]}))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
