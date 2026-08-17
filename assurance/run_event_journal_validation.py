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

# 34-event registry: event_type -> (slug, payload schema file). The three
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
    "orientation_checkpoint": (
        "orientation-checkpoint",
        RUNTIME / "orientation-checkpoint-event-payload-v0.2.schema.json",
    ),
    "diagnostic_coverage_checkpoint": (
        "diagnostic-coverage-checkpoint",
        RUNTIME / "diagnostic-coverage-checkpoint-event-payload-v0.2.schema.json",
    ),
    # ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16): the
    # forced-template checkpoint round's answer + validation + evidence
    # cross-check. One mechanism event for both inquiry families; the
    # payload inquiry_kind names the fire event it answers.
    "checkpoint_response": (
        "checkpoint-response",
        RUNTIME / "checkpoint-response-event-payload-v0.2.schema.json",
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
    # FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the output-level citation
    # verifier event — written ONLY on a block/degradation (passing final
    # answers journal nothing), ADR-0010 §3.7.9 / RETRIEVAL_MECHANICAL_CONTROLS
    # _DESIGN §3.2.
    "citation_validation": (
        "citation-validation",
        RUNTIME / "citation-validation-event-payload-v0.2.schema.json",
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
    # orientation-stagnation track — de-suffixed track identifiers, schema
    # files added slice #17.
    "orientation-stagnation-preflight-v0.1": ASSURANCE
    / "orientation-stagnation-preflight-v0.1.schema.json",
    "orientation-stagnation-run-started-v0.1": ASSURANCE
    / "orientation-stagnation-run-started-v0.1.schema.json",
    "orientation-stagnation-terminal-v0.1": ASSURANCE
    / "orientation-stagnation-terminal-v0.1.schema.json",
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

_V02_NEUTRAL_INQUIRY_EVENTS = frozenset(
    {"orientation_checkpoint", "diagnostic_coverage_checkpoint"}
)


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


def _verify_v02_checkpoint_responses(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §4.2/§14.16 forced-template checkpoint round cross-checks:

    - a `checkpoint_response` must answer an earlier fire event of the same
      family (inquiry_kind == fire event_type) with the same checkpoint_id
      and agent_role (legacy fires without `agent_role` are tolerated —
      the DC fire schema made the field optional for 2026-08-15 compatibility);
    - attempt is 1-based; at most two attempts per checkpoint_id;
    - `refill_requested` only on attempt 1 and must be followed by a second
      response for the same checkpoint_id;
    - `accepted` may close at attempt 1 or 2; `degraded` only closes at
      attempt 2 and must carry a degrade_reason;
    - a closing response (accepted/degraded) must be the LAST response for
      that checkpoint_id;
    - outcome↔validation consistency: accepted requires `validation.valid`
      and a non-null template response; refill_requested/degraded require an
      invalid validation result;
    - `next_action=gather_evidence` requires a non-empty `missing_evidence`
      and `cross_check.gather_evidence_missing_surface_provided=true`.
    """
    errors: list[str] = []
    fire_by_id: dict[str, tuple[int, dict[str, Any]]] = {}
    for index, event in enumerate(events):
        if not _is_v02(event):
            continue
        event_type = event.get("event_type")
        if event_type not in _V02_NEUTRAL_INQUIRY_EVENTS:
            continue
        payload = event.get("payload", {})
        cid = payload.get("checkpoint_id")
        if isinstance(cid, str):
            fire_by_id[cid] = (index, event)

    responses: list[tuple[int, dict[str, Any]]] = []
    for index, event in enumerate(events):
        if _is_v02(event) and event.get("event_type") == "checkpoint_response":
            responses.append((index, event))

    by_checkpoint: dict[str, list[tuple[int, dict[str, Any]]]] = {}
    for index, event in responses:
        payload = event["payload"]
        cid = payload["checkpoint_id"]
        by_checkpoint.setdefault(cid, []).append((index, event))

    for cid, attempts in by_checkpoint.items():
        attempts.sort(key=lambda item: item[0])
        fire = fire_by_id.get(cid)
        if fire is None:
            errors.append(
                f"event {attempts[0][0]}: checkpoint_response {cid!r} has no "
                "preceding orientation_checkpoint / diagnostic_coverage_checkpoint fire"
            )
        else:
            fire_index, fire_event = fire
            fire_payload = fire_event["payload"]
            if fire_index >= attempts[0][0]:
                errors.append(
                    f"event {attempts[0][0]}: checkpoint_response {cid!r} references "
                    f"fire at event {fire_index}, which does not precede it"
                )
            for index, event in attempts:
                payload = event["payload"]
                if payload["inquiry_kind"] != fire_payload.get("inquiry_kind"):
                    errors.append(
                        f"event {index}: checkpoint_response {cid!r} inquiry_kind "
                        f"{payload['inquiry_kind']!r} != fire inquiry_kind "
                        f"{fire_payload.get('inquiry_kind')!r}"
                    )
                # DC fires predate the optional `agent_role` field
                # (2026-08-15); compare only when the fire carries it.
                fire_role = fire_payload.get("agent_role")
                if fire_role is not None and payload.get("agent_role") != fire_role:
                    errors.append(
                        f"event {index}: checkpoint_response {cid!r} agent_role "
                        f"{payload.get('agent_role')!r} != fire agent_role "
                        f"{fire_role!r}"
                    )
        if len(attempts) > 2:
            errors.append(
                f"event {attempts[0][0]}: checkpoint {cid!r} has {len(attempts)} "
                "response attempts (max 2)"
            )
        for position, (index, event) in enumerate(attempts):
            payload = event["payload"]
            attempt = payload["attempt"]
            outcome = payload["outcome"]
            if attempt != position + 1:
                errors.append(
                    f"event {index}: checkpoint {cid!r} attempt {attempt} "
                    f"out of sequence (expected {position + 1})"
                )
            if outcome == "refill_requested":
                if attempt != 1:
                    errors.append(
                        f"event {index}: checkpoint {cid!r} refill_requested on "
                        f"attempt {attempt} (only attempt 1 may request a refill)"
                    )
                if position + 1 >= len(attempts):
                    errors.append(
                        f"event {index}: checkpoint {cid!r} refill_requested "
                        "without a following attempt"
                    )
            if outcome == "degraded":
                if attempt != 2:
                    errors.append(
                        f"event {index}: checkpoint {cid!r} degraded on attempt "
                        f"{attempt} (degrade only after the refill attempt)"
                    )
                if not payload.get("degrade_reason"):
                    errors.append(
                        f"event {index}: checkpoint {cid!r} degraded without "
                        "degrade_reason"
                    )
            if outcome == "accepted" and position + 1 < len(attempts):
                errors.append(
                    f"event {index}: checkpoint {cid!r} accepted but a later "
                    "response attempt exists"
                )
            if outcome == "accepted":
                if payload.get("validation", {}).get("valid") is not True:
                    errors.append(
                        f"event {index}: checkpoint {cid!r} accepted but "
                        "validation.valid is not true"
                    )
                if payload.get("response") is None:
                    errors.append(
                        f"event {index}: checkpoint {cid!r} accepted but "
                        "response is null"
                    )
            if outcome in ("refill_requested", "degraded"):
                if payload.get("validation", {}).get("valid") is not False:
                    errors.append(
                        f"event {index}: checkpoint {cid!r} {outcome} but "
                        "validation.valid is not false"
                    )
            response = payload.get("response")
            if (
                isinstance(response, dict)
                and response.get("next_action") == "gather_evidence"
            ):
                missing = response.get("missing_evidence")
                if not isinstance(missing, list) or not missing:
                    errors.append(
                        f"event {index}: checkpoint {cid!r} "
                        "next_action=gather_evidence requires non-empty "
                        "response.missing_evidence"
                    )
                if (
                    payload.get("cross_check", {}).get(
                        "gather_evidence_missing_surface_provided"
                    )
                    is not True
                ):
                    errors.append(
                        f"event {index}: checkpoint {cid!r} "
                        "next_action=gather_evidence requires "
                        "cross_check.gather_evidence_missing_surface_provided=true"
                    )
            if payload["inquiry_family"] != "neutral":
                errors.append(
                    f"event {index}: checkpoint_response {cid!r} inquiry_family "
                    f"{payload['inquiry_family']!r} != neutral"
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
        if event.get("event_type") not in ("tool_started", "tool_completed"):
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


# §3.7.5 claim × visibility matrix: mechanical ranks, derived in the verifier
# from the committed result's own source_ledger.
_VISIBILITY_RANK = {
    "full_text_observed": 3,
    "partial_text_observed": 2,
    "metadata_only": 1,
    "unavailable": 0,
}

# GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 fixed
# tier/weight table — the mechanical judge and the model annotation share
# the same three multipliers (relative ranking, not 0-1 confidence).
_WEIGHT_BY_TIER = {
    "authoritative": 1.1,
    "default": 1.0,
    "low_quality": 0.7,
}
_ALLOWED_WEIGHTS = frozenset((0.7, 1.0, 1.1))
_CLAIM_MIN_VISIBILITY = {
    "observed": 3,
    "derived": 2,
    "synthesized": 1,
}


def _verify_v02_result_consistency(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §3.3.3/§3.7.5 mechanical facts on committed retrieval results:

    - source_counts equal the visibility distribution mechanically counted
      from the commit's own source_ledger;
    - every section/claim claim_strength is bounded by the visibility of all
      its bound sources (§3.7.5: observed needs full text, derived needs >=
      partial, synthesized needs >= metadata; "none" carries no obligation);
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

    def _strength_ok(strength: str, entry: dict[str, Any]) -> bool:
        min_rank = _CLAIM_MIN_VISIBILITY.get(strength)
        if min_rank is None:
            return True
        return _VISIBILITY_RANK.get(entry["visibility"], -1) >= min_rank

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

        ledger_by_id = {entry["source_id"]: entry for entry in ledger}
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
        for section in p["organized_response"]["sections"]:
            strength = section["claim_strength"]
            for sid in section["source_ids"]:
                entry = ledger_by_id.get(sid)
                if entry is None:
                    errors.append(
                        f"event {index}: section {section['section_title']!r} "
                        f"binds unknown source {sid}"
                    )
                elif not _strength_ok(strength, entry):
                    errors.append(
                        f"event {index}: section {section['section_title']!r} "
                        f"claim_strength {strength} exceeds source {sid} "
                        f"visibility {entry['visibility']}"
                    )
        for claim in p["organized_response"]["claims"]:
            strength = claim["claim_strength"]
            for sid in claim["source_ids"]:
                entry = ledger_by_id.get(sid)
                if entry is None:
                    errors.append(
                        f"event {index}: claim {claim['claim_id']} binds unknown "
                        f"source {sid}"
                    )
                elif not _strength_ok(strength, entry):
                    errors.append(
                        f"event {index}: claim {claim['claim_id']} "
                        f"claim_strength {strength} exceeds source {sid} "
                        f"visibility {entry['visibility']}"
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
            if a_payload.get("source_counts") != p["source_counts"]:
                errors.append(
                    f"event {a_index}: assessment {a_id} source_counts "
                    f"{a_payload.get('source_counts')} != committed result "
                    f"{p['result_id']} (event {index})"
                )
    return errors


def _verify_v02_source_weighting(events: list[dict[str, Any]]) -> list[str]:
    """ADR-0010 §3.7 条 12 / FUS-SOURCE-WEIGHTING (GAP-SOURCE-WEIGHTING-IMPL):
    mechanical tier/weight facts on committed retrieval results:

    - web_page ledger entries MUST carry tier + mechanical_weight +
      weight_reason; the tier/weight pair is fixed (authoritative 1.1 /
      default 1.0 / low_quality 0.7);
    - model annotation fields are all-or-none and status/weight consistent
      (annotated -> 0.7; adopted -> >= 1.0; a mechanically low-quality
      source cannot be adopted);
    - a low-quality source used in sections/claims MUST carry status
      "annotated" (v0 annotate-and-rank, no hard interception);
    - every organized_response.source_annotation matches the merged ledger
      fields, and every merged ledger field has a matching annotation.
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "retrieval_result_committed":
            continue
        p = event["payload"]
        ledger = p["source_ledger"]
        ledger_by_id = {entry["source_id"]: entry for entry in ledger}

        used_ids: set[str] = set()
        for section in p["organized_response"]["sections"]:
            used_ids.update(section.get("source_ids", []))
        for claim in p["organized_response"]["claims"]:
            used_ids.update(claim.get("source_ids", []))

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

            model_weight = entry.get("model_weight")
            model_reason = entry.get("model_weight_reason")
            annotation_status = entry.get("annotation_status")
            model_fields = [model_weight, model_reason, annotation_status]
            present = [field for field in model_fields if field is not None]
            if present and len(present) != 3:
                errors.append(
                    f"event {index}: source {sid} has partial model annotation fields"
                )
            elif present:
                if model_weight not in _ALLOWED_WEIGHTS:
                    errors.append(
                        f"event {index}: source {sid} model_weight {model_weight} "
                        "outside {0.7, 1.0, 1.1}"
                    )
                if annotation_status == "annotated" and model_weight != 0.7:
                    errors.append(
                        f"event {index}: source {sid} annotated requires model_weight 0.7"
                    )
                if annotation_status == "adopted" and model_weight < 1.0:
                    errors.append(
                        f"event {index}: source {sid} adopted requires model_weight >= 1.0"
                    )
                if tier == "low_quality" and annotation_status == "adopted":
                    errors.append(
                        f"event {index}: source {sid} low_quality cannot be adopted"
                    )

            if tier == "low_quality" and (
                sid in used_ids or entry.get("used_in_sections")
            ) and annotation_status != "annotated":
                errors.append(
                    f"event {index}: source {sid} low_quality used without "
                    "annotated status"
                )

        annotations = p["organized_response"].get("source_annotations", [])
        annotation_by_id: dict[str, dict[str, Any]] = {}
        seen_annotation_ids: set[str] = set()
        for annotation in annotations:
            sid = annotation["source_id"]
            if sid in seen_annotation_ids:
                errors.append(f"event {index}: duplicate source_annotation for {sid}")
                continue
            seen_annotation_ids.add(sid)
            annotation_by_id[sid] = annotation
            entry = ledger_by_id.get(sid)
            if entry is None:
                errors.append(
                    f"event {index}: source_annotation references unknown source {sid}"
                )
            elif (
                entry.get("model_weight") != annotation["weight"]
                or entry.get("model_weight_reason") != annotation["reason"]
                or entry.get("annotation_status") != annotation["status"]
            ):
                errors.append(
                    f"event {index}: source_annotation for {sid} does not match "
                    "merged ledger fields"
                )
        for entry in ledger:
            if "model_weight" in entry and entry["source_id"] not in annotation_by_id:
                errors.append(
                    f"event {index}: source {entry['source_id']} has model fields "
                    "without a source_annotation"
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


def _verify_v02_citation_validation(events: list[dict[str, Any]]) -> list[str]:
    """FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): ADR-0010 §3.7.9 output-level
    citation verifier events on the v0.2 track (RETRIEVAL_MECHANICAL_CONTROLS
    _DESIGN §3.2):

    - a block decision must degrade the answer (degraded=true), carry at least
      one reason code, the mechanical degradation message block and at least
      one failed marker;
    - a pass decision must not degrade and must carry no reason codes (the v0.2
      producer currently writes only block events — a passing final answer
      journals nothing);
    - marker_count must equal the markers array length;
    - every failed marker must carry at least one reason code and every passed
      marker none.
    """
    errors: list[str] = []
    for index, event in enumerate(events):
        if not _is_v02(event) or event.get("event_type") != "citation_validation":
            continue
        payload = event["payload"]
        decision = payload["decision"]
        degraded = payload["degraded"]
        reason_codes = payload["reason_codes"]
        marker_count = payload["marker_count"]
        markers = payload.get("markers", [])
        if decision == "block":
            if not degraded:
                errors.append(
                    f"event {index}: citation_validation block must degrade the answer"
                )
            if not reason_codes:
                errors.append(
                    f"event {index}: citation_validation block needs reason codes"
                )
            if not markers:
                errors.append(
                    f"event {index}: citation_validation block needs marker details"
                )
            if not payload.get("message_block", "").startswith(
                "[CITATION_VALIDATION_FAILED"
            ):
                errors.append(
                    f"event {index}: citation_validation block carries a "
                    "non-mechanical message block"
                )
        elif decision == "pass":
            if degraded:
                errors.append(
                    f"event {index}: citation_validation pass must not degrade"
                )
            if reason_codes:
                errors.append(
                    f"event {index}: citation_validation pass must carry no reason codes"
                )
        if marker_count != len(markers):
            errors.append(
                f"event {index}: citation_validation marker_count {marker_count} "
                f"!= {len(markers)} marker details"
            )
        for marker in markers:
            marker_reasons = marker.get("reason_codes", [])
            if marker.get("status") == "failed" and not marker_reasons:
                errors.append(
                    f"event {index}: failed citation marker {marker.get('raw')!r} "
                    "lacks reason codes"
                )
            if marker.get("status") == "passed" and marker_reasons:
                errors.append(
                    f"event {index}: passed citation marker {marker.get('raw')!r} "
                    "carries reason codes"
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
    the v0.2 `context_compressed` event is a five-section template summary:
    - mode must be `template_summary`, reason one of rhythm/fallback;
    - a complete summary must carry a non-null archive id/digest/path;
    - the termination state (summary_incomplete=true) must carry null
      archive fields (no archive was written).

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
        if payload.get("mode") != "template_summary":
            errors.append(
                f"event {index}: context_compressed mode must be template_summary"
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
        activation = event["payload"]["activation_id"]
        if activation in close_first:
            errors.append(
                f"event {index}: second close record "
                f"{event['payload']['close_record_id']} on activation "
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
        errors.extend(_verify_v02_checkpoint_responses(events))
        errors.extend(_verify_v02_plan_write(events))
        errors.extend(_verify_v02_console_mode_transition(events))
        errors.extend(_verify_v02_console_order_written(events))
        errors.extend(_verify_v02_console_order_rejected(events))
        errors.extend(_verify_v02_ledger_fold_advance(events))
        errors.extend(_verify_v02_lifecycle(events))
        errors.extend(_verify_v02_retrieval_mode(events))
        errors.extend(_verify_v02_result_consistency(events))
        errors.extend(_verify_v02_source_weighting(events))
        errors.extend(_verify_v02_search_candidate_pool(events))
        errors.extend(_verify_v02_candidate_prefilter(events))
        errors.extend(_verify_v02_candidate_count(events))
        errors.extend(_verify_v02_inject_budget(events))
        errors.extend(_verify_v02_policy_denial(events))
        errors.extend(_verify_v02_citation_validation(events))
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
