from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from .contracts import ASSURANCE_ROOT, validate_contract
from .disposable_reproduction import (
    DEFAULT_MANIFEST_NAME,
    verify_disposable_reproduction_run_proof,
)
from .errors import AssuranceError
from .utils import canonical_bytes, exclusive_create_bytes, sha256_bytes, sha256_file


ROOT = ASSURANCE_ROOT.parent
RUNTIME_ROOT = ROOT / "runtime"
PROJECTION_SCHEMA = "gsa-runtime-preflight-projection-v0.1.schema.json"
JOURNAL_RECEIPT_SCHEMA = "gsa-runtime-journal-receipt-v0.1.schema.json"
RUN_ID = "RUN-GSA-DISPOSABLE-REPRODUCTION-PREFLIGHT"
CREATED_AT = "2026-07-26T00:00:00Z"
NO_MODEL_DIGEST = sha256_bytes(canonical_bytes({"not_applicable": "no model"}))
NO_TOOL_DIGEST = sha256_bytes(canonical_bytes({"not_applicable": "no tools"}))
EXPECTED_EVENT_TYPES = ("run_preflight", "run_started", "run_finished")
MODEL_OR_TOOL_EVENT_TYPES = {
    "model_request",
    "model_output",
    "tool_proposal",
    "permission_decision",
    "tool_started",
    "tool_completed",
}


def _validate_runtime_contract(value: dict[str, Any], schema_name: str, *, label: str) -> None:
    schema_path = RUNTIME_ROOT / schema_name
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda item: list(item.absolute_path),
    )
    if not errors:
        return
    rendered = [
        f"{label}#/{'/'.join(map(str, item.absolute_path))}: {item.message}"
        for item in errors
    ]
    raise AssuranceError("; ".join(rendered))


def _event(
    *,
    sequence: int,
    event_type: str,
    payload: dict[str, Any],
    run_manifest_sha256: str,
    previous_event_sha256: str | None,
) -> dict[str, Any]:
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": RUN_ID,
        "event_id": f"EVT-GSA-DISPOSABLE-PREFLIGHT-{sequence:03d}",
        "sequence": sequence,
        "timestamp": CREATED_AT,
        "event_type": event_type,
        "run_manifest_sha256": run_manifest_sha256,
        "previous_event_sha256": previous_event_sha256,
        "payload_schema": "gsa-runtime-preflight-projection-v0.1.schema.json#runtime_event_payload",
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": "metadata_only",
        "event_sha256": "",
    }
    event["event_sha256"] = sha256_bytes(
        canonical_bytes(
            {
                key: value
                for key, value in event.items()
                if key != "event_sha256"
            }
        )
    )
    _validate_runtime_contract(event, "run-event-v0.1.schema.json", label="runtime event")
    return event


def _event_hash(event: dict[str, Any]) -> str:
    return sha256_bytes(
        canonical_bytes(
            {
                key: value
                for key, value in event.items()
                if key != "event_sha256"
            }
        )
    )


def _journal_bytes(events: list[dict[str, Any]]) -> bytes:
    return b"".join(
        json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
        for event in events
    )


def _load_journal_events(journal_path: Path) -> list[dict[str, Any]]:
    try:
        raw_lines = journal_path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise AssuranceError(f"cannot read runtime journal {journal_path}: {exc}") from exc
    events: list[dict[str, Any]] = []
    for line_number, line in enumerate(raw_lines, 1):
        if not line:
            raise AssuranceError(f"runtime journal has empty line: {line_number}")
        try:
            event = json.loads(line)
        except json.JSONDecodeError as exc:
            raise AssuranceError(
                f"runtime journal line {line_number} is not JSON: {exc}"
            ) from exc
        if not isinstance(event, dict):
            raise AssuranceError(f"runtime journal line {line_number} is not an object")
        events.append(event)
    return events


