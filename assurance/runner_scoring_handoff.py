from __future__ import annotations

import math
import uuid
from pathlib import Path
from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, load_json, sha256_bytes, utc_now


SCORING_INPUT_SCHEMA = "runner-scoring-input-v0.1.schema.json"


# ── Scoring Input Builder ──


def build_scoring_input(
    *,
    run_id: str,
    conversation_id: str,
    task_id: str | None = None,
    answer_packet: dict[str, Any],
    ipg_receipt: dict[str, Any] | None = None,
    tool_availability_gate_receipt: dict[str, Any] | None = None,
    source_gate_receipt: dict[str, Any] | None = None,
    journal_events: Sequence[dict[str, Any]] | None = None,
    enforcement_receipt: dict[str, Any] | None = None,
    output_validation: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build a structured scoring input from a completed runner's outputs.

    This is the bridge: runner output → evaluation scoring system.
    It extracts gate results, journal summary, integrity checks, and
    scoring hints from the runner's artifacts.
    """
    # ── Gate results ──
    gate_results: dict[str, Any] = {
        "ipg_decision": ipg_receipt.get("gate_decision", "block") if ipg_receipt else "block",
        "tool_availability_decision": (
            tool_availability_gate_receipt.get("decisions", {}).get("gate_decision", "block")
            if tool_availability_gate_receipt else "block"
        ),
        "source_visibility_decision": (
            source_gate_receipt.get("decision", "block") if source_gate_receipt else "block"
        ),
    }
    if ipg_receipt:
        gate_results["ipg_source_summary"] = ipg_receipt.get("source_summary", {})
    if source_gate_receipt:
        gate_results["source_visibility_summary"] = source_gate_receipt.get(
            "reference_decisions", []
        )

    # ── Journal summary ──
    events = list(journal_events) if journal_events is not None else []
    gate_indices = {
        i for i, e in enumerate(events)
        if e.get("event_type") in {"gate_decision", "instruction_provenance_gate"}
    }
    model_indices = {
        i for i, e in enumerate(events)
        if e.get("event_type") in {"model_request", "model_output"}
    }
    gate_before_model = (
        not gate_indices
        or not model_indices
        or min(gate_indices) < min(model_indices)
    )
    journal_summary = {
        "event_count": len(events),
        "terminal_event": events[-1].get("event_type", "unknown") if events else "none",
        "gate_before_model": gate_before_model,
        "event_types": [e.get("event_type", "unknown") for e in events],
    }

    # ── Integrity checks ──
    integrity_checks = {
        "gate_chain_complete": (
            ipg_receipt is not None
            and tool_availability_gate_receipt is not None
            and source_gate_receipt is not None
        ),
        "all_gates_evaluated_before_model": gate_before_model,
        "adapter_call_gated": (
            enforcement_receipt is not None
            and enforcement_receipt.get("adapter_call_allowed", False)
        ),
        "output_schema_valid": (
            output_validation is not None and output_validation.get("valid", False)
        ),
        "no_credential_leak": (
            output_validation is not None
            and output_validation.get("checks", {}).get("no_credential_leak", False)
        ),
        "source_visibility_annotated": (
            output_validation is not None
            and output_validation.get("checks", {}).get("visibility_annotated", False)
        ),
    }

    # ── Scoring hints ──
    claim_boundaries = answer_packet.get("claim_boundaries", {})
    source_vis = answer_packet.get("answer", {}).get("source_visibility_summary", [])
    scoring_hints = {
        "proposed_gates": claim_boundaries.get("gate_chain_order", []),
        "claim_type": "observation" if "observation" in str(answer_packet.get("answer", {}).get("summary", [])).lower() else "unknown",
        "claim_scope": "single_source" if len(source_vis) <= 1 else "multi_source",
        "claim_strength": claim_boundaries.get("scientific_claim_strength", "none"),
        "supporting_source_ids": [
            item.get("ref_id", "") for item in source_vis
            if isinstance(item, dict) and item.get("claim_allowed") not in (None, False, "")
        ],
        "deferred_claims": answer_packet.get("answer", {}).get("deferred_claims", []),
    }

    scoring_input = {
        "schema_version": "0.1.0-draft",
        "input_kind": "runner_scoring_input",
        "run_id": run_id,
        "conversation_id": conversation_id,
        "task_id": task_id or "",
        "created_at": utc_now(),
        "answer_packet": {
            "run_id": answer_packet.get("run_id", run_id),
            "task_id": answer_packet.get("task_id", task_id),
            "answer": answer_packet.get("answer", {}),
            "claim_boundaries": claim_boundaries,
        },
        "gate_results": gate_results,
        "journal_summary": journal_summary,
        "integrity_checks": integrity_checks,
        "scoring_hints": scoring_hints,
        "limitations": [
            "Scoring input is a mechanical projection of runner outputs.",
            "It does not apply scoring rubrics or compute metrics.",
            "Oracle comparison requires a separate evaluation run.",
        ],
    }
    validate_contract(scoring_input, SCORING_INPUT_SCHEMA, label="runner scoring input")
    return scoring_input


# ── Deterministic Scoring ──


def _safe_ratio(numerator: int, denominator: int) -> float:
    if denominator == 0:
        return 1.0 if numerator == 0 else 0.0
    return numerator / denominator


def compute_gate_scores(
    proposed_gate_ids: set[str],
    oracle_gate_ids: set[str],
) -> dict[str, Any]:
    """Compute gate identification precision, recall, and F1.

    Args:
        proposed_gate_ids: Gates the agent identified.
        oracle_gate_ids: Gates the oracle says should be identified.

    Returns a dict with precision, recall, f1, false_positives, false_negatives.
    """
    tp = len(proposed_gate_ids & oracle_gate_ids)
    fp = len(proposed_gate_ids - oracle_gate_ids)
    fn = len(oracle_gate_ids - proposed_gate_ids)

    precision = _safe_ratio(tp, tp + fp)
    recall = _safe_ratio(tp, tp + fn)
    f1 = _safe_ratio(2 * precision * recall, precision + recall) if (precision + recall) > 0 else 0.0

    return {
        "true_positives": tp,
        "false_positives": fp,
        "false_negatives": fn,
        "precision": round(precision, 4),
        "recall": round(recall, 4),
        "f1": round(f1, 4),
    }


def compute_claim_boundary_score(
    agent_claim_strength: str,
    source_visibility_statuses: list[str],
    *,
    oracle_claim_strength: str | None = None,
) -> dict[str, Any]:
    """Score claim boundary alignment.

    Checks whether the agent's claim strength is appropriate given
    the source visibility status of referenced sources.

    Returns a dict with score, max_score, and diagnostics.
    """
    strength_levels = {
        "none": 0,
        "metadata_only": 1,
        "observed_fragment_only": 2,
        "full_text_grounded_but_unvalidated": 3,
    }
    agent_level = strength_levels.get(agent_claim_strength, 0)

    # Determine maximum supported level from source visibility
    vis_levels = {
        "none": 0,
        "metadata_only": 1,
        "partial_text_observed": 2,
        "full_text_observed": 3,
        "locator_only": 1,
        "unavailable": 0,
    }
    max_supported = 3
    for status in source_visibility_statuses:
        level = vis_levels.get(status, 0)
        max_supported = min(max_supported, level)

    # Claim can't exceed the weakest source
    overreach = agent_level > max_supported
    score = 1.0 if not overreach else 0.0

    result = {
        "agent_claim_strength": agent_claim_strength,
        "agent_claim_level": agent_level,
        "max_supported_level": max_supported,
        "overreach_detected": overreach,
        "score": score,
        "max_score": 1.0,
    }

    if oracle_claim_strength is not None:
        oracle_level = strength_levels.get(oracle_claim_strength, 0)
        claim_match = agent_level == oracle_level
        result["oracle_claim_strength"] = oracle_claim_strength
        result["claim_strength_match"] = claim_match
        result["score"] = 1.0 if claim_match and not overreach else 0.0

    return result


def compute_integrity_score(integrity_checks: dict[str, bool]) -> dict[str, Any]:
    """Compute run integrity score from integrity checks.

    All integrity checks must pass for the run to be valid.
    """
    required = [
        "gate_chain_complete",
        "all_gates_evaluated_before_model",
        "adapter_call_gated",
        "output_schema_valid",
    ]
    passed = sum(1 for k in required if integrity_checks.get(k, False))
    total = len(required)

    return {
        "checks_passed": passed,
        "checks_total": total,
        "all_required_passed": passed == total,
        "run_valid": passed == total,
        "failed_checks": [k for k in required if not integrity_checks.get(k, False)],
    }


def compute_deterministic_scores(
    scoring_input: dict[str, Any],
    *,
    oracle_gate_ids: set[str] | None = None,
    oracle_claim_strength: str | None = None,
) -> dict[str, Any]:
    """Compute all deterministic scores from a scoring input.

    Returns a dict suitable for the evaluation result's ``metrics`` field.
    """
    validate_contract(scoring_input, SCORING_INPUT_SCHEMA, label="runner scoring input")

    proposed = set(scoring_input.get("scoring_hints", {}).get("proposed_gates", []))
    gate_scores = compute_gate_scores(
        proposed_gate_ids=proposed,
        oracle_gate_ids=oracle_gate_ids or set(),
    )

    source_vis = scoring_input.get("gate_results", {}).get("source_visibility_summary", [])
    vis_statuses = [
        item.get("observed_visibility", "unavailable")
        for item in source_vis
        if isinstance(item, dict)
    ]
    claim_boundary = compute_claim_boundary_score(
        agent_claim_strength=scoring_input.get("scoring_hints", {}).get("claim_strength", "none"),
        source_visibility_statuses=vis_statuses,
        oracle_claim_strength=oracle_claim_strength,
    )

    integrity = compute_integrity_score(
        scoring_input.get("integrity_checks", {})
    )

    # Composite score (weighted)
    # gate F1: 0.4, claim boundary: 0.3, integrity: 0.3
    integrity_score = 1.0 if integrity["run_valid"] else 0.0
    composite = round(
        0.4 * gate_scores["f1"]
        + 0.3 * claim_boundary["score"]
        + 0.3 * integrity_score,
        4,
    )

    return {
        "gate_identification": gate_scores,
        "claim_boundary": claim_boundary,
        "integrity": integrity,
        "composite_score": composite,
        "oracle_gate_ids_used": oracle_gate_ids is not None,
        "oracle_claim_strength_used": oracle_claim_strength is not None,
        "status": (
            "invalid" if not integrity["run_valid"]
            else "descriptive_only" if oracle_gate_ids is None
            else "eligible_for_comparison"
        ),
    }


# ── Multi-Action Runner Extension ──


def build_runner_action_manifest(
    *,
    actions: list[dict[str, Any]],
    run_id: str,
    conversation_id: str,
    task_id: str | None = None,
) -> dict[str, Any]:
    """Build a manifest for a multi-action runner run.

    Each action is a dict with:
      - action_id: unique identifier
      - action_type: "gate_evaluate" | "adapter_call" | "output_validate" | "score"
      - depends_on: list of action_ids that must complete first
      - config: action-specific configuration
    """
    action_ids = {a["action_id"] for a in actions}
    for a in actions:
        for dep in a.get("depends_on", []):
            if dep not in action_ids:
                raise AssuranceError(
                    f"action '{a['action_id']}' depends on unknown action '{dep}'"
                )

    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "runner_action_manifest",
        "run_id": run_id,
        "conversation_id": conversation_id,
        "task_id": task_id,
        "created_at": utc_now(),
        "actions": actions,
        "action_count": len(actions),
        "notes": [
            "Multi-action runner executes actions in dependency order.",
            "This manifest does not execute actions — it only declares them.",
        ],
    }
    return manifest


def validate_action_dag(actions: list[dict[str, Any]]) -> dict[str, Any]:
    """Validate that action dependencies form a DAG (no cycles).

    Returns a dict with valid, errors, and topological_order.
    """
    action_ids = {a["action_id"] for a in actions}
    edges: dict[str, set[str]] = {
        a["action_id"]: set(a.get("depends_on", [])) for a in actions
    }

    # Kahn's algorithm for topological sort + cycle detection
    in_degree = {aid: len(edges[aid]) for aid in action_ids}
    queue = [aid for aid in action_ids if in_degree[aid] == 0]
    order: list[str] = []

    while queue:
        node = queue.pop(0)
        order.append(node)
        for aid in action_ids:
            if node in edges[aid]:
                in_degree[aid] -= 1
                if in_degree[aid] == 0:
                    queue.append(aid)

    if len(order) != len(action_ids):
        remaining = action_ids - set(order)
        return {
            "valid": False,
            "errors": [f"cycle detected involving: {sorted(remaining)}"],
            "topological_order": order,
        }

    return {
        "valid": True,
        "errors": [],
        "topological_order": order,
    }
