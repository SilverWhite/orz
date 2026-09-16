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

V02_ENVELOPE_TIMESTAMP_OVERRIDES: dict[str, str] = {
    # TER T0.2 (2026-09-03): budget_cue_injected hand-written envelope uses
    # the TER date — keep it stable across regeneration.
    "budget_cue_injected": "2026-09-03T00:00:00Z",
    # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37): committed
    # envelope carries the batch date — keep it stable across regeneration.
    "transport_retry": "2026-08-21T00:00:00Z",
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): the committed
    # tool_running envelope carries the S5-2 batch date — keep it stable.
    "tool_running": "2026-08-29T00:00:00Z",
    # BLACKBOARD-CONVERSATION-SCOPE-FOLD B3 (2026-09-03, ADR-0010 §14.52):
    # session_archive envelope carries the B3 batch date.
    "session_archive": "2026-09-03T00:00:00Z",
}

# PLAN-FIRST 阶段 C / P0-E / FUS-LEDGER-FOLD-STATE (2026-08-16/17/18): the
# console-family and plan_write envelope fixtures in the committed tree
# carry hand-crafted run/event identities and timestamps (added before the
# generator covered them). Keep the overrides so a regeneration is
# byte-identical to the committed tree — (run_id, event_id, timestamp);
# None keeps the derived value.
V02_ENVELOPE_IDENTITY_OVERRIDES = {
    "console_mode_transition": (
        "RUN-CONF-CMODE",
        "EVT-CONF-CMODE-000",
        "2026-08-16T00:00:00Z",
    ),
    "console_order_written": (
        "RUN-CONF-CORDER",
        "EVT-CONF-CORDER-000",
        "2026-08-16T00:00:00Z",
    ),
    "console_order_rejected": (
        "RUN-CONF-CREJ",
        "EVT-CONF-CREJ-000",
        "2026-08-17T00:00:00Z",
    ),
    "plan_write": (None, None, "2026-08-16T00:00:00Z"),
    # MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): the committed
    # envelope fixture carries the audit batch identity/date; the generator
    # slug-derived run id would be RUN-CONF-MECHANICAL-AUDIT-UPDATE.
    "mechanical_audit_update": (
        "RUN-CONF-MECH-AUDIT",
        None,
        "2026-08-24T00:00:00Z",
    ),
}

# TER-0.1 v0.2 表全面对齐 (2026-09-09): 已验收信封树中三个事件的信封样例与
# payload 最小正例分轨（信封样例保留手工捕获形态；最小正例随 schema 演进）——
# 生成信封时优先取本表，否则回落 PAYLOAD_GOOD_V02/PAYLOAD_GOOD。
V02_ENVELOPE_PAYLOAD_OVERRIDES: dict[str, dict] = {
    # MECHANICAL-AUDIT-LAYER (2026-08-24): 信封样例为对象键覆盖写留痕的
    # file:app/result.txt 形态（anomaly=null），与 payloads 目录最小正例
    # （cmd:call-9 / anomaly 非空）分轨。
    "mechanical_audit_update": {
        "kind": "tool_result",
        "payload": {
            "key": "file:app/result.txt",
            "round": 5,
            "summary": "search_replace 成功（+1 文件）",
            "anomaly": None,
        },
    },
    # BLACKBOARD-CONVERSATION-SCOPE-FOLD B3 (2026-09-03, ADR-0010 §14.52):
    # 已验收信封样例（P2-13 B3 存档契约）不含后续加入的 effort 字段。
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
    # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37): 信封样例为
    # recovered/midstream 形态（与 payloads 目录 exhausted/zero_chunk 最小
    # 正例分轨，二者均为合法 schema 形态）。
    "transport_retry": {
        "agent_role": "main",
        "outcome": "recovered",
        "kind": "midstream",
        "retries": 1,
        "reason": "stream ended without finish_reason",
    },
}

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
    # tool_running 是 v0.2-only 事件（THIN-HARNESS-REDESIGN-V2 §9.7.3，
    # 2026-08-29 S5-2：v0.1 schema 无此事件、v0.1 夹具树无对应文件）。
    # 2026-09-03（TER T0.2 残留处理，方向 A）从 v0.1 EVENT_TYPES 移除，
    # 否则 v0.1 树重建在 PAYLOAD_GOOD 处 KeyError；该事件只随
    # V02_EVENT_TYPES 生成。
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
    # ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6/§14.9): the
    # model-request header fingerprint (system+tools+config) — initial/change
    # only, per agent lane.
    "request_header_change",
    "acp_initialize",
    "acp_session_created",
    "tool_proposal",
    "permission_requested",
    "permission_decision",
    "tool_started",
    "tool_completed",
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): terminal command
    # mid-run status (auto-backgrounded at the 300s report point).
    "tool_running",
    # TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §3.3): F6 push 档中性
    # 预算提示（剩余秒/已用轮/触发档位；默认 off 零注入）。
    "budget_cue_injected",
    "orientation_checkpoint",
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
    # 0t (2026-09-09, ADR-0010 §14.65, v1.65): 浏览器启动/探活尝试事实事件
    # ——success/failure + 真实原因（三值检索模式 γ 退役后纯事件事实化；
    # retrieval_mode_transition 生产者侧退役，枚举保留只读回放）。
    "browser_launch_result",
    "mechanical_audit_update",
    "control_ticket_issued",
    "control_ticket_consumed",
    "control_ticket_rejected",
    "context_compressed",
    "context_recovery_truncated",
    # F7 (2026-08-15, BACKLOG 6e 复查遗留 / ADR-0010 §14.15): blackboard
    # plan-epoch archive write failure audit trace.
    "epoch_archive_write_failed",
    # BLACKBOARD-CONVERSATION-SCOPE-FOLD B3 (2026-09-03, ADR-0010 §14.52):
    # 会话黑板存档单包（.gsa/archives/<session8>.json.gz 纯打包原子写 +
    # digest；run_id 前缀 +ARC 的专用归档 run journal）。
    "session_archive",
    # PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): first-round plan
    # gate result — plan identity/goal/step count, mechanical validation,
    # one-refill attempt progression and degrade reason.
    "plan_write",
    # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console/direct
    # dual-mode transition decision record; action-bar order record;
    # P0-E 第 4 项 (2026-08-17): pre-issuance order rejection.
    "console_mode_transition",
    "console_order_written",
    "console_order_rejected",
    # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): mechanical
    # action-ledger fold advance (cache-miss attribution).
    "ledger_fold_advance",
    # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.28 审查修复):
    # external-ledger append failure — fold rollback + failure count +
    # budget-exhaustion disable (audit trace of the degrade path).
    "ledger_fold_write_failed",
    # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 第 2 项): transport
    # 重试计数事件面——零 chunk/中段截断重试的 recovered/exhausted 摘要。
    "transport_retry",
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
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): terminal command
    # auto-backgrounded at the 300s report point.
    "tool_running": "tool-running",
    # TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §3.3): F6 push 档中性预算
    # 提示事件——v0.2-only 事件（无 v0.1 对应 slug），必须在此登记，否则
    # v0.2 信封树的 `SLUGS_V02.get(...) or SLUGS[...]` 回退 KeyError。
    "budget_cue_injected": "budget-cue-injected",
    "orientation_checkpoint": "orientation-checkpoint",
    "information_sufficiency_assessment": "information-sufficiency-assessment",
    "retrieval_parent_disposition": "retrieval-parent-disposition",
    "retrieval_close_record": "retrieval-close-record",
    "retrieval_mode_transition": "retrieval-mode-transition",
    "retrieval_result_committed": "retrieval-result",
    "retrieval_activation_restored": "retrieval-activation-restored",
    # 0t (2026-09-09, ADR-0010 §14.65, v1.65): browser-launch-result。
    "browser_launch_result": "browser-launch-result",
    # MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): 机械审查层
    # 轻量事件留痕（对象键覆盖写/键/轮/摘要/异常）。
    "mechanical_audit_update": "mechanical-audit-update",
    # ACAF Slice 1 (设计文档 §4.2/§4.6): control-ticket lifecycle events.
    "control_ticket_issued": "control-ticket-issued",
    "control_ticket_consumed": "control-ticket-consumed",
    "control_ticket_rejected": "control-ticket-rejected",
    # FUS-TOOL-PROBE (2026-08-13; P0-A-2): two-state single probe face
    # snapshot (work tools).
    "tool_availability_check": "tool-availability-check",
    # ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): model-request
    # header fingerprint event.
    "request_header_change": "request-header-change",
    # D2-2 (2026-08-14; ADR-0010 v1.10): recovery pre-check truncation.
    "context_recovery_truncated": "context-recovery-truncated",
    # P0-D S3 (2026-08-14; ADR-0010 v1.10): five-section template summary
    # (the A6 whole-round drop payload stays v0.1 replay-only).
    "context_compressed": "context-compressed",
    # F7 (2026-08-15; ADR-0010 §14.15): blackboard plan-epoch archive
    # write failure (rotated/current + attempts).
    "epoch_archive_write_failed": "epoch-archive-write-failed",
    # BLACKBOARD-CONVERSATION-SCOPE-FOLD B3 (2026-09-03, ADR-0010 §14.52):
    # 会话黑板存档事件。
    "session_archive": "session-archive",
    # PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): first-round plan
    # gate result.
    "plan_write": "plan-write",
    # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console mode
    # transition / action-bar order record; P0-E 第 4 项 (2026-08-17):
    # pre-issuance order rejection.
    "console_mode_transition": "console-mode-transition",
    "console_order_written": "console-order-written",
    "console_order_rejected": "console-order-rejected",
    # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): mechanical
    # action-ledger fold advance.
    "ledger_fold_advance": "ledger-fold-advance",
    # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.28 审查修复):
    # external-ledger append failure audit trace.
    "ledger_fold_write_failed": "ledger-fold-write-failed",
    # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3):
    # transport 重试计数事件面（recovered/exhausted 摘要）。
    "transport_retry": "transport-retry",
}

