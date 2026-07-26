from __future__ import annotations

import json
from pathlib import Path
import shutil
from typing import Any, Sequence

from jsonschema import Draft202012Validator, FormatChecker

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .source_visibility import evaluate_source_visibility_gate
from .utils import (
    atomic_write_json,
    canonical_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)


ROOT = ASSURANCE_ROOT.parent
RUNTIME_ROOT = ROOT / "runtime"
ANSWER_SCHEMA = "canonical-cli-answer-packet-v0.1.schema.json"
RECEIPT_SCHEMA = "canonical-cli-run-receipt-v0.1.schema.json"
DEFAULT_RUN_ID = "RUN-CANONICAL-CLI-FAKE-001"
DEFAULT_TASK_ID = "TASK-CANONICAL-CLI-FAKE-001"
TERMINAL_EVENTS = {"run_finished", "run_failed", "run_cancelled", "run_invalidated"}


def _validate_runtime_contract(value: dict[str, Any], schema_name: str, *, label: str) -> None:
    schema = load_json(RUNTIME_ROOT / schema_name)
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda item: list(item.absolute_path),
    )
    if errors:
        rendered = [
            f"{label}#/{'/'.join(map(str, item.absolute_path))}: {item.message}"
            for item in errors
        ]
        raise AssuranceError("; ".join(rendered))


def _event_hash(event: dict[str, Any]) -> str:
    projection = dict(event)
    projection.pop("event_sha256", None)
    return sha256_bytes(canonical_bytes(projection))


def _event_line(event: dict[str, Any]) -> bytes:
    return (
        json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
    )


def _digest_path(relative: str) -> str:
    return sha256_file(ROOT / relative)


def _zero_digest(label: str) -> str:
    return sha256_bytes(label.encode("utf-8"))


def build_canonical_cli_run_manifest(
    *,
    run_id: str = DEFAULT_RUN_ID,
    created_at: str | None = None,
) -> dict[str, Any]:
    created = created_at or utc_now()
    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "immutable_precommit",
        "run_id": run_id,
        "created_at": created,
        "execution_mode": "development",
        "scenario_export": {
            "export_id": "EXP-CANONICAL-CLI-FAKE-001",
            "manifest_sha256": _zero_digest("canonical-cli-fake-scenario-manifest"),
            "bundle_sha256": _zero_digest("canonical-cli-fake-scenario-bundle"),
            "leak_scan_report_sha256": _zero_digest("canonical-cli-fake-leak-scan"),
        },
        "adapter": {
            "provider": "fake-deepseek-shaped",
            "model_id": "deepseek-v4-pro",
            "adapter_id": "canonical-cli-fake-deepseek-adapter",
            "adapter_version": "0.1.0-draft",
            "capabilities": {
                "structured_output": True,
                "tool_calling": False,
                "streaming": False,
                "seed_control": False,
                "cancellation": False,
                "context_limit_tokens": 8000,
            },
        },
        "prompts": {
            "system_prompt_sha256": _zero_digest("canonical-cli-system-order"),
            "project_rules_sha256": _digest_path("README.md"),
            "generated_context_sha256": _zero_digest("source-gate-before-context"),
        },
        "tools": {
            "allowlist": [],
            "policy_sha256": _digest_path("assurance/source-visibility-ledger-v0.1.schema.json"),
            "permission_mode": "deny_by_default",
            "filesystem_profile": "workspace",
            "network_profile": "disabled",
        },
        "isolation": {
            "memory": "disabled",
            "historical_case_retrieval": "disabled",
            "oracle_mounted": False,
            "reviewer_fixtures_mounted": False,
            "inherited_user_config": False,
            "writable_roots": ["run_root"],
        },
        "budgets": {
            "wall_time_seconds": 60,
            "max_turns": 1,
            "max_input_tokens": 8000,
            "max_output_tokens": 2000,
            "max_tool_calls": 0,
        },
        "randomness": {
            "seed": None,
            "temperature": 0,
            "retry_policy": "none",
        },
        "protocol_digests": {
            "agent_protocol": _digest_path("protocol/PROTOCOL_DRAFT_v0.1.md"),
            "reason_codes": _digest_path("protocol/reason-codes-v0.1.yaml"),
            "gate_matrix": _digest_path("protocol/gate-matrix-v0.1.yaml"),
            "scoring_protocol": _digest_path("evaluation/SCORING_PROTOCOL_v0.1.md"),
            "output_schema": _digest_path(f"assurance/{ANSWER_SCHEMA}"),
        },
        "journal_policy": {
            "format": "jsonl",
            "event_schema_version": "0.1.0-draft",
            "canonicalization": "RFC8785",
            "hash_chain": "sha256",
            "redaction_policy_sha256": _digest_path(
                "docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md"
            ),
        },
        "notes": [
            "Canonical guarded CLI path uses fake offline adapter and no network.",
            "Source visibility gate must run before any model_request event.",
            "This manifest proves CLI orchestration shape, not model ability.",
        ],
    }
    _validate_runtime_contract(
        manifest,
        "run-manifest-v0.1.schema.json",
        label="canonical CLI run manifest",
    )
    return manifest


