"""Run-event journal cross-validation — the Phase 3 #7 conformance suite.

The final validator of the Python reference-spec contract
(`architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md`): validates REAL run
journals (e.g. produced by the Rust implementation) line-by-line against the
run-event envelope schema, per-event payload schemas selected by the
`payload_schema` TRACK string (contract §5 — never by event_type alone),
and the full hash chain.

Hash canonical form is the RUST parity form — `json.dumps(..., ensure_ascii=False,
sort_keys=True, separators=(",", ":"))` mirrors orz-assurance `journal/chain.rs`
(recursive key sort, compact, raw UTF-8). This is deliberately NOT
`assurance.utils.canonical_bytes` (RFC 8785, used by the self-contained Python
journal producers); the captured real journals are the parity test — a digest
mismatch on a captured journal is a real bug on one side.

Parity scope (matches the Rust verifier): `event_id` format, timestamp
parseability and `schema_version` are NOT checked beyond the envelope schema
itself plus the chain rules.

Single registry source: `PAYLOAD_SCHEMA_BY_EVENT_TYPE` (34 events, the 3
dual-track slugs pointing at the `runtime/` Rust-track files) is consumed by
this module, `runtime/tests/test_run_event_conformance.py` and
`scripts/check_repository.py`.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "runtime"
ASSURANCE = ROOT / "assurance"

RUN_EVENT_SCHEMA = RUNTIME / "run-event-v0.1.schema.json"
RUN_EVENT_SCHEMA_V02 = RUNTIME / "run-event-v0.2.schema.json"

# 34-event registry (v0.1 historical freeze; the retired
# `runtime_stagnation_guard` stays registered for v0.1 replay — the v0.2
# producer no longer writes it). The three
# dual-track slugs resolve to the runtime/ Rust-track files (slice #17
# adjudication — contract §6); `tool_belief_stagnation` stays assurance-only
# (Rust never constructs it).
PAYLOAD_SCHEMA_BY_EVENT_TYPE: dict[str, tuple[str, Path]] = {
    "run_preflight": ("run-preflight", RUNTIME / "run-preflight-event-payload-v0.1.schema.json"),
    "run_started": ("run-started", RUNTIME / "run-started-event-payload-v0.1.schema.json"),
    "prompt_submitted": ("prompt-submitted", RUNTIME / "prompt-submitted-event-payload-v0.1.schema.json"),
    "model_request": ("model-request", RUNTIME / "model-request-event-payload-v0.1.schema.json"),
    "model_response_received": ("model-response-received", RUNTIME / "model-response-received-event-payload-v0.1.schema.json"),
    "model_output": ("model-output", RUNTIME / "model-output-event-payload-v0.1.schema.json"),
    "acp_initialize": ("acp-initialize", RUNTIME / "acp-initialize-event-payload-v0.1.schema.json"),
    "acp_session_created": ("acp-session-created", RUNTIME / "acp-session-created-event-payload-v0.1.schema.json"),
    "tool_proposal": ("tool-proposal", RUNTIME / "tool-proposal-event-payload-v0.1.schema.json"),
    "permission_requested": ("permission-requested", RUNTIME / "permission-requested-event-payload-v0.1.schema.json"),
    "permission_decision": ("permission-decision", RUNTIME / "permission-decision-event-payload-v0.1.schema.json"),
    "tool_started": ("tool-started", RUNTIME / "tool-started-event-payload-v0.1.schema.json"),
    "tool_completed": ("tool-completed", RUNTIME / "tool-completed-event-payload-v0.1.schema.json"),
    # dual-track: Rust track (runtime/) — the assurance/ twins remain the
    # orientation-track shapes (contract §6, slice #17).
    "orientation_checkpoint": ("orientation-checkpoint", RUNTIME / "orientation-checkpoint-event-payload-v0.1.schema.json"),
    "runtime_stagnation_guard": ("runtime-stagnation-guard", RUNTIME / "runtime-stagnation-guard-event-payload-v0.1.schema.json"),
    "tool_availability_check": ("tool-availability-check", RUNTIME / "tool-availability-check-event-payload-v0.1.schema.json"),
    "tool_belief_stagnation": ("tool-belief-stagnation", ASSURANCE / "tool-belief-stagnation-event-payload-v0.1.schema.json"),
    "instruction_provenance_gate": ("instruction-provenance-gate", RUNTIME / "instruction-provenance-gate-event-payload-v0.1.schema.json"),
    "gate_decision": ("gate-decision", RUNTIME / "gate-decision-event-payload-v0.1.schema.json"),
    "neutral_inquiry": ("neutral-inquiry", RUNTIME / "neutral-inquiry-event-payload-v0.1.schema.json"),
    "counterexample_gate": ("counterexample-gate", RUNTIME / "counterexample-gate-event-payload-v0.1.schema.json"),
    "retrieval_completion_check": ("retrieval-completion-check", RUNTIME / "retrieval-completion-check-event-payload-v0.1.schema.json"),
    # A6 (2026-08-08): explicit context compaction event — controller-written.
    "context_compressed": ("context-compressed", RUNTIME / "context-compressed-event-payload-v0.1.schema.json"),
    "snapshot_created": ("snapshot-created", RUNTIME / "snapshot-created-event-payload-v0.1.schema.json"),
    "snapshot_restored": ("snapshot-restored", RUNTIME / "snapshot-restored-event-payload-v0.1.schema.json"),
    "artifact_registered": ("artifact-registered", RUNTIME / "artifact-registered-event-payload-v0.1.schema.json"),
    "plan_proposed": ("plan-proposed", RUNTIME / "plan-proposed-event-payload-v0.1.schema.json"),
    "plan_approved": ("plan-approved", RUNTIME / "plan-approved-event-payload-v0.1.schema.json"),
    "plan_rejected": ("plan-rejected", RUNTIME / "plan-rejected-event-payload-v0.1.schema.json"),
    "action_approved": ("action-approved", RUNTIME / "action-approved-event-payload-v0.1.schema.json"),
    "run_finished": ("run-finished", RUNTIME / "run-finished-event-payload-v0.1.schema.json"),
    "run_failed": ("run-failed", RUNTIME / "run-failed-event-payload-v0.1.schema.json"),
    "run_cancelled": ("run-cancelled", RUNTIME / "run-cancelled-event-payload-v0.1.schema.json"),
    "run_invalidated": ("run-invalidated", RUNTIME / "run-invalidated-event-payload-v0.1.schema.json"),
}

# v0.2 track (Phase B, ADR-0010 §11.2/§5.2): the five events with their own
# v0.2 payload schema. Any other event on the v0.2 track resolves to its v0.1
# payload schema file (payload shapes unchanged — adjudicated decision, see
# runtime/fixtures/run-event-v0.2/README.md).
# GAP-RETRIEVAL-TOOLS (2026-08-10): +retrieval_mode_transition,
# +retrieval_result_committed, +retrieval_activation_restored (ADR §3.7.1/§3.3.3).
PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02: dict[str, tuple[str, Path]] = {
    # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): terminal command
    # auto-backgrounded at the 300s report point — mid-run status between
    # the call's tool_started and tool_completed (at most one per call_id).
    "tool_running": (
        "tool-running",
        RUNTIME / "tool-running-event-payload-v0.2.schema.json",
    ),
    # TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §10-S0): tool_completed
    # moves to a v0.2 payload shape on the v0.2 track — W-F13b truncation
    # fields (output_truncated / total_bytes / output_object_id). The v0.1
    # file stays authoritative for the v0.1 replay track (payload schema
    # version bump v0.1 → v0.2).
    "tool_completed": (
        "tool-completed",
        RUNTIME / "tool-completed-event-payload-v0.2.schema.json",
    ),
    # TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §3.3/§10-S0): F6 push 档
    # 中性预算提示——剩余秒/已用轮/触发档位；默认 off 零注入，每 run
    # ≤4 次（_verify_v02_budget_cue_injected 强制）。
    "budget_cue_injected": (
        "budget-cue-injected",
        RUNTIME / "budget-cue-injected-event-payload-v0.2.schema.json",
    ),
    "orientation_checkpoint": (
        "orientation-checkpoint",
        RUNTIME / "orientation-checkpoint-event-payload-v0.2.schema.json",
    ),
    "information_sufficiency_assessment": (
        "information-sufficiency-assessment",
        RUNTIME / "information-sufficiency-assessment-event-payload-v0.2.schema.json",
    ),
    "retrieval_parent_disposition": (
        "retrieval-parent-disposition",
        RUNTIME / "retrieval-parent-disposition-event-payload-v0.2.schema.json",
    ),
    "retrieval_close_record": (
        "retrieval-close-record",
        RUNTIME / "retrieval-close-record-event-payload-v0.2.schema.json",
    ),
    "retrieval_mode_transition": (
        "retrieval-mode-transition",
        RUNTIME / "retrieval-mode-transition-event-payload-v0.2.schema.json",
    ),
    "retrieval_result_committed": (
        "retrieval-result",
        RUNTIME / "retrieval-result-event-payload-v0.2.schema.json",
    ),
    "retrieval_activation_restored": (
        "retrieval-activation-restored",
        RUNTIME / "retrieval-activation-restored-event-payload-v0.2.schema.json",
    ),
    # MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / 设计 §2.4):
    # 机械审查层轻量事件留痕——对象键覆盖写（含 step/契约类）以一条记录
    # 留痕；报告块随最终答案前中立问询轮注入且不进归档。主车道专属。
    "mechanical_audit_update": (
        "mechanical-audit-update",
        RUNTIME / "mechanical-audit-update-event-payload-v0.2.schema.json",
    ),
    # ACAF Slice 1 (设计文档 §4.2/§4.6): control-ticket lifecycle events —
    # issued → consumed|rejected pairing enforced by _verify_v02_control_tickets.
    "control_ticket_issued": (
        "control-ticket-issued",
        RUNTIME / "control-ticket-issued-event-payload-v0.2.schema.json",
    ),
    "control_ticket_consumed": (
        "control-ticket-consumed",
        RUNTIME / "control-ticket-consumed-event-payload-v0.2.schema.json",
    ),
    "control_ticket_rejected": (
        "control-ticket-rejected",
        RUNTIME / "control-ticket-rejected-event-payload-v0.2.schema.json",
    ),
    # FUS-TOOL-PROBE (2026-08-13; P0-A-2 2026-08-13): tool_availability_check
    # moves to the two-state work-tool probe shape on the v0.2 track (old
    # available/unavailable/degraded/unprobed shape stays v0.1 replay-only).
    "tool_availability_check": (
        "tool-availability-check",
        RUNTIME / "tool-availability-check-event-payload-v0.2.schema.json",
    ),
    # ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6/§14.9): the
    # model-request header fingerprint — system+tools+config digests,
    # emitted only on initial/change per agent lane.
    "request_header_change": (
        "request-header-change",
        RUNTIME / "request-header-change-event-payload-v0.2.schema.json",
    ),
    # D2-2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6):
    # recovery pre-check truncation of a restored conversation — written by
    # the controller between prompt_submitted and the first model_request.
    "context_recovery_truncated": (
        "context-recovery-truncated",
        RUNTIME / "context-recovery-truncated-event-payload-v0.2.schema.json",
    ),
    # P0-D S3 (2026-08-14): template-summary payload on the v0.2 track (the
    # v0.1 file stays authoritative for the v0.1 replay track).
      "context_compressed": (
          "context-compressed",
          RUNTIME / "context-compressed-event-payload-v0.2.schema.json",
      ),
      # F7 (2026-08-15, BACKLOG 6e 复查遗留 / ADR-0010 §14.15): blackboard
      # plan-epoch archive write failure — audit trace when a rotation
      # committed but the durable snapshot is missing.
      "epoch_archive_write_failed": (
          "epoch-archive-write-failed",
          RUNTIME / "epoch-archive-write-failed-event-payload-v0.2.schema.json",
      ),
      # P2-13 B3 (2026-09-03, ADR-0010 §14.52 / 设计 §12 R3): conversation
      # archived at session close as a single gzip package (pure packaging,
      # atomic write + digest; once per session close).
      "session_archive": (
          "session-archive",
          RUNTIME / "session-archive-event-payload-v0.2.schema.json",
      ),
      # PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): first-round plan
      # gate result — plan identity/goal/step count, mechanical validation,
      # one-refill attempt progression and degrade reason.
      "plan_write": (
          "plan-write",
          RUNTIME / "plan-write-event-payload-v0.2.schema.json",
      ),
      # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console/direct
      # dual-mode transition decision record (PLAN_FIRST_BLACKBOARD_DESIGN §7).
      "console_mode_transition": (
          "console-mode-transition",
          RUNTIME / "console-mode-transition-event-payload-v0.2.schema.json",
      ),
      # PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): the action-bar
      # order record — order identity, step binding and mechanical stamps
      # (PLAN_FIRST_BLACKBOARD_DESIGN §5-§6).
      "console_order_written": (
          "console-order-written",
          RUNTIME / "console-order-written-event-payload-v0.2.schema.json",
      ),
      # P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): pre-issuance
      # rejection of a written order — order identity, envelope step,
      # phase (pre_issue/issue), rejection code and reason; the receipt
      # stays the human-readable view (PLAN_FIRST_BLACKBOARD_DESIGN §5-§6).
      "console_order_rejected": (
          "console-order-rejected",
          RUNTIME / "console-order-rejected-event-payload-v0.2.schema.json",
      ),
      # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): mechanical
      # action-ledger fold advance — the request-view prefix is rewritten
      # once per fold window, so the event carries the new fold point, the
      # folded-round count and the triggering view estimate for cache-miss
      # attribution (LEDGER_FOLD_STATE_CACHE_DESIGN §3.3/§8).
      "ledger_fold_advance": (
          "ledger-fold-advance",
          RUNTIME / "ledger-fold-advance-event-payload-v0.2.schema.json",
      ),
      # FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.28 审查修复):
      # external-ledger append failure — fold rollback + consecutive failure
      # count + budget-exhaustion disable (the audit trace of the degrade
      # path that keeps the session moving with the unfolded view).
      "ledger_fold_write_failed": (
          "ledger-fold-write-failed",
          RUNTIME / "ledger-fold-write-failed-event-payload-v0.2.schema.json",
      ),
      # MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3):
      # transport 重试计数事件面——一次逻辑模型请求内零 chunk/中段截断
      # 重试的 recovered/exhausted 摘要（成功随 ModelResponse 上报、
      # 耗尽随 StreamInterrupted 上报）。v0.2 专用新事件（v0.1 为历史
      # 冻结面，不扩展）。
      "transport_retry": (
          "transport-retry",
          RUNTIME / "transport-retry-event-payload-v0.2.schema.json",
      ),
  }

# Track-resolution table (contract §5 enforcement): every registered
# `payload_schema` string outside the Rust track, mapped to the payload
# schema file it corresponds to (`None` = registered envelope-only producer).
# The Rust-track string `"run-event-v0.1.schema.json"` and the
# `#runtime_event_payload` fragment form are handled in `_resolve_payload_schema`.
PRODUCER_SCHEMAS: dict[str, Path | None] = {
    # orientation track — suffixed filename references (never the runtime/
    # dual-track twins).
    "orientation-checkpoint-event-payload-v0.1.schema.json": ASSURANCE
    / "orientation-checkpoint-event-payload-v0.1.schema.json",
    "runtime-stagnation-guard-event-payload-v0.1.schema.json": ASSURANCE
    / "runtime-stagnation-guard-event-payload-v0.1.schema.json",
    "tool-availability-check-event-payload-v0.1.schema.json": ASSURANCE
    / "tool-availability-check-event-payload-v0.1.schema.json",
    "tool-belief-stagnation-event-payload-v0.1.schema.json": ASSURANCE
    / "tool-belief-stagnation-event-payload-v0.1.schema.json",
    # canonical-cli track — de-suffixed identifiers.
    "canonical-cli-preflight-v0.1": ASSURANCE / "canonical-cli-preflight-v0.1.schema.json",
    "canonical-cli-run-started-v0.1": ASSURANCE / "canonical-cli-run-started-v0.1.schema.json",
    "canonical-cli-fake-model-request-v0.1": ASSURANCE / "canonical-cli-fake-model-request-v0.1.schema.json",
    "canonical-cli-real-model-request-v0.1": ASSURANCE / "canonical-cli-real-model-request-v0.1.schema.json",
    "canonical-cli-fake-model-output-v0.1": ASSURANCE / "canonical-cli-fake-model-output-v0.1.schema.json",
    "canonical-cli-real-model-output-v0.1": ASSURANCE / "canonical-cli-real-model-output-v0.1.schema.json",
    "canonical-cli-terminal-v0.1": ASSURANCE / "canonical-cli-terminal-v0.1.schema.json",
    "instruction-provenance-gate-receipt-v0.1": ASSURANCE
    / "instruction-provenance-gate-receipt-v0.1.schema.json",
    "tool-availability-check-event-payload-v0.1": ASSURANCE
    / "tool-availability-check-event-payload-v0.1.schema.json",
    "orientation-checkpoint-event-payload-v0.1": ASSURANCE
    / "orientation-checkpoint-event-payload-v0.1.schema.json",
    "source-visibility-gate-receipt-v0.1": ASSURANCE
    / "source-visibility-gate-receipt-v0.1.schema.json",
    "canonical-cli-answer-packet-v0.1": ASSURANCE / "canonical-cli-answer-packet-v0.1.schema.json",
    # cli session lifecycle track.
    "cli-session-lifecycle-event-v0.2.schema.json": RUNTIME
    / "cli-session-lifecycle-event-v0.2.schema.json",
    # Registered envelope-only producers (no payload schema by design).
    "grok-runtime-normalized-v0.1": None,
    "deepseek-runtime-normalized-v0.1": None,
}

# The gsa-runtime-preflight fragment form: file + JSON-pointer fragment into
# the schema document (the fragment target was added in slice #17).
GSA_PREFLIGHT_FRAGMENT = "gsa-runtime-preflight-projection-v0.1.schema.json#runtime_event_payload"
GSA_PREFLIGHT_SCHEMA = ASSURANCE / "gsa-runtime-preflight-projection-v0.1.schema.json"

_TERMINAL_TYPES = {"run_finished", "run_failed", "run_cancelled", "run_invalidated"}

# The 12 fields Rust's chain.rs projects when recomputing event_sha256.
_EVENT_HASH_FIELDS = (
    "schema_version",
    "run_id",
    "event_id",
    "sequence",
    "timestamp",
    "event_type",
    "run_manifest_sha256",
    "previous_event_sha256",
    "payload_schema",
    "payload",
    "payload_sha256",
    "redaction",
)


# ── canonical hashing (Rust parity form) ─────────────────────────────

def _canonical_bytes(value: Any) -> bytes:
    """Rust-parity canonical JSON bytes (chain.rs) — NOT RFC 8785."""
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _payload_sha256(payload: dict[str, Any]) -> str:
    return _sha256_hex(_canonical_bytes(payload))


def _event_sha256(event: dict[str, Any]) -> str:
    projected = {key: event[key] for key in _EVENT_HASH_FIELDS}
    return _sha256_hex(_canonical_bytes(projected))


# ── schema resolution ────────────────────────────────────────────────

def _resolve_payload_schema(event: dict[str, Any]) -> tuple[str, Path | None]:
    """Resolve the payload schema for one event by its `payload_schema`
    string (contract §5: track identifier, never event_type). Returns
    ("payload", path) or ("envelope_only", None)."""
    producer = event.get("payload_schema")
    if producer == "run-event-v0.2.schema.json":
        event_type = event.get("event_type")
        if event_type in PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02:
            return ("payload", PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02[event_type][1])
        if event_type not in PAYLOAD_SCHEMA_BY_EVENT_TYPE:
            raise ValueError(
                f"unknown event_type on the Rust v0.2 track: {event_type!r}"
            )
        # v0.2 events without their own payload schema reuse the v0.1 payload
        # schema file (payload shapes unchanged — Phase B adjudication).
        return ("payload", PAYLOAD_SCHEMA_BY_EVENT_TYPE[event_type][1])
    if producer == "run-event-v0.1.schema.json":
        event_type = event.get("event_type")
        if event_type not in PAYLOAD_SCHEMA_BY_EVENT_TYPE:
            raise ValueError(
                f"unknown event_type on the Rust track: {event_type!r}"
            )
        return ("payload", PAYLOAD_SCHEMA_BY_EVENT_TYPE[event_type][1])
    if producer == GSA_PREFLIGHT_FRAGMENT:
        return ("pointer", GSA_PREFLIGHT_SCHEMA)
    if producer in PRODUCER_SCHEMAS:
        schema = PRODUCER_SCHEMAS[producer]
        if schema is None:
            return ("envelope_only", None)
        return ("payload", schema)
    raise ValueError(
        f"unknown payload_schema string: {producer!r}; "
        f"registered producers: {sorted(PRODUCER_SCHEMAS)}"
    )


_envelope_validators: dict[Path, Draft202012Validator] = {}
_schema_cache: dict[Path, dict[str, Any]] = {}


def _load_schema(path: Path) -> dict[str, Any]:
    if path not in _schema_cache:
        _schema_cache[path] = json.loads(path.read_text(encoding="utf-8"))
    return _schema_cache[path]


def _envelope_validator_for(event: dict[str, Any]) -> Draft202012Validator:
    """The envelope version is selected by the track string: v0.2 events are
    validated against run-event-v0.2.schema.json, everything else against
    run-event-v0.1.schema.json (a v0.2-only producer must never be silently
    checked against the v0.1 enum)."""
    schema_path = (
        RUN_EVENT_SCHEMA_V02
        if event.get("payload_schema") == "run-event-v0.2.schema.json"
        else RUN_EVENT_SCHEMA
    )
    validator = _envelope_validators.get(schema_path)
    if validator is None:
        validator = Draft202012Validator(
            _load_schema(schema_path), format_checker=FormatChecker()
        )
        _envelope_validators[schema_path] = validator
    return validator


def _envelope_errors(event: dict[str, Any], index: int) -> list[str]:
    return [
        f"envelope schema violation at event {index}: {error.message}"
        for error in _envelope_validator_for(event).iter_errors(event)
    ]


def _payload_errors(event: dict[str, Any], index: int) -> list[str]:
    mode, target = _resolve_payload_schema(event)
    if mode == "envelope_only":
        return []
    schema = _load_schema(target)
    if mode == "pointer":
        subschema = schema["runtime_event_payload"]
    else:
        subschema = schema
    validator = Draft202012Validator(subschema, format_checker=FormatChecker())
    return [
        f"payload schema violation at event {index}: {error.message}"
        for error in validator.iter_errors(event.get("payload"))
    ]


# ── v0.2 mechanism cross-checks (ADR-0010 §5.1/§4.4) ─────────────────

_V02_NEUTRAL_INQUIRY_EVENTS = frozenset({"orientation_checkpoint"})


def _is_v02(event: dict[str, Any]) -> bool:
    return event.get("payload_schema") == "run-event-v0.2.schema.json"


def _verify_v02_inquiry_kind(events: list[dict[str, Any]]) -> list[str]:
    """§5.1: the two neutral-inquiry events carry a const `inquiry_kind` in
    their payload that must equal the envelope `event_type` — the payload
    schema alone cannot express the cross-layer equality."""
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        if event_type in _V02_NEUTRAL_INQUIRY_EVENTS:
            payload = event.get("payload", {})
            inquiry_kind = payload.get("inquiry_kind")
            if inquiry_kind != event_type:
                errors.append(
                    f"event {index}: v0.2 neutral inquiry payload inquiry_kind "
                    f"{inquiry_kind!r} != envelope event_type {event_type!r}"
                )
    return errors


_RETRIEVAL_TARGETS = frozenset({"internal_retrieval", "external_retrieval"})

# Host-lane retrieval tools whose tool events carry NO `target` field
# (D-2, 2026-08-10 local_browser slice): `browser_read` runs on the host
# lane, so the target-based dispatch rules below cannot see it — a
# successful browser_read under off / unavailable capability would slip
# past exactly the "no silent fallback" the mode rules enforce. web_fetch /
# web_search are NOT listed: they dispatch through the external lane and
# their events carry target=external_retrieval (covered by the target
# checks).
_HOST_LANE_RETRIEVAL_TOOLS = frozenset({"browser_read"})


def _verify_v02_plan_write(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD_DESIGN §3-§5 cross-checks
    (2026-08-16 审查收口):

    - `refill_requested` only on attempt 1 and must be followed by a second
      plan_write event (the single refill);
    - `degraded` with `validation_failed_after_refill` only on attempt 2;
    - `accepted` requires `validation.valid=true`;
    - mechanical degrades (`plan_rotate_failed` / `plan_not_submitted`)
      carry no structural validation errors (`validation.valid=true`),
      while `validation_failed`-family degrades require it to be false;
    - later plan_write events after the gate closes are plan revisions and
      are not constrained by gate attempt sequencing.
    """
    errors: list[str] = []
    writes = [
        (index, event)
        for index, event in enumerate(events)
        if _is_v02(event) and event.get("event_type") == "plan_write"
    ]
    for position, (index, event) in enumerate(writes):
        payload = event["payload"]
        attempt = payload["attempt"]
        outcome = payload["outcome"]
        degrade_reason = payload.get("degrade_reason")
        valid = payload.get("validation", {}).get("valid")
        if outcome == "refill_requested":
            if attempt != 1:
                errors.append(
                    f"event {index}: plan_write refill_requested on attempt "
                    f"{attempt} (only attempt 1 may request a refill)"
                )
            if position + 1 >= len(writes):
                errors.append(
                    f"event {index}: plan_write refill_requested without a "
                    "following refill attempt"
                )
            if valid is not False:
                errors.append(
                    f"event {index}: plan_write refill_requested but "
                    "validation.valid is not false"
                )
        if outcome == "degraded":
            if degrade_reason == "validation_failed_after_refill":
                if attempt != 2:
                    errors.append(
                        f"event {index}: plan_write "
                        "validation_failed_after_refill degrade on attempt "
                        f"{attempt} (degrade only after the refill attempt)"
                    )
                if valid is not False:
                    errors.append(
                        f"event {index}: plan_write "
                        "validation_failed_after_refill but validation.valid "
                        "is not false"
                    )
            elif degrade_reason in ("plan_rotate_failed", "plan_not_submitted"):
                if valid is not True:
                    errors.append(
                        f"event {index}: plan_write {degrade_reason} carries "
                        "structural validation errors but the failure is "
                        "mechanical (validation.valid must be true)"
                    )
            elif degrade_reason == "validation_failed":
                if valid is not False:
                    errors.append(
                        f"event {index}: plan_write revision validation_failed "
                        "but validation.valid is not false"
                    )
        if outcome == "accepted":
            if valid is not True:
                errors.append(
                    f"event {index}: plan_write accepted but "
                    "validation.valid is not true"
                )
            if attempt not in (1, 2):
                errors.append(
                    f"event {index}: plan_write accepted on attempt {attempt} "
                    "(expected 1 or 2)"
                )
    return errors


