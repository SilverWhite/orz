"""Generate the run-event reference-spec fixture tree.

Phase 3 slice #14 (Python reference-spec): this script is the single source
of truth for the good/bad fixture shapes under
`runtime/fixtures/run-event-v0.1/`, `runtime/fixtures/run-event-v0.2/` and
`assurance/fixtures/canonical_cli/`. Re-run it after any payload schema change
to regenerate the tree (the check_repository gate asserts every generated
file is covered by a mapping).

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

Phase B v0.2 (2026-08-09, ADR-0010 §5.3.1/§11.2/§11.6) changes:
- The v0.2 tree (`runtime/fixtures/run-event-v0.2/`) covers the five v1.1
  mechanism events: orientation_checkpoint (v0.2 payload) plus the four new
  event types. neutral_inquiry / retrieval_completion_check are retired from
  the v0.2 envelope enum (v0.1 replay only).
- The remaining 31 events reuse the v0.1 payload schema files unchanged
  (payload shapes did not change; adjudicated in the v0.2 fixture README).
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
    "context_compressed",
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

# v0.2 event system (Phase B, ADR-0010 §11.2): v0.1 enum minus the two
# retired events (neutral_inquiry, retrieval_completion_check) plus the four
# new mechanism events. orientation_checkpoint keeps its slot with a v0.2
# payload shape. Order mirrors run-event-v0.2.schema.json.
V02_EVENT_TYPES = [
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
    "diagnostic_coverage_checkpoint",
    "runtime_stagnation_guard",
    "tool_availability_check",
    "tool_belief_stagnation",
    "instruction_provenance_gate",
    "gate_decision",
    "counterexample_gate",
    "information_sufficiency_assessment",
    "retrieval_parent_disposition",
    "retrieval_close_record",
    "retrieval_mode_transition",
    "retrieval_result_committed",
    "retrieval_activation_restored",
    "control_ticket_issued",
    "control_ticket_consumed",
    "control_ticket_rejected",
    "context_compressed",
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

SLUGS_V02 = {
    "orientation_checkpoint": "orientation-checkpoint",
    "diagnostic_coverage_checkpoint": "diagnostic-coverage-checkpoint",
    "information_sufficiency_assessment": "information-sufficiency-assessment",
    "retrieval_parent_disposition": "retrieval-parent-disposition",
    "retrieval_close_record": "retrieval-close-record",
    "retrieval_mode_transition": "retrieval-mode-transition",
    "retrieval_result_committed": "retrieval-result",
    "retrieval_activation_restored": "retrieval-activation-restored",
    "control_ticket_issued": "control-ticket-issued",
    "control_ticket_consumed": "control-ticket-consumed",
    "control_ticket_rejected": "control-ticket-rejected",
}

# The v0.2 events with their own v0.2 payload schema (the rest of the v0.2
# envelope reuses the v0.1 payload schema files). GAP-RETRIEVAL-TOOLS
# (2026-08-10) adds the three retrieval events.
V02_PAYLOAD_EVENTS = [
    "orientation_checkpoint",
    "diagnostic_coverage_checkpoint",
    "information_sufficiency_assessment",
    "retrieval_parent_disposition",
    "retrieval_close_record",
    "retrieval_mode_transition",
    "retrieval_result_committed",
    "retrieval_activation_restored",
    # ACAF Slice 1 (设计文档 §4.2/§4.6) — control-ticket lifecycle events.
    "control_ticket_issued",
    "control_ticket_consumed",
    "control_ticket_rejected",
    # FUS-TOOL-PROBE (2026-08-13; P0-A-2): two-state single probe face
    # snapshot (work tools).
    "tool_availability_check",
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
    "context_compressed": "context-compressed",
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
        # D-6 usage observation (FIX_PLAN 2026-08-06) + F-06 incomplete
        # marker (2026-08-07 review): the Rust construction point emits
        # reasoning_tokens/completion_tokens on every model output; the
        # optional "incomplete" marks an aborted stream's partial output.
        "reasoning_tokens": 10,
        "completion_tokens": 42,
        "incomplete": False,
    },
    "acp_initialize": {"protocol_version": 1},
    "acp_session_created": {"session_id": "sess-1"},
    "tool_proposal": {"tool": "bash", "call_id": "call-1", "input_summary": "ls"},
    "permission_requested": {"tool": "bash", "risk": "SandboxEscape", "call_id": "call-1"},
    "permission_decision": {"tool": "bash", "decision": "deny"},
    "tool_started": {"tool": "read_file", "call_id": "call-1"},
    "tool_completed": {
        "tool": "search_replace",
        "call_id": "call-1",
        "exit_code": 0,
        # Blackboard partition (2026-08-08): successful file-edit tools
        # record their line-range delta — old_lines/new_lines counted from
        # the tool's old_string/new_string args. Event-level timestamp
        # carries the time (see docs/INQUIRY_FIX_AND_BLACKBOARD_PARTITION
        # _2026-08-08.md §4.1).
        "edits": [
            {"file": "1.py", "old_lines": 12, "new_lines": 34},
        ],
    },
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
    # A6 (2026-08-08): explicit context compaction — all seven counters
    # required, non-negative integers (Rust construction: controller.rs
    # compact_messages).
    "context_compressed": {
        "trigger_tokens": 152000,
        "target_tokens": 100000,
        "rounds_since_last_compaction": 22,
        "rounds_dropped": 4,
        "messages_dropped": 12,
        "messages_kept": 8,
        "estimated_tokens_after": 95000,
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
    "tool_completed": {
        "tool": "search_replace",
        "call_id": "call-1",
        "exit_code": 0,
        # Edits entry missing the required `old_lines` field.
        "edits": [{"file": "1.py", "new_lines": 34}],
    },
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
    "context_compressed": {
        # Missing the required rounds_since_last_compaction field.
        "trigger_tokens": 152000,
        "target_tokens": 100000,
        "rounds_dropped": 4,
        "messages_dropped": 12,
        "messages_kept": 8,
        "estimated_tokens_after": 95000,
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

# v0.2 payload shapes (Phase B, ADR-0010 §5.2). Only the five events with
# their own v0.2 payload schema live here; all other v0.2 events reuse the
# v0.1 PAYLOAD_GOOD/PAYLOAD_BAD shapes.
PAYLOAD_GOOD_V02: dict[str, dict] = {
    "orientation_checkpoint": {
        "checkpoint_id": "ORIENT-RUN-CONF-0001-0000",
        "inquiry_family": "neutral",
        "inquiry_kind": "orientation_checkpoint",
        "agent_role": "main",
        "session_id": "sess-main-1",
        "trigger": "completed_turns_interval",
        "completed_turns_since_orientation": 7,
        "step_index": 0,
        "message_block": "[ORIENTATION v0.2] 当前任务、位置与下一目标是什么？",
        "injection_position": "post_tool_batch_gap",
    },
    "diagnostic_coverage_checkpoint": {
        "checkpoint_id": "DIAG-COV-RUN-CONF-0001-0001",
        "inquiry_family": "neutral",
        "inquiry_kind": "diagnostic_coverage_checkpoint",
        "debug_episode_id": "BUG-RUN-CONF-0001",
        "threshold_stage": 3,
        "hard_signal_count": 3,
        "trigger_count": 2,
        "signals": [
            {
                "signal_id": "SIG-0001",
                "signal_type": "consecutive_same_failure",
                "evidence_identity": "EVT-CONF-007",
            },
            {
                "signal_id": "SIG-0002",
                "signal_type": "same_module_no_evidence",
                "evidence_identity": "EVT-CONF-011",
            },
        ],
        "covered_surfaces": ["test_logs", "stack_trace"],
        "missing_surfaces": ["edge_cases"],
        "message_block": "[DIAG_COV v0.2] 已覆盖：测试日志、堆栈；缺失：边界条件；最小补诊断动作：运行最小复现",
        "minimal_next_diagnostic_action": "运行最小复现并采集 trace",
    },
    "information_sufficiency_assessment": {
        "assessment_id": "ASSESS-0001",
        "activation_id": "ACT-EXT-0001",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": ZERO_HASH,
        "ledger_digest": ZERO_HASH,
        "source_counts": {
            "total": 4,
            "full_text_observed": 2,
            "partial_text_observed": 1,
            "metadata_only": 1,
            "unavailable": 0,
        },
        "source_categories": ["official_docs", "source_code", "forum"],
        "source_visibility_gate": "passed",
        "missing_categories": ["vendor_changelog"],
        "filtering_reasons": ["paywall"],
        "status": "sufficient",
        "reason_codes": ["COVERAGE_OK"],
        "assessment_version": "0.2.0",
    },
    "retrieval_parent_disposition": {
        "disposition_id": "DISP-0001",
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-1",
        "activation_id": "ACT-EXT-0001",
        "assessment_id": "ASSESS-0001",
        "expected_contract_revision": 0,
        "decision": "close",
        "requirement_delta": None,
        "capability_gate": "not_applicable",
        "outcome": "accepted",
    },
    "retrieval_close_record": {
        "close_record_id": "CLOSE-0001",
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-2",
        "activation_id": "ACT-EXT-0001",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": ZERO_HASH,
        "assessment_id": "ASSESS-0001",
        "validated_disposition_id": "DISP-0001",
        "terminal_reason": "normal_close",
        "resumable": True,
        "live_state_reset": True,
        "archive_ref": "archive/ACT-EXT-0001",
    },
    "retrieval_mode_transition": {
        "transition_id": "MODETRANS-CONF-0001",
        "session_id": "sess-main-1",
        "old_mode": "off",
        "new_mode": "framework_fallback",
        "authority": "session_bootstrap",
        "reason_code": "session_default",
        "capability_status": "available",
    },
    "retrieval_result_committed": {
        "schema_version": "0.2.0-draft",
        "result_kind": "retrieval_subagent_result",
        "result_id": "RET-RES-0001",
        "activation_id": "ACT-EXT-0001",
        "subagent_session_id": "sess-ext-1",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": ZERO_HASH,
        "ledger_digest": ZERO_HASH,
        "query_summary": [
            {
                "query_id": "QRY-0001",
                "query_text": "rust channel docs",
                "source_category": "official_docs",
                "result_count": 2,
                "action_taken": "searched",
                "tool_used": "web_search",
            }
        ],
        "source_ledger": [
            {
                "source_id": "SRC-0001",
                "source_title": "Rust reference",
                "source_url_or_ref": "https://doc.rust-lang.org/reference",
                "source_type": "web_page",
                "visibility": "full_text_observed",
                "accessed_at": "2026-08-10T00:00:00Z",
                "observed_scope": "full document",
                "missing_scope": "none",
                "relevance": "direct",
                "used_in_sections": ["Rust channels"],
                "content_sha256": ZERO_HASH,
                "highest_allowed_claim": "observed",
                "tier": "default",
                "mechanical_weight": 1.0,
                "weight_reason": "default",
                "model_weight": 1.0,
                "model_weight_reason": "official docs",
                "annotation_status": "adopted",
            },
            {
                "source_id": "SRC-0002",
                "source_title": "Rust forum thread",
                "source_url_or_ref": "https://forum.rust-lang.org/t/42",
                "source_type": "web_page",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-10T00:00:01Z",
                "observed_scope": "first section",
                "missing_scope": "rest of page",
                "relevance": "partial",
                "used_in_sections": [],
                "highest_allowed_claim": "derived",
                "tier": "default",
                "mechanical_weight": 1.0,
                "weight_reason": "default",
            },
        ],
        "filtering_log": [
            {
                "source_id": "SRC-0003",
                "reason": "policy_blocked",
                "action": "excluded",
                "filtered_at": "2026-08-10T00:00:02Z",
            }
        ],
        "organized_response": {
            "sections": [
                {
                    "section_title": "Rust channels",
                    "content": "std::sync::mpsc provides channels.",
                    "source_ids": ["SRC-0001"],
                    "claim_strength": "observed",
                }
            ],
            "claims": [],
            "source_annotations": [
                {
                    "source_id": "SRC-0001",
                    "weight": 1.0,
                    "reason": "official docs",
                    "status": "adopted",
                }
            ],
        },
        "raw_source_refs": [
            {
                "source_id": "SRC-0001",
                "source_title": "Rust reference",
                "source_url_or_ref": "https://doc.rust-lang.org/reference",
                "visibility": "full_text_observed",
                "content_sha256": ZERO_HASH,
            }
        ],
        "source_counts": {
            "total": 2,
            "full_text_observed": 1,
            "partial_text_observed": 1,
            "metadata_only": 0,
            "unavailable": 0,
        },
        "visibility_degraded": False,
    },
    "retrieval_activation_restored": {
        "restore_id": "RST-ACT-0001",
        "activation_id": "ACT-EXT-0001",
        "subagent_session_id": "sess-ext-1",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "status": "awaiting_disposition",
        "assessment_id": "ASSESS-0001",
        "result_digest": ZERO_HASH,
        "origin_run_id": "RUN-CONF-0001",
        "sidecar_ref": ".gsa/activations/sess-abc.json",
        "tool_rounds_used": 3,
    },
    # ACAF Slice 1 (设计文档 §4.2/§4.6) — control-ticket lifecycle events.
    "control_ticket_issued": {
        "ticket_id": "TKT-CONF-0001",
        "ticket_kind": "orientation_v1",
        "session_id": "sess-main-1",
        "agent_id": "main",
        "activation_id": None,
        "goal_version": 0,
        "goal_digest": ZERO_HASH,
        "policy_revision": 0,
        "sequence": 0,
        "capability_scope": "orientation_injection",
        "template_sha256": ZERO_HASH,
        "canonical_arguments_sha256": ZERO_HASH,
        "resolved_target_sha256": None,
        "issued_at": TIMESTAMP,
        "expires_at": TIMESTAMP,
        "signer_revision": 1,
        "signer_measurement": ZERO_HASH,
    },
    "control_ticket_consumed": {
        "ticket_id": "TKT-CONF-0001",
        "ticket_kind": "orientation_v1",
        "consumed_at": TIMESTAMP,
        "outcome": "accepted",
    },
    "control_ticket_rejected": {
        "ticket_id": "TKT-CONF-0002",
        "ticket_kind": "disposition_v1",
        "rejected_at": TIMESTAMP,
        "reject_code": "replay_detected",
        "detail": "nonce already consumed",
    },
    # FUS-TOOL-PROBE (2026-08-13, design §7 v0.2; P0-A-2): single probe
    # face snapshot — complete/incomplete cover ALL work tools; reasons
    # are stable neutral statements.
    "tool_availability_check": {
        "probe_scope": "main_agent_work_tools",
        "probe_timestamp": TIMESTAMP,
        "complete": [
            "read_file",
            "list_dir",
            "grep",
            "search_tool",
            "search_replace",
            "blackboard_read",
            "todo_write",
            "update_goal",
            "compaction_whitelist_add",
        ],
        "incomplete": [
            {"tool": "run_tests", "reason": "缺少测试运行器"},
            {"tool": "ask_user_question", "reason": "无交互式用户会话"},
            {"tool": "enter_plan_mode", "reason": "会话不支持计划模式"},
            {"tool": "exit_plan_mode", "reason": "会话不支持计划模式"},
            {"tool": "retrieval_disposition", "reason": "检索会话未激活"},
            {"tool": "run_terminal_cmd", "reason": "终端链路不完整"},
            {"tool": "lsp", "reason": "语言服务未配置"},
            {"tool": "memory_get", "reason": "记忆存储未启用"},
            {"tool": "memory_search", "reason": "记忆存储未启用"},
            {"tool": "image_gen", "reason": "图像后端未配置"},
            {"tool": "image_edit", "reason": "图像后端未配置"},
            {"tool": "image_to_video", "reason": "视频后端未配置"},
            {"tool": "reference_to_video", "reason": "视频后端未配置"},
            {"tool": "use_tool", "reason": "能力注册未配置"},
        ],
        "gate_decision": "pass",
    },
}

# One constraint violation per v0.2 event (never a bare missing-required when
# a sharper constraint exists; conditional constraints preferred where the
# schema expresses them).
PAYLOAD_BAD_V02: dict[str, dict] = {
    "orientation_checkpoint": {
        "checkpoint_id": "ORIENT-RUN-CONF-0001-0000",
        "inquiry_family": "neutral",
        "inquiry_kind": "orientation_checkpoint",
        "agent_role": "main",
        "session_id": "sess-main-1",
        "trigger": "manual",
        "completed_turns_since_orientation": 7,
        "step_index": 0,
        "message_block": "[ORIENTATION v0.2] 当前任务、位置与下一目标是什么？",
        "injection_position": "post_tool_batch_gap",
    },
    "diagnostic_coverage_checkpoint": {
        "checkpoint_id": "DIAG-COV-RUN-CONF-0001-0001",
        "inquiry_family": "neutral",
        "inquiry_kind": "diagnostic_coverage_checkpoint",
        "debug_episode_id": "BUG-RUN-CONF-0001",
        "threshold_stage": 3,
        "hard_signal_count": 3,
        "trigger_count": 2,
        "signals": [
            {
                "signal_id": "SIG-0001",
                "signal_type": "hypothesis_only",
                "evidence_identity": "EVT-CONF-007",
            }
        ],
        "covered_surfaces": ["test_logs"],
        "missing_surfaces": [],
        "message_block": "[DIAG_COV v0.2] x",
        "minimal_next_diagnostic_action": "运行最小复现",
    },
    "information_sufficiency_assessment": {
        "assessment_id": "ASSESS-0001",
        "activation_id": "ACT-EXT-0001",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": ZERO_HASH,
        "ledger_digest": ZERO_HASH,
        "source_counts": {
            "total": 4,
            "full_text_observed": 2,
            "partial_text_observed": 1,
            "metadata_only": 1,
            "unavailable": 0,
        },
        "source_categories": ["official_docs"],
        "source_visibility_gate": "passed",
        "missing_categories": [],
        "filtering_reasons": [],
        "status": "probably_sufficient",
        "reason_codes": ["COVERAGE_OK"],
        "assessment_version": "0.2.0",
    },
    "retrieval_parent_disposition": {
        "disposition_id": "DISP-0001",
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-2",
        "activation_id": "ACT-EXT-0001",
        "assessment_id": "ASSESS-0001",
        "expected_contract_revision": 0,
        "decision": "continue",
        "requirement_delta": None,
        "capability_gate": "passed",
        "outcome": "accepted",
    },
    "retrieval_close_record": {
        "close_record_id": "CLOSE-0001",
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-2",
        "activation_id": "ACT-EXT-0001",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": ZERO_HASH,
        "assessment_id": "ASSESS-0001",
        "validated_disposition_id": None,
        "terminal_reason": "normal_close",
        "resumable": True,
        "live_state_reset": True,
        "archive_ref": "archive/ACT-EXT-0001",
    },
    "retrieval_mode_transition": {
        "transition_id": "MODETRANS-CONF-0001",
        "session_id": "sess-main-1",
        "old_mode": "off",
        "new_mode": "off",
        "authority": "session_bootstrap",
        "reason_code": "session_default",
        "capability_status": "available",
    },
    "retrieval_result_committed": {
        "schema_version": "0.2.0-draft",
        "result_kind": "retrieval_subagent_result",
        "result_id": "RET-RES-0001",
        "activation_id": "ACT-EXT-0001",
        "subagent_session_id": "sess-ext-1",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": ZERO_HASH,
        "ledger_digest": ZERO_HASH,
        "query_summary": [
            {
                "query_id": "QRY-0001",
                "query_text": "rust channel docs",
                "source_category": "official_docs",
                "result_count": 2,
                "action_taken": "searched",
                "tool_used": "web_search",
            }
        ],
        "source_ledger": [
            {
                "source_id": "SRC-0001",
                "source_title": "Rust reference",
                "source_url_or_ref": "https://doc.rust-lang.org/reference",
                "source_type": "web_page",
                "visibility": "full_text_observed",
                "accessed_at": "2026-08-10T00:00:00Z",
                "observed_scope": "full document",
                "missing_scope": "none",
                "relevance": "direct",
                "used_in_sections": ["Rust channels"],
                "content_sha256": ZERO_HASH,
                "highest_allowed_claim": "derived",
            },
        ],
        "filtering_log": [],
        "organized_response": {
            "sections": [
                {
                    "section_title": "Rust channels",
                    "content": "std::sync::mpsc provides channels.",
                    "source_ids": ["SRC-0001"],
                    "claim_strength": "legendary",
                }
            ],
            "claims": [],
        },
        "raw_source_refs": [
            {
                "source_id": "SRC-0001",
                "source_title": "Rust reference",
                "source_url_or_ref": "https://doc.rust-lang.org/reference",
                "visibility": "full_text_observed",
                "content_sha256": ZERO_HASH,
            }
        ],
        "source_counts": {
            "total": 1,
            "full_text_observed": 1,
            "partial_text_observed": 0,
            "metadata_only": 0,
            "unavailable": 0,
        },
        "visibility_degraded": True,
    },
    "retrieval_activation_restored": {
        "restore_id": "RST-ACT-0001",
        "activation_id": "ACT-EXT-0001",
        "subagent_session_id": "sess-ext-1",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "status": "awaiting_disposition",
        "result_digest": ZERO_HASH,
        "origin_run_id": "RUN-CONF-0001",
        "sidecar_ref": ".gsa/activations/sess-abc.json",
        "tool_rounds_used": 3,
    },
    # ACAF Slice 1 (设计文档 §4.2) — constraint violations: capability_scope
    # disagrees with ticket_kind (conditional allOf; the activation binding
    # stays valid so exactly ONE constraint is violated — review C2-4),
    # consumed outcome outside the accepted const, unknown reject_code.
    "control_ticket_issued": {
        "ticket_id": "TKT-CONF-0001",
        "ticket_kind": "close_v1",
        "session_id": "sess-main-1",
        "agent_id": "main",
        "activation_id": "ACT-1",
        "goal_version": 0,
        "goal_digest": ZERO_HASH,
        "policy_revision": 0,
        "sequence": 0,
        "capability_scope": "orientation_injection",
        "template_sha256": None,
        "canonical_arguments_sha256": ZERO_HASH,
        "resolved_target_sha256": None,
        "issued_at": TIMESTAMP,
        "expires_at": TIMESTAMP,
        "signer_revision": 1,
        "signer_measurement": ZERO_HASH,
    },
    "control_ticket_consumed": {
        "ticket_id": "TKT-CONF-0001",
        "ticket_kind": "orientation_v1",
        "consumed_at": TIMESTAMP,
        "outcome": "rejected",
    },
    "control_ticket_rejected": {
        "ticket_id": "TKT-CONF-0002",
        "ticket_kind": "disposition_v1",
        "rejected_at": TIMESTAMP,
        "reject_code": "forged",
        "detail": "unknown reject code",
    },
    "tool_availability_check": {
        "probe_scope": "main_agent_work_tools",
        "probe_timestamp": TIMESTAMP,
        "complete": [
            "read_file",
            "list_dir",
            "grep",
            "search_tool",
            "search_replace",
            "blackboard_read",
            "todo_write",
            "update_goal",
            "compaction_whitelist_add",
        ],
        "incomplete": [
            {"tool": "run_tests", "reason": "缺少测试运行器"},
            {"tool": "ask_user_question", "reason": "无交互式用户会话"},
            {"tool": "enter_plan_mode", "reason": "会话不支持计划模式"},
            {"tool": "exit_plan_mode", "reason": "会话不支持计划模式"},
            {"tool": "retrieval_disposition", "reason": "检索会话未激活"},
            {"tool": "run_terminal_cmd", "reason": "终端链路不完整"},
            {"tool": "lsp", "reason": "语言服务未配置"},
            {"tool": "memory_get", "reason": "记忆存储未启用"},
            {"tool": "memory_search", "reason": "记忆存储未启用"},
            {"tool": "image_gen", "reason": "图像后端未配置"},
            {"tool": "image_edit", "reason": "图像后端未配置"},
            {"tool": "image_to_video", "reason": "视频后端未配置"},
            {"tool": "reference_to_video", "reason": "视频后端未配置"},
            {"tool": "use_tool", "reason": "能力注册未配置"},
        ],
        "gate_decision": "stop",
    },
}

# ACAF Slice 2 fail-closed (2026-08-13): extra positive payload fixtures for
# the new reject codes — each carries a null ticket_id (no ticket was issued;
# the refusal happens BEFORE signing, D-14/D-15). The primary GOOD fixture
# above keeps the replay_detected shape; these three exercise the schema's
# extended enum.
EXTRA_V02_PAYLOAD_POSITIVES: dict[str, dict] = {
    "control-ticket-rejected.missing-target-argument.valid": {
        "ticket_id": None,
        "ticket_kind": "network_v1",
        "rejected_at": TIMESTAMP,
        "reject_code": "missing_target_argument",
        "detail": "url argument missing",
    },
    "control-ticket-rejected.missing-snapshot-store.valid": {
        "ticket_id": None,
        "ticket_kind": "file_write_v1",
        "rejected_at": TIMESTAMP,
        "reject_code": "missing_snapshot_store",
        "detail": "no snapshot store for target resolution",
    },
    "control-ticket-rejected.missing-goal-context.valid": {
        "ticket_id": None,
        "ticket_kind": "goal_revision_v1",
        "rejected_at": TIMESTAMP,
        "reject_code": "missing_goal_context",
        "detail": "goal digest not pinned",
    },
}

# GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): extra negative payload fixture —
# the fixed tier/weight pair is violated (authoritative MUST be 1.1; the
# good fixture's SRC-0001 keeps mechanical_weight 1.0). The model annotation
# fields are stripped so exactly ONE constraint is violated.
EXTRA_V02_PAYLOAD_BADS: dict[str, dict] = {}
_weighting_bad = json.loads(json.dumps(PAYLOAD_GOOD_V02["retrieval_result_committed"]))
_weighting_bad["source_ledger"] = json.loads(
    json.dumps(PAYLOAD_GOOD_V02["retrieval_result_committed"]["source_ledger"])
)
_weighting_bad["source_ledger"][0]["tier"] = "authoritative"
_weighting_bad["source_ledger"][0].pop("model_weight", None)
_weighting_bad["source_ledger"][0].pop("model_weight_reason", None)
_weighting_bad["source_ledger"][0].pop("annotation_status", None)
_weighting_bad["organized_response"] = json.loads(
    json.dumps(PAYLOAD_GOOD_V02["retrieval_result_committed"]["organized_response"])
)
_weighting_bad["organized_response"].pop("source_annotations", None)
EXTRA_V02_PAYLOAD_BADS[
    "retrieval-result.tier-weight-mismatch.constraint.invalid"
] = _weighting_bad


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


def _envelope_v02(event_type: str, payload: dict, sequence: int, previous: str | None) -> dict:
    slug = SLUGS_V02.get(event_type) or SLUGS[event_type]
    return {
        "schema_version": "0.2.0-draft",
        "run_id": f"RUN-CONF-{slug.upper()}",
        "event_id": f"EVT-CONF-{sequence:03d}",
        "sequence": sequence,
        "timestamp": TIMESTAMP,
        "event_type": event_type,
        "run_manifest_sha256": ZERO_HASH,
        "previous_event_sha256": previous,
        "payload_schema": "run-event-v0.2.schema.json",
        "payload": payload,
        "payload_sha256": ZERO_HASH,
        "redaction": "none",
        "event_sha256": ZERO_HASH,
    }


def _bad_envelope(mutate: dict) -> dict:
    return mutate(
        _envelope("run_preflight", {"run_id": "RUN-CONF-0001"}, 0, None)
    )


def _bad_envelope_v02(mutate: dict) -> dict:
    return mutate(
        _envelope_v02("run_preflight", {"run_id": "RUN-CONF-0001"}, 0, None)
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

# v0.2 envelope negatives: built on the v0.2 envelope base so every negative
# isolates exactly one constraint violation on the v0.2 track (review fix:
# the v0.1 base made all 14 samples fail on schema_version instead). The
# schema-version bad value flips to the v0.1 constant; each retired event
# gets its own event-type negative.
V02_ENVELOPE_BAD: dict[str, dict] = {
    "bad-schema-version": _bad_envelope_v02(
        lambda e: e.update({"schema_version": "0.1.0-draft"}) or e
    ),
    "retired-event-type": _bad_envelope_v02(
        lambda e: e.update({"event_type": "neutral_inquiry"}) or e
    ),
    "retired-retrieval-completion-check": _bad_envelope_v02(
        lambda e: e.update({"event_type": "retrieval_completion_check"}) or e
    ),
    # v0.1's negative-sequence (sequence=-1, previous=None) trips both the
    # minimum and the allOf else-branch; on the v0.2 track keep it to exactly
    # the sequence violation so every negative isolates one constraint.
    "negative-sequence": _bad_envelope_v02(
        lambda e: e.update({"sequence": -1, "previous_event_sha256": DUMMY_HASH})
        or e
    ),
}
for _name, _payload in ENVELOPE_BAD.items():
    if _name not in V02_ENVELOPE_BAD:
        _v02 = dict(_payload)
        _v02["schema_version"] = "0.2.0-draft"
        # missing-required-payload-schema pops the key — do not re-add it.
        if "payload_schema" in _v02:
            _v02["payload_schema"] = "run-event-v0.2.schema.json"
        V02_ENVELOPE_BAD[_name] = _v02

FIXTURES_README = """# run-event-v0.1 reference fixtures