def _verify_projected_runtime_events(projection: dict[str, Any]) -> None:
    run_manifest = projection["runtime_manifest"]
    run_manifest_sha256 = sha256_bytes(canonical_bytes(run_manifest))
    if projection["run_manifest_sha256"] != run_manifest_sha256:
        raise AssuranceError("runtime manifest digest mismatch")
    if run_manifest["run_id"] != RUN_ID:
        raise AssuranceError("runtime manifest run_id mismatch")

    events = projection["runtime_events"]
    if len(events) != len(EXPECTED_EVENT_TYPES):
        raise AssuranceError("runtime event count mismatch")
    previous_event_sha256: str | None = None
    for sequence, event in enumerate(events):
        _validate_runtime_contract(
            event,
            "run-event-v0.1.schema.json",
            label=f"runtime event {sequence}",
        )
        if event["sequence"] != sequence:
            raise AssuranceError("runtime event sequence mismatch")
        if event["event_type"] != EXPECTED_EVENT_TYPES[sequence]:
            raise AssuranceError("runtime event type mismatch")
        if event["event_type"] in MODEL_OR_TOOL_EVENT_TYPES:
            raise AssuranceError("model/tool runtime event is forbidden")
        if event["run_id"] != RUN_ID:
            raise AssuranceError("runtime event run_id mismatch")
        if event["run_manifest_sha256"] != run_manifest_sha256:
            raise AssuranceError("runtime event manifest digest mismatch")
        if event["previous_event_sha256"] != previous_event_sha256:
            raise AssuranceError("runtime event hash chain mismatch")
        if event["payload_sha256"] != sha256_bytes(canonical_bytes(event["payload"])):
            raise AssuranceError("runtime event payload digest mismatch")
        if event["event_sha256"] != _event_hash(event):
            raise AssuranceError("runtime event digest mismatch")
        if event["redaction"] != "metadata_only":
            raise AssuranceError("runtime event redaction mismatch")
        previous_event_sha256 = event["event_sha256"]
    if events[-1]["payload"].get("formal_runner_claimed") is not False:
        raise AssuranceError("formal runner claim is forbidden")


def _runtime_manifest(proof: dict[str, Any], proof_sha256: str) -> dict[str, Any]:
    source_bundle = next(
        item for item in proof["inputs"] if item["role"] == "source_bundle"
    )
    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "immutable_precommit",
        "run_id": RUN_ID,
        "created_at": CREATED_AT,
        "execution_mode": "development",
        "scenario_export": {
            "export_id": "EXP-GSA-DISPOSABLE-REPRODUCTION-PREFLIGHT",
            "manifest_sha256": proof["manifest_sha256"],
            "bundle_sha256": source_bundle["sha256"],
            "leak_scan_report_sha256": proof_sha256,
        },
        "adapter": {
            "provider": "none",
            "model_id": "no-model",
            "adapter_id": "gsa-disposable-reproduction-preflight",
            "adapter_version": "0.1.0",
            "capabilities": {
                "structured_output": True,
                "tool_calling": False,
                "streaming": False,
                "seed_control": False,
                "cancellation": False,
                "context_limit_tokens": 1,
            },
        },
        "prompts": {
            "system_prompt_sha256": NO_MODEL_DIGEST,
            "project_rules_sha256": NO_MODEL_DIGEST,
            "generated_context_sha256": proof_sha256,
        },
        "tools": {
            "allowlist": [],
            "policy_sha256": NO_TOOL_DIGEST,
            "permission_mode": "deny_by_default",
            "filesystem_profile": "custom",
            "network_profile": "disabled",
        },
        "isolation": {
            "memory": "disabled",
            "historical_case_retrieval": "disabled",
            "oracle_mounted": False,
            "reviewer_fixtures_mounted": False,
            "inherited_user_config": False,
            "writable_roots": ["disposable-output-root"],
        },
        "budgets": {
            "wall_time_seconds": 1,
            "max_turns": 1,
            "max_input_tokens": 1,
            "max_output_tokens": 1,
            "max_tool_calls": 0,
        },
        "randomness": {
            "seed": None,
            "temperature": 0,
            "retry_policy": "none",
        },
        "protocol_digests": {
            "agent_protocol": sha256_file(ROOT / "protocol" / "PROTOCOL_DRAFT_v0.1.md"),
            "reason_codes": sha256_file(ROOT / "protocol" / "reason-codes-v0.1.yaml"),
            "gate_matrix": sha256_file(ROOT / "protocol" / "gate-matrix-v0.1.yaml"),
            "scoring_protocol": sha256_file(ROOT / "evaluation" / "SCORING_PROTOCOL_v0.1.md"),
            "output_schema": sha256_file(ASSURANCE_ROOT / "disposable-reproduction-run-proof-v0.1.schema.json"),
        },
        "journal_policy": {
            "format": "jsonl",
            "event_schema_version": "0.1.0-draft",
            "canonicalization": "RFC8785",
            "hash_chain": "sha256",
            "redaction_policy_sha256": sha256_bytes(
                canonical_bytes({"content_policy": "metadata_only"})
            ),
        },
        "notes": [
            "Development-only projection from GSA disposable reproduction run-proof.",
            "No model, tool, network, external code, scoring, or holdout execution is claimed.",
        ],
    }
    _validate_runtime_contract(manifest, "run-manifest-v0.1.schema.json", label="runtime manifest")
    return manifest


