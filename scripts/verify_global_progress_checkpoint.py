#!/usr/bin/env python3
"""Independently reconstruct and verify a source-bound Global Progress checkpoint."""

from __future__ import annotations

import argparse
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from verify_global_progress_holistic_review import (
    verify as independently_verify_holistic_review,
)
from verify_global_progress_review import verify as independently_verify_progress_review


ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "runtime"
SCHEMAS = {
    "input": RUNTIME / "global-progress-input-v0.1.schema.json",
    "review": RUNTIME / "global-progress-review-v0.1.schema.json",
    "disposition": RUNTIME / "global-progress-disposition-v0.1.schema.json",
    "verification": RUNTIME / "global-progress-verification-v0.1.schema.json",
    "policy": RUNTIME / "global-progress-checkpoint-policy-v0.1.schema.json",
    "history": RUNTIME / "global-progress-history-input-v0.1.schema.json",
    "holistic_review": RUNTIME / "global-progress-holistic-review-v0.1.schema.json",
    "holistic_disposition": (
        RUNTIME / "global-progress-holistic-disposition-v0.1.schema.json"
    ),
    "holistic_verification": (
        RUNTIME / "global-progress-holistic-verification-v0.1.schema.json"
    ),
    "checkpoint_verification": (
        RUNTIME / "global-progress-checkpoint-verification-v0.1.schema.json"
    ),
}
ADAPTER_FIELDS = {
    "task_id",
    "task_contract_sha256",
    "source_verification_sha256",
    "checkpoint_policy_sha256",
    "focus_window_consumption",
}


class CheckpointVerificationError(RuntimeError):
    pass


def _canonical(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


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
        raise CheckpointVerificationError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise CheckpointVerificationError(f"expected JSON object: {path}")
    return value


def _schema_errors(
    schema_path: Path,
    value: dict[str, Any],
    label: str,
) -> list[str]:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    failures = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda error: list(error.absolute_path),
    )
    return [
        f"{label}#/{'/'.join(map(str, failure.absolute_path))}: {failure.message}"
        for failure in failures
    ]


def _checkpoint_digest(checkpoint: dict[str, Any]) -> str:
    material = {
        key: value for key, value in checkpoint.items() if key != "checkpoint_sha256"
    }
    return _digest_bytes(_canonical(material))


def _checkpoint_schema_errors(
    checkpoint: dict[str, Any],
    label: str,
) -> list[str]:
    history_schema = json.loads(SCHEMAS["history"].read_text(encoding="utf-8"))
    checkpoint_schema = {
        "$schema": history_schema["$schema"],
        "$defs": history_schema["$defs"],
        **history_schema["$defs"]["checkpoint"],
    }
    failures = sorted(
        Draft202012Validator(
            checkpoint_schema,
            format_checker=FormatChecker(),
        ).iter_errors(checkpoint),
        key=lambda error: list(error.absolute_path),
    )
    errors = [
        f"{label}#/{'/'.join(map(str, failure.absolute_path))}: {failure.message}"
        for failure in failures
    ]
    missing = sorted(ADAPTER_FIELDS - set(checkpoint))
    if missing:
        errors.append(f"{label} lacks adapter fields: {missing}")
    return errors


