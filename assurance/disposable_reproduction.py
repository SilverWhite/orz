from __future__ import annotations

from copy import deepcopy
from decimal import Decimal
import json
import math
import platform
from pathlib import Path
import sys
from typing import Any

from .artifact_registry import (
    index_artifact_schemas,
    load_general_science_artifact_registry,
)
from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .general_science_review import review_general_science_bundle
from .utils import (
    atomic_write_json,
    canonical_bytes,
    is_link_or_reparse,
    load_json,
    require_no_linked_ancestors,
    require_within,
    safe_relative_path,
    sha256_bytes,
    sha256_file,
)


MANIFEST_SCHEMA = "disposable-reproduction-manifest-v0.1.schema.json"
RECEIPT_SCHEMA = "disposable-reproduction-receipt-v0.1.schema.json"
VERIFICATION_SCHEMA = "disposable-reproduction-verification-v0.1.schema.json"
RUN_PROOF_SCHEMA = "disposable-reproduction-run-proof-v0.1.schema.json"
DEFAULT_MANIFEST_NAME = "disposable-reproduction-manifest.json"
OUTPUT_ARTIFACT_SCHEMA_ID = "GSAS_NUMERICAL_SINGLE_RUN_0_1"
REPLAY_IMPLEMENTATION = "explicit_euler_decay_replay_v0.1"
VALUE_POINTERS = (
    "/run/step_size",
    "/run/step_count",
    "/run/final_value",
    "/run/final_absolute_error",
)


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
    payload = path.read_bytes()
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


def _parse_json(payload: bytes, *, label: str) -> Any:
    try:
        return json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise AssuranceError(f"cannot parse {label} as UTF-8 JSON: {exc}") from exc


def _require_output_root_outside_source(output_root: Path, source_root: Path) -> None:
    source_resolved = source_root.resolve(strict=True)
    output_resolved = output_root.resolve(strict=False)
    try:
        output_resolved.relative_to(source_resolved)
    except ValueError:
        return
    raise AssuranceError(
        "disposable reproduction output_root must not be inside source root"
    )


