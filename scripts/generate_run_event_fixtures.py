"""Generate the run-event reference-spec fixture tree.

Phase 3 slice #14 (Python reference-spec): this script is the single source
of truth for the good/bad fixture shapes under
`runtime/fixtures/run-event-v0.1/` and `assurance/fixtures/canonical_cli/`.
Re-run it after any payload schema change to regenerate the tree (the
check_repository gate asserts every generated file is covered by a mapping).

Fixture conventions (see `runtime/fixtures/run-event-v0.1/README.md`):
- `<slug>.minimal.valid.json` — a legal payload for the event type (the
  required fields plus representative optional fields where they matter,
  e.g. a non-empty tool_calls entry for model_output).
- `<slug>.constraint.invalid.json` — violates exactly one constraint of the
  payload schema (fail-closed direction: never a merely-missing required
  field when a sharper constraint exists).
- envelope samples use legal dummy hashes (all-zero lowercase 64-hex).

Slice #17 (conformance suite) changes:
- Three events (orientation_checkpoint / tool_availability_check /
  runtime_stagnation_guard) carry RUST-TRACK payload shapes here — the
  `runtime/` schemas of the same basenames are the Rust-track authority;
  the `assurance/` twins remain the orientation-track shapes (dual-track
  adjudication, contract §6).
- Hash-chain integrity is NOT validated by this generator's fixtures — it is
  verified against real captured journals by
  `assurance/run_event_journal_validation.py` (journals/ directory).
"""

from __future__ import annotations

import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

ZERO_HASH = "0" * 64
DUMMY_HASH = "1" * 64
TIMESTAMP = "2026-08-06T00:00:00Z"

# 33 event types in run-event-v0.1.schema.json enum order.
EVENT_TYPES = [
    "run_preflight",
    "run_started",
    "prompt_submitted",
    "model_request",
    "model_response_received",
    "model_output",
    "acp_initialize",
    "acp_session_created",
    "tool_proposal",
    "permission_requested",
    "permission_decision",
    "tool_started",
    "tool_completed",
    "orientation_checkpoint",
    "runtime_stagnation_guard",
    "tool_availability_check",
    "tool_belief_stagnation",
    "instruction_provenance_gate",
    "gate_decision",
    "neutral_inquiry",
    "counterexample_gate",
    "retrieval_completion_check",
    "snapshot_created",
    "snapshot_restored",
    "artifact_registered",
    "plan_proposed",
    "plan_approved",
    "plan_rejected",
    "action_approved",
    "run_finished",
    "run_failed",
    "run_cancelled",
    "run_invalidated",
]

SLUGS = {
    "run_preflight": "run-preflight",
    "run_started": "run-started",
    "prompt_submitted": "prompt-submitted",
    "model_request": "model-request",
    "model_response_received": "model-response-received",
    "model_output": "model-output",
    "acp_initialize": "acp-initialize",
    "acp_session_created": "acp-session-created",
    "tool_proposal": "tool-proposal",
    "permission_requested": "permission-requested",
    "permission_decision": "permission-decision",
    "tool_started": "tool-started",
    "tool_completed": "tool-completed",
    "orientation_checkpoint": "orientation-checkpoint",
    "runtime_stagnation_guard": "runtime-stagnation-guard",
    "tool_availability_check": "tool-availability-check",
    "tool_belief_stagnation": "tool-belief-stagnation",
    "instruction_provenance_gate": "instruction-provenance-gate",
    "gate_decision": "gate-decision",
    "neutral_inquiry": "neutral-inquiry",
    "counterexample_gate": "counterexample-gate",
    "retrieval_completion_check": "retrieval-completion-check",
    "snapshot_created": "snapshot-created",
    "snapshot_restored": "snapshot-restored",
    "artifact_registered": "artifact-registered",
    "plan_proposed": "plan-proposed",
    "plan_approved": "plan-approved",
    "plan_rejected": "plan-rejected",
    "action_approved": "action-approved",
    "run_finished": "run-finished",
    "run_failed": "run-failed",
    "run_cancelled": "run-cancelled",
    "run_invalidated": "run-invalidated",
}

_ZERO_COUNTERS = {
    "output_repeats": 0,
    "tool_calls": 0,
    "actions": 0,
    "rounds": 0,
}

