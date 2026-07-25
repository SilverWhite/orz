#!/usr/bin/env python3
"""Independently rebuild and verify a holistic GPS review and disposition."""

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
DISPOSITION_SCHEMA = ROOT / "runtime" / "global-progress-holistic-disposition-v0.1.schema.json"
VERIFICATION_SCHEMA = ROOT / "runtime" / "global-progress-holistic-verification-v0.1.schema.json"


class HolisticVerificationError(RuntimeError):
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
        raise HolisticVerificationError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise HolisticVerificationError(f"expected JSON object: {path}")
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


def _checkpoint_digest(checkpoint: dict[str, Any]) -> str:
    material = {
        key: value for key, value in checkpoint.items() if key != "checkpoint_sha256"
    }
    return _digest_bytes(_canonical(material))


def _semantic_errors(source: dict[str, Any]) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    chain_errors: list[str] = []
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

    prior: str | None = None
    for checkpoint in checkpoints:
        checkpoint_id = checkpoint["checkpoint_id"]
        if checkpoint["checkpoint_sha256"] != _checkpoint_digest(checkpoint):
            chain_errors.append(f"{checkpoint_id} checkpoint digest mismatch")
        if checkpoint["previous_checkpoint_sha256"] != prior:
            chain_errors.append(f"{checkpoint_id} previous checkpoint linkage mismatch")
        prior = checkpoint["checkpoint_sha256"]

        count_directions = [
            item["direction_id"] for item in checkpoint["direction_action_counts"]
        ]
        if len(count_directions) != len(set(count_directions)):
            errors.append(f"{checkpoint_id} has duplicate direction action counts")
        known = set(count_directions)
        active = set(checkpoint["active_direction_ids"])
        critical = set(checkpoint["critical_direction_ids"])
        deferred = set(checkpoint["deferred_direction_ids"])
        if (active | critical | deferred) - known:
            errors.append(f"{checkpoint_id} contains an uncounted direction")
        if deferred - critical:
            errors.append(f"{checkpoint_id} defers a non-critical direction")
        if active & deferred:
            errors.append(f"{checkpoint_id} has active/deferred overlap")
        selected = checkpoint["selected_direction_id"]
        if selected is not None and selected not in known:
            errors.append(f"{checkpoint_id} selected direction is unknown")
        acceptance_refs = [
            item["acceptance_ref"] for item in checkpoint["critical_acceptance_states"]
        ]
        if len(acceptance_refs) != len(set(acceptance_refs)):
            errors.append(f"{checkpoint_id} repeats a critical acceptance")
    return errors, chain_errors


def _make_warning(
    source: dict[str, Any],
    kind: str,
    message: str,
    subjects: list[str],
    sources: list[str],
) -> dict[str, Any]:
    fingerprint = {
        "task_contract_sha256": source["task_contract_sha256"],
        "history_head": source["checkpoints"][-1]["checkpoint_sha256"],
        "kind": kind,
        "subject_refs": sorted(subjects),
    }
    token = _digest_bytes(_canonical(fingerprint))
    return {
        "warning_id": f"GHW-{token[:16]}",
        "kind": kind,
        "severity": "warn",
        "message": message,
        "subject_refs": sorted(subjects),
        "source_refs": sorted(sources),
        "review_required": True,
    }


