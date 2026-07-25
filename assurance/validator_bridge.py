from __future__ import annotations

import math
from pathlib import Path
from typing import Any

from .artifact_registry import (
    DEFAULT_ARTIFACT_REGISTRY,
    index_artifact_schemas,
    load_general_science_artifact_registry,
)
from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .utils import load_json, sha256_file


REGISTRY_SCHEMA = "general-science-validator-registry-v0.1.schema.json"
DEFAULT_VALIDATOR_REGISTRY = (
    ASSURANCE_ROOT / "general-science-validator-registry-v0.1.json"
)
REQUIRED_VALIDATORS = {
    "GSV_DESIGN_SCHEMA": {
        "stage": "design",
        "implementation": "json_schema",
        "failure_decision": "block",
        "reason_code": "TASK-CONTRACT-001",
    },
    "GSV_ACTION_SCHEMA": {
        "stage": "action",
        "implementation": "json_schema",
        "failure_decision": "block",
        "reason_code": "TASK-CONTRACT-001",
    },
    "GSV_ARTIFACT_REGISTERED_SCHEMA": {
        "stage": "artifact",
        "implementation": "registered_artifact_schema",
        "failure_decision": "block",
        "reason_code": "ART-SCHEMA-001",
    },
    "GSV_ARTIFACT_FINITE_JSON": {
        "stage": "artifact",
        "implementation": "finite_json",
        "failure_decision": "block",
        "reason_code": "ART-FINITE-001",
    },
    "GSV_STATISTICAL_REPORTING": {
        "stage": "artifact",
        "implementation": "statistical_reporting",
        "failure_decision": "defer",
        "reason_code": "EVD-COVERAGE-001",
    },
    "GSV_CROSS_ARTIFACT_COMPARABILITY": {
        "stage": "evidence",
        "implementation": "cross_artifact_comparability",
        "failure_decision": "defer",
        "reason_code": "EVD-COMPARABILITY-001",
    },
}


def validate_validator_registry_semantics(
    registry: dict[str, Any],
) -> None:
    validators = registry["validators"]
    identifiers = [item["validator_id"] for item in validators]
    if len(identifiers) != len(set(identifiers)):
        raise AssuranceError("validator registry IDs must be unique")
    missing = sorted(set(REQUIRED_VALIDATORS) - set(identifiers))
    if missing:
        raise AssuranceError(
            f"validator registry is missing required validators: {missing}"
        )

    for validator in validators:
        implementation = validator["implementation"]
        stage = validator["stage"]
        schema_name = validator.get("schema_name")
        artifact_kinds = validator["artifact_kinds"]
        if implementation == "json_schema":
            if stage not in {"design", "action"}:
                raise AssuranceError(
                    f"{validator['validator_id']} schema validator has invalid stage"
                )
            if schema_name is None:
                raise AssuranceError(
                    f"{validator['validator_id']} is missing schema_name"
                )
            if artifact_kinds:
                raise AssuranceError(
                    f"{validator['validator_id']} control validator cannot "
                    "declare artifact kinds"
                )
            if not (ASSURANCE_ROOT / schema_name).is_file():
                raise AssuranceError(
                    f"{validator['validator_id']} schema does not exist: "
                    f"{schema_name}"
                )
        elif implementation == "cross_artifact_comparability":
            if stage != "evidence" or artifact_kinds or schema_name is not None:
                raise AssuranceError(
                    f"{validator['validator_id']} evidence validator has "
                    "invalid applicability"
                )
        else:
            if stage != "artifact" or not artifact_kinds:
                raise AssuranceError(
                    f"{validator['validator_id']} artifact validator has "
                    "invalid stage or applicability"
                )
            if schema_name is not None:
                raise AssuranceError(
                    f"{validator['validator_id']} built-in validator cannot "
                    "declare schema_name"
                )
        expected = REQUIRED_VALIDATORS.get(validator["validator_id"])
        if expected is not None:
            observed = {
                key: validator[key]
                for key in expected
            }
            if observed != expected:
                raise AssuranceError(
                    f"{validator['validator_id']} weakens or changes its "
                    "required base policy"
                )