Reference-spec fixtures for the run-event envelope and its 34 event payload
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

FIXTURES_README_V02 = """# run-event-v0.2 reference fixtures

Phase B (ADR-0010 §5.3.1/§11.2/§11.6) fixtures for the v0.2 event system
(generated by `scripts/generate_run_event_fixtures.py` — re-run that script
after any v0.2 payload schema change).

Scope:

- `payloads/<slug>.minimal.valid.json` / `<slug>.constraint.invalid.json` —
  legal / one-constraint-violation payloads for the **twelve** v0.2 mechanism
  events with their own v0.2 payload schema: `orientation_checkpoint`
  (v0.2 shape), `diagnostic_coverage_checkpoint`,
  `information_sufficiency_assessment`, `retrieval_parent_disposition`,
  `retrieval_close_record`, plus the GAP-RETRIEVAL-TOOLS trio
  `retrieval_mode_transition`, `retrieval_result_committed`,
  `retrieval_activation_restored`, plus the ACAF trio (Slice 1, 2026-08-12)
  `control_ticket_issued`, `control_ticket_consumed`,
  `control_ticket_rejected` (ticket lifecycle binding fields — never the
  HMAC tag, which stays inside the issuing process), plus
  `tool_availability_check` (FUS-TOOL-PROBE 2026-08-13, P0-A-2: two-state
  single probe face snapshot — complete/incomplete cover ALL work tools).
- ACAF Slice 2 fail-closed (2026-08-13): `control-ticket-rejected` gains
  three extra positive payload fixtures for the new pre-signing reject codes
  `missing_target_argument` / `missing_snapshot_store` / `missing_goal_context`
  (each carries `ticket_id: null` — no ticket exists when the refusal
  happens, D-14/D-15).
- GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): `retrieval-result` gains one extra
  negative payload fixture for the fixed tier/weight table (authoritative
  MUST pair with 1.1; the good fixture carries the full weighting fields).
- `envelope/<slug>.valid.json` — a full 13-field v0.2 envelope for **every**
  event in the v0.2 enum (42 events). The twelve v0.2-payload events carry
  their v0.2 payload; the other 30 events reuse the v0.1 payload shape
  unchanged (their payload schema files did not change — adjudicated
  decision: no copied schema files, the v0.1 files remain authoritative for
  unchanged payloads). `chained-run-finished.valid.json` covers the
  sequence-1 branch (v0.1 README documents the same convention).
- `envelope/<bad-name>.invalid.json` — v0.1 envelope constraints (identical
  structure) plus three v0.2-specific negatives: the v0.1 schema version and
  one negative per retired event type (`neutral_inquiry` /
  `retrieval_completion_check`). Every negative isolates exactly one
  constraint on the v0.2 track.

Adjudications (three-agent review closure, 2026-08-09; GAP-RETRIEVAL-TOOLS
extension 2026-08-10):

- `neutral_inquiry` and `retrieval_completion_check` are absent from the
  v0.2 enum — they exist only on the v0.1 track for historical journal
  replay (ADR-0010 §11.2).
- The v0.2 envelope's `payload_schema` value is `"run-event-v0.2.schema.json"`.
  The cross-validator resolves the twelve v0.2-payload events to their v0.2
  payload schema files and every other event to its v0.1 payload schema file.
- **ACAF ticket kinds are payload-level** (Slice 1, 2026-08-12; Slice 2
  2026-08-12): the control-ticket trio's event types are stable; the
  `ticket_kind` / `capability_scope` enums extend inside the payload schemas
  (`orientation_v1` / `disposition_v1` / `close_v1` / `goal_revision_v1`;
  plus `file_write_v1` / `credential_read_v1` / `command_exec_v1` /
  `network_v1` from Slice 2). Action kinds bind `resolved_target_sha256`
  (digest of the parsed real target object, §4.2 check 5 TOCTOU); control
  kinds leave it null or absent so historical journals remain valid.
- **Producer/consumer/verifier** (§5.2): `information_sufficiency_assessment`,
  `retrieval_parent_disposition` and `retrieval_close_record` — plus the
  GAP-RETRIEVAL-TOOLS trio `retrieval_mode_transition`,
  `retrieval_result_committed` and `retrieval_activation_restored` — are
  mechanical records written by the controller (single writer, serial
  commit); the disposition's `decision`/`requirement_delta` originate from
  the main Agent's structured input, `outcome` is the controller's
  mechanical result (never model self-report). Verifier: the assurance
  dual-track validator. Migration version: `0.2.0-draft`.
- **Termination without results** (§4.3/§4.4): a termination-authority close
  (user cancel / session cancel / wallclock / budget exhaustion / subagent
  failure) may close an activation directly, without an assessment — close
  record `assessment_id` / `result_digest` are therefore required only for
  `terminal_reason=normal_close`. `budget_exhausted` is a distinct terminal
  reason (ADR-0010 §3.3.6), not a wallclock alias.
- **Scope completion** (§3.3.6): scope completion flows through the normal
  path (result → assessment → parent disposition `close` → close record), so
  it is represented by `normal_close`; no separate terminal value.
- **`unabsorbed_new_evidence`** (diagnostic signal taxonomy, §4.6.2/§4.6.3):
  the name means "new evidence identity not yet counted in this episode" —
  it carries no judgment about whether the plan absorbed the evidence;
  signal counting is bound to evidence identity / failure fingerprint only.
- **Closed signal taxonomy**: the six `signal_type` values are a closed
  contract — new signal categories require a schema change (the §4.6.2 list
  is an "e.g." list, the schema narrows it deliberately).
- **`trigger_count` / `hard_signal_count` / `threshold_stage`** are
  informational episode metrics (§5.2 lists the payload boundary; the counts
  are derived facts). `decision=close` requires `requirement_delta=null`
  (a close carrying a delta is self-contradictory, §4.4).
- **ID formats** (`ORIENT-...-[0-9]{4}`, `ASSESS-`, `DISP-`, `CLOSE-`,
  `SIG-`, `DIAG-COV-`) are producer-local formats, not ADR-mandated
  contracts. `assessment_version` uses dotted numeric form (`1.2`).

## Real journals (`journals/`)

The 14 journals below are captured by the orz conformance capture tests
(`cargo test -p orz-bin -- --ignored conformance_capture --test-threads=1`,
staged under `target/conformance-journals/` and dev-copied here — see the
GAP-RETRIEVAL-TOOLS audit doc §5). Re-captured 2026-08-10 after the review
fixes (H1 off projection now hides `project_doc_index` too); the 13th
(`local-browser-read`) is the local_browser slice (2026-08-10) available-
capability scenario:

| journal | scenario |
|---|---|
| `plain-run.jsonl` | text-only turn through the real CLI host (gate round) |
| `tool-snapshot-run.jsonl` | `search_replace` + allow-once + `snapshot_created` |
| `plan-run.jsonl` | plan-write gate + plan_proposed/approved + execution turn |
| `cancelled-run.jsonl` | cooperative cancel (run_cancelled terminal) |
| `failed-run.jsonl` | empty script → run_failed terminal |
| `restore-run.jsonl` | RST- restore journal (preflight → snapshot_restored → finished) |
| `orientation-fire-run.jsonl` | **7 retrieval tool rounds cross the session-level threshold — the orientation fires once in the post-tool-batch gap of round 7, and each retrieval round records a mechanical `information_sufficiency_assessment` (`indeterminate`) plus a `retrieval_result_committed`** |
| `mode-off-refusal.jsonl` | retrieval dispatch under mode=off — refused as `retrieval_mode_off` with NO ToolStarted (verifier mode rule) |
| `local-browser-capability.jsonl` | bootstrap transition to `local_browser` with capability `unsupported` — dispatch fails explicitly (`retrieval_capability_unavailable`), no silent fallback |
| `local-browser-read.jsonl` | bootstrap transition to `local_browser` with capability `available` — external lane runs the host `browser_read` tool (fake lane), committed result carries real full-text `web_page` evidence (ADR §3.7.3/§3.7.5) |
| `real-doc-retrieval.jsonl` | internal lane: `project_doc_index` include_content → mechanical ledger/visibility/`retrieval_result_committed`/assessment (ADR §3.7.4/§3.7.5) |
| `cross-prompt-restore.jsonl` | activation sidecar restore → restore event → cross-run disposition close (verifier restore-declaration chain) |
| `pre-handoff-checkpoint.jsonl` | stagnation restart_requested → `orientation_checkpoint{trigger: "pre_handoff", injection_position: "pre_terminal"}` with empty message_block (audit-only, §11.1) |
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

    # v0.2 tree (Phase B): payload fixtures for the five v0.2-payload events;
    # envelope fixtures for every event in the v0.2 enum.
    v02_root = ROOT / "runtime/fixtures/run-event-v0.2"
    v02_payloads_dir = v02_root / "payloads"
    v02_envelope_dir = v02_root / "envelope"
    v02_journals_dir = v02_root / "journals"
    for directory in (v02_payloads_dir, v02_envelope_dir):
        shutil.rmtree(directory, ignore_errors=True)
        directory.mkdir(parents=True, exist_ok=True)
    v02_journals_dir.mkdir(parents=True, exist_ok=True)

    for event_type in V02_PAYLOAD_EVENTS:
        slug = SLUGS_V02.get(event_type) or SLUGS[event_type]
        write_json(
            v02_payloads_dir / f"{slug}.minimal.valid.json",
            PAYLOAD_GOOD_V02[event_type],
        )
        write_json(
            v02_payloads_dir / f"{slug}.constraint.invalid.json",
            PAYLOAD_BAD_V02[event_type],
        )
    for name, payload in EXTRA_V02_PAYLOAD_POSITIVES.items():
        write_json(v02_payloads_dir / f"{name}.json", payload)
    for name, payload in EXTRA_V02_PAYLOAD_BADS.items():
        write_json(v02_payloads_dir / f"{name}.json", payload)
    for event_type in V02_EVENT_TYPES:
        slug = SLUGS_V02.get(event_type) or SLUGS[event_type]
        payload = PAYLOAD_GOOD_V02.get(event_type) or PAYLOAD_GOOD[event_type]
        write_json(
            v02_envelope_dir / f"{slug}.valid.json",
            _envelope_v02(event_type, payload, 0, None),
        )
    write_json(
        v02_envelope_dir / "chained-run-finished.valid.json",
        _envelope_v02("run_finished", PAYLOAD_GOOD["run_finished"], 1, DUMMY_HASH),
    )
    for name, payload in V02_ENVELOPE_BAD.items():
        write_json(v02_envelope_dir / f"{name}.invalid.json", payload)

    v02_root.joinpath("README.md").write_text(FIXTURES_README_V02, encoding="utf-8")

    print(
        f"payloads: {len(PAYLOAD_GOOD) * 2} files, "
        f"envelope: {len(EVENT_TYPES) + 1 + len(ENVELOPE_BAD)} files, "
        f"canonical_cli: {len(CANONICAL_CLI_GOOD)} files, "
        f"v0.2 payloads: "
        f"{len(PAYLOAD_GOOD_V02) * 2 + len(EXTRA_V02_PAYLOAD_POSITIVES) + len(EXTRA_V02_PAYLOAD_BADS)} files, "
        f"v0.2 envelope: {len(V02_EVENT_TYPES) + 1 + len(V02_ENVELOPE_BAD)} files"
    )


if __name__ == "__main__":
    main()