def _verify_v02_console_mode_transition(
    events: list[dict[str, Any]],
) -> list[str]:
    """ADR-0010 §14.17⑱ / PLAN_FIRST_BLACKBOARD_DESIGN §7 cross-checks
    (PLAN-FIRST 阶段 C, 2026-08-16):

    - console→direct requires trigger=assistant_failure_streak and
      model_decision=switch (streak ≥ 1, order_ids present);
    - direct→console requires trigger=model_return,
      model_decision=return_to_console and related_transition_id matching an
      earlier console→direct transition of the same run;
    - the stay decision (to=console, from=console) requires
      trigger=assistant_failure_streak and model_decision=stay;
    - at most one assistant_failure_streak decision per run (the design's
      '每 run 至多询问一次' — switch or stay);
    - a direct-mode tool event (console_mode=direct) must be preceded by a
      console→direct transition of the same run and carry the transition_id
      of that transition.
    """
    errors: list[str] = []
    transitions = [
        (index, event)
        for index, event in enumerate(events)
        if _is_v02(event) and event.get("event_type") == "console_mode_transition"
    ]
    per_run_streak_decisions: dict[str, int] = {}
    direct_transitions_by_run: dict[str, dict[str, int]] = {}
    seen_transition_ids: dict[str, set[str]] = {}
    for index, event in transitions:
        payload = event["payload"]
        run_id = payload["run_id"]
        t_from = payload["from"]
        t_to = payload["to"]
        trigger = payload["trigger"]
        decision = payload["model_decision"]
        transition_id = payload["transition_id"]
        seen = seen_transition_ids.setdefault(run_id, set())
        if transition_id in seen:
            errors.append(
                f"event {index}: duplicate console_mode_transition "
                f"transition_id {transition_id!r} in run {run_id}"
            )
        seen.add(transition_id)
        if t_from == "console" and t_to == "direct":
            if trigger != "assistant_failure_streak":
                errors.append(
                    f"event {index}: console→direct transition requires "
                    f"trigger=assistant_failure_streak, got {trigger!r}"
                )
            if decision != "switch":
                errors.append(
                    f"event {index}: console→direct transition requires "
                    f"model_decision=switch, got {decision!r}"
                )
            if not isinstance(payload.get("streak"), int) or payload["streak"] < 1:
                errors.append(
                    f"event {index}: console→direct transition requires "
                    "streak ≥ 1"
                )
            if not isinstance(payload.get("order_ids"), list) or not payload[
                "order_ids"
            ]:
                errors.append(
                    f"event {index}: console→direct transition requires "
                    "non-empty order_ids"
                )
            if payload.get("related_transition_id") is not None:
                errors.append(
                    f"event {index}: console→direct transition requires "
                    "related_transition_id=null"
                )
            direct_transitions_by_run.setdefault(run_id, {})[transition_id] = index
        elif t_from == "direct" and t_to == "console":
            if trigger != "model_return":
                errors.append(
                    f"event {index}: direct→console transition requires "
                    f"trigger=model_return, got {trigger!r}"
                )
            if decision != "return_to_console":
                errors.append(
                    f"event {index}: direct→console transition requires "
                    f"model_decision=return_to_console, got {decision!r}"
                )
            related = payload.get("related_transition_id")
            if not related:
                errors.append(
                    f"event {index}: direct→console transition requires "
                    "related_transition_id"
                )
            elif run_id not in direct_transitions_by_run or related not in (
                direct_transitions_by_run[run_id]
            ):
                errors.append(
                    f"event {index}: direct→console related_transition_id "
                    f"{related!r} does not match an earlier console→direct "
                    "transition of the same run"
                )
        elif t_from == "console" and t_to == "console":
            if trigger != "assistant_failure_streak":
                errors.append(
                    f"event {index}: stay decision requires "
                    f"trigger=assistant_failure_streak, got {trigger!r}"
                )
            if decision != "stay":
                errors.append(
                    f"event {index}: stay decision requires "
                    f"model_decision=stay, got {decision!r}"
                )
            if payload.get("related_transition_id") is not None:
                errors.append(
                    f"event {index}: stay decision requires "
                    "related_transition_id=null"
                )
        else:
            errors.append(
                f"event {index}: invalid console mode direction "
                f"{t_from!r} → {t_to!r}"
            )
        if trigger == "assistant_failure_streak":
            per_run_streak_decisions[run_id] = (
                per_run_streak_decisions.get(run_id, 0) + 1
            )
    for run_id, count in per_run_streak_decisions.items():
        if count > 1:
            errors.append(
                f"run {run_id}: {count} assistant_failure_streak console mode "
                "decisions (at most one inquiry per run)"
            )

    # Direct-mode tool events must carry the current direct transition id.
    current_direct: dict[str, str] = {}
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        if event.get("event_type") == "console_mode_transition":
            payload = event["payload"]
            run_id = payload["run_id"]
            if payload["from"] == "console" and payload["to"] == "direct":
                current_direct[run_id] = payload["transition_id"]
            elif payload["from"] == "direct" and payload["to"] == "console":
                current_direct.pop(run_id, None)
            continue
        # THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2 审查处理 P2-3):
        # tool_running 属 direct 模式工具事件——必须携带当前 direct
        # transition_id，与 tool_started/tool_completed 同口径关联。
        if event.get("event_type") not in (
            "tool_started",
            "tool_completed",
            "tool_running",
        ):
            continue
        payload = event.get("payload", {})
        if payload.get("console_mode") == "direct":
            run_id = event.get("run_id", "")
            tid = payload.get("transition_id")
            expected = current_direct.get(run_id)
            if expected is None:
                errors.append(
                    f"event {index}: direct-mode tool event without a "
                    "preceding console→direct transition in its run"
                )
            elif tid != expected:
                errors.append(
                    f"event {index}: direct-mode tool event transition_id "
                    f"{tid!r} != current direct transition {expected!r}"
                )
    return errors


