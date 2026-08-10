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
        errors.extend(_verify_v02_lifecycle(events))
        errors.extend(_verify_v02_retrieval_mode(events))
        errors.extend(_verify_v02_result_consistency(events))
        errors.extend(_verify_v02_activation_restore(events))
    return errors


def validate_journal_file(journal_path: Path) -> list[str]:
    """Validate a journal file; [] == valid. One string per problem."""
    try:
        text = journal_path.read_text(encoding="utf-8")
    except OSError:
        return [f"journal file not found: {journal_path}"]
    return validate_journal_text(text)