def load_general_science_validator_registry(
    path: Path | None = None,
) -> dict[str, Any]:
    selected = path or DEFAULT_VALIDATOR_REGISTRY
    registry = load_json(selected)
    validate_contract(
        registry,
        REGISTRY_SCHEMA,
        label="general science validator registry",
    )
    validate_validator_registry_semantics(registry)
    return registry


def _finite(value: Any) -> bool:
    if isinstance(value, float):
        return math.isfinite(value)
    if isinstance(value, dict):
        return all(_finite(child) for child in value.values())
    if isinstance(value, list):
        return all(_finite(child) for child in value)
    return True


def _positive_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def _nonnegative_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def _finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(value)
    )


def _statistical_reporting_findings(document: Any) -> list[str]:
    if not isinstance(document, dict):
        return ["artifact is not a JSON object"]
    reporting = document.get("statistical_reporting")
    if not isinstance(reporting, dict):
        return ["missing statistical_reporting object"]
    required = {
        "analysis_role",
        "sampling_unit",
        "sample_size",
        "effect_estimate",
        "uncertainty",
        "missing_data",
        "multiplicity",
        "stopping_rule",
    }
    findings = [
        f"missing statistical_reporting.{field}"
        for field in sorted(required - set(reporting))
    ]
    if findings:
        return findings
    if not isinstance(reporting["analysis_role"], str) or reporting[
        "analysis_role"
    ] not in {"confirmatory", "exploratory"}:
        findings.append(
            "analysis_role must be confirmatory or exploratory"
        )
    if not isinstance(reporting["sampling_unit"], str) or not reporting[
        "sampling_unit"
    ].strip():
        findings.append("sampling_unit must be a non-empty string")
    if not _positive_integer(reporting["sample_size"]):
        findings.append("sample_size must be a positive integer")
    estimate = reporting["effect_estimate"]
    if not _finite_number(estimate):
        findings.append("effect_estimate must be finite")

    uncertainty = reporting["uncertainty"]
    if not isinstance(uncertainty, dict):
        findings.append("uncertainty must be an object")
    else:
        uncertainty_required = {"kind", "level", "lower", "upper"}
        missing = sorted(uncertainty_required - set(uncertainty))
        findings.extend(f"uncertainty missing {field}" for field in missing)
        if not missing:
            if not isinstance(uncertainty["kind"], str) or uncertainty[
                "kind"
            ] not in {
                "confidence_interval",
                "credible_interval",
            }:
                findings.append("uncertainty kind is unsupported")
            level = uncertainty["level"]
            if not _finite_number(level) or not 0 < level < 1:
                findings.append("uncertainty level must be between 0 and 1")
            lower = uncertainty["lower"]
            upper = uncertainty["upper"]
            if not _finite_number(lower) or not _finite_number(upper):
                findings.append("uncertainty bounds must be finite")
            elif lower > upper:
                findings.append("uncertainty lower bound exceeds upper bound")
            elif _finite_number(estimate) and not lower <= estimate <= upper:
                findings.append("effect estimate lies outside uncertainty bounds")

    missing_data = reporting["missing_data"]
    if not isinstance(missing_data, dict):
        findings.append("missing_data must be an object")
    else:
        if not _nonnegative_integer(missing_data.get("count")):
            findings.append("missing_data.count must be a non-negative integer")
        if not isinstance(missing_data.get("handling"), str) or not missing_data[
            "handling"
        ].strip():
            findings.append("missing_data.handling must be a non-empty string")

    multiplicity = reporting["multiplicity"]
    if not isinstance(multiplicity, dict):
        findings.append("multiplicity must be an object")
    else:
        if not _positive_integer(multiplicity.get("comparison_count")):
            findings.append(
                "multiplicity.comparison_count must be a positive integer"
            )
        if not isinstance(multiplicity.get("adjustment"), str) or not multiplicity[
            "adjustment"
        ].strip():
            findings.append(
                "multiplicity.adjustment must be a non-empty string"
            )
    if not isinstance(reporting["stopping_rule"], str) or not reporting[
        "stopping_rule"
    ].strip():
        findings.append("stopping_rule must be a non-empty string")
    return findings


