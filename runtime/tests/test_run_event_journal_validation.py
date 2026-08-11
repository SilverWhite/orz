"""Cross-validation tests for the Phase 3 #7 conformance suite.

Validates the committed REAL Rust journals under
`runtime/fixtures/run-event-v0.1/journals/` (historical freeze) and
`runtime/fixtures/run-event-v0.2/journals/` (production track since
GAP-INQUIRY-SPLIT) through `assurance.run_event_journal_validation`
(envelope schema + per-event payload schema selected by the `payload_schema`
track string + full hash-chain recompute), plus synthetic bad journals and
the track-resolution table. The real-journal tests are the parity proof: any
digest mismatch on a captured journal is a real bug on one side.
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
# GAP-INQUIRY-SPLIT (2026-08-09): the production track since the v0.2 flip —
# real journals captured by the v0.2 producer.
JOURNALS_V02 = ROOT / "runtime/fixtures/run-event-v0.2/journals"

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
# NOTE (GAP-INQUIRY-SPLIT): the v0.1 journals below are a HISTORICAL FREEZE —
# they still contain the per-turn `orientation_checkpoint` and the retired
# `neutral_inquiry`/`retrieval_completion_check` producers; the v0.2
# production journals live in `EXPECTED_SEQUENCES_V02`.
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

# GAP-INQUIRY-SPLIT (2026-08-09): the v0.2 production track — the per-turn
# orientation event is GONE (fires only on the session-level 7-round trigger;
# see `orientation-fire-run`), the retired event types are absent, and the
# retrieval path produces mechanical `information_sufficiency_assessment`.
ALL_JOURNALS_V02 = (
    "plain-run.jsonl",
    "tool-snapshot-run.jsonl",
    "plan-run.jsonl",
    "cancelled-run.jsonl",
    "failed-run.jsonl",
    "restore-run.jsonl",
    "orientation-fire-run.jsonl",
    # GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval mode / structured result /
    # activation restore / pre-handoff scenarios.
    "mode-off-refusal.jsonl",
    "local-browser-capability.jsonl",
    "real-doc-retrieval.jsonl",
    "cross-prompt-restore.jsonl",
    "pre-handoff-checkpoint.jsonl",
)

EXPECTED_SEQUENCES_V02: dict[str, tuple[str, ...]] = {
    "plain-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "model_output", "counterexample_gate",
        "model_output", "runtime_stagnation_guard", "run_finished",
    ),
    "tool-snapshot-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "model_output", "permission_requested",
        "permission_decision", "snapshot_created", "tool_started",
        "tool_completed", "model_output", "counterexample_gate",
        "model_output", "runtime_stagnation_guard", "run_finished",
    ),
    "plan-run.jsonl": (
        "run_preflight", "counterexample_gate", "plan_proposed",
        "plan_approved", "tool_availability_check", "run_started",
        "prompt_submitted", "model_output", "counterexample_gate",
        "model_output", "runtime_stagnation_guard", "run_finished",
    ),
    "cancelled-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "run_cancelled",
    ),
    "failed-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "run_failed",
    ),
    "restore-run.jsonl": (
        "run_preflight", "snapshot_restored", "run_finished",
    ),
    # GAP-SUBAGENT-RUNTIME (2026-08-10, M3/M4): each retrieval dispatch runs
    # the SHARED loop — the subagent's model round and its own stagnation
    # guard land in the same chain between the parent's tool_started/
    # tool_completed wrapper; each assessment is followed by the parent's
    # retrieval_disposition round (the control call's disposition event —
    # and the close record on the accepted close — land between the tool's
    # start and completion); the 7-round orientation crossing fires once, on
    # the 4th retrieve's post-tool-batch gap.
    # GAP-RETRIEVAL-TOOLS (2026-08-10): each iteration commits its structured
    # result (retrieval_result_committed) between the dispatch's
    # tool_completed and the mechanical assessment.
    "orientation-fire-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment", "orientation_checkpoint",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "retrieval_close_record", "tool_completed",
        "model_output", "counterexample_gate", "model_output",
        "runtime_stagnation_guard", "run_finished",
    ),
    # GAP-RETRIEVAL-TOOLS (2026-08-10): mode=off refuses the scripted
    # retrieval dispatch — the refusal is the terminal ToolCompleted(error)
    # ALONE (no ToolStarted: the verifier's mode rule forbids any dispatch
    # after a transition to off).
    "mode-off-refusal.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "model_output", "tool_completed",
        "model_output", "counterexample_gate", "model_output",
        "runtime_stagnation_guard", "run_finished",
    ),
    # local_browser with an unsupported capability: the bootstrap transition
    # journals before the availability gate; the refusal follows the
    # standard ToolStarted → ToolCompleted(error) audit shape.
    "local-browser-capability.jsonl": (
        "run_preflight", "retrieval_mode_transition",
        "tool_availability_check", "run_started", "prompt_submitted",
        "model_output", "tool_started", "tool_completed", "model_output",
        "counterexample_gate", "model_output", "runtime_stagnation_guard",
        "run_finished",
    ),
    # Real project-doc retrieval: the subagent's index call runs through
    # the host (permission bridge records its auto-allow), the committed
    # result carries the real visibility, the assessment consumes it.
    "real-doc-retrieval.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "model_output", "tool_started",
        "model_output", "permission_requested", "permission_decision",
        "tool_started", "tool_completed", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed", "information_sufficiency_assessment",
        "model_output", "counterexample_gate", "model_output",
        "runtime_stagnation_guard", "run_finished",
    ),
    # local_browser (2026-08-10): mode=local_browser with an AVAILABLE
    # capability — the external subagent runs the host browser_read tool
    # (permission auto-allow), the committed result carries REAL full-text
    # web_page evidence, and the transition records capability_status
    # = available.
    "local-browser-read.jsonl": (
        "run_preflight", "retrieval_mode_transition",
        "tool_availability_check", "run_started", "prompt_submitted",
        "model_output", "tool_started", "model_output",
        "permission_requested", "permission_decision", "tool_started",
        "tool_completed", "model_output", "runtime_stagnation_guard",
        "tool_completed", "retrieval_result_committed",
        "information_sufficiency_assessment", "model_output",
        "counterexample_gate", "model_output", "runtime_stagnation_guard",
        "run_finished",
    ),
    # Cross-run activation restore: the seeded AwaitingDisposition
    # activation journals its restore at startup, the parent's disposition
    # closes it (close record binds through the restore declaration).
    "cross-prompt-restore.jsonl": (
        "run_preflight", "retrieval_activation_restored",
        "tool_availability_check", "run_started", "prompt_submitted",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "retrieval_close_record", "tool_completed", "model_output",
        "counterexample_gate", "model_output", "runtime_stagnation_guard",
        "run_finished",
    ),
    # Pre-handoff checkpoint: the stagnation restart decision journals the
    # orientation checkpoint (independent lifecycle trigger) before the
    # run_invalidated terminal.
    "pre-handoff-checkpoint.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "model_output", "counterexample_gate",
        "model_output", "runtime_stagnation_guard",
        "orientation_checkpoint", "run_invalidated",
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


def load_journal_v02(name: str) -> list[dict]:
    return [
        json.loads(line)
        for line in (JOURNALS_V02 / name).read_text(encoding="utf-8").splitlines()
    ]


class V02JournalConformanceTests(unittest.TestCase):
    """GAP-INQUIRY-SPLIT (2026-08-09): the production-track real journals —
    captured by the v0.2 Rust producer after the flip. Same parity proof as
    the v0.1 class, plus v0.2-track assertions (homogeneous envelope, retired
    event types absent, the orientation 7-round fire and the mechanical
    information-sufficiency assessment)."""

    def test_all_seven_v02_journals_validate(self) -> None:
        for name in ALL_JOURNALS_V02:
            with self.subTest(journal=name):
                self.assertEqual(
                    validate_journal_file(JOURNALS_V02 / name),
                    [],
                    f"{name} failed",
                )

    def test_v02_journals_are_homogeneous_v02_track(self) -> None:
        """Every event of every v0.2 journal: v0.2 envelope schema_version +
        v0.2 payload_schema (a mixed track would break the homogeneous-chain
        requirement, ADR-0010 §11.6.2)."""
        for name in ALL_JOURNALS_V02:
            with self.subTest(journal=name):
                for event in load_journal_v02(name):
                    self.assertEqual(
                        event["schema_version"], "0.2.0-draft", name
                    )
                    self.assertEqual(
                        event["payload_schema"], "run-event-v0.2.schema.json", name
                    )

    def test_v02_sequence_staleness_signal(self) -> None:
        for name, expected in EXPECTED_SEQUENCES_V02.items():
            with self.subTest(journal=name):
                types = tuple(event_types(load_journal_v02(name)))
                self.assertEqual(types, expected, f"{name} sequence drifted")
                self.assertEqual(len(types), len(expected), f"{name} count drifted")

    def test_retired_event_types_absent_from_v02(self) -> None:
        """`neutral_inquiry` / `retrieval_completion_check` are retired on the
        v0.2 track (ADR-0010 §5.1) — no v0.2 producer may write them."""
        retired = {"neutral_inquiry", "retrieval_completion_check"}
        for name in ALL_JOURNALS_V02:
            with self.subTest(journal=name):
                for event in load_journal_v02(name):
                    self.assertNotIn(event["event_type"], retired, name)

    def test_orientation_fire_payload_is_v02_shape(self) -> None:
        events = load_journal_v02("orientation-fire-run.jsonl")
        orientation = next(
            e for e in events if e["event_type"] == "orientation_checkpoint"
        )
        p = orientation["payload"]
        self.assertEqual(p["inquiry_family"], "neutral")
        self.assertEqual(p["inquiry_kind"], "orientation_checkpoint")
        self.assertEqual(p["agent_role"], "main")
        self.assertEqual(p["trigger"], "completed_turns_interval")
        self.assertEqual(p["completed_turns_since_orientation"], 7)
        self.assertEqual(p["injection_position"], "post_tool_batch_gap")
        self.assertTrue(p["message_block"].startswith("[ORIENTATION"))

    def test_orientation_fires_exactly_once_in_seven_rounds(self) -> None:
        events = load_journal_v02("orientation-fire-run.jsonl")
        fires = [e for e in events if e["event_type"] == "orientation_checkpoint"]
        self.assertEqual(len(fires), 1)

    def test_information_sufficiency_assessment_is_mechanical(self) -> None:
        events = load_journal_v02("orientation-fire-run.jsonl")
        assessments = [
            e for e in events if e["event_type"] == "information_sufficiency_assessment"
        ]
        # GAP-SUBAGENT-RUNTIME (2026-08-10): the M4 scenario interleaves a
        # disposition after every assessment (4 × continue + 1 × close) —
        # each continue bumps the contract revision, so the assessments
        # carry revisions 0..4.
        self.assertEqual(len(assessments), 5)
        for i, a in enumerate(assessments):
            p = a["payload"]
            self.assertEqual(p["status"], "indeterminate")
            self.assertEqual(p["source_visibility_gate"], "not_applicable")
            self.assertEqual(p["contract_revision"], i)
            # GAP-RETRIEVAL-TOOLS (2026-08-10): the structured result is
            # PER-ITERATION — each round's ledger covers that round's one
            # [DOC] declaration (metadata-grade).
            self.assertEqual(p["source_counts"]["total"], 1)
            self.assertEqual(p["source_counts"]["metadata_only"], 1)
            # No inquiry family / model verdict on a mechanical event.
            self.assertNotIn("inquiry_family", p)

    def test_v02_lifecycle_chain_rules_still_hold(self) -> None:
        """The 7 production journals must not trip the §4.4 lifecycle
        verifier (orphan assessments are legal; close-before-disposition is
        not produced this slice — the verifier accepts what exists)."""
        from assurance.run_event_journal_validation import _verify_v02_lifecycle

        for name in ALL_JOURNALS_V02:
            with self.subTest(journal=name):
                events = load_journal_v02(name)
                errors = _verify_v02_lifecycle(events)
                self.assertEqual(errors, [], f"{name}: {errors}")


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
    activation: str = "ACT-1",
) -> dict:
    return {
        "disposition_id": disposition_id,
        "parent_session_id": "sess-main-1",
        "subagent_session_id": "sess-ext-1",
        "activation_id": activation,
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
            any("contract_revision 2 != continue revision + 1" in e for e in errors)
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
        """§5.1 cross-check on the verifier function itself: the payload
        schema's inquiry_kind const already rejects a mismatched kind at the
        schema layer (end-to-end the P1-1 fix skips cross-checks when payload
        schema errors exist), so the cross-check unit is exercised directly.
        """
        from assurance.run_event_journal_validation import _verify_v02_inquiry_kind

        event = {
            "schema_version": "0.2.0-draft",
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "orientation_checkpoint",
            "payload": {"inquiry_kind": "diagnostic_coverage_checkpoint"},
        }
        errors = _verify_v02_inquiry_kind([event])
        self.assertTrue(
            any("inquiry_kind 'diagnostic_coverage_checkpoint' != envelope" in e for e in errors)
        )
        ok_event = {
            "schema_version": "0.2.0-draft",
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "orientation_checkpoint",
            "payload": {"inquiry_kind": "orientation_checkpoint"},
        }
        self.assertEqual(_verify_v02_inquiry_kind([ok_event]), [])

    def test_v02_checks_do_not_fire_on_v01_journals(self) -> None:
        for name in ALL_JOURNALS:
            with self.subTest(journal=name):
                self.assertEqual(validate_journal_file(JOURNALS / name), [])

    def test_schema_invalid_payload_does_not_crash_lifecycle(self) -> None:
        """P1-1: a v0.2 lifecycle event with a schema-invalid payload must be
        reported as errors, never crash the validator with a KeyError."""
        bad_disposition = _disposition("DISP-1", "close", 0, "ASSESS-1")
        del bad_disposition["assessment_id"]  # schema-required field missing
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("retrieval_parent_disposition", bad_disposition, 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(errors)
        self.assertTrue(any("payload schema violation" in e for e in errors))

    def test_second_close_record_on_activation_rejected(self) -> None:
        """P1-2: a close record is terminal for the activation — a second
        close on the same activation is a §4.4 violation."""
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("retrieval_parent_disposition", _disposition("DISP-1", "close", 0, "ASSESS-1"), 1, _ZERO),
                _mk_v02_event("retrieval_close_record", _close_record("CLOSE-1", 0, "ASSESS-1", "DISP-1"), 2, _ZERO),
                _mk_v02_event("retrieval_close_record", _close_record("CLOSE-2", 0, "ASSESS-1", "DISP-1"), 3, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("second close record" in e for e in errors))

    def test_close_between_close_records_rejected(self) -> None:
        """P1-2: after the FIRST close, even a disposition sandwiched between
        two closes is rejected."""
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("retrieval_parent_disposition", _disposition("DISP-1", "close", 0, "ASSESS-1"), 1, _ZERO),
                _mk_v02_event("retrieval_close_record", _close_record("CLOSE-1", 0, "ASSESS-1", "DISP-1"), 2, _ZERO),
                _mk_v02_event("retrieval_parent_disposition", _disposition("DISP-2", "close", 0, "ASSESS-1"), 3, _ZERO),
                _mk_v02_event("retrieval_close_record", _close_record("CLOSE-2", 0, "ASSESS-1", "DISP-2"), 4, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("after its close record" in e for e in errors))
        self.assertTrue(any("second close record" in e for e in errors))

    def test_revision_decrease_rejected(self) -> None:
        """P2-1: revisions never decrease even without a continue."""
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 5), 0, None),
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-2", 3), 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("revisions never decrease" in e for e in errors))

    def test_assessment_conflicting_replay_rejected(self) -> None:
        """P2-2/C1-1e: a replayed assessment_id with a different payload is a
        conflict; the CAS binding stays on the first occurrence."""
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 2), 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("assessment ASSESS-1 replayed with a conflicting payload" in e for e in errors))

    def test_assessment_replay_does_not_rebind_cas(self) -> None:
        """P2-2: a later identical replay must not rebind the disposition's
        assessment reference away from the first occurrence."""
        assessment_dup = _assessment("ASSESS-1", 0)
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", assessment_dup, 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1"),
                    1, _ZERO,
                ),
                _mk_v02_event("information_sufficiency_assessment", assessment_dup, 2, _ZERO),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_conflicting_decisions_on_same_assessment_rejected(self) -> None:
        """C1-1a: two accepted dispositions with different decisions on the
        same assessment are a §4.4 conflict."""
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("retrieval_parent_disposition", _disposition("DISP-1", "close", 0, "ASSESS-1"), 1, _ZERO),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-2", "continue", 0, "ASSESS-1", delta="x"),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("conflicts with prior accepted decision" in e for e in errors))

    def test_late_close_at_stale_revision_rejected(self) -> None:
        """C1-1b/c: after an accepted continue advanced the revision, an
        accepted close acting on the old revision is stale — the
        'new requirement accepted, old close later' race is rejected."""
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "continue", 0, "ASSESS-1", delta="x"),
                    1, _ZERO,
                ),
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-2", 1), 2, _ZERO),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-2", "close", 0, "ASSESS-1"),
                    3, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_close_record",
                    _close_record("CLOSE-1", 0, "ASSESS-1", "DISP-2"),
                    4, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("accepted on activation ACT-1 at revision 0, current revision is 1" in e for e in errors)
        )

    def test_close_references_rejected_disposition_fails(self) -> None:
        """C1-1d: only a validated close disposition (outcome accepted or
        replayed) commits a close record."""
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1", outcome="rejected_conflicting"),
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
        self.assertTrue(any("only a validated close disposition commits" in e for e in errors))

    def test_disposition_activation_mismatch_rejected(self) -> None:
        """C3-3: the disposition's activation must equal the referenced
        assessment's activation."""
        disposition = _disposition("DISP-1", "close", 0, "ASSESS-1")
        disposition["activation_id"] = "ACT-2"
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("retrieval_parent_disposition", disposition, 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("activation_id ACT-2 != referenced assessment" in e for e in errors))

    def test_close_assessment_cross_fields_mismatch_rejected(self) -> None:
        """C3-4: close record contract_id / result_digest must match the
        referenced assessment."""
        close = _close_record("CLOSE-1", 0, "ASSESS-1", "DISP-1")
        close["contract_id"] = "CONTRACT-OTHER"
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("retrieval_parent_disposition", _disposition("DISP-1", "close", 0, "ASSESS-1"), 1, _ZERO),
                _mk_v02_event("retrieval_close_record", close, 2, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("contract_id CONTRACT-OTHER != referenced assessment" in e for e in errors))

    def test_disposition_replay_with_json_semantic_equivalence_rejected(self) -> None:
        """P3-1: canonical-bytes comparison distinguishes 0 vs 0.0 — a replay
        that only swaps JSON semantics is a conflicting payload."""
        first = _disposition("DISP-1", "close", 0, "ASSESS-1")
        second = _disposition("DISP-1", "close", 0, "ASSESS-1")
        second["expected_contract_revision"] = 0.0
        journal = _v02_journal(
            [
                _mk_v02_event("information_sufficiency_assessment", _assessment("ASSESS-1", 0), 0, None),
                _mk_v02_event("retrieval_parent_disposition", first, 1, _ZERO),
                _mk_v02_event("retrieval_parent_disposition", second, 2, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("replayed with a conflicting payload" in e for e in errors))


def _issued_ticket(
    ticket_id: str, kind: str, seq: int, activation: str | None, template: str | None = None
) -> dict:
    """A schema-valid `control_ticket_issued` payload (ACAF Slice 1 —
    ADR-0011 §4.2 binding fields; the HMAC never reaches the journal)."""
    return {
        "ticket_id": ticket_id,
        "ticket_kind": kind,
        "session_id": "sess-main-1",
        "agent_id": "main",
        "activation_id": activation,
        "goal_version": 0,
        "goal_digest": _ZERO,
        "policy_revision": 0,
        "sequence": seq,
        "capability_scope": {
            "orientation_v1": "orientation_injection",
            "disposition_v1": "disposition_submit",
            "close_v1": "close_record",
            "goal_revision_v1": "goal_revision",
        }[kind],
        "template_sha256": template,
        "canonical_arguments_sha256": _ZERO,
        "issued_at": "2026-08-12T00:00:00Z",
        "expires_at": "2026-08-12T00:00:05Z",
        "signer_revision": 1,
        "signer_measurement": _ZERO,
    }


def _consumed_ticket(ticket_id: str, kind: str) -> dict:
    return {
        "ticket_id": ticket_id,
        "ticket_kind": kind,
        "consumed_at": "2026-08-12T00:00:01Z",
        "outcome": "accepted",
    }


def _rejected_ticket(ticket_id: str | None, kind: str, code: str) -> dict:
    return {
        "ticket_id": ticket_id,
        "ticket_kind": kind,
        "rejected_at": "2026-08-12T00:00:01Z",
        "reject_code": code,
        "detail": "review fixture",
    }


class ControlTicketPairingTests(unittest.TestCase):
    """ACAF Slice 1 (ADR-0011 §4.2/§4.6) — the issued → consumed|rejected
    pairing rule on synthetic v0.2 journals (review C1-1 2026-08-12: the
    rule must have its own positive/negative coverage, not just the
    captured-journal path)."""

    def test_issued_then_consumed_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_issued",
                    _issued_ticket("TKT-0001", "orientation_v1", 1, None, template=_ZERO),
                    0,
                    None,
                ),
                _mk_v02_event(
                    "control_ticket_consumed",
                    _consumed_ticket("TKT-0001", "orientation_v1"),
                    1,
                    _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_consumed_references_unknown_ticket(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_consumed",
                    _consumed_ticket("TKT-9999", "close_v1"),
                    0,
                    None,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(any("references unknown ticket TKT-9999" in e for e in errors))

    def test_consumed_precedes_issued(self) -> None:
        # The consumed event exists BEFORE the issued event for the same id
        # (a synthetic "consume before issue" journal must be rejected — the
        # single-pass verifier surfaces it as the unknown-ticket rejection).
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_consumed",
                    _consumed_ticket("TKT-0001", "close_v1"),
                    0,
                    None,
                ),
                _mk_v02_event(
                    "control_ticket_issued",
                    _issued_ticket("TKT-0001", "close_v1", 1, "ACT-1"),
                    1,
                    _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("references unknown ticket TKT-0001" in e for e in errors),
            f"expected consume-before-issue rejection, got: {errors}",
        )

    def test_terminal_kind_mismatch(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_issued",
                    _issued_ticket("TKT-0001", "close_v1", 1, "ACT-1"),
                    0,
                    None,
                ),
                _mk_v02_event(
                    "control_ticket_consumed",
                    _consumed_ticket("TKT-0001", "disposition_v1"),
                    1,
                    _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("disagrees with issued" in e for e in errors),
            f"expected kind-mismatch error, got: {errors}",
        )

    def test_one_shot_terminal_is_exclusive(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_issued",
                    _issued_ticket("TKT-0001", "close_v1", 1, "ACT-1"),
                    0,
                    None,
                ),
                _mk_v02_event(
                    "control_ticket_consumed",
                    _consumed_ticket("TKT-0001", "close_v1"),
                    1,
                    _ZERO,
                ),
                _mk_v02_event(
                    "control_ticket_rejected",
                    _rejected_ticket("TKT-0001", "close_v1", "replay_detected"),
                    2,
                    _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("after it was already" in e for e in errors),
            f"expected one-shot error, got: {errors}",
        )

    def test_rejected_after_issued_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_issued",
                    _issued_ticket("TKT-0001", "orientation_v1", 1, None, template=_ZERO),
                    0,
                    None,
                ),
                _mk_v02_event(
                    "control_ticket_rejected",
                    _rejected_ticket("TKT-0001", "orientation_v1", "expired"),
                    1,
                    _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])


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