def _source_errors(
    paths: dict[str, Path],
    values: dict[str, dict[str, Any]],
) -> tuple[list[str], list[str]]:
    semantic: list[str] = []
    digest_errors: list[str] = []
    for key in ("input", "review", "disposition", "verification", "policy"):
        semantic.extend(_schema_errors(SCHEMAS[key], values[key], key))
    if semantic:
        return semantic, digest_errors

    source = values["input"]
    review = values["review"]
    disposition = values["disposition"]
    verification = values["verification"]
    policy = values["policy"]
    if not verification["valid"] or not all(verification["checks"].values()):
        semantic.append("source verification is not fully valid")
    expected_verification = independently_verify_progress_review(
        paths["input"],
        paths["review"],
        paths["disposition"],
    )
    if _canonical(expected_verification) != _canonical(verification):
        semantic.append("source verification differs from independent recomputation")
    for field, source_key in (
        ("input_sha256", "input"),
        ("review_sha256", "review"),
        ("disposition_sha256", "disposition"),
    ):
        if verification[field] != _digest_file(paths[source_key]):
            digest_errors.append(f"source verification {field} mismatch")
    if (
        verification["review_id"] != review["review_id"]
        or disposition["review_id"] != review["review_id"]
        or disposition["review_sha256"] != _digest_file(paths["review"])
    ):
        semantic.append("source review/disposition linkage mismatch")
    if review["source_input_sha256"] != _digest_bytes(_canonical(source)):
        digest_errors.append("review source input digest mismatch")
    if (
        review["task_id"] != source["task_id"]
        or review["task_contract_sha256"] != source["task_contract"]["sha256"]
        or review["plan_revision"] != source["plan"]["revision"]
        or review["review_cycle"] != source["plan"]["review_cycle"]
        or review["journal_span"]["last_sequence"] != source["journal"][-1]["sequence"]
        or review["journal_span"]["head_event_id"] != source["journal"][-1]["event_id"]
    ):
        semantic.append("review does not project the current input head")
    if (
        policy["task_id"] != source["task_id"]
        or policy["task_contract_sha256"] != source["task_contract"]["sha256"]
    ):
        semantic.append("policy task-contract binding mismatch")

    directions = [item["direction_id"] for item in policy["critical_directions"]]
    acceptance_refs = [
        acceptance_ref
        for item in policy["critical_directions"]
        for acceptance_ref in item["acceptance_refs"]
    ]
    if len(directions) != len(set(directions)):
        semantic.append("policy repeats a critical direction")
    if len(acceptance_refs) != len(set(acceptance_refs)):
        semantic.append("policy repeats a critical acceptance")
    plan_directions = {step["direction_id"] for step in source["plan"]["steps"]}
    contract_acceptance = {
        item["acceptance_ref"] for item in source["task_contract"]["acceptance"]
    }
    if set(directions) - plan_directions:
        semantic.append("policy has an unknown direction")
    if set(acceptance_refs) - contract_acceptance:
        semantic.append("policy has an unknown acceptance")
    return semantic, digest_errors


def _previous_errors(
    previous: dict[str, Any] | None,
    source: dict[str, Any],
    policy_path: Path,
) -> list[str]:
    if previous is None:
        return []
    errors = _checkpoint_schema_errors(previous, "previous_checkpoint")
    if not errors and previous["checkpoint_sha256"] != _checkpoint_digest(previous):
        errors.append("previous checkpoint digest mismatch")
    if not errors and (
        previous["task_id"] != source["task_id"]
        or previous["task_contract_sha256"] != source["task_contract"]["sha256"]
    ):
        errors.append("previous checkpoint task-contract mismatch")
    if (
        not errors
        and previous["checkpoint_policy_sha256"] != _digest_file(policy_path)
    ):
        errors.append("checkpoint policy changed without an explicit revision")
    if not errors and source["plan"]["review_cycle"] <= previous["review_cycle"]:
        errors.append("review cycle did not advance")
    if (
        not errors
        and source["journal"][-1]["sequence"] <= previous["journal_head_sequence"]
    ):
        errors.append("journal head did not advance")
    return errors


