"""Cross-validation tests for the Phase 3 #7 conformance suite.

Validates the committed REAL Rust journals under
`runtime/fixtures/run-event-v0.1/journals/` through
`assurance.run_event_journal_validation` (envelope schema + per-event
payload schema selected by the `payload_schema` track string + full
hash-chain recompute), plus synthetic bad journals and the track-resolution
table. The real-journal tests are the parity proof: any digest mismatch on
a captured journal is a real bug on one side.
"""

from __future__ import annotations

import json
from pathlib import Path
import unittest

from assurance.run_event_journal_validation import (
    GSA_PREFLIGHT_FRAGMENT,
    PAYLOAD_SCHEMA_BY_EVENT_TYPE,
    PRODUCER_SCHEMAS,
    RUNTIME,
    _canonical_bytes,
    _event_sha256,
    _payload_sha256,
    _resolve_payload_schema,
    validate_journal_file,
    validate_journal_text,
)

ROOT = Path(__file__).resolve().parents[2]
JOURNALS = ROOT / "runtime/fixtures/run-event-v0.1/journals"

ALL_JOURNALS = (
    "plain-run.jsonl",
    "tool-snapshot-run.jsonl",
    "plan-run.jsonl",
    "cancelled-run.jsonl",
    "failed-run.jsonl",
    "restore-run.jsonl",
)

# Exact event-type sequences per captured scenario — the staleness signal:
# a re-captured journal with a changed loop structure fails here at commit
# time (counts included), so drift from the current orz code is loud even
# though CI never runs the Rust binary.
EXPECTED_SEQUENCES: dict[str, tuple[str, ...]] = {
    "plain-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "orientation_checkpoint", "model_output",
        "counterexample_gate", "model_output", "runtime_stagnation_guard",
        "run_finished",
    ),
    "tool-snapshot-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "orientation_checkpoint", "model_output",
        "permission_requested", "permission_decision", "snapshot_created",
        "tool_started", "tool_completed", "model_output", "counterexample_gate",
        "model_output", "runtime_stagnation_guard", "run_finished",
    ),
    "plan-run.jsonl": (
        "run_preflight", "counterexample_gate", "plan_proposed",
        "plan_approved", "tool_availability_check", "run_started",
        "prompt_submitted", "orientation_checkpoint", "model_output",
        "counterexample_gate", "model_output", "runtime_stagnation_guard",
        "run_finished",
    ),
    "cancelled-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "orientation_checkpoint", "run_cancelled",
    ),
    "failed-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "orientation_checkpoint", "run_failed",
    ),
    "restore-run.jsonl": (
        "run_preflight", "snapshot_restored", "run_finished",
    ),
}


def load_journal(name: str) -> list[dict]:
    return [
        json.loads(line)
        for line in (JOURNALS / name).read_text(encoding="utf-8").splitlines()
    ]


def dump_journal(events: list[dict]) -> str:
    return "".join(json.dumps(event, ensure_ascii=False) + "\n" for event in events)


def event_types(events: list[dict]) -> list[str]:
    return [event["event_type"] for event in events]


class RealJournalConformanceTests(unittest.TestCase):
    """The parity proof: every captured Rust journal validates clean."""

    def test_all_six_real_journals_validate(self) -> None:
        for path in sorted(JOURNALS.glob("*.jsonl")):
            with self.subTest(journal=path.name):
                self.assertEqual(
                    validate_journal_file(path), [], f"{path.name} failed"
                )

    def test_signature_events_per_scenario(self) -> None:
        """EXACT event-type sequences + counts per scenario — the staleness
        signal (a re-captured journal with changed loop structure fails
        loudly, even though CI never runs the Rust binary)."""
        for name, expected in EXPECTED_SEQUENCES.items():
            with self.subTest(journal=name):
                types = tuple(event_types(load_journal(name)))
                self.assertEqual(types, expected, f"{name} sequence drifted")
                self.assertEqual(len(types), len(expected), f"{name} count drifted")

    def test_restore_run_id_is_rst_prefix(self) -> None:
        events = load_journal("restore-run.jsonl")
        run_id = events[0]["run_id"]
        self.assertTrue(run_id.startswith("RST-"), run_id)

    def test_captured_event_types_are_registered(self) -> None:
        seen: set[str] = set()
        for name in ALL_JOURNALS:
            for event in load_journal(name):
                self.assertIn(
                    event["event_type"],
                    PAYLOAD_SCHEMA_BY_EVENT_TYPE,
                    f"{name}: unregistered event type {event['event_type']}",
                )
                # All captured journals are Rust production track.
                self.assertEqual(
                    event["payload_schema"], "run-event-v0.1.schema.json"
                )
                seen.add(event["event_type"])
        # The real journals jointly cover the produced-and-registered set
        # (reference-shape events are never constructed by Rust).
        self.assertGreaterEqual(len(seen), 15)