# The v0.2 events with their own v0.2 payload schema (the rest of the v0.2
# envelope reuses the v0.1 payload schema files). GAP-RETRIEVAL-TOOLS
# (2026-08-10) adds the three retrieval events.
V02_PAYLOAD_EVENTS = [
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): terminal command
    # mid-run status (auto-backgrounded at the 300s report point).
    "tool_running",
    # TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §3.6/§10-S0): tool_completed
    # moves to a v0.2 payload shape on the v0.2 track (W-F13b truncation
    # fields); budget_cue_injected is the F6 push cue event.
    "tool_completed",
    "budget_cue_injected",
    "orientation_checkpoint",
    "information_sufficiency_assessment",
    "retrieval_parent_disposition",
    "retrieval_close_record",
    "retrieval_mode_transition",
    "retrieval_result_committed",
    "retrieval_activation_restored",
    # 0t (2026-09-09, ADR-0010 §14.65, v1.65): browser_launch_result。
    "browser_launch_result",
    # MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): 机械审查层
    # 轻量事件留痕（对象键覆盖写/键/轮/摘要/异常）。
    "mechanical_audit_update",
    # ACAF Slice 1 (设计文档 §4.2/§4.6) — control-ticket lifecycle events.
    "control_ticket_issued",
    "control_ticket_consumed",
    "control_ticket_rejected",
    # FUS-TOOL-PROBE (2026-08-13; P0-A-2): two-state single probe face
    # snapshot (work tools).
    "tool_availability_check",
    # ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): model-request
    # header fingerprint event.
    "request_header_change",
    # D2-2 (2026-08-14; ADR-0010 v1.10): recovery pre-check truncation.
    "context_recovery_truncated",
    # P0-D S3 (2026-08-14; ADR-0010 v1.10): five-section template summary
    # (the A6 whole-round drop payload stays v0.1 replay-only).
    "context_compressed",
    # F7 (2026-08-15; ADR-0010 §14.15): blackboard plan-epoch archive
    # write failure (rotated/current + attempts).
    "epoch_archive_write_failed",
    # BLACKBOARD-CONVERSATION-SCOPE-FOLD B3 (2026-09-03, ADR-0010 §14.52):
    # 会话黑板存档事件。
    "session_archive",
    # PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): first-round plan
    # gate result — plan identity/goal/step count, mechanical validation,
    # one-refill attempt progression and degrade reason.
    "plan_write",
    # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console mode
    # transition / action-bar order record; P0-E 第 4 项 (2026-08-17):
    # pre-issuance order rejection.
    "console_mode_transition",
    "console_order_written",
    "console_order_rejected",
    # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): mechanical
    # action-ledger fold advance.
    "ledger_fold_advance",
    # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.28 审查修复):
    # external-ledger append failure audit trace.
    "ledger_fold_write_failed",
    # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3):
    # transport 重试计数事件面（recovered/exhausted 摘要）。
    "transport_retry",
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
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): tool_running was
    # added to the v0.1 enum without a slug (7c85e34) — restored so the
    # generator's v0.1 tree is executable again (2026-08-30 全面审查轮).
    "tool_running": "tool-running",
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
        "plan_epoch": 1,
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
        "plan_epoch": 1,
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
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): terminal command
    # mid-run status (auto-backgrounded at the 300s report point).
    "tool_running": {
        "tool": "run_terminal_cmd",
        "call_id": "call-term-1",
        "wall_ms": 300_012,
        "task_id": "call-term-1",
        "pid": 42,
        "total_bytes": 8_192,
        "output_file": "/tmp/terminal/call-term-1.log",
    },
    # TER T0.2 (2026-09-03, TODO2 T0.2): tool_completed keeps its v0.1
    # minimal shape on the v0.2 track (the v0.2 payload is a superset).
    "tool_completed": {
        "tool": "search_replace",
        "call_id": "call-1",
        "exit_code": 0,
        "edits": [{"file": "1.py", "old_lines": 12, "new_lines": 34}],
    },
    # TER T0.2 (2026-09-03, TODO2 T0.2): F6 push 档——590s 剩余穿过 600s 档。
    "budget_cue_injected": {
        "remaining_seconds": 590,
        "rounds_used": 12,
        "threshold_seconds": 600,
    },
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
        # 0q/0p 复审后 schema 增补：effort 档位（standard/extended/deep）。
        "effort": "standard",
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
                "content_sha256": ZERO_HASH,
                "highest_allowed_claim": "observed",
                "tier": "default",
                "mechanical_weight": 1.0,
                "weight_reason": "default",
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
    # 0t (2026-09-09, ADR-0010 §14.65, v1.65): 浏览器启动/探活尝试成功
    # 事实事件——cause 为 null。
    "browser_launch_result": {
        "attempt_id": "BLAUNCH-RUN-CONF-0000",
        "status": "success",
        "cause": None,
    },
    # MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / 设计 §2.4):
    # 机械审查层轻量事件留痕——对象键覆盖写（键/轮/摘要/异常）。
    "mechanical_audit_update": {
        "kind": "tool_result",
        "payload": {
            "key": "cmd:call-9",
            "round": 3,
            "summary": "exit 1，超时",
            "anomaly": "exit 1（120s 超时）",
        },
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
            # 0aj-review（2026-09-16）：`blackboard_write` 随 0aj 入工作工具表
            # （WORK_TOOLS 23 → 24，两侧 + Python reference 三处同批），本
            # fixture 的 complete 面同步补入——否则样本仍是旧 23 全集形态，
            # 「全分区」只作子集形态回放（§14.59 合法，但失真）。
            "blackboard_write",
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
    # ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): model-request
    # header fingerprint — initial event of the main lane.
    "request_header_change": {
        "reason": "initial",
        "header_sha256": ZERO_HASH,
        "system_sha256": ZERO_HASH,
        "tools_sha256": ZERO_HASH,
        "config_sha256": ZERO_HASH,
        "agent_role": "main",
        "tools": ["read_file", "grep"],
        "tool_count": 2,
    },
    # D2-2 (2026-08-14; ADR-0010 v1.10): recovery pre-check truncation.
    "context_recovery_truncated": {
        "before_estimate_tokens": 260000,
        "target_tokens": 160000,
        "after_estimate_tokens": 155000,
        "rounds_dropped": 3,
        "messages_dropped": 9,
        "messages_kept": 7,
        "audit_path": ".gsa/runs/RUN-CONF-0001/recovery-conversation-full.json",
    },
    # P0-D S3 (2026-08-14): five-section template summary (v0.2 shape).
    # 2026-08-18 B 定案 (ADR-0010 §14.29): mode is mechanical (zero model
    # calls) — the canonical generator must stay in sync with the producer.
      "context_compressed": {
          "trigger_tokens": 165000,
          "target_tokens": 12000,
        "rounds_since_last_compaction": 3,
        "rounds_dropped": 12,
        "messages_dropped": 40,
        "messages_kept": 9,
        "estimated_tokens_after": 11000,
        "mode": "mechanical",
        "reason": "rhythm",
        "summary_id": "compaction-RUN-CONF-0001-0001",
        "summary_digest": ZERO_HASH,
        "summary_path": ".gsa/compaction/compaction-RUN-CONF-0001-0001.md",
        "summary_incomplete": False,
        "retained_rounds": 2,
          "guard_failed": False,
          "archive_write_failed": False,
      },
      # F7 (2026-08-15, BACKLOG 6e 复查遗留 / ADR-0010 §14.15): blackboard
      # plan-epoch archive write failure — rotation committed but the
      # durable snapshot is missing.
      "epoch_archive_write_failed": {
          "archive_dir": ".gsa/blackboard",
          "plan_epoch": 2,
          "kind": "rotated",
          "attempts": 3,
      },
      # PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): first-round plan
      # gate result — plan identity/goal/step count, mechanical validation,
      # one-refill attempt progression and degrade reason.
      "plan_write": {
          "plan_id": "plan-00000000-0000-0000-0000-000000000000",
          "goal": "修复缓存回归",
          "step_count": 2,
          "outcome": "accepted",
          "attempt": 1,
          "validation": {
              "valid": True,
              "errors": [],
              "ignored_fields": [],
          },
          "degrade_reason": None,
      },
      # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console mode
      # transition decision record (PLAN_FIRST_BLACKBOARD_DESIGN §7).
      "console_mode_transition": {
          "transition_id": "TRANS-00000000-0000-0000-0000-000000000000",
          "from": "console",
          "to": "direct",
          "trigger": "assistant_failure_streak",
          "streak": 3,
          "order_ids": ["ORD-000001", "ORD-000002", "ORD-000003"],
          "model_decision": "switch",
          "model_reason": "assistant layer failed three consecutive orders",
          "run_id": "RUN-CONF-CMODE",
          "round": 4,
          "plan_epoch": 1,
          "related_transition_id": None,
      },
      # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): the action-bar
      # order record — identity, step binding and mechanical stamps
      # (PLAN_FIRST_BLACKBOARD_DESIGN §5-§6).
      "console_order_written": {
          "order_id": "ORD-000001",
          "write_call_id": "call-write-1",
          "action": "workspace.read_file",
          "step_id": "s1",
          "round": 2,
          "plan_epoch": 1,
          "run_id": "RUN-CONF-CORDER",
      },
      # P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): pre-issuance
      # rejection of a written order (receipt stays the human view).
      "console_order_rejected": {
          "order_id": "ORD-000011",
          "step": "protocol",
          "phase": "pre_issue",
          "code": "step_not_done",
          "reason": "order refused — step_not_done: the current executable step is s1",
          "round": 3,
          "plan_epoch": 1,
          "run_id": "RUN-CONF-CREJ",
      },
      # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): mechanical
      # action-ledger fold advance — the request-view prefix is rewritten
      # once per fold window (cache-miss attribution).
      "ledger_fold_advance": {
          "fold_start": 1,
          "fold_cut": 5,
          "rounds_folded": 2,
          "view_estimate_tokens": 128000,
          "view_estimate_after": 64000,
          "agent_role": "main",
      },
      # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.28 审查修复):
      # external-ledger append failure — mid-burst (attempt 2 of 3,
      # disabled=false); the budget-exhaustion shape is the third event.
      "ledger_fold_write_failed": {
          "ledger_path": "/app/.gsa/ledger/current.md",
          "attempt": 2,
          "disabled": False,
          "rows": 3,
          "view_estimate_tokens": 135000,
          "agent_role": "main",
      },
      # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3):
      # transport 重试计数事件面。2026-09-03（TER T0.2 残留处理）补回 v0.2
      # 表缺失项——规范形态以已提交 payload 样例为准（outcome=exhausted /
      # kind=zero_chunk），envelope 实例随之归一。
      "transport_retry": {
          "agent_role": "main",
          "outcome": "exhausted",
          "kind": "zero_chunk",
          "retries": 3,
          "reason": "error sending request: connection reset",
      },
      # BLACKBOARD-CONVERSATION-SCOPE-FOLD B3 (2026-09-03, ADR-0010 §14.52):
      # 会话黑板存档最小正例——digest/status/attempts/fatigue_pct（payload
      # 形态以已验收样例为准）。
      "session_archive": {
          "archive_id": "sess-abcdef12-20260903T120000Z",
          "path": ".gsa/archives/abcdef12.json.gz",
          "digest": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
          "status": "completed",
          "attempts": 1,
          "fatigue_pct": 72,
      },
  }

