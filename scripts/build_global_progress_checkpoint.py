#!/usr/bin/env python3
"""Derive one source-bound Global Progress checkpoint without a model call."""

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
}
ADAPTER_REQUIRED = {
    "task_id",
    "task_contract_sha256",
    "source_verification_sha256",
    "checkpoint_policy_sha256",
    "focus_window_consumption",
}


class CheckpointError(RuntimeError):
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
        raise CheckpointError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise CheckpointError(f"expected JSON object: {path}")
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


def _checkpoint_errors(checkpoint: dict[str, Any], label: str) -> list[str]:
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
    missing = sorted(ADAPTER_REQUIRED - set(checkpoint))
    if missing:
        errors.append(f"{label} lacks adapter provenance fields: {missing}")
    if not failures and checkpoint.get("checkpoint_sha256") != _checkpoint_digest(checkpoint):
        errors.append(f"{label} checkpoint_sha256 mismatch")
    return errors


def _checkpoint_digest(checkpoint: dict[str, Any]) -> str:
    material = {
        key: value for key, value in checkpoint.items() if key != "checkpoint_sha256"
    }
    return _digest_bytes(_canonical(material))


def _validate_or_raise(
    schema_key: str,
    value: dict[str, Any],
    label: str,
) -> None:
    errors = _schema_errors(SCHEMAS[schema_key], value, label)
    if errors:
        raise CheckpointError("; ".join(errors[:8]))


def _validate_source_bundle(
    paths: dict[str, Path],
    values: dict[str, dict[str, Any]],
) -> None:
    for key in ("input", "review", "disposition", "verification", "policy"):
        _validate_or_raise(key, values[key], key)
    source = values["input"]
    review = values["review"]
    disposition = values["disposition"]
    verification = values["verification"]
    policy = values["policy"]

    if not verification["valid"] or not all(verification["checks"].values()):
        raise CheckpointError("source global-progress verification is not fully valid")
    expected_verification = independently_verify_progress_review(
        paths["input"],
        paths["review"],
        paths["disposition"],
    )
    if _canonical(expected_verification) != _canonical(verification):
        raise CheckpointError(
            "source global-progress verification does not match independent recomputation"
        )
    for field, source_key in (
        ("input_sha256", "input"),
        ("review_sha256", "review"),
        ("disposition_sha256", "disposition"),
    ):
        if verification[field] != _digest_file(paths[source_key]):
            raise CheckpointError(f"source verification {field} does not match its file")
    if verification["review_id"] != review["review_id"]:
        raise CheckpointError("source verification review_id mismatch")
    if disposition["review_id"] != review["review_id"]:
        raise CheckpointError("source disposition review_id mismatch")
    if disposition["review_sha256"] != _digest_file(paths["review"]):
        raise CheckpointError("source disposition review digest mismatch")
    if review["source_input_sha256"] != _digest_bytes(_canonical(source)):
        raise CheckpointError("review source_input_sha256 mismatch")
    if (
        review["task_id"] != source["task_id"]
        or review["task_contract_sha256"] != source["task_contract"]["sha256"]
        or review["plan_revision"] != source["plan"]["revision"]
        or review["review_cycle"] != source["plan"]["review_cycle"]
        or review["journal_span"]["last_sequence"] != source["journal"][-1]["sequence"]
        or review["journal_span"]["head_event_id"] != source["journal"][-1]["event_id"]
    ):
        raise CheckpointError("review projection does not match current input head")
    if (
        policy["task_id"] != source["task_id"]
        or policy["task_contract_sha256"] != source["task_contract"]["sha256"]
    ):
        raise CheckpointError("checkpoint policy is not bound to the source task contract")

    directions = [item["direction_id"] for item in policy["critical_directions"]]
    if len(directions) != len(set(directions)):
        raise CheckpointError("checkpoint policy repeats a critical direction")
    acceptance_refs = [
        acceptance_ref
        for item in policy["critical_directions"]
        for acceptance_ref in item["acceptance_refs"]
    ]
    if len(acceptance_refs) != len(set(acceptance_refs)):
        raise CheckpointError("checkpoint policy repeats a critical acceptance")
    plan_directions = {step["direction_id"] for step in source["plan"]["steps"]}
    contract_acceptance = {
        item["acceptance_ref"] for item in source["task_contract"]["acceptance"]
    }
    if set(directions) - plan_directions:
        raise CheckpointError("checkpoint policy references an unknown plan direction")
    if set(acceptance_refs) - contract_acceptance:
        raise CheckpointError("checkpoint policy references an unknown acceptance")