def _runtime_events(
    proof: dict[str, Any],
    *,
    run_manifest_sha256: str,
) -> list[dict[str, Any]]:
    first = _event(
        sequence=0,
        event_type="run_preflight",
        previous_event_sha256=None,
        run_manifest_sha256=run_manifest_sha256,
        payload={
            "reproduction_id": proof["reproduction_id"],
            "proof_sha256": sha256_bytes(canonical_bytes(proof)),
            "manifest_sha256": proof["manifest_sha256"],
            "code_file_count": len(proof["code"]["files"]),
            "input_count": len(proof["inputs"]),
        },
    )
    second = _event(
        sequence=1,
        event_type="run_started",
        previous_event_sha256=first["event_sha256"],
        run_manifest_sha256=run_manifest_sha256,
        payload={
            "reproduction_id": proof["reproduction_id"],
            "receipt_sha256": proof["receipt_sha256"],
            "output_count": len(proof["outputs"]),
            "model_invoked": False,
            "tool_invoked": False,
        },
    )
    third = _event(
        sequence=2,
        event_type="run_finished",
        previous_event_sha256=second["event_sha256"],
        run_manifest_sha256=run_manifest_sha256,
        payload={
            "reproduction_id": proof["reproduction_id"],
            "verification_sha256": proof["verification_sha256"],
            "valid": True,
            "formal_runner_claimed": False,
        },
    )
    return [first, second, third]


def build_gsa_runtime_preflight_projection(
    proof: dict[str, Any],
    receipt: dict[str, Any],
    *,
    manifest_root: Path,
    output_root: Path,
    manifest_name: str = DEFAULT_MANIFEST_NAME,
) -> dict[str, Any]:
    verify_disposable_reproduction_run_proof(
        proof,
        receipt,
        manifest_root=manifest_root,
        output_root=output_root,
        manifest_name=manifest_name,
    )
    proof_sha256 = sha256_bytes(canonical_bytes(proof))
    runtime_manifest = _runtime_manifest(proof, proof_sha256)
    run_manifest_sha256 = sha256_bytes(canonical_bytes(runtime_manifest))
    runtime_events = _runtime_events(
        proof,
        run_manifest_sha256=run_manifest_sha256,
    )
    projection = {
        "schema_version": "0.1.0-draft",
        "projection_kind": "gsa_disposable_reproduction_runtime_preflight_projection",
        "reproduction_id": proof["reproduction_id"],
        "proof_sha256": proof_sha256,
        "run_manifest_sha256": run_manifest_sha256,
        "runtime_manifest": runtime_manifest,
        "runtime_events": runtime_events,
        "checks": {
            "proof_verified": True,
            "runtime_manifest_schema_valid": True,
            "runtime_events_schema_valid": True,
            "hash_chain_valid": True,
            "metadata_only": True,
            "no_model_or_tool_events": True,
            "formal_runner_not_claimed": True,
        },
        "evidence_boundary": {
            "runner_status": "runtime_preflight_projection_only",
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
        },
        "valid": True,
    }
    validate_contract(
        projection,
        PROJECTION_SCHEMA,
        label="GSA runtime preflight projection",
    )
    _verify_projected_runtime_events(projection)
    return projection


def verify_gsa_runtime_preflight_projection(
    projection: dict[str, Any],
    proof: dict[str, Any],
    receipt: dict[str, Any],
    *,
    manifest_root: Path,
    output_root: Path,
    manifest_name: str = DEFAULT_MANIFEST_NAME,
) -> dict[str, Any]:
    validate_contract(
        projection,
        PROJECTION_SCHEMA,
        label="GSA runtime preflight projection",
    )
    expected = build_gsa_runtime_preflight_projection(
        proof,
        receipt,
        manifest_root=manifest_root,
        output_root=output_root,
        manifest_name=manifest_name,
    )
    if canonical_bytes(projection) != canonical_bytes(expected):
        raise AssuranceError("GSA runtime preflight projection mismatch")
    return {
        "schema_version": "0.1.0-draft",
        "verification_kind": "gsa_runtime_preflight_projection_verification",
        "reproduction_id": projection["reproduction_id"],
        "valid": True,
        "projection_sha256": sha256_bytes(canonical_bytes(projection)),
        "run_manifest_sha256": projection["run_manifest_sha256"],
        "runtime_event_count": len(projection["runtime_events"]),
        "evidence_boundary": projection["evidence_boundary"],
    }