def _build_event(
    *,
    run_id: str,
    manifest_sha256: str,
    sequence: int,
    event_type: str,
    previous_event_sha256: str | None,
    payload_schema: str,
    payload: dict[str, Any],
    redaction: str,
    timestamp: str,
) -> dict[str, Any]:
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": run_id,
        "event_id": f"EVT-{run_id[4:]}-{sequence:03d}",
        "sequence": sequence,
        "timestamp": timestamp,
        "event_type": event_type,
        "run_manifest_sha256": manifest_sha256,
        "previous_event_sha256": previous_event_sha256,
        "payload_schema": payload_schema,
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": redaction,
        "event_sha256": "0" * 64,
    }
    event["event_sha256"] = _event_hash(event)
    _validate_runtime_contract(
        event,
        "run-event-v0.1.schema.json",
        label=f"canonical CLI event {sequence}",
    )
    return event


def _answer_strength(source_gate_receipt: dict[str, Any]) -> str:
    if source_gate_receipt["decision"] == "block":
        return "none"
    allowed = {
        item["claim_allowed"] for item in source_gate_receipt["reference_decisions"]
    }
    if "full_text_claim" in allowed and source_gate_receipt["decision"] == "allow":
        return "full_text_grounded_but_unvalidated"
    if "observed_fragment_only" in allowed:
        return "observed_fragment_only"
    return "metadata_only"


