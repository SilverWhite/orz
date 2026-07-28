from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
import tomllib
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker
import yaml


ROOT = Path(__file__).resolve().parents[1]
NON_REPOSITORY_PARTS = {
    ".git",
    ".observed-runs",
    ".pytest_cache",
    ".tools",
    "__pycache__",
}


class UniqueKeyLoader(yaml.SafeLoader):
    pass


def _unique_mapping(
    loader: UniqueKeyLoader, node: yaml.Node, deep: bool = False
) -> dict[Any, Any]:
    mapping: dict[Any, Any] = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in mapping:
            raise ValueError(
                f"duplicate YAML key {key!r} at line {key_node.start_mark.line + 1}"
            )
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


UniqueKeyLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _unique_mapping
)


def _load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def _profile_registry_semantic_errors(registry: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    profile_list = registry.get("profiles", [])
    profiles = {
        profile.get("profile_id"): profile
        for profile in profile_list
        if isinstance(profile, dict)
    }
    if len(profiles) != len(profile_list):
        errors.append("profile IDs must be unique")
        return errors

    for profile_id, profile in profiles.items():
        chain: list[dict[str, Any]] = []
        seen: list[str] = []
        cursor: str | None = profile_id
        invalid_chain = False
        while cursor is not None:
            if cursor in seen:
                cycle = " -> ".join([*seen[seen.index(cursor) :], cursor])
                errors.append(f"profile inheritance cycle: {cycle}")
                invalid_chain = True
                break
            seen.append(cursor)
            current = profiles.get(cursor)
            if current is None:
                errors.append(
                    f"profile {profile_id} references unknown parent: {cursor}"
                )
                invalid_chain = True
                break
            chain.append(current)
            cursor = current.get("extends_profile_id")
        if invalid_chain:
            continue
        chain.reverse()

        inherited_extensions = {
            item
            for ancestor in chain[:-1]
            for item in ancestor.get("assurance_extensions", [])
        }
        inherited_capabilities = {
            item
            for ancestor in chain[:-1]
            for item in ancestor.get("required_capabilities", [])
        }
        direct_extensions = set(profile.get("assurance_extensions", []))
        direct_capabilities = set(profile.get("required_capabilities", []))
        duplicate_extensions = sorted(inherited_extensions & direct_extensions)
        duplicate_capabilities = sorted(inherited_capabilities & direct_capabilities)
        if duplicate_extensions:
            errors.append(
                f"profile {profile_id} redeclares inherited extensions: "
                f"{duplicate_extensions}"
            )
        if duplicate_capabilities:
            errors.append(
                f"profile {profile_id} redeclares inherited capabilities: "
                f"{duplicate_capabilities}"
            )
    return errors


def _validator_registry_semantic_errors(
    registry: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    validators = registry.get("validators", [])
    identifiers = [item.get("validator_id") for item in validators]
    if len(identifiers) != len(set(identifiers)):
        errors.append("validator registry IDs must be unique")
    required = {
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
    missing = sorted(set(required) - set(identifiers))
    if missing:
        errors.append(f"validator registry missing required validators: {missing}")
    for validator in validators:
        implementation = validator.get("implementation")
        stage = validator.get("stage")
        schema_name = validator.get("schema_name")
        artifact_kinds = validator.get("artifact_kinds", [])
        if implementation == "json_schema":
            if stage not in {"design", "action"} or artifact_kinds:
                errors.append(
                    f"{validator.get('validator_id')} has invalid control applicability"
                )
            if not schema_name or not (ROOT / "assurance" / schema_name).is_file():
                errors.append(
                    f"{validator.get('validator_id')} references a missing schema"
                )
        elif implementation == "cross_artifact_comparability":
            if stage != "evidence" or artifact_kinds or schema_name is not None:
                errors.append(
                    f"{validator.get('validator_id')} has invalid evidence "
                    "applicability"
                )
        elif implementation in {
            "registered_artifact_schema",
            "finite_json",
            "statistical_reporting",
        }:
            if stage != "artifact" or not artifact_kinds or schema_name is not None:
                errors.append(
                    f"{validator.get('validator_id')} has invalid artifact applicability"
                )
        expected = required.get(validator.get("validator_id"))
        if expected is not None:
            observed = {
                key: validator.get(key)
                for key in expected
            }
            if observed != expected:
                errors.append(
                    f"{validator.get('validator_id')} weakens its base policy"
                )
    return errors


def _artifact_registry_semantic_errors(
    registry: dict[str, Any],
) -> list[str]:
    errors: list[str] = []
    records = registry.get("artifact_schemas", [])
    identifiers = [item.get("artifact_schema_id") for item in records]
    if len(identifiers) != len(set(identifiers)):
        errors.append("artifact schema registry IDs must be unique")
    schema_names = [item.get("schema_name") for item in records]
    if len(schema_names) != len(set(schema_names)):
        errors.append("artifact schema registry schema names must be unique")
    required = {
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
    missing = sorted(set(required) - set(identifiers))
    if missing:
        errors.append(f"artifact registry missing required entries: {missing}")
    for record in records:
        schema_name = record.get("schema_name")
        if not schema_name or not (ROOT / "assurance" / schema_name).is_file():
            errors.append(
                f"{record.get('artifact_schema_id')} references a missing schema"
            )
        expected = required.get(record.get("artifact_schema_id"))
        if expected is not None:
            observed = {
                key: record.get(key)
                for key in expected
            }
            if observed != expected:
                errors.append(
                    f"{record.get('artifact_schema_id')} weakens its base policy"
                )
    return errors


def _load_yaml(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return yaml.load(handle, Loader=UniqueKeyLoader)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _validate_instance(instance: Any, schema_path: Path, label: str) -> list[str]:
    validator = Draft202012Validator(
        _load_json(schema_path), format_checker=FormatChecker()
    )
    errors: list[str] = []
    for error in sorted(validator.iter_errors(instance), key=lambda item: list(item.absolute_path)):
        pointer = "/".join(map(str, error.absolute_path))
        errors.append(f"{label}#/{pointer}: {error.message}")
    return errors


def _safe_fixture_path(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if not value or "\\" in value or path.is_absolute() or any(
        part in {"", ".", ".."} for part in path.parts
    ):
        raise ValueError(f"unsafe fixture path: {value!r}")
    return path


def _json_pointer_value(document: Any, pointer: str) -> Any:
    if pointer == "":
        return document
    current = document
    for raw_token in pointer[1:].split("/"):
        token = raw_token.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            current = current[int(token)]
        else:
            current = current[token]
    return current


def _cross_artifact_fixture_errors(
    bundle: dict[str, Any],
    fixture_root: Path,
) -> list[str]:
    errors: list[str] = []
    artifacts = {
        item["record_id"]: item
        for item in bundle.get("artifacts", [])
    }
    documents = {
        artifact_id: _load_json(fixture_root / record["path"])
        for artifact_id, record in artifacts.items()
        if record.get("media_type") == "application/json"
        and (fixture_root / record["path"]).is_file()
    }
    sources = {
        item["record_id"]: item
        for item in bundle.get("sources", [])
    }
    action_record = sources.get(bundle.get("action_source_id"))
    if action_record is None:
        return ["cross-artifact fixture has no action source"]
    action = _load_json(fixture_root / action_record["path"])
    direct = [
        item
        for item in bundle.get("evidence", [])
        if item.get("evidence_class") == "direct_comparison"
    ]
    if len(direct) != 1:
        return ["cross-artifact fixture must have exactly one direct comparison"]
    evidence = direct[0]
    comparison = evidence["comparison"]
    left_id = comparison.get("left_artifact_id")
    right_id = comparison.get("right_artifact_id")
    if not left_id or not right_id or left_id == right_id:
        return ["fixture direct comparison must use two distinct artifacts"]
    if {left_id, right_id} - set(evidence.get("artifact_ids", [])):
        errors.append("fixture comparison artifacts are not declared as evidence")
    left_record = artifacts.get(left_id)
    right_record = artifacts.get(right_id)
    left = documents.get(left_id)
    right = documents.get(right_id)
    if (
        left_record is None
        or right_record is None
        or not isinstance(left, dict)
        or not isinstance(right, dict)
    ):
        return [*errors, "fixture cross-artifact documents are missing"]
    left_context = left.get("comparison_context", {})
    right_context = right.get("comparison_context", {})
    for field in (
        "task_id",
        "protocol_id",
        "metric_id",
        "population_id",
        "unit",
        "comparison_family_id",
        "varied_dimension",
    ):
        if left_context.get(field) != right_context.get(field):
            errors.append(f"fixture comparison context differs: {field}")
    for field in (
        "method",
        "equation",
        "initial_value",
        "final_time",
        "exact_final_value",
    ):
        if left.get(field) != right.get(field):
            errors.append(f"fixture artifact invariant differs: {field}")
    for document, context, record, label in (
        (left, left_context, left_record, "left"),
        (right, right_context, right_record, "right"),
    ):
        lineage = context.get("lineage", {})
        if lineage.get("producer_action_id") != record.get(
            "producer_action_id"
        ) or lineage.get("producer_action_id") != action.get("action_id"):
            errors.append(f"fixture {label} producer lineage mismatch")
        if set(lineage.get("source_ids", [])) != set(
            action.get("source_ids", [])
        ):
            errors.append(f"fixture {label} source lineage mismatch")
        condition = context.get("condition", {})
        if condition.get("dimension") != context.get("varied_dimension"):
            errors.append(f"fixture {label} condition dimension mismatch")
        try:
            observed = _json_pointer_value(
                document, condition["value_pointer"]
            )
        except Exception:
            errors.append(f"fixture {label} condition pointer is invalid")
        else:
            if observed != condition.get("value"):
                errors.append(f"fixture {label} condition value mismatch")
    if left_context.get("lineage", {}).get(
        "transformation_id"
    ) != right_context.get("lineage", {}).get("transformation_id"):
        errors.append("fixture transformation lineage differs")
    if left_context.get("condition", {}).get(
        "value"
    ) == right_context.get("condition", {}).get("value"):
        errors.append("fixture comparison condition does not vary")
    try:
        left_value = _json_pointer_value(left, comparison["left_pointer"])
        right_value = _json_pointer_value(right, comparison["right_pointer"])
    except Exception:
        errors.append("fixture evidence comparison pointer is invalid")
    else:
        if comparison.get("operator") != "lt" or not left_value < right_value:
            errors.append("fixture direct comparison does not evaluate true")
    return errors


def _disposable_reproduction_fixture_errors(
    manifest: dict[str, Any],
    bundle: dict[str, Any],
    fixture_root: Path,
) -> list[str]:
    errors: list[str] = []
    records = {
        item["record_id"]: item
        for item in [*bundle.get("sources", []), *bundle.get("artifacts", [])]
        if isinstance(item, dict)
    }
    bundle_record = manifest.get("source_bundle", {})
    if bundle_record.get("path") != "review-bundle.json":
        errors.append("disposable reproduction must bind review-bundle.json")
    bundle_path = fixture_root / "review-bundle.json"
    if bundle_path.is_file() and bundle_record.get("sha256") != _sha256(bundle_path):
        errors.append("disposable reproduction source bundle digest mismatch")

    for record in manifest.get("input_snapshot", []):
        record_id = record.get("record_id")
        source_record = records.get(record_id)
        if source_record is None:
            errors.append(f"disposable reproduction input is unknown: {record_id}")
            continue
        if record.get("path") != source_record.get("path"):
            errors.append(
                f"disposable reproduction input path mismatch: {record_id}"
            )
        try:
            file_path = fixture_root / Path(*_safe_fixture_path(record["path"]).parts)
        except Exception as exc:
            errors.append(f"invalid disposable reproduction input path: {exc}")
            continue
        if not file_path.is_file():
            errors.append(f"disposable reproduction input missing: {record_id}")
        elif record.get("sha256") != _sha256(file_path):
            errors.append(
                f"disposable reproduction input digest mismatch: {record_id}"
            )

    action = manifest.get("allowed_action", {})
    if action.get("implementation") != "explicit_euler_decay_replay_v0.1":
        errors.append("disposable reproduction implementation changed")
    action_source_id = bundle.get("action_source_id")
    action_record = records.get(action_source_id)
    if not action_record:
        errors.append("disposable reproduction cannot resolve source action")
    else:
        source_action = _load_json(fixture_root / action_record["path"])
        if source_action.get("action_id") != action.get("source_action_id"):
            errors.append("disposable reproduction source action mismatch")
        if source_action.get("state") != "completed":
            errors.append("disposable reproduction source action is not completed")

    output_ids = [item.get("output_id") for item in manifest.get("outputs", [])]
    if len(output_ids) != len(set(output_ids)):
        errors.append("disposable reproduction output IDs are not unique")
    output_paths = [item.get("path") for item in manifest.get("outputs", [])]
    if len(output_paths) != len(set(output_paths)):
        errors.append("disposable reproduction output paths are not unique")
    expected_pointers = [
        "/run/step_size",
        "/run/step_count",
        "/run/final_value",
        "/run/final_absolute_error",
    ]
    for output in manifest.get("outputs", []):
        if output.get("value_pointers") != expected_pointers:
            errors.append(
                f"disposable reproduction output pointer set changed: "
                f"{output.get('output_id')}"
            )
        artifact = records.get(output.get("original_artifact_id"))
        if not artifact:
            errors.append(
                f"disposable reproduction output references unknown artifact: "
                f"{output.get('output_id')}"
            )
            continue
        if artifact.get("artifact_schema_id") != output.get("artifact_schema_id"):
            errors.append(
                f"disposable reproduction output schema mismatch: "
                f"{output.get('output_id')}"
            )
        artifact_document = _load_json(fixture_root / artifact["path"])
        try:
            observed_step = _json_pointer_value(artifact_document, "/run/step_size")
        except Exception:
            errors.append(
                f"disposable reproduction cannot read step size: "
                f"{output.get('output_id')}"
            )
        else:
            if observed_step != output.get("step_size"):
                errors.append(
                    f"disposable reproduction step size mismatch: "
                    f"{output.get('output_id')}"
                )
    safety = manifest.get("safety", {})
    for field in (
        "source_write_allowed",
        "model_invocation_allowed",
        "network_allowed",
        "child_process_allowed",
        "external_code_execution_allowed",
    ):
        if safety.get(field) is not False:
            errors.append(f"disposable reproduction safety field changed: {field}")
    boundary = manifest.get("evidence_boundary", {})
    if boundary.get("independence_effect") != "no_new_independent_evidence":
        errors.append("disposable reproduction independence boundary changed")
    if boundary.get("claim_strength_effect") != "no_claim_promotion":
        errors.append("disposable reproduction claim boundary changed")
    return errors


def _source_visibility_fixture_errors(ledger: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    source_ids = [source.get("source_id") for source in ledger.get("sources", [])]
    if len(source_ids) != len(set(source_ids)):
        errors.append("source visibility fixture has duplicate source IDs")
    references = ledger.get("references", [])
    reference_ids = [reference.get("ref_id") for reference in references]
    if len(reference_ids) != len(set(reference_ids)):
        errors.append("source visibility fixture has duplicate reference IDs")
    source_id_set = set(source_ids)
    for reference in references:
        if reference.get("source_id") not in source_id_set:
            errors.append(
                f"source visibility reference points at unknown source: "
                f"{reference.get('ref_id')}"
            )
    registered_refs = set(reference_ids)
    for ref_id in ledger.get("gate_request", {}).get("cited_ref_ids", []):
        if ref_id not in registered_refs:
            errors.append(f"source visibility gate cites unknown ref: {ref_id}")
    policy = ledger.get("retrieval_policy", {})
    if policy.get("fulltext_required_for_high_claims") is not True:
        errors.append("source visibility fixture must require full text for high claims")
    if policy.get("allow_incremental_retrieval") is not True:
        errors.append("source visibility fixture must allow incremental retrieval")
    fulltext_count = sum(
        1
        for source in ledger.get("sources", [])
        if source.get("visibility_status") == "full_text_observed"
    )
    if fulltext_count > policy.get("max_fulltext_sources_per_pass", -1):
        errors.append("source visibility fixture exceeds full-text source budget")
    token_estimate = sum(
        attempt.get("context_tokens_estimate", 0)
        for source in ledger.get("sources", [])
        for attempt in source.get("retrieval_attempts", [])
    )
    if token_estimate > policy.get("max_context_tokens_per_pass", -1):
        errors.append("source visibility fixture exceeds context budget")
    return errors


def _check_markdown_links() -> list[str]:
    errors: list[str] = []
    pattern = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    for path in sorted(ROOT.rglob("*.md")):
        relative_path = path.relative_to(ROOT)
        if any(part in NON_REPOSITORY_PARTS for part in relative_path.parts):
            continue
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in pattern.finditer(line):
                target = match.group(1).split("#", 1)[0]
                if not target or re.match(r"^(?:https?://|mailto:)", target):
                    continue
                resolved = (path.parent / target).resolve()
                if not resolved.exists():
                    errors.append(
                        f"{path.relative_to(ROOT)}:{line_number}: broken local link {target}"
                    )
    return errors


def check_repository() -> dict[str, Any]:
    errors: list[str] = []
    counts: dict[str, int] = {}

    schema_paths = sorted(ROOT.rglob("*.schema.json"))
    counts["schemas"] = len(schema_paths)
    for schema_path in schema_paths:
        try:
            Draft202012Validator.check_schema(_load_json(schema_path))
        except Exception as exc:
            errors.append(f"invalid schema {schema_path.relative_to(ROOT)}: {exc}")

    assurance_root = ROOT / "assurance"
    assurance_fixture_root = assurance_root / "fixtures/p0"
    assurance_positive_contracts = {
        assurance_root / "profile-registry-v0.1.json": (
            assurance_root / "assurance-profile-registry-v0.1.schema.json"
        ),
        assurance_root / "retention-policy-v0.1.json": (
            assurance_root / "retention-policy-v0.1.schema.json"
        ),
        assurance_fixture_root / "effective-security-envelope.valid.json": (
            assurance_root / "effective-security-envelope-v0.1.schema.json"
        ),
        assurance_fixture_root / "sandbox-selection-receipt.valid.json": (
            assurance_root / "sandbox-selection-receipt-v0.1.schema.json"
        ),
        assurance_fixture_root / "session-lifecycle-receipt.valid.json": (
            assurance_root / "session-lifecycle-receipt-v0.1.schema.json"
        ),
    }
    for instance_path, schema_path in assurance_positive_contracts.items():
        errors.extend(
            _validate_instance(
                _load_json(instance_path),
                schema_path,
                str(instance_path.relative_to(ROOT)),
            )
        )
    counts["assurance_p0_positive_contracts"] = len(assurance_positive_contracts)

    assurance_negative_contracts = {
        assurance_fixture_root
        / "effective-security-envelope.missing-evidence-status.invalid.json": (
            assurance_root / "effective-security-envelope-v0.1.schema.json"
        ),
        assurance_fixture_root / "profile-registry.required-runtime.invalid.json": (
            assurance_root / "assurance-profile-registry-v0.1.schema.json"
        ),
        assurance_fixture_root
        / "sandbox-selection-receipt.unverified-allow.invalid.json": (
            assurance_root / "sandbox-selection-receipt-v0.1.schema.json"
        ),
        assurance_fixture_root
        / "session-lifecycle-receipt.archived-with-residue.invalid.json": (
            assurance_root / "session-lifecycle-receipt-v0.1.schema.json"
        ),
        assurance_fixture_root
        / "retention-policy.private-reasoning-persisted.invalid.json": (
            assurance_root / "retention-policy-v0.1.schema.json"
        ),
    }
    for instance_path, schema_path in assurance_negative_contracts.items():
        validator = Draft202012Validator(
            _load_json(schema_path), format_checker=FormatChecker()
        )
        validation_errors = list(validator.iter_errors(_load_json(instance_path)))
        if not validation_errors:
            errors.append(
                "negative assurance fixture unexpectedly validated: "
                f"{instance_path.relative_to(ROOT)}"
            )
    counts["assurance_p0_negative_contracts"] = len(assurance_negative_contracts)

    assurance_p1_fixture_root = assurance_root / "fixtures/p1"
    assurance_p1_positive_contracts = {
        assurance_p1_fixture_root / "installation-key-metadata.valid.json": (
            assurance_root / "installation-key-metadata-v0.1.schema.json"
        ),
        assurance_p1_fixture_root / "conversation-state.active.valid.json": (
            assurance_root / "conversation-state-v0.1.schema.json"
        ),
        assurance_p1_fixture_root / "archive-deletion-receipt.failed.valid.json": (
            assurance_root / "archive-deletion-receipt-v0.1.schema.json"
        ),
    }
    for instance_path, schema_path in assurance_p1_positive_contracts.items():
        errors.extend(
            _validate_instance(
                _load_json(instance_path),
                schema_path,
                str(instance_path.relative_to(ROOT)),
            )
        )
    counts["assurance_p1_positive_contracts"] = len(
        assurance_p1_positive_contracts
    )

    assurance_p1_negative_contracts = {
        assurance_p1_fixture_root
        / "installation-key-metadata.raw-secret.invalid.json": (
            assurance_root / "installation-key-metadata-v0.1.schema.json"
        ),
        assurance_p1_fixture_root
        / "conversation-state.archived-without-receipt.invalid.json": (
            assurance_root / "conversation-state-v0.1.schema.json"
        ),
        assurance_p1_fixture_root
        / "archive-deletion-receipt.success-with-residue.invalid.json": (
            assurance_root / "archive-deletion-receipt-v0.1.schema.json"
        ),
    }
    for instance_path, schema_path in assurance_p1_negative_contracts.items():
        validator = Draft202012Validator(
            _load_json(schema_path), format_checker=FormatChecker()
        )
        validation_errors = list(validator.iter_errors(_load_json(instance_path)))
        if not validation_errors:
            errors.append(
                "negative assurance P1 fixture unexpectedly validated: "
                f"{instance_path.relative_to(ROOT)}"
            )
    counts["assurance_p1_negative_contracts"] = len(
        assurance_p1_negative_contracts
    )

    docker_profile_path = assurance_root / "docker-sandbox-profile-v0.1.json"
    errors.extend(
        _validate_instance(
            _load_json(docker_profile_path),
            assurance_root / "docker-sandbox-profile-v0.1.schema.json",
            str(docker_profile_path.relative_to(ROOT)),
        )
    )
    counts["assurance_p2_profiles"] = 1

    retention_policy = _load_json(assurance_root / "retention-policy-v0.1.json")
    expected_delete_categories = {
        "active_security_envelope",
        "confirmation_token",
        "credential_lease",
        "full_stdout_stderr",
        "network_body",
        "one_shot_permit",
        "private_reasoning",
        "raw_provider_payload",
        "raw_tool_result",
        "sandbox_ephemeral_storage",
        "session_recall_index",
        "temporary_checkpoint",
        "temporary_import_receipt",
        "temporary_lifecycle_receipt",
        "temporary_profile",
        "unpinned_snapshot",
    }
    actual_delete_categories = set(retention_policy.get("delete_on_archive", []))
    missing_delete_categories = sorted(
        expected_delete_categories - actual_delete_categories
    )
    if missing_delete_categories:
        errors.append(
            "assurance retention policy lost mandatory delete categories: "
            f"{missing_delete_categories}"
        )

    profile_registry = _load_json(assurance_root / "profile-registry-v0.1.json")
    profile_semantic_errors = _profile_registry_semantic_errors(profile_registry)
    errors.extend(
        f"assurance profile registry semantic error: {error}"
        for error in profile_semantic_errors
    )
    semantic_invalid_path = (
        assurance_fixture_root
        / "profile-registry.inheritance-cycle.semantic-invalid.json"
    )
    semantic_invalid_registry = _load_json(semantic_invalid_path)
    semantic_invalid_schema_errors = _validate_instance(
        semantic_invalid_registry,
        assurance_root / "assurance-profile-registry-v0.1.schema.json",
        str(semantic_invalid_path.relative_to(ROOT)),
    )
    if semantic_invalid_schema_errors:
        errors.append(
            "profile inheritance-cycle fixture must remain structurally valid: "
            f"{semantic_invalid_schema_errors}"
        )
    if not _profile_registry_semantic_errors(semantic_invalid_registry):
        errors.append(
            "profile inheritance-cycle fixture unexpectedly passed semantic validation"
        )
    counts["assurance_p0_semantic_negative_contracts"] = 1

    general_science_fixture_root = (
        assurance_root / "fixtures/general_science/computational_decay"
    )
    general_science_bundle_path = (
        general_science_fixture_root / "review-bundle.json"
    )
    general_science_bundle = _load_json(general_science_bundle_path)
    errors.extend(
        _validate_instance(
            general_science_bundle,
            assurance_root / "general-science-review-bundle-v0.1.schema.json",
            str(general_science_bundle_path.relative_to(ROOT)),
        )
    )
    general_science_records = [
        *general_science_bundle.get("sources", []),
        *general_science_bundle.get("artifacts", []),
    ]
    record_ids = [item.get("record_id") for item in general_science_records]
    if len(record_ids) != len(set(record_ids)):
        errors.append("general-science fixture record IDs are not unique")
    record_paths = [item.get("path") for item in general_science_records]
    if len(record_paths) != len(set(record_paths)):
        errors.append("general-science fixture paths are not unique")
    for record in general_science_records:
        try:
            relative = _safe_fixture_path(record["path"])
            file_path = general_science_fixture_root / Path(*relative.parts)
            if not file_path.is_file():
                errors.append(
                    "general-science fixture file is missing: "
                    f"{file_path.relative_to(ROOT)}"
                )
            elif _sha256(file_path) != record["sha256"]:
                errors.append(
                    "general-science fixture digest mismatch: "
                    f"{file_path.relative_to(ROOT)}"
                )
        except Exception as exc:
            errors.append(f"invalid general-science fixture record: {exc}")
    counts["general_science_review_fixtures"] = 1

    validator_registry_path = (
        assurance_root / "general-science-validator-registry-v0.1.json"
    )
    validator_registry = _load_json(validator_registry_path)
    errors.extend(
        _validate_instance(
            validator_registry,
            assurance_root
            / "general-science-validator-registry-v0.1.schema.json",
            str(validator_registry_path.relative_to(ROOT)),
        )
    )
    errors.extend(
        f"general-science validator registry semantic error: {error}"
        for error in _validator_registry_semantic_errors(validator_registry)
    )
    for filename, schema_name in (
        ("design.json", "general-science-study-design-v0.1.schema.json"),
        (
            "run-manifest.json",
            "general-science-action-manifest-v0.1.schema.json",
        ),
    ):
        fixture_path = general_science_fixture_root / filename
        errors.extend(
            _validate_instance(
                _load_json(fixture_path),
                assurance_root / schema_name,
                str(fixture_path.relative_to(ROOT)),
            )
        )
    invalid_validator_registry_path = (
        assurance_root
        / "fixtures/general_science/"
        "validator-registry.duplicate-id.semantic-invalid.json"
    )
    invalid_validator_registry = _load_json(invalid_validator_registry_path)
    structural_errors = _validate_instance(
        invalid_validator_registry,
        assurance_root / "general-science-validator-registry-v0.1.schema.json",
        str(invalid_validator_registry_path.relative_to(ROOT)),
    )
    if structural_errors:
        errors.append(
            "duplicate-validator fixture must remain structurally valid: "
            f"{structural_errors}"
        )
    if not _validator_registry_semantic_errors(invalid_validator_registry):
        errors.append(
            "duplicate-validator fixture unexpectedly passed semantic validation"
        )
    counts["general_science_validators"] = len(
        validator_registry.get("validators", [])
    )
    counts["general_science_validator_semantic_negatives"] = 1

    artifact_registry_path = (
        assurance_root / "general-science-artifact-registry-v0.1.json"
    )
    artifact_registry = _load_json(artifact_registry_path)
    errors.extend(
        _validate_instance(
            artifact_registry,
            assurance_root
            / "general-science-artifact-registry-v0.1.schema.json",
            str(artifact_registry_path.relative_to(ROOT)),
        )
    )
    errors.extend(
        f"general-science artifact registry semantic error: {error}"
        for error in _artifact_registry_semantic_errors(artifact_registry)
    )
    artifact_schemas = {
        item["artifact_schema_id"]: item
        for item in artifact_registry.get("artifact_schemas", [])
    }
    for artifact in general_science_bundle.get("artifacts", []):
        registration = artifact_schemas.get(artifact.get("artifact_schema_id"))
        if registration is None:
            errors.append(
                "general-science fixture uses an unknown artifact schema: "
                f"{artifact.get('artifact_schema_id')}"
            )
            continue
        if artifact.get("artifact_kind") != registration["artifact_kind"]:
            errors.append(
                f"{artifact['record_id']} kind disagrees with artifact registry"
            )
        if artifact.get("media_type") != registration["media_type"]:
            errors.append(
                f"{artifact['record_id']} media type disagrees with artifact registry"
            )
        artifact_path = general_science_fixture_root / Path(
            *_safe_fixture_path(artifact["path"]).parts
        )
        if artifact_path.is_file():
            errors.extend(
                _validate_instance(
                    _load_json(artifact_path),
                    assurance_root / registration["schema_name"],
                    str(artifact_path.relative_to(ROOT)),
                )
            )
    invalid_artifact_registry_path = (
        assurance_root
        / "fixtures/general_science/"
        "artifact-registry.duplicate-id.semantic-invalid.json"
    )
    invalid_artifact_registry = _load_json(invalid_artifact_registry_path)
    artifact_structural_errors = _validate_instance(
        invalid_artifact_registry,
        assurance_root / "general-science-artifact-registry-v0.1.schema.json",
        str(invalid_artifact_registry_path.relative_to(ROOT)),
    )
    if artifact_structural_errors:
        errors.append(
            "duplicate-artifact-schema fixture must remain structurally valid: "
            f"{artifact_structural_errors}"
        )
    if not _artifact_registry_semantic_errors(invalid_artifact_registry):
        errors.append(
            "duplicate-artifact-schema fixture unexpectedly passed semantic "
            "validation"
        )
    counts["general_science_artifact_schemas"] = len(artifact_schemas)
    counts["general_science_artifact_registry_semantic_negatives"] = 1
    errors.extend(
        f"general-science cross-artifact fixture error: {error}"
        for error in _cross_artifact_fixture_errors(
            general_science_bundle,
            general_science_fixture_root,
        )
    )
    counts["general_science_cross_artifact_comparisons"] = 1

    reproduction_manifest_path = (
        general_science_fixture_root / "disposable-reproduction-manifest.json"
    )
    reproduction_manifest = _load_json(reproduction_manifest_path)
    errors.extend(
        _validate_instance(
            reproduction_manifest,
            assurance_root / "disposable-reproduction-manifest-v0.1.schema.json",
            str(reproduction_manifest_path.relative_to(ROOT)),
        )
    )
    errors.extend(
        f"general-science disposable reproduction fixture error: {error}"
        for error in _disposable_reproduction_fixture_errors(
            reproduction_manifest,
            general_science_bundle,
            general_science_fixture_root,
        )
    )
    counts["general_science_disposable_reproduction_fixtures"] = 1

    source_visibility_fixture_path = (
        assurance_root / "fixtures/source_visibility/mixed-visibility-ledger.json"
    )
    source_visibility_ledger = _load_json(source_visibility_fixture_path)
    errors.extend(
        _validate_instance(
            source_visibility_ledger,
            assurance_root / "source-visibility-ledger-v0.1.schema.json",
            str(source_visibility_fixture_path.relative_to(ROOT)),
        )
    )
    errors.extend(
        f"source visibility fixture error: {error}"
        for error in _source_visibility_fixture_errors(source_visibility_ledger)
    )
    counts["source_visibility_fixtures"] = 1

    for required_path in (
        ROOT / "gsa.py",
        assurance_root / "cli.py",
        assurance_root / "task-contract-v0.1.schema.json",
        assurance_root / "task_contract.py",
        assurance_root / "canonical-cli-answer-packet-v0.1.schema.json",
        assurance_root / "canonical-cli-run-receipt-v0.1.schema.json",
        assurance_root / "canonical_cli.py",
        assurance_root / "canonical_cli_main.py",
        assurance_root / "tests/test_cli_dispatcher.py",
        assurance_root / "tests/test_canonical_cli.py",
        ROOT / "docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md",
    ):
        if not required_path.is_file():
            errors.append(
                f"missing canonical CLI P0 file: {required_path.relative_to(ROOT)}"
            )
    counts["canonical_cli_fixtures"] = 1

    for required_path in (
        assurance_root / "orientation-checkpoint-v0.1.schema.json",
        assurance_root / "orientation-checkpoint-verification-v0.1.schema.json",
        assurance_root / "orientation-stagnation-integration-fixture-v0.1.schema.json",
        assurance_root / "orientation-stagnation-integration-receipt-v0.1.schema.json",
        assurance_root / "orientation-stagnation-journal-receipt-v0.1.schema.json",
        assurance_root / "orientation-checkpoint-event-payload-v0.1.schema.json",
        assurance_root / "runtime-stagnation-guard-event-payload-v0.1.schema.json",
        assurance_root / "runner-public-output-stream-v0.1.schema.json",
        assurance_root / "runner-public-output-extraction-receipt-v0.1.schema.json",
        assurance_root / "deepseek-api-observation-result-v0.1.schema.json",
        assurance_root / "deepseek-api-observation-pipeline-receipt-v0.1.schema.json",
        assurance_root / "deepseek-stream-observation-fixture-v0.1.schema.json",
        assurance_root / "deepseek-stream-observation-pipeline-receipt-v0.1.schema.json",
        assurance_root / "runtime-stagnation-guard-receipt-v0.1.schema.json",
        assurance_root / "deepseek_api_observation.py",
        assurance_root / "deepseek_stream_observation.py",
        assurance_root / "orientation_runtime_integration.py",
        assurance_root / "orientation_runtime_guard.py",
        assurance_root / "orientation_runtime_journal.py",
        assurance_root / "runner_public_output.py",
        assurance_root / "tests/test_deepseek_api_observation.py",
        assurance_root / "tests/test_deepseek_stream_observation.py",
        assurance_root / "tests/test_orientation_runtime_integration.py",
        assurance_root / "tests/test_orientation_runtime_guard.py",
        assurance_root / "tests/test_orientation_runtime_journal.py",
        assurance_root / "tests/test_runner_public_output.py",
        ROOT / "scripts/build_deepseek_public_output_observation.py",
        ROOT / "scripts/build_deepseek_stream_observation_fixture.py",
        ROOT / "scripts/invoke_deepseek_public_output_observation.ps1",
        ROOT / "docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md",
        ROOT / "runtime/run-event-v0.1.schema.json",
    ):
        if not required_path.is_file():
            errors.append(
                "missing orientation/stagnation guard file: "
                f"{required_path.relative_to(ROOT)}"
            )
    counts["orientation_stagnation_guard_fixtures"] = 1
    deepseek_observation_launcher = (
        ROOT / "scripts/invoke_deepseek_public_output_observation.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "billable_external_request = $true",
        "credential_value_recorded = $false",
        "raw_response_recorded = $false",
        "retry_budget = 0",
        "Assert-NoCommonSecretPattern",
        "CredRead(target, CRED_TYPE_GENERIC",
        "LIF_DEEPSEEK_PUBLIC_OUTPUT_OBSERVATION_OK",
    ):
        if marker not in deepseek_observation_launcher:
            errors.append(f"DeepSeek API observation launcher is missing marker: {marker}")
    deepseek_stream_source = (
        assurance_root / "deepseek_stream_observation.py"
    ).read_text(encoding="utf-8")
    for marker in (
        "deepseek-chat-completions-stream",
        "reasoning_content_sha256",
        "private_reasoning_text_recorded",
        "run_deepseek_stream_observation_fixture",
        "restart_packet_source_policy",
    ):
        if marker not in deepseek_stream_source:
            errors.append(f"DeepSeek stream fixture is missing marker: {marker}")
    counts["deepseek_api_observation_fixtures"] = 1
    counts["deepseek_stream_observation_fixtures"] = 1

    profiles = profile_registry.get("profiles", [])
    profile_ids = [profile.get("profile_id") for profile in profiles]
    if len(profile_ids) != len(set(profile_ids)):
        errors.append("assurance profile IDs are not unique")
    general_science_profiles = [
        profile
        for profile in profiles
        if profile.get("profile_id") == "general-science"
    ]
    lif_profiles = [
        profile for profile in profiles if profile.get("profile_id") == "lif-research"
    ]
    if len(general_science_profiles) != 1:
        errors.append(
            "assurance registry must contain exactly one general-science profile"
        )
    if len(lif_profiles) != 1:
        errors.append("assurance registry must contain exactly one lif-research profile")
    if len(general_science_profiles) == 1 and len(lif_profiles) == 1:
        general_science_profile = general_science_profiles[0]
        lif_profile = lif_profiles[0]
        if general_science_profile.get("extends_profile_id") is not None:
            errors.append("general-science must be a domain-neutral root profile")
        if lif_profile.get("extends_profile_id") != "general-science":
            errors.append("lif-research must extend general-science")
        general_science_text = json.dumps(
            general_science_profile, sort_keys=True
        ).lower()
        for forbidden in (
            "lif-",
            "lif_",
            "lif ",
            " fep",
            "r211",
            "current_index",
            "map6",
            "index/map/r",
        ):
            if forbidden in general_science_text:
                errors.append(
                    "general-science contains project-specific token: "
                    f"{forbidden}"
                )
        required_general_extensions = {
            "ClaimBoundary",
            "EvidenceKernel",
            "EvaluationRunner",
            "LeakScanner",
            "ResearchLifecycle",
            "ScenarioExporter",
            "SourceRouter",
            "ValidatorBridge",
        }
        if set(general_science_profile.get("assurance_extensions", [])) != (
            required_general_extensions
        ):
            errors.append(
                "general-science assurance extensions do not match the frozen contract"
            )
        expected_lif_extensions = {
            "LifCurrentSourceRouting",
            "LifValidatorProfile",
        }
        if set(lif_profile.get("assurance_extensions", [])) != (
            expected_lif_extensions
        ):
            errors.append("lif-research must contain only the frozen LIF delta")
        if required_general_extensions.intersection(
            lif_profile.get("assurance_extensions", [])
        ):
            errors.append("lif-research redeclares general-science extensions")
        if "required_runtime_family" in lif_profile:
            errors.append("lif-research profile must not bind a required runtime family")
        # grok-build may be declared on general-science (inherited by lif-research)
        # or on lif-research directly — check both
        grok_in_general = [
            reference
            for reference in general_science_profile.get("reference_runtimes", [])
            if reference.get("runtime_family") == "grok-build"
        ]
        grok_in_lif = [
            reference
            for reference in lif_profile.get("reference_runtimes", [])
            if reference.get("runtime_family") == "grok-build"
        ]
        grok_references = grok_in_general + grok_in_lif
        if len(grok_references) < 1 or not all(
            r.get("role") == "reference_only" for r in grok_references
        ):
            errors.append(
                "Grok Build must be represented at least once as reference_only "
                "(on general-science, lif-research, or inherited)"
            )
        if lif_profile.get("acceptance_gate", {}).get(
            "reference_status_grants_acceptance"
        ) is not False:
            errors.append("reference runtime status must not grant profile acceptance")

    corpus_path = ROOT / "regression/cases-v0.1.yaml"
    coverage_path = ROOT / "regression/coverage-matrix-v0.1.yaml"
    reason_path = ROOT / "protocol/reason-codes-v0.1.yaml"
    gate_path = ROOT / "protocol/gate-matrix-v0.1.yaml"
    progress_reason_migration_path = (
        ROOT / "protocol/global-progress-reason-code-migration-v0.1.yaml"
    )
    try:
        corpus = _load_yaml(corpus_path)
        coverage = _load_yaml(coverage_path)
        reasons = _load_yaml(reason_path)
        gates = _load_yaml(gate_path)
        progress_reason_migration = _load_yaml(progress_reason_migration_path)
    except Exception as exc:
        errors.append(f"YAML load failed: {exc}")
        corpus = coverage = reasons = gates = progress_reason_migration = {}

    if corpus:
        errors.extend(
            _validate_instance(
                corpus,
                ROOT / "regression/case-corpus-v0.1.schema.json",
                "regression/cases-v0.1.yaml",
            )
        )
    if coverage:
        errors.extend(
            _validate_instance(
                coverage,
                ROOT / "regression/coverage-matrix-v0.1.schema.json",
                "regression/coverage-matrix-v0.1.yaml",
            )
        )

    upstream_lock_path = ROOT / "upstream/grok-build.lock.json"
    upstream_lock = _load_json(upstream_lock_path)
    errors.extend(
        _validate_instance(
            upstream_lock,
            ROOT / "upstream/grok-build-lock-v0.1.schema.json",
            "upstream/grok-build.lock.json",
        )
    )
    ownership = upstream_lock.get("ownership", {})
    ownership_sets = {
        name: set(values) for name, values in ownership.items() if isinstance(values, list)
    }
    ownership_names = sorted(ownership_sets)
    for left_index, left_name in enumerate(ownership_names):
        for right_name in ownership_names[left_index + 1 :]:
            overlap = sorted(ownership_sets[left_name] & ownership_sets[right_name])
            if overlap:
                errors.append(
                    f"upstream ownership overlap {left_name}/{right_name}: {overlap}"
                )
    counts["upstream_locks"] = 1

    upstream_candidate_path = ROOT / "upstream/grok-build.candidate.json"
    upstream_candidate = _load_json(upstream_candidate_path)
    errors.extend(
        _validate_instance(
            upstream_candidate,
            ROOT / "upstream/grok-build-candidate-v0.1.schema.json",
            "upstream/grok-build.candidate.json",
        )
    )
    candidate_baseline = upstream_candidate.get("baseline", {})
    candidate_discovery = upstream_candidate.get("discovery", {})
    candidate_release = upstream_candidate.get("binary_release", {})
    candidate_promotion = upstream_candidate.get("promotion", {})
    lock_release = upstream_lock.get("binary_release", {})
    candidate_selected = candidate_promotion.get("selected_as_default") is True
    if candidate_selected:
        if candidate_release.get("version") != lock_release.get("version"):
            errors.append("selected upstream candidate version does not match default lock")
        if candidate_release.get("sha256") != lock_release.get("sha256"):
            errors.append(
                "selected upstream candidate SHA-256 does not match default lock"
            )
    else:
        if candidate_baseline.get("version") != lock_release.get("version"):
            errors.append(
                "unselected upstream candidate baseline version does not match default lock"
            )
        if candidate_baseline.get("sha256") != lock_release.get("sha256"):
            errors.append(
                "unselected upstream candidate baseline SHA-256 does not match default lock"
            )
    if candidate_discovery.get("channel_pointer_version") != candidate_release.get(
        "version"
    ):
        errors.append("upstream candidate binary version does not match stable pointer")
    try:
        baseline_version = tuple(
            int(part) for part in candidate_baseline["version"].split(".")
        )
        candidate_version = tuple(
            int(part) for part in candidate_release["version"].split(".")
        )
        if candidate_version <= baseline_version:
            errors.append("upstream candidate version must be newer than observed baseline")
    except (KeyError, TypeError, ValueError):
        errors.append("upstream candidate versions are not comparable numeric triples")
    expected_candidate_gates = {
        "binary_identity",
        "acp_initialize",
        "fake_tool_allow",
        "fake_tool_cancel",
        "windows_child_tree_timeout",
        "deepseek_reasoning_continuity",
        "repository_regression",
    }
    candidate_gate_ids = [
        gate.get("id")
        for gate in candidate_promotion.get("gates", [])
        if isinstance(gate, dict)
    ]
    if len(candidate_gate_ids) != len(set(candidate_gate_ids)):
        errors.append("upstream candidate promotion gates contain duplicate IDs")
    if set(candidate_gate_ids) != expected_candidate_gates:
        errors.append("upstream candidate promotion gate set is incomplete")
    if (
        candidate_promotion.get("status") != "eligible"
        and candidate_selected
    ):
        errors.append("non-eligible upstream candidate cannot be selected as default")
    if candidate_selected and any(
        gate.get("status") != "passed"
        for gate in candidate_promotion.get("gates", [])
        if isinstance(gate, dict)
    ):
        errors.append("selected upstream candidate has an unpassed promotion gate")
    counts["upstream_candidates"] = 1

    grok_config_path = ROOT / "integration/grok/deepseek-custom-model.example.toml"
    with grok_config_path.open("rb") as handle:
        grok_config = tomllib.load(handle)
    model_config = grok_config.get("model", {}).get("lif-deepseek-v4-pro", {})
    expected_model_config = {
        "model": "deepseek-v4-pro",
        "base_url": "https://api.deepseek.com",
        "env_key": "LIF_DEEPSEEK_API_KEY",
        "api_backend": "chat_completions",
    }
    for key, expected in expected_model_config.items():
        if model_config.get(key) != expected:
            errors.append(
                f"DeepSeek Grok config {key} must be {expected!r}, "
                f"got {model_config.get(key)!r}"
            )
    if "api_key" in model_config or "extra_headers" in model_config:
        errors.append("DeepSeek Grok config must not contain embedded credentials")
    if grok_config.get("models", {}).get("default"):
        errors.append("discovery-only DeepSeek Grok config must not set a default model")
    counts["grok_config_fixtures"] = 1

    grok_plan_example = ROOT / "integration/grok/examples/example-grok-observed-plan.json"
    errors.extend(
        _validate_instance(
            _load_json(grok_plan_example),
            ROOT / "integration/grok/grok-observed-plan-v0.1.schema.json",
            "integration/grok/examples/example-grok-observed-plan.json",
        )
    )
    counts["grok_observed_plan_examples"] = 1

    trust_example_path = (
        ROOT / "integration/grok/examples/example-grok-workspace-trust-receipt.json"
    )
    errors.extend(
        _validate_instance(
            _load_json(trust_example_path),
            ROOT / "integration/grok/grok-workspace-trust-receipt-v0.1.schema.json",
            "integration/grok/examples/example-grok-workspace-trust-receipt.json",
        )
    )
    trust_script_source = (
        ROOT / "scripts/new_grok_workspace_trust_receipt.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "ExpectedAggregateSha256",
        "expires_on_control_change",
        "launch_permitted",
        "reparse-policy=record-and-do-not-follow",
        "refusing to overwrite",
    ):
        if marker not in trust_script_source:
            errors.append(f"workspace trust preflight is missing safety marker: {marker}")
    counts["grok_workspace_trust_examples"] = 1

    discovery_example_path = (
        ROOT / "integration/grok/examples/example-grok-workspace-discovery-result.json"
    )
    errors.extend(
        _validate_instance(
            _load_json(discovery_example_path),
            ROOT / "integration/grok/grok-workspace-discovery-result-v0.1.schema.json",
            "integration/grok/examples/example-grok-workspace-discovery-result.json",
        )
    )
    discovery_script_source = (
        ROOT / "scripts/invoke_grok_workspace_discovery_probe.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "inspect_trust_flag_is_non_mutating",
        "clean_child_environment",
        "fail_closed_proxy_configured",
        "debug_capture_disabled",
        "refusing to overwrite",
    ):
        if marker not in discovery_script_source:
            errors.append(f"workspace discovery probe is missing safety marker: {marker}")
    if "--debug-file" in discovery_script_source:
        errors.append("workspace discovery probe must not enable Grok debug-file")
    counts["grok_workspace_discovery_examples"] = 1

    fake_provider_path = ROOT / "scripts/fake_deepseek_provider.py"
    fake_launcher_path = ROOT / "scripts/invoke_grok_fake_provider_conformance.ps1"
    fake_provider_source = fake_provider_path.read_text(encoding="utf-8")
    fake_launcher_source = fake_launcher_path.read_text(encoding="utf-8")
    provider_required = (
        'HOST = "127.0.0.1"',
        '"authorization_value_recorded": False',
        '"requests.private.jsonl"',
    )
    launcher_required = (
        "New-NetFirewallRule",
        "Remove-NetFirewallRule",
        "CleanEnvironment",
        "LifJobObject",
        "tool-continuity",
        "reasoning_marker_preserved",
        "credential_value_absent_from_artifacts",
        "debug_capture_disabled",
        "New-RestrictedWorkspaceTrustReceipt",
        "workspace_trust_receipt_preflight",
        "workspace_trust_receipt_launch",
        "workspace_control_aggregate_unchanged",
        "workspace_scan_policy_unchanged",
        "Workspace control surface changed between preflight and launch receipts",
    )
    for marker in provider_required:
        if marker not in fake_provider_source:
            errors.append(f"fake provider is missing safety marker: {marker}")
    for marker in launcher_required:
        if marker not in fake_launcher_source:
            errors.append(f"fake-provider launcher is missing safety marker: {marker}")
    ordered_launcher_markers = (
        "$workspaceTrustReceiptPreflight = New-RestrictedWorkspaceTrustReceipt",
        "$inspectionArgs = @(",
        "$providerHandle = Start-RedirectedProcess",
        "$workspaceTrustReceiptLaunch = New-RestrictedWorkspaceTrustReceipt",
        "$grokHandle = Start-RedirectedProcess",
    )
    marker_positions = [fake_launcher_source.find(marker) for marker in ordered_launcher_markers]
    if any(position < 0 for position in marker_positions) or marker_positions != sorted(marker_positions):
        errors.append(
            "fake-provider launcher must order preflight receipt before binary/provider "
            "processes and launch receipt immediately before the Grok agent process"
        )
    if "'--debug-file'" in fake_launcher_source or '"--debug-file"' in fake_launcher_source:
        errors.append(
            "fake-provider launcher must not enable Grok debug-file after the "
            "0.2.106 credential-leak observation"
        )
    for schema_name in (
        "grok-fake-provider-result-v0.2.schema.json",
        "grok-tool-continuity-result-v0.3.schema.json",
    ):
        if not (ROOT / "integration/grok" / schema_name).is_file():
            errors.append(f"missing trust-integrated result schema: {schema_name}")
    counts["grok_fake_provider_fixtures"] = 1
    counts["grok_tool_continuity_fixtures"] = 1

    acp_probe_source = (
        ROOT / "scripts/invoke_grok_acp_initialize_probe.ps1"
    ).read_text(encoding="utf-8")
    acp_verifier_source = (
        ROOT / "scripts/verify_grok_acp_initialize_probe.py"
    ).read_text(encoding="utf-8")
    for schema_name in (
        "grok-acp-initialize-probe-result-v0.1.schema.json",
        "grok-acp-initialize-probe-verification-v0.1.schema.json",
    ):
        if not (ROOT / "integration/grok" / schema_name).is_file():
            errors.append(f"missing ACP initialize probe schema: {schema_name}")
    for marker in (
        "New-NetFirewallRule",
        "EnvironmentVariables.Clear()",
        "CreateKillOnClose",
        "clientCapabilities = [ordered]@{}",
        "session_or_prompt_requests_sent = 0",
        "_x.ai/mcp/servers_updated",
        "$lock.binary_release.installed_path",
    ):
        if marker not in acp_probe_source:
            errors.append(f"ACP initialize probe is missing safety marker: {marker}")
    for marker in (
        "request is not the exact initialize-only no-model shape",
        "stdout/stderr transcript or initialize notification classification mismatch",
        "response capability/meta projection mismatch",
        "workspace trust receipts do not prove an empty stable restricted workspace",
    ):
        if marker not in acp_verifier_source:
            errors.append(f"ACP initialize verifier is missing replay marker: {marker}")
    if not (ROOT / "integration/grok/tests/test_acp_initialize_probe.py").is_file():
        errors.append("missing ACP initialize verifier regression tests")
    counts["grok_acp_initialize_probe_fixtures"] = 1

    acp_tool_probe_source = (
        ROOT / "scripts/invoke_grok_acp_fake_tool_probe.ps1"
    ).read_text(encoding="utf-8")
    acp_tool_client_source = (
        ROOT / "scripts/run_grok_acp_fake_tool_client.py"
    ).read_text(encoding="utf-8")
    acp_tool_verifier_source = (
        ROOT / "scripts/verify_grok_acp_fake_tool_probe.py"
    ).read_text(encoding="utf-8")
    fake_provider_source = (
        ROOT / "scripts/fake_deepseek_provider.py"
    ).read_text(encoding="utf-8")
    for schema_name in (
        "grok-acp-fake-tool-probe-result-v0.1.schema.json",
        "grok-acp-fake-tool-probe-verification-v0.1.schema.json",
    ):
        if not (ROOT / "integration/grok" / schema_name).is_file():
            errors.append(f"missing ACP fake-tool probe schema: {schema_name}")
    for marker in (
        "New-NetFirewallRule",
        "EnvironmentVariables.Clear()",
        "CreateKillOnClose",
        "GROK_SANDBOX",
        "workspace-trust.preflight.json",
        "workspace-trust.launch.json",
        "workspace-trust.postrun.json",
        "allow_once",
        "cancel_permission",
    ):
        if marker not in acp_tool_probe_source:
            errors.append(f"ACP fake-tool launcher is missing safety marker: {marker}")
    for marker in (
        "session/request_permission",
        "session/cancel",
        "tool_call_update",
        "transcript.private.jsonl",
        "inherited_job_at_start",
    ):
        if marker not in acp_tool_client_source:
            errors.append(f"ACP fake-tool client is missing protocol marker: {marker}")
    for marker in (
        "binary_lock_matches",
        "transcript_projection_matches",
        "provider_capture_matches",
        "session_evidence_matches",
        "workspace_receipts_match",
        "scenario_semantics_match",
    ):
        if marker not in acp_tool_verifier_source:
            errors.append(f"ACP fake-tool verifier is missing replay marker: {marker}")
    if "tool-cancel" not in fake_provider_source:
        errors.append("fake provider is missing the ACP tool-cancel scenario")
    if not (ROOT / "integration/grok/tests/test_acp_fake_tool_probe.py").is_file():
        errors.append("missing ACP fake-tool verifier regression tests")
    counts["grok_acp_fake_tool_probe_fixtures"] = 2

    child_tree_launcher_source = (
        ROOT / "scripts/invoke_grok_windows_child_tree_probe.ps1"
    ).read_text(encoding="utf-8")
    child_tree_driver_source = (
        ROOT / "scripts/run_grok_windows_child_tree_probe.py"
    ).read_text(encoding="utf-8")
    child_tree_fixture_source = (
        ROOT / "scripts/child_tree_fixture.py"
    ).read_text(encoding="utf-8")
    child_tree_verifier_source = (
        ROOT / "scripts/verify_grok_windows_child_tree_probe.py"
    ).read_text(encoding="utf-8")
    firewall_source = (
        ROOT / "scripts/manage_grok_probe_firewall.ps1"
    ).read_text(encoding="utf-8")
    for schema_name in (
        "grok-windows-child-tree-probe-result-v0.1.schema.json",
        "grok-windows-child-tree-probe-verification-v0.1.schema.json",
    ):
        if not (ROOT / "integration/grok" / schema_name).is_file():
            errors.append(f"missing Windows child-tree probe schema: {schema_name}")
    for marker in (
        "Administrator token required",
        "tool_timeout",
        "task_cancel",
        "parent_exit",
        "ReleaseMetadataPath",
    ):
        if marker not in child_tree_launcher_source:
            errors.append(f"child-tree launcher is missing safety marker: {marker}")
    for marker in (
        "KillOnCloseJob",
        "refusing to overwrite output directory",
        "process-timeout",
        "process-cancel",
        "process-parent-exit",
        "post_trigger_residue_zero",
        "output_drain_matches_policy",
        "real_model_not_invoked",
        "workspace-trust.preflight.json",
        "workspace-trust.launch.json",
        "workspace-trust.postrun.json",
    ):
        if marker not in child_tree_driver_source:
            errors.append(f"child-tree driver is missing safety marker: {marker}")
    for marker in (
        "root",
        "child",
        "grandchild",
        "normal_completion",
        "refusing to overwrite role record",
        "LIF_CHILD_TREE_STDOUT",
        "LIF_CHILD_TREE_STDERR",
    ):
        if marker not in child_tree_fixture_source:
            errors.append(f"child-tree fixture is missing process marker: {marker}")
    for marker in (
        "binary_live_matches",
        "provider_tool_sequence_replays",
        "replay_nonce_residue_zero",
        "output_drain_policy_replays",
        "workspace_control_replays",
        "firewall_cleanup_replays",
    ):
        if marker not in child_tree_verifier_source:
            errors.append(f"child-tree verifier is missing replay marker: {marker}")
    for marker in (
        "New-NetFirewallRule",
        "Remove-NetFirewallRule",
        "nonloopback_ipv4_blocked",
        "all_ipv6_blocked",
        "remaining_rule_count",
    ):
        if marker not in firewall_source:
            errors.append(f"child-tree firewall helper is missing safety marker: {marker}")
    if "process-timeout" not in fake_provider_source or "process-cancel" not in fake_provider_source:
        errors.append("fake provider is missing Windows child-tree scenarios")
    if not (ROOT / "integration/grok/tests/test_windows_child_tree_probe.py").is_file():
        errors.append("missing Windows child-tree verifier regression tests")
    counts["grok_windows_child_tree_fixtures"] = 3

    compaction_launcher_source = (
        ROOT / "scripts/invoke_grok_compaction_provenance_probe.ps1"
    ).read_text(encoding="utf-8")
    compaction_driver_source = (
        ROOT / "scripts/run_grok_compaction_provenance_probe.py"
    ).read_text(encoding="utf-8")
    compaction_hook_source = (
        ROOT / "scripts/record_grok_compaction_hook.py"
    ).read_text(encoding="utf-8")
    compaction_verifier_source = (
        ROOT / "scripts/verify_grok_compaction_provenance_probe.py"
    ).read_text(encoding="utf-8")
    for schema_name in (
        "grok-compaction-provenance-probe-result-v0.1.schema.json",
        "grok-compaction-provenance-probe-verification-v0.1.schema.json",
    ):
        if not (ROOT / "integration/grok" / schema_name).is_file():
            errors.append(f"missing Grok compaction provenance schema: {schema_name}")
    for marker in (
        "Administrator token required",
        "ReleaseMetadataPath",
        "TimeoutSeconds",
    ):
        if marker not in compaction_launcher_source:
            errors.append(f"compaction launcher is missing safety marker: {marker}")
    for marker in (
        "refusing to overwrite output directory",
        "KillOnCloseJob",
        "compaction_requests",
        "compaction_checkpoints",
        "derived_unverified",
        "source-index retention/discard mapping",
        "real_model_invoked",
        "LIFGrokChild-Compact",
    ):
        if marker not in compaction_driver_source:
            errors.append(f"compaction driver is missing provenance marker: {marker}")
    for marker in ("pre_compact", "post_compact", "LIF_COMPACTION_HOOK_LOG"):
        if marker not in compaction_hook_source:
            errors.append(f"compaction hook recorder is missing marker: {marker}")
    for marker in (
        "artifact_hashes_valid",
        "derived_summary_boundary",
        "source_snapshot_matches_first_request",
        "unknown_mapping_explicit",
        "containment_declared",
    ):
        if marker not in compaction_verifier_source:
            errors.append(f"compaction verifier is missing replay marker: {marker}")
    if "compaction-provenance" not in fake_provider_source:
        errors.append("fake provider is missing the compaction provenance scenario")
    if not (
        ROOT / "integration/grok/tests/test_compaction_provenance_probe.py"
    ).is_file():
        errors.append("missing compaction provenance verifier regression tests")
    counts["grok_compaction_provenance_fixtures"] = 1

    bridge_builder_source = (
        ROOT / "scripts/build_grok_event_bridge.py"
    ).read_text(encoding="utf-8")
    bridge_verifier_source = (
        ROOT / "scripts/verify_grok_event_bridge.py"
    ).read_text(encoding="utf-8")
    postrun_bridge_source = (
        ROOT / "scripts/invoke_grok_postrun_evidence_bridge.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "cross_source_runtime_order",
        "not_established",
        "raw_content_omitted_from_bridge",
        "source_result_artifacts_match",
        "refusing to overwrite",
    ):
        if marker not in bridge_builder_source:
            errors.append(f"Grok event bridge is missing provenance marker: {marker}")
    for marker in (
        "event_hash_chain_valid",
        "source_artifacts_match",
        "event_count_and_breakdown_match",
        "completeness_not_promoted",
        "refusing to overwrite",
    ):
        if marker not in bridge_verifier_source:
            errors.append(f"Grok event bridge verifier is missing replay marker: {marker}")
    for marker in (
        "--local",
        "all_network_firewall_block",
        "LifPostrunJobObject",
        "workspace-trust-receipt.postrun.json",
        "debug_capture_disabled",
        "credential_present = $false",
    ):
        if marker not in postrun_bridge_source:
            errors.append(f"Grok post-run bridge is missing safety marker: {marker}")
    counts["grok_event_bridge_fixtures"] = 1

    runtime_examples = {
        "example-deepseek-adapter-profile.json": "deepseek-adapter-profile-v0.1.schema.json",
        "example-leak-scan-report.json": "leak-scan-report-v0.1.schema.json",
        "example-run-event.json": "run-event-v0.1.schema.json",
        "example-run-manifest.json": "run-manifest-v0.1.schema.json",
        "example-scenario-export-manifest.json": "scenario-export-manifest-v0.1.schema.json",
    }
    for example_name, schema_name in runtime_examples.items():
        errors.extend(
            _validate_instance(
                _load_json(ROOT / "runtime/examples" / example_name),
                ROOT / "runtime" / schema_name,
                f"runtime/examples/{example_name}",
            )
        )
    progress_fixture_root = ROOT / "runtime/fixtures/global-progress-sentinel-v0.1"
    progress_fixtures = {
        "input.json": "global-progress-input-v0.1.schema.json",
        "reasoned-continue.disposition.json": "global-progress-disposition-v0.1.schema.json",
        "replan.disposition.json": "global-progress-disposition-v0.1.schema.json",
    }
    for fixture_name, schema_name in progress_fixtures.items():
        errors.extend(
            _validate_instance(
                _load_json(progress_fixture_root / fixture_name),
                ROOT / "runtime" / schema_name,
                f"runtime/fixtures/global-progress-sentinel-v0.1/{fixture_name}",
            )
        )
    counts["global_progress_fixtures"] = len(progress_fixtures)
    holistic_progress_fixture = (
        ROOT / "runtime/fixtures/global-progress-holistic-v0.1/input.json"
    )
    errors.extend(
        _validate_instance(
            _load_json(holistic_progress_fixture),
            ROOT / "runtime/global-progress-history-input-v0.1.schema.json",
            "runtime/fixtures/global-progress-holistic-v0.1/input.json",
        )
    )
    for required_path in (
        ROOT / "runtime/global-progress-holistic-review-v0.1.schema.json",
        ROOT / "runtime/global-progress-holistic-disposition-v0.1.schema.json",
        ROOT / "runtime/global-progress-holistic-verification-v0.1.schema.json",
        ROOT / "scripts/build_global_progress_holistic_review.py",
        ROOT / "scripts/verify_global_progress_holistic_review.py",
    ):
        if not required_path.is_file():
            errors.append(
                f"missing holistic global-progress file: {required_path.relative_to(ROOT)}"
            )
    counts["global_progress_holistic_fixtures"] = 1
    checkpoint_policy_path = (
        ROOT
        / "runtime/fixtures/global-progress-checkpoint-adapter-v0.1/policy.json"
    )
    errors.extend(
        _validate_instance(
            _load_json(checkpoint_policy_path),
            ROOT / "runtime/global-progress-checkpoint-policy-v0.1.schema.json",
            "runtime/fixtures/global-progress-checkpoint-adapter-v0.1/policy.json",
        )
    )
    for required_path in (
        ROOT / "runtime/global-progress-checkpoint-verification-v0.1.schema.json",
        ROOT / "scripts/build_global_progress_checkpoint.py",
        ROOT / "scripts/verify_global_progress_checkpoint.py",
    ):
        if not required_path.is_file():
            errors.append(
                f"missing global-progress checkpoint adapter file: "
                f"{required_path.relative_to(ROOT)}"
            )
    counts["global_progress_checkpoint_policy_fixtures"] = 1
    for required_path in (
        ROOT / "runtime/global-progress-transition-request-v0.1.schema.json",
        ROOT / "runtime/global-progress-transition-receipt-v0.1.schema.json",
        ROOT / "runtime/global-progress-transition-verification-v0.1.schema.json",
        ROOT / "runtime/global-progress-controller-receipt-v0.1.schema.json",
        ROOT / "runtime/global-progress-controller-verification-v0.1.schema.json",
        ROOT / "runtime/global-progress-state-reduction-v0.1.schema.json",
        ROOT / "runtime/cli-session-lifecycle-observation-v0.1.schema.json",
        ROOT / "runtime/cli-session-lifecycle-event-v0.1.schema.json",
        ROOT / "runtime/cli-session-lifecycle-adapter-receipt-v0.1.schema.json",
        ROOT / "runtime/cli-session-lifecycle-adapter-verification-v0.1.schema.json",
        ROOT / "runtime/cli-session-lifecycle-observation-v0.2.schema.json",
        ROOT / "runtime/cli-session-lifecycle-event-v0.2.schema.json",
        ROOT / "runtime/cli-session-lifecycle-adapter-receipt-v0.2.schema.json",
        ROOT / "runtime/cli-session-lifecycle-adapter-verification-v0.2.schema.json",
        ROOT / "runtime/codex-app-server-capture-record-v0.1.schema.json",
        ROOT / "runtime/codex-app-server-lifecycle-normalization-v0.1.schema.json",
        ROOT / "runtime/codex-app-server-lifecycle-verification-v0.1.schema.json",
        ROOT / "scripts/build_global_progress_transition_event.py",
        ROOT / "scripts/verify_global_progress_transition_event.py",
        ROOT / "scripts/append_global_progress_transition_event.py",
        ROOT / "scripts/run_global_progress_controller.py",
        ROOT / "scripts/verify_global_progress_controller_receipt.py",
        ROOT / "scripts/reduce_global_progress_state.py",
        ROOT / "scripts/append_cli_session_lifecycle_event.py",
        ROOT / "scripts/verify_cli_session_lifecycle_receipt.py",
        ROOT / "scripts/normalize_codex_app_server_lifecycle.py",
        ROOT / "scripts/verify_codex_app_server_lifecycle.py",
        ROOT / "scripts/recover_torn_journal.py",
        ROOT / "prototype/fep_agent_proto/journal_lock.py",
        ROOT / "prototype/fep_agent_proto/journal_recovery.py",
        ROOT / "prototype/fep_agent_proto/global_progress_state.py",
        ROOT / "prototype/fep_agent_proto/cli_session_lifecycle.py",
        ROOT / "prototype/fep_agent_proto/codex_app_server_lifecycle.py",
        ROOT / "runtime/journal-recovery-event-v0.1.schema.json",
        ROOT / "runtime/journal-recovery-receipt-v0.1.schema.json",
        ROOT / "protocol/global-progress-reason-code-migration-v0.1.schema.json",
        ROOT / "protocol/global-progress-reason-code-migration-v0.1.yaml",
    ):
        if not required_path.is_file():
            errors.append(
                f"missing global-progress transition gate file: "
                f"{required_path.relative_to(ROOT)}"
            )
    counts["global_progress_transition_gate_fixtures"] = 1
    counts["global_progress_controller_fixtures"] = 1
    counts["global_progress_controller_verifier_fixtures"] = 1
    counts["global_progress_state_reducer_fixtures"] = 1
    counts["cli_session_lifecycle_adapter_fixtures"] = 1
    codex_fixture_root = (
        ROOT / "runtime/fixtures/codex-app-server-lifecycle-v0.1"
    )
    codex_positive_fixtures = (
        "happy.capture.jsonl",
        "interrupted-then-second-turn.capture.jsonl",
        "failed-turn.capture.jsonl",
        "interrupt-without-terminal.capture.jsonl",
    )
    codex_negative_fixtures = (
        "duplicate-sequence.invalid.jsonl",
        "cross-thread.invalid.jsonl",
        "truncated.invalid.jsonl",
    )
    capture_record_schema = (
        ROOT / "runtime/codex-app-server-capture-record-v0.1.schema.json"
    )
    for fixture_name in codex_positive_fixtures:
        fixture_path = codex_fixture_root / fixture_name
        for line_number, line in enumerate(
            fixture_path.read_text(encoding="utf-8").splitlines(),
            1,
        ):
            errors.extend(
                _validate_instance(
                    json.loads(line),
                    capture_record_schema,
                    (
                        "runtime/fixtures/codex-app-server-lifecycle-v0.1/"
                        f"{fixture_name}:{line_number}"
                    ),
                )
            )
    for fixture_name in codex_negative_fixtures:
        fixture_path = codex_fixture_root / fixture_name
        if not fixture_path.is_file():
            errors.append(
                "missing Codex lifecycle negative fixture: "
                f"{fixture_path.relative_to(ROOT)}"
            )
    counts["codex_app_server_lifecycle_fixtures"] = (
        len(codex_positive_fixtures) + len(codex_negative_fixtures)
    )
    counts["journal_recovery_fixtures"] = 1
    errors.extend(
        _validate_instance(
            _load_json(ROOT / "evaluation/example-evaluation-result-v0.1.json"),
            ROOT / "evaluation/evaluation-result-v0.1.schema.json",
            "evaluation/example-evaluation-result-v0.1.json",
        )
    )

    reason_codes = {item["code"] for item in reasons.get("reason_codes", [])}
    if progress_reason_migration:
        errors.extend(
            _validate_instance(
                progress_reason_migration,
                ROOT / "protocol/global-progress-reason-code-migration-v0.1.schema.json",
                "protocol/global-progress-reason-code-migration-v0.1.yaml",
            )
        )
        raw_mappings = progress_reason_migration.get("mappings", [])
        mappings = raw_mappings if isinstance(raw_mappings, list) else []
        counts["global_progress_reason_code_mappings"] = len(mappings)
        mapping_codes = [
            item.get("control_code") for item in mappings if isinstance(item, dict)
        ]
        if len(mapping_codes) != len(set(mapping_codes)):
            errors.append("global-progress reason-code migration has duplicate control codes")
        receipt_schema = _load_json(
            ROOT / "runtime/global-progress-transition-receipt-v0.1.schema.json"
        )
        expected_control_codes = set(
            receipt_schema["properties"]["control_codes"]["items"]["enum"]
        )
        if set(mapping_codes) != expected_control_codes:
            errors.append(
                "global-progress reason-code migration does not exactly cover receipt control codes"
            )
        raw_source_registry = progress_reason_migration.get("source_registry", {})
        source_registry = (
            raw_source_registry if isinstance(raw_source_registry, dict) else {}
        )
        if source_registry.get("sha256") != _sha256(reason_path):
            errors.append("global-progress reason-code migration source registry digest mismatch")
        for mapping in mappings:
            if not isinstance(mapping, dict):
                continue
            registered = set(mapping.get("registered_reason_codes", []))
            unknown_registered = sorted(registered - reason_codes)
            if unknown_registered:
                errors.append(
                    f"{mapping.get('control_code')} projects unknown registered reason codes: "
                    f"{unknown_registered}"
                )
            candidate = mapping.get("candidate_reason_code")
            if candidate in reason_codes:
                errors.append(
                    f"{mapping.get('control_code')} candidate reason code is already registered: "
                    f"{candidate}"
                )
    corpus_reason_codes = set(corpus.get("reason_codes", []))
    unknown_corpus_reasons = sorted(corpus_reason_codes - reason_codes)
    if unknown_corpus_reasons:
        errors.append(f"corpus uses unknown reason codes: {unknown_corpus_reasons}")
    for stage in gates.get("stages", []):
        unknown = sorted(set(stage.get("gate_codes", [])) - reason_codes)
        if unknown:
            errors.append(f"gate stage {stage['stage_id']} uses unknown reason codes: {unknown}")

    cases = corpus.get("cases", [])
    case_by_id = {case["case_id"]: case for case in cases}
    counts["cases"] = len(cases)
    if len(case_by_id) != len(cases):
        errors.append("case IDs are not unique")
    for case in cases:
        case_id = case["case_id"]
        for decision in case["oracle"]["expected_gate_decisions"]:
            if decision["gate_id"] not in reason_codes:
                errors.append(f"{case_id} uses unknown oracle gate {decision['gate_id']}")
        for other_id in case.get("countercase_ids", []):
            other = case_by_id.get(other_id)
            if other is None:
                errors.append(f"{case_id} references missing countercase {other_id}")
            elif case_id not in other.get("countercase_ids", []):
                errors.append(f"countercase link is not reciprocal: {case_id} -> {other_id}")

    coverage_case_ids: set[str] = set()
    for cluster in coverage.get("clusters", []):
        unknown_reasons = sorted(set(cluster["primary_reason_codes"]) - reason_codes)
        if unknown_reasons:
            errors.append(
                f"{cluster['cluster_id']} uses unknown reason codes: {unknown_reasons}"
            )
        referenced = list(cluster["historical_case_ids"]) + [
            item["case_id"] for item in cluster["challenge_cases"]
        ]
        coverage_case_ids.update(referenced)
        for case_id in referenced:
            if case_id not in case_by_id:
                errors.append(f"{cluster['cluster_id']} references missing case {case_id}")
    uncovered = sorted(set(case_by_id) - coverage_case_ids)
    if uncovered:
        errors.append(f"cases missing from coverage matrix: {uncovered}")

    fixture_schema = ROOT / "regression/fixture-v0.1.schema.json"
    provenance_schema = ROOT / "regression/historical-excerpt-provenance-v0.1.schema.json"
    fixture_count = 0
    for case in cases:
        references = []
        if case["scenario"].get("fixture_manifest"):
            references.append((case["scenario"]["fixture_manifest"], "scenario_only"))
        if case.get("curation_fixture_manifest"):
            references.append((case["curation_fixture_manifest"], "reviewer_only"))
        for relative, expected_visibility in references:
            fixture_path = ROOT / "regression" / Path(*_safe_fixture_path(relative).parts)
            if not fixture_path.is_file():
                errors.append(f"{case['case_id']} fixture is missing: {relative}")
                continue
            fixture_count += 1
            fixture = _load_json(fixture_path)
            errors.extend(
                _validate_instance(
                    fixture,
                    fixture_schema,
                    str(fixture_path.relative_to(ROOT)),
                )
            )
            if fixture.get("case_id") != case["case_id"]:
                errors.append(f"fixture case mismatch at {fixture_path.relative_to(ROOT)}")
            if fixture.get("visibility") != expected_visibility:
                errors.append(
                    f"fixture visibility mismatch at {fixture_path.relative_to(ROOT)}; "
                    f"expected {expected_visibility}"
                )
            for file_record in fixture.get("files", []):
                file_path = fixture_path.parent / Path(
                    *_safe_fixture_path(file_record["path"]).parts
                )
                if not file_path.is_file():
                    errors.append(f"fixture file is missing: {file_path.relative_to(ROOT)}")
                    continue
                if _sha256(file_path) != file_record["sha256"]:
                    errors.append(f"fixture digest mismatch: {file_path.relative_to(ROOT)}")
                if file_path.name == "provenance.json":
                    errors.extend(
                        _validate_instance(
                            _load_json(file_path),
                            provenance_schema,
                            str(file_path.relative_to(ROOT)),
                        )
                    )
    counts["fixture_manifests"] = fixture_count

    errors.extend(_check_markdown_links())
    errors.sort()
    return {
        "valid": not errors,
        "counts": counts,
        "error_count": len(errors),
        "errors": errors,
        "limitations": [
            "This is a deterministic repository-integrity check, not scientific validation.",
            "It does not establish oracle-free semantics or evaluation/holdout readiness.",
        ],
    }


def main() -> int:
    try:
        report = check_repository()
    except Exception as exc:
        report = {
            "valid": False,
            "error_count": 1,
            "errors": [f"repository checker failed: {type(exc).__name__}: {exc}"],
        }
    print(
        json.dumps(
            report, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False
        )
    )
    return 0 if report["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