def _focus_errors_and_control(
    paths: dict[str, Path],
    values: dict[str, dict[str, Any]],
    previous: dict[str, Any] | None,
    focus_present: bool,
) -> tuple[list[str], dict[str, Any] | None]:
    if not focus_present:
        return [], None
    if previous is None:
        return ["focus enforcement lacks a previous checkpoint"], None
    errors: list[str] = []
    schema_map = {
        "focus_history": "history",
        "focus_review": "holistic_review",
        "focus_disposition": "holistic_disposition",
        "focus_verification": "holistic_verification",
    }
    for key, schema_key in schema_map.items():
        errors.extend(_schema_errors(SCHEMAS[schema_key], values[key], key))
    if errors:
        return errors, None
    history = values["focus_history"]
    review = values["focus_review"]
    disposition = values["focus_disposition"]
    verification = values["focus_verification"]
    if not verification["valid"] or not all(verification["checks"].values()):
        errors.append("focus verification is not fully valid")
    expected_verification = independently_verify_holistic_review(
        paths["focus_history"],
        paths["focus_review"],
        paths["focus_disposition"],
    )
    if _canonical(expected_verification) != _canonical(verification):
        errors.append("focus verification differs from independent recomputation")
    for field, source_key in (
        ("input_sha256", "focus_history"),
        ("assessment_sha256", "focus_review"),
        ("disposition_sha256", "focus_disposition"),
    ):
        if verification[field] != _digest_file(paths[source_key]):
            errors.append(f"focus verification {field} mismatch")
    if (
        disposition["assessment_id"] != review["assessment_id"]
        or disposition["assessment_sha256"] != _digest_file(paths["focus_review"])
        or verification["assessment_id"] != review["assessment_id"]
    ):
        errors.append("focus review/disposition linkage mismatch")
    if history["checkpoints"][-1] != previous:
        errors.append("focus history head differs from previous checkpoint")
    if review["window"]["last_checkpoint_id"] != previous["checkpoint_id"]:
        errors.append("focus review does not end at previous checkpoint")
    control = disposition["bounded_focus"]
    if disposition["decision"] != "continue" or control is None:
        errors.append("focus disposition has no bounded continuation")
    return errors, control if not errors else None