def write_gsa_runtime_preflight_journal(
    projection: dict[str, Any],
    *,
    journal_path: Path,
) -> dict[str, Any]:
    validate_contract(
        projection,
        PROJECTION_SCHEMA,
        label="GSA runtime preflight projection",
    )
    _verify_projected_runtime_events(projection)
    events = projection["runtime_events"]
    encoded = _journal_bytes(events)
    exclusive_create_bytes(journal_path, encoded)
    journal_sha256 = sha256_bytes(encoded)
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "gsa_runtime_preflight_journal_receipt",
        "reproduction_id": projection["reproduction_id"],
        "projection_sha256": sha256_bytes(canonical_bytes(projection)),
        "run_id": projection["runtime_manifest"]["run_id"],
        "run_manifest_sha256": projection["run_manifest_sha256"],
        "journal_path": str(journal_path),
        "journal_sha256": journal_sha256,
        "event_count": len(events),
        "first_event_sha256": events[0]["event_sha256"],
        "last_event_sha256": events[-1]["event_sha256"],
        "terminal_event": events[-1]["event_type"],
        "checks": {
            "projection_schema_valid": True,
            "runtime_events_schema_valid": True,
            "hash_chain_valid": True,
            "journal_written_once": True,
            "journal_events_match_projection": True,
            "metadata_only": True,
            "no_model_or_tool_events": True,
            "formal_runner_not_claimed": True,
        },
        "evidence_boundary": {
            "runner_status": "runtime_journal_disposable_write_replay_only",
            "independence_effect": "no_new_independent_evidence",
            "claim_strength_effect": "no_claim_promotion",
        },
        "valid": True,
    }
    validate_contract(
        receipt,
        JOURNAL_RECEIPT_SCHEMA,
        label="GSA runtime preflight journal receipt",
    )
    return receipt


def verify_gsa_runtime_preflight_journal(
    receipt: dict[str, Any],
    projection: dict[str, Any],
    *,
    journal_path: Path,
) -> dict[str, Any]:
    validate_contract(
        receipt,
        JOURNAL_RECEIPT_SCHEMA,
        label="GSA runtime preflight journal receipt",
    )
    validate_contract(
        projection,
        PROJECTION_SCHEMA,
        label="GSA runtime preflight projection",
    )
    _verify_projected_runtime_events(projection)
    events = _load_journal_events(journal_path)
    if canonical_bytes(events) != canonical_bytes(projection["runtime_events"]):
        raise AssuranceError("runtime journal events do not match projection")
    encoded = journal_path.read_bytes()
    expected_journal_sha256 = sha256_bytes(_journal_bytes(events))
    if sha256_bytes(encoded) != expected_journal_sha256:
        raise AssuranceError("runtime journal serialization mismatch")

    expected = {
        "projection_sha256": sha256_bytes(canonical_bytes(projection)),
        "run_id": projection["runtime_manifest"]["run_id"],
        "run_manifest_sha256": projection["run_manifest_sha256"],
        "journal_path": str(journal_path),
        "journal_sha256": expected_journal_sha256,
        "event_count": len(events),
        "first_event_sha256": events[0]["event_sha256"],
        "last_event_sha256": events[-1]["event_sha256"],
        "terminal_event": events[-1]["event_type"],
    }
    for key, value in expected.items():
        if receipt[key] != value:
            raise AssuranceError(f"runtime journal receipt mismatch: {key}")
    return {
        "schema_version": "0.1.0-draft",
        "verification_kind": "gsa_runtime_preflight_journal_verification",
        "reproduction_id": projection["reproduction_id"],
        "valid": True,
        "journal_sha256": expected_journal_sha256,
        "verified_event_count": len(events),
        "terminal_event": events[-1]["event_type"],
        "evidence_boundary": receipt["evidence_boundary"],
    }