class SyntheticBadJournalTests(unittest.TestCase):
    """Fail-closed behavior on tampered/partial journals (built from the
    captured plain-run.jsonl so the bytes stay realistic)."""

    def setUp(self) -> None:
        self.events = load_journal("plain-run.jsonl")

    def assert_has_error(self, text: str, fragment: str) -> None:
        errors = validate_journal_text(text)
        self.assertTrue(
            any(fragment in error for error in errors),
            f"expected {fragment!r} in {errors}",
        )

    def test_broken_chain_link(self) -> None:
        self.events[3]["previous_event_sha256"] = "0" * 64
        self.assert_has_error(dump_journal(self.events), "previous_event_sha256 mismatch at event 3")

    def test_sequence_gap(self) -> None:
        self.events[4]["sequence"] = 99
        self.assert_has_error(dump_journal(self.events), "sequence mismatch at event 4")

    def test_double_terminal(self) -> None:
        self.events.append(dict(self.events[-1]))
        self.assert_has_error(dump_journal(self.events), "multiple terminal events found")

    def test_terminal_not_last(self) -> None:
        self.events[1], self.events[-1] = self.events[-1], self.events[1]
        self.assert_has_error(dump_journal(self.events), "terminal event at index 1 is not the last event")

    def test_missing_terminal(self) -> None:
        self.assert_has_error(
            dump_journal(self.events[:-1]), "expected exactly one terminal event, found 0"
        )

    def test_unknown_payload_schema_string(self) -> None:
        self.events[1]["payload_schema"] = "brand-new-track-v0.1"
        self.assert_has_error(
            dump_journal(self.events), "unknown payload_schema string"
        )

    def test_invalid_json_line(self) -> None:
        text = dump_journal(self.events[:2]) + "{not json}\n" + dump_journal(self.events[2:])
        self.assert_has_error(text, "invalid JSON at line 3")

    def test_blank_line(self) -> None:
        text = dump_journal(self.events[:2]) + "\n" + dump_journal(self.events[2:])
        self.assert_has_error(text, "blank journal line 3")

    def test_payload_schema_violation(self) -> None:
        for event in self.events:
            if event["event_type"] == "tool_availability_check":
                event["payload"]["available"] = "not-an-array"
        self.assert_has_error(dump_journal(self.events), "payload schema violation at event 1")

    def test_envelope_violation(self) -> None:
        del self.events[1]["timestamp"]
        self.assert_has_error(dump_journal(self.events), "envelope schema violation at event 1")

    def test_empty_journal(self) -> None:
        self.assert_has_error("", "journal contains no valid events")

    def test_nan_payload_is_parse_error_not_crash(self) -> None:
        """serde_json rejects NaN at parse; the validator must report a parse
        error, never crash the hasher (check_repository aborting)."""
        text = dump_journal(self.events)
        line = text.splitlines()[3]
        text = text.replace(line, line[:-1] + '"x": NaN}')
        self.assert_has_error(text, "invalid JSON at line 4")


# ── v0.2 synthetic lifecycle journals (Phase B, ADR-0010 §4.4) ─────────

_ZERO = "0" * 64


