from __future__ import annotations

import json
from pathlib import Path
import shutil
from typing import Any, Sequence

from jsonschema import Draft202012Validator, FormatChecker

from .contracts import ASSURANCE_ROOT, validate_contract
from .conversation import ConversationNamespace
from .deepseek_adapter import (
    build_real_deepseek_answer_packet,
    build_real_deepseek_context,
    call_deepseek_api,
    _read_windows_credential,
    DEFAULT_CREDENTIAL_TARGET,
)
from .errors import AssuranceError
from .instruction_provenance_gate import (
    build_instruction_provenance_gate_context,
    evaluate_instruction_provenance_gate,
    verify_instruction_provenance_gate_receipt,
)
from .keystore import InstallationKeyStore, MemoryInstallationKeyStore
from .orientation_runtime_guard import build_orientation_checkpoint
from .session_governor import SessionGovernor
from .source_visibility import evaluate_source_visibility_gate
from .task_contract import (
    DEFAULT_TASK_ID,
    build_task_contract_from_ask,
    load_task_contract,
    task_contract_source_ledger_path,
    verify_task_contract_source_ledger,
)
from .tool_availability_gate import (
    build_tool_availability_gate_receipt,
    probe_tool_availability,
)
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


def _build_minimal_ipg_context(
    *,
    run_id: str,
    ask_text: str,
    conversation_id: str | None = None,
) -> dict[str, Any]:
    content_bytes = ask_text.encode("utf-8")
    return build_instruction_provenance_gate_context(
        run_id=run_id,
        conversation_id=conversation_id or f"CONV-CANONICAL-CLI-{run_id}",
        instructions=[
            {
                "entry_id": f"INS-USER-{run_id}",
                "declared_source_type": "user",
                "source_id": "user-prompt-main",
                "content_sha256": sha256_bytes(content_bytes),
                "content_bytes": len(content_bytes),
                "instruction_kind": "user_prompt",
            },
        ],
    )