def _resolve_pointer(document: Any, pointer: str) -> Any:
    if not isinstance(pointer, str):
        raise KeyError(pointer)
    if pointer == "":
        return document
    current = document
    for raw_token in pointer[1:].split("/"):
        token = raw_token.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            if token == "-" or not token.isdigit():
                raise KeyError(pointer)
            current = current[int(token)]
        elif isinstance(current, dict):
            current = current[token]
        else:
            raise KeyError(pointer)
    return current


def _comparison_artifact_ids(
    evidence: dict[str, Any],
) -> tuple[str, str]:
    comparison = evidence["comparison"]
    if "artifact_id" in comparison:
        return comparison["artifact_id"], comparison["artifact_id"]
    return (
        comparison["left_artifact_id"],
        comparison["right_artifact_id"],
    )


def _cross_artifact_findings(
    *,
    left_id: str,
    right_id: str,
    artifacts: dict[str, dict[str, Any]],
    artifact_documents: dict[str, Any],
    action: dict[str, Any],
) -> list[str]:
    findings: list[str] = []
    left_record = artifacts.get(left_id)
    right_record = artifacts.get(right_id)
    left = artifact_documents.get(left_id)
    right = artifact_documents.get(right_id)
    if left_record is None or right_record is None:
        return ["comparison references an unknown artifact"]
    if not isinstance(left, dict) or not isinstance(right, dict):
        return ["cross-artifact comparison requires JSON object artifacts"]
    left_context = left.get("comparison_context")
    right_context = right.get("comparison_context")
    if not isinstance(left_context, dict) or not isinstance(
        right_context, dict
    ):
        return ["both artifacts must declare comparison_context"]

    invariant_context_fields = (
        "task_id",
        "protocol_id",
        "metric_id",
        "population_id",
        "unit",
        "comparison_family_id",
        "varied_dimension",
    )
    for field in invariant_context_fields:
        if left_context.get(field) != right_context.get(field):
            findings.append(f"comparison_context.{field} differs")
    invariant_document_fields = (
        "method",
        "equation",
        "initial_value",
        "final_time",
        "exact_final_value",
    )
    for field in invariant_document_fields:
        if left.get(field) != right.get(field):
            findings.append(f"artifact field {field} differs")

    left_lineage = left_context.get("lineage")
    right_lineage = right_context.get("lineage")
    if not isinstance(left_lineage, dict) or not isinstance(
        right_lineage, dict
    ):
        findings.append("both artifacts must declare lineage")
    else:
        action_source_ids = action.get("source_ids", [])
        action_sources = (
            set(action_source_ids)
            if isinstance(action_source_ids, list)
            and all(isinstance(item, str) for item in action_source_ids)
            else None
        )
        for lineage, record, label in (
            (left_lineage, left_record, "left"),
            (right_lineage, right_record, "right"),
        ):
            if lineage.get("producer_action_id") != record.get(
                "producer_action_id"
            ):
                findings.append(
                    f"{label} lineage producer does not match artifact record"
                )
            if lineage.get("producer_action_id") != action.get("action_id"):
                findings.append(
                    f"{label} lineage producer does not match action manifest"
                )
            lineage_source_ids = lineage.get("source_ids", [])
            lineage_sources = (
                set(lineage_source_ids)
                if isinstance(lineage_source_ids, list)
                and all(
                    isinstance(item, str)
                    for item in lineage_source_ids
                )
                else None
            )
            if (
                action_sources is None
                or lineage_sources is None
                or lineage_sources != action_sources
            ):
                findings.append(
                    f"{label} lineage sources do not match action manifest"
                )
        if left_lineage.get("transformation_id") != right_lineage.get(
            "transformation_id"
        ):
            findings.append("lineage transformation_id differs")

    left_condition = left_context.get("condition")
    right_condition = right_context.get("condition")
    if not isinstance(left_condition, dict) or not isinstance(
        right_condition, dict
    ):
        findings.append("both artifacts must declare comparison condition")
    else:
        varied_dimension = left_context.get("varied_dimension")
        if left_condition.get("dimension") != varied_dimension:
            findings.append("left condition does not match varied_dimension")
        if right_condition.get("dimension") != varied_dimension:
            findings.append("right condition does not match varied_dimension")
        if left_condition.get("dimension") != right_condition.get("dimension"):
            findings.append("condition dimensions differ")
        if left_condition.get("value") == right_condition.get("value"):
            findings.append("condition values do not differ")
        for document, condition, label in (
            (left, left_condition, "left"),
            (right, right_condition, "right"),
        ):
            try:
                observed = _resolve_pointer(
                    document, condition.get("value_pointer", "")
                )
            except (KeyError, IndexError, TypeError):
                findings.append(f"{label} condition value pointer is invalid")
            else:
                if observed != condition.get("value"):
                    findings.append(
                        f"{label} condition value disagrees with artifact"
                    )
    return findings


