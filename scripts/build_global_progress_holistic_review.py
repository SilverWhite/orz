#!/usr/bin/env python3
"""Build a deterministic cross-checkpoint Global Progress Sentinel review."""

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


ROOT = Path(__file__).resolve().parents[1]
INPUT_SCHEMA = ROOT / "runtime" / "global-progress-history-input-v0.1.schema.json"
REVIEW_SCHEMA = ROOT / "runtime" / "global-progress-holistic-review-v0.1.schema.json"


class HolisticReviewError(RuntimeError):
    pass


def _canonical(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _load(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise HolisticReviewError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise HolisticReviewError(f"expected JSON object: {path}")
    return value


def _schema_errors(schema_path: Path, value: dict[str, Any]) -> list[str]:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    failures = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda error: list(error.absolute_path),
    )
    return [
        f"{'/'.join(map(str, failure.absolute_path)) or '<root>'}: {failure.message}"
        for failure in failures
    ]


def _checkpoint_material(checkpoint: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in checkpoint.items() if key != "checkpoint_sha256"}


def _semantic_errors(source: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    checkpoints = source["checkpoints"]
    checkpoint_ids = [item["checkpoint_id"] for item in checkpoints]
    if len(checkpoint_ids) != len(set(checkpoint_ids)):
        errors.append("duplicate checkpoint_id")
    cycles = [item["review_cycle"] for item in checkpoints]
    if cycles != sorted(cycles) or len(cycles) != len(set(cycles)):
        errors.append("review cycles must be strictly increasing")
    heads = [item["journal_head_sequence"] for item in checkpoints]
    if heads != sorted(heads) or len(heads) != len(set(heads)):
        errors.append("journal heads must be strictly increasing")

    previous: str | None = None
    for checkpoint in checkpoints:
        checkpoint_id = checkpoint["checkpoint_id"]
        expected = _digest(_canonical(_checkpoint_material(checkpoint)))
        if checkpoint["checkpoint_sha256"] != expected:
            errors.append(f"{checkpoint_id} checkpoint_sha256 mismatch")
        if checkpoint["previous_checkpoint_sha256"] != previous:
            errors.append(f"{checkpoint_id} previous checkpoint linkage mismatch")
        previous = checkpoint["checkpoint_sha256"]

        counts = checkpoint["direction_action_counts"]
        count_directions = [item["direction_id"] for item in counts]
        if len(count_directions) != len(set(count_directions)):
            errors.append(f"{checkpoint_id} has duplicate direction action counts")
        known = set(count_directions)
        active = set(checkpoint["active_direction_ids"])
        critical = set(checkpoint["critical_direction_ids"])
        deferred = set(checkpoint["deferred_direction_ids"])
        if not active.issubset(known) or not critical.issubset(known) or not deferred.issubset(known):
            errors.append(f"{checkpoint_id} direction sets must be represented in action counts")
        if not deferred.issubset(critical):
            errors.append(f"{checkpoint_id} deferred directions must be critical")
        if active & deferred:
            errors.append(f"{checkpoint_id} directions cannot be both active and deferred")
        selected = checkpoint["selected_direction_id"]
        if selected is not None and selected not in known:
            errors.append(f"{checkpoint_id} selected direction is unknown")
        acceptance_refs = [
            item["acceptance_ref"] for item in checkpoint["critical_acceptance_states"]
        ]
        if len(acceptance_refs) != len(set(acceptance_refs)):
            errors.append(f"{checkpoint_id} has duplicate critical acceptance state")
    return errors


def _warning(
    source: dict[str, Any],
    kind: str,
    message: str,
    subjects: list[str],
    sources: list[str],
) -> dict[str, Any]:
    material = {
        "task_contract_sha256": source["task_contract_sha256"],
        "history_head": source["checkpoints"][-1]["checkpoint_sha256"],
        "kind": kind,
        "subject_refs": sorted(subjects),
    }
    warning_digest = _digest(_canonical(material))
    return {
        "warning_id": f"GHW-{warning_digest[:16]}",
        "kind": kind,
        "severity": "warn",
        "message": message,
        "subject_refs": sorted(subjects),
        "source_refs": sorted(sources),
        "review_required": True,
    }


def build_review(source: dict[str, Any]) -> dict[str, Any]:
    schema_failures = _schema_errors(INPUT_SCHEMA, source)
    if schema_failures:
        raise HolisticReviewError(
            f"schema validation failed for {INPUT_SCHEMA.name}: {'; '.join(schema_failures[:8])}"
        )
    semantic_failures = _semantic_errors(source)
    if semantic_failures:
        raise HolisticReviewError("semantic validation failed: " + "; ".join(semantic_failures))

    thresholds = source["thresholds"]
    checkpoints = source["checkpoints"]
    window = checkpoints[-thresholds["history_window_checkpoints"] :]
    latest = checkpoints[-1]

    action_totals: dict[str, int] = defaultdict(int)
    action_sources: dict[str, list[str]] = defaultdict(list)
    all_directions: set[str] = set()
    for checkpoint in window:
        for count in checkpoint["direction_action_counts"]:
            direction = count["direction_id"]
            all_directions.add(direction)
            action_totals[direction] += count["action_count"]
            if count["action_count"] > 0:
                action_sources[direction].append(checkpoint["checkpoint_id"])
    total_actions = sum(action_totals.values())
    latest_active = set(latest["active_direction_ids"])
    latest_critical = set(latest["critical_direction_ids"])
    direction_budget = [
        {
            "direction_id": direction,
            "action_count": action_totals[direction],
            "share": round(action_totals[direction] / total_actions, 6)
            if total_actions
            else 0.0,
            "active": direction in latest_active,
            "critical": direction in latest_critical,
            "source_refs": sorted(action_sources[direction])
            or [latest["checkpoint_id"]],
        }
        for direction in sorted(all_directions)
    ]

    selected_direction = latest["selected_direction_id"]
    no_novelty_refs: list[str] = []
    if selected_direction is not None:
        for checkpoint in reversed(window):
            if (
                checkpoint["decision"] == "continue"
                and checkpoint["selected_direction_id"] == selected_direction
                and not checkpoint["novel_evidence_refs"]
                and not checkpoint["verified_acceptance_refs_added"]
            ):
                no_novelty_refs.append(checkpoint["checkpoint_id"])
            else:
                break
    no_novelty_refs.reverse()
    evidence_novelty = {
        "same_direction_continue_without_novelty": len(no_novelty_refs),
        "direction_id": selected_direction if no_novelty_refs else None,
        "checkpoint_refs": no_novelty_refs,
    }

    deferral_debt: list[dict[str, Any]] = []
    for direction in sorted(latest_critical):
        refs: list[str] = []
        for checkpoint in reversed(window):
            if direction in checkpoint["deferred_direction_ids"]:
                refs.append(checkpoint["checkpoint_id"])
            else:
                break
        refs.reverse()
        if refs:
            deferral_debt.append(
                {
                    "direction_id": direction,
                    "consecutive_checkpoints": len(refs),
                    "source_refs": refs,
                }
            )

    blockers = sorted(
        set(
            [
                item["acceptance_ref"]
                for item in latest["critical_acceptance_states"]
                if item["state"] != "verified"
            ]
            + latest["unresolved_constraint_refs"]
        )
    )
    completion_requested = source["completion_requested"]
    completion_gate = {
        "requested": completion_requested,
        "status": (
            "ineligible"
            if completion_requested and blockers
            else "eligible"
            if completion_requested
            else "not_requested"
        ),
        "blocker_refs": blockers if completion_requested else [],
    }

    warnings: list[dict[str, Any]] = []
    dominant = (
        min(direction_budget, key=lambda item: (-item["action_count"], item["direction_id"]))
        if direction_budget
        else None
    )
    other_active_critical = (
        sorted((latest_active & latest_critical) - {dominant["direction_id"]})
        if dominant
        else []
    )
    if (
        dominant
        and total_actions >= thresholds["direction_dominance_min_actions"]
        and dominant["share"] >= thresholds["direction_dominance_ratio"]
        and other_active_critical
    ):
        warnings.append(
            _warning(
                source,
                "direction_budget_dominance",
                (
                    f"{dominant['direction_id']} consumed {dominant['share']:.1%} of "
                    f"{total_actions} observed actions while other active critical directions remain."
                ),
                [dominant["direction_id"], *other_active_critical],
                dominant["source_refs"],
            )
        )
    if len(no_novelty_refs) >= thresholds["stagnation_checkpoints"]:
        warnings.append(
            _warning(
                source,
                "evidence_stagnation",
                (
                    f"{selected_direction} continued for {len(no_novelty_refs)} checkpoints "
                    "without a novel evidence reference or newly verified acceptance."
                ),
                [selected_direction],
                no_novelty_refs,
            )
        )
    for debt in deferral_debt:
        if debt["consecutive_checkpoints"] >= thresholds["deferral_debt_checkpoints"]:
            warnings.append(
                _warning(
                    source,
                    "critical_direction_deferral_debt",
                    (
                        f"Critical direction {debt['direction_id']} was explicitly deferred "
                        f"for {debt['consecutive_checkpoints']} consecutive checkpoints."
                    ),
                    [debt["direction_id"]],
                    debt["source_refs"],
                )
            )
    if completion_gate["status"] == "ineligible":
        warnings.append(
            _warning(
                source,
                "local_pass_global_incomplete",
                "Completion was requested while critical acceptance or constraint blockers remain.",
                completion_gate["blocker_refs"],
                [latest["checkpoint_id"]],
            )
        )
    warnings.sort(key=lambda item: (item["kind"], item["warning_id"]))

    source_digest = _digest(_canonical(source))
    identity = {
        "source_history_sha256": source_digest,
        "task_id": source["task_id"],
        "history_head": latest["checkpoint_sha256"],
    }
    review = {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-holistic-review",
        "assessment_id": f"GPH-{_digest(_canonical(identity))[:16]}",
        "source_history_sha256": source_digest,
        "task_id": source["task_id"],
        "task_contract_sha256": source["task_contract_sha256"],
        "window": {
            "first_checkpoint_id": window[0]["checkpoint_id"],
            "last_checkpoint_id": window[-1]["checkpoint_id"],
            "first_review_cycle": window[0]["review_cycle"],
            "last_review_cycle": window[-1]["review_cycle"],
            "checkpoint_count": len(window),
        },
        "direction_budget": direction_budget,
        "evidence_novelty": evidence_novelty,
        "critical_deferral_debt": deferral_debt,
        "completion_gate": completion_gate,
        "warnings": warnings,
        "next_review_trigger": "bounded_focus_exit_or_next_checkpoint_or_before_completion",
        "generator": {
            "name": "build_global_progress_holistic_review.py",
            "version": "0.1.0",
            "summary_state": "deterministic_cross_checkpoint_projection",
        },
    }
    review_failures = _schema_errors(REVIEW_SCHEMA, review)
    if review_failures:
        raise HolisticReviewError(
            f"generated review failed schema: {'; '.join(review_failures[:8])}"
        )
    return review


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise HolisticReviewError(f"refusing to overwrite output: {path}") from exc
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
        source = _load(Path(args.input))
        review = build_review(source)
        _write_new(Path(args.output), review)
    except HolisticReviewError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(
        json.dumps(
            {
                "valid": True,
                "assessment_id": review["assessment_id"],
                "warnings": len(review["warnings"]),
                "completion_gate": review["completion_gate"]["status"],
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