def _rebuild(
    paths: dict[str, Path],
    values: dict[str, dict[str, Any]],
    previous: dict[str, Any] | None,
    focus: dict[str, Any] | None,
) -> dict[str, Any]:
    source = values["input"]
    review = values["review"]
    disposition = values["disposition"]
    policy = values["policy"]
    prior_head = previous["journal_head_sequence"] if previous else -1
    events = [event for event in source["journal"] if event["sequence"] > prior_head]
    steps = source["plan"]["steps"]
    step_by_id = {step["step_id"]: step for step in steps}
    directions = sorted({step["direction_id"] for step in steps})
    counts: dict[str, int] = defaultdict(int)
    for event in events:
        if (
            event["event_type"] == "action_terminal"
            and event["direction_id"] is not None
        ):
            counts[event["direction_id"]] += 1

    critical = sorted(
        item["direction_id"] for item in policy["critical_directions"]
    )
    deferred = []
    for direction in critical:
        direction_steps = [
            step for step in steps if step["direction_id"] == direction
        ]
        if direction_steps and all(step["state"] == "deferred" for step in direction_steps):
            deferred.append(direction)
    active = sorted(
        {
            step["direction_id"]
            for step in steps
            if step["state"] in {"pending", "in_progress"}
        }
        - set(deferred)
    )
    coverage = {
        item["acceptance_ref"]: item["state"]
        for item in review["acceptance_coverage"]
    }
    critical_acceptance = sorted(
        {
            acceptance_ref
            for item in policy["critical_directions"]
            for acceptance_ref in item["acceptance_refs"]
        }
    )
    states = [
        {"acceptance_ref": acceptance_ref, "state": coverage[acceptance_ref]}
        for acceptance_ref in critical_acceptance
    ]
    prior_states = (
        {
            item["acceptance_ref"]: item["state"]
            for item in previous["critical_acceptance_states"]
        }
        if previous
        else {}
    )
    verified_added = sorted(
        item["acceptance_ref"]
        for item in states
        if item["state"] == "verified"
        and prior_states.get(item["acceptance_ref"]) != "verified"
    )
    selected_step = disposition["selected_step_id"]
    selected_direction = (
        step_by_id[selected_step]["direction_id"] if selected_step else None
    )
    focus_consumption = None
    if focus:
        observed = counts[focus["direction_id"]]
        focus_consumption = {
            "disposition_id": values["focus_disposition"]["disposition_id"],
            "disposition_sha256": _digest_file(paths["focus_disposition"]),
            "authorized_direction_id": focus["direction_id"],
            "max_additional_actions": focus["max_additional_actions"],
            "observed_actions": observed,
            "within_action_limit": observed <= focus["max_additional_actions"],
            "review_by_cycle": focus["review_by_cycle"],
            "within_review_deadline": (
                source["plan"]["review_cycle"] <= focus["review_by_cycle"]
            ),
        }
    identity = {
        "task_id": source["task_id"],
        "review_cycle": source["plan"]["review_cycle"],
        "journal_head_sequence": source["journal"][-1]["sequence"],
        "review_sha256": _digest_file(paths["review"]),
        "previous_checkpoint_sha256": (
            previous["checkpoint_sha256"] if previous else None
        ),
    }
    result = {
        "checkpoint_id": f"GPC-{_digest_bytes(_canonical(identity))[:16].upper()}",
        "task_id": source["task_id"],
        "task_contract_sha256": source["task_contract"]["sha256"],
        "review_cycle": source["plan"]["review_cycle"],
        "plan_revision": source["plan"]["revision"],
        "journal_head_sequence": source["journal"][-1]["sequence"],
        "review_id": review["review_id"],
        "review_sha256": _digest_file(paths["review"]),
        "disposition_id": disposition["disposition_id"],
        "disposition_sha256": _digest_file(paths["disposition"]),
        "source_verification_sha256": _digest_file(paths["verification"]),
        "checkpoint_policy_sha256": _digest_file(paths["policy"]),
        "decision": disposition["decision"],
        "selected_direction_id": selected_direction,
        "direction_action_counts": [
            {"direction_id": direction, "action_count": counts[direction]}
            for direction in directions
        ],
        "active_direction_ids": active,
        "critical_direction_ids": critical,
        "deferred_direction_ids": deferred,
        "novel_evidence_refs": sorted(
            event["event_id"]
            for event in events
            if event["event_type"] == "artifact_registered"
        ),
        "verified_acceptance_refs_added": verified_added,
        "critical_acceptance_states": states,
        "unresolved_constraint_refs": sorted(
            item["constraint_id"] for item in review["unresolved"]
        ),
        "focus_window_consumption": focus_consumption,
        "previous_checkpoint_sha256": (
            previous["checkpoint_sha256"] if previous else None
        ),
        "checkpoint_sha256": "",
    }
    result["checkpoint_sha256"] = _checkpoint_digest(result)
    return result


