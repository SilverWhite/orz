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

Single registry source: `PAYLOAD_SCHEMA_BY_EVENT_TYPE` (33 events, the 3
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

# 33-event registry: event_type -> (slug, payload schema file). The three
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


_envelope_validator: Draft202012Validator | None = None
_schema_cache: dict[Path, dict[str, Any]] = {}


def _load_schema(path: Path) -> dict[str, Any]:
    if path not in _schema_cache:
        _schema_cache[path] = json.loads(path.read_text(encoding="utf-8"))
    return _schema_cache[path]


def _envelope_errors(event: dict[str, Any], index: int) -> list[str]:
    global _envelope_validator
    if _envelope_validator is None:
        _envelope_validator = Draft202012Validator(
            _load_schema(RUN_EVENT_SCHEMA), format_checker=FormatChecker()
        )
    return [
        f"envelope schema violation at event {index}: {error.message}"
        for error in _envelope_validator.iter_errors(event)
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

    for index, event in enumerate(events):
        try:
            errors.extend(_payload_errors(event, index))
        except ValueError as exc:
            errors.append(f"event {index}: {exc}")
    errors.extend(_verify_chain(events))
    return errors


def validate_journal_file(journal_path: Path) -> list[str]:
    """Validate a journal file; [] == valid. One string per problem."""
    try:
        text = journal_path.read_text(encoding="utf-8")
    except OSError:
        return [f"journal file not found: {journal_path}"]
    return validate_journal_text(text)