# Minimal legal payload per event type. Shape authority: the Rust production
# construction points for the 24 slice-#14 schemas (controller.rs / session.rs
# / main.rs); the pre-existing payload schemas for the other nine.
PAYLOAD_GOOD: dict[str, dict] = {
    "run_preflight": {
        "run_id": "RUN-CONF-0001",
        "created_at": TIMESTAMP,
        "schema_version": "0.1.0-draft",
    },
    "run_started": {"prompt": "hello"},
    "prompt_submitted": {"prompt": "hello", "character_count": 5},
    "model_request": {
        "provider": "deepseek",
        "model_id": "deepseek-v4-pro",
        "message_order": ["system", "user"],
        "real_network_used": True,
        "tool_calls_allowed": False,
        "task_contract_sha256": ZERO_HASH,
    },
    "model_response_received": {
        "tool_calls": [
            {"name": "read_file", "arguments": {"path": "a.txt"}, "call_id": "call-1"}
        ],
        "response_sha256": ZERO_HASH,
        "response_summary_sha256": ZERO_HASH,
        "finish_reason": "tool_calls",
        "token_count": 10,
        "output_format": "text",
    },
    "model_output": {
        "text": "ok",
        "tool_calls": [
            {"name": "read_file", "arguments": {"path": "a.txt"}, "call_id": "call-1"}
        ],
        "finish_reason": "tool_calls",
    },
    "acp_initialize": {"protocol_version": 1},
    "acp_session_created": {"session_id": "sess-1"},
    "tool_proposal": {"tool": "bash", "call_id": "call-1", "input_summary": "ls"},
    "permission_requested": {"tool": "bash", "risk": "SandboxEscape", "call_id": "call-1"},
    "permission_decision": {"tool": "bash", "decision": "deny"},
    "tool_started": {"tool": "read_file", "call_id": "call-1"},
    "tool_completed": {"tool": "read_file", "call_id": "call-1", "exit_code": 0},
    # Rust-track shapes (slice #17 dual-track adjudication): these three
    # events' payloads are the shapes the Rust loop actually constructs
    # (controller.rs) — the assurance/ schemas of the same basenames remain
    # the orientation-track shapes, validated by orientation_runtime_journal.
    "orientation_checkpoint": {
        "checkpoint_id": "ORIENT-RUN-CONF-0001-0000",
        "trigger": "fixed_step_interval",
        "step_index": 0,
        "message_block": "[ORIENTATION v0.1] 当前正在做什么？",
    },
    "runtime_stagnation_guard": {
        "decision": "continue",
        "reason_codes": [],
        "max_consecutive_repeated_content": 0,
        "max_ngram_repeat": 0,
    },
    "tool_availability_check": {
        "available": ["read_file", "grep"],
        "unavailable": [],
        "degraded": [],
        "unprobed": [],
        "gate_decision": "pass",
    },
    "tool_belief_stagnation": {
        "tool_belief_stagnation_receipt_sha256": ZERO_HASH,
        "decision": "continue",
        "reason_codes": [],
        "mismatch_count": 0,
        "public_output_only": True,
        "asks_model_if_stuck": False,
        "hidden_chain_of_thought_saved": False,
    },
    "instruction_provenance_gate": {"decision": "pass", "entries": 1},
    "gate_decision": {
        "gate": "instruction_provenance_gate",
        "decision": "block",
        "tools": ["bash"],
    },
    "neutral_inquiry": {
        "trigger_reason": "rounds",
        "counters": {
            "main": dict(_ZERO_COUNTERS, rounds=9),
            "internal": dict(_ZERO_COUNTERS),
            "external": dict(_ZERO_COUNTERS),
        },
        "message_block": "[INFO_SUFFICIENCY v0.1] 当前进展是否已覆盖完成主任务所需内容？",
        "block_present": True,
    },
    "counterexample_gate": {
        "position": "final_answer",
        "message_block": "[COUNTEREXAMPLE_GATE v0.1] 反例检查",
        "once_only": True,
    },
    "retrieval_completion_check": {
        "role": "internal_retrieval",
        "tool": "search",
        "decision": "yes",
        "response": "found",
        "neutral_only": True,
        "new_subagent_requested": False,
        "global_review_requested": False,
        "claim_strength_effect": "none",
    },
    "snapshot_created": {"tool": "edit_file", "targets": ["lib.rs"], "snapshot_hash": ZERO_HASH},
    "snapshot_restored": {"snapshot_hash": ZERO_HASH, "restored": ["lib.rs"]},
    "artifact_registered": {"artifact_path": "answer-packet.json", "artifact_sha256": ZERO_HASH},
    "plan_proposed": {
        "plan_id": "PLAN-RUN-CONF-0001",
        "task_id": "TASK-RUN-CONF-0001",
        "sections": 4,
    },
    "plan_approved": {
        "plan_id": "PLAN-RUN-CONF-0001",
        "authority": "user",
        "decision": "approve",
        "execution_policy": "manual",
    },
    "plan_rejected": {"plan_id": "PLAN-RUN-CONF-0001", "authority": "user", "reason": "revise"},
    "action_approved": {"action_id": "ACT-1", "plan_id": "PLAN-RUN-CONF-0001"},
    "run_finished": {"status": "completed", "turn_count": 1, "tool_rounds": 0},
    "run_failed": {"error": "transport error"},
    "run_cancelled": {"reason": "user_cancelled"},
    "run_invalidated": {"status": "restart_requested", "turn_count": 1, "tool_rounds": 0},
}

