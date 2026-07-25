from __future__ import annotations

from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .utils import load_json


REGISTRY_SCHEMA = "general-science-artifact-registry-v0.1.schema.json"
DEFAULT_ARTIFACT_REGISTRY = (
    ASSURANCE_ROOT / "general-science-artifact-registry-v0.1.json"
)
REQUIRED_ARTIFACT_SCHEMAS = {
    "GSAS_NUMERICAL_TIME_STEP_0_1": {
        "version": "0.1.0",
        "artifact_kind": "numerical_result",
        "media_type": "application/json",
        "schema_name": (
            "general-science-numerical-time-step-result-v0.1.schema.json"
        ),
        "failure_decision": "block",
        "reason_code": "ART-SCHEMA-001",
    },
    "GSAS_STATISTICAL_SUMMARY_BASE_0_1": {
        "version": "0.1.0",
        "artifact_kind": "statistical_summary",
        "media_type": "application/json",
        "schema_name": (
            "general-science-statistical-summary-base-v0.1.schema.json"
        ),
        "failure_decision": "block",
        "reason_code": "ART-SCHEMA-001",
    },
    "GSAS_NUMERICAL_SINGLE_RUN_0_1": {
        "version": "0.1.0",
        "artifact_kind": "numerical_result",
        "media_type": "application/json",
        "schema_name": (
            "general-science-numerical-single-run-result-v0.1.schema.json"
        ),
        "failure_decision": "block",
        "reason_code": "ART-SCHEMA-001",
    },
}


def validate_artifact_registry_semantics(
    registry: dict[str, Any],
) -> None:
    records = registry["artifact_schemas"]
    identifiers = [item["artifact_schema_id"] for item in records]
    if len(identifiers) != len(set(identifiers)):
        raise AssuranceError("artifact schema registry IDs must be unique")
    schema_names = [item["schema_name"] for item in records]
    if len(schema_names) != len(set(schema_names)):
        raise AssuranceError(
            "artifact schema registry schema names must be unique"
        )
    missing = sorted(set(REQUIRED_ARTIFACT_SCHEMAS) - set(identifiers))
    if missing:
        raise AssuranceError(
            f"artifact schema registry is missing required entries: {missing}"
        )

    for record in records:
        schema_path = ASSURANCE_ROOT / record["schema_name"]
        if not schema_path.is_file():
            raise AssuranceError(
                f"{record['artifact_schema_id']} schema does not exist: "
                f"{record['schema_name']}"
            )
        try:
            Draft202012Validator.check_schema(load_json(schema_path))
        except Exception as exc:
            raise AssuranceError(
                f"{record['artifact_schema_id']} references an invalid schema: "
                f"{exc}"
            ) from exc
        expected = REQUIRED_ARTIFACT_SCHEMAS.get(
            record["artifact_schema_id"]
        )
        if expected is not None:
            observed = {
                key: record[key]
                for key in expected
            }
            if observed != expected:
                raise AssuranceError(
                    f"{record['artifact_schema_id']} weakens or changes its "
                    "required base policy"
                )


def load_general_science_artifact_registry(
    path: Path | None = None,
) -> dict[str, Any]:
    selected = path or DEFAULT_ARTIFACT_REGISTRY
    registry = load_json(selected)
    validate_contract(
        registry,
        REGISTRY_SCHEMA,
        label="general science artifact schema registry",
    )
    validate_artifact_registry_semantics(registry)
    return registry


def index_artifact_schemas(
    registry: dict[str, Any],
) -> dict[str, dict[str, Any]]:
    validate_artifact_registry_semantics(registry)
    return {
        item["artifact_schema_id"]: item
        for item in registry["artifact_schemas"]
    }