def verify(
    paths: dict[str, Path],
    values: dict[str, dict[str, Any]],
    previous: dict[str, Any] | None,
    focus_present: bool,
) -> dict[str, Any]:
    checkpoint = values["checkpoint"]
    errors: list[str] = []
    checks = {
        "source_bundle_valid": False,
        "source_artifact_digests_valid": False,
        "previous_checkpoint_valid": False,
        "focus_artifact_bundle_valid": False,
        "checkpoint_schema_valid": False,
        "checkpoint_rebuilt_exactly": False,
        "checkpoint_hash_valid": False,
        "focus_window_within_bounds": False,
    }
    source_errors, digest_errors = _source_errors(paths, values)
    errors.extend(source_errors)
    errors.extend(digest_errors)
    checks["source_bundle_valid"] = not source_errors
    checks["source_artifact_digests_valid"] = not digest_errors

    previous_failures = (
        _previous_errors(previous, values["input"], paths["policy"])
        if checks["source_bundle_valid"]
        else ["previous checkpoint not evaluated because source bundle is invalid"]
    )
    errors.extend(previous_failures)
    checks["previous_checkpoint_valid"] = not previous_failures

    focus_errors, focus = _focus_errors_and_control(
        paths, values, previous, focus_present
    )
    errors.extend(focus_errors)
    checks["focus_artifact_bundle_valid"] = not focus_errors

    checkpoint_failures = _checkpoint_schema_errors(checkpoint, "checkpoint")
    errors.extend(checkpoint_failures)
    checks["checkpoint_schema_valid"] = not checkpoint_failures
    if checks["checkpoint_schema_valid"]:
        checks["checkpoint_hash_valid"] = (
            checkpoint["checkpoint_sha256"] == _checkpoint_digest(checkpoint)
        )
        if not checks["checkpoint_hash_valid"]:
            errors.append("checkpoint_sha256 mismatch")

    if (
        checks["source_bundle_valid"]
        and checks["source_artifact_digests_valid"]
        and checks["previous_checkpoint_valid"]
        and checks["focus_artifact_bundle_valid"]
    ):
        expected = _rebuild(paths, values, previous, focus)
        checks["checkpoint_rebuilt_exactly"] = (
            _canonical(expected) == _canonical(checkpoint)
        )
        if not checks["checkpoint_rebuilt_exactly"]:
            errors.append("checkpoint does not match independent reconstruction")

    consumption = checkpoint.get("focus_window_consumption")
    checks["focus_window_within_bounds"] = (
        consumption is None
        if not focus_present
        else isinstance(consumption, dict)
        and consumption.get("within_action_limit") is True
        and consumption.get("within_review_deadline") is True
    )
    if not checks["focus_window_within_bounds"]:
        errors.append("bounded-focus action limit or review deadline was exceeded")

    report = {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-checkpoint-verification",
        "valid": all(checks.values()) and not errors,
        "checkpoint_id": checkpoint.get("checkpoint_id")
        if isinstance(checkpoint.get("checkpoint_id"), str)
        else None,
        "input_sha256": _digest_file(paths["input"]),
        "review_sha256": _digest_file(paths["review"]),
        "disposition_sha256": _digest_file(paths["disposition"]),
        "checkpoint_sha256": _digest_file(paths["checkpoint"]),
        "checks": checks,
        "errors": errors,
    }
    generated_errors = _schema_errors(
        SCHEMAS["checkpoint_verification"],
        report,
        "generated_verification",
    )
    if generated_errors:
        raise CheckpointVerificationError("; ".join(generated_errors))
    return report


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise CheckpointVerificationError(
            f"refusing to overwrite output: {path}"
        ) from exc
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
    for name in (
        "input",
        "review",
        "disposition",
        "verification",
        "policy",
        "checkpoint",
    ):
        parser.add_argument(f"--{name}", required=True)
    parser.add_argument("--previous-checkpoint")
    parser.add_argument("--focus-history")
    parser.add_argument("--focus-review")
    parser.add_argument("--focus-disposition")
    parser.add_argument("--focus-verification")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    base_names = (
        "input",
        "review",
        "disposition",
        "verification",
        "policy",
        "checkpoint",
    )
    focus_names = (
        "focus_history",
        "focus_review",
        "focus_disposition",
        "focus_verification",
    )
    focus_arguments = [getattr(args, name) for name in focus_names]
    try:
        if any(focus_arguments) and not all(focus_arguments):
            raise CheckpointVerificationError(
                "focus artifact arguments must be supplied together"
            )
        paths = {name: Path(getattr(args, name)) for name in base_names}
        values = {name: _read(path) for name, path in paths.items()}
        previous = (
            _read(Path(args.previous_checkpoint))
            if args.previous_checkpoint
            else None
        )
        if all(focus_arguments):
            for name in focus_names:
                paths[name] = Path(getattr(args, name))
                values[name] = _read(paths[name])
        report = verify(paths, values, previous, all(focus_arguments))
        _write_new(Path(args.output), report)
    except CheckpointVerificationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps({"valid": report["valid"], "checks": report["checks"]}))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