def _verify_v02_console_order_written(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §14.17⑱ / PLAN_FIRST_BLACKBOARD_DESIGN §5-§6 cross-checks
    (PLAN-FIRST 阶段 C, 2026-08-16):

    - every console_order_written carries the mechanical
      round/plan_epoch/run_id stamps and the write_call_id of the
      blackboard_action_write completion it closes;
    - order ids are unique per run;
    - a console_order_written must be preceded by a blackboard_action_write
      tool_completed success of the same run whose call_id matches the
      write_call_id (the producer ordering: write completion, then the
      order record); each write completion backs at most one order record.
    """
    errors: list[str] = []
    order_ids_per_run: dict[str, set[str]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "console_order_written":
            continue
        payload = event["payload"]
        run_id = event.get("run_id", "")
        order_id = payload["order_id"]
        seen = order_ids_per_run.setdefault(run_id, set())
        if order_id in seen:
            errors.append(
                f"event {index}: duplicate console_order_written order_id "
                f"{order_id!r} in run {run_id}"
            )
        seen.add(order_id)
        action = payload.get("action")
        if not isinstance(action, str) or not action:
            errors.append(
                f"event {index}: console_order_written action must be a "
                "non-empty string"
            )
        step_id = payload.get("step_id")
        if step_id is not None and (not isinstance(step_id, str) or not step_id):
            errors.append(
                f"event {index}: console_order_written step_id must be null "
                "or a non-empty string"
            )
    # Producer ordering: the action_write success completion precedes the
    # order record in the same run; the order record carries the write's
    # call_id (write_call_id), and one write backs at most one order record.
    # (2026-08-16 review closure F1: match against write_call_id instead of
    # order_id — the producer stamps the model's real call_id on the write
    # completion while order_id is the internal ORD-xxxxx identity.)
    writes_per_run: dict[str, list[tuple[int, str]]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "tool_completed":
            continue
        payload = event.get("payload", {})
        if payload.get("tool") == "blackboard_action_write" and payload.get(
            "exit_code"
        ) == 0:
            writes_per_run.setdefault(event.get("run_id", ""), []).append(
                (index, str(payload.get("call_id", "")))
            )
    consumed: dict[str, set[int]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "console_order_written":
            continue
        payload = event["payload"]
        run_id = event.get("run_id", "")
        write_call_id = payload.get("write_call_id")
        matched: int | None = None
        for wi, (w_index, w_call) in enumerate(writes_per_run.get(run_id, [])):
            if wi in consumed.setdefault(run_id, set()):
                continue
            if w_index < index and w_call == write_call_id:
                matched = wi
                break
        if matched is None:
            errors.append(
                f"event {index}: console_order_written {payload['order_id']!r} "
                "without a prior blackboard_action_write success in its run"
            )
        else:
            consumed[run_id].add(matched)
    return errors


def _verify_v02_console_order_rejected(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §14.21 项 3 / PLAN_FIRST_BLACKBOARD_DESIGN §5-§6 cross-checks
    (P0-E 第 4 项, 2026-08-17):

    - every console_order_rejected carries the rejected order's mechanical
      stamps (round/plan_epoch/run_id) and a phase/step/code triple:
      * phase=pre_issue → step=protocol and code ∈ {order_stale,
        step_not_done, budget_insufficient} (refused before issuance);
      * phase=issue → step ∈ {registry, contract, target, policy}
        (refused at issuance before any execution; ACAF/mode/permission
        denials normalize to step=policy / code=policy_denied);
      execute/verify steps never appear here — executed orders journal
      their outcome through tool_started/tool_completed.
    - a console_order_rejected must be preceded by a console_order_written
      of the same run carrying the same order_id and the same
      round/plan_epoch/run_id stamps (the order record precedes its
      rejection; the stamps are the order's own);
    - at most one console_order_rejected per order_id per run (a rejected
      order is consumed and never re-issued).
    """
    errors: list[str] = []
    written_by_run: dict[str, dict[str, dict[str, Any]]] = {}
    for event in events:
        if not _is_v02(event) or event.get("event_type") != "console_order_written":
            continue
        payload = event["payload"]
        run_id = event.get("run_id", "")
        written_by_run.setdefault(run_id, {})[payload["order_id"]] = payload

    rejected_per_run: dict[str, set[str]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "console_order_rejected":
            continue
        payload = event["payload"]
        run_id = event.get("run_id", "")
        order_id = payload["order_id"]
        seen = rejected_per_run.setdefault(run_id, set())
        if order_id in seen:
            errors.append(
                f"event {index}: duplicate console_order_rejected order_id "
                f"{order_id!r} in run {run_id}"
            )
        seen.add(order_id)

        phase = payload.get("phase")
        step = payload.get("step")
        code = payload.get("code")
        if phase not in ("pre_issue", "issue"):
            errors.append(
                f"event {index}: console_order_rejected phase must be "
                f"pre_issue or issue, got {phase!r}"
            )
        if phase == "pre_issue":
            if step != "protocol":
                errors.append(
                    f"event {index}: console_order_rejected pre_issue phase "
                    f"requires step=protocol, got {step!r}"
                )
            if code not in ("order_stale", "step_not_done", "budget_insufficient"):
                errors.append(
                    f"event {index}: console_order_rejected pre_issue code "
                    f"must be order_stale/step_not_done/budget_insufficient, "
                    f"got {code!r}"
                )
        elif phase == "issue":
            if step not in ("registry", "contract", "target", "policy"):
                errors.append(
                    f"event {index}: console_order_rejected issue phase step "
                    f"must be registry/contract/target/policy, got {step!r}"
                )
        reason = payload.get("reason")
        if not isinstance(reason, str) or not reason:
            errors.append(
                f"event {index}: console_order_rejected reason must be a "
                "non-empty string"
            )

        written = written_by_run.get(run_id, {}).get(order_id)
        if written is None:
            errors.append(
                f"event {index}: console_order_rejected {order_id!r} without "
                "a prior console_order_written of the same run"
            )
            continue
        for stamp in ("round", "plan_epoch", "run_id"):
            if payload.get(stamp) != written.get(stamp):
                errors.append(
                    f"event {index}: console_order_rejected {order_id!r} "
                    f"{stamp}={payload.get(stamp)!r} != written order "
                    f"{stamp}={written.get(stamp)!r}"
                )
    return errors


def _verify_v02_ledger_fold_advance(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §14.26 / LEDGER_FOLD_STATE_CACHE_DESIGN §3.3-§3.5
    cross-checks (FUS-LEDGER-FOLD-STATE, 2026-08-18 review fix):

    - every ledger_fold_advance carries a valid fold point: fold_start and
      fold_cut are non-negative integers with fold_start < fold_cut (the
      design invariant), rounds_folded ≥ 1, view_estimate_tokens ≥ 0 and
      agent_role in the lane enum (main / internal_retrieval /
      external_retrieval);
    - GAP-EVENT-SCHEMA-DRIFT (2026-08-26): view_estimate_after must be a
      non-negative integer and strictly below view_estimate_tokens — a real
      advance resets the post-fold estimate below the fold trigger, so the
      triggering estimate (≥ threshold) strictly dominates the after value
      (the schema makes the field required; this is the semantic
      cross-check);
    - within a fold window (bounded by context_compressed events of the
      same run) fold_start is CONSTANT (set at the first advance) and
      fold_cut is STRICTLY increasing (each advance moves the retention
      start forward; the anti-spin guard refuses kept_start <= fold_cut);
      rounds_folded is non-decreasing (the ledger never shrinks inside a
      window);
    - a context_compressed event resets the fold state — the next
      ledger_fold_advance starts a fresh window and may legitimately use
      any indices (re-accumulation from the marker).
    """
    errors: list[str] = []
    # Per-run fold-window state: (fold_start, fold_cut, rounds_folded) or
    # None after a compaction reset (the next advance starts a fresh window).
    window: dict[str, tuple[int, int, int] | None] = {}
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        etype = event.get("event_type")
        run_id = event.get("run_id", "")
        if etype == "context_compressed":
            window[run_id] = None
            continue
        if etype != "ledger_fold_advance":
            continue
        payload = event["payload"]
        fold_start = payload.get("fold_start")
        fold_cut = payload.get("fold_cut")
        rounds_folded = payload.get("rounds_folded")
        estimate = payload.get("view_estimate_tokens")
        estimate_after = payload.get("view_estimate_after")
        agent_role = payload.get("agent_role")
        if not isinstance(fold_start, int) or fold_start < 0:
            errors.append(
                f"event {index}: ledger_fold_advance fold_start must be a "
                f"non-negative integer, got {fold_start!r}"
            )
        if not isinstance(fold_cut, int) or fold_cut < 0:
            errors.append(
                f"event {index}: ledger_fold_advance fold_cut must be a "
                f"non-negative integer, got {fold_cut!r}"
            )
        if isinstance(fold_start, int) and isinstance(fold_cut, int) and not (
            fold_start < fold_cut
        ):
            errors.append(
                f"event {index}: ledger_fold_advance invariant "
                f"fold_start < fold_cut violated ({fold_start} >= {fold_cut})"
            )
        if not isinstance(rounds_folded, int) or rounds_folded < 1:
            errors.append(
                f"event {index}: ledger_fold_advance rounds_folded must be "
                f"a positive integer, got {rounds_folded!r}"
            )
        if not isinstance(estimate, int) or estimate < 0:
            errors.append(
                f"event {index}: ledger_fold_advance view_estimate_tokens "
                f"must be a non-negative integer, got {estimate!r}"
            )
        if not isinstance(estimate_after, int) or estimate_after < 0:
            errors.append(
                f"event {index}: ledger_fold_advance view_estimate_after "
                f"must be a non-negative integer, got {estimate_after!r}"
            )
        elif isinstance(estimate, int) and estimate_after >= estimate:
            errors.append(
                f"event {index}: ledger_fold_advance view_estimate_after "
                f"{estimate_after} must be below the triggering estimate "
                f"{estimate} (a real advance resets below the fold trigger)"
            )
        if agent_role not in ("main", "internal_retrieval", "external_retrieval"):
            errors.append(
                f"event {index}: ledger_fold_advance agent_role must be "
                f"main/internal_retrieval/external_retrieval, got {agent_role!r}"
            )
        prev = window.get(run_id)
        if prev is not None:
            prev_start, prev_cut, prev_rounds = prev
            if isinstance(fold_start, int) and fold_start != prev_start:
                errors.append(
                    f"event {index}: ledger_fold_advance fold_start changed "
                    f"inside a fold window ({prev_start} -> {fold_start})"
                )
            if isinstance(fold_cut, int) and fold_cut <= prev_cut:
                errors.append(
                    f"event {index}: ledger_fold_advance fold_cut must "
                    f"increase inside a fold window (previous {prev_cut}, "
                    f"got {fold_cut})"
                )
            if isinstance(rounds_folded, int) and rounds_folded < prev_rounds:
                errors.append(
                    f"event {index}: ledger_fold_advance rounds_folded "
                    f"shrank inside a fold window "
                    f"({prev_rounds} -> {rounds_folded})"
                )
        if (
            isinstance(fold_start, int)
            and isinstance(fold_cut, int)
            and isinstance(rounds_folded, int)
        ):
            window[run_id] = (fold_start, fold_cut, rounds_folded)
    return errors


def _verify_v02_ledger_fold_write_failed(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §14.28 审查修复 (FUS-LEDGER-FOLD-STATE external-file design)
    cross-checks — the external-ledger append failure audit trace:

    - every ledger_fold_write_failed carries the full audit shape: a
      non-empty ledger_path, attempt >= 1, disabled bool, rows >= 1,
      view_estimate_tokens >= 0 and agent_role in the lane enum (folding is
      main-lane only — the producer always writes "main");
    - within a run, consecutive failures increase attempt by exactly 1 (the
      producer increments the counter per failure); a successful append
      (a ledger_fold_advance event) resets the counter, so the next failure
      starts a fresh burst at attempt 1;
    - the disabled flag follows the producer budget
      (FOLD_WRITE_FAILURE_LIMIT = 3, ADR-0010 §14.28): disabled ==
      (attempt >= 3), and no further write failures may follow once the
      budget is exhausted (folding is disabled for the rest of the loop).
    """
    errors: list[str] = []
    # Per-run failure-burst state: (previous attempt, budget exhausted).
    bursts: dict[str, tuple[int, bool]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        etype = event.get("event_type")
        run_id = event.get("run_id", "")
        if etype == "ledger_fold_advance":
            # A successful append resets the consecutive-failure counter —
            # the next write failure starts a fresh burst at attempt 1.
            bursts[run_id] = (0, False)
            continue
        if etype != "ledger_fold_write_failed":
            continue
        payload = event["payload"]
        ledger_path = payload.get("ledger_path")
        attempt = payload.get("attempt")
        disabled = payload.get("disabled")
        rows = payload.get("rows")
        estimate = payload.get("view_estimate_tokens")
        agent_role = payload.get("agent_role")
        if not isinstance(ledger_path, str) or not ledger_path:
            errors.append(
                f"event {index}: ledger_fold_write_failed ledger_path must "
                f"be a non-empty string, got {ledger_path!r}"
            )
        if not isinstance(attempt, int) or attempt < 1:
            errors.append(
                f"event {index}: ledger_fold_write_failed attempt must be a "
                f"positive integer, got {attempt!r}"
            )
        if not isinstance(disabled, bool):
            errors.append(
                f"event {index}: ledger_fold_write_failed disabled must be "
                f"a boolean, got {disabled!r}"
            )
        if not isinstance(rows, int) or rows < 1:
            errors.append(
                f"event {index}: ledger_fold_write_failed rows must be a "
                f"positive integer, got {rows!r}"
            )
        if not isinstance(estimate, int) or estimate < 0:
            errors.append(
                f"event {index}: ledger_fold_write_failed "
                f"view_estimate_tokens must be a non-negative integer, got "
                f"{estimate!r}"
            )
        if agent_role not in ("main", "internal_retrieval", "external_retrieval"):
            errors.append(
                f"event {index}: ledger_fold_write_failed agent_role must "
                f"be main/internal_retrieval/external_retrieval, got "
                f"{agent_role!r}"
            )
        prev_attempt, exhausted = bursts.get(run_id, (0, False))
        if exhausted:
            errors.append(
                f"event {index}: ledger_fold_write_failed after the failure "
                f"budget was exhausted (disabled=true) for run {run_id!r} — "
                "folding is disabled, no further events"
            )
        if isinstance(attempt, int) and attempt != prev_attempt + 1:
            errors.append(
                f"event {index}: ledger_fold_write_failed attempt must "
                f"increase by 1 across consecutive failures (previous "
                f"{prev_attempt}, got {attempt})"
            )
        if isinstance(attempt, int) and isinstance(disabled, bool):
            # Producer budget: FOLD_WRITE_FAILURE_LIMIT = 3 (ADR-0010 §14.28).
            if disabled != (attempt >= 3):
                errors.append(
                    f"event {index}: ledger_fold_write_failed disabled must "
                    f"equal (attempt >= 3) per the producer budget, got "
                    f"attempt={attempt} disabled={disabled}"
                )
            bursts[run_id] = (attempt, disabled)
    return errors


def _verify_v02_retrieval_mode(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §3.7.1 mechanical mode facts on the v0.2 track:

    - a session_bootstrap transition occurs at most once per journal;
    - a transition's old_mode equals the previous transition's new_mode
      (mode is one session-level value; the first transition has no in-journal
      predecessor because the mode is set before the run);
    - after a transition to off, no retrieval dispatch (tool_started with a
      retrieval target), no assessment and no committed result may follow —
      parent dispositions of already-pending activations stay legal;
    - after a transition to local_browser, the obligation depends on the
      transition's capability_status (2026-08-10 local_browser slice): with
      `available`, successful dispatches and committed results are legal;
      with `unsupported`/`degraded`, every retrieval dispatch must fail
      explicitly (no silent fallback) and no committed result may follow;
      a missing capability_status on a local_browser transition is itself
      an error — the schema's allOf constrains the VALUE when present but
      does not require presence, so this check is the presence gate
      (2026-08-10 review: corrected the earlier "schema double backstop"
      wording).
    """
    errors: list[str] = []
    transitions: list[tuple[int, dict[str, Any]]] = []
    for index, event in enumerate(events):
        if _is_v02(event) and event.get("event_type") == "retrieval_mode_transition":
            transitions.append((index, event))

    bootstrap_count = 0
    current_mode: str | None = None
    for position, (index, event) in enumerate(transitions):
        payload = event["payload"]
        old_mode = payload["old_mode"]
        new_mode = payload["new_mode"]
        if payload["authority"] == "session_bootstrap":
            bootstrap_count += 1
        if current_mode is not None and old_mode != current_mode:
            errors.append(
                f"event {index}: transition old_mode {old_mode!r} != previous "
                f"transition new_mode {current_mode!r}"
            )
        current_mode = new_mode

        next_transition = (
            transitions[position + 1][0] if position + 1 < len(transitions) else len(events)
        )
        if new_mode == "off":
            for j in range(index + 1, next_transition):
                payload_j = events[j].get("payload", {})
                if events[j].get("event_type") == "tool_started" and (
                    payload_j.get("target") in _RETRIEVAL_TARGETS
                    or payload_j.get("tool") in _HOST_LANE_RETRIEVAL_TOOLS
                ):
                    errors.append(
                        f"event {j}: retrieval dispatch {payload_j.get('tool')} "
                        f"after transition to off (event {index})"
                    )
                elif events[j].get("event_type") == "information_sufficiency_assessment":
                    errors.append(
                        f"event {j}: assessment on activation "
                        f"{payload_j.get('activation_id')} after transition to "
                        f"off (event {index})"
                    )
                elif events[j].get("event_type") == "retrieval_result_committed":
                    errors.append(
                        f"event {j}: committed result after transition to off "
                        f"(event {index})"
                    )
        elif new_mode == "local_browser":
            capability_status = payload.get("capability_status")
            if capability_status not in ("available", "unsupported", "degraded"):
                errors.append(
                    f"event {index}: capability_status {capability_status!r} "
                    f"missing/invalid on local_browser transition — the schema "
                    f"requires one of available/unsupported/degraded"
                )
                continue
            for j in range(index + 1, next_transition):
                payload_j = events[j].get("payload", {})
                if capability_status != "available":
                    # Capability unavailable → every dispatch must fail
                    # explicitly; a committed result is impossible (no silent
                    # fallback — ADR-0010 §3.7.1/§3.7.2).
                    if events[j].get("event_type") == "retrieval_result_committed":
                        errors.append(
                            f"event {j}: committed result under local_browser "
                            f"mode with capability_status={capability_status} "
                            f"(event {index}) — capability unavailable, no "
                            f"silent fallback allowed"
                        )
                    elif (
                        events[j].get("event_type") == "tool_completed"
                        and (
                            payload_j.get("target") in _RETRIEVAL_TARGETS
                            or payload_j.get("tool") in _HOST_LANE_RETRIEVAL_TOOLS
                        )
                        and payload_j.get("status") != "error"
                    ):
                        errors.append(
                            f"event {j}: retrieval dispatch "
                            f"{payload_j.get('tool')} completed non-error under "
                            f"local_browser mode with "
                            f"capability_status={capability_status} (event "
                            f"{index}) — capability unavailable must fail "
                            f"explicitly"
                        )
                # capability_status == "available": successful dispatches and
                # committed results are legal — no obligation beyond the
                # shared rules above.
    if bootstrap_count > 1:
        errors.append(
            f"journal has {bootstrap_count} session_bootstrap transitions — "
            f"at most one allowed"
        )
    return errors


# GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 fixed
# tier/weight table — the mechanical judge's three multipliers (relative
# ranking, not 0-1 confidence). GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C
# (2026-08-30): the model annotation vocabulary is retired — only the
# mechanical tier/weight facts remain.
_WEIGHT_BY_TIER = {
    "authoritative": 1.1,
    "default": 1.0,
    "low_quality": 0.7,
}


def _verify_v02_result_consistency(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §3.3.3/§3.7.5 mechanical facts on committed retrieval results:

    - source_counts equal the visibility distribution mechanically counted
      from the commit's own source_ledger;
    - visibility_degraded is exactly "no text-level evidence" (zero
      full_text_observed and zero partial_text_observed — metadata-only
      declarations or no tool calls; GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C);
    - an assessment on the same (activation_id, contract_revision) after the
      commit must carry identical result_digest/ledger_digest/source_counts;
    - the commit precedes its assessment.
    """
    errors: list[str] = []
    commits: list[tuple[int, dict[str, Any]]] = []
    for index, event in enumerate(events):
        if _is_v02(event) and event.get("event_type") == "retrieval_result_committed":
            commits.append((index, event))

    assessments_by_key: dict[tuple[str, int], list[tuple[int, dict[str, Any]]]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "information_sufficiency_assessment":
            continue
        p = event["payload"]
        assessments_by_key.setdefault((p["activation_id"], p["contract_revision"]), []).append(
            (index, event)
        )

    for index, event in commits:
        p = event["payload"]
        ledger = p["source_ledger"]
        counted: dict[str, int] = {
            "full_text_observed": 0,
            "partial_text_observed": 0,
            "metadata_only": 0,
            "unavailable": 0,
        }
        for entry in ledger:
            vis = entry["visibility"]
            if vis in counted:
                counted[vis] += 1
            else:
                errors.append(
                    f"event {index}: source {entry.get('source_id')} has unknown "
                    f"visibility {vis!r}"
                )
        expected_counts = {**counted, "total": len(ledger)}
        if p["source_counts"] != expected_counts:
            errors.append(
                f"event {index}: source_counts {p['source_counts']} != mechanical "
                f"distribution {expected_counts} of source_ledger"
            )

        # D1 (review 2026-08-10): highest_allowed_claim is a mechanical
        # projection of visibility (§3.7.5) — a declared value that does not
        # match the matrix means the ledger lies about its own cap.
        _HIGHEST_CLAIM_BY_VISIBILITY = {
            "full_text_observed": "observed",
            "partial_text_observed": "derived",
            "metadata_only": "synthesized",
            "unavailable": "none",
        }
        for entry in ledger:
            declared = entry.get("highest_allowed_claim")
            if declared is None:
                continue
            expected = _HIGHEST_CLAIM_BY_VISIBILITY.get(entry["visibility"])
            if expected is None:
                continue  # unknown visibility already reported above
            if declared != expected:
                errors.append(
                    f"event {index}: source {entry['source_id']} "
                    f"highest_allowed_claim {declared!r} != mechanical cap "
                    f"{expected!r} for visibility {entry['visibility']!r}"
                )
        # GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): the committed
        # payload must not carry the retired organized_response, and
        # visibility_degraded must equal "no text-level evidence" — the
        # mechanical definition that replaced the organized-block fallback.
        if "organized_response" in p:
            errors.append(
                f"event {index}: organized_response is retired "
                "(GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C)"
            )
        no_text_evidence = (
            p["source_counts"]["full_text_observed"] == 0
            and p["source_counts"]["partial_text_observed"] == 0
        )
        if bool(p["visibility_degraded"]) != no_text_evidence:
            errors.append(
                f"event {index}: visibility_degraded {p['visibility_degraded']} "
                "!= no-text-evidence flag "
                f"{no_text_evidence} (full={p['source_counts']['full_text_observed']}, "
                f"partial={p['source_counts']['partial_text_observed']})"
            )
        # GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C 全面审查轮 (2026-08-30):
        # result_digest must be the canonical SHA-256 of the four mechanical
        # segments (query_summary/source_ledger/filtering_log/
        # raw_source_refs) and ledger_digest of source_ledger — a stale
        # digest (e.g. the retired five-segment form) means the payload lies
        # about its own content; result_id must carry the digest prefix.
        expected_digest = _payload_sha256(
            {
                "query_summary": p["query_summary"],
                "source_ledger": p["source_ledger"],
                "filtering_log": p["filtering_log"],
                "raw_source_refs": p["raw_source_refs"],
            }
        )
        if p["result_digest"] != expected_digest:
            errors.append(
                f"event {index}: result_digest does not match the canonical "
                f"four-segment digest (expected {expected_digest[:16]}…)"
            )
        expected_ledger = _payload_sha256(p["source_ledger"])
        if p["ledger_digest"] != expected_ledger:
            errors.append(
                f"event {index}: ledger_digest does not match the canonical "
                f"source_ledger digest (expected {expected_ledger[:16]}…)"
            )
        if not p["result_id"].startswith(f"RET-RES-{p['result_digest'][:16]}-"):
            errors.append(
                f"event {index}: result_id {p['result_id']!r} does not carry "
                f"the result_digest prefix {p['result_digest'][:16]}"
            )

        for a_index, a_event in assessments_by_key.get(
            (p["activation_id"], p["contract_revision"]), []
        ):
            if a_index < index:
                continue
            a_payload = a_event["payload"]
            a_id = a_payload["assessment_id"]
            if a_payload.get("result_digest") != p["result_digest"]:
                errors.append(
                    f"event {a_index}: assessment {a_id} result_digest != "
                    f"committed result {p['result_id']} (event {index})"
                )
            if a_payload.get("ledger_digest") != p["ledger_digest"]:
                errors.append(
                    f"event {a_index}: assessment {a_id} ledger_digest != "
                    f"committed result {p['result_id']} (event {index})"
                )
            if not a_id.startswith(f"ASSESS-{p['result_digest'][:16]}-"):
                errors.append(
                    f"event {a_index}: assessment {a_id} id does not carry "
                    f"the committed result_digest prefix {p['result_digest'][:16]}"
                )
            a_codes = a_payload.get("reason_codes") or []
            if p["visibility_degraded"] and "no_fulltext_evidence" not in a_codes:
                errors.append(
                    f"event {a_index}: assessment {a_id} for a degraded "
                    "committed result must carry reason code "
                    "'no_fulltext_evidence'"
                )
            if not p["visibility_degraded"] and "no_fulltext_evidence" in a_codes:
                errors.append(
                    f"event {a_index}: assessment {a_id} claims "
                    "'no_fulltext_evidence' for a committed result with "
                    "text-level evidence"
                )
            if a_payload.get("source_counts") != p["source_counts"]:
                errors.append(
                    f"event {a_index}: assessment {a_id} source_counts "
                    f"{a_payload.get('source_counts')} != committed result "
                    f"{p['result_id']} (event {index})"
                )
    return errors


_RETRIEVAL_REASON_CODES_ALLOWED = frozenset(
    {"no_mechanical_coverage_requirement", "no_fulltext_evidence"}
)
_RETRIEVAL_REASON_CODES_RETIRED = frozenset(
    {
        "structured_result_validation_failed",
        "low_quality_source_without_annotation",
    }
)


def _verify_v02_reason_codes(events: list[dict[str, Any]]) -> list[str]:
    """GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C 全面审查轮 (2026-08-30):
    the v0.2 retrieval assessment reason_codes vocabulary is mechanical —
    only the producer's codes are legal; the block-era codes
    (structured_result_validation_failed / low_quality_source_without_
    annotation) are retired and must never reappear in fixtures or
    journals."""
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != (
            "information_sufficiency_assessment"
        ):
            continue
        codes = event["payload"].get("reason_codes") or []
        for code in codes:
            if code in _RETRIEVAL_REASON_CODES_RETIRED:
                errors.append(
                    f"event {index}: reason code {code!r} is retired "
                    "(GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C)"
                )
            elif code not in _RETRIEVAL_REASON_CODES_ALLOWED:
                errors.append(
                    f"event {index}: unknown retrieval assessment reason "
                    f"code {code!r} (allowed: "
                    f"{sorted(_RETRIEVAL_REASON_CODES_ALLOWED)})"
                )
    return errors


def _verify_v02_source_weighting(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §3.7 条 12 / FUS-SOURCE-WEIGHTING (GAP-SOURCE-WEIGHTING-IMPL):
    mechanical tier/weight facts on committed retrieval results:

    - web_page ledger entries MUST carry tier + mechanical_weight +
      weight_reason; the tier/weight pair is fixed (authoritative 1.1 /
      default 1.0 / low_quality 0.7);
    - GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): the layer-3
      model annotation vocabulary (model_weight / model_weight_reason /
      annotation_status) is retired and must be absent.
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "retrieval_result_committed":
            continue
        p = event["payload"]
        ledger = p["source_ledger"]

        for entry in ledger:
            sid = entry["source_id"]
            tier = entry.get("tier")
            mechanical_weight = entry.get("mechanical_weight")
            weight_reason = entry.get("weight_reason")
            has_any_mechanical = (
                tier is not None or mechanical_weight is not None or weight_reason is not None
            )
            if entry.get("source_type") == "web_page" and tier is None:
                errors.append(
                    f"event {index}: source {sid} is web_page but carries no mechanical tier"
                )
            elif has_any_mechanical:
                if tier is None or mechanical_weight is None or weight_reason is None:
                    errors.append(
                        f"event {index}: source {sid} has partial mechanical weighting fields"
                    )
                elif _WEIGHT_BY_TIER.get(tier) != mechanical_weight:
                    errors.append(
                        f"event {index}: source {sid} tier {tier!r} weight "
                        f"{mechanical_weight} violates the fixed tier/weight table"
                    )
            retired = [
                field
                for field in ("model_weight", "model_weight_reason", "annotation_status")
                if field in entry
            ]
            if retired:
                errors.append(
                    f"event {index}: source {sid} carries retired model "
                    f"annotation fields {retired} "
                    "(GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C)"
                )
    return errors


def _verify_v02_search_candidate_pool(events: list[dict[str, Any]]) -> list[str]:
    """FUS-RETRIEVAL-MECH B-1 (2026-08-13) + step 3 (2026-08-14): the
    web_search citation candidate pool on committed retrieval results.

    B-1:
    - candidate_urls may only appear on web_search_result ledger entries;
    - candidate_urls is a non-empty list of unique non-empty strings;
    - raw_source_refs mirrors the ledger entry's candidate_urls exactly.

    Step 3 (mechanical prefilter):
    - candidate_urls is the POST-PREFILTER retained pool and always travels
      with candidate_pool (per-candidate metadata: canonical_url/tier/
      mechanical_weight/weight_reason/relevance/form_reasons);
    - an EMPTY retained pool is legitimate ONLY as the result of mechanical
      purification — the source must carry at least one prefilter_log
      removal; empty with no removal is a contract violation (the producer
      never writes the fields for an empty raw pool);
    - candidate_pool mirrors candidate_urls in order and URL identity
      (pool[i]["url"] == candidate_urls[i]);
    - mechanical_weight must match its tier (authoritative 1.1 /
      default 1.0 / low_quality 0.7) — shape-consistent with the schema;
    - raw_source_refs mirrors both candidate_urls and candidate_pool.
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "retrieval_result_committed":
            continue
        p = event["payload"]
        ledger = p["source_ledger"]
        refs = p["raw_source_refs"]
        ledger_by_id = {entry["source_id"]: entry for entry in ledger}
        ref_by_id = {ref["source_id"]: ref for ref in refs}

        for entry in ledger:
            sid = entry["source_id"]
            candidates = entry.get("candidate_urls")
            pool = entry.get("candidate_pool")
            if candidates is None and pool is not None:
                errors.append(
                    f"event {index}: source {sid} carries candidate_pool "
                    "without candidate_urls (the fields travel together)"
                )
                continue
            if candidates is None:
                continue
            if entry.get("source_type") != "web_search_result":
                errors.append(
                    f"event {index}: source {sid} carries candidate_urls but "
                    f"source_type is {entry.get('source_type')!r} (only "
                    "web_search_result may carry a candidate pool)"
                )
            if not isinstance(candidates, list) or any(
                not isinstance(url, str) or not url for url in candidates
            ):
                errors.append(
                    f"event {index}: source {sid} candidate_urls must be a "
                    "list of non-empty strings"
                )
                continue
            if not candidates:
                removals_for_source = [
                    item
                    for item in p.get("prefilter_log", [])
                    if item.get("source_id") == sid
                ]
                if not removals_for_source:
                    errors.append(
                        f"event {index}: source {sid} candidate_urls is empty "
                        "with no prefilter_log removal — an empty retained "
                        "pool must be the result of mechanical purification"
                    )
            if len(set(candidates)) != len(candidates):
                errors.append(
                    f"event {index}: source {sid} candidate_urls contains duplicates"
                )
            if pool is None:
                errors.append(
                    f"event {index}: source {sid} candidate_urls without "
                    "candidate_pool (the prefiltered pool must carry "
                    "per-candidate metadata)"
                )
                continue
            if not isinstance(pool, list) or len(pool) != len(candidates):
                errors.append(
                    f"event {index}: source {sid} candidate_pool must mirror "
                    "candidate_urls exactly (same length, same order)"
                )
                continue
            tier_weight = {
                "authoritative": 1.1,
                "default": 1.0,
                "low_quality": 0.7,
            }
            for i, item in enumerate(pool):
                if not isinstance(item, dict):
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] is "
                        "not an object"
                    )
                    continue
                url = item.get("url")
                if url != candidates[i]:
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] url "
                        f"{url!r} does not match candidate_urls[{i}] "
                        f"{candidates[i]!r}"
                    )
                canonical = item.get("canonical_url")
                tier = item.get("tier")
                weight = item.get("mechanical_weight")
                weight_reason = item.get("weight_reason")
                relevance = item.get("relevance")
                form_reasons = item.get("form_reasons")
                if not isinstance(canonical, str) or not canonical:
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] "
                        "canonical_url must be a non-empty string"
                    )
                if tier not in tier_weight:
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] "
                        f"invalid tier {tier!r}"
                    )
                elif weight != tier_weight[tier]:
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] "
                        f"mechanical_weight {weight!r} does not match tier "
                        f"{tier!r}"
                    )
                if not isinstance(weight_reason, str) or not weight_reason:
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] "
                        "weight_reason must be a non-empty string"
                    )
                if relevance not in ("direct", "partial", "tangential"):
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] "
                        f"invalid relevance {relevance!r}"
                    )
                if (
                    not isinstance(form_reasons, list)
                    or any(
                        not isinstance(r, str) or not r for r in form_reasons
                    )
                ):
                    errors.append(
                        f"event {index}: source {sid} candidate_pool[{i}] "
                        "form_reasons must be a list of non-empty strings"
                    )
            ref = ref_by_id.get(sid)
            if (
                ref is None
                or ref.get("candidate_urls") != candidates
                or ref.get("candidate_pool") != pool
            ):
                errors.append(
                    f"event {index}: source {sid} candidate_urls/candidate_pool "
                    "not mirrored in raw_source_refs"
                )

        for ref in refs:
            entry = ledger_by_id.get(ref["source_id"])
            for field in ("candidate_urls", "candidate_pool"):
                if field in ref:
                    if entry is None or entry.get(field) != ref[field]:
                        errors.append(
                            f"event {index}: raw_source_refs {ref['source_id']} "
                            f"{field} does not match the ledger"
                        )
    return errors


_PREFILTER_REMOVAL_REASONS = {
    "bad_url",
    "login_wall",
    "redirect_chain",
    "duplicate_canonical",
    "duplicate_host",
}
_PREFILTER_DUPLICATE_REASONS = {"duplicate_canonical", "duplicate_host"}


def _verify_v02_candidate_prefilter(events: list[dict[str, Any]]) -> list[str]:
    """FUS-RETRIEVAL-MECH P0-B step 3 (2026-08-14): mechanical prefilter
    removal log on committed retrieval results:

    - prefilter_log is required whenever a web_search_result entry carries
      a candidate pool (may be empty when nothing was removed);
    - every entry references a web_search_result ledger entry that carries
      candidate_urls; (source_id, url, reason) is recorded at most once;
    - a URL removed for a non-duplicate reason (bad_url/login_wall/
      redirect_chain) must NOT appear in that source's retained pool;
      duplicate removals may reference a URL still retained (the first-seen
      copy stays).
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "retrieval_result_committed":
            continue
        p = event["payload"]
        ledger = p["source_ledger"]
        ledger_by_id = {entry["source_id"]: entry for entry in ledger}
        has_pool = any(
            entry.get("candidate_urls") is not None for entry in ledger
        )
        log = p.get("prefilter_log")
        if log is None:
            if has_pool:
                errors.append(
                    f"event {index}: prefilter_log missing while a candidate "
                    "pool is present"
                )
            continue
        if not isinstance(log, list):
            errors.append(
                f"event {index}: prefilter_log must be an array"
            )
            continue
        seen: set[tuple[str, str, str]] = set()
        retained_by_source: dict[str, list[str]] = {}
        for entry in ledger:
            candidates = entry.get("candidate_urls")
            if isinstance(candidates, list):
                retained_by_source[entry["source_id"]] = candidates
        for item in log:
            sid = item.get("source_id")
            url = item.get("url")
            reason = item.get("reason")
            canonical = item.get("canonical_url")
            action = item.get("action")
            if not isinstance(sid, str) or not isinstance(url, str) or not url:
                errors.append(
                    f"event {index}: prefilter_log entry needs source_id and "
                    "a non-empty url"
                )
                continue
            if reason not in _PREFILTER_REMOVAL_REASONS:
                errors.append(
                    f"event {index}: prefilter_log {sid} invalid removal "
                    f"reason {reason!r}"
                )
                continue
            if action != "removed":
                errors.append(
                    f"event {index}: prefilter_log {sid} action must be "
                    f"'removed', got {action!r}"
                )
            if canonical is not None and (
                not isinstance(canonical, str) or not canonical
            ):
                errors.append(
                    f"event {index}: prefilter_log {sid} canonical_url must "
                    "be a non-empty string when present"
                )
            key = (sid, url, reason)
            if key in seen:
                errors.append(
                    f"event {index}: duplicate prefilter_log removal "
                    f"{sid} {url} {reason}"
                )
            seen.add(key)
            entry = ledger_by_id.get(sid)
            if (
                entry is None
                or entry.get("source_type") != "web_search_result"
                or entry.get("candidate_urls") is None
            ):
                errors.append(
                    f"event {index}: prefilter_log {sid} does not reference a "
                    "web_search_result entry carrying a candidate pool"
                )
                continue
            if (
                reason not in _PREFILTER_DUPLICATE_REASONS
                and url in retained_by_source.get(sid, [])
            ):
                errors.append(
                    f"event {index}: prefilter_log {sid} removed {url} "
                    f"({reason}) but the URL is still retained"
                )
    return errors


def _is_web_fetch_tool(name: str) -> bool:
    """FUS-RETRIEVAL-MECH P0-B step 2: the web_fetch family — bare
    `web_fetch` and every `web_fetch_*` variant (mirrors the Rust relay's
    `is_web_fetch_tool` prefix boundary)."""
    return name == "web_fetch" or name.startswith("web_fetch_")


def _is_candidate_counted_tool(name: str) -> bool:
    """FUS-RETRIEVAL-MECH P0-B step 2/4: the candidate-counted family —
    web_fetch variants plus `browser_read` (local_browser second segment
    shares the SAME per-activation count domain; mirrors the Rust relay's
    `is_candidate_counted_tool`)."""
    return _is_web_fetch_tool(name) or name == "browser_read"


def _verify_v02_candidate_count(events: list[dict[str, Any]]) -> list[str]:
    """FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): candidate count
    fields on tool_completed events (web_fetch family + browser_read):

    - candidate_count/candidate_cap appear only on candidate-counted tools
      and only together;
    - 0 <= candidate_count <= candidate_cap, candidate_cap >= 1;
    - a non-error LANE candidate-counted completion (no `target` — the tool
      actually executed) must carry both fields; the dispatch wrapper
      completion (`target` present — the parent call answered by the
      subagent dispatch, not a fetch/read) must NOT carry them;
    - a cap-exceeded rejection carries status=error with
      error={web_fetch|browser_read}_candidate_cap_exceeded and
      candidate_count == candidate_cap (the refusal happens at the cap
      boundary), and must carry both fields.
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if event.get("event_type") != "tool_completed":
            continue
        p = event["payload"]
        tool = p.get("tool")
        has_count = "candidate_count" in p
        has_cap = "candidate_cap" in p
        if has_count != has_cap:
            errors.append(
                f"event {index}: tool_completed carries candidate_count but "
                "not candidate_cap (or vice versa) — the fields travel together"
            )
            continue
        if not has_count:
            if (
                p.get("status") != "error"
                and _is_candidate_counted_tool(tool)
                and p.get("target") is None
            ):
                errors.append(
                    f"event {index}: non-error lane candidate-counted completion for "
                    f"{tool!r} must carry candidate_count/candidate_cap"
                )
            elif p.get("error") in (
                "web_fetch_candidate_cap_exceeded",
                "browser_read_candidate_cap_exceeded",
            ):
                errors.append(
                    f"event {index}: cap-exceeded refusal must carry "
                    "candidate_count/candidate_cap"
                )
            continue
        if not _is_candidate_counted_tool(tool):
            errors.append(
                f"event {index}: tool {tool!r} carries candidate count "
                "fields (web_fetch/browser_read family only)"
            )
            continue
        count = p["candidate_count"]
        cap = p["candidate_cap"]
        if (
            not isinstance(count, int)
            or isinstance(count, bool)
            or count < 0
            or not isinstance(cap, int)
            or isinstance(cap, bool)
            or cap < 1
            or count > cap
        ):
            errors.append(
                f"event {index}: candidate_count {count!r} / candidate_cap "
                f"{cap!r} out of range — need 0 <= count <= cap, cap >= 1"
            )
        if p.get("error") in (
            "web_fetch_candidate_cap_exceeded",
            "browser_read_candidate_cap_exceeded",
        ):
            if p.get("status") != "error":
                errors.append(
                    f"event {index}: cap-exceeded refusal must be status=error"
                )
            if count != cap:
                errors.append(
                    f"event {index}: cap-exceeded refusal candidate_count "
                    f"{count!r} must equal candidate_cap {cap!r} (refusal "
                    "happens at the cap boundary)"
                )
    return errors


def _verify_v02_mechanical_audit(events: list[dict[str, Any]]) -> list[str]:
    """MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / 设计 §2.4):
    `mechanical_audit_update` 轻量事件留痕——每次对象键覆盖写（含
    step/契约类检查）以一条记录留痕，供回放与验证；报告块本身不进归档
    （与 counterexample 注入同语义）。规则：

    - kind ∈ {tool_result, plan_gate, budget}；
    - payload 必须携带 key（对象键，非空）/ round（非负整数）/ summary
      （非空机械事实摘要）/ anomaly（字符串或 null）；
    - 键形为 file:<path> / cmd:<call_id> / plan / budget / retrieval:<n>。
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "mechanical_audit_update":
            continue
        payload = event["payload"]
        kind = payload.get("kind")
        entry = payload.get("payload")
        if kind not in ("tool_result", "plan_gate", "budget"):
            errors.append(
                f"event {index}: mechanical_audit_update kind {kind!r} must be "
                "tool_result / plan_gate / budget"
            )
        if not isinstance(entry, dict):
            errors.append(
                f"event {index}: mechanical_audit_update needs a payload object"
            )
            continue
        key = entry.get("key")
        round_ = entry.get("round")
        summary = entry.get("summary")
        anomaly = entry.get("anomaly")
        if not isinstance(key, str) or not key:
            errors.append(
                f"event {index}: mechanical_audit_update key must be a non-empty string"
            )
        if not isinstance(round_, int) or round_ < 0:
            errors.append(
                f"event {index}: mechanical_audit_update round must be a non-negative integer"
            )
        if not isinstance(summary, str) or not summary:
            errors.append(
                f"event {index}: mechanical_audit_update summary must be a non-empty string"
            )
        if anomaly is not None and not isinstance(anomaly, str):
            errors.append(
                f"event {index}: mechanical_audit_update anomaly must be a string or null"
            )
    return errors


def _verify_v02_recovery_truncation(events: list[dict[str, Any]]) -> list[str]:
    """D2-2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6):
    a `context_recovery_truncated` event is the controller's pre-request
    recovery pre-check — it must appear after `prompt_submitted` and before
    the first `model_request`, and it must drop at least one whole round
    (a no-op recovery truncation is never journaled)."""
    errors: list[str] = []
    seen_model_request = False
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        if event_type == "model_request":
            seen_model_request = True
            continue
        if event_type != "context_recovery_truncated":
            continue
        if seen_model_request:
            errors.append(
                f"event {index}: context_recovery_truncated must precede the "
                "first model_request"
            )
        if event["payload"]["rounds_dropped"] <= 0:
            errors.append(
                f"event {index}: context_recovery_truncated must drop at "
                "least one whole round"
            )
    return errors


def _verify_v02_context_compressed(events: list[dict[str, Any]]) -> list[str]:
    """P0-D S3 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §4):
    the v0.2 `context_compressed` event is a five-section summary:
    - mode is `mechanical` for the 2026-08-18 B 定案 producer (ADR-0010
      §14.29 — compaction makes zero model calls) and `template_summary`
      for historical journals; reason one of rhythm/fallback;
    - a complete summary must carry a non-null archive id/digest/path;
    - the termination state (summary_incomplete=true) must carry null
      archive fields (no archive was written); a mechanical-mode event
      must never be incomplete.

    P0-D review fix (2026-08-14, ADR-0010 v1.14): reason may also be
    `session_end` (the end-of-session compaction); the reduction-guard
    retry/force path reports `guard_failed` (only on rhythm/fallback — the
    session-end compaction is deliberately forced and never reports a guard
    failure); an `archive_write_failed` report may only ride a COMPLETE
    summary (a failed summary never attempts the archive write)."""
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "context_compressed":
            continue
        payload = event["payload"]
        mode = payload.get("mode")
        if mode not in ("template_summary", "mechanical"):
            errors.append(
                f"event {index}: context_compressed mode must be "
                "template_summary/mechanical"
            )
        reason = payload.get("reason")
        if reason not in ("rhythm", "fallback", "session_end"):
            errors.append(
                f"event {index}: context_compressed reason must be "
                "rhythm/fallback/session_end"
            )
        guard_failed = payload.get("guard_failed", False)
        if guard_failed and reason == "session_end":
            errors.append(
                f"event {index}: guard_failed may only ride rhythm/fallback "
                "triggers, never session_end"
            )
        incomplete = payload.get("summary_incomplete", False)
        if mode == "mechanical" and incomplete:
            errors.append(
                f"event {index}: mechanical compaction must never be "
                "summary_incomplete (no model slots to fail)"
            )
        archive_fields = (
            payload.get("summary_id"),
            payload.get("summary_digest"),
            payload.get("summary_path"),
        )
        if incomplete and any(f is not None for f in archive_fields):
            errors.append(
                f"event {index}: incomplete summary must carry null archive fields"
            )
        if not incomplete and any(f is None for f in archive_fields):
            errors.append(
                f"event {index}: complete summary must carry archive id/digest/path"
            )
        archive_write_failed = payload.get("archive_write_failed", False)
        if archive_write_failed and incomplete:
            errors.append(
                f"event {index}: archive_write_failed may only ride a "
                "complete summary (a failed summary never attempts the write)"
            )
    return errors


def _verify_v02_activation_restore(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §3.3/§4.4 restore facts on the v0.2 track: the same activation
    is restored at most once per journal (each prompt's controller build may
    declare the sidecar handover once)."""
    errors: list[str] = []
    seen: dict[str, int] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "retrieval_activation_restored":
            continue
        activation = event["payload"]["activation_id"]
        if activation in seen:
            errors.append(
                f"event {index}: second restore of activation {activation} "
                f"(first at event {seen[activation]})"
            )
        else:
            seen[activation] = index
    return errors


def _verify_v02_control_tickets(events: list[dict[str, Any]]) -> list[str]:
    """ACAF Slice 1 (设计文档 §4.2/§4.6) ticket lifecycle on the v0.2 track:

    - a `control_ticket_consumed` / `control_ticket_rejected` must reference a
      `control_ticket_issued` for the same `ticket_id` EARLIER in the same
      journal (a ticket cannot be consumed before it was issued, and there is
      no cross-run ticket import on the v0.2 track);
    - issued/consumed/rejected for one ticket_id must agree on `ticket_kind`;
    - one ticket has at most one terminal event (consumed XOR rejected) —
      the one-shot contract of ADR-0011 §2.1 / the nonce consumption of 设计文档 §4.2 check 3.
    """
    errors: list[str] = []
    issued: dict[str, tuple[int, dict[str, Any]]] = {}
    terminals: dict[str, tuple[int, str, dict[str, Any]]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        if event_type not in (
            "control_ticket_issued",
            "control_ticket_consumed",
            "control_ticket_rejected",
        ):
            continue
        payload = event.get("payload", {})
        ticket_id = payload.get("ticket_id")
        kind = payload.get("ticket_kind")
        if not isinstance(ticket_id, str) or not isinstance(kind, str):
            continue  # envelope/payload schema violations are reported elsewhere
        if event_type == "control_ticket_issued":
            issued[ticket_id] = (index, event)
            continue
        # terminal event — must follow its issue. A single-pass scan cannot
        # see a LATER issue, so a consume-before-issue surfaces as the
        # unknown-ticket rejection (equally refusing, sharper message is a
        # two-pass upgrade).
        if ticket_id not in issued:
            errors.append(
                f"event {index}: {event_type} references unknown ticket "
                f"{ticket_id} (not issued in this journal)"
            )
            continue
        issue_index, issue_event = issued[ticket_id]
        if index <= issue_index:
            errors.append(
                f"event {index}: {event_type} for {ticket_id} precedes its "
                f"issue at event {issue_index}"
            )
        if kind != issue_event["payload"].get("ticket_kind"):
            errors.append(
                f"event {index}: {event_type} ticket_kind {kind!r} disagrees "
                f"with issued {issue_event['payload'].get('ticket_kind')!r} "
                f"for {ticket_id}"
            )
        if ticket_id in terminals:
            first_index, first_type, _ = terminals[ticket_id]
            errors.append(
                f"event {index}: {event_type} for {ticket_id} after it was "
                f"already {first_type} at event {first_index} (one-shot)"
            )
        else:
            terminals[ticket_id] = (index, event_type, event)
    return errors


_WORK_TOOLS = frozenset(
    {
        # Locally deterministic tools (former Face B).
        "read_file",
        "list_dir",
        "grep",
        "search_tool",
        "search_replace",
        "run_tests",
        "ask_user_question",
        # Former Face A — control tools with real mechanical chains.
        "blackboard_read",
        "todo_write",
        "update_goal",
        "enter_plan_mode",
        "exit_plan_mode",
        "compaction_whitelist_add",
        "retrieval_disposition",
        # Former Face C — backend-gated tools.
        "run_terminal_cmd",
        "lsp",
        "memory_get",
        "memory_search",
        "image_gen",
        "image_edit",
        "image_to_video",
        "reference_to_video",
        "use_tool",
    }
)

_JUDGMENT_WORD_TOKENS = (
    "可用",
    "不可用",
    "成功",
    "失败",
    "available",
    "unavailable",
    "success",
    "failure",
)


def _verify_v02_tool_availability_probe(events: list[dict[str, Any]]) -> list[str]:
    """FUS-TOOL-PROBE (2026-08-13, design §3/§7) v0.2 cross-checks:

    - `complete`/`incomplete` cover exactly the work tools, each exactly
      once (two-state partition — no tool may appear on both sides, no
      work tool may be absent);
    - incomplete reasons are stable neutral statements — never availability
      judgment words (可用/不可用/成功/失败/available/...).
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "tool_availability_check":
            continue
        payload = event["payload"]
        complete = set(payload.get("complete", []))
        incomplete = {
            item["tool"]: item.get("reason", "")
            for item in payload.get("incomplete", [])
        }
        overlap = complete & set(incomplete)
        if overlap:
            errors.append(
                f"event {index}: tool(s) in both complete and incomplete: "
                f"{sorted(overlap)}"
            )
        union = complete | set(incomplete)
        if union != _WORK_TOOLS:
            errors.append(
                f"event {index}: probe partition must cover exactly the work tools; "
                f"missing={sorted(_WORK_TOOLS - union)}, "
                f"extra={sorted(union - _WORK_TOOLS)}"
            )
        for tool, reason in sorted(incomplete.items()):
            lowered = reason.lower()
            if any(token in lowered for token in _JUDGMENT_WORD_TOKENS):
                errors.append(
                    f"event {index}: incomplete reason for {tool!r} uses an "
                    f"availability judgment word: {reason!r}"
                )
    return errors


def _verify_v02_request_header(events: list[dict[str, Any]]) -> list[str]:
    """ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6) v0.2
    cross-checks:

    - per agent lane (payload `agent_role`), every loop invocation starts
      with `initial` carrying no `previous_header_sha256` and no
      `change_kind`; a lane may contain MULTIPLE chains (each subagent
      activation / main run emits its own `initial`) — a later `initial`
      resets the lane's last-observed header (2026-08-15 review fix);
    - `change` must follow the lane's last event, carry
      `previous_header_sha256` equal to it, differ from it, and carry
      `change_kind` matching exactly the component digests that changed
      (system/tools/config/multiple — the mechanical「变化原因」);
    - `tools` is unique and `tool_count` matches its length.
    """
    errors: list[str] = []
    last_by_role: dict[Any, tuple[int, str, Any, Any, Any]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "request_header_change":
            continue
        payload = event["payload"]
        role = payload.get("agent_role")
        reason = payload.get("reason")
        header = payload.get("header_sha256")
        previous = payload.get("previous_header_sha256")
        change_kind = payload.get("change_kind")
        tools = payload.get("tools", [])
        if len(set(tools)) != len(tools):
            errors.append(f"event {index}: request_header_change tools must be unique")
        if payload.get("tool_count") != len(tools):
            errors.append(
                f"event {index}: request_header_change tool_count "
                f"{payload.get('tool_count')!r} != tools length {len(tools)}"
            )
        last = last_by_role.get(role)
        if reason == "initial":
            if previous is not None:
                errors.append(
                f"event {index}: initial request_header_change must not carry "
                "previous_header_sha256"
            )
            if change_kind is not None:
                errors.append(
                    f"event {index}: initial request_header_change must not carry "
                    "change_kind"
                )
        elif reason == "change":
            if last is None:
                errors.append(
                    f"event {index}: change request_header_change without a prior "
                    f"initial for role {role!r}"
                )
            else:
                _, last_header, last_system, last_tools, last_config = last
                if previous != last_header:
                    errors.append(
                        f"event {index}: change previous_header_sha256 {previous!r} != "
                        f"last header {last_header!r} for role {role!r}"
                    )
                if previous == header:
                    errors.append(
                        f"event {index}: change request_header_change must differ from "
                        "the previous header"
                    )
                if change_kind is None:
                    errors.append(
                        f"event {index}: change request_header_change must carry "
                        "change_kind"
                    )
                else:
                    expected = _header_change_kind_from_digests(
                        last_system,
                        last_tools,
                        last_config,
                        payload.get("system_sha256"),
                        payload.get("tools_sha256"),
                        payload.get("config_sha256"),
                    )
                    if change_kind != expected:
                        errors.append(
                            f"event {index}: change_kind {change_kind!r} != actual "
                            f"changed components {expected!r}"
                        )
        else:
            errors.append(
                f"event {index}: request_header_change reason {reason!r} not in "
                "initial/change"
            )
        last_by_role[role] = (
            index,
            header,
            payload.get("system_sha256"),
            payload.get("tools_sha256"),
            payload.get("config_sha256"),
        )
    return errors


def _header_change_kind_from_digests(
    prev_system: Any,
    prev_tools: Any,
    prev_config: Any,
    system: Any,
    tools: Any,
    config: Any,
) -> str:
    """Mechanical「变化原因」: the set of header components whose digest
    changed between two request_header_change events. One component → its
    name; two or three → `multiple`. Mirrors the Rust producer's
    `header_change_kind`."""
    changed: list[str] = []
    if prev_system != system:
        changed.append("system")
    if prev_tools != tools:
        changed.append("tools")
    if prev_config != config:
        changed.append("config")
    if len(changed) == 1:
        return changed[0]
    return "multiple"


def _verify_v02_inject_budget(events: list[dict[str, Any]]) -> list[str]:
    """ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6) cross-checks on
    the per-round tool-result injection budget fields:

    - `round_inject_budget_exceeded` must carry both
      `inject_tokens_used` (>= 0) and `inject_tokens_budget` (>= 1);
    - the fields travel together and are only legal with that error code.

    Review fix (2026-08-15): the schema fields were producer-only before
    this rule — a producer regression dropping them would have passed the
    reference verifier (contrast `_verify_v02_candidate_count`)."""
    errors: list[str] = []
    for index, event in enumerate(events):
        if event.get("event_type") != "tool_completed":
            continue
        payload = event.get("payload", {})
        error = payload.get("error") or ""
        used = payload.get("inject_tokens_used")
        budget = payload.get("inject_tokens_budget")
        has_used = "inject_tokens_used" in payload
        has_budget = "inject_tokens_budget" in payload
        if error == "round_inject_budget_exceeded":
            if not has_used or not has_budget:
                errors.append(
                    f"event {index}: round_inject_budget_exceeded must carry "
                    "inject_tokens_used and inject_tokens_budget"
                )
                continue
            if (
                not isinstance(used, int)
                or isinstance(used, bool)
                or used < 0
                or not isinstance(budget, int)
                or isinstance(budget, bool)
                or budget < 1
            ):
                errors.append(
                    f"event {index}: inject_tokens_used {used!r} / "
                    f"inject_tokens_budget {budget!r} out of range — need "
                    "used >= 0, budget >= 1"
                )
            continue
        if has_used != has_budget:
            errors.append(
                f"event {index}: tool_completed carries inject_tokens_used but "
                "not inject_tokens_budget (or vice versa) — the fields travel "
                "together"
            )
        elif has_used:
            errors.append(
                f"event {index}: inject_tokens_used/inject_tokens_budget are "
                f"only legal with error=round_inject_budget_exceeded (got "
                f"{error!r})"
            )
    return errors


_ACAF_TICKETED_TOOLS = frozenset(
    {
        "search_replace",
        "run_tests",
        "run_terminal_cmd",
        # P0-C S3 前置审查修复 (F2): mirrors Rust `acaf::action_kind_for_tool`
        # — network tickets cover web_fetch / browser_read as well.
        "web_fetch",
        "browser_read",
    }
)
_RETRIEVAL_MODE_GATED_TOOLS = frozenset({"project_doc_index", "browser_read", "pdf_read"})
# P0-C S3 前置审查修复 (F7): permission-gated host tools include the
# host-routed retrieval family (project_doc_index / browser_read / pdf_read)
# on the main lane. Web-family tools are not permission-gated today (lane
# self-execution skips the bridge), so they stay outside this set.
_PERMISSION_GATED_TOOLS = _WORK_TOOLS | _RETRIEVAL_MODE_GATED_TOOLS


def _is_retrieval_mode_gated_tool(name: str) -> bool:
    """P0-C S3 前置 (2026-08-15, P1-2 定案): mirrors the Rust relay family —
    host-routed retrieval tools (`project_doc_index`/`browser_read`/`pdf_read`)
    plus the retrieval dispatch names (`retrieve_project_*`, web_search /
    web_fetch families)."""
    return (
        name in _RETRIEVAL_MODE_GATED_TOOLS
        or name.startswith("retrieve_project_")
        or name == "web_search"
        or name.startswith("web_search_")
        or name == "web_fetch"
        or name.startswith("web_fetch_")
    )


def _verify_v02_policy_denial(events: list[dict[str, Any]]) -> list[str]:
    """P0-C S3 前置 (2026-08-15, P1-2 定案): structured policy denial carried
    by no-ToolStarted tool_completed events (ACAF ticket gate / retrieval
    mode gates; permission denials stay event-less by the existing audit
    contract, so their `ToolResult.policy_denial` never reaches the journal):

    - policy_denial must carry a known source (permission | acaf |
      retrieval_mode | taint), a non-empty code and a string reason;
    - a completion carrying policy_denial must be a refusal completion:
      status=error and a non-zero exit_code (refusals are
      `exit_code=Some(1)`);
    - the tool must belong to the known refusal path of its source:
      acaf → ticketed tools (search_replace / run_tests / run_terminal_cmd /
      web_fetch / browser_read),
      retrieval_mode → retrieval family,
      permission → permission-gated host tools (work tools + host-routed
      retrieval family),
      taint → work tools (reserved, no runtime wiring).
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if event.get("event_type") != "tool_completed":
            continue
        p = event["payload"]
        if "policy_denial" not in p:
            continue
        denial = p["policy_denial"]
        source = denial.get("source")
        tool = p.get("tool")
        exit_code = p.get("exit_code")
        status = p.get("status")
        if status != "error":
            errors.append(
                f"event {index}: policy_denial requires status=error "
                f"(refusal completion); got {status!r}"
            )
        if (
            not isinstance(exit_code, int)
            or isinstance(exit_code, bool)
            or exit_code == 0
        ):
            errors.append(
                f"event {index}: policy_denial requires a non-zero exit_code "
                f"(refusals are exit_code=Some(1)); got {exit_code!r}"
            )
        code = denial.get("code")
        reason = denial.get("reason")
        if not isinstance(code, str) or not code.strip():
            errors.append(
                f"event {index}: policy_denial.code must be a non-empty string"
            )
        if not isinstance(reason, str):
            errors.append(
                f"event {index}: policy_denial.reason must be a string"
            )
        if source == "acaf":
            if tool not in _ACAF_TICKETED_TOOLS:
                errors.append(
                    f"event {index}: source=acaf on non-ticketed tool {tool!r} "
                    "(ticketed family: search_replace/run_tests/run_terminal_cmd)"
                )
        elif source == "retrieval_mode":
            if tool is None or not _is_retrieval_mode_gated_tool(tool):
                errors.append(
                    f"event {index}: source=retrieval_mode on non-retrieval "
                    f"tool {tool!r}"
                )
        elif source == "permission":
            if tool not in _PERMISSION_GATED_TOOLS:
                errors.append(
                    f"event {index}: source=permission on non-permission-gated "
                    f"tool {tool!r}"
                )
        elif source == "taint":
            if tool not in _WORK_TOOLS:
                errors.append(
                    f"event {index}: source=taint on non-work tool {tool!r}"
                )
        else:
            errors.append(
                f"event {index}: policy_denial.source {source!r} not in "
                "permission/acaf/retrieval_mode/taint"
            )
    return errors


_FAILURE_TARGET_KINDS = frozenset({"cmd_target", "anchor_target", "file_target", "url_target"})
# F4 §5.2 tool family → target kind mapping (mirrors the Rust producer
# `failure_target` helper in orz-loop).
_CMD_TARGET_TOOLS = frozenset({"run_terminal_cmd", "run_tests"})
_ANCHOR_TARGET_TOOLS = frozenset({"search_replace"})
_FILE_TARGET_TOOLS = frozenset({"search_replace", "read_file", "grep"})
_URL_TARGET_TOOLS = frozenset({"web_fetch", "browser_read"})

# F11 §5.4 (2026-08-31): receipt-step vocabulary mirroring the Rust
# production predicate (orz-assurance lif/channels.rs `is_denial_code`) —
# a gate-segment refusal is identified by the policy_denial marker or one
# of these structured rejection codes.
_DENIAL_CODES = frozenset(
    {
        "content_anchor_mismatch",
        "sealed_tool_denied",
        "retired_tool_denied",
        "retrieval_mode_off",
        "retrieval_mode_requires_framework_fallback",
        "retrieval_mode_requires_local_browser",
        "permission_deny",
        "permission_defer",
        "missing_test_runner",
        "retrieval_role_write_denied",
        "retrieval_role_execution_denied",
        "retrieval_role_shell_denied",
        "nested_subagent_dispatch_refused",
        "control_tool_lane_denied",
        "plan_round_tool_denied",
        "plan_write_already_submitted",
        "plan_write_lane_denied",
        "plan_write_disabled",
        "round_inject_budget_exceeded",
        "submit_disabled",
        "submit_lane_denied",
        "console_return_lane_denied",
        "console_action_write_lane_denied",
        "order_slot_busy",
    }
)
_DENIAL_CODE_PREFIXES = ("control_ticket_rejected:", "console_step_done_")
_DENIAL_CODE_SUFFIXES = (
    "_candidate_count_unbound",
    "_candidate_url_missing",
    "_candidate_cap_exceeded",
)


def _is_denial_code_v02(code: str) -> bool:
    return (
        code in _DENIAL_CODES
        or code.startswith(_DENIAL_CODE_PREFIXES)
        or code.endswith(_DENIAL_CODE_SUFFIXES)
    )


def _verify_v02_failure_target(events: list[dict[str, Any]]) -> list[str]:
    """MECHANICAL-LAYER-MATH-CALCULUS F4 §5.3 (2026-08-30): optional
    failure-target identity on tool_completed failure events.

    - failure_target may only appear on a failure completion (status=error);
    - kind ∈ {cmd_target, anchor_target, file_target, url_target};
    - id is a 64-char lowercase SHA-256 hex digest (the journal never carries
      full command text — only the digest + bounded preview);
    - kind-specific carried fields: cmd_target → cmd_preview (≤ 80 UTF-8 bytes),
      anchor_target → path + anchor_hash + size, file_target → path,
      url_target → canonical_url;
    - tool-family consistency: cmd_target on the terminal family,
      anchor_target on search_replace, file_target on the file family,
      url_target on the web family.
    The producer computes the digest; the verifier only re-checks format and
    the carried recheckable payload (it never re-hashes original text).
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if event.get("event_type") != "tool_completed":
            continue
        p = event["payload"]
        ft = p.get("failure_target")
        if ft is None:
            continue
        if p.get("status") != "error":
            errors.append(
                f"event {index}: failure_target requires status=error "
                f"(failure completion); got {p.get('status')!r}"
            )
        if not isinstance(ft, dict):
            errors.append(
                f"event {index}: failure_target must be an object; got {type(ft).__name__}"
            )
            continue
        kind = ft.get("kind")
        if kind not in _FAILURE_TARGET_KINDS:
            errors.append(
                f"event {index}: failure_target.kind {kind!r} not in "
                "cmd_target/anchor_target/file_target/url_target"
            )
            continue
        ft_id = ft.get("id")
        if not isinstance(ft_id, str) or len(ft_id) != 64 or any(
            c not in "0123456789abcdef" for c in ft_id
        ):
            errors.append(
                f"event {index}: failure_target.id must be 64-char lowercase "
                f"sha256 hex; got {ft_id!r}"
            )
        tool = p.get("tool")
        if kind == "cmd_target":
            if tool not in _CMD_TARGET_TOOLS:
                errors.append(
                    f"event {index}: kind=cmd_target on non-terminal tool {tool!r} "
                    "(terminal family: run_terminal_cmd/run_tests)"
                )
            preview = ft.get("cmd_preview")
            if not isinstance(preview, str) or len(preview.encode("utf-8")) > 80:
                errors.append(
                    f"event {index}: kind=cmd_target requires cmd_preview "
                    "(string ≤ 80 UTF-8 bytes)"
                )
        elif kind == "anchor_target":
            if tool not in _ANCHOR_TARGET_TOOLS:
                errors.append(
                    f"event {index}: kind=anchor_target on non-anchor tool {tool!r} "
                    "(search_replace only)"
                )
            path = ft.get("path")
            anchor_hash = ft.get("anchor_hash")
            size = ft.get("size")
            if not isinstance(path, str) or not path:
                errors.append(
                    f"event {index}: kind=anchor_target requires non-empty path"
                )
            if (
                not isinstance(anchor_hash, str)
                or len(anchor_hash) != 64
                or any(c not in "0123456789abcdef" for c in anchor_hash)
            ):
                errors.append(
                    f"event {index}: kind=anchor_target requires anchor_hash "
                    "(64-char sha256 hex)"
                )
            if (
                not isinstance(size, int)
                or isinstance(size, bool)
                or size < 0
            ):
                errors.append(
                    f"event {index}: kind=anchor_target requires size ≥ 0"
                )
        elif kind == "file_target":
            if tool not in _FILE_TARGET_TOOLS:
                errors.append(
                    f"event {index}: kind=file_target on non-file tool {tool!r} "
                    "(search_replace/read_file/grep)"
                )
            path = ft.get("path")
            if not isinstance(path, str) or not path:
                errors.append(
                    f"event {index}: kind=file_target requires non-empty path"
                )
        elif kind == "url_target":
            if tool not in _URL_TARGET_TOOLS:
                errors.append(
                    f"event {index}: kind=url_target on non-web tool {tool!r} "
                    "(web_fetch/browser_read)"
                )
            url = ft.get("canonical_url")
            if not isinstance(url, str) or not url:
                errors.append(
                    f"event {index}: kind=url_target requires non-empty canonical_url"
                )
    return errors


def _verify_v02_receipt_event_isomorphism(
    events: list[dict[str, Any]],
) -> list[str]:
    """MECHANICAL-LAYER-MATH-CALCULUS §5.4 (F11, 2026-08-31): receipt ↔
    event-chain segment isomorphism on the v0.2 track.

    A tool-execution receipt has four segments (arg_validation / gate /
    execution / delivery) and each must correspond to event-chain evidence:
    - gate segment: a refusal completion (policy_denial marker or a
      structured denial code, §5.4/R2 vocabulary) is the receipt's gate
      segment. If a tool_started exists for the same (run, tool, call_id)
      it must precede the completion (host-level refusal after start);
      otherwise the refusal is controller-side (no ToolStarted — allowed);
    - execution segment: a host-level error / timeout completion (no gate
      evidence) maps 1:1 to a preceding tool_started of the same
      (run, tool, call_id) — a completed execution without its start is a
      broken receipt;
    - arg_validation / delivery segments: pure-subterm and value-semantics
      completions must ride a start that precedes them when one exists;
    - run-level pairing: in a run that reached a terminal event other than
      run_invalidated (wall-clock kill), every tool_started must have a
      matching tool_completed. Open starts at a run_invalidated terminal are
      the documented in-flight exemption (S4 墙钟超时豁免: the call is
      abandoned mid-flight, no completion is journaled).
    """
    errors: list[str] = []
    started: dict[tuple[str, str, str], int] = {}
    completed: dict[tuple[str, str, str], list[tuple[int, dict[str, Any]]]] = {}
    terminal_type: dict[str, str] = {}
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        payload = event.get("payload", {})
        run = str(event.get("run_id", ""))
        if event_type in _TERMINAL_TYPES:
            terminal_type[run] = str(event_type)
        elif event_type == "tool_started":
            key = (
                run,
                str(payload.get("tool", "")),
                str(payload.get("call_id", "")),
            )
            started.setdefault(key, index)
        elif event_type == "tool_completed":
            key = (
                run,
                str(payload.get("tool", "")),
                str(payload.get("call_id", "")),
            )
            completed.setdefault(key, []).append((index, event))

    def _is_gate(payload: dict[str, Any]) -> bool:
        if payload.get("policy_denial") is not None:
            return True
        code = payload.get("error")
        return isinstance(code, str) and _is_denial_code_v02(code)

    for key, endings in completed.items():
        if len(endings) > 1:
            errors.append(
                f"event {endings[0][0]}: duplicate tool_completed for "
                f"{key[1]}/{key[2]} — a receipt segment is journaled at "
                "most once per call"
            )
            continue
        index, event = endings[0]
        payload = event["payload"]
        # §5.4 的 receipt 段（arg_validation/gate/execution/delivery）都是
        # 失败段——配对检查只对失败完成生效；成功完成由运行时结构保证。
        if payload.get("status") != "error":
            continue
        start_index = started.get(key)
        if _is_gate(payload):
            if start_index is not None and start_index > index:
                errors.append(
                    f"event {index}: gate refusal completion precedes its "
                    f"tool_started (receipt order broken) for "
                    f"{key[1]}/{key[2]}"
                )
            continue
        if start_index is None:
            errors.append(
                f"event {index}: non-gate completion without a preceding "
                f"tool_started (execution segment missing) for "
                f"{key[1]}/{key[2]}"
            )
        elif start_index > index:
            errors.append(
                f"event {index}: tool_completed precedes its tool_started "
                f"(receipt order broken) for {key[1]}/{key[2]}"
            )

    for key, start_index in started.items():
        run, tool, call_id = key
        term = terminal_type.get(run)
        if term is None or term == "run_invalidated":
            # 无终止事件（中断 run）或墙钟超时杀（in-flight 豁免，S4）——
            # 不要求补终止完成事件。
            continue
        if key not in completed:
            errors.append(
                f"event {start_index}: tool_started {tool}/{call_id} has no "
                f"matching tool_completed in run {run} that reached "
                f"{term} (execution receipt incomplete)"
            )
    return errors


def _verify_v02_dep_graph_events(events: list[dict[str, Any]]) -> list[str]:
    """P2-11 第 4 项 / 依赖图主线设计 §5 (2026-09-01): dependency-graph
    facts journaled on successful read_file/search_replace completions must
    be self-consistent and consistent with the event chain (F11 closure):

    - shape: `dep_graph.kind` ∈ {read, write}; `path` is a non-empty string;
      anchor values are objects or null (field types / sha256 64-hex are
      schema-enforced; this function checks object-vs-null and cross-event
      consistency only);
    - kind=read → tool must be `read_file` with exit_code 0;
    - kind=write → tool must be `search_replace` with exit_code 0;
      `consumed_read` (when non-null) must reference an earlier read fact of
      the SAME run with the SAME normalized path and a matching anchor
      (sha256 authoritative; fallback size+mtime) — the read→write anchor
      edge must point at the read whose anchor was consumed;
    - tool→entity mutation edges are implied by every write fact (no extra
      check beyond the shape above).

    Presence is optional (backward-compatible with pre-graph journals): only
    journaled facts are checked. D3 boundary: a dep_graph fact on a
    command/retrieval completion is a shape violation (tool not in the
    read/write family)."""
    errors: list[str] = []
    reads: dict[tuple[str, str], dict[str, Any]] = {}

    def _anchor_matches(a: Any, b: Any) -> bool:
        if not isinstance(a, dict) or not isinstance(b, dict):
            return False
        a_sha = a.get("sha256")
        b_sha = b.get("sha256")
        if isinstance(a_sha, str) and isinstance(b_sha, str):
            return a_sha == b_sha
        return a.get("size") == b.get("size") and a.get("mtime") == b.get("mtime")

    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "tool_completed":
            continue
        payload = event.get("payload", {})
        fact = payload.get("dep_graph")
        if fact is None:
            continue
        run = str(event.get("run_id", ""))
        tool = str(payload.get("tool", ""))
        call_id = str(payload.get("call_id", ""))
        if not isinstance(fact, dict):
            errors.append(f"event {index}: dep_graph must be an object")
            continue
        kind = fact.get("kind")
        path = fact.get("path")
        if kind not in ("read", "write"):
            errors.append(
                f"event {index}: dep_graph.kind must be read|write (got {kind!r})"
            )
            continue
        if not isinstance(path, str) or not path:
            errors.append(f"event {index}: dep_graph.path must be a non-empty string")
        if payload.get("exit_code") != 0:
            errors.append(
                f"event {index}: dep_graph present on non-success completion "
                f"({tool}/{call_id}, exit_code={payload.get('exit_code')!r})"
            )
        if kind == "read":
            if tool != "read_file":
                errors.append(
                    f"event {index}: dep_graph.kind=read on tool {tool!r} "
                    "(must be read_file)"
                )
            anchor = fact.get("anchor")
            if anchor is not None and not isinstance(anchor, dict):
                errors.append(
                    f"event {index}: dep_graph.anchor must be an object or null"
                )
            reads[(run, call_id)] = fact
        else:  # write
            if tool != "search_replace":
                errors.append(
                    f"event {index}: dep_graph.kind=write on tool {tool!r} "
                    "(must be search_replace)"
                )
            for key in ("consumed_anchor", "new_anchor"):
                val = fact.get(key)
                if val is not None and not isinstance(val, dict):
                    errors.append(
                        f"event {index}: dep_graph.{key} must be an object or null"
                    )
            consumed = fact.get("consumed_read")
            if consumed is not None:
                if not isinstance(consumed, str) or not consumed:
                    errors.append(
                        f"event {index}: dep_graph.consumed_read must be a "
                        "non-empty string or null"
                    )
                else:
                    read = reads.get((run, consumed))
                    if read is None:
                        errors.append(
                            f"event {index}: dep_graph.consumed_read={consumed!r} "
                            "has no earlier read fact in the same run "
                            "(anchor edge dangling)"
                        )
                    else:
                        read_fact = read
                        if read_fact.get("path") != path:
                            errors.append(
                                f"event {index}: dep_graph anchor edge points at "
                                f"read {consumed} on a different path "
                                f"({read_fact.get('path')!r} != {path!r})"
                            )
                        elif not _anchor_matches(
                            fact.get("consumed_anchor"), read_fact.get("anchor")
                        ):
                            errors.append(
                                f"event {index}: dep_graph anchor edge {consumed} "
                                "anchors do not match (sha256 authoritative; "
                                "fallback size+mtime)"
                            )
    return errors


def _verify_v02_probe_accuracy(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §3.5 条7 (ORZ-CACHE-CONTEXT-COST 2026-08-15): probe flips
    must be accompanied by a real request-header change — a
    `tool_availability_check` whose complete set changed must be followed by
    a main-lane `request_header_change(reason=change)` before the next
    `model_output`. This is the mechanical「翻转与 header 留痕事后核对」: a
    flip with no header change would mean the projected tool list did not
    actually change (a probe bug / stale flip)."""
    errors: list[str] = []
    # Compatibility boundary (2026-08-15): journals captured BEFORE the
    # request-header feature carry no request_header_change events; the
    # flip↔header cross-check applies only to journals produced by the
    # current producer (a journal that HAS header events must satisfy it).
    if not any(
        _is_v02(event) and event.get("event_type") == "request_header_change"
        for event in events
    ):
        return errors
    prev_complete: frozenset[str] | None = None
    pending_flip: tuple[int, frozenset[str], frozenset[str]] | None = None
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        if event_type == "tool_availability_check":
            complete = frozenset(event["payload"].get("complete", []))
            if prev_complete is not None and complete != prev_complete:
                pending_flip = (index, prev_complete, complete)
            prev_complete = complete
            continue
        if (
            event_type == "request_header_change"
            and event["payload"].get("agent_role") == "main"
        ):
            # Note (2026-08-15 review): a main-lane `initial` deliberately
            # does NOT clear a pending flip. This is currently unreachable —
            # an `initial` only precedes the first flip of a loop and the
            # seed flip is ignored below — so no false positive exists.
            if pending_flip is not None and event["payload"].get("reason") == "change":
                pending_flip = None
            continue
        if event_type == "model_output" and pending_flip is not None:
            flip_index, before, after = pending_flip
            errors.append(
                f"event {index}: tool_availability_check flip at event {flip_index} "
                f"(complete {sorted(before)} -> {sorted(after)}) was not followed by "
                "a request_header_change(reason=change) before the next model_output"
            )
            pending_flip = None
    return errors


def _verify_v02_lifecycle(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §4.4 mechanical lifecycle facts on the v0.2 track:

    - every parent disposition references an assessment that precedes it and
      whose contract_revision equals the disposition's
      expected_contract_revision (CAS); activation ids must match;
    - an accepted disposition must act on the activation's current revision —
      stale/conflicting accepted dispositions and the "new requirement
      accepted, old close later" race are rejected (§4.4);
    - only one accepted decision per assessment; a conflicting second
      decision is rejected (§4.4);
    - an accepted continue increments the activation's contract revision — the
      first assessment after it must carry revision + 1, and revisions never
      decrease;
    - a normal_close close record references a preceding disposition with
      decision=close, outcome in {accepted, replayed_idempotent}, and
      matching revision/assessment/activation/contract/result ids;
    - after the first close record on an activation, no further
      disposition/assessment/close on it is allowed;
    - a replayed disposition_id or assessment_id must carry an identical
      payload (idempotent replay, canonical-bytes comparison); a different
      payload is a conflict.
    """
    errors: list[str] = []
    assessments: list[tuple[int, dict[str, Any]]] = []
    dispositions: list[tuple[int, dict[str, Any]]] = []
    closes: list[tuple[int, dict[str, Any]]] = []
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        if event_type == "information_sufficiency_assessment":
            assessments.append((index, event))
        elif event_type == "retrieval_parent_disposition":
            dispositions.append((index, event))
        elif event_type == "retrieval_close_record":
            closes.append((index, event))
    if not (assessments or dispositions or closes):
        return errors

    # First-occurrence binding: later replays never rebind references.
    assessment_first: dict[str, tuple[int, dict[str, Any]]] = {}
    for index, event in assessments:
        payload = event["payload"]
        a_id = payload["assessment_id"]
        if a_id not in assessment_first:
            assessment_first[a_id] = (index, event)
    disposition_first: dict[str, tuple[int, dict[str, Any]]] = {}
    for index, event in dispositions:
        payload = event["payload"]
        disp_id = payload["disposition_id"]
        if disp_id not in disposition_first:
            disposition_first[disp_id] = (index, event)
    # GAP-RETRIEVAL-TOOLS: a restore event declares its awaiting disposition's
    # assessment as known, legalizing a cross-run disposition reference (§3.3
    # — the assessment itself lives in an earlier run's journal).
    restore_assessment_declared: dict[str, tuple[int, dict[str, Any]]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "retrieval_activation_restored":
            continue
        payload = event["payload"]
        if payload.get("status") == "awaiting_disposition":
            a_id = payload["assessment_id"]
            if a_id not in restore_assessment_declared:
                restore_assessment_declared[a_id] = (index, event)

    # FIRST close per activation; a second close on the same activation is
    # itself a §4.4 violation (close is terminal for the activation).
    close_first: dict[str, int] = {}
    for index, event in closes:
        payload = event["payload"]
        activation = event["payload"]["activation_id"]
        # RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
        # close-record 可选 `effort` 档——显式白名单自检（jsonschema 已
        # 锁枚举，verifier 再显式核一遍，防 schema 未接线时静默漂移）。
        effort = payload.get("effort")
        if effort is not None and effort not in ("standard", "extended", "deep"):
            errors.append(
                f"event {index}: close record effort {effort!r} is not a "
                "known delegation tier (standard/extended/deep)"
            )
        if activation in close_first:
            errors.append(
                f"event {index}: second close record "
                f"{payload['close_record_id']} on activation "
                f"{activation} (first at event {close_first[activation]})"
            )
        else:
            close_first[activation] = index

    def _after_close(index: int, what: str, activation: str) -> bool:
        closed_at = close_first.get(activation)
        if closed_at is not None and closed_at < index:
            errors.append(
                f"event {index}: {what} on activation {activation} after its "
                f"close record (event {closed_at})"
            )
            return True
        return False

    # Single pass over the lifecycle events in journal order: disposition
    # state machine (current revision, accepted decision per assessment,
    # close freeze) then assessment checks (monotonic revision, continue+1).
    current_revision: dict[str, int] = {}
    assessment_decision: dict[str, str] = {}
    # activation -> (continue event index, required next revision): the
    # revision increment only applies to assessments AFTER the continue.
    continue_pending: dict[str, tuple[int, int]] = {}
    seen_max: dict[str, int] = {}

    for index, event in dispositions:
        payload = event["payload"]
        disp_id = payload["disposition_id"]
        first_index, first_event = disposition_first[disp_id]
        if first_index != index:
            # Idempotent replay: identical canonical payload required —
            # JSON-semantic equivalents (0 vs 0.0) must NOT be treated as
            # equal replays.
            if _canonical_bytes(first_event["payload"]) != _canonical_bytes(payload):
                errors.append(
                    f"event {index}: disposition {disp_id} replayed with a "
                    f"conflicting payload"
                )
            continue

        activation = payload["activation_id"]
        if _after_close(index, f"disposition {disp_id}", activation):
            continue

        a_ref = payload["assessment_id"]
        a_entry = assessment_first.get(a_ref)
        if a_entry is None:
            # Cross-run reference: only a preceding restore declaration of the
            # same activation makes the assessment known (§3.3).
            r_entry = restore_assessment_declared.get(a_ref)
            if r_entry is None:
                errors.append(
                    f"event {index}: disposition {disp_id} references unknown "
                    f"assessment {a_ref}"
                )
                continue
            r_index, r_event = r_entry
            r_payload = r_event["payload"]
            if r_index >= index:
                errors.append(
                    f"event {index}: disposition {disp_id} references "
                    f"assessment {a_ref} restored at event {r_index}, which does "
                    f"not precede it"
                )
            if r_payload["activation_id"] != activation:
                errors.append(
                    f"event {index}: disposition {disp_id} activation_id "
                    f"{activation} != restored activation {r_payload['activation_id']} "
                    f"declaring assessment {a_ref}"
                )
            a_revision = r_payload["contract_revision"]
        else:
            a_index, a_event = a_entry
            a_payload = a_event["payload"]
            if a_index >= index:
                errors.append(
                    f"event {index}: disposition {disp_id} references "
                    f"assessment {a_ref} that does not precede it"
                )
            if a_payload["activation_id"] != activation:
                errors.append(
                    f"event {index}: disposition {disp_id} activation_id {activation} "
                    f"!= referenced assessment {a_ref} activation_id "
                    f"{a_payload['activation_id']}"
                )
            a_revision = a_payload["contract_revision"]
        if payload["expected_contract_revision"] != a_revision:
            errors.append(
                f"event {index}: disposition {disp_id} expected_contract_revision "
                f"{payload['expected_contract_revision']} != referenced "
                f"assessment {a_ref} contract_revision {a_revision}"
            )

        outcome = payload["outcome"]
        decision = payload["decision"]
        if outcome == "accepted":
            # Stale-revision / late-close rejection (§4.4): accepted
            # dispositions must act on the current revision.
            cur = current_revision.get(activation, a_revision)
            if payload["expected_contract_revision"] != cur:
                errors.append(
                    f"event {index}: disposition {disp_id} accepted on "
                    f"activation {activation} at revision "
                    f"{payload['expected_contract_revision']}, current revision "
                    f"is {cur}"
                )
            # Conflicting decision on the same assessment (§4.4).
            prior = assessment_decision.get(a_ref)
            if prior is not None and prior != decision:
                errors.append(
                    f"event {index}: disposition {disp_id} decision {decision} "
                    f"conflicts with prior accepted decision {prior} on "
                    f"assessment {a_ref}"
                )
            assessment_decision[a_ref] = decision
            if decision == "continue":
                current_revision[activation] = cur + 1
                continue_pending[activation] = (index, cur + 1)
            # decision == close: revision frozen; the close record commits it.
        elif outcome in ("rejected_stale", "rejected_conflicting"):
            # Refusal records document the rejection; they don't advance the
            # activation state.
            pass
        else:
            errors.append(
                f"event {index}: disposition {disp_id} unknown outcome {outcome!r}"
            )

    for index, event in closes:
        payload = event["payload"]
        close_id = payload["close_record_id"]
        activation = payload["activation_id"]
        if _after_close(index, f"close record {close_id}", activation):
            continue
        if payload["terminal_reason"] != "normal_close":
            continue
        d_ref = payload["validated_disposition_id"]
        d_entry = disposition_first.get(d_ref)
        if d_entry is None or d_entry[0] >= index:
            errors.append(
                f"event {index}: close record {close_id} references "
                f"disposition {d_ref} that does not precede it"
            )
            continue
        d_payload = d_entry[1]["payload"]
        if d_payload["decision"] != "close":
            errors.append(
                f"event {index}: close record {close_id} references "
                f"disposition {d_ref} with decision != close"
            )
        if d_payload["outcome"] not in ("accepted", "replayed_idempotent"):
            errors.append(
                f"event {index}: close record {close_id} references "
                f"disposition {d_ref} with outcome {d_payload['outcome']} — "
                f"only a validated close disposition commits a close record"
            )
        if d_payload["expected_contract_revision"] != payload["contract_revision"]:
            errors.append(
                f"event {index}: close record {close_id} contract_revision "
                f"{payload['contract_revision']} != disposition {d_ref} "
                f"expected_contract_revision {d_payload['expected_contract_revision']}"
            )
        if d_payload["assessment_id"] != payload["assessment_id"]:
            errors.append(
                f"event {index}: close record {close_id} assessment_id "
                f"{payload['assessment_id']} != disposition {d_ref} "
                f"assessment_id {d_payload['assessment_id']}"
            )
        if d_payload["activation_id"] != activation:
            errors.append(
                f"event {index}: close record {close_id} activation_id "
                f"{activation} != disposition {d_ref} activation_id "
                f"{d_payload['activation_id']}"
            )
        # Cross-field binding to the referenced assessment (§4.4 identity
        # chain): contract and result digests must match when the close
        # record carries them (normal_close always does). An assessment that
        # lives in an earlier run is bound through its restore declaration.
        a_ref = payload["assessment_id"]
        a_entry = assessment_first.get(a_ref)
        if a_entry is not None:
            a_payload = a_entry[1]["payload"]
            if a_payload.get("contract_id") != payload.get("contract_id"):
                errors.append(
                    f"event {index}: close record {close_id} contract_id "
                    f"{payload.get('contract_id')} != referenced assessment "
                    f"{a_ref} contract_id {a_payload.get('contract_id')}"
                )
            if a_payload.get("result_digest") != payload.get("result_digest"):
                errors.append(
                    f"event {index}: close record {close_id} result_digest "
                    f"{payload.get('result_digest')} != referenced assessment "
                    f"{a_ref} result_digest {a_payload.get('result_digest')}"
                )
        else:
            r_entry = restore_assessment_declared.get(a_ref)
            if r_entry is not None:
                r_payload = r_entry[1]["payload"]
                if r_payload.get("contract_id") != payload.get("contract_id"):
                    errors.append(
                        f"event {index}: close record {close_id} contract_id "
                        f"{payload.get('contract_id')} != restored assessment "
                        f"{a_ref} contract_id {r_payload.get('contract_id')}"
                    )
                if (
                    r_payload.get("result_digest")
                    and r_payload.get("result_digest") != payload.get("result_digest")
                ):
                    errors.append(
                        f"event {index}: close record {close_id} result_digest "
                        f"{payload.get('result_digest')} != restored assessment "
                        f"{a_ref} result_digest {r_payload.get('result_digest')}"
                    )

    for index, event in assessments:
        payload = event["payload"]
        activation = payload["activation_id"]
        a_id = payload["assessment_id"]
        if _after_close(index, f"assessment {a_id}", activation):
            continue
        # Replayed assessment must carry an identical payload.
        first_index, first_event = assessment_first[a_id]
        if first_index != index and (
            _canonical_bytes(first_event["payload"]) != _canonical_bytes(payload)
        ):
            errors.append(
                f"event {index}: assessment {a_id} replayed with a "
                f"conflicting payload"
            )
        revision = payload["contract_revision"]
        if first_index == index:
            # continue+1: the first assessment AFTER an accepted continue must
            # carry exactly current revision (assessments before the continue
            # are untouched by the pending increment).
            expected = continue_pending.get(activation)
            if expected is not None and index > expected[0]:
                if revision != expected[1]:
                    errors.append(
                        f"event {index}: assessment {a_id} contract_revision "
                        f"{revision} != continue revision + 1 ({expected[1]}) on "
                        f"activation {activation}"
                    )
                continue_pending.pop(activation, None)
            # Monotonic revisions never decrease (temporal order).
            prev_max = seen_max.get(activation)
            if prev_max is not None and revision < prev_max:
                errors.append(
                    f"event {index}: assessment {a_id} contract_revision "
                    f"{revision} < previous max {prev_max} on activation "
                    f"{activation} (revisions never decrease)"
                )
            seen_max[activation] = revision
            current_revision.setdefault(activation, revision)
    return errors


def _verify_v02_tool_running(events: list[dict[str, Any]]) -> list[str]:
    """THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): `tool_running`
    mid-run facts on the v0.2 track:

    - every tool_running must sit between the same run's tool_started and
      tool_completed for the same tool/call_id in the same run (the mid-run
      report belongs to exactly one in-flight call; run_id is part of the
      correlation key so reused call_ids across runs never cross-link);
    - at most one tool_running per call_id (单次仅一次 — no accumulation,
      no periodic repeats);
    - the call's tool_completed must carry `running: true` when a mid-run
      was journaled (the command is still running), and that completion
      must keep `exit_code: null` — a still-running command has no exit
      status yet (P2-4 审查处理);
    - exactly one tool_completed per mid-run call — the background task's
      final state rides the completion reminder, never a second
      ToolCompleted (P2-4 审查处理, §9.7.3 边界).

    TER T0.2 (2026-09-03, TODO2 T0.2 / 设计稿 §3.1/§3.2): the event type
    also carries the idle-kill lifecycle form (`status: idle_killed` +
    `reason`) — the 5-minute no-activity watchdog kill of an
    auto-backgrounded task. Idle-kill facts:

    - must reference a call_id that already produced exactly one mid-run
      tool_running and its `running: true` tool_completed (auto-bg first);
    - must appear after that completion (the kill is a background-task
      lifecycle event, not an in-flight report) and never before it;
    - at most one idle-kill per call_id; a call_id never mixes two mid-run
      or two idle-kill events;
    - `status`/`reason` pairing is schema-enforced (status requires reason
      and vice versa); no second tool_completed is introduced by a kill.
    """
    errors: list[str] = []
    running_mid: list[tuple[int, dict[str, Any]]] = []
    running_killed: list[tuple[int, dict[str, Any]]] = []
    started: dict[tuple[str, str, str], int] = {}
    completed: dict[tuple[str, str, str], list[int]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        payload = event.get("payload", {})
        key = (
            str(event.get("run_id", "")),
            str(payload.get("tool", "")),
            str(payload.get("call_id", "")),
        )
        if event_type == "tool_running":
            if payload.get("status") == "idle_killed":
                running_killed.append((index, event))
            else:
                running_mid.append((index, event))
        elif event_type == "tool_started":
            started.setdefault(key, index)
        elif event_type == "tool_completed":
            completed.setdefault(key, []).append(index)

    seen_mid: set[tuple[str, str, str]] = set()
    mid_index: dict[tuple[str, str, str], int] = {}
    for index, event in running_mid:
        payload = event["payload"]
        key = (
            str(event.get("run_id", "")),
            str(payload["tool"]),
            str(payload["call_id"]),
        )
        if key in seen_mid:
            errors.append(
                f"event {index}: duplicate tool_running for call_id "
                f"{payload['call_id']!r} — at most one mid-run report per call"
            )
            continue
        seen_mid.add(key)
        mid_index[key] = index
        start_index = started.get(key)
        if start_index is None or start_index > index:
            errors.append(
                f"event {index}: tool_running for call_id {payload['call_id']!r} "
                "has no preceding tool_started of the same tool/call_id in its run"
            )
            continue
        all_end = completed.get(key, [])
        if len(all_end) > 1:
            errors.append(
                f"event {index}: call_id {payload['call_id']!r} has "
                f"{len(all_end)} tool_completed events — exactly one completion "
                "per mid-run call (the background terminal state rides the "
                "completion reminder, not a second ToolCompleted)"
            )
        end_indices = [i for i in all_end if i > index]
        if not end_indices:
            errors.append(
                f"event {index}: tool_running for call_id {payload['call_id']!r} "
                "has no following tool_completed of the same tool/call_id in its run"
            )
            continue
        end_index = end_indices[0]
        completed_event = events[end_index]
        cpayload = completed_event.get("payload", {})
        if cpayload.get("running") is not True:
            errors.append(
                f"event {end_index}: tool_completed for call_id "
                f"{payload['call_id']!r} must carry running:true after a "
                "tool_running mid-run report"
            )
        elif cpayload.get("exit_code") is not None:
            errors.append(
                f"event {end_index}: tool_completed for call_id "
                f"{payload['call_id']!r} carries running:true but exit_code "
                f"{cpayload.get('exit_code')!r} — a still-running command must "
                "stay exit_code=null"
            )

    seen_killed: set[tuple[str, str, str]] = set()
    for index, event in running_killed:
        payload = event["payload"]
        key = (
            str(event.get("run_id", "")),
            str(payload["tool"]),
            str(payload["call_id"]),
        )
        if key in seen_killed:
            errors.append(
                f"event {index}: duplicate idle-kill tool_running for call_id "
                f"{payload['call_id']!r} — at most one idle-kill per call"
            )
            continue
        seen_killed.add(key)
        reason = payload.get("reason")
        if not isinstance(reason, str) or not reason:
            errors.append(
                f"event {index}: idle-killed tool_running for call_id "
                f"{payload['call_id']!r} must carry a non-empty reason"
            )
        mid_run_index = mid_index.get(key)
        if mid_run_index is None:
            errors.append(
                f"event {index}: idle-killed tool_running for call_id "
                f"{payload['call_id']!r} requires a preceding auto-background "
                "mid-run tool_running of the same tool/call_id in its run"
            )
            continue
        all_end = completed.get(key, [])
        end_indices = [i for i in all_end if i > mid_run_index]
        if not end_indices:
            errors.append(
                f"event {index}: idle-killed tool_running for call_id "
                f"{payload['call_id']!r} has no completed auto-bg call to kill"
            )
            continue
        completion_index = end_indices[0]
        if index <= completion_index:
            errors.append(
                f"event {index}: idle-killed tool_running for call_id "
                f"{payload['call_id']!r} must post-date the call's "
                f"running:true tool_completed at event {completion_index}"
            )
    return errors


def _verify_v02_output_truncation(events: list[dict[str, Any]]) -> list[str]:
    """TER W-F13b (2026-09-03, TODO2 T0.2 / 设计稿 §3.6): `tool_completed`
    truncation facts on the v0.2 track:

    - `output_truncated` (const true in the schema) is the explicit
      truncation marker; when present the completion must carry
      `total_bytes` (the true monotonic byte count before truncation);
    - `output_object_id` (the persisted retrieval-object pointer) only
      makes sense together with an explicit truncation marker and its byte
      count — a pointer to a retrieval object without a truncated delivery
      would be a dangling contract.
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "tool_completed":
            continue
        payload = event.get("payload", {})
        truncated = payload.get("output_truncated") is True
        has_bytes = "total_bytes" in payload
        has_object = "output_object_id" in payload
        if truncated and not has_bytes:
            errors.append(
                f"event {index}: tool_completed carries output_truncated but "
                "no total_bytes — the true byte count must accompany the "
                "truncation marker"
            )
        if has_object and not (truncated and has_bytes):
            errors.append(
                f"event {index}: tool_completed carries output_object_id but "
                "no output_truncated+total_bytes pair — the retrieval-object "
                "pointer requires an explicit truncation fact"
            )
    return errors


def _verify_v02_budget_cue_injected(events: list[dict[str, Any]]) -> list[str]:
    """TER F6 push (2026-09-03, TODO2 T0.2 / 设计稿 §3.3): `budget_cue_injected`
    cross-run facts on the v0.2 track:

    - at most 4 cues per run (design cap 3–4; TODO2 T0.2/T1.9: ≤4 次/run);
    - the cue is neutral and only fires when the remaining wallclock has
      crossed the tier threshold: remaining_seconds must be strictly below
      threshold_seconds (600/300/120).
    """
    errors: list[str] = []
    counts: dict[str, int] = {}
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "budget_cue_injected":
            continue
        run_id = str(event.get("run_id", ""))
        counts[run_id] = counts.get(run_id, 0) + 1
        payload = event.get("payload", {})
        remaining = payload.get("remaining_seconds")
        threshold = payload.get("threshold_seconds")
        if (
            isinstance(remaining, int)
            and isinstance(threshold, int)
            and remaining >= threshold
        ):
            errors.append(
                f"event {index}: budget_cue_injected remaining_seconds "
                f"{remaining} is not below its threshold_seconds {threshold} "
                "— cues fire only after the tier boundary is crossed"
            )
    for run_id, count in counts.items():
        if count > 4:
            errors.append(
                f"run {run_id}: {count} budget_cue_injected events exceed the "
                "≤4-per-run push cap"
            )
    return errors


# ── chain verification (mirror of orz-assurance chain.rs) ────────────

def _verify_chain(events: list[dict[str, Any]]) -> list[str]:
    errors: list[str] = []
    terminal_indices = [
        index
        for index, event in enumerate(events)
        if event.get("event_type") in _TERMINAL_TYPES
    ]
    # All applicable terminal violations are reported (chain.rs reports
    # per-terminal not-last inside its loop plus both count errors).
    for index in terminal_indices:
        if index != len(events) - 1:
            errors.append(f"terminal event at index {index} is not the last event")
    if len(terminal_indices) > 1:
        errors.append("multiple terminal events found")
    elif not terminal_indices:
        errors.append("expected exactly one terminal event, found 0")

    for index, event in enumerate(events):
        if event.get("sequence") != index:
            errors.append(
                f"sequence mismatch at event {index}: "
                f"expected {index}, got {event.get('sequence')}"
            )
        run_id = events[0].get("run_id")
        if event.get("run_id") != run_id:
            errors.append(
                f"run_id mismatch at event {index}: "
                f"expected {run_id}, got {event.get('run_id')}"
            )
        manifest = events[0].get("run_manifest_sha256")
        if event.get("run_manifest_sha256") != manifest:
            errors.append(
                f"manifest digest mismatch at event {index}: "
                f"expected {manifest}, got {event.get('run_manifest_sha256')}"
            )
        if index == 0:
            previous = event.get("previous_event_sha256")
            if previous is not None:
                errors.append(
                    f"first event has non-null previous_event_sha256: {previous}"
                )
        else:
            previous = event.get("previous_event_sha256")
            if previous is None:
                errors.append(f"non-first event {index} has null previous_event_sha256")
            elif previous != events[index - 1].get("event_sha256"):
                errors.append(
                    f"previous_event_sha256 mismatch at event {index}: "
                    f"expected {events[index - 1].get('event_sha256')}, got {previous}"
                )
        expected_payload = _payload_sha256(event["payload"])
        if event.get("payload_sha256") != expected_payload:
            errors.append(
                f"payload digest mismatch at event {index}: "
                f"expected {expected_payload}, got {event.get('payload_sha256')}"
            )
        expected_event = _event_sha256(event)
        if event.get("event_sha256") != expected_event:
            errors.append(
                f"event digest mismatch at event {index}: "
                f"expected {expected_event}, got {event.get('event_sha256')}"
            )
    return errors


# ── public entry points ──────────────────────────────────────────────

def _load_events(lines: list[str]) -> tuple[list[dict[str, Any]], list[str]]:
    events: list[dict[str, Any]] = []
    errors: list[str] = []
    for line_number, line in enumerate(lines, 1):
        if not line.strip():
            errors.append(f"blank journal line {line_number}")
            continue
        try:
            # parse_constant rejects NaN/Infinity at parse time — serde_json
            # (the Rust writer) refuses them too, so a NaN-bearing line is a
            # parse error, never a downstream crash in the hasher.
            events.append(
                json.loads(
                    line,
                    parse_constant=lambda value: (_ for _ in ()).throw(
                        ValueError(f"non-finite float {value}")
                    ),
                )
            )
        except (json.JSONDecodeError, ValueError) as exc:
            errors.append(f"invalid JSON at line {line_number}: {exc}")
    return events, errors


def validate_journal_text(text: str) -> list[str]:
    """Validate a journal's full text; [] == valid. One string per problem."""
    events, errors = _load_events(text.splitlines())
    if errors:
        # Parse/blank-line errors are reported first and chain verification
        # is skipped over the partial event set. NOTE: the Rust replay
        # collects parse errors and still validates the chain of the parsed
        # events — the verdicts coincide (both invalid), but Python is
        # deliberately stricter (the reference side) and reports fewer
        # secondary chain messages on a malformed journal.
        return errors
    if not events:
        return ["journal contains no valid events"]

    for index, event in enumerate(events):
        errors.extend(_envelope_errors(event, index))
    if errors:
        # Envelope failures can leave fields missing that hashing needs.
        return errors

    payload_errors: list[str] = []
    for index, event in enumerate(events):
        try:
            payload_errors.extend(_payload_errors(event, index))
        except ValueError as exc:
            payload_errors.append(f"event {index}: {exc}")
    errors.extend(payload_errors)
    errors.extend(_verify_chain(events))
    if not payload_errors:
        # Cross-layer checks (inquiry_kind, §4.4 lifecycle, retrieval mode /
        # result / restore) need complete payloads — with payload schema
        # violations present they would crash or report misleading facts, so
        # they run only on schema-valid input.
        errors.extend(_verify_v02_inquiry_kind(events))
        errors.extend(_verify_v02_plan_write(events))
        errors.extend(_verify_v02_console_mode_transition(events))
        errors.extend(_verify_v02_console_order_written(events))
        errors.extend(_verify_v02_console_order_rejected(events))
        errors.extend(_verify_v02_ledger_fold_advance(events))
        errors.extend(_verify_v02_ledger_fold_write_failed(events))
        errors.extend(_verify_v02_lifecycle(events))
        errors.extend(_verify_v02_tool_running(events))
        errors.extend(_verify_v02_output_truncation(events))
        errors.extend(_verify_v02_budget_cue_injected(events))
        errors.extend(_verify_v02_retrieval_mode(events))
        errors.extend(_verify_v02_result_consistency(events))
        errors.extend(_verify_v02_reason_codes(events))
        errors.extend(_verify_v02_source_weighting(events))
        errors.extend(_verify_v02_search_candidate_pool(events))
        errors.extend(_verify_v02_candidate_prefilter(events))
        errors.extend(_verify_v02_candidate_count(events))
        errors.extend(_verify_v02_inject_budget(events))
        errors.extend(_verify_v02_policy_denial(events))
        errors.extend(_verify_v02_failure_target(events))
        errors.extend(_verify_v02_receipt_event_isomorphism(events))
        errors.extend(_verify_v02_dep_graph_events(events))
        errors.extend(_verify_v02_mechanical_audit(events))
        errors.extend(_verify_v02_recovery_truncation(events))
        errors.extend(_verify_v02_context_compressed(events))
        errors.extend(_verify_v02_activation_restore(events))
        errors.extend(_verify_v02_control_tickets(events))
        errors.extend(_verify_v02_tool_availability_probe(events))
        errors.extend(_verify_v02_request_header(events))
        errors.extend(_verify_v02_probe_accuracy(events))
    return errors


def validate_journal_file(journal_path: Path) -> list[str]:
    """Validate a journal file; [] == valid. One string per problem."""
    try:
        text = journal_path.read_text(encoding="utf-8")
    except OSError:
        return [f"journal file not found: {journal_path}"]
    return validate_journal_text(text)