def _mk_v02_event(
    event_type: str, payload: dict, seq: int, previous: str
) -> dict:
    event = {
        "schema_version": "0.2.0-draft",
        "run_id": "RUN-V02-0001",
        "event_id": f"EVT-V02-{seq:03d}",
        "sequence": seq,
        "timestamp": "2026-08-09T00:00:00Z",
        "event_type": event_type,
        "run_manifest_sha256": _ZERO,
        "previous_event_sha256": previous,
        "payload_schema": "run-event-v0.2.schema.json",
        "payload": payload,
        "payload_sha256": _payload_sha256(payload),
        "redaction": "none",
        "event_sha256": _ZERO,
    }
    event["event_sha256"] = _event_sha256(event)
    return event


def _assessment(assessment_id: str, revision: int, activation: str = "ACT-1") -> dict:
    return {
        "assessment_id": assessment_id,
        "activation_id": activation,
        "contract_id": "CONTRACT-1",
        "contract_revision": revision,
        "result_digest": _ZERO,
        "ledger_digest": _ZERO,
        "source_counts": {
            "total": 2,
            "full_text_observed": 1,
            "partial_text_observed": 1,
            "metadata_only": 0,
            "unavailable": 0,
        },
        "source_categories": ["official_docs"],
        "source_visibility_gate": "passed",
        "missing_categories": [],
        "filtering_reasons": [],
        "status": "sufficient",
        "reason_codes": ["COVERAGE_OK"],
        "assessment_version": "0.2.0",
    }


def _disposition(
    disposition_id: str,
    decision: str,
    revision: int,
    assessment_id: str,
    outcome: str = "accepted",
    delta: str | None = None,
) -> dict:
    return {
        "disposition_id": disposition_id,
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-1",
        "activation_id": "ACT-1",
        "assessment_id": assessment_id,
        "expected_contract_revision": revision,
        "decision": decision,
        "requirement_delta": delta,
        "capability_gate": "passed",
        "outcome": outcome,
    }


def _close_record(
    close_id: str,
    revision: int,
    assessment_id: str,
    disposition_id: str,
    terminal_reason: str = "normal_close",
) -> dict:
    return {
        "close_record_id": close_id,
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-1",
        "activation_id": "ACT-1",
        "contract_id": "CONTRACT-1",
        "contract_revision": revision,
        "result_digest": _ZERO,
        "assessment_id": assessment_id,
        "validated_disposition_id": disposition_id,
        "terminal_reason": terminal_reason,
        "resumable": True,
        "live_state_reset": True,
        "archive_ref": "archive/ACT-1",
    }


def _v02_journal(events: list[dict]) -> str:
    """Chain a v0.2 event sequence with correct hashes and a terminal."""
    terminal = _mk_v02_event(
        "run_finished",
        {"status": "completed", "turn_count": 1, "tool_rounds": 0},
        len(events),
        events[-1]["event_sha256"],
    )
    chain = []
    previous: str | None = None
    for seq, event in enumerate(events):
        event["sequence"] = seq
        event["previous_event_sha256"] = previous
        event["event_sha256"] = _ZERO
        event["event_sha256"] = _event_sha256(event)
        chain.append(event)
        previous = event["event_sha256"]
    terminal["sequence"] = len(chain)
    terminal["previous_event_sha256"] = chain[-1]["event_sha256"]
    terminal["event_sha256"] = _event_sha256(terminal)
    chain.append(terminal)
    return dump_journal(chain)