def _mode_transition(
    transition_id: str,
    old_mode: str,
    new_mode: str,
    authority: str = "session_bootstrap",
    reason_code: str = "session_default",
    capability_status: str | None = None,
) -> dict:
    payload = {
        "transition_id": transition_id,
        "session_id": "sess-main-1",
        "old_mode": old_mode,
        "new_mode": new_mode,
        "authority": authority,
        "reason_code": reason_code,
    }
    if capability_status is not None:
        payload["capability_status"] = capability_status
    return payload


def _tool_started(target: str, tool: str = "web_search") -> dict:
    return {"tool": tool, "call_id": "call-1", "target": target}


def _tool_completed(target: str, tool: str = "web_search", errored: bool = False) -> dict:
    payload: dict = {"tool": tool, "call_id": "call-1", "target": target, "exit_code": None}
    if errored:
        payload["status"] = "error"
        payload["error"] = "retrieval_capability_unavailable"
    else:
        payload["exit_code"] = 0
    return payload


class RetrievalModeTransitionTests(unittest.TestCase):
    """ADR-0010 §3.7.1 mode mechanics on synthetic v0.2 journals: at most one
    session_bootstrap transition, old_mode chains to the previous new_mode,
    and mode obligations hold until the next transition (off forbids retrieval
    dispatch/assessment/result; local_browser requires every retrieval
    dispatch to fail explicitly)."""

    def test_bootstrap_transition_to_framework_fallback_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "framework_fallback",
                        capability_status="available",
                    ),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 1, _ZERO),
                _mk_v02_event("tool_completed", _tool_completed("external_retrieval"), 2, _ZERO),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_dispatch_after_transition_to_off_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition("MODETRANS-1", "framework_fallback", "off"),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("internal_retrieval"), 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("after transition to off", " | ".join(errors))

    def test_assessment_after_transition_to_off_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition("MODETRANS-1", "framework_fallback", "off"),
                    0, None,
                ),
                _mk_v02_event(
                    "information_sufficiency_assessment",
                    _assessment("ASSESS-1", 0),
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("after transition to off", " | ".join(errors))

    def test_committed_result_after_transition_to_off_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition("MODETRANS-1", "framework_fallback", "off"),
                    0, None,
                ),
                _mk_v02_event("retrieval_result_committed", _committed_result(), 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("after transition to off", " | ".join(errors))

    def test_local_browser_dispatch_must_fail_explicitly(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="unsupported",
                    ),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 1, _ZERO),
                _mk_v02_event("tool_completed", _tool_completed("external_retrieval"), 2, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("non-error under local_browser", " | ".join(errors))

    def test_local_browser_explicit_error_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="unsupported",
                    ),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 1, _ZERO),
                _mk_v02_event(
                    "tool_completed", _tool_completed("external_retrieval", errored=True),
                    2, _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_local_browser_committed_result_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="unsupported",
                    ),
                    0, None,
                ),
                _mk_v02_event("retrieval_result_committed", _committed_result(), 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("under local_browser", " | ".join(errors))

    # ── local_browser capability_status branch (2026-08-10 local_browser
    # slice): available → successful dispatches and committed results are
    # legal; unsupported/degraded → the strict rules above; a missing
    # capability_status on a local_browser transition is itself an error.

    def test_local_browser_available_full_chain_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="available",
                    ),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 1, _ZERO),
                _mk_v02_event("tool_completed", _tool_completed("external_retrieval"), 2, _ZERO),
                _mk_v02_event("retrieval_result_committed", _committed_result(), 3, _ZERO),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_local_browser_available_explicit_error_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="available",
                    ),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 1, _ZERO),
                _mk_v02_event(
                    "tool_completed", _tool_completed("external_retrieval", errored=True),
                    2, _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_local_browser_degraded_non_error_dispatch_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="degraded",
                    ),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 1, _ZERO),
                _mk_v02_event("tool_completed", _tool_completed("external_retrieval"), 2, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("non-error under local_browser", " | ".join(errors))
        self.assertIn("degraded", " | ".join(errors))

    def test_local_browser_degraded_committed_result_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="degraded",
                    ),
                    0, None,
                ),
                _mk_v02_event("retrieval_result_committed", _committed_result(), 1, _ZERO),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("under local_browser", " | ".join(errors))

    def test_local_browser_transition_missing_capability_status_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition("MODETRANS-1", "off", "local_browser"),
                    0, None,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 1, _ZERO),
                _mk_v02_event(
                    "tool_completed", _tool_completed("external_retrieval", errored=True),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("capability_status", " | ".join(errors))

    # ── host-lane retrieval tools under off/degraded (2026-08-10 review
    # M2): browser_read's tool events carry NO `target` field (D-2 host
    # lane), so the target-based checks alone cannot see a successful
    # host-lane retrieval under an unavailable capability — the name-based
    # rule below closes that gap.

    def test_local_browser_degraded_host_lane_success_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="degraded",
                    ),
                    0, None,
                ),
                _mk_v02_event(
                    "tool_started",
                    {"tool": "browser_read", "call_id": "call-b1"},
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "tool_completed",
                    {"tool": "browser_read", "call_id": "call-b1", "exit_code": 0},
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("browser_read", " | ".join(errors))
        self.assertIn("degraded", " | ".join(errors))

    def test_local_browser_degraded_host_lane_explicit_error_validates(self) -> None:
        # The same dispatch failing explicitly stays legal.
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "local_browser",
                        capability_status="degraded",
                    ),
                    0, None,
                ),
                _mk_v02_event(
                    "tool_started",
                    {"tool": "browser_read", "call_id": "call-b1"},
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "tool_completed",
                    {
                        "tool": "browser_read", "call_id": "call-b1",
                        "status": "error", "error": "browser_read_failed",
                    },
                    2, _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_off_host_lane_dispatch_rejected(self) -> None:
        # The off rule covers host-lane retrieval dispatches the same way
        # (browser_read has no target field).
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition("MODETRANS-1", "off", "off"),
                    0, None,
                ),
                _mk_v02_event(
                    "tool_started",
                    {"tool": "browser_read", "call_id": "call-b1"},
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("browser_read", " | ".join(errors))
        self.assertIn("off", " | ".join(errors))

    def test_second_bootstrap_transition_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "framework_fallback",
                        capability_status="available",
                    ),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-2", "framework_fallback", "off",
                        authority="user", reason_code="explicit_selection",
                    ),
                    1, _ZERO,
                ),
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-3", "off", "framework_fallback",
                        authority="session_bootstrap",
                        capability_status="available",
                    ),
                    2, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("at most one allowed", " | ".join(errors))

    def test_disjoint_old_mode_chain_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-1", "off", "framework_fallback",
                        capability_status="available",
                    ),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-2", "off", "off",
                        authority="user", reason_code="explicit_selection",
                    ),
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("old_mode", " | ".join(errors))

    def test_mode_obligations_end_at_next_transition(self) -> None:
        """off-mode forbids dispatch only until the next transition — the
        transition back to framework_fallback restores dispatch legality."""
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition("MODETRANS-1", "off", "off"),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_mode_transition",
                    _mode_transition(
                        "MODETRANS-2", "off", "framework_fallback",
                        authority="user", reason_code="explicit_selection",
                        capability_status="available",
                    ),
                    1, _ZERO,
                ),
                _mk_v02_event("tool_started", _tool_started("external_retrieval"), 2, _ZERO),
                _mk_v02_event("tool_completed", _tool_completed("external_retrieval"), 3, _ZERO),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])


