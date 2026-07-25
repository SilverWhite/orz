from __future__ import annotations

import json
import math
import operator
from pathlib import Path
from typing import Any, Callable

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import (
    is_link_or_reparse,
    require_no_linked_ancestors,
    require_within,
    safe_relative_path,
    sha256_bytes,
)
from .validator_bridge import run_general_science_validators


BUNDLE_SCHEMA = "general-science-review-bundle-v0.1.schema.json"
RESULT_SCHEMA = "general-science-review-result-v0.1.schema.json"
UNCERTAIN_EVIDENCE_CLASSES = {
    "bridge_hypothesis",
    "user_supplied_unverified",
    "agent_inferred",
    "unchecked_risk",
}
COMPARATORS: dict[str, Callable[[Any, Any], bool]] = {
    "lt": operator.lt,
    "lte": operator.le,
    "gt": operator.gt,
    "gte": operator.ge,
    "eq": operator.eq,
    "neq": operator.ne,
}


def _unique_by_id(
    records: list[dict[str, Any]], key: str, *, label: str
) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for record in records:
        record_id = record[key]
        if record_id in indexed:
            raise AssuranceError(f"duplicate {label} ID: {record_id}")
        indexed[record_id] = record
    return indexed


def _read_regular_file(
    root: Path,
    relative_value: str,
    *,
    expected_sha256: str | None,
    label: str,
) -> tuple[bytes, dict[str, Any]]:
    relative = safe_relative_path(relative_value)
    path = root / Path(*relative.parts)
    require_within(path, root, must_exist=True)
    require_no_linked_ancestors(path, root)
    if is_link_or_reparse(path) or not path.is_file():
        raise AssuranceError(f"{label} must be a regular non-linked file: {path}")
    try:
        payload = path.read_bytes()
    except OSError as exc:
        raise AssuranceError(f"cannot read {label} {path}: {exc}") from exc
    if not payload:
        raise AssuranceError(f"{label} is empty: {path}")
    digest = sha256_bytes(payload)
    if expected_sha256 is not None and digest != expected_sha256:
        raise AssuranceError(
            f"{label} digest mismatch: {relative_value}; "
            f"expected {expected_sha256}, observed {digest}"
        )
    return payload, {
        "path": relative_value,
        "bytes": len(payload),
        "sha256": digest,
    }


def _parse_json(payload: bytes, *, label: str) -> Any:
    try:
        value = json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise AssuranceError(f"cannot parse {label} as UTF-8 JSON: {exc}") from exc
    _reject_nonfinite(value, label=label)
    return value


def _reject_nonfinite(value: Any, *, label: str) -> None:
    if isinstance(value, float) and not math.isfinite(value):
        raise AssuranceError(f"{label} contains a non-finite number")
    if isinstance(value, dict):
        for child in value.values():
            _reject_nonfinite(child, label=label)
    elif isinstance(value, list):
        for child in value:
            _reject_nonfinite(child, label=label)


def _require_ids(
    values: list[str], known: dict[str, Any], *, label: str
) -> None:
    missing = sorted(set(values) - set(known))
    if missing:
        raise AssuranceError(f"{label} references unknown IDs: {missing}")


def _json_pointer(document: Any, pointer: str, *, label: str) -> Any:
    if pointer == "":
        return document
    current = document
    for raw_token in pointer[1:].split("/"):
        token = raw_token.replace("~1", "/").replace("~0", "~")
        try:
            if isinstance(current, list):
                if token == "-" or not token.isdigit():
                    raise KeyError(token)
                current = current[int(token)]
            elif isinstance(current, dict):
                current = current[token]
            else:
                raise KeyError(token)
        except (KeyError, IndexError) as exc:
            raise AssuranceError(
                f"{label} cannot resolve JSON pointer {pointer!r}"
            ) from exc
    return current