def _result(
    validator: dict[str, Any],
    *,
    target_type: str,
    target_id: str,
    applicable: bool,
    status: str,
    details: list[str],
) -> dict[str, Any]:
    return {
        "validator_id": validator["validator_id"],
        "target_type": target_type,
        "target_id": target_id,
        "applicable": applicable,
        "status": status,
        "reason_codes": [
            validator["reason_code"]
            if status in {"defer", "block"}
            else "VALIDATOR_PASS"
            if status == "pass"
            else "VALIDATOR_NOT_APPLICABLE"
        ],
        "details": details,
    }


def run_general_science_validators(
    *,
    design: dict[str, Any],
    action: dict[str, Any],
    artifacts: dict[str, dict[str, Any]],
    artifact_documents: dict[str, Any],
    evidence: list[dict[str, Any]],
    registry_path: Path | None = None,
    artifact_registry_path: Path | None = None,
) -> dict[str, Any]:
    registry_file = registry_path or DEFAULT_VALIDATOR_REGISTRY
    registry = load_general_science_validator_registry(registry_file)
    artifact_registry_file = (
        artifact_registry_path or DEFAULT_ARTIFACT_REGISTRY
    )
    artifact_registry = load_general_science_artifact_registry(
        artifact_registry_file
    )
    artifact_schemas = index_artifact_schemas(artifact_registry)
    results: list[dict[str, Any]] = []
    for validator in registry["validators"]:
        implementation = validator["implementation"]
        stage = validator["stage"]
        if stage in {"design", "action"}:
            target = design if stage == "design" else action
            target_id = (
                target.get("design_id", "DESIGN_UNKNOWN")
                if stage == "design"
                else target.get("action_id", "ACTION_UNKNOWN")
            )
            try:
                validate_contract(
                    target,
                    validator["schema_name"],
                    label=f"{validator['validator_id']} target",
                )
            except AssuranceError as exc:
                results.append(
                    _result(
                        validator,
                        target_type=stage,
                        target_id=target_id,
                        applicable=True,
                        status=validator["failure_decision"],
                        details=[str(exc)],
                    )
                )
            else:
                results.append(
                    _result(
                        validator,
                        target_type=stage,
                        target_id=target_id,
                        applicable=True,
                        status="pass",
                        details=["registered schema validated"],
                    )
                )
            continue

        if stage == "evidence":
            for evidence_item in evidence:
                if evidence_item["evidence_class"] != "direct_comparison":
                    continue
                left_id, right_id = _comparison_artifact_ids(evidence_item)
                if left_id == right_id:
                    results.append(
                        _result(
                            validator,
                            target_type="evidence",
                            target_id=evidence_item["evidence_id"],
                            applicable=False,
                            status="not_applicable",
                            details=[
                                "comparison is contained in one artifact"
                            ],
                        )
                    )
                    continue
                findings = _cross_artifact_findings(
                    left_id=left_id,
                    right_id=right_id,
                    artifacts=artifacts,
                    artifact_documents=artifact_documents,
                    action=action,
                )
                results.append(
                    _result(
                        validator,
                        target_type="evidence",
                        target_id=evidence_item["evidence_id"],
                        applicable=True,
                        status=(
                            validator["failure_decision"]
                            if findings
                            else "pass"
                        ),
                        details=(
                            findings
                            if findings
                            else [
                                "cross-artifact lineage and comparability match"
                            ]
                        ),
                    )
                )
            continue

        for artifact_id, artifact in artifacts.items():
            artifact_kind = artifact["artifact_kind"]
            if artifact_kind not in validator["artifact_kinds"]:
                results.append(
                    _result(
                        validator,
                        target_type="artifact",
                        target_id=artifact_id,
                        applicable=False,
                        status="not_applicable",
                        details=[
                            f"artifact kind {artifact_kind} is outside applicability"
                        ],
                    )
                )
                continue
            document = artifact_documents.get(artifact_id)
            if implementation == "registered_artifact_schema":
                schema_id = artifact["artifact_schema_id"]
                registration = artifact_schemas.get(schema_id)
                details: list[str] = []
                if registration is None:
                    details.append(
                        f"artifact schema is not registered: {schema_id}"
                    )
                else:
                    if registration["artifact_kind"] != artifact_kind:
                        details.append(
                            "artifact kind does not match schema registration"
                        )
                    if registration["media_type"] != artifact["media_type"]:
                        details.append(
                            "artifact media type does not match schema registration"
                        )
                    if document is None:
                        details.append(
                            "registered JSON artifact has no parsed document"
                        )
                    if not details:
                        try:
                            validate_contract(
                                document,
                                registration["schema_name"],
                                label=(
                                    f"{validator['validator_id']} "
                                    f"target {artifact_id}"
                                ),
                            )
                        except AssuranceError as exc:
                            details.append(str(exc))
                results.append(
                    _result(
                        validator,
                        target_type="artifact",
                        target_id=artifact_id,
                        applicable=True,
                        status=(
                            validator["failure_decision"]
                            if details
                            else "pass"
                        ),
                        details=(
                            details
                            if details
                            else [
                                f"validated registered schema {schema_id}"
                            ]
                        ),
                    )
                )
            elif implementation == "finite_json":
                passed = document is not None and _finite(document)
                results.append(
                    _result(
                        validator,
                        target_type="artifact",
                        target_id=artifact_id,
                        applicable=True,
                        status="pass" if passed else validator["failure_decision"],
                        details=[
                            "all JSON numeric values are finite"
                            if passed
                            else "artifact is non-JSON or contains non-finite values"
                        ],
                    )
                )
            elif implementation == "statistical_reporting":
                findings = _statistical_reporting_findings(document)
                results.append(
                    _result(
                        validator,
                        target_type="artifact",
                        target_id=artifact_id,
                        applicable=True,
                        status=(
                            validator["failure_decision"]
                            if findings
                            else "pass"
                        ),
                        details=(
                            findings
                            if findings
                            else [
                                "basic statistical reporting fields are complete"
                            ]
                        ),
                    )
                )
            else:
                raise AssuranceError(
                    f"unsupported validator implementation: {implementation}"
                )
    return {
        "registry_id": registry["registry_id"],
        "registry_sha256": sha256_file(registry_file),
        "artifact_registry_id": artifact_registry["registry_id"],
        "artifact_registry_sha256": sha256_file(artifact_registry_file),
        "results": results,
    }