def _validate_focus_bundle(
    paths: dict[str, Path],
    values: dict[str, dict[str, Any]],
    previous: dict[str, Any],
) -> dict[str, Any]:
    for key in (
        "focus_history",
        "focus_review",
        "focus_disposition",
        "focus_verification",
    ):
        schema_key = key.removeprefix("focus_")
        if schema_key == "review":
            schema_key = "holistic_review"
        elif schema_key == "disposition":
            schema_key = "holistic_disposition"
        elif schema_key == "verification":
            schema_key = "holistic_verification"
        _validate_or_raise(schema_key, values[key], key)

    history = values["focus_history"]
    review = values["focus_review"]
    disposition = values["focus_disposition"]
    verification = values["focus_verification"]
    if not verification["valid"] or not all(verification["checks"].values()):
        raise CheckpointError("focus-window verification is not fully valid")
    expected_verification = independently_verify_holistic_review(
        paths["focus_history"],
        paths["focus_review"],
        paths["focus_disposition"],
    )
    if _canonical(expected_verification) != _canonical(verification):
        raise CheckpointError(
            "focus-window verification does not match independent recomputation"
        )
    for field, source_key in (
        ("input_sha256", "focus_history"),
        ("assessment_sha256", "focus_review"),
        ("disposition_sha256", "focus_disposition"),
    ):
        if verification[field] != _digest_file(paths[source_key]):
            raise CheckpointError(f"focus verification {field} does not match its file")
    if (
        disposition["assessment_id"] != review["assessment_id"]
        or disposition["assessment_sha256"] != _digest_file(paths["focus_review"])
        or verification["assessment_id"] != review["assessment_id"]
    ):
        raise CheckpointError("focus assessment/disposition linkage mismatch")
    if history["checkpoints"][-1] != previous:
        raise CheckpointError("focus history head does not exactly match previous checkpoint")
    if review["window"]["last_checkpoint_id"] != previous["checkpoint_id"]:
        raise CheckpointError("focus assessment does not end at previous checkpoint")
    focus = disposition["bounded_focus"]
    if disposition["decision"] != "continue" or focus is None:
        raise CheckpointError("focus disposition does not authorize bounded continuation")
    return focus