# One constraint violation per v0.2 event (never a bare missing-required when
# a sharper constraint exists; conditional constraints preferred where the
# schema expresses them).
PAYLOAD_BAD_V02: dict[str, dict] = {
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): negative wall_ms
    # violates the one constraint of the mid-run payload.
    "tool_running": {
        "tool": "run_terminal_cmd",
        "call_id": "call-term-1",
        "wall_ms": -1,
        "task_id": "call-term-1",
        "output_file": "/tmp/terminal/call-term-1.log",
    },
    # TER T0.2 (2026-09-03, TODO2 T0.2): const-true marker violated by false.
    "tool_completed": {
        "tool": "search_replace",
        "call_id": "call-1",
        "exit_code": 0,
        "edits": [{"file": "1.py", "old_lines": 12, "new_lines": 34}],
        "output_truncated": False,
    },
    # TER T0.2 (2026-09-03, TODO2 T0.2): threshold outside the closed tier set.
    "budget_cue_injected": {
        "remaining_seconds": 590,
        "rounds_used": 12,
        "threshold_seconds": 999,
    },
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
                "visibility": "full_text_observed",
                "accessed_at": "2026-08-10T00:00:00Z",
                "observed_scope": "full document",
                "missing_scope": "none",
                "relevance": "direct",
                "content_sha256": ZERO_HASH,
                "highest_allowed_claim": "derived",
            },
        ],
        "filtering_log": [],
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
        "visibility_degraded": False,
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
    # 0t (2026-09-09, ADR-0010 §14.65, v1.65): 约束违反——status=failure
    # 必须携带非空 cause（if/then）。
    "browser_launch_result": {
        "attempt_id": "BLAUNCH-RUN-CONF-0000",
        "status": "failure",
        "cause": None,
    },
    # MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / 设计 §2.4):
    # 约束违反——kind 未注册 / key 空 / round 负 / anomaly 非字符串。
    "mechanical_audit_update": {
        "kind": "budget",
        "payload": {
            "key": "",
            "round": -1,
            "summary": "",
            "anomaly": 7,
        },
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
            # 0aj-review（2026-09-16）：同 PAYLOAD_GOOD_V02——工作工具表 23 → 24
            # 后「全分区」形态须含 `blackboard_write`（本 fixture 的唯一违规点
            # 不在工具面，补入不改变其单约束违反性）。
            "blackboard_write",
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
    "request_header_change": {
        # reason outside the enum violates exactly one schema constraint.
        "reason": "bogus",
        "header_sha256": ZERO_HASH,
        "system_sha256": ZERO_HASH,
        "tools_sha256": ZERO_HASH,
        "config_sha256": ZERO_HASH,
        "agent_role": "main",
        "tools": ["read_file"],
        "tool_count": 1,
    },
    "context_recovery_truncated": {
        "before_estimate_tokens": 260000,
        "target_tokens": 160000,
        # Negative rounds_dropped violates the schema minimum (the verifier's
        # ≥1-round rule is a separate cross-check).
        "after_estimate_tokens": 155000,
        "rounds_dropped": -1,
        "messages_dropped": 9,
        "messages_kept": 7,
        "audit_path": ".gsa/runs/RUN-CONF-0001/recovery-conversation-full.json",
    },
      "context_compressed": {
          "trigger_tokens": 165000,
          "target_tokens": 12000,
        "rounds_since_last_compaction": 3,
        "rounds_dropped": 12,
        "messages_dropped": 40,
        "messages_kept": 9,
        "estimated_tokens_after": 11000,
        # mode outside the const violates exactly one schema constraint.
        "mode": "whole_round_drop",
        "reason": "rhythm",
        "summary_id": "compaction-RUN-CONF-0001-0001",
        "summary_digest": ZERO_HASH,
        "summary_path": ".gsa/compaction/compaction-RUN-CONF-0001-0001.md",
          "summary_incomplete": False,
          "retained_rounds": 2,
      },
      "epoch_archive_write_failed": {
          "archive_dir": ".gsa/blackboard",
          # kind outside the enum violates exactly one schema constraint.
          "kind": "replaced",
          "plan_epoch": 2,
          "attempts": 3,
      },
      # PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): one constraint
      # violation — outcome outside the closed enum.
      "plan_write": {
          "plan_id": "plan-00000000-0000-0000-0000-000000000000",
          "goal": "修复缓存回归",
          "step_count": 2,
          "outcome": "rejected",
          "attempt": 1,
          "validation": {
              "valid": True,
              "errors": [],
              "ignored_fields": [],
          },
          "degrade_reason": None,
      },
      # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): one constraint
      # violation — related_transition_id must be null when the transition
      # stays on the same mode.
      "console_mode_transition": {
          "transition_id": "TRANS-BAD",
          "from": "console",
          "to": "direct",
          "trigger": "assistant_failure_streak",
          "streak": 0,
          "order_ids": [],
          "model_decision": "stay",
          "model_reason": None,
          "run_id": "RUN-CONF-CMODE",
          "round": 4,
          "plan_epoch": 1,
          "related_transition_id": "not-null",
      },
      # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): one constraint
      # violation — action must be a string.
      "console_order_written": {
          "order_id": "",
          "write_call_id": "",
          "action": 7,
          "step_id": "",
          "round": -1,
          "plan_epoch": -1,
          "run_id": "",
      },
      # P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): one constraint
      # violation — phase outside the closed enum.
      "console_order_rejected": {
          "order_id": "",
          "step": "",
          "phase": "executed",
          "code": "",
          "reason": "",
          "round": -1,
          "plan_epoch": -1,
          "run_id": "",
      },
      # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): one constraint
      # violation — agent_role outside the lane enum.
      "ledger_fold_advance": {
          "fold_start": 1,
          "fold_cut": 5,
          "rounds_folded": 2,
          "view_estimate_tokens": 128000,
          "view_estimate_after": 64000,
          "agent_role": "orchestrator",
      },
      # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.28 审查修复): one
      # constraint violation — agent_role outside the lane enum.
      "ledger_fold_write_failed": {
          "ledger_path": "/app/.gsa/ledger/current.md",
          "attempt": 2,
          "disabled": False,
          "rows": 3,
          "view_estimate_tokens": 135000,
          "agent_role": "orchestrator",
      },
      # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37): one constraint
      # violation — retries below the schema minimum (>= 1).
      "transport_retry": {
          "agent_role": "main",
          "outcome": "exhausted",
          "kind": "zero_chunk",
          "retries": 0,
          "reason": "retries must be >= 1",
      },
      # BLACKBOARD-CONVERSATION-SCOPE-FOLD B3 (2026-09-03, ADR-0010 §14.52):
      # 约束违反——digest 非 sha256 且 attempts < 1（已验收样例原样入表）。
      "session_archive": {
          "archive_id": "sess-abcdef12-20260903T120000Z",
          "path": ".gsa/archives/abcdef12.json.gz",
          "digest": "not-a-sha256",
          "status": "completed",
          "attempts": 0,
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
    # GAP-EVENT-SCHEMA-DRIFT (2026-08-26): retrieval_mode_transition 模式 A
    # 自动降级 shape——local_browser probe 失败 → framework_fallback 以
    # authority=mechanical_probe + reason_code=browser_launch_failed 落盘
    # （RETRIEVAL-SUBAGENT-WIRING 2026-08-25, ADR-0010 §14.40）。
    "retrieval-mode-transition.mechanical-degrade.valid": {
        "transition_id": "MODETRANS-CONF-DEGRADE",
        "session_id": "sess-main-1",
        "old_mode": "local_browser",
        "new_mode": "framework_fallback",
        "authority": "mechanical_probe",
        "reason_code": "browser_launch_failed",
        "capability_status": "available",
    },
    # GAP-EVENT-SCHEMA-DRIFT (2026-08-26): D-13 检索 lane 绑定语义入 schema
    # ——动作票 activation_id 放开为可选绑定：检索 lane 内 network_v1 票携带
    # 真实 activation_id（主 lane 保持 null）。
    # P0-0x S2 (2026-09-11, ADR-0010 §14.66): the SECOND orientation trigger —
    # the one-shot initial-round inquiry. Same payload schema as the periodic
    # fire; the trigger value selects the injected block, so the positive
    # fixture pairs `trigger=initial_round` with the `[INITIAL_ROUND_INQUIRY`
    # block at `post_tool_batch_gap` (the negative fixture above keeps an
    # out-of-enum trigger value).
    "orientation-checkpoint.initial-round.valid": {
        "checkpoint_id": "ORIENT-RUN-CONF-0002-0000",
        "inquiry_family": "neutral",
        "inquiry_kind": "orientation_checkpoint",
        "agent_role": "main",
        "session_id": "sess-main-1",
        "trigger": "initial_round",
        "completed_turns_since_orientation": 1,
        "step_index": 0,
        "message_block": (
            "[INITIAL_ROUND_INQUIRY v0.1]\n"
            "开局问询（一次性，非强制模板，不打断动作）：\n"
            "1. 本任务实际要交付什么、会被按什么判定？\n"
            "2. 大方向是什么？当前处在什么阶段、下一步要解决什么？\n"
            "3. 当前做法优劣如何？你对任务有何评估？\n"
            "[/INITIAL_ROUND_INQUIRY]"
        ),
        "injection_position": "post_tool_batch_gap",
    },
    # P0-0v P2-3/P2-4 (2026-09-10): browser_control SERP refusal completions
    # carry structured counts — the lane budget refusal (engines per lane) and
    # the session-floor refusal (engines spent in the shared browser session).
    # These two were landed by 0v **without** a generator entry, so the next
    # regeneration silently dropped them (found by the 0x S2 fixture run —
    # registered here so the fixture tree is reproducible again).
    "tool-completed.serp-budget-exceeded.valid": {
        "tool": "browser_control",
        "call_id": "call-serp-budget-1",
        "status": "error",
        "error": "browser_control_search_budget_exceeded",
        "serp_budget_used": 8,
        "serp_budget_cap": 8,
    },
    "tool-completed.serp-session-reserved.valid": {
        "tool": "browser_control",
        "call_id": "call-serp-session-1",
        "status": "error",
        "error": "browser_control_search_session_reserved",
        "serp_session_navigations": 24,
        "serp_session_ceiling": 40,
    },
    "control-ticket-issued.network-lane-bound.valid": {
        "ticket_id": "TKT-CONF-LANE-0001",
        "ticket_kind": "network_v1",
        "session_id": "sess-main-1",
        "agent_id": "main",
        "activation_id": "retrieval-external_retrieval-RUN-CONF-00",
        "goal_version": 0,
        "goal_digest": ZERO_HASH,
        "policy_revision": 0,
        "sequence": 7,
        "capability_scope": "network",
        "template_sha256": None,
        "canonical_arguments_sha256": ZERO_HASH,
        "resolved_target_sha256": ZERO_HASH,
        "issued_at": TIMESTAMP,
        "expires_at": TIMESTAMP,
        "signer_revision": 1,
        "signer_measurement": ZERO_HASH,
    },
    # TER-0.1 v0.2 表全面对齐 (2026-09-09): P2-11/P2-13 起手工维护、未入
    # 生成器表的额外正例——检索关闭记录 effort 档位变体（auto_close /
    # subagent_timeout 终止无 assessment，effort 档位随 schema 演进）。
    "retrieval-close-record.auto-close.valid": {
        "close_record_id": "CLOSE-0001",
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-2",
        "activation_id": "ACT-EXT-0001",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": ZERO_HASH,
        "assessment_id": "ASSESS-0001",
        "validated_disposition_id": None,
        "terminal_reason": "auto_close",
        "effort": "extended",
        "resumable": True,
        "live_state_reset": True,
        "archive_ref": "archive/ACT-EXT-0001",
    },
    "retrieval-close-record.subagent-timeout.valid": {
        "close_record_id": "CLOSE-0001",
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-2",
        "activation_id": "ACT-EXT-0001",
        "contract_id": "CONTRACT-EXT-0001",
        "contract_revision": 0,
        "result_digest": None,
        "assessment_id": None,
        "validated_disposition_id": None,
        "terminal_reason": "subagent_timeout",
        "effort": "deep",
        "resumable": True,
        "live_state_reset": True,
        "archive_ref": "run-journal:RUN-0001",
    },
    # P2-11 依赖图主线 (2026-09-01, ADR-0010 §14.51): tool_completed 可选
    # dep_graph 事件字段 read/write 正例。
    "tool-completed.dep-graph-read.valid": {
        "tool": "read_file",
        "call_id": "call-dep-r1",
        "exit_code": 0,
        "dep_graph": {
            "kind": "read",
            "path": "src/a.rs",
            "anchor": {
                "sha256": DUMMY_HASH,
                "size": 100,
                "mtime": 1700000000,
            },
        },
    },
    "tool-completed.dep-graph-write.valid": {
        "tool": "search_replace",
        "call_id": "call-dep-w1",
        "exit_code": 0,
        "dep_graph": {
            "kind": "write",
            "path": "src/a.rs",
            "consumed_read": "call-dep-r1",
            "consumed_anchor": {
                "sha256": DUMMY_HASH,
                "size": 100,
            },
            "new_anchor": {
                "sha256": "2" * 64,
                "size": 110,
            },
        },
    },
    # 0q 统一失败事件管线 (2026-09-08, ADR-0010 §14.63): failure_target
    # 四族身份正例（anchor_target / cmd_target / file_target / url_target）。
    "tool-completed.failure-target-anchor.valid": {
        "tool": "search_replace",
        "call_id": "call-ft-anchor",
        "exit_code": 1,
        "status": "error",
        "error": "content_anchor_mismatch",
        "file_path": "src/lib.rs",
        "failure_target": {
            "kind": "anchor_target",
            "id": "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4",
            "path": "src/lib.rs",
            "anchor_hash": "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4",
            "size": 4096,
        },
    },
    "tool-completed.failure-target-cmd.valid": {
        "tool": "run_terminal_cmd",
        "call_id": "call-ft-cmd",
        "exit_code": 1,
        "status": "error",
        "error": "timed_out",
        "timed_out": True,
        "wall_ms": 120000,
        "failure_target": {
            "kind": "cmd_target",
            "id": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
            "cmd_preview": "test",
        },
    },
    "tool-completed.failure-target-file.valid": {
        "tool": "read_file",
        "call_id": "call-ft-file",
        "exit_code": 1,
        "status": "error",
        "error": "file_not_found",
        "failure_target": {
            "kind": "file_target",
            "id": ZERO_HASH,
            "path": "missing/target.rs",
        },
    },
    "tool-completed.failure-target-url.valid": {
        "tool": "web_fetch",
        "call_id": "call-ft-url",
        "exit_code": 1,
        "status": "error",
        "error": "fetch_failed",
        "failure_target": {
            "kind": "url_target",
            "id": "d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2",
            "canonical_url": "https://example.com/papers/mr-cr.html",
        },
    },
}

# GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): extra negative payload fixture —
# the fixed tier/weight pair is violated (authoritative MUST be 1.1; the
# good fixture's SRC-0001 keeps mechanical_weight 1.0). The retired model
# annotation fields are absent from the good fixture already.
EXTRA_V02_PAYLOAD_BADS: dict[str, dict] = {}
_weighting_bad = json.loads(json.dumps(PAYLOAD_GOOD_V02["retrieval_result_committed"]))
_weighting_bad["source_ledger"] = json.loads(
    json.dumps(PAYLOAD_GOOD_V02["retrieval_result_committed"]["source_ledger"])
)
_weighting_bad["source_ledger"][0]["tier"] = "authoritative"
EXTRA_V02_PAYLOAD_BADS[
    "retrieval-result.tier-weight-mismatch.constraint.invalid"
] = _weighting_bad

# ORZ-CACHE-CONTEXT-COST (2026-08-15 review fix): `reason=change` without
# `change_kind` violates the schema's conditional requirement (allOf) — the
# mechanical「变化原因」must always be attributed on a real header change.
EXTRA_V02_PAYLOAD_BADS[
    "request-header-change.change-missing-kind.constraint.invalid"
] = {
    "reason": "change",
    "header_sha256": "1" * 64,
    "system_sha256": ZERO_HASH,
    "tools_sha256": "2" * 64,
    "config_sha256": ZERO_HASH,
    "agent_role": "main",
    "previous_header_sha256": ZERO_HASH,
    "tools": ["read_file"],
    "tool_count": 1,
}

# TER-0.1 v0.2 表全面对齐 (2026-09-09): 手工维护的约束反例入表——
# retrieval-close-record effort 档位非法值。
EXTRA_V02_PAYLOAD_BADS["retrieval-close-record.bad-effort.invalid"] = {
    "close_record_id": "CLOSE-0001",
    "parent_session_id": "sess-main-1",
    "subagent_session_id": "sess-ext-2",
    "activation_id": "ACT-EXT-0001",
    "contract_id": "CONTRACT-EXT-0001",
    "contract_revision": 0,
    "result_digest": None,
    "assessment_id": None,
    "validated_disposition_id": None,
    "terminal_reason": "subagent_timeout",
    "effort": "turbo",
    "resumable": True,
    "live_state_reset": True,
    "archive_ref": "run-journal:RUN-0001",
}
# P2-11 依赖图主线 (2026-09-01, ADR-0010 §14.51): dep_graph 约束反例——
# kind 未注册 / 额外字段（均落在 dep_graph 子对象内、单一约束违反）。
EXTRA_V02_PAYLOAD_BADS[
    "tool-completed.dep-graph-bad-kind.constraint.invalid"
] = {
    "tool": "read_file",
    "call_id": "call-dep-bad-kind",
    "exit_code": 0,
    "dep_graph": {
        "kind": "scan",
        "path": "src/a.rs",
    },
}
EXTRA_V02_PAYLOAD_BADS[
    "tool-completed.dep-graph-extra-field.constraint.invalid"
] = {
    "tool": "read_file",
    "call_id": "call-dep-extra",
    "exit_code": 0,
    "dep_graph": {
        "kind": "read",
        "path": "src/a.rs",
        "anchor": None,
        "bogus": 1,
    },
}
# 0q 统一失败事件管线 (2026-09-08, ADR-0010 §14.63): failure_target 约束
# 反例——id 非 sha256 / kind 未注册。
EXTRA_V02_PAYLOAD_BADS[
    "tool-completed.failure-target-bad-id.constraint.invalid"
] = {
    "tool": "run_terminal_cmd",
    "call_id": "call-ft-bad-id",
    "exit_code": 1,
    "status": "error",
    "error": "timed_out",
    "failure_target": {
        "kind": "cmd_target",
        "id": "not-a-sha256-digest",
        "cmd_preview": "make -j8",
    },
}
EXTRA_V02_PAYLOAD_BADS[
    "tool-completed.failure-target-bad-kind.constraint.invalid"
] = {
    "tool": "run_terminal_cmd",
    "call_id": "call-ft-bad-kind",
    "exit_code": 1,
    "status": "error",
    "error": "timed_out",
    "failure_target": {
        "kind": "proc_target",
        "id": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "cmd_preview": "make -j8",
    },
}


# P0-D review fix (2026-08-14, ADR-0010 v1.14): the five-section summary
# gained three schema-legal shapes — session_end reason, guard-retry forced
# compaction (guard_failed), and the explicit archive-write failure. The
# guard_failed×session_end prohibition is a verifier cross-rule, exercised
# synthetically in runtime/tests/test_run_event_journal_validation.py.
def _v02_context_compressed_payload() -> dict:
    return json.loads(json.dumps(PAYLOAD_GOOD_V02["context_compressed"]))


_session_end = _v02_context_compressed_payload()
_session_end["reason"] = "session_end"
_session_end["trigger_tokens"] = 155000
EXTRA_V02_PAYLOAD_POSITIVES["context-compressed.session-end.valid"] = _session_end

_guard_failed = _v02_context_compressed_payload()
_guard_failed["reason"] = "fallback"
_guard_failed["trigger_tokens"] = 210000
_guard_failed["guard_failed"] = True
EXTRA_V02_PAYLOAD_POSITIVES["context-compressed.guard-failed.valid"] = _guard_failed

_archive_failed = _v02_context_compressed_payload()
_archive_failed["reason"] = "rhythm"
_archive_failed["archive_write_failed"] = True
EXTRA_V02_PAYLOAD_POSITIVES["context-compressed.archive-write-failed.valid"] = (
    _archive_failed
)

# ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6/§14.9): the
# `change` shape of request_header_change — carries the mechanical
#「变化原因」`change_kind` and `previous_header_sha256` (both schema-required
# when reason=change).
EXTRA_V02_PAYLOAD_POSITIVES["request-header-change.change.valid"] = {
    "reason": "change",
    "header_sha256": "1" * 64,
    "system_sha256": ZERO_HASH,
    "tools_sha256": "2" * 64,
    "config_sha256": ZERO_HASH,
    "agent_role": "main",
    "previous_header_sha256": ZERO_HASH,
    "change_kind": "tools",
    "tools": ["read_file"],
    "tool_count": 1,
}

# P0-C S3 前置 (2026-08-15, P1-2 定案): structured policy denial on
# tool_completed — schema-valid positive (source=retrieval_mode on a
# retrieval-mode-gated tool, non-zero exit_code) and one schema constraint
# violation (unknown source; the cross-layer exit_code/tool-family rules are
# exercised in runtime/tests/test_run_event_journal_validation.py).
EXTRA_V02_PAYLOAD_POSITIVES["tool-completed.policy-denial.valid"] = {
    "tool": "project_doc_index",
    "call_id": "call-pd1",
    "exit_code": 1,
    "status": "error",
    "error": "retrieval_mode_off",
    "policy_denial": {
        "source": "retrieval_mode",
        "code": "retrieval_mode_off",
        "reason": "retrieval mode is 'off' for this session (ADR-0010 "
        "§3.7.1); no retrieval tools are available.",
    },
}
EXTRA_V02_PAYLOAD_BADS["tool-completed.policy-denial-bad-source.constraint.invalid"] = {
    "tool": "project_doc_index",
    "call_id": "call-pd1",
    "exit_code": 1,
    "status": "error",
    "error": "retrieval_mode_off",
    "policy_denial": {
        "source": "policy_engine",
        "code": "retrieval_mode_off",
        "reason": "unknown source",
    },
}

# TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §3.6): W-F13b truncation
# positive — truncated long output with byte count and retrieval-object
# pointer; the pairing rules live in _verify_v02_output_truncation.
EXTRA_V02_PAYLOAD_POSITIVES["tool-completed.output-object.valid"] = {
    "tool": "run_terminal_cmd",
    "call_id": "call-term-long-1",
    "exit_code": None,
    "running": True,
    "output_truncated": True,
    "total_bytes": 1_048_576,
    "output_object_id": "outobj-run-term-long-1",
}

# TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §3.1/§3.2): idle-kill positive
# and its schema constraint violation (status without reason — the
# if/then pairing rule in the tool-running payload schema).
EXTRA_V02_PAYLOAD_POSITIVES["tool-running.idle-killed.valid"] = {
    "tool": "run_terminal_cmd",
    "call_id": "call-term-1",
    "wall_ms": 600_123,
    "task_id": "call-term-1",
    "pid": 42,
    "total_bytes": 8_192,
    "output_file": "/tmp/terminal/call-term-1.log",
    "status": "idle_killed",
    "reason": "no output growth or CPU activity for 300s",
}
EXTRA_V02_PAYLOAD_BADS["tool-running.idle-killed-missing-reason.constraint.invalid"] = {
    "tool": "run_terminal_cmd",
    "call_id": "call-term-1",
    "wall_ms": 600_123,
    "task_id": "call-term-1",
    "pid": 42,
    "total_bytes": 8_192,
    "output_file": "/tmp/terminal/call-term-1.log",
    "status": "idle_killed",
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


def _envelope_v02(event_type: str, payload: dict, sequence: int, previous: str | None) -> dict:
    slug = SLUGS_V02.get(event_type) or SLUGS[event_type]
    identity = V02_ENVELOPE_IDENTITY_OVERRIDES.get(event_type, (None, None, None))
    run_id = identity[0] or f"RUN-CONF-{slug.upper()}"
    event_id = identity[1] or f"EVT-CONF-{sequence:03d}"
    timestamp = identity[2] or V02_ENVELOPE_TIMESTAMP_OVERRIDES.get(event_type, TIMESTAMP)
    return {
        "schema_version": "0.2.0-draft",
        "run_id": run_id,
        "event_id": event_id,
        "sequence": sequence,
        "timestamp": timestamp,
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
  legal / one-constraint-violation payloads for the v0.2 mechanism events
  with their own v0.2 payload schema: `orientation_checkpoint`
  (v0.2 shape), `information_sufficiency_assessment`,
  `retrieval_parent_disposition`,
  `retrieval_close_record`, plus the GAP-RETRIEVAL-TOOLS trio
  `retrieval_mode_transition`, `retrieval_result_committed`,
  `retrieval_activation_restored`, plus the ACAF trio (Slice 1, 2026-08-12)
  `control_ticket_issued`, `control_ticket_consumed`,
  `control_ticket_rejected` (ticket lifecycle binding fields — never the
  HMAC tag, which stays inside the issuing process), plus
  `tool_availability_check` (FUS-TOOL-PROBE 2026-08-13, P0-A-2: two-state
  single probe face snapshot — complete/incomplete cover ALL work tools).
- P2-11 DC 清理 (2026-08-31): `diagnostic_coverage_checkpoint` and
  `checkpoint_response` are retired — the diagnostic-coverage forced-template
  mechanism is deleted (MODEL-RESIDUAL-PRESSURE-FOLLOWUP 裁决 2); their
  payload schemas and fixtures are removed from the v0.2 track.
- ACAF Slice 2 fail-closed (2026-08-13): `control-ticket-rejected` gains
  three extra positive payload fixtures for the new pre-signing reject codes
  `missing_target_argument` / `missing_snapshot_store` / `missing_goal_context`
  (each carries `ticket_id: null` — no ticket exists when the refusal
  happens, D-14/D-15).
- GAP-EVENT-SCHEMA-DRIFT (2026-08-26, BACKLOG 0i): 事件面三类 Schema 漂移
  修复的 fixture 锁——① `retrieval-mode-transition.mechanical-degrade.valid`
  （模式 A 自动降级：authority=mechanical_probe +
  reason_code=browser_launch_failed，RETRIEVAL-SUBAGENT-WIRING §14.40）；
  ② `ledger-fold-advance` 最小正例/约束反例补 `view_estimate_after`
  （推进后视图估算，schema 必填 + verifier 交叉检查 < view_estimate_tokens）；
  ③ `control-ticket-issued.network-lane-bound.valid`（D-13 检索 lane 绑定
  语义——动作票 activation_id 可选绑定，检索 lane network 票携带真实
  activation_id，主 lane 保持 null）。
- P0-0x S2 (2026-09-11, ADR-0010 §14.66)：`orientation-checkpoint` 的
  **第二个合法 trigger 值**——`orientation-checkpoint.initial-round.valid` 把
  `trigger=initial_round` 与 `[INITIAL_ROUND_INQUIRY` 块、`post_tool_batch_gap`
  三点对齐（一次性初始轮中立问询）；枚举负例沿用
  `orientation-checkpoint.constraint.invalid`（trigger=manual）。trigger ↔
  message_block ↔ injection_position 的耦合与「会话内恰好一次」是
  **家族级**规则（schema 表达不了），由 Rust/Python 双方
  `initial_round_inquiry` 族执法。
- P0-0v P2-3/P2-4 (2026-09-10, 0x S2 重捕补齐登记)：`tool-completed.serp-budget-exceeded`
  / `tool-completed.serp-session-reserved` 两个 browser_control SERP 拒绝计数
  正例——0v 落地时未登记生成器条目，重跑生成器会静默删除；现已登记。
- 0t (2026-09-09, ADR-0010 §14.65 / 设计 v1.3)：`retrieval_mode_transition`
  生产者侧退役——事件类型与 v0.2 payload schema 保留为旧 v0.2 刊只读回放
  （旧 fixture `retrieval-mode-transition.*` 与旧期刊均继续合法，不设负向
  检查）；新增 `browser_launch_result` 事实事件（浏览器每次启动/探活尝试
  success/failure + 真实原因，`attempt_id`/`status`/`cause` 条件约束），
  覆盖 envelope + payload 最小正例/约束反例。
- GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): `retrieval-result` gains one extra
  negative payload fixture for the fixed tier/weight table (authoritative
  MUST pair with 1.1; the good fixture carries the full weighting fields).
- P0-D S1 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6):
  `context_recovery_truncated` — the D2-2 recovery pre-check event
  (before/after estimates, dropped rounds and the run-journal audit path
  of the full sidecar copy).
- P0-D S3 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §4):
  `context_compressed` moves to a v0.2 payload shape — the five-section
  template summary (mode/reason, archive id/digest/path, completeness and
  retained tail; the v0.1 file stays authoritative for the v0.1 replay
  track).