def build_fake_answer_packet(
    *,
    run_id: str,
    task_id: str,
    source_gate_receipt: dict[str, Any],
    source_gate_receipt_sha256: str | None = None,
) -> dict[str, Any]:
    gate_digest = source_gate_receipt_sha256 or sha256_bytes(
        canonical_bytes(source_gate_receipt)
    )
    deferred = [
        f"{item['ref_id']} requires {item['required_visibility']} before stronger use"
        for item in source_gate_receipt["reference_decisions"]
        if item["decision"] != "allow"
    ]
    packet = {
        "schema_version": "0.1.0-draft",
        "packet_kind": "canonical_guarded_cli_answer_packet",
        "run_id": run_id,
        "task_id": task_id,
        "adapter": {
            "adapter_id": "canonical-cli-fake-deepseek-adapter",
            "provider": "fake-deepseek-shaped",
            "model_id": "deepseek-v4-pro",
            "mode": "fake_offline",
            "real_network_used": False,
            "tool_calls_used": False,
        },
        "source_visibility_gate": {
            "receipt_sha256": gate_digest,
            "decision": source_gate_receipt["decision"],
            "must_report_visibility_status": True,
            "reference_decision_count": len(source_gate_receipt["reference_decisions"]),
        },
        "answer": {
            "summary": [
                "Offline fake adapter produced a guarded answer packet only after source visibility gate evaluation.",
                "The packet reports visibility status and does not upgrade deferred source claims.",
            ],
            "source_visibility_summary": [
                {
                    "ref_id": item["ref_id"],
                    "observed_visibility": item["observed_visibility"],
                    "decision": item["decision"],
                    "claim_allowed": item["claim_allowed"],
                }
                for item in source_gate_receipt["reference_decisions"]
            ],
            "deferred_claims": deferred,
        },
        "claim_boundaries": {
            "scientific_claim_strength": _answer_strength(source_gate_receipt),
            "fulltext_missing_blocks_mechanism_claims": True,
            "source_gate_decision_authoritative": True,
        },
        "next_actions": [
            "If a deferred mechanism/methods/comparison claim is needed, run another retrieval pass for full text.",
            "Do not treat this fake adapter packet as evidence of model scientific ability.",
        ],
        "limitations": [
            "No real DeepSeek request was made.",
            "No crawler, PDF parser, tool broker, scoring, or holdout evaluation was used.",
        ],
    }
    validate_contract(packet, ANSWER_SCHEMA, label="canonical CLI answer packet")
    return packet