def _verify_comparison(
    evidence: dict[str, Any],
    artifact_documents: dict[str, Any],
) -> dict[str, Any]:
    comparison = evidence["comparison"]
    artifact_id = comparison["artifact_id"]
    if artifact_id not in artifact_documents:
        raise AssuranceError(
            f"comparison references non-JSON or unknown artifact: {artifact_id}"
        )
    document = artifact_documents[artifact_id]
    left = _json_pointer(
        document,
        comparison["left_pointer"],
        label=evidence["evidence_id"],
    )
    right = _json_pointer(
        document,
        comparison["right_pointer"],
        label=evidence["evidence_id"],
    )
    if isinstance(left, (dict, list)) or isinstance(right, (dict, list)):
        raise AssuranceError(
            f"{evidence['evidence_id']} comparison values must be scalars"
        )
    op_name = comparison["operator"]
    if op_name in {"lt", "lte", "gt", "gte"}:
        numeric = (
            isinstance(left, (int, float))
            and not isinstance(left, bool)
            and isinstance(right, (int, float))
            and not isinstance(right, bool)
        )
        if not numeric:
            raise AssuranceError(
                f"{evidence['evidence_id']} ordered comparison requires numbers"
            )
    try:
        passed = COMPARATORS[op_name](left, right)
    except TypeError as exc:
        raise AssuranceError(
            f"{evidence['evidence_id']} comparison operands are incompatible"
        ) from exc
    if not passed:
        raise AssuranceError(
            f"{evidence['evidence_id']} declared direct comparison is false"
        )
    return {
        "evidence_id": evidence["evidence_id"],
        "artifact_id": artifact_id,
        "left_pointer": comparison["left_pointer"],
        "operator": op_name,
        "right_pointer": comparison["right_pointer"],
        "left_value": left,
        "right_value": right,
        "passed": True,
    }


def _validate_design(document: Any, *, expected_source_id: str) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise AssuranceError(
            f"design source {expected_source_id} must contain a JSON object"
        )
    required = {
        "design_id",
        "question",
        "design_kind",
        "hypotheses",
        "controls",
        "falsifiers",
        "analysis_plan",
        "coverage_complete",
    }
    missing = sorted(required - set(document))
    if missing:
        raise AssuranceError(f"design source is missing fields: {missing}")
    if document["design_kind"] not in {
        "descriptive_comparison",
        "observational",
        "controlled_intervention",
    }:
        raise AssuranceError("design source has an unsupported design_kind")
    for field in ("hypotheses", "controls", "falsifiers", "analysis_plan"):
        if not isinstance(document[field], list):
            raise AssuranceError(f"design source field {field} must be an array")
    if not isinstance(document["coverage_complete"], bool):
        raise AssuranceError("design coverage_complete must be boolean")
    return document


def _validate_action(
    document: Any,
    *,
    expected_source_id: str,
    sources: dict[str, Any],
    artifacts: dict[str, Any],
) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise AssuranceError(
            f"action source {expected_source_id} must contain a JSON object"
        )
    required = {
        "action_id",
        "kind",
        "state",
        "terminal_event_count",
        "read_only_review_eligible",
        "source_ids",
        "artifact_ids",
        "safety",
    }
    missing = sorted(required - set(document))
    if missing:
        raise AssuranceError(f"action source is missing fields: {missing}")
    if document["state"] != "completed":
        raise AssuranceError("reviewed action must be completed")
    if document["terminal_event_count"] != 1:
        raise AssuranceError("reviewed action must have exactly one terminal event")
    if document["read_only_review_eligible"] is not True:
        raise AssuranceError("reviewed action is not eligible for read-only review")
    if not isinstance(document["source_ids"], list) or not isinstance(
        document["artifact_ids"], list
    ):
        raise AssuranceError("action source/artifact references must be arrays")
    _require_ids(document["source_ids"], sources, label="action")
    _require_ids(document["artifact_ids"], artifacts, label="action")
    expected_safety = {
        "generated_before_review": True,
        "review_requires_execution": False,
        "review_requires_network": False,
        "review_requires_model": False,
    }
    if document["safety"] != expected_safety:
        raise AssuranceError(
            "action safety must prove pre-generation and no review-time "
            "execution, network, or model requirement"
        )
    return document