# One constraint violation per event type (never a bare missing-required when
# a sharper constraint exists).
PAYLOAD_BAD: dict[str, dict] = {
    "run_preflight": {"run_id": "RUN-CONF-0001", "created_at": TIMESTAMP, "schema_version": "0.2.0"},
    "run_started": {"prompt": 123},
    "prompt_submitted": {"prompt": "hello", "character_count": -1},
    "model_request": {
        "model_id": "deepseek-v4-pro",
        "message_order": ["system"],
        "real_network_used": True,
        "tool_calls_allowed": False,
        "task_contract_sha256": ZERO_HASH,
    },
    "model_response_received": {
        "tool_calls": [{"name": "read_file", "arguments": 123, "call_id": "call-1"}]
    },
    "model_output": {
        "text": "ok",
        "tool_calls": [{"name": "read_file", "arguments": 123, "call_id": "call-1"}],
        "finish_reason": "stop",
    },
    "acp_initialize": {"protocol_version": None},
    "acp_session_created": {"session_id": "sess-1", "extra": 1},
    "tool_proposal": {"tool": "bash"},
    "permission_requested": {"tool": "bash", "risk": "Medium", "call_id": "call-1"},
    "permission_decision": {"tool": "bash", "decision": "maybe"},
    "tool_started": {"tool": "read_file"},
    "tool_completed": {"tool": "read_file", "call_id": "call-1", "exit_code": 0, "extra": 1},
    "orientation_checkpoint": {
        "checkpoint_id": "ORIENT-RUN-CONF-0001-0000",
        "trigger": "manual",
        "step_index": 0,
        "message_block": "[ORIENTATION v0.1] 当前正在做什么？",
    },
    "runtime_stagnation_guard": {
        "decision": "stop",
        "reason_codes": [],
        "max_consecutive_repeated_content": 0,
        "max_ngram_repeat": 0,
    },
    "tool_availability_check": {
        "available": ["read_file", "grep"],
        "unavailable": [],
        "degraded": [],
        "unprobed": [],
        "gate_decision": "stop",
    },
    "tool_belief_stagnation": {
        "tool_belief_stagnation_receipt_sha256": ZERO_HASH,
        "decision": "continue",
        "reason_codes": [],
        "mismatch_count": 0,
        "public_output_only": True,
        "asks_model_if_stuck": False,
        "hidden_chain_of_thought_saved": False,
        "extra": True,
    },
    "instruction_provenance_gate": {"decision": "pass", "entries": 0},
    "gate_decision": {"gate": "instruction_provenance_gate", "decision": "stop_x"},
    "neutral_inquiry": {
        "trigger_reason": "other",
        "counters": {
            "main": dict(_ZERO_COUNTERS),
            "internal": dict(_ZERO_COUNTERS),
            "external": dict(_ZERO_COUNTERS),
        },
        "message_block": "[INFO_SUFFICIENCY v0.1] x",
        "block_present": True,
    },
    "counterexample_gate": {
        "position": "mid_answer",
        "message_block": "[COUNTEREXAMPLE_GATE v0.1] x",
        "once_only": True,
    },
    "retrieval_completion_check": {
        "role": "internal_retrieval",
        "tool": "search",
        "decision": "maybe",
        "response": "found",
        "neutral_only": True,
        "new_subagent_requested": False,
        "global_review_requested": False,
        "claim_strength_effect": "none",
    },
    "snapshot_created": {
        "tool": "edit_file",
        "targets": ["lib.rs"],
        "snapshot_hash": ZERO_HASH,
        "snapshot_error": "boom",
    },
    "snapshot_restored": {"snapshot_hash": ZERO_HASH},
    "artifact_registered": {"artifact_path": ""},
    "plan_proposed": {"plan_id": "PLANX-1", "task_id": "TASK-RUN-CONF-0001", "sections": 4},
    "plan_approved": {
        "plan_id": "PLAN-RUN-CONF-0001",
        "authority": "user",
        "decision": "approved",
        "execution_policy": "manual",
    },
    "plan_rejected": {"plan_id": "X-1", "authority": "user", "reason": "revise"},
    "action_approved": {"plan_id": "PLAN-RUN-CONF-0001"},
    "run_finished": {"status": "failed"},
    "run_failed": {"error": ""},
    "run_cancelled": {"reason": "user"},
    "run_invalidated": {"status": "invalidated"},
}