- P0-D S5 (2026-08-14, ADR-0010 v1.14): extra `context_compressed` payload
  positives — `session_end` reason, `guard_failed` (guard-retry force) and
  `archive_write_failed` (explicit archive-write failure).
- ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6/§14.9):
  `request_header_change` — the model-request header fingerprint event
  (initial/change per loop invocation; `change_kind` carries the mechanical
  「变化原因」). Extra fixtures cover the `change` shape and the schema rule
  that `reason=change` must carry `change_kind` +
  `previous_header_sha256`.
- P0-C S3 前置 (2026-08-15, P1-2 定案): `tool_completed` gains the optional
  structured `policy_denial` object (source ∈ permission | acaf |
  retrieval_mode | taint; non-zero exit_code; tool must belong to the
  source's known refusal path). Extra fixtures:
  `tool-completed.policy-denial.valid` (retrieval-mode refusal shape) and
  `tool-completed.policy-denial-bad-source.constraint.invalid` (unknown
  source enum).
- `envelope/<slug>.valid.json` — a full 13-field v0.2 envelope for **every**
  event in the v0.2 enum (**55 events** — 54 prior +
  `browser_launch_result`（0t 2026-09-09, ADR-0010 §14.65；54 prior −
  `diagnostic_coverage_checkpoint` − `checkpoint_response`（P2-11 DC 清理
  2026-08-31，MODEL-RESIDUAL-PRESSURE-FOLLOWUP 裁决 2）后含其余
  TER/P2-13 增量；`runtime_stagnation_guard` 已于 2026-08-22 退役). The
  v0.2-payload events carry
  their v0.2 payload; the other events reuse the v0.1 payload shape
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
  The cross-validator resolves the v0.2-payload events to their v0.2
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
  capability scenario. Re-captured again 2026-08-14 (P0-B step 4:
  browser_read joins the candidate count domain — `tool_completed` carries
  `candidate_count`/`candidate_cap`). MECHANICAL-AUDIT-LAYER (2026-08-24,
  ADR-0010 §14.39): 引用校验器整体删除——`citation-validation-block` 场景
  退役；`mechanical_audit_update` 为事件面轻量留痕（对象键覆盖写），
  报告随最终答案前中立问询轮注入且不进归档：

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
| `local-browser-read.jsonl` | bootstrap transition to `local_browser` with capability `available` — external lane runs the host `browser_read` tool (fake lane), committed result carries real full-text `web_page` evidence (ADR §3.7.3/§3.7.5); P0-B step 4: browser_read 计入候选计数域，`tool_completed` 携带 `candidate_count`/`candidate_cap` |
| `real-doc-retrieval.jsonl` | internal lane: `project_doc_index` include_content → mechanical ledger/visibility/`retrieval_result_committed`/assessment (ADR §3.7.4/§3.7.5) |
| `cross-prompt-restore.jsonl` | activation sidecar restore → restore event → cross-run disposition close (verifier restore-declaration chain) |
"""


def write_json(path: Path, data: dict) -> None:
    path.write_text(
        json.dumps(data, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
        newline="\n",
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

    fixture_root.joinpath("README.md").write_text(
        FIXTURES_README, encoding="utf-8", newline="\n"
    )

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
        payload = V02_ENVELOPE_PAYLOAD_OVERRIDES.get(event_type)
        if payload is None:
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

    v02_root.joinpath("README.md").write_text(
        FIXTURES_README_V02, encoding="utf-8", newline="\n"
    )

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