def _load_manifest(
    manifest_root: Path,
    manifest_name: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    manifest_relative = safe_relative_path(manifest_name)
    manifest_path = manifest_root / Path(*manifest_relative.parts)
    payload, file_record = _read_regular_file(
        manifest_root,
        manifest_name,
        expected_sha256=None,
        label="disposable reproduction manifest",
    )
    manifest = _parse_json(payload, label="disposable reproduction manifest")
    validate_contract(
        manifest,
        MANIFEST_SCHEMA,
        label="disposable reproduction manifest",
    )
    return manifest, {
        **file_record,
        "path": str(manifest_relative),
    }


def _index_records(records: list[dict[str, Any]], *, label: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for record in records:
        record_id = record["record_id"]
        if record_id in indexed:
            raise AssuranceError(f"duplicate {label} record ID: {record_id}")
        indexed[record_id] = record
    return indexed


def _load_source_bundle(
    manifest_root: Path,
    manifest: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any], dict[str, dict[str, Any]]]:
    bundle_record = manifest["source_bundle"]
    bundle_payload, bundle_file = _read_regular_file(
        manifest_root,
        bundle_record["path"],
        expected_sha256=bundle_record["sha256"],
        label="source bundle",
    )
    bundle = _parse_json(bundle_payload, label="source bundle")
    validate_contract(
        bundle,
        "general-science-review-bundle-v0.1.schema.json",
        label="source bundle",
    )
    review_general_science_bundle(
        bundle_root=manifest_root,
        bundle_name=bundle_record["path"],
    )
    bundle_records = _index_records(
        [*bundle["sources"], *bundle["artifacts"]],
        label="source bundle",
    )
    return bundle, bundle_file, bundle_records


def _verify_input_snapshot(
    manifest_root: Path,
    manifest: dict[str, Any],
    bundle_records: dict[str, dict[str, Any]],
) -> list[dict[str, Any]]:
    observed: list[dict[str, Any]] = []
    seen: set[str] = set()
    for record in manifest["input_snapshot"]:
        record_id = record["record_id"]
        if record_id in seen:
            raise AssuranceError(f"duplicate input snapshot record ID: {record_id}")
        seen.add(record_id)
        bundle_record = bundle_records.get(record_id)
        if bundle_record is None:
            raise AssuranceError(
                f"input snapshot references unknown source record: {record_id}"
            )
        if record["path"] != bundle_record["path"]:
            raise AssuranceError(
                f"input snapshot path disagrees with source bundle: {record_id}"
            )
        _, file_record = _read_regular_file(
            manifest_root,
            record["path"],
            expected_sha256=record["sha256"],
            label=f"input snapshot {record_id}",
        )
        observed.append({"record_id": record_id, **file_record})
    return observed


def _load_action_source(
    manifest_root: Path,
    manifest: dict[str, Any],
    bundle: dict[str, Any],
    bundle_records: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    source_action_id = manifest["allowed_action"]["source_action_id"]
    action_source_id = bundle["action_source_id"]
    action_record = bundle_records[action_source_id]
    payload, _ = _read_regular_file(
        manifest_root,
        action_record["path"],
        expected_sha256=action_record["sha256"],
        label="source action manifest",
    )
    action = _parse_json(payload, label="source action manifest")
    if action.get("action_id") != source_action_id:
        raise AssuranceError(
            "allowed action source_action_id does not match source manifest"
        )
    if action.get("state") != "completed":
        raise AssuranceError("source action must already be completed")
    return action


def _load_original_artifact(
    manifest_root: Path,
    artifact_record: dict[str, Any],
) -> dict[str, Any]:
    payload, _ = _read_regular_file(
        manifest_root,
        artifact_record["path"],
        expected_sha256=artifact_record["sha256"],
        label=f"original artifact {artifact_record['record_id']}",
    )
    document = _parse_json(
        payload,
        label=f"original artifact {artifact_record['record_id']}",
    )
    return document


def _explicit_euler_decay_document(
    *,
    original: dict[str, Any],
    action_id: str,
    parameters: dict[str, Any],
    step_size: float,
) -> dict[str, Any]:
    final_time = parameters["final_time"]
    step_count_float = final_time / step_size
    step_count = round(step_count_float)
    if not math.isclose(step_count_float, step_count, rel_tol=0.0, abs_tol=1e-12):
        raise AssuranceError("final_time must divide evenly by step_size")
    decimal_step = Decimal(str(step_size))
    decimal_initial = Decimal(str(parameters["initial_value"]))
    final_value = float(decimal_initial * ((Decimal("1.0") - decimal_step) ** step_count))
    final_absolute_error = abs(final_value - parameters["exact_final_value"])

    document = deepcopy(original)
    if document.get("method") != parameters["method"]:
        raise AssuranceError("original artifact method disagrees with replay parameters")
    if document.get("equation") != parameters["equation"]:
        raise AssuranceError("original artifact equation disagrees with replay parameters")
    if document.get("initial_value") != parameters["initial_value"]:
        raise AssuranceError(
            "original artifact initial_value disagrees with replay parameters"
        )
    if document.get("final_time") != final_time:
        raise AssuranceError("original artifact final_time disagrees with replay parameters")
    if document.get("exact_final_value") != parameters["exact_final_value"]:
        raise AssuranceError(
            "original artifact exact_final_value disagrees with replay parameters"
        )
    context = document.get("comparison_context", {})
    lineage = context.get("lineage", {})
    if lineage.get("producer_action_id") != action_id:
        raise AssuranceError("original artifact producer lineage is not replay action")
    condition = context.get("condition", {})
    if condition.get("value") != step_size:
        raise AssuranceError("original artifact condition disagrees with replay step")
    document["run"] = {
        "step_size": step_size,
        "step_count": step_count,
        "final_value": final_value,
        "final_absolute_error": final_absolute_error,
    }
    return document


def _validate_generated_artifact(document: dict[str, Any]) -> None:
    artifact_registry = load_general_science_artifact_registry()
    schemas = index_artifact_schemas(artifact_registry)
    schema_name = schemas[OUTPUT_ARTIFACT_SCHEMA_ID]["schema_name"]
    validate_contract(
        document,
        schema_name,
        label="generated disposable reproduction artifact",
    )


def _build_outputs(
    manifest_root: Path,
    manifest: dict[str, Any],
    bundle_records: dict[str, dict[str, Any]],
) -> list[tuple[dict[str, Any], dict[str, Any], dict[str, Any]]]:
    action = manifest["allowed_action"]
    if action["implementation"] != REPLAY_IMPLEMENTATION:
        raise AssuranceError("unsupported disposable reproduction implementation")
    built = []
    seen_paths: set[str] = set()
    for output in manifest["outputs"]:
        if output["path"] in seen_paths:
            raise AssuranceError(f"duplicate output path: {output['path']}")
        seen_paths.add(output["path"])
        if tuple(output["value_pointers"]) != VALUE_POINTERS:
            raise AssuranceError(
                f"{output['output_id']} must use the frozen value pointer set"
            )
        artifact_record = bundle_records.get(output["original_artifact_id"])
        if artifact_record is None:
            raise AssuranceError(
                f"{output['output_id']} references unknown original artifact"
            )
        if artifact_record.get("artifact_schema_id") != output["artifact_schema_id"]:
            raise AssuranceError(
                f"{output['output_id']} artifact schema disagrees with source bundle"
            )
        original = _load_original_artifact(manifest_root, artifact_record)
        generated = _explicit_euler_decay_document(
            original=original,
            action_id=action["source_action_id"],
            parameters=action["parameters"],
            step_size=output["step_size"],
        )
        _validate_generated_artifact(generated)
        value_checks = []
        for pointer in output["value_pointers"]:
            if _json_pointer(generated, pointer, label=output["output_id"]) != _json_pointer(
                original,
                pointer,
                label=artifact_record["record_id"],
            ):
                raise AssuranceError(
                    f"{output['output_id']} replay value differs at {pointer}"
                )
            value_checks.append(
                {
                    "pointer": pointer,
                    "matches_original": True,
                }
            )
        built.append((output, generated, {
            "record": artifact_record,
            "value_checks": value_checks,
        }))
    return built


def _write_outputs(
    output_root: Path,
    built_outputs: list[tuple[dict[str, Any], dict[str, Any], dict[str, Any]]],
) -> list[dict[str, Any]]:
    written: list[dict[str, Any]] = []
    output_root.mkdir(parents=True, exist_ok=True)
    require_no_linked_ancestors(output_root, output_root)
    for output, document, original in built_outputs:
        relative = safe_relative_path(output["path"])
        path = output_root / Path(*relative.parts)
        require_within(path, output_root, must_exist=False)
        if path.exists():
            raise AssuranceError(f"refusing to overwrite output artifact: {path}")
        atomic_write_json(path, document)
        digest = sha256_file(path)
        written.append(
            {
                "output_id": output["output_id"],
                "path": output["path"],
                "bytes": path.stat().st_size,
                "sha256": digest,
                "original_artifact_id": output["original_artifact_id"],
                "original_artifact_sha256": original["record"]["sha256"],
                "artifact_schema_id": output["artifact_schema_id"],
                "value_checks": original["value_checks"],
            }
        )
    return written


def run_disposable_reproduction(
    *,
    manifest_root: Path,
    output_root: Path,
    manifest_name: str = DEFAULT_MANIFEST_NAME,
) -> dict[str, Any]:
    manifest_root = manifest_root.resolve(strict=True)
    _require_output_root_outside_source(output_root, manifest_root)
    manifest, manifest_file = _load_manifest(manifest_root, manifest_name)
    bundle, bundle_file, bundle_records = _load_source_bundle(
        manifest_root,
        manifest,
    )
    _verify_input_snapshot(manifest_root, manifest, bundle_records)
    _load_action_source(manifest_root, manifest, bundle, bundle_records)
    source_before = sha256_file(manifest_root / manifest["source_bundle"]["path"])
    built_outputs = _build_outputs(manifest_root, manifest, bundle_records)
    outputs = _write_outputs(output_root, built_outputs)
    source_after = sha256_file(manifest_root / manifest["source_bundle"]["path"])
    if source_after != source_before:
        raise AssuranceError("source bundle changed during disposable reproduction")
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "general_science_disposable_reproduction_receipt",
        "reproduction_id": manifest["reproduction_id"],
        "manifest_sha256": manifest_file["sha256"],
        "source_bundle_sha256": bundle_file["sha256"],
        "source_snapshot_unchanged": True,
        "output_root": str(output_root.resolve(strict=True)),
        "outputs": outputs,
        "safety": {
            "source_write_attempted": False,
            "model_invoked": False,
            "network_requested": False,
            "child_process_spawned": False,
            "external_code_executed": False,
        },
        "evidence_boundary": {
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
        },
        "valid": True,
        "limitations": [
            "Replay uses a fixed in-process deterministic fixture action only.",
            "Matching replay outputs do not add independent scientific evidence.",
            "No model quality, causal, mechanism, generality, or holdout claim is established.",
        ],
    }
    validate_contract(
        receipt,
        RECEIPT_SCHEMA,
        label="disposable reproduction receipt",
    )
    return receipt


def verify_disposable_reproduction_receipt(
    receipt: dict[str, Any],
    *,
    manifest_root: Path,
    output_root: Path,
    manifest_name: str = DEFAULT_MANIFEST_NAME,
) -> dict[str, Any]:
    validate_contract(
        receipt,
        RECEIPT_SCHEMA,
        label="disposable reproduction receipt",
    )
    manifest_root = manifest_root.resolve(strict=True)
    manifest, manifest_file = _load_manifest(manifest_root, manifest_name)
    if receipt["reproduction_id"] != manifest["reproduction_id"]:
        raise AssuranceError("receipt reproduction_id disagrees with manifest")
    if receipt["manifest_sha256"] != manifest_file["sha256"]:
        raise AssuranceError("receipt manifest digest disagrees with current manifest")
    bundle, bundle_file, bundle_records = _load_source_bundle(
        manifest_root,
        manifest,
    )
    if receipt["source_bundle_sha256"] != bundle_file["sha256"]:
        raise AssuranceError("receipt source bundle digest disagrees with source")
    output_root_resolved = output_root.resolve(strict=True)
    if receipt["output_root"] != str(output_root_resolved):
        raise AssuranceError("receipt output_root disagrees with verifier input")
    _verify_input_snapshot(manifest_root, manifest, bundle_records)
    _load_action_source(manifest_root, manifest, bundle, bundle_records)
    expected_outputs = _build_outputs(manifest_root, manifest, bundle_records)
    receipt_outputs = {
        item["output_id"]: item
        for item in receipt["outputs"]
    }
    if len(receipt_outputs) != len(receipt["outputs"]):
        raise AssuranceError("receipt output IDs are not unique")
    for output, document, original in expected_outputs:
        receipt_record = receipt_outputs.get(output["output_id"])
        if receipt_record is None:
            raise AssuranceError(f"receipt missing output: {output['output_id']}")
        if receipt_record["path"] != output["path"]:
            raise AssuranceError(f"receipt output path mismatch: {output['output_id']}")
        if receipt_record["original_artifact_id"] != output["original_artifact_id"]:
            raise AssuranceError(
                f"receipt original artifact mismatch: {output['output_id']}"
            )
        if receipt_record["artifact_schema_id"] != output["artifact_schema_id"]:
            raise AssuranceError(
                f"receipt artifact schema mismatch: {output['output_id']}"
            )
        if receipt_record["value_checks"] != original["value_checks"]:
            raise AssuranceError(f"receipt value checks mismatch: {output['output_id']}")
        relative = safe_relative_path(output["path"])
        path = output_root / Path(*relative.parts)
        require_within(path, output_root, must_exist=True)
        require_no_linked_ancestors(path, output_root)
        observed = load_json(path)
        _validate_generated_artifact(observed)
        if canonical_bytes(observed) != canonical_bytes(document):
            raise AssuranceError(f"output content differs: {output['output_id']}")
        digest = sha256_file(path)
        if digest != receipt_record["sha256"]:
            raise AssuranceError(f"receipt output digest mismatch: {output['output_id']}")
        if path.stat().st_size != receipt_record["bytes"]:
            raise AssuranceError(f"receipt output size mismatch: {output['output_id']}")
    receipt_sha256 = sha256_bytes(canonical_bytes(receipt))
    verification = {
        "schema_version": "0.1.0-draft",
        "verification_kind": "general_science_disposable_reproduction_verification",
        "reproduction_id": manifest["reproduction_id"],
        "valid": True,
        "manifest_sha256": manifest_file["sha256"],
        "receipt_sha256": receipt_sha256,
        "verified_outputs": len(expected_outputs),
        "source_snapshot_unchanged": True,
        "evidence_boundary": {
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
        },
    }
    validate_contract(
        verification,
        VERIFICATION_SCHEMA,
        label="disposable reproduction verification",
    )
    return verification


def _code_snapshot() -> dict[str, Any]:
    module_path = ASSURANCE_ROOT / "disposable_reproduction.py"
    return {
        "implementation": REPLAY_IMPLEMENTATION,
        "files": [
            {
                "path": "assurance/disposable_reproduction.py",
                "sha256": sha256_file(module_path),
            }
        ],
    }


def _environment_snapshot() -> dict[str, Any]:
    return {
        "python_version": platform.python_version(),
        "python_implementation": platform.python_implementation(),
        "platform_system": platform.system() or "unknown",
        "platform_release": platform.release() or "unknown",
        "filesystem_encoding": sys.getfilesystemencoding() or "unknown",
    }


def _journal_event(
    *,
    sequence: int,
    event_type: str,
    payload: dict[str, Any],
    previous_event_sha256: str | None,
) -> dict[str, Any]:
    event = {
        "sequence": sequence,
        "event_type": event_type,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "previous_event_sha256": previous_event_sha256,
    }
    return {
        **event,
        "event_sha256": sha256_bytes(canonical_bytes(event)),
    }


def _journal_events(
    *,
    manifest_sha256: str,
    receipt_sha256: str,
    verification_sha256: str,
    code: dict[str, Any],
    inputs: list[dict[str, Any]],
    outputs: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    preflight = _journal_event(
        sequence=0,
        event_type="run_preflight",
        previous_event_sha256=None,
        payload={
            "manifest_sha256": manifest_sha256,
            "code_sha256": sha256_bytes(canonical_bytes(code)),
            "input_count": len(inputs),
        },
    )
    started = _journal_event(
        sequence=1,
        event_type="run_started",
        previous_event_sha256=preflight["event_sha256"],
        payload={
            "receipt_sha256": receipt_sha256,
            "output_count": len(outputs),
        },
    )
    finished = _journal_event(
        sequence=2,
        event_type="run_finished",
        previous_event_sha256=started["event_sha256"],
        payload={
            "verification_sha256": verification_sha256,
            "valid": True,
        },
    )
    return [preflight, started, finished]


def _proof_inputs(
    manifest: dict[str, Any],
) -> list[dict[str, Any]]:
    return [
        {
            "record_id": manifest["source_bundle"]["record_id"],
            "role": "source_bundle",
            "path": manifest["source_bundle"]["path"],
            "sha256": manifest["source_bundle"]["sha256"],
        },
        *[
            {
                "record_id": item["record_id"],
                "role": "input_snapshot",
                "path": item["path"],
                "sha256": item["sha256"],
            }
            for item in manifest["input_snapshot"]
        ],
    ]


def _proof_outputs(receipt: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "output_id": item["output_id"],
            "path": item["path"],
            "sha256": item["sha256"],
            "original_artifact_id": item["original_artifact_id"],
            "artifact_schema_id": item["artifact_schema_id"],
        }
        for item in receipt["outputs"]
    ]


def build_disposable_reproduction_run_proof(
    receipt: dict[str, Any],
    *,
    manifest_root: Path,
    output_root: Path,
    manifest_name: str = DEFAULT_MANIFEST_NAME,
) -> dict[str, Any]:
    verification = verify_disposable_reproduction_receipt(
        receipt,
        manifest_root=manifest_root,
        output_root=output_root,
        manifest_name=manifest_name,
    )
    manifest_root = manifest_root.resolve(strict=True)
    manifest, manifest_file = _load_manifest(manifest_root, manifest_name)
    code = _code_snapshot()
    environment = _environment_snapshot()
    inputs = _proof_inputs(manifest)
    outputs = _proof_outputs(receipt)
    receipt_sha256 = sha256_bytes(canonical_bytes(receipt))
    verification_sha256 = sha256_bytes(canonical_bytes(verification))
    events = _journal_events(
        manifest_sha256=manifest_file["sha256"],
        receipt_sha256=receipt_sha256,
        verification_sha256=verification_sha256,
        code=code,
        inputs=inputs,
        outputs=outputs,
    )
    proof = {
        "schema_version": "0.1.0-draft",
        "proof_kind": "general_science_disposable_reproduction_run_proof",
        "reproduction_id": manifest["reproduction_id"],
        "manifest_sha256": manifest_file["sha256"],
        "receipt_sha256": receipt_sha256,
        "verification_sha256": verification_sha256,
        "code": code,
        "environment": environment,
        "inputs": inputs,
        "outputs": outputs,
        "journal": {
            "policy": {
                "canonicalization": "RFC8785",
                "hash_chain": "sha256",
                "event_count": 3,
                "content_policy": "metadata_only",
            },
            "events": events,
        },
        "evidence_boundary": {
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
            "runner_status": "development_fixture_only",
        },
        "valid": True,
    }
    validate_contract(
        proof,
        RUN_PROOF_SCHEMA,
        label="disposable reproduction run proof",
    )
    return proof


def verify_disposable_reproduction_run_proof(
    proof: dict[str, Any],
    receipt: dict[str, Any],
    *,
    manifest_root: Path,
    output_root: Path,
    manifest_name: str = DEFAULT_MANIFEST_NAME,
) -> dict[str, Any]:
    validate_contract(
        proof,
        RUN_PROOF_SCHEMA,
        label="disposable reproduction run proof",
    )
    expected = build_disposable_reproduction_run_proof(
        receipt,
        manifest_root=manifest_root,
        output_root=output_root,
        manifest_name=manifest_name,
    )
    if canonical_bytes(proof) != canonical_bytes(expected):
        raise AssuranceError("disposable reproduction run proof mismatch")
    return {
        "schema_version": "0.1.0-draft",
        "verification_kind": "general_science_disposable_reproduction_run_proof_verification",
        "reproduction_id": proof["reproduction_id"],
        "valid": True,
        "proof_sha256": sha256_bytes(canonical_bytes(proof)),
        "journal_event_count": len(proof["journal"]["events"]),
        "code_file_count": len(proof["code"]["files"]),
        "input_count": len(proof["inputs"]),
        "output_count": len(proof["outputs"]),
        "evidence_boundary": proof["evidence_boundary"],
    }