# canonical_cli payload shapes (its own `canonical-cli-*` track). Shapes taken
# from canonical_cli.py event_specs (fake path L900-1009, real path L1322-1360).
CANONICAL_CLI_GOOD: dict[str, dict] = {
    "canonical-cli-preflight": {
        "adapter_id": "deepseek",
        "provider": "deepseek",
        "model_id": "deepseek-v4-pro",
        "real_network_allowed": False,
        "source_visibility_gate_required": True,
        "instruction_provenance_gate_applied": True,
        "tool_availability_gate_applied": True,
        "task_contract_sha256": ZERO_HASH,
    },
    "canonical-cli-run-started": {
        "task_id": "TASK1",
        "task_contract_sha256": ZERO_HASH,
        "run_root": "C:/work",
    },
    "canonical-cli-fake-model-request": {
        "provider": "fake-deepseek-shaped",
        "model_id": "deepseek-v4-pro",
        "message_order": ["task", "task_contract_digest"],
        "real_network_used": False,
        "tool_calls_allowed": False,
        "task_contract_sha256": ZERO_HASH,
    },
    "canonical-cli-real-model-request": {
        "provider": "deepseek",
        "model_id": "deepseek-v4-pro",
        "message_order": ["system", "user"],
        "real_network_used": True,
        "tool_calls_allowed": False,
        "task_contract_sha256": ZERO_HASH,
    },
    "canonical-cli-fake-model-output": {
        "answer_packet_sha256": ZERO_HASH,
        "structured_output_valid": True,
        "raw_content_persisted": False,
        "real_network_used": False,
    },
    "canonical-cli-real-model-output": {
        "answer_packet_sha256": ZERO_HASH,
        "structured_output_valid": True,
        "raw_content_persisted": False,
        "real_network_used": True,
        "api_error": None,
    },
    "canonical-cli-terminal": {
        "status": "completed",
        "source_gate_decision": "allow",
        "answer_packet_sha256": ZERO_HASH,
    },
}


def _envelope(event_type: str, payload: dict, sequence: int, previous: str | None) -> dict:
    return {
        "schema_version": "0.1.0-draft",
        "run_id": f"RUN-CONF-{SLUGS[event_type].upper()}",
        "event_id": f"EVT-CONF-{sequence:03d}",
        "sequence": sequence,
        "timestamp": TIMESTAMP,
        "event_type": event_type,
        "run_manifest_sha256": ZERO_HASH,
        "previous_event_sha256": previous,
        "payload_schema": "run-event-v0.1.schema.json",
        "payload": payload,
        "payload_sha256": ZERO_HASH,
        "redaction": "none",
        "event_sha256": ZERO_HASH,
    }


def _bad_envelope(mutate: dict) -> dict:
    return mutate(
        _envelope("run_preflight", {"run_id": "RUN-CONF-0001"}, 0, None)
    )


def _pop(e: dict, key: str) -> dict:
    e.pop(key, None)
    return e


ENVELOPE_BAD: dict[str, dict] = {
    "bad-schema-version": _bad_envelope(lambda e: e.update({"schema_version": "0.2.0"}) or e),
    "bad-run-id": _bad_envelope(lambda e: e.update({"run_id": "RAN-1"}) or e),
    "bad-event-id": _bad_envelope(lambda e: e.update({"event_id": "EVTX-000"}) or e),
    "missing-required-payload-schema": _bad_envelope(
        lambda e: _pop(e, "payload_schema")
    ),
    "negative-sequence": _bad_envelope(lambda e: e.update({"sequence": -1}) or e),
    "bad-sha256-pattern": _bad_envelope(
        lambda e: e.update({"run_manifest_sha256": "ABCD"}) or e
    ),
    "unknown-event-type": _bad_envelope(lambda e: e.update({"event_type": "run_foo"}) or e),
    "seq0-non-null-previous": _bad_envelope(
        lambda e: e.update({"previous_event_sha256": DUMMY_HASH}) or e
    ),
    "seq1-null-previous": _bad_envelope(
        lambda e: e.update({"sequence": 1, "previous_event_sha256": None}) or e
    ),
    "bad-redaction": _bad_envelope(lambda e: e.update({"redaction": "partial"}) or e),
    "non-object-payload": _bad_envelope(lambda e: e.update({"payload": ["x"]}) or e),
    "additional-properties": _bad_envelope(
        lambda e: e.update({"extra_field": 1}) or e
    ),
    # Note: jsonschema 4.26's date-time format check accepts arbitrary
    # strings (observed no-op), so a format-level timestamp violation cannot
    # be expressed as a reliable bad fixture — use a structural violation
    # (missing required) instead.
    "missing-timestamp": _bad_envelope(lambda e: _pop(e, "timestamp")),
}