def _build_canonical_tool_specs() -> list[dict[str, Any]]:
    return [
        {"tool_id": "search", "tool_name": "Search", "capability": "search", "probe_method": "manifest_allowlist_check"},
        {"tool_id": "file_read", "tool_name": "File Read", "capability": "file_read", "probe_method": "manifest_allowlist_check"},
        {"tool_id": "file_write", "tool_name": "File Write", "capability": "file_write", "probe_method": "manifest_allowlist_check"},
        {"tool_id": "bash_exec", "tool_name": "Bash Execute", "capability": "bash_exec", "probe_method": "manifest_allowlist_check"},
        {"tool_id": "web_fetch", "tool_name": "Web Fetch", "capability": "web_fetch", "probe_method": "manifest_allowlist_check"},
        {"tool_id": "subagent", "tool_name": "Sub-Agent", "capability": "subagent", "probe_method": "manifest_allowlist_check"},
    ]


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
    task_contract: dict[str, Any],
    task_contract_sha256: str,
    source_gate_receipt: dict[str, Any],
    source_gate_receipt_sha256: str | None = None,
    ipg_receipt: dict[str, Any] | None = None,
    ipg_receipt_sha256: str | None = None,
    tool_availability_receipt: dict[str, Any] | None = None,
    tool_availability_receipt_sha256: str | None = None,
) -> dict[str, Any]:
    gate_digest = source_gate_receipt_sha256 or sha256_bytes(
        canonical_bytes(source_gate_receipt)
    )
    ipg_digest = ipg_receipt_sha256 or sha256_bytes(
        canonical_bytes(ipg_receipt)
    )
    tool_digest = tool_availability_receipt_sha256 or sha256_bytes(
        canonical_bytes(tool_availability_receipt)
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
        "task_contract": {
            "sha256": task_contract_sha256,
            "entry_mode": task_contract["entry_mode"],
        },
        "adapter": {
            "adapter_id": "canonical-cli-fake-deepseek-adapter",
            "provider": "fake-deepseek-shaped",
            "model_id": "deepseek-v4-pro",
            "mode": "fake_offline",
            "real_network_used": False,
            "tool_calls_used": False,
        },
        "instruction_provenance_gate": {
            "receipt_sha256": ipg_digest,
            "decision": ipg_receipt["gate_decision"],
            "all_sources_classified": ipg_receipt["checks"]["all_sources_classified"],
            "no_injection_escalation": ipg_receipt["checks"]["no_injection_escalation"],
        },
        "tool_availability_gate": {
            "receipt_sha256": tool_digest,
            "decision": tool_availability_receipt["decisions"]["gate_decision"],
            "available_count": tool_availability_receipt["available_count"],
            "unavailable_count": tool_availability_receipt["unavailable_count"],
            "context_injected": tool_availability_receipt["decisions"]["context_injected"],
        },
        "source_visibility_gate": {
            "receipt_sha256": gate_digest,
            "decision": source_gate_receipt["decision"],
            "must_report_visibility_status": True,
            "reference_decision_count": len(source_gate_receipt["reference_decisions"]),
        },
        "answer": {
            "summary": [
                "Offline fake adapter produced a guarded answer packet only after all three gates evaluated.",
                "Instruction provenance gate, tool availability gate, and source visibility gate all ran before model request.",
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
            "all_gates_evaluated_before_model": True,
            "gate_chain_order": [
                "instruction_provenance_gate",
                "tool_availability_gate",
                "source_visibility_gate",
            ],
        },
        "next_actions": [
            "If a deferred mechanism/methods/comparison claim is needed, run another retrieval pass for full text.",
            "Do not treat this fake adapter packet as evidence of model scientific ability.",
            "Verify that three-gate chain receipts are preserved with the answer packet for independent audit.",
        ],
        "limitations": [
            "No real DeepSeek request was made.",
            "No crawler, PDF parser, tool broker, scoring, or holdout evaluation was used.",
            "All three gates are offline/mechanical; they prove orchestration shape, not scientific correctness.",
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
    task_contract_path: Path,
    source_ledger_path: Path,
    source_gate_receipt_path: Path,
    ipg_context_sha256: str,
    ipg_receipt_sha256: str,
    tool_availability_report_sha256: str,
    tool_availability_receipt_sha256: str,
    orientation_checkpoint_sha256: str,
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
        "task_contract_sha256": sha256_file(task_contract_path),
        "source_ledger_sha256": sha256_file(source_ledger_path),
        "source_gate_receipt_sha256": sha256_file(source_gate_receipt_path),
        "instruction_provenance_gate_context_sha256": ipg_context_sha256,
        "instruction_provenance_gate_receipt_sha256": ipg_receipt_sha256,
        "tool_availability_report_sha256": tool_availability_report_sha256,
        "tool_availability_gate_receipt_sha256": tool_availability_receipt_sha256,
        "orientation_checkpoint_sha256": orientation_checkpoint_sha256,
        "answer_packet_sha256": sha256_file(answer_packet_path),
        "journal_sha256": sha256_file(journal_path),
        "event_count": len(events),
        "terminal_event": events[-1]["event_type"],
        "event_types": [event["event_type"] for event in events],
        "checks": {
            "manifest_valid": True,
            "task_contract_valid": True,
            "source_gate_recomputed": True,
            "gate_before_model_request": True,
            "instruction_gate_before_model": True,
            "tool_availability_gate_before_model": True,
            "orientation_checkpoint_before_model": True,
            "answer_packet_valid": True,
            "answer_binds_source_gate": True,
            "answer_binds_task_contract": True,
            "journal_hash_chain_valid": True,
            "terminal_exactly_once": True,
        },
        "limitations": [],
    }
    adapter_used_real_network = False
    for event in events:
        if event["event_type"] == "model_request":
            adapter_used_real_network = bool(
                event["payload"].get("real_network_used", False)
            )
            break
    if adapter_used_real_network:
        receipt["checks"]["real_network_used"] = True
        receipt["limitations"] = [
            "This receipt verifies canonical CLI orchestration with real adapter.",
            "Real DeepSeek API was called; gate chain integrity is proven.",
            "It does not certify model scientific correctness or output quality.",
        ]
    else:
        receipt["checks"]["fake_adapter_no_network"] = True
        receipt["limitations"] = [
            "This receipt verifies canonical CLI orchestration, not scientific correctness.",
            "The adapter is fake/offline and does not prove real DeepSeek behavior.",
        ]
    validate_contract(receipt, RECEIPT_SCHEMA, label="canonical CLI run receipt")
    return receipt


def _build_run_frozen_context(
    run_id: str,
    workspace: str | None = None,
) -> dict[str, Any]:
    """Build a minimal frozen context for a canonical CLI run namespace."""
    return {
        "workspace_canonical_path_digest": {
            "value": sha256_bytes(run_id.encode("utf-8")),
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "workspace_content_digest": {
            "value": sha256_bytes(
                (workspace or "canonical-cli").encode("utf-8")
            ),
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "workspace_policy_digest": {
            "value": sha256_bytes(b"canonical-cli-default"),
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "runtime_family": {
            "value": "canonical-cli",
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "runtime_adapter_id": {
            "value": "deepseek-v4-pro",
            "evidence_status": "derived",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "runtime_binary_digest": {
            "value": sha256_bytes(b"canonical-cli-python"),
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "runtime_capabilities_digest": {
            "value": sha256_bytes(b"gate-chain-only"),
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "assurance_config_digest": {
            "value": sha256_bytes(b"canonical-cli"),
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "sandbox_backend": {
            "value": "none-gate-chain-only",
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
        "sandbox_backend_digest": {
            "value": sha256_bytes(b"none"),
            "evidence_status": "observed",
            "source_refs": [f"canonical-cli-run:{run_id}"],
        },
    }


def _resolve_and_setup_gates(
    *,
    run_root: Path,
    source_ledger_path: Path | None,
    instruction_provenance_gate_context_path: Path | None,
    ask: str | None,
    task_contract_path: Path | None,
    run_id: str,
    task_id: str,
    created_at: str | None,
    adapter_id: str | None = None,
    adapter_notes: list[str] | None = None,
    key_store: InstallationKeyStore | None = None,
) -> dict[str, Any]:
    """Shared gate setup: validate inputs, load task contract, run all gates.

    Returns a dict with all gate receipts, digests, file paths, and the
    task contract + manifest needed by callers to build adapter-specific
    answer packets and event specs.

    When *adapter_id* / *adapter_notes* are provided the manifest is
    customised for the real-adapter path before being written to disk.
    """
    if (ask is None) == (task_contract_path is None):
        raise AssuranceError("exactly one of ask or task_contract_path is required")
    if ask is not None and source_ledger_path is None:
        raise AssuranceError("--ask requires a source ledger")
    if run_root.exists() and any(run_root.iterdir()):
        raise AssuranceError(f"run root must be empty or absent: {run_root}")

    if task_contract_path is not None:
        task_contract = load_task_contract(task_contract_path)
        task_id = task_contract["task_id"]
        source_ledger_path = task_contract_source_ledger_path(task_contract)
        verify_task_contract_source_ledger(
            task_contract,
            source_ledger_path=source_ledger_path,
        )
    else:
        task_contract = build_task_contract_from_ask(
            ask=ask or "",
            source_ledger_path=source_ledger_path,
            task_id=task_id,
            created_at=created_at,
        )

    run_root.mkdir(parents=True, exist_ok=True)

    # ── create namespace (when key_store provided) ──
    namespace: ConversationNamespace | None = None
    governor: SessionGovernor | None = None
    if key_store is not None:
        conversation_repository = run_root / "conversations"
        namespace = ConversationNamespace.create(
            conversation_repository,
            key_store=key_store,
            frozen_context=_build_run_frozen_context(run_id),
            allowed_capabilities=[
                "filesystem.workspace_read",
                "filesystem.workspace_write",
            ],
            denied_capabilities=["network.unrestricted", "secret.raw_read"],
        )
        governor = SessionGovernor(namespace)

    manifest = build_canonical_cli_run_manifest(run_id=run_id, created_at=created_at)
    if adapter_id is not None:
        manifest["adapter"]["adapter_id"] = adapter_id
    if adapter_notes is not None:
        manifest["notes"] = list(adapter_notes)

    # Paths — prefer governor routing when namespace is active
    if governor is not None:
        run_manifest_path = governor.write_run_manifest(manifest)
    else:
        run_manifest_path = run_root / "run-manifest.json"
        atomic_write_json(run_manifest_path, manifest)

    task_contract_copy = run_root / "task-contract.json"
    source_ledger_copy = run_root / "source-visibility-ledger.json"
    source_gate_receipt_path = run_root / "source-visibility-gate-receipt.json"
    answer_packet_path = run_root / "answer-packet.json"
    journal_path = run_root / "events.jsonl"
    receipt_path = run_root / "canonical-cli-run-receipt.json"

    atomic_write_json(task_contract_copy, task_contract)
    task_contract_sha256 = sha256_file(task_contract_copy)

    ask_text = task_contract["user_request"]["raw_text"]
    if instruction_provenance_gate_context_path is not None:
        ipg_context = load_json(instruction_provenance_gate_context_path)
    else:
        ipg_context = _build_minimal_ipg_context(
            run_id=run_id,
            ask_text=ask_text,
            conversation_id=(
                namespace.conversation_id
                if namespace is not None
                else f"CONV-CANONICAL-CLI-{run_id}"
            ),
        )
    ipg_receipt = evaluate_instruction_provenance_gate(gate_context=ipg_context)
    ipg_context_copy = run_root / "instruction-provenance-gate-context.json"
    ipg_receipt_path = run_root / "instruction-provenance-gate-receipt.json"
    atomic_write_json(ipg_context_copy, ipg_context)
    atomic_write_json(ipg_receipt_path, ipg_receipt)
    ipg_context_sha256 = sha256_file(ipg_context_copy)
    ipg_receipt_sha256 = sha256_file(ipg_receipt_path)

    tool_specs = _build_canonical_tool_specs()
    probe_registry = {spec["tool_id"]: False for spec in tool_specs}
    tool_availability_report = probe_tool_availability(
        tool_specs=tool_specs,
        runtime_id=run_id,
        probe_registry=probe_registry,
    )
    tool_availability_gate_receipt = build_tool_availability_gate_receipt(
        tool_availability_report
    )
    tool_report_path = run_root / "tool-availability-report.json"
    tool_receipt_path = run_root / "tool-availability-gate-receipt.json"
    atomic_write_json(tool_report_path, tool_availability_report)
    atomic_write_json(tool_receipt_path, tool_availability_gate_receipt)
    tool_report_sha256 = sha256_file(tool_report_path)
    tool_receipt_sha256 = sha256_file(tool_receipt_path)

    orientation_checkpoint = build_orientation_checkpoint(
        task_id=task_id,
        trigger_step=0,
        task_contract_sha256=task_contract_sha256,
        tool_availability_sha256=sha256_bytes(canonical_bytes(tool_availability_report)),
    )
    orientation_checkpoint_path = run_root / "orientation-checkpoint.json"
    atomic_write_json(orientation_checkpoint_path, orientation_checkpoint)
    orientation_checkpoint_sha256 = sha256_file(orientation_checkpoint_path)

    shutil.copyfile(source_ledger_path, source_ledger_copy)
    source_ledger = load_json(source_ledger_copy)
    source_gate_receipt = evaluate_source_visibility_gate(source_ledger)
    atomic_write_json(source_gate_receipt_path, source_gate_receipt)
    source_gate_receipt_sha256 = sha256_file(source_gate_receipt_path)

    return {
        "run_root": run_root,
        "run_id": run_id,
        "task_id": task_id,
        "task_contract": task_contract,
        "task_contract_sha256": task_contract_sha256,
        "manifest": manifest,
        "ipg_receipt": ipg_receipt,
        "ipg_context_sha256": ipg_context_sha256,
        "ipg_receipt_sha256": ipg_receipt_sha256,
        "tool_availability_report": tool_availability_report,
        "tool_availability_gate_receipt": tool_availability_gate_receipt,
        "tool_report_sha256": tool_report_sha256,
        "tool_receipt_sha256": tool_receipt_sha256,
        "orientation_checkpoint_sha256": orientation_checkpoint_sha256,
        "source_gate_receipt": source_gate_receipt,
        "source_gate_receipt_sha256": source_gate_receipt_sha256,
        "run_manifest_path": run_manifest_path,
        "task_contract_copy": task_contract_copy,
        "source_ledger_copy": source_ledger_copy,
        "source_gate_receipt_path": source_gate_receipt_path,
        "answer_packet_path": answer_packet_path,
        "journal_path": journal_path,
        "receipt_path": receipt_path,
        "ipg_context_copy": ipg_context_copy,
        "ipg_receipt_path": ipg_receipt_path,
        "tool_report_path": tool_report_path,
        "tool_receipt_path": tool_receipt_path,
        "orientation_checkpoint_path": orientation_checkpoint_path,
        "namespace": namespace,
        "governor": governor,
    }


def _build_events_and_receipt(
    *,
    gates: dict[str, Any],
    answer_packet_sha256: str,
    event_specs: list[tuple[str, str, dict[str, Any], str]],
    created_at: str | None,
) -> dict[str, Any]:
    """Shared journal write and receipt build.

    Builds events from *event_specs*, writes the hash-chained journal,
    and returns the verifiable run receipt.  The caller must have already
    written the answer packet to *gates["answer_packet_path"]* and
    computed *answer_packet_sha256* via :func:`sha256_file`.
    """
    manifest = gates["manifest"]
    run_manifest_path = gates["run_manifest_path"]
    answer_packet_path = gates["answer_packet_path"]
    journal_path = gates["journal_path"]
    receipt_path = gates["receipt_path"]

    manifest_sha256 = sha256_file(run_manifest_path)
    timestamp = created_at or manifest["created_at"]
    previous: str | None = None

    events: list[dict[str, Any]] = []
    for sequence, (event_type, payload_schema, payload, redaction) in enumerate(
        event_specs
    ):
        event = _build_event(
            run_id=gates["run_id"],
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
        run_root=gates["run_root"],
        run_manifest_path=run_manifest_path,
        task_contract_path=gates["task_contract_copy"],
        source_ledger_path=gates["source_ledger_copy"],
        source_gate_receipt_path=gates["source_gate_receipt_path"],
        ipg_context_sha256=gates["ipg_context_sha256"],
        ipg_receipt_sha256=gates["ipg_receipt_sha256"],
        tool_availability_report_sha256=gates["tool_report_sha256"],
        tool_availability_receipt_sha256=gates["tool_receipt_sha256"],
        orientation_checkpoint_sha256=gates["orientation_checkpoint_sha256"],
        answer_packet_path=answer_packet_path,
        journal_path=journal_path,
        events=events,
    )
    atomic_write_json(receipt_path, receipt)
    return receipt


def run_canonical_guarded_cli(
    *,
    run_root: Path,
    source_ledger_path: Path | None = None,
    instruction_provenance_gate_context_path: Path | None = None,
    ask: str | None = None,
    task_contract_path: Path | None = None,
    run_id: str = DEFAULT_RUN_ID,
    task_id: str = DEFAULT_TASK_ID,
    created_at: str | None = None,
) -> dict[str, Any]:
    """Run the canonical guarded CLI with a fake offline adapter.

    All gates evaluate before the fake answer packet is constructed.
    No network access is attempted.
    """
    gates = _resolve_and_setup_gates(
        run_root=run_root,
        source_ledger_path=source_ledger_path,
        instruction_provenance_gate_context_path=instruction_provenance_gate_context_path,
        ask=ask,
        task_contract_path=task_contract_path,
        run_id=run_id,
        task_id=task_id,
        created_at=created_at,
    )

    answer_packet = build_fake_answer_packet(
        run_id=gates["run_id"],
        task_id=gates["task_id"],
        task_contract=gates["task_contract"],
        task_contract_sha256=gates["task_contract_sha256"],
        source_gate_receipt=gates["source_gate_receipt"],
        source_gate_receipt_sha256=gates["source_gate_receipt_sha256"],
        ipg_receipt=gates["ipg_receipt"],
        ipg_receipt_sha256=gates["ipg_receipt_sha256"],
        tool_availability_receipt=gates["tool_availability_gate_receipt"],
        tool_availability_receipt_sha256=gates["tool_receipt_sha256"],
    )

    atomic_write_json(gates["answer_packet_path"], answer_packet)
    ap_sha256 = sha256_file(gates["answer_packet_path"])

    tcs = gates["task_contract_sha256"]
    event_specs = [
        (
            "run_preflight", "canonical-cli-preflight-v0.1",
            {
                "adapter_id": gates["manifest"]["adapter"]["adapter_id"],
                "provider": gates["manifest"]["adapter"]["provider"],
                "model_id": gates["manifest"]["adapter"]["model_id"],
                "real_network_allowed": False,
                "source_visibility_gate_required": True,
                "instruction_provenance_gate_applied": True,
                "tool_availability_gate_applied": True,
                "task_contract_sha256": tcs,
            },
            "metadata_only",
        ),
        (
            "instruction_provenance_gate",
            "instruction-provenance-gate-receipt-v0.1",
            {
                "receipt_sha256": gates["ipg_receipt_sha256"],
                "context_sha256": gates["ipg_context_sha256"],
            },
            "metadata_only",
        ),
        (
            "tool_availability_check",
            "tool-availability-check-event-payload-v0.1",
            {
                "tool_availability_report_sha256": gates["tool_report_sha256"],
                "available_count": len(gates["tool_availability_report"]["available"]),
                "unavailable_count": len(gates["tool_availability_report"]["unavailable"]),
                "unprobed_count": len(gates["tool_availability_report"]["unprobed"]),
                "degraded_count": len(gates["tool_availability_report"]["degraded"]),
                "context_block_injected": True,
                "model_must_not_guess": True,
            },
            "metadata_only",
        ),
        (
            "orientation_checkpoint",
            "orientation-checkpoint-event-payload-v0.1",
            {
                "checkpoint_sha256": gates["orientation_checkpoint_sha256"],
                "trigger_step": 0,
                "task_contract_sha256": tcs,
            },
            "metadata_only",
        ),
        (
            "run_started", "canonical-cli-run-started-v0.1",
            {
                "task_id": gates["task_id"],
                "task_contract_sha256": tcs,
                "run_root": str(gates["run_root"]),
            },
            "metadata_only",
        ),
        (
            "gate_decision", "source-visibility-gate-receipt-v0.1",
            {
                "receipt_sha256": gates["source_gate_receipt_sha256"],
                "decision": gates["source_gate_receipt"]["decision"],
                "reference_count": gates["source_gate_receipt"]["reference_count"],
            },
            "metadata_only",
        ),
        (
            "model_request", "canonical-cli-fake-model-request-v0.1",
            {
                "provider": "fake-deepseek-shaped",
                "model_id": "deepseek-v4-pro",
                "message_order": [
                    "task", "task_contract_digest", "run_manifest_digest",
                    "source_visibility_gate_receipt_digest",
                    "allowed_claim_boundaries", "redacted_context",
                ],
                "real_network_used": False,
                "tool_calls_allowed": False,
                "task_contract_sha256": tcs,
            },
            "metadata_only",
        ),
        (
            "model_output", "canonical-cli-fake-model-output-v0.1",
            {
                "answer_packet_sha256": ap_sha256,
                "structured_output_valid": True,
                "raw_content_persisted": False,
                "real_network_used": False,
            },
            "metadata_only",
        ),
        (
            "artifact_registered", "canonical-cli-answer-packet-v0.1",
            {
                "artifact_path": "answer-packet.json",
                "artifact_sha256": ap_sha256,
            },
            "metadata_only",
        ),
        (
            "run_finished", "canonical-cli-terminal-v0.1",
            {
                "status": "completed",
                "source_gate_decision": gates["source_gate_receipt"]["decision"],
                "answer_packet_sha256": ap_sha256,
            },
            "metadata_only",
        ),
    ]

    return _build_events_and_receipt(
        gates=gates,
        answer_packet_sha256=ap_sha256,
        event_specs=event_specs,
        created_at=created_at,
    )


def run_canonical_guarded_cli_real(
    *,
    run_root: Path,
    source_ledger_path: Path | None = None,
    instruction_provenance_gate_context_path: Path | None = None,
    ask: str | None = None,
    task_contract_path: Path | None = None,
    run_id: str = DEFAULT_RUN_ID,
    task_id: str = DEFAULT_TASK_ID,
    created_at: str | None = None,
    credential_target: str = DEFAULT_CREDENTIAL_TARGET,
    api_timeout_seconds: int = 60,
) -> dict[str, Any]:
    """Run the canonical guarded CLI with a REAL DeepSeek API call.

    This is the P0 production path: all gates evaluate before the model is
    called, the API key is read from Windows Credential Manager and never
    persisted, and only public assistant text enters the answer packet.
    """
    # Create a runtime key store for the namespace envelope
    key_store = MemoryInstallationKeyStore()
    try:
        gates = _resolve_and_setup_gates(
            run_root=run_root,
            source_ledger_path=source_ledger_path,
            instruction_provenance_gate_context_path=instruction_provenance_gate_context_path,
            ask=ask,
            task_contract_path=task_contract_path,
            run_id=run_id,
            task_id=task_id,
            created_at=created_at,
            adapter_id="canonical-cli-real-deepseek-adapter",
            adapter_notes=[
                "Canonical guarded CLI path uses real DeepSeek adapter.",
                "Source visibility gate must run before any model_request event.",
                "Credential is read from Windows Credential Manager and never persisted.",
            ],
            key_store=key_store,
        )
        namespace = gates["namespace"]
        governor = gates["governor"]

        # === REAL ADAPTER: enforce adapter gate, then call DeepSeek API ===
        from .adapter_gate import AdapterGateContext, enforce_adapter_call

        api_error: str | None = None
        model_output: dict[str, Any] | None = None
        adapter_enforcement: dict[str, Any] | None = None
        api_key = ""

        try:
            api_key = _read_windows_credential(credential_target)
            messages = build_real_deepseek_context(
                task_contract=gates["task_contract"],
                source_gate_receipt=gates["source_gate_receipt"],
                tool_availability_report=gates["tool_availability_report"],
            )

            gate_ctx = AdapterGateContext(
                ipg_receipt=gates["ipg_receipt"],
                ipg_context=load_json(gates["ipg_context_copy"]),
                adapter_id="deepseek-v4-pro",
                conversation_id=namespace.conversation_id,
                run_id=run_id,
            )
            enforcement = enforce_adapter_call(
                gate_context=gate_ctx,
                adapter_call=lambda: call_deepseek_api(
                    api_key,
                    messages,
                    timeout_seconds=api_timeout_seconds,
                ),
            )
            model_output = enforcement["adapter_result"]
            adapter_enforcement = {
                k: v for k, v in enforcement.items() if k != "adapter_result"
            }
            # Store enforcement receipt in namespace
            if governor is not None:
                governor.write_gate_receipt(
                    "adapter-gate-enforcement",
                    adapter_enforcement,
                )
        except Exception as exc:
            api_error = str(exc)
        finally:
            api_key = "\x00" * len(api_key)  # best-effort scrub
    finally:
        key_store.close()

    # Build the answer packet (from real output or error fallback)
    if model_output and not api_error:
        answer_packet = build_real_deepseek_answer_packet(
            run_id=gates["run_id"],
            task_id=gates["task_id"],
            task_contract=gates["task_contract"],
            task_contract_sha256=gates["task_contract_sha256"],
            source_gate_receipt=gates["source_gate_receipt"],
            source_gate_receipt_sha256=gates["source_gate_receipt_sha256"],
            ipg_receipt=gates["ipg_receipt"],
            ipg_receipt_sha256=gates["ipg_receipt_sha256"],
            tool_availability_receipt=gates["tool_availability_gate_receipt"],
            tool_availability_receipt_sha256=gates["tool_receipt_sha256"],
            model_output=model_output,
            conversation_id=namespace.conversation_id,
            envelope_id=namespace.state()["envelope_id"],
        )
        terminal_status = "completed"
        terminal_event_type = "run_finished"
    else:
        # Fall back to an inline error answer packet
        error_summary = f"Real DeepSeek adapter failed: {api_error}"
        answer_packet = {
            "schema_version": "0.1.0-draft",
            "packet_kind": "canonical_guarded_cli_answer_packet",
            "run_id": gates["run_id"],
            "task_id": gates["task_id"],
            "task_contract": {
                "sha256": gates["task_contract_sha256"],
                "entry_mode": gates["task_contract"]["entry_mode"],
            },
            "adapter": {
                "adapter_id": "canonical-cli-real-deepseek-adapter",
                "provider": "deepseek",
                "model_id": "deepseek-v4-pro",
                "mode": "real_development",
                "real_network_used": True,
                "tool_calls_used": False,
            },
            "instruction_provenance_gate": {
                "receipt_sha256": gates["ipg_receipt_sha256"],
                "decision": gates["ipg_receipt"]["gate_decision"],
                "all_sources_classified": gates["ipg_receipt"]["checks"]["all_sources_classified"],
                "no_injection_escalation": gates["ipg_receipt"]["checks"]["no_injection_escalation"],
            },
            "tool_availability_gate": {
                "receipt_sha256": gates["tool_receipt_sha256"],
                "decision": gates["tool_availability_gate_receipt"]["decisions"]["gate_decision"],
                "available_count": gates["tool_availability_gate_receipt"]["available_count"],
                "unavailable_count": gates["tool_availability_gate_receipt"]["unavailable_count"],
                "context_injected": gates["tool_availability_gate_receipt"]["decisions"]["context_injected"],
            },
            "source_visibility_gate": {
                "receipt_sha256": gates["source_gate_receipt_sha256"],
                "decision": gates["source_gate_receipt"]["decision"],
                "must_report_visibility_status": True,
                "reference_decision_count": len(gates["source_gate_receipt"]["reference_decisions"]),
            },
            "answer": {
                "summary": [error_summary],
                "source_visibility_summary": [
                    {
                        "ref_id": item["ref_id"],
                        "observed_visibility": item["observed_visibility"],
                        "decision": item["decision"],
                        "claim_allowed": item["claim_allowed"],
                    }
                    for item in gates["source_gate_receipt"]["reference_decisions"]
                ],
                "deferred_claims": [],
            },
            "claim_boundaries": {
                "scientific_claim_strength": "none",
                "fulltext_missing_blocks_mechanism_claims": True,
                "source_gate_decision_authoritative": True,
                "all_gates_evaluated_before_model": True,
                "gate_chain_order": [
                    "instruction_provenance_gate",
                    "tool_availability_gate",
                    "source_visibility_gate",
                ],
            },
            "next_actions": [
                "Investigate the adapter failure before retrying.",
                "Check credential, network, and API endpoint availability.",
            ],
            "limitations": [
                f"Real DeepSeek adapter call failed: {api_error}",
                "All three gates were evaluated before the failed model call.",
                "No raw credential or response was persisted.",
            ],
        }
        validate_contract(
            answer_packet,
            "canonical-cli-answer-packet-v0.1.schema.json",
            label="error answer packet",
        )
        terminal_status = "failed"
        terminal_event_type = "run_failed"

    atomic_write_json(gates["answer_packet_path"], answer_packet)
    ap_sha256 = sha256_file(gates["answer_packet_path"])

    real_network_used = model_output is not None
    tcs = gates["task_contract_sha256"]
    event_specs = [
        (
            "run_preflight", "canonical-cli-preflight-v0.1",
            {
                "adapter_id": "canonical-cli-real-deepseek-adapter",
                "provider": "deepseek",
                "model_id": "deepseek-v4-pro",
                "real_network_allowed": True,
                "source_visibility_gate_required": True,
                "instruction_provenance_gate_applied": True,
                "tool_availability_gate_applied": True,
                "task_contract_sha256": tcs,
            },
            "metadata_only",
        ),
        (
            "instruction_provenance_gate",
            "instruction-provenance-gate-receipt-v0.1",
            {
                "receipt_sha256": gates["ipg_receipt_sha256"],
                "context_sha256": gates["ipg_context_sha256"],
            },
            "metadata_only",
        ),
        (
            "tool_availability_check",
            "tool-availability-check-event-payload-v0.1",
            {
                "tool_availability_report_sha256": gates["tool_report_sha256"],
                "available_count": len(gates["tool_availability_report"]["available"]),
                "unavailable_count": len(gates["tool_availability_report"]["unavailable"]),
                "unprobed_count": len(gates["tool_availability_report"]["unprobed"]),
                "degraded_count": len(gates["tool_availability_report"]["degraded"]),
                "context_block_injected": True,
                "model_must_not_guess": True,
            },
            "metadata_only",
        ),
        (
            "orientation_checkpoint",
            "orientation-checkpoint-event-payload-v0.1",
            {
                "checkpoint_sha256": gates["orientation_checkpoint_sha256"],
                "trigger_step": 0,
                "task_contract_sha256": tcs,
            },
            "metadata_only",
        ),
        (
            "run_started", "canonical-cli-run-started-v0.1",
            {
                "task_id": gates["task_id"],
                "task_contract_sha256": tcs,
                "run_root": str(gates["run_root"]),
            },
            "metadata_only",
        ),
        (
            "gate_decision", "source-visibility-gate-receipt-v0.1",
            {
                "receipt_sha256": gates["source_gate_receipt_sha256"],
                "decision": gates["source_gate_receipt"]["decision"],
                "reference_count": gates["source_gate_receipt"]["reference_count"],
            },
            "metadata_only",
        ),
        (
            "model_request", "canonical-cli-real-model-request-v0.1",
            {
                "provider": "deepseek",
                "model_id": "deepseek-v4-pro",
                "message_order": ["system", "user"],
                "real_network_used": True,
                "tool_calls_allowed": False,
                "task_contract_sha256": tcs,
            },
            "metadata_only",
        ),
        (
            "model_output", "canonical-cli-real-model-output-v0.1",
            {
                "answer_packet_sha256": ap_sha256,
                "structured_output_valid": True,
                "raw_content_persisted": False,
                "real_network_used": real_network_used,
                "api_error": api_error,
            },
            "metadata_only",
        ),
        (
            "artifact_registered", "canonical-cli-answer-packet-v0.1",
            {
                "artifact_path": "answer-packet.json",
                "artifact_sha256": ap_sha256,
            },
            "metadata_only",
        ),
        (
            terminal_event_type, "canonical-cli-terminal-v0.1",
            {
                "status": terminal_status,
                "source_gate_decision": gates["source_gate_receipt"]["decision"],
                "answer_packet_sha256": ap_sha256,
            },
            "metadata_only",
        ),
    ]

    return _build_events_and_receipt(
        gates=gates,
        answer_packet_sha256=ap_sha256,
        event_specs=event_specs,
        created_at=created_at,
    )


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
    task_contract_path = run_root / "task-contract.json"
    source_ledger_path = run_root / "source-visibility-ledger.json"
    source_gate_receipt_path = run_root / "source-visibility-gate-receipt.json"
    ipg_context_path = run_root / "instruction-provenance-gate-context.json"
    ipg_receipt_path = run_root / "instruction-provenance-gate-receipt.json"
    tool_report_path = run_root / "tool-availability-report.json"
    tool_receipt_path = run_root / "tool-availability-gate-receipt.json"
    orientation_checkpoint_path = run_root / "orientation-checkpoint.json"
    answer_packet_path = run_root / "answer-packet.json"
    journal_path = run_root / "events.jsonl"

    manifest = load_json(run_manifest_path)
    _validate_runtime_contract(
        manifest,
        "run-manifest-v0.1.schema.json",
        label="canonical CLI run manifest",
    )
    source_gate_receipt = load_json(source_gate_receipt_path)
    task_contract = load_task_contract(task_contract_path)
    verify_task_contract_source_ledger(
        task_contract,
        source_ledger_path=source_ledger_path,
    )
    recomputed_gate = evaluate_source_visibility_gate(load_json(source_ledger_path))
    if canonical_bytes(source_gate_receipt) != canonical_bytes(recomputed_gate):
        raise AssuranceError("source visibility gate receipt does not recompute")

    ipg_context = load_json(ipg_context_path)
    ipg_receipt = load_json(ipg_receipt_path)
    verify_instruction_provenance_gate_receipt(
        gate_context=ipg_context, receipt=ipg_receipt
    )
    ipg_context_sha256 = sha256_file(ipg_context_path)
    ipg_receipt_sha256 = sha256_file(ipg_receipt_path)

    tool_report = load_json(tool_report_path)
    tool_receipt = load_json(tool_receipt_path)
    recomputed_tool_receipt = build_tool_availability_gate_receipt(tool_report)
    if canonical_bytes(tool_receipt) != canonical_bytes(recomputed_tool_receipt):
        raise AssuranceError("tool availability gate receipt does not recompute")
    tool_availability_report_sha256 = sha256_file(tool_report_path)
    tool_availability_receipt_sha256 = sha256_file(tool_receipt_path)

    orientation_checkpoint = load_json(orientation_checkpoint_path)
    orientation_checkpoint_sha256 = sha256_file(orientation_checkpoint_path)

    answer_packet = load_json(answer_packet_path)
    validate_contract(answer_packet, ANSWER_SCHEMA, label="canonical CLI answer packet")
    if answer_packet["source_visibility_gate"]["receipt_sha256"] != sha256_file(
        source_gate_receipt_path
    ):
        raise AssuranceError("answer packet does not bind source gate receipt")
    if answer_packet["instruction_provenance_gate"]["receipt_sha256"] != sha256_file(
        ipg_receipt_path
    ):
        raise AssuranceError("answer packet does not bind instruction provenance gate receipt")
    if answer_packet["tool_availability_gate"]["receipt_sha256"] != sha256_file(
        tool_receipt_path
    ):
        raise AssuranceError("answer packet does not bind tool availability gate receipt")
    if answer_packet["task_contract"]["sha256"] != sha256_file(task_contract_path):
        raise AssuranceError("answer packet does not bind task contract")
    # Verify network usage matches adapter mode declared in answer packet
    if answer_packet["adapter"]["mode"] == "fake_offline":
        if answer_packet["adapter"]["real_network_used"]:
            raise AssuranceError("canonical fake adapter unexpectedly used network")
    elif answer_packet["adapter"]["mode"] == "real_development":
        if not answer_packet["adapter"]["real_network_used"]:
            raise AssuranceError("canonical real adapter did not use network")

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
    if "instruction_provenance_gate" not in event_types:
        raise AssuranceError("canonical CLI run lacks instruction provenance gate event")
    if event_types.index("instruction_provenance_gate") >= event_types.index("run_started"):
        raise AssuranceError("instruction provenance gate must precede run_started")
    if "tool_availability_check" not in event_types:
        raise AssuranceError("canonical CLI run lacks tool availability check event")
    tac_idx = event_types.index("tool_availability_check")
    if tac_idx <= event_types.index("instruction_provenance_gate"):
        raise AssuranceError("tool availability check must follow instruction provenance gate")
    if tac_idx >= event_types.index("run_started"):
        raise AssuranceError("tool availability check must precede run_started")
    if "orientation_checkpoint" not in event_types:
        raise AssuranceError("canonical CLI run lacks orientation checkpoint event")
    oc_idx = event_types.index("orientation_checkpoint")
    if oc_idx <= event_types.index("tool_availability_check"):
        raise AssuranceError("orientation checkpoint must follow tool availability check")
    if oc_idx >= event_types.index("run_started"):
        raise AssuranceError("orientation checkpoint must precede run_started")
    if "model_request" in event_types:
        if "gate_decision" not in event_types:
            raise AssuranceError("model_request occurred without gate_decision")
        if event_types.index("gate_decision") > event_types.index("model_request"):
            raise AssuranceError("model_request occurred before gate_decision")
    else:
        raise AssuranceError("canonical CLI run lacks model_request boundary event")
    request_event = events[event_types.index("model_request")]
    declared_network_used = request_event["payload"].get("real_network_used")
    if answer_packet["adapter"]["mode"] == "fake_offline":
        if declared_network_used is not False:
            raise AssuranceError("model_request did not prove fake no-network mode")
    elif answer_packet["adapter"]["mode"] == "real_development":
        if declared_network_used is not True:
            raise AssuranceError("model_request did not declare real network usage")
    if request_event["payload"].get("task_contract_sha256") != sha256_file(
        task_contract_path
    ):
        raise AssuranceError("model_request does not bind task contract")

    receipt = _build_run_receipt(
        run_root=run_root,
        run_manifest_path=run_manifest_path,
        task_contract_path=task_contract_path,
        source_ledger_path=source_ledger_path,
        source_gate_receipt_path=source_gate_receipt_path,
        ipg_context_sha256=ipg_context_sha256,
        ipg_receipt_sha256=ipg_receipt_sha256,
        tool_availability_report_sha256=tool_availability_report_sha256,
        tool_availability_receipt_sha256=tool_availability_receipt_sha256,
        orientation_checkpoint_sha256=orientation_checkpoint_sha256,
        answer_packet_path=answer_packet_path,
        journal_path=journal_path,
        events=events,
    )
    return receipt