def _claim_decision(
    claim: dict[str, Any],
    evidence_by_id: dict[str, dict[str, Any]],
    comparisons: dict[str, dict[str, Any]],
    design: dict[str, Any],
) -> dict[str, Any]:
    support_ids = claim["supporting_evidence_ids"]
    supporting = [evidence_by_id[item] for item in support_ids]
    refuting = [
        evidence_by_id[item]
        for item in claim["refuting_evidence_ids"]
        if evidence_by_id[item]["status"] == "observed"
    ]
    reasons: list[str] = []
    decision = "allow"

    if not supporting:
        decision = "block"
        reasons.append("MISSING_SUPPORT")
    if refuting:
        if decision != "block":
            decision = "defer"
        reasons.append("REFUTING_EVIDENCE_PRESENT")
    if any(
        item["evidence_class"] in UNCERTAIN_EVIDENCE_CLASSES
        for item in supporting
    ):
        if decision != "block":
            decision = "defer"
        reasons.append("UNCERTAIN_EVIDENCE_CLASS")
    if any(item["status"] != "observed" for item in supporting):
        if decision != "block":
            decision = "defer"
        reasons.append("SUPPORT_NOT_OBSERVED")

    direct_support = [
        item
        for item in supporting
        if item["evidence_class"] == "direct_comparison"
        and item["evidence_id"] in comparisons
    ]
    claim_type = claim["claim_type"]
    if claim_type in {"causal", "mechanism", "generality"}:
        if design["design_kind"] != "controlled_intervention":
            decision = "defer" if decision != "block" else decision
            reasons.append("CONTROLLED_INTERVENTION_REQUIRED")
        if not design["controls"]:
            decision = "defer" if decision != "block" else decision
            reasons.append("DESIGN_CONTROLS_REQUIRED")
        if not design["falsifiers"] or not claim["falsifiers"]:
            decision = "defer" if decision != "block" else decision
            reasons.append("DESIGN_FALSIFIERS_REQUIRED")
        if not direct_support:
            decision = "defer" if decision != "block" else decision
            reasons.append("DIRECT_COMPARISON_REQUIRED")
        decision = "defer" if decision != "block" else decision
        reasons.append("STRONG_CLAIM_VALIDATOR_UNAVAILABLE")
    if claim_type == "generality" and not design["coverage_complete"]:
        decision = "defer" if decision != "block" else decision
        reasons.append("COVERAGE_INCOMPLETE")

    if not reasons:
        reasons.append(
            "ASSOCIATION_SUPPORTED"
            if claim_type == "association"
            else "DIRECT_OBSERVATION_SUPPORTED"
        )
    return {
        "claim_id": claim["claim_id"],
        "decision": decision,
        "reason_codes": reasons,
        "supporting_evidence_ids": support_ids,
        "verified_comparisons": [
            comparisons[item["evidence_id"]]
            for item in direct_support
        ],
    }