FIXTURES_README = """# run-event-v0.1 reference fixtures

Reference-spec fixtures for the run-event envelope and its 33 event payload
schemas (generated by `scripts/generate_run_event_fixtures.py` — re-run that
script after any payload schema change).

Conventions:

- `payloads/<slug>.minimal.valid.json` — a legal payload for the event type
  (required fields plus representative optional fields where they matter,
  e.g. a non-empty tool_calls entry for model_output; validated against the
  matching `*-event-payload-v0.1.schema.json`).
- `payloads/<slug>.constraint.invalid.json` — violates exactly one constraint
  of the payload schema. Invalid samples never rely on a bare
  missing-required when a sharper constraint exists.
- `envelope/<slug>.valid.json` — full 13-field envelope with the event's
  minimal legal payload; `previous_event_sha256` is null (sequence 0).
  `chained-run-finished.valid.json` covers the sequence-1 branch.
- `envelope/<bad-name>.invalid.json` — one envelope constraint violation each.

Hashes: fixtures use legal dummy lowercase 64-hex values (all zeros).

`journals/*.jsonl` — REAL run journals captured from the Rust production
implementation (conformance suite, Phase 3 #7). They are NOT produced by this
generator (which only touches `payloads/`, `envelope/` and the canonical_cli
fixtures); re-capture via the orz `#[ignore]` conformance capture tests and
copy into this directory (see
`docs/CONFORMANCE_SUITE_SLICE_17_2026-08-06.md`). Every journal line is
validated (envelope schema + per-event payload schema selected by the
`payload_schema` track string + full hash-chain recompute) by
`assurance/run_event_journal_validation.py` — never compare against stored
hashes, recompute is the check. Re-captures will byte-differ (timestamps,
run ids) — that is expected and semantic-only.

Envelope `payload_schema` value: `"run-event-v0.1.schema.json"` — the Rust
production track records this string for every event (reference-spec
contract, see `architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md`).
"""


def write_json(path: Path, data: dict) -> None:
    path.write_text(
        json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def main() -> None:
    fixture_root = ROOT / "runtime/fixtures/run-event-v0.1"
    payloads_dir = fixture_root / "payloads"
    envelope_dir = fixture_root / "envelope"
    canonical_cli_dir = ROOT / "assurance/fixtures/canonical_cli"
    for directory in (payloads_dir, envelope_dir, canonical_cli_dir):
        shutil.rmtree(directory, ignore_errors=True)
        directory.mkdir(parents=True, exist_ok=True)

    for event_type in EVENT_TYPES:
        slug = SLUGS[event_type]
        write_json(payloads_dir / f"{slug}.minimal.valid.json", PAYLOAD_GOOD[event_type])
        write_json(payloads_dir / f"{slug}.constraint.invalid.json", PAYLOAD_BAD[event_type])
        write_json(
            envelope_dir / f"{slug}.valid.json",
            _envelope(event_type, PAYLOAD_GOOD[event_type], 0, None),
        )
    write_json(
        envelope_dir / "chained-run-finished.valid.json",
        _envelope("run_finished", PAYLOAD_GOOD["run_finished"], 1, DUMMY_HASH),
    )
    for name, payload in ENVELOPE_BAD.items():
        write_json(envelope_dir / f"{name}.invalid.json", payload)

    for name, payload in CANONICAL_CLI_GOOD.items():
        write_json(canonical_cli_dir / f"{name}.minimal.valid.json", payload)

    fixture_root.joinpath("README.md").write_text(FIXTURES_README, encoding="utf-8")

    print(
        f"payloads: {len(PAYLOAD_GOOD) * 2} files, "
        f"envelope: {len(EVENT_TYPES) + 1 + len(ENVELOPE_BAD)} files, "
        f"canonical_cli: {len(CANONICAL_CLI_GOOD)} files"
    )


if __name__ == "__main__":
    main()
