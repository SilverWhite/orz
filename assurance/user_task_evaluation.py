from __future__ import annotations

from copy import deepcopy
from pathlib import Path
from typing import Any
import uuid

from .contracts import ASSURANCE_ROOT, validate_contract
from .conversation import ConversationNamespace
from .errors import AssuranceError
from .integrated_run import (
    BACKEND_POLICY_NAME,
    execute_workspace_first_integrated_run,
    initialize_workspace_marker,
    load_execution_backend_policy,
    select_execution_backend,
    verify_workspace_first_integrated_run,
)
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_json,
    canonical_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)


DEFAULT_SUITE = (
    ASSURANCE_ROOT
    / "fixtures"
    / "p5"
    / "synthetic-user-task-suite-v0.1.json"
)
RESULT_SCHEMA = "synthetic-user-task-evaluation-receipt-v0.1.schema.json"
SUITE_SCHEMA = "synthetic-user-task-suite-v0.1.schema.json"


def load_synthetic_user_task_suite(
    path: Path | None = None,
) -> dict[str, Any]:
    selected = path or DEFAULT_SUITE
    suite = load_json(selected)
    validate_contract(
        suite,
        SUITE_SCHEMA,
        label="P5 synthetic user task suite",
    )
    task_ids = [item["task_id"] for item in suite["tasks"]]
    if len(task_ids) != len(set(task_ids)):
        raise AssuranceError("P5 synthetic task IDs must be unique")
    personas = [item["persona"] for item in suite["tasks"]]
    if set(personas) != {"novice", "experienced"}:
        raise AssuranceError(
            "P5 task suite must exercise novice and experienced personas"
        )
    return suite


def _sign(
    body: dict[str, Any], *, key_store: InstallationKeyStore
) -> dict[str, Any]:
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": key_store.key_id,
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": key_store.sign(payload),
        },
    }