def _committed_result() -> dict:
    """A minimal valid committed retrieval result (schema-valid; ledger with
    one full-text source and one partial source, counts derived)."""
    return {
        "schema_version": "0.2.0-draft",
        "result_kind": "retrieval_subagent_result",
        "result_id": "RET-RES-1",
        "activation_id": "ACT-1",
        "subagent_session_id": "sess-ext-1",
        "contract_id": "CONTRACT-1",
        "contract_revision": 0,
        "result_digest": _ZERO,
        "ledger_digest": _ZERO,
        "query_summary": [
            {
                "query_id": "QRY-1",
                "query_text": "rust channels",
                "source_category": "official_docs",
                "result_count": 2,
                "action_taken": "searched",
                "tool_used": "web_search",
            }
        ],
        "source_ledger": [
            {
                "source_id": "SRC-1",
                "source_title": "Rust reference",
                "source_url_or_ref": "https://doc.rust-lang.org/reference",
                "source_type": "web_page",
                "visibility": "full_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "full document",
                "missing_scope": "none",
                "relevance": "direct",
                "used_in_sections": ["Rust channels"],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "observed",
            },
            {
                "source_id": "SRC-2",
                "source_title": "Rust forum thread",
                "source_url_or_ref": "https://forum.rust-lang.org/t/42",
                "source_type": "web_page",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "first section",
                "missing_scope": "rest",
                "relevance": "partial",
                "used_in_sections": [],
                "highest_allowed_claim": "derived",
            },
        ],
        "filtering_log": [],
        "organized_response": {
            "sections": [
                {
                    "section_title": "Rust channels",
                    "content": "std::sync::mpsc provides channels.",
                    "source_ids": ["SRC-1"],
                    "claim_strength": "observed",
                }
            ],
            "claims": [],
        },
        "raw_source_refs": [
            {
                "source_id": "SRC-1",
                "source_title": "Rust reference",
                "source_url_or_ref": "https://doc.rust-lang.org/reference",
                "visibility": "full_text_observed",
                "content_sha256": _ZERO,
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
    }


class RetrievalResultConsistencyTests(unittest.TestCase):
    """ADR-0010 §3.3.3/§3.7.5 mechanical consistency of committed results:
    source_counts equal the ledger's visibility distribution; claim_strength
    never exceeds the bound sources' visibility; the following assessment on
    the same (activation, revision) carries identical digests and counts."""

    def _journal(self, *extra: dict) -> str:
        events = [_mk_v02_event("retrieval_result_committed", _committed_result(), 0, None)]
        seq = 1
        previous = events[-1]["event_sha256"]
        for event in extra:
            event["sequence"] = seq
            event["previous_event_sha256"] = previous
            event["event_sha256"] = _ZERO
            event["event_sha256"] = _event_sha256(event)
            events.append(event)
            previous = event["event_sha256"]
            seq += 1
        return _v02_journal(events)

    def test_consistent_commit_and_assessment_validates(self) -> None:
        journal = self._journal(
            _mk_v02_event(
                "information_sufficiency_assessment",
                _assessment("ASSESS-1", 0),
                1, _ZERO,
            )
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_source_counts_must_match_ledger_distribution(self) -> None:
        result = _committed_result()
        result["source_counts"]["full_text_observed"] = 2
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("source_counts", " | ".join(errors))

    def test_observed_claim_requires_full_text_source(self) -> None:
        result = _committed_result()
        result["organized_response"]["sections"][0]["source_ids"] = ["SRC-2"]
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("exceeds source", " | ".join(errors))

    def test_claim_binding_unknown_source_rejected(self) -> None:
        result = _committed_result()
        result["organized_response"]["sections"][0]["source_ids"] = ["SRC-999"]
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("unknown source", " | ".join(errors))

    def test_assessment_digest_must_match_committed_result(self) -> None:
        assessment = _assessment("ASSESS-1", 0)
        assessment["result_digest"] = "1" * 64
        journal = self._journal(
            _mk_v02_event("information_sufficiency_assessment", assessment, 1, _ZERO)
        )
        errors = validate_journal_text(journal)
        self.assertIn("result_digest", " | ".join(errors))

    def test_assessment_source_counts_must_match_committed_result(self) -> None:
        assessment = _assessment("ASSESS-1", 0)
        assessment["source_counts"]["total"] = 7
        journal = self._journal(
            _mk_v02_event("information_sufficiency_assessment", assessment, 1, _ZERO)
        )
        errors = validate_journal_text(journal)
        self.assertIn("source_counts", " | ".join(errors))

    def test_assessment_before_commit_is_exempt(self) -> None:
        """An assessment that PRECEDES the commit (e.g. a budget-exhaustion
        partial assessment) is not cross-checked against the later result."""
        assessment = _assessment("ASSESS-1", 0)
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "information_sufficiency_assessment", assessment, 0, None
                ),
                _mk_v02_event(
                    "retrieval_result_committed", _committed_result(), 1, _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_highest_allowed_claim_must_match_visibility(self) -> None:
        """D1 (review 2026-08-10): the declared highest_allowed_claim is a
        mechanical projection of the entry's visibility (§3.7.5) — a
        mismatched declaration means the ledger lies about its own cap."""
        result = _committed_result()
        result["source_ledger"][0]["highest_allowed_claim"] = "derived"
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_result_committed", result, 0, None
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertEqual(len(errors), 1, errors)
        self.assertIn("highest_allowed_claim", errors[0])
        self.assertIn("SRC-1", errors[0])


def _restore(
    restore_id: str,
    assessment_id: str | None,
    revision: int,
    activation: str = "ACT-1",
    status: str = "awaiting_disposition",
) -> dict:
    payload = {
        "restore_id": restore_id,
        "activation_id": activation,
        "subagent_session_id": "sess-ext-1",
        "contract_id": "CONTRACT-1",
        "contract_revision": revision,
        "status": status,
        "result_digest": _ZERO,
        "origin_run_id": "RUN-PREV-0001",
        "sidecar_ref": ".gsa/activations/sess-abc.json",
        "tool_rounds_used": 3,
    }
    if assessment_id is not None:
        payload["assessment_id"] = assessment_id
    return payload


class RetrievalActivationRestoreTests(unittest.TestCase):
    """ADR-0010 §3.3 cross-run activation restore on synthetic v0.2 journals:
    a restore declaration legalizes a parent disposition referencing the
    earlier run's assessment; the same activation is restored at most once
    per journal; the close record still binds through the declaration."""

    def test_restore_then_disposition_then_close_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_activation_restored",
                    _restore("RST-ACT-1", "ASSESS-1", 0),
                    0, None,
                ),
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
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_second_restore_of_same_activation_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_activation_restored",
                    _restore("RST-ACT-1", "ASSESS-1", 0),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_activation_restored",
                    _restore("RST-ACT-2", "ASSESS-1", 0),
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("second restore", " | ".join(errors))

    def test_disposition_before_restore_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1"),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_activation_restored",
                    _restore("RST-ACT-1", "ASSESS-1", 0),
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("does not precede", " | ".join(errors))

    def test_restore_activation_mismatch_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_activation_restored",
                    _restore("RST-ACT-1", "ASSESS-1", 0),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1", activation="ACT-2"),
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("activation_id", " | ".join(errors))

    def test_restore_revision_cas_enforced(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_activation_restored",
                    _restore("RST-ACT-1", "ASSESS-1", 1),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1"),
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("expected_contract_revision", " | ".join(errors))

    def test_active_restore_declares_no_assessment(self) -> None:
        """A status=active restore has no assessment to declare — a
        disposition still referencing ASSESS-1 stays unknown."""
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "retrieval_activation_restored",
                    _restore("RST-ACT-1", None, 0, status="active"),
                    0, None,
                ),
                _mk_v02_event(
                    "retrieval_parent_disposition",
                    _disposition("DISP-1", "close", 0, "ASSESS-1"),
                    1, _ZERO,
                ),
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("unknown assessment", " | ".join(errors))


if __name__ == "__main__":
    unittest.main()