class V02LifecycleChainTests(unittest.TestCase):
    """ADR-0010 §4.4 mechanical lifecycle facts on synthetic v0.2 journals."""

    def test_happy_path_assessment_continue_close_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "continue", 0, "ASSESS-1", delta="补充截止日期"),
                    1, _ZERO,
                ),
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-2", 1), 2, _ZERO),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-2", "close", 1, "ASSESS-2"),
                    3, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_close_record",
                    _close_record("CLOSE-1", 1, "ASSESS-2", "DISP-2"),
                    4, _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_disposition_references_unknown_assessment(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-99"),
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_close_record",
                    _close_record("CLOSE-1", 0, "ASSESS-99", "DISP-1"),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("unknown assessment ASSESS-99" in e for e in errors))

    def test_disposition_cas_revision_mismatch(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 3, "ASSESS-1"),
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_close_record",
                    _close_record("CLOSE-1", 3, "ASSESS-1", "DISP-1"),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("expected_contract_revision 3 != referenced assessment" in e for e in errors)
        )

    def test_close_references_non_close_disposition(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "continue", 0, "ASSESS-1", delta="x"),
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_close_record",
                    _close_record("CLOSE-1", 0, "ASSESS-1", "DISP-1"),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("decision != close" in e for e in errors))

    def test_close_revision_mismatch_with_disposition(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1"),
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_close_record",
                    _close_record("CLOSE-1", 5, "ASSESS-1", "DISP-1"),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("contract_revision 5 != disposition" in e for e in errors))

    def test_continue_increments_next_assessment_revision(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "continue", 0, "ASSESS-1", delta="x"),
                    1, _ZERO,
                ),
                # Wrong: after an accepted continue the next assessment on the
                # same activation must carry revision + 1 (i.e. 1).
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-2", 2), 2, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("contract_revision 2 != continue disposition" in e for e in errors)
        )

    def test_lifecycle_event_after_close_record_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1"),
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_close_record",
                    _close_record("CLOSE-1", 0, "ASSESS-1", "DISP-1"),
                    2, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-2", "close", 0, "ASSESS-1"),
                    3, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("after its close record" in e for e in errors))

    def test_disposition_replayed_with_conflicting_payload(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1"),
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "continue", 0, "ASSESS-1", delta="x"),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("replayed with a conflicting payload" in e for e in errors))

    def test_v02_inquiry_kind_cross_check(self) -> None:
        payload = {
            "checkpoint_id": "ORIENT-RUN-V02-0000",
            "inquiry_family": "neutral",
            "inquiry_kind": "diagnostic_coverage_checkpoint",  # wrong kind
            "agent_role": "main",
            "session_id": "sess-main-1",
            "trigger": "completed_turns_interval",
            "completed_turns_since_orientation": 7,
            "step_index": 0,
            "message_block": "[ORIENTATION v0.2] x",
            "injection_position": "post_tool_batch_gap",
        }
        journal = _v02_journal(
            [_mk_v02_event("orientation_checkpoint", payload, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("inquiry_kind 'diagnostic_coverage_checkpoint' != envelope" in e for e in errors)
        )

    def test_v02_checks_do_not_fire_on_v01_journals(self) -> None:
        for name in ALL_JOURNALS:
            with self.subTest(journal=name):
                self.assertEqual(validate_journal_file(JOURNALS / name), [])


class TrackResolutionTests(unittest.TestCase):
    def test_rust_track_string_resolves_via_registry(self) -> None:
        mode, path = _resolve_payload_schema(
            {"payload_schema": "run-event-v0.1.schema.json", "event_type": "run_preflight"}
        )
        self.assertEqual(mode, "payload")
        self.assertEqual(path, RUNTIME / "run-preflight-event-payload-v0.1.schema.json")

    def test_rust_track_unknown_event_type_raises(self) -> None:
        with self.assertRaises(ValueError):
            _resolve_payload_schema(
                {"payload_schema": "run-event-v0.1.schema.json", "event_type": "made_up"}
            )

    def test_dual_track_slugs_resolve_to_runtime_files(self) -> None:
        dual_track = {
            "orientation_checkpoint",
            "tool_availability_check",
            "runtime_stagnation_guard",
        }
        for event_type, (slug, path) in PAYLOAD_SCHEMA_BY_EVENT_TYPE.items():
            with self.subTest(event_type=event_type):
                if event_type in dual_track:
                    self.assertTrue(path.is_relative_to(RUNTIME), path)
                if event_type == "tool_belief_stagnation":
                    self.assertFalse(path.is_relative_to(RUNTIME), path)
                self.assertTrue(path.is_file(), f"missing schema {path}")

    def test_producer_schema_entries_resolve_and_exist(self) -> None:
        for producer, schema_path in PRODUCER_SCHEMAS.items():
            with self.subTest(producer=producer):
                mode, resolved = _resolve_payload_schema(
                    {"payload_schema": producer, "event_type": "x"}
                )
                if schema_path is None:
                    self.assertEqual(mode, "envelope_only")
                    self.assertIsNone(resolved)
                else:
                    self.assertEqual(mode, "payload")
                    self.assertTrue(resolved.is_file(), f"missing schema {resolved}")

    def test_fragment_form_resolves_to_pointer(self) -> None:
        mode, path = _resolve_payload_schema({"payload_schema": GSA_PREFLIGHT_FRAGMENT})
        self.assertEqual(mode, "pointer")

    def test_fragment_payload_validation(self) -> None:
        """The gsa fragment subschema accepts the three real producer shapes
        and rejects drift — regression lock for the review finding where the
        branches' $refs resolved against the fragment root (PointerToNowhere
        crash) and the top-level additionalProperties rejected everything."""
        import json as _json

        from jsonschema import Draft202012Validator

        schema = _json.loads(
            (ROOT / "assurance/gsa-runtime-preflight-projection-v0.1.schema.json")
            .read_text(encoding="utf-8")
        )["runtime_event_payload"]
        validator = Draft202012Validator(schema)
        # Real shapes from runtime_preflight.py (with a validation-valid
        # reproduction id — the producer's real ids satisfy the pattern).
        preflight = {
            "reproduction_id": "GSA-RUN-1",
            "proof_sha256": "0" * 64,
            "manifest_sha256": "0" * 64,
            "code_file_count": 1,
            "input_count": 2,
        }
        run_started = {
            "reproduction_id": "GSA-RUN-1",
            "receipt_sha256": "0" * 64,
            "output_count": 3,
            "model_invoked": False,
            "tool_invoked": False,
        }
        run_finished = {
            "reproduction_id": "GSA-RUN-1",
            "verification_sha256": "0" * 64,
            "valid": True,
            "formal_runner_claimed": False,
        }
        for payload in (preflight, run_started, run_finished):
            with self.subTest(payload=payload):
                self.assertEqual(list(validator.iter_errors(payload)), [])
        # Drift: an extra field matches none of the three branches.
        drifted = dict(preflight, extra_field=1)
        self.assertTrue(list(validator.iter_errors(drifted)))

    def test_unknown_string_lists_registry(self) -> None:
        with self.assertRaises(ValueError) as ctx:
            _resolve_payload_schema({"payload_schema": "nope-v9"})
        self.assertIn("registered producers", str(ctx.exception))


class CanonicalFormTests(unittest.TestCase):
    def test_sort_keys_compact_separators(self) -> None:
        self.assertEqual(
            _canonical_bytes({"b": 2, "a": 1}), b'{"a":1,"b":2}'
        )

    def test_ensure_ascii_false_raw_utf8(self) -> None:
        self.assertEqual(_canonical_bytes({"x": "北"}), b'{"x":"\xe5\x8c\x97"}')

    def test_nested_keys_sorted_recursively(self) -> None:
        self.assertEqual(
            _canonical_bytes({"outer": {"z": 1, "a": 2}}),
            b'{"outer":{"a":2,"z":1}}',
        )

    def test_rfc8785_equals_rust_form_on_captured_vocabulary(self) -> None:
        """Design-review D3: the Python producers' RFC 8785 canonical form
        (assurance.utils.canonical_bytes) and the Rust-parity form coincide
        over the current payload vocabulary (ints/bools/null/UTF-8 strings
        only). Canary: fails loudly the day a payload introduces a value
        (e.g. a float) where the two forms diverge."""
        from assurance.run_event_journal_validation import _payload_sha256
        from assurance.utils import canonical_bytes, sha256_bytes

        for name in ALL_JOURNALS:
            for event in load_journal(name):
                with self.subTest(journal=name, event=event["sequence"]):
                    rust_form = _payload_sha256(event["payload"])
                    rfc8785_form = sha256_bytes(canonical_bytes(event["payload"]))
                    self.assertEqual(rust_form, rfc8785_form)


if __name__ == "__main__":
    unittest.main()