def _write_journal(path: Path, events: Sequence[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        raise AssuranceError(f"refusing to overwrite existing journal: {path}")
    with path.open("wb") as handle:
        for event in events:
            handle.write(_event_line(event))


def _build_run_receipt(
    *,
    run_root: Path,
    run_manifest_path: Path,
    source_ledger_path: Path,
    source_gate_receipt_path: Path,
    answer_packet_path: Path,
    journal_path: Path,
    events: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "canonical_guarded_cli_run_receipt",
        "valid": True,
        "run_id": load_json(run_manifest_path)["run_id"],
        "run_root": str(run_root),
        "run_manifest_sha256": sha256_file(run_manifest_path),
        "source_ledger_sha256": sha256_file(source_ledger_path),
        "source_gate_receipt_sha256": sha256_file(source_gate_receipt_path),
        "answer_packet_sha256": sha256_file(answer_packet_path),
        "journal_sha256": sha256_file(journal_path),
        "event_count": len(events),
        "terminal_event": events[-1]["event_type"],
        "event_types": [event["event_type"] for event in events],
        "checks": {
            "manifest_valid": True,
            "source_gate_recomputed": True,
            "gate_before_model_request": True,
            "answer_packet_valid": True,
            "answer_binds_source_gate": True,
            "journal_hash_chain_valid": True,
            "terminal_exactly_once": True,
            "fake_adapter_no_network": True,
        },
        "limitations": [
            "This receipt verifies canonical CLI orchestration, not scientific correctness.",
            "The adapter is fake/offline and does not prove real DeepSeek behavior.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="canonical CLI run receipt")
    return receipt


def run_canonical_guarded_cli(
    *,
    run_root: Path,
    source_ledger_path: Path,
    run_id: str = DEFAULT_RUN_ID,
    task_id: str = DEFAULT_TASK_ID,
    created_at: str | None = None,
) -> dict[str, Any]:
    if run_root.exists() and any(run_root.iterdir()):
        raise AssuranceError(f"run root must be empty or absent: {run_root}")
    run_root.mkdir(parents=True, exist_ok=True)
    manifest = build_canonical_cli_run_manifest(run_id=run_id, created_at=created_at)
    run_manifest_path = run_root / "run-manifest.json"
    source_ledger_copy = run_root / "source-visibility-ledger.json"
    source_gate_receipt_path = run_root / "source-visibility-gate-receipt.json"
    answer_packet_path = run_root / "answer-packet.json"
    journal_path = run_root / "events.jsonl"
    receipt_path = run_root / "canonical-cli-run-receipt.json"

    atomic_write_json(run_manifest_path, manifest)
    shutil.copyfile(source_ledger_path, source_ledger_copy)
    source_ledger = load_json(source_ledger_copy)
    source_gate_receipt = evaluate_source_visibility_gate(source_ledger)
    atomic_write_json(source_gate_receipt_path, source_gate_receipt)
    source_gate_receipt_sha256 = sha256_file(source_gate_receipt_path)
    answer_packet = build_fake_answer_packet(
        run_id=run_id,
        task_id=task_id,
        source_gate_receipt=source_gate_receipt,
        source_gate_receipt_sha256=source_gate_receipt_sha256,
    )
    atomic_write_json(answer_packet_path, answer_packet)

    manifest_sha256 = sha256_file(run_manifest_path)
    answer_packet_sha256 = sha256_file(answer_packet_path)
    timestamp = created_at or manifest["created_at"]
    previous: str | None = None
    event_specs = [
        (
            "run_preflight",
            "canonical-cli-preflight-v0.1",
            {
                "adapter_id": manifest["adapter"]["adapter_id"],
                "provider": manifest["adapter"]["provider"],
                "model_id": manifest["adapter"]["model_id"],
                "real_network_allowed": False,
                "source_visibility_gate_required": True,
            },
            "metadata_only",
        ),
        (
            "run_started",
            "canonical-cli-run-started-v0.1",
            {"task_id": task_id, "run_root": str(run_root)},
            "metadata_only",
        ),
        (
            "gate_decision",
            "source-visibility-gate-receipt-v0.1",
            {
                "receipt_sha256": source_gate_receipt_sha256,
                "decision": source_gate_receipt["decision"],
                "reference_count": source_gate_receipt["reference_count"],
            },
            "metadata_only",
        ),
        (
            "model_request",
            "canonical-cli-fake-model-request-v0.1",
            {
                "provider": "fake-deepseek-shaped",
                "model_id": "deepseek-v4-pro",
                "message_order": [
                    "task",
                    "run_manifest_digest",
                    "source_visibility_gate_receipt_digest",
                    "allowed_claim_boundaries",
                    "redacted_context",
                ],
                "real_network_used": False,
                "tool_calls_allowed": False,
            },
            "metadata_only",
        ),
        (
            "model_output",
            "canonical-cli-fake-model-output-v0.1",
            {
                "answer_packet_sha256": answer_packet_sha256,
                "structured_output_valid": True,
                "raw_content_persisted": False,
                "real_network_used": False,
            },
            "metadata_only",
        ),
        (
            "artifact_registered",
            "canonical-cli-answer-packet-v0.1",
            {
                "artifact_path": "answer-packet.json",
                "artifact_sha256": answer_packet_sha256,
            },
            "metadata_only",
        ),
        (
            "run_finished",
            "canonical-cli-terminal-v0.1",
            {
                "status": "completed",
                "source_gate_decision": source_gate_receipt["decision"],
                "answer_packet_sha256": answer_packet_sha256,
            },
            "metadata_only",
        ),
    ]
    events: list[dict[str, Any]] = []
    for sequence, (event_type, payload_schema, payload, redaction) in enumerate(event_specs):
        event = _build_event(
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            sequence=sequence,
            event_type=event_type,
            previous_event_sha256=previous,
            payload_schema=payload_schema,
            payload=payload,
            redaction=redaction,
            timestamp=timestamp,
        )
        events.append(event)
        previous = event["event_sha256"]
    _write_journal(journal_path, events)
    receipt = _build_run_receipt(
        run_root=run_root,
        run_manifest_path=run_manifest_path,
        source_ledger_path=source_ledger_copy,
        source_gate_receipt_path=source_gate_receipt_path,
        answer_packet_path=answer_packet_path,
        journal_path=journal_path,
        events=events,
    )
    atomic_write_json(receipt_path, receipt)
    return receipt


def _read_journal(journal_path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line_number, line in enumerate(journal_path.read_text(encoding="utf-8").splitlines(), 1):
        if not line:
            raise AssuranceError(f"blank journal line {line_number}")
        event = json.loads(line)
        _validate_runtime_contract(
            event,
            "run-event-v0.1.schema.json",
            label=f"canonical CLI journal line {line_number}",
        )
        events.append(event)
    return events


def verify_canonical_guarded_cli_run(*, run_root: Path) -> dict[str, Any]:
    run_manifest_path = run_root / "run-manifest.json"
    source_ledger_path = run_root / "source-visibility-ledger.json"
    source_gate_receipt_path = run_root / "source-visibility-gate-receipt.json"
    answer_packet_path = run_root / "answer-packet.json"
    journal_path = run_root / "events.jsonl"

    manifest = load_json(run_manifest_path)
    _validate_runtime_contract(
        manifest,
        "run-manifest-v0.1.schema.json",
        label="canonical CLI run manifest",
    )
    source_gate_receipt = load_json(source_gate_receipt_path)
    recomputed_gate = evaluate_source_visibility_gate(load_json(source_ledger_path))
    if canonical_bytes(source_gate_receipt) != canonical_bytes(recomputed_gate):
        raise AssuranceError("source visibility gate receipt does not recompute")
    answer_packet = load_json(answer_packet_path)
    validate_contract(answer_packet, ANSWER_SCHEMA, label="canonical CLI answer packet")
    if answer_packet["source_visibility_gate"]["receipt_sha256"] != sha256_file(
        source_gate_receipt_path
    ):
        raise AssuranceError("answer packet does not bind source gate receipt")
    if answer_packet["adapter"]["real_network_used"]:
        raise AssuranceError("canonical fake adapter unexpectedly used network")

    events = _read_journal(journal_path)
    if not events:
        raise AssuranceError("canonical CLI journal is empty")
    expected_manifest_sha256 = sha256_file(run_manifest_path)
    previous: str | None = None
    terminal_count = 0
    for expected_sequence, event in enumerate(events):
        if event["sequence"] != expected_sequence:
            raise AssuranceError("journal sequence mismatch")
        if event["run_id"] != manifest["run_id"]:
            raise AssuranceError("journal run_id mismatch")
        if event["run_manifest_sha256"] != expected_manifest_sha256:
            raise AssuranceError("journal manifest digest mismatch")
        if event["previous_event_sha256"] != previous:
            raise AssuranceError("journal previous hash mismatch")
        if event["payload_sha256"] != sha256_bytes(canonical_bytes(event["payload"])):
            raise AssuranceError("journal payload digest mismatch")
        if event["event_sha256"] != _event_hash(event):
            raise AssuranceError("journal event digest mismatch")
        if event["event_type"] in TERMINAL_EVENTS:
            terminal_count += 1
            if expected_sequence != len(events) - 1:
                raise AssuranceError("terminal event is not last")
        previous = event["event_sha256"]
    if terminal_count != 1:
        raise AssuranceError("journal must contain exactly one terminal event")
    event_types = [event["event_type"] for event in events]
    if "model_request" in event_types:
        if "gate_decision" not in event_types:
            raise AssuranceError("model_request occurred without gate_decision")
        if event_types.index("gate_decision") > event_types.index("model_request"):
            raise AssuranceError("model_request occurred before gate_decision")
    else:
        raise AssuranceError("canonical CLI run lacks model_request boundary event")
    request_event = events[event_types.index("model_request")]
    if request_event["payload"].get("real_network_used") is not False:
        raise AssuranceError("model_request did not prove fake no-network mode")

    receipt = _build_run_receipt(
        run_root=run_root,
        run_manifest_path=run_manifest_path,
        source_ledger_path=source_ledger_path,
        source_gate_receipt_path=source_gate_receipt_path,
        answer_packet_path=answer_packet_path,
        journal_path=journal_path,
        events=events,
    )
    return receipt