def derive_checkpoint(
    paths: dict[str, Path],
    values: dict[str, dict[str, Any]],
    previous: dict[str, Any] | None,
    focus: dict[str, Any] | None,
) -> dict[str, Any]:
    source = values["input"]
    review = values["review"]
    disposition = values["disposition"]
    policy = values["policy"]
    previous_head = previous["journal_head_sequence"] if previous else -1
    if previous:
        errors = _checkpoint_errors(previous, "previous_checkpoint")
        if errors:
            raise CheckpointError("; ".join(errors[:8]))
        if (
            previous["task_id"] != source["task_id"]
            or previous["task_contract_sha256"] != source["task_contract"]["sha256"]
        ):
            raise CheckpointError("previous checkpoint belongs to another task contract")
        if previous["checkpoint_policy_sha256"] != _digest_file(paths["policy"]):
            raise CheckpointError("checkpoint policy changed without an explicit revision")
        if source["plan"]["review_cycle"] <= previous["review_cycle"]:
            raise CheckpointError("review cycle did not advance beyond previous checkpoint")
        if source["journal"][-1]["sequence"] <= previous_head:
            raise CheckpointError("journal head did not advance beyond previous checkpoint")

    new_events = [
        event for event in source["journal"] if event["sequence"] > previous_head
    ]
    steps = source["plan"]["steps"]
    step_map = {step["step_id"]: step for step in steps}
    plan_directions = sorted({step["direction_id"] for step in steps})
    counts: dict[str, int] = defaultdict(int)
    for event in new_events:
        if (
            event["event_type"] == "action_terminal"
            and event["direction_id"] is not None
        ):
            counts[event["direction_id"]] += 1
    direction_action_counts = [
        {"direction_id": direction, "action_count": counts[direction]}
        for direction in plan_directions
    ]

    critical_directions = sorted(
        item["direction_id"] for item in policy["critical_directions"]
    )
    active_directions = sorted(
        {
            step["direction_id"]
            for step in steps
            if step["state"] in {"pending", "in_progress"}
        }
    )
    deferred_directions = []
    for direction in critical_directions:
        matching = [step for step in steps if step["direction_id"] == direction]
        if matching and all(step["state"] == "deferred" for step in matching):
            deferred_directions.append(direction)
    active_directions = sorted(set(active_directions) - set(deferred_directions))

    coverage = {
        item["acceptance_ref"]: item["state"]
        for item in review["acceptance_coverage"]
    }
    critical_acceptance_refs = sorted(
        {
            acceptance_ref
            for item in policy["critical_directions"]
            for acceptance_ref in item["acceptance_refs"]
        }
    )
    critical_acceptance_states = [
        {"acceptance_ref": acceptance_ref, "state": coverage[acceptance_ref]}
        for acceptance_ref in critical_acceptance_refs
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
        for item in critical_acceptance_states
        if item["state"] == "verified"
        and prior_states.get(item["acceptance_ref"]) != "verified"
    )
    novel_evidence = sorted(
        event["event_id"]
        for event in new_events
        if event["event_type"] == "artifact_registered"
    )
    selected_step = disposition["selected_step_id"]
    selected_direction = (
        step_map[selected_step]["direction_id"] if selected_step is not None else None
    )

    focus_consumption = None
    if focus is not None:
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
    checkpoint = {
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
        "direction_action_counts": direction_action_counts,
        "active_direction_ids": active_directions,
        "critical_direction_ids": critical_directions,
        "deferred_direction_ids": deferred_directions,
        "novel_evidence_refs": novel_evidence,
        "verified_acceptance_refs_added": verified_added,
        "critical_acceptance_states": critical_acceptance_states,
        "unresolved_constraint_refs": sorted(
            item["constraint_id"] for item in review["unresolved"]
        ),
        "focus_window_consumption": focus_consumption,
        "previous_checkpoint_sha256": (
            previous["checkpoint_sha256"] if previous else None
        ),
        "checkpoint_sha256": "",
    }
    checkpoint["checkpoint_sha256"] = _checkpoint_digest(checkpoint)
    errors = _checkpoint_errors(checkpoint, "generated_checkpoint")
    if errors:
        raise CheckpointError("; ".join(errors[:8]))
    return checkpoint


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise CheckpointError(f"refusing to overwrite output: {path}") from exc
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
    parser.add_argument("--verification", required=True)
    parser.add_argument("--policy", required=True)
    parser.add_argument("--previous-checkpoint")
    parser.add_argument("--focus-history")
    parser.add_argument("--focus-review")
    parser.add_argument("--focus-disposition")
    parser.add_argument("--focus-verification")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    paths = {
        key: Path(getattr(args, key))
        for key in (
            "input",
            "review",
            "disposition",
            "verification",
            "policy",
        )
    }
    focus_names = (
        "focus_history",
        "focus_review",
        "focus_disposition",
        "focus_verification",
    )
    focus_values = [getattr(args, name) for name in focus_names]
    try:
        if any(focus_values) and not all(focus_values):
            raise CheckpointError("focus artifact arguments must be supplied together")
        values = {key: _read(path) for key, path in paths.items()}
        _validate_source_bundle(paths, values)
        previous = (
            _read(Path(args.previous_checkpoint))
            if args.previous_checkpoint
            else None
        )
        focus = None
        if all(focus_values):
            if previous is None:
                raise CheckpointError("focus enforcement requires --previous-checkpoint")
            for name in focus_names:
                paths[name] = Path(getattr(args, name))
                values[name] = _read(paths[name])
            focus = _validate_focus_bundle(paths, values, previous)
        checkpoint = derive_checkpoint(paths, values, previous, focus)
        _write_new(Path(args.output), checkpoint)
    except CheckpointError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    within_focus = (
        checkpoint["focus_window_consumption"] is None
        or (
            checkpoint["focus_window_consumption"]["within_action_limit"]
            and checkpoint["focus_window_consumption"]["within_review_deadline"]
        )
    )
    print(
        json.dumps(
            {
                "valid": within_focus,
                "checkpoint_id": checkpoint["checkpoint_id"],
                "focus_window_within_bounds": within_focus,
            }
        )
    )
    return 0 if within_focus else 2


if __name__ == "__main__":
    raise SystemExit(main())