def review_general_science_bundle(
    *,
    bundle_root: Path,
    bundle_name: str = "review-bundle.json",
) -> dict[str, Any]:
    """Mechanically review a pre-existing multi-file scientific bundle.

    The function performs file reads and in-process validation only. It does
    not write reviewed content, execute scientific code, invoke a model,
    request a network resource, or spawn a child process.
    """

    if not bundle_root.is_dir() or is_link_or_reparse(bundle_root):
        raise AssuranceError(
            "bundle root must be an existing non-linked directory"
        )
    root = bundle_root.resolve(strict=True)
    bundle_payload, bundle_file = _read_regular_file(
        root,
        bundle_name,
        expected_sha256=None,
        label="review bundle",
    )
    bundle = _parse_json(bundle_payload, label="review bundle")
    validate_contract(bundle, BUNDLE_SCHEMA, label="general science review bundle")

    sources = _unique_by_id(bundle["sources"], "record_id", label="source")
    artifacts = _unique_by_id(bundle["artifacts"], "record_id", label="artifact")
    overlap = sorted(set(sources) & set(artifacts))
    if overlap:
        raise AssuranceError(f"source/artifact IDs overlap: {overlap}")
    paths = [item["path"] for item in [*sources.values(), *artifacts.values()]]
    if len(paths) != len(set(paths)):
        raise AssuranceError("source and artifact paths must be unique")
    _require_ids(
        bundle["task_contract"]["source_of_truth"],
        sources,
        label="task contract",
    )
    _require_ids(
        [bundle["design_source_id"], bundle["action_source_id"]],
        sources,
        label="bundle control sources",
    )
    source_of_truth = set(bundle["task_contract"]["source_of_truth"])
    control_sources = {
        bundle["design_source_id"],
        bundle["action_source_id"],
    }
    if not control_sources.issubset(source_of_truth):
        raise AssuranceError(
            "design and action control sources must be task sources of truth"
        )

    files = [
        {
            "record_type": "bundle",
            "record_id": bundle["bundle_id"],
            **bundle_file,
        }
    ]
    source_documents: dict[str, Any] = {}
    artifact_documents: dict[str, Any] = {}
    for record_type, records, documents in (
        ("source", sources, source_documents),
        ("artifact", artifacts, artifact_documents),
    ):
        for record_id, record in records.items():
            payload, file_record = _read_regular_file(
                root,
                record["path"],
                expected_sha256=record["sha256"],
                label=f"{record_type} {record_id}",
            )
            files.append(
                {
                    "record_type": record_type,
                    "record_id": record_id,
                    **file_record,
                }
            )
            if record["media_type"] == "application/json":
                documents[record_id] = _parse_json(
                    payload, label=f"{record_type} {record_id}"
                )

    design_id = bundle["design_source_id"]
    action_source_id = bundle["action_source_id"]
    if design_id not in source_documents:
        raise AssuranceError("design source must use application/json")
    if action_source_id not in source_documents:
        raise AssuranceError("action source must use application/json")
    validator_run = run_general_science_validators(
        design=source_documents[design_id],
        action=source_documents[action_source_id],
        artifacts=artifacts,
        artifact_documents=artifact_documents,
    )
    blocking_validators = [
        item
        for item in validator_run["results"]
        if item["status"] == "block"
    ]
    if blocking_validators:
        rendered = ", ".join(
            f"{item['validator_id']}:{item['target_id']}"
            for item in blocking_validators
        )
        raise AssuranceError(f"validator bridge blocked review: {rendered}")
    design = _validate_design(
        source_documents[design_id], expected_source_id=design_id
    )
    action = _validate_action(
        source_documents[action_source_id],
        expected_source_id=action_source_id,
        sources=sources,
        artifacts=artifacts,
    )
    for artifact_id, artifact in artifacts.items():
        if artifact["producer_action_id"] != action["action_id"]:
            raise AssuranceError(
                f"artifact {artifact_id} producer does not match reviewed action"
            )
        if artifact_id not in action["artifact_ids"]:
            raise AssuranceError(
                f"artifact {artifact_id} is absent from the action manifest"
            )

    evidence_by_id = _unique_by_id(
        bundle["evidence"], "evidence_id", label="evidence"
    )
    claims_by_id = _unique_by_id(bundle["claims"], "claim_id", label="claim")
    comparisons: dict[str, dict[str, Any]] = {}
    for evidence_id, evidence in evidence_by_id.items():
        _require_ids(evidence["source_ids"], sources, label=evidence_id)
        _require_ids(evidence["artifact_ids"], artifacts, label=evidence_id)
        if evidence["evidence_class"] == "direct_comparison":
            comparison_artifact = evidence["comparison"]["artifact_id"]
            if comparison_artifact not in evidence["artifact_ids"]:
                raise AssuranceError(
                    f"{evidence_id} comparison artifact is not declared as evidence"
                )
            comparisons[evidence_id] = _verify_comparison(
                evidence, artifact_documents
            )
    for claim in claims_by_id.values():
        _require_ids(
            claim["supporting_evidence_ids"],
            evidence_by_id,
            label=claim["claim_id"],
        )
        _require_ids(
            claim["refuting_evidence_ids"],
            evidence_by_id,
            label=claim["claim_id"],
        )

    claim_decisions = [
        _claim_decision(claim, evidence_by_id, comparisons, design)
        for claim in claims_by_id.values()
    ]
    decisions = {item["decision"] for item in claim_decisions}
    validator_decisions = {
        item["status"]
        for item in validator_run["results"]
        if item["status"] in {"defer", "block"}
    }
    overall = (
        "block"
        if "block" in decisions or "block" in validator_decisions
        else "defer"
        if "defer" in decisions or "defer" in validator_decisions
        else "allow"
    )
    bundle_digest = sha256_bytes(bundle_payload)
    result = {
        "schema_version": "0.1.0-draft",
        "result_kind": "general_science_readonly_review_result",
        "review_id": f"REVIEW_{bundle_digest[:24].upper()}",
        "bundle_id": bundle["bundle_id"],
        "bundle_sha256": bundle_digest,
        "validator_registry_id": validator_run["registry_id"],
        "validator_registry_sha256": validator_run["registry_sha256"],
        "valid": True,
        "decision": overall,
        "files": files,
        "checks": {
            "contract_valid": True,
            "paths_safe": True,
            "digests_match": True,
            "references_resolve": True,
            "artifacts_parse": True,
            "comparisons_verified": True,
            "validators_completed": True,
        },
        "validator_results": validator_run["results"],
        "claim_decisions": claim_decisions,
        "safety": {
            "source_write_attempted": False,
            "analysis_code_executed": False,
            "model_invoked": False,
            "network_requested": False,
            "child_process_spawned": False,
        },
        "limitations": [
            "This is deterministic contract and evidence plumbing, not full scientific peer review.",
            "No statistical uncertainty, robustness, power, convergence order, or external validity is inferred.",
            "The reviewer trusts declared pre-review generation provenance only after digest and reference checks; it does not reproduce the experiment.",
            "The result is not a runtime security boundary or an EvaluationRunner score.",
        ],
    }
    validate_contract(result, RESULT_SCHEMA, label="general science review result")
    return result