def _rebuild(source: dict[str, Any]) -> dict[str, Any]:
    thresholds = source["thresholds"]
    history = source["checkpoints"]
    window = history[-thresholds["history_window_checkpoints"] :]
    latest = history[-1]

    totals: dict[str, int] = defaultdict(int)
    sources: dict[str, list[str]] = defaultdict(list)
    directions: set[str] = set()
    for checkpoint in window:
        for observation in checkpoint["direction_action_counts"]:
            direction = observation["direction_id"]
            directions.add(direction)
            totals[direction] += observation["action_count"]
            if observation["action_count"]:
                sources[direction].append(checkpoint["checkpoint_id"])
    total_actions = sum(totals.values())
    active = set(latest["active_direction_ids"])
    critical = set(latest["critical_direction_ids"])
    budget = []
    for direction in sorted(directions):
        budget.append(
            {
                "direction_id": direction,
                "action_count": totals[direction],
                "share": round(totals[direction] / total_actions, 6)
                if total_actions
                else 0.0,
                "active": direction in active,
                "critical": direction in critical,
                "source_refs": sorted(sources[direction])
                or [latest["checkpoint_id"]],
            }
        )

    selected = latest["selected_direction_id"]
    stagnant: list[str] = []
    if selected is not None:
        for checkpoint in window[::-1]:
            no_novelty = (
                not checkpoint["novel_evidence_refs"]
                and not checkpoint["verified_acceptance_refs_added"]
            )
            same_continue = (
                checkpoint["decision"] == "continue"
                and checkpoint["selected_direction_id"] == selected
            )
            if not (no_novelty and same_continue):
                break
            stagnant.append(checkpoint["checkpoint_id"])
    stagnant.reverse()
    novelty = {
        "same_direction_continue_without_novelty": len(stagnant),
        "direction_id": selected if stagnant else None,
        "checkpoint_refs": stagnant,
    }

    debts = []
    for direction in sorted(critical):
        refs: list[str] = []
        for checkpoint in window[::-1]:
            if direction not in checkpoint["deferred_direction_ids"]:
                break
            refs.append(checkpoint["checkpoint_id"])
        refs.reverse()
        if refs:
            debts.append(
                {
                    "direction_id": direction,
                    "consecutive_checkpoints": len(refs),
                    "source_refs": refs,
                }
            )

    blockers = sorted(
        set(
            [
                state["acceptance_ref"]
                for state in latest["critical_acceptance_states"]
                if state["state"] != "verified"
            ]
            + latest["unresolved_constraint_refs"]
        )
    )
    requested = source["completion_requested"]
    if not requested:
        gate_status = "not_requested"
        gate_blockers: list[str] = []
    elif blockers:
        gate_status = "ineligible"
        gate_blockers = blockers
    else:
        gate_status = "eligible"
        gate_blockers = []
    gate = {
        "requested": requested,
        "status": gate_status,
        "blocker_refs": gate_blockers,
    }

    warnings = []
    dominant = min(budget, key=lambda item: (-item["action_count"], item["direction_id"]))
    alternatives = sorted((active & critical) - {dominant["direction_id"]})
    if (
        total_actions >= thresholds["direction_dominance_min_actions"]
        and dominant["share"] >= thresholds["direction_dominance_ratio"]
        and alternatives
    ):
        warnings.append(
            _make_warning(
                source,
                "direction_budget_dominance",
                (
                    f"{dominant['direction_id']} consumed {dominant['share']:.1%} of "
                    f"{total_actions} observed actions while other active critical directions remain."
                ),
                [dominant["direction_id"], *alternatives],
                dominant["source_refs"],
            )
        )
    if len(stagnant) >= thresholds["stagnation_checkpoints"]:
        warnings.append(
            _make_warning(
                source,
                "evidence_stagnation",
                (
                    f"{selected} continued for {len(stagnant)} checkpoints "
                    "without a novel evidence reference or newly verified acceptance."
                ),
                [selected],
                stagnant,
            )
        )
    for debt in debts:
        if debt["consecutive_checkpoints"] >= thresholds["deferral_debt_checkpoints"]:
            warnings.append(
                _make_warning(
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
    if gate["status"] == "ineligible":
        warnings.append(
            _make_warning(
                source,
                "local_pass_global_incomplete",
                "Completion was requested while critical acceptance or constraint blockers remain.",
                gate["blocker_refs"],
                [latest["checkpoint_id"]],
            )
        )
    warnings.sort(key=lambda item: (item["kind"], item["warning_id"]))

    source_digest = _digest_bytes(_canonical(source))
    identity = {
        "source_history_sha256": source_digest,
        "task_id": source["task_id"],
        "history_head": latest["checkpoint_sha256"],
    }
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-holistic-review",
        "assessment_id": f"GPH-{_digest_bytes(_canonical(identity))[:16]}",
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
        "direction_budget": budget,
        "evidence_novelty": novelty,
        "critical_deferral_debt": debts,
        "completion_gate": gate,
        "warnings": warnings,
        "next_review_trigger": "bounded_focus_exit_or_next_checkpoint_or_before_completion",
        "generator": {
            "name": "build_global_progress_holistic_review.py",
            "version": "0.1.0",
            "summary_state": "deterministic_cross_checkpoint_projection",
        },
    }


def _disposition_errors(
    source: dict[str, Any],
    review: dict[str, Any],
    disposition: dict[str, Any],
) -> tuple[list[str], bool, bool, bool]:
    errors: list[str] = []
    expected_ids = {item["warning_id"] for item in review["warnings"]}
    dispositions = disposition["warning_dispositions"]
    actual_ids = [item["warning_id"] for item in dispositions]
    coverage_valid = len(actual_ids) == len(set(actual_ids)) and set(actual_ids) == expected_ids
    if not coverage_valid:
        errors.append("warning dispositions do not exactly cover active warnings")

    latest = source["checkpoints"][-1]
    active = set(latest["active_direction_ids"])
    critical = set(latest["critical_direction_ids"])
    selected = disposition["selected_direction_id"]
    decision_valid = True
    if disposition["decision"] in {"continue", "pivot"}:
        if selected not in active:
            errors.append("continue/pivot requires a currently active selected direction")
            decision_valid = False
    elif selected is not None and selected not in active | critical:
        errors.append("selected direction is unknown at the latest checkpoint")
        decision_valid = False

    warnings = {item["warning_id"]: item for item in review["warnings"]}
    reasoned = [
        warnings[item["warning_id"]]
        for item in dispositions
        if item["status"] == "reasoned_continue" and item["warning_id"] in warnings
    ]
    focus = disposition["bounded_focus"]
    focus_kinds = {"direction_budget_dominance", "evidence_stagnation"}
    if reasoned:
        if disposition["decision"] != "continue":
            errors.append("reasoned_continue requires decision=continue")
            decision_valid = False
        unsupported = [item["kind"] for item in reasoned if item["kind"] not in focus_kinds]
        if unsupported:
            errors.append("reasoned_continue is not valid for completion or deferral-debt warnings")
            decision_valid = False
        if focus is None:
            errors.append("reasoned_continue requires a bounded_focus control")
            decision_valid = False
        else:
            dominant = min(
                review["direction_budget"],
                key=lambda item: (-item["action_count"], item["direction_id"]),
            )["direction_id"]
            if focus["direction_id"] != selected or selected != dominant:
                errors.append("bounded focus must name the selected dominant direction")
                decision_valid = False
            current_cycle = latest["review_cycle"]
            if not current_cycle < focus["review_by_cycle"] <= current_cycle + 1:
                errors.append("bounded focus must expire by the next review cycle")
                decision_valid = False
            deferred_required = (active & critical) - {selected}
            if not deferred_required.issubset(set(focus["deferred_direction_ids"])):
                errors.append("bounded focus must name every displaced active critical direction")
                decision_valid = False
            critical_acceptance_refs = {
                item["acceptance_ref"] for item in latest["critical_acceptance_states"]
            }
            if focus["critical_path_ref"] not in critical_acceptance_refs:
                errors.append("bounded focus critical_path_ref must name a critical acceptance")
                decision_valid = False
            if (
                not focus["acceptance_refs"]
                or not set(focus["acceptance_refs"]).issubset(critical_acceptance_refs)
            ):
                errors.append("bounded focus must serve a current critical acceptance")
                decision_valid = False
    elif focus is not None:
        errors.append("bounded_focus is only valid with a reasoned_continue disposition")
        decision_valid = False

    gate_status = review["completion_gate"]["status"]
    expected_ack = {
        "not_requested": "not_requested",
        "eligible": "eligible",
        "ineligible": "completion_withheld",
    }[gate_status]
    completion_valid = disposition["completion_acknowledgement"] == expected_ack
    if not completion_valid:
        errors.append(
            f"completion acknowledgement must be {expected_ack} for gate status {gate_status}"
        )
    return errors, coverage_valid, decision_valid, completion_valid


def verify(
    input_path: Path,
    review_path: Path,
    disposition_path: Path,
) -> dict[str, Any]:
    source = _read(input_path)
    review = _read(review_path)
    disposition = _read(disposition_path)
    errors: list[str] = []
    checks = {
        "input_valid": False,
        "checkpoint_chain_valid": False,
        "assessment_schema_valid": False,
        "assessment_rebuilt_exactly": False,
        "disposition_schema_valid": False,
        "assessment_linkage_valid": False,
        "warning_dispositions_complete": False,
        "decision_semantics_valid": False,
        "completion_gate_acknowledged": False,
    }

    input_failures = _schema_errors(INPUT_SCHEMA, source, "input")
    errors.extend(input_failures)
    semantic_failures: list[str] = []
    chain_failures: list[str] = []
    if not input_failures:
        semantic_failures, chain_failures = _semantic_errors(source)
        errors.extend(semantic_failures)
        errors.extend(chain_failures)
    checks["checkpoint_chain_valid"] = not input_failures and not chain_failures
    checks["input_valid"] = not input_failures and not semantic_failures and not chain_failures

    review_failures = _schema_errors(REVIEW_SCHEMA, review, "assessment")
    errors.extend(review_failures)
    checks["assessment_schema_valid"] = not review_failures
    if checks["input_valid"]:
        expected = _rebuild(source)
        checks["assessment_rebuilt_exactly"] = _canonical(expected) == _canonical(review)
        if not checks["assessment_rebuilt_exactly"]:
            errors.append("assessment does not match independent reconstruction")

    disposition_failures = _schema_errors(
        DISPOSITION_SCHEMA, disposition, "disposition"
    )
    errors.extend(disposition_failures)
    checks["disposition_schema_valid"] = not disposition_failures
    assessment_digest = _digest_file(review_path)
    checks["assessment_linkage_valid"] = (
        disposition.get("assessment_id") == review.get("assessment_id")
        and disposition.get("assessment_sha256") == assessment_digest
    )
    if not checks["assessment_linkage_valid"]:
        errors.append("disposition assessment linkage is invalid")

    if (
        checks["input_valid"]
        and checks["assessment_schema_valid"]
        and checks["disposition_schema_valid"]
    ):
        disposition_errors, coverage, decision, completion = _disposition_errors(
            source, review, disposition
        )
        errors.extend(disposition_errors)
        checks["warning_dispositions_complete"] = coverage
        checks["decision_semantics_valid"] = decision
        checks["completion_gate_acknowledged"] = completion

    report = {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-holistic-verification",
        "valid": all(checks.values()) and not errors,
        "assessment_id": review.get("assessment_id")
        if isinstance(review.get("assessment_id"), str)
        else None,
        "input_sha256": _digest_file(input_path),
        "assessment_sha256": assessment_digest,
        "disposition_sha256": _digest_file(disposition_path),
        "warning_count": len(review.get("warnings", []))
        if isinstance(review.get("warnings"), list)
        else 0,
        "checks": checks,
        "errors": errors,
    }
    generated_failures = _schema_errors(
        VERIFICATION_SCHEMA, report, "verification"
    )
    if generated_failures:
        raise HolisticVerificationError(
            "generated verification report failed schema: "
            + "; ".join(generated_failures)
        )
    return report


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise HolisticVerificationError(f"refusing to overwrite output: {path}") from exc
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
            Path(args.input),
            Path(args.review),
            Path(args.disposition),
        )
        _write_new(Path(args.output), report)
    except HolisticVerificationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps({"valid": report["valid"], "checks": report["checks"]}))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