def _verify_signature(
    receipt: dict[str, Any], *, key_store: InstallationKeyStore
) -> list[str]:
    errors: list[str] = []
    body = {key: value for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    integrity = receipt["integrity"]
    if integrity["key_id"] != key_store.key_id:
        errors.append("P5 receipt key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(payload):
        errors.append("P5 receipt signed payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        errors.append("P5 receipt signature verification failed")
    return errors


def _standard_task_result(
    task: dict[str, Any],
    *,
    task_root: Path,
    key_store: InstallationKeyStore,
    policy_path: Path,
) -> dict[str, Any]:
    workspace = task_root / "workspace"
    workspace.mkdir(parents=True)
    initialize_workspace_marker(workspace)
    result = execute_workspace_first_integrated_run(
        workspace=workspace,
        conversation_repository=task_root / "conversations",
        key_store=key_store,
        policy_path=policy_path,
    )
    receipt = result["receipt"]
    atomic_write_json(task_root / "integrated-run-receipt.json", receipt)
    expected = task["expected"]
    return {
        "task_id": task["task_id"],
        "persona": task["persona"],
        "requested_mode": "standard",
        "observed_outcome": "completed",
        "reason_code": "STANDARD_WORKSPACE_GUARDED_COMPLETED",
        "backend": "workspace_guarded",
        "evidence": {
            "conversation_id": result["namespace"].conversation_id,
            "integrated_receipt_sha256": sha256_bytes(
                canonical_bytes(receipt)
            ),
            "archive_complete": result["archive"]["archive_complete"],
            "verification_valid": result["verification"]["valid"],
        },
        "execution": {
            "docker_invoked": False,
            "child_process_spawned": False,
            "model_invoked": False,
            "network_requested": False,
            "fallback_used": False,
        },
        "ux_projection": {
            "message_code": expected["message_code"],
            "boundary_code": expected["boundary_code"],
            "next_action_code": expected["next_action_code"],
            "human_comprehension_assessed": False,
            "actual_human_response": None,
        },
        "conformance_passed": (
            result["verification"]["valid"]
            and result["archive"]["archive_complete"]
        ),
    }


def _strict_unavailable_task_result(
    task: dict[str, Any],
    *,
    task_root: Path,
    policy_path: Path,
) -> dict[str, Any]:
    task_root.mkdir(parents=True)
    refusal_observed = False
    try:
        select_execution_backend("strict", policy_path=policy_path)
    except AssuranceError as exc:
        refusal_observed = (
            "requires an explicit, observed P2 Docker selection" in str(exc)
            and "fallback to standard" in str(exc)
        )
    if not refusal_observed:
        raise AssuranceError("P5 strict task did not fail closed as required")
    expected = task["expected"]
    return {
        "task_id": task["task_id"],
        "persona": task["persona"],
        "requested_mode": "strict",
        "observed_outcome": "fail_closed",
        "reason_code": "STRICT_OBSERVED_DOCKER_REQUIRED",
        "backend": None,
        "evidence": {
            "conversation_id": None,
            "integrated_receipt_sha256": None,
            "archive_complete": None,
            "verification_valid": True,
        },
        "execution": {
            "docker_invoked": False,
            "child_process_spawned": False,
            "model_invoked": False,
            "network_requested": False,
            "fallback_used": False,
        },
        "ux_projection": {
            "message_code": expected["message_code"],
            "boundary_code": expected["boundary_code"],
            "next_action_code": expected["next_action_code"],
            "human_comprehension_assessed": False,
            "actual_human_response": None,
        },
        "conformance_passed": True,
    }


def run_synthetic_user_task_evaluation(
    *,
    run_root: Path,
    key_store: InstallationKeyStore,
    suite_path: Path | None = None,
    policy_path: Path | None = None,
) -> dict[str, Any]:
    suite_file = suite_path or DEFAULT_SUITE
    policy_file = policy_path or ASSURANCE_ROOT / BACKEND_POLICY_NAME
    suite = load_synthetic_user_task_suite(suite_file)
    load_execution_backend_policy(policy_file)
    run_root.mkdir(parents=True, exist_ok=True)
    tasks_root = run_root / "tasks"
    if tasks_root.exists():
        raise AssuranceError("refusing to reuse a P5 task root")
    tasks_root.mkdir()

    results: list[dict[str, Any]] = []
    for task in suite["tasks"]:
        task_root = tasks_root / task["task_id"]
        if task["requested_mode"] == "standard":
            item = _standard_task_result(
                task,
                task_root=task_root,
                key_store=key_store,
                policy_path=policy_file,
            )
        else:
            item = _strict_unavailable_task_result(
                task,
                task_root=task_root,
                policy_path=policy_file,
            )
        results.append(item)

    passed = sum(item["conformance_passed"] for item in results)
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "synthetic_user_task_evaluation",
        "receipt_id": f"UTE-{uuid.uuid4().hex.upper()}",
        "created_at": utc_now(),
        "suite_id": suite["suite_id"],
        "status": (
            "mechanical_preflight_passed"
            if passed == len(results)
            else "mechanical_preflight_failed"
        ),
        "bindings": {
            "suite_sha256": sha256_bytes(canonical_bytes(suite)),
            "backend_policy_sha256": sha256_file(policy_file),
        },
        "environment": {
            "partition": suite["partition"],
            "disposable_workspace_only": True,
            "fake_credentials_only": True,
            "real_project_data_used": False,
            "network_requested": False,
            "model_invoked": False,
            "docker_invoked": False,
            "child_process_spawned": False,
            "external_participant_count": 0,
        },
        "task_results": results,
        "aggregate": {
            "planned_tasks": len(suite["tasks"]),
            "completed_tasks": len(results),
            "conformance_passed_tasks": passed,
            "conformance_failed_tasks": len(results) - passed,
        },
        "claims": {
            "mechanical_conformance_assessed": True,
            "human_comprehension_assessed": False,
            "human_usability_assessed": False,
            "ordinary_user_safety_established": False,
            "production_readiness_established": False,
        },
    }
    receipt = _sign(body, key_store=key_store)
    validate_contract(
        receipt,
        RESULT_SCHEMA,
        label="P5 synthetic user task evaluation receipt",
    )
    verification = verify_synthetic_user_task_evaluation(
        receipt,
        run_root=run_root,
        key_store=key_store,
        suite_path=suite_file,
        policy_path=policy_file,
    )
    if not verification["valid"]:
        raise AssuranceError(
            "P5 synthetic evaluation failed verification: "
            + "; ".join(verification["errors"])
        )
    return {
        "receipt": receipt,
        "verification": verification,
        "suite": suite,
    }


def verify_synthetic_user_task_evaluation(
    receipt: dict[str, Any],
    *,
    run_root: Path,
    key_store: InstallationKeyStore,
    suite_path: Path | None = None,
    policy_path: Path | None = None,
) -> dict[str, Any]:
    errors: list[str] = []
    suite_file = suite_path or DEFAULT_SUITE
    policy_file = policy_path or ASSURANCE_ROOT / BACKEND_POLICY_NAME
    try:
        validate_contract(
            receipt,
            RESULT_SCHEMA,
            label="P5 synthetic user task evaluation receipt",
        )
        errors.extend(_verify_signature(receipt, key_store=key_store))
        suite = load_synthetic_user_task_suite(suite_file)
        policy = load_execution_backend_policy(policy_file)
    except Exception as exc:
        return {"valid": False, "errors": [str(exc)]}

    if receipt["bindings"]["suite_sha256"] != sha256_bytes(
        canonical_bytes(suite)
    ):
        errors.append("P5 suite digest mismatch")
    if receipt["bindings"]["backend_policy_sha256"] != sha256_file(policy_file):
        errors.append("P5 backend policy digest mismatch")
    if receipt["suite_id"] != suite["suite_id"]:
        errors.append("P5 suite identity mismatch")
    if policy["default_mode"] != "standard":
        errors.append("P5 backend policy default is not standard")
    if policy["modes"]["strict"]["fallback_to_standard"]:
        errors.append("P5 strict policy permits fallback")

    expected_by_id = {item["task_id"]: item for item in suite["tasks"]}
    result_ids = [item["task_id"] for item in receipt["task_results"]]
    if len(result_ids) != len(set(result_ids)):
        errors.append("P5 result task IDs are not unique")
    if set(result_ids) != set(expected_by_id):
        errors.append("P5 result task set mismatch")

    verified_passes = 0
    for item in receipt["task_results"]:
        expected = expected_by_id.get(item["task_id"])
        if expected is None:
            continue
        if item["persona"] != expected["persona"]:
            errors.append(f"{item['task_id']}: persona mismatch")
        if item["requested_mode"] != expected["requested_mode"]:
            errors.append(f"{item['task_id']}: requested mode mismatch")
        if item["observed_outcome"] != expected["expected"]["outcome"]:
            errors.append(f"{item['task_id']}: outcome mismatch")
        if item["reason_code"] != expected["expected"]["reason_code"]:
            errors.append(f"{item['task_id']}: reason code mismatch")
        for key in ("message_code", "boundary_code", "next_action_code"):
            if item["ux_projection"][key] != expected["expected"][key]:
                errors.append(f"{item['task_id']}: {key} mismatch")

        task_root = run_root / "tasks" / item["task_id"]
        task_valid = True
        if item["requested_mode"] == "standard":
            try:
                integrated = load_json(
                    task_root / "integrated-run-receipt.json"
                )
                conversations = [
                    path
                    for path in (task_root / "conversations").iterdir()
                    if path.is_dir()
                ]
                if len(conversations) != 1:
                    raise AssuranceError(
                        "standard task must have one conversation"
                    )
                namespace = ConversationNamespace(conversations[0])
                integrated_verification = (
                    verify_workspace_first_integrated_run(
                        integrated,
                        namespace=namespace,
                        key_store=key_store,
                        workspace=task_root / "workspace",
                        policy_path=policy_file,
                    )
                )
                if not integrated_verification["valid"]:
                    errors.extend(
                        f"{item['task_id']}: integrated {message}"
                        for message in integrated_verification["errors"]
                    )
                    task_valid = False
                if item["evidence"]["conversation_id"] != (
                    namespace.conversation_id
                ):
                    errors.append(
                        f"{item['task_id']}: conversation binding mismatch"
                    )
                    task_valid = False
                if item["evidence"][
                    "integrated_receipt_sha256"
                ] != sha256_bytes(canonical_bytes(integrated)):
                    errors.append(
                        f"{item['task_id']}: integrated receipt mismatch"
                    )
                    task_valid = False
            except Exception as exc:
                errors.append(f"{item['task_id']}: {exc}")
                task_valid = False
        else:
            if (task_root / "workspace").exists():
                errors.append(
                    f"{item['task_id']}: strict refusal created a workspace"
                )
                task_valid = False
            if (task_root / "conversations").exists():
                errors.append(
                    f"{item['task_id']}: strict refusal created a conversation"
                )
                task_valid = False
            if item["backend"] is not None or any(
                item["execution"][key]
                for key in (
                    "docker_invoked",
                    "child_process_spawned",
                    "model_invoked",
                    "network_requested",
                    "fallback_used",
                )
            ):
                errors.append(
                    f"{item['task_id']}: strict refusal execution mismatch"
                )
                task_valid = False
        if not item["conformance_passed"]:
            task_valid = False
        if task_valid:
            verified_passes += 1

    aggregate = receipt["aggregate"]
    if aggregate["planned_tasks"] != len(suite["tasks"]):
        errors.append("P5 planned task count mismatch")
    if aggregate["completed_tasks"] != len(receipt["task_results"]):
        errors.append("P5 completed task count mismatch")
    if aggregate["conformance_passed_tasks"] != verified_passes:
        errors.append("P5 conformance pass count mismatch")
    if aggregate["conformance_failed_tasks"] != (
        len(receipt["task_results"]) - verified_passes
    ):
        errors.append("P5 conformance failure count mismatch")
    expected_status = (
        "mechanical_preflight_passed"
        if verified_passes == len(suite["tasks"])
        else "mechanical_preflight_failed"
    )
    if receipt["status"] != expected_status:
        errors.append("P5 status mismatch")
    return {
        "valid": not errors,
        "errors": errors,
        "status": receipt["status"],
        "planned_tasks": len(suite["tasks"]),
        "verified_passes": verified_passes,
        "external_participant_count": 0,
        "human_comprehension_assessed": False,
        "docker_invoked": False,
        "model_invoked": False,
        "network_requested": False,
        "child_process_spawned": False,
    }
