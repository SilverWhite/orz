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
    _verify_v02_checkpoint_responses,
    _verify_v02_inject_budget,
    _verify_v02_policy_denial,
    _verify_v02_probe_accuracy,
    _verify_v02_request_header,
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


def _v02_event(event_type: str, payload: dict) -> dict:
    return {
        "payload_schema": "run-event-v0.2.schema.json",
        "event_type": event_type,
        "payload": payload,
    }


_CHECKPOINT_FIRE = _v02_event(
    "orientation_checkpoint",
    {
        "checkpoint_id": "ORIENT-RUN-1-0000",
        "inquiry_family": "neutral",
        "inquiry_kind": "orientation_checkpoint",
        "agent_role": "main",
    },
)

_CHECKPOINT_FIRE_DC = _v02_event(
    "diagnostic_coverage_checkpoint",
    {
        "checkpoint_id": "DIAG-COV-RUN-1-0001",
        "inquiry_family": "neutral",
        "inquiry_kind": "diagnostic_coverage_checkpoint",
        # 2026-08-15 复核：DC fire 可选携带 agent_role（主车道恒 main）；
        # 验证器对无该字段的历史 fire 保持兼容。
    },
)


def _checkpoint_response(**overrides: object) -> dict:
    outcome = overrides.get("outcome", "accepted")
    if outcome in ("refill_requested", "degraded") and "validation" not in overrides:
        overrides["validation"] = {
            "valid": False,
            "errors": ["mechanical_validation_failed"],
            "ignored_fields": [],
        }
    payload = {
        "checkpoint_id": "ORIENT-RUN-1-0000",
        "inquiry_family": "neutral",
        "inquiry_kind": "orientation_checkpoint",
        "agent_role": "main",
        "attempt": 1,
        "outcome": outcome,
        "response": {
            "task_position": "修复缓存回归",
            "progress_evidence": ["src/cache.rs"],
            "blockers": [],
            "missing_evidence": [],
            "next_action": "continue",
            "changed_direction": False,
        },
        "validation": {"valid": True, "errors": [], "ignored_fields": []},
        "cross_check": {
            "evidence_identity_found": [],
            "evidence_identity_missing": [],
            "gather_evidence_missing_surface_provided": True,
        },
        "degrade_reason": None,
    }
    payload.update(overrides)
    return _v02_event("checkpoint_response", payload)

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
    # FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the final-answer citation
    # verifier blocks an unknown-source marker with a mechanical degradation.
    "citation-validation-block.jsonl",
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
    # FUS-TOOL-PROBE P0-A-2 审查复核（2026-08-13）：评估落地后
    # has_live_activation 置真（未决 pending assessment）、disposition 消费后
    # 清除——每轮 retrieval_disposition 探针翻转各记一次
    # tool_availability_check。
    "orientation-fire-run.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "tool_availability_check",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "tool_availability_check",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "tool_availability_check",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "tool_availability_check",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "tool_availability_check",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "tool_availability_check",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment", "orientation_checkpoint",
        "tool_availability_check",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "tool_completed",
        "tool_availability_check",
        "model_output", "tool_started", "model_output",
        "runtime_stagnation_guard", "tool_completed",
        "retrieval_result_committed",
        "information_sufficiency_assessment",
        "tool_availability_check",
        "model_output", "tool_started", "retrieval_parent_disposition",
        "retrieval_close_record", "tool_completed",
        "tool_availability_check",
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
        "tool_availability_check",
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
        "information_sufficiency_assessment",
        "tool_availability_check",
        "model_output",
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
        "retrieval_close_record", "tool_completed",
        "tool_availability_check",
        "model_output",
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
    # FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the post-gate final answer
    # carries `[来源: SRC-999]` (not in this run's evidence) — the citation
    # verifier blocks it; the journal records the mechanical event and the
    # run finishes normally.
    "citation-validation-block.jsonl": (
        "run_preflight", "tool_availability_check", "run_started",
        "prompt_submitted", "model_output", "counterexample_gate",
        "model_output", "citation_validation", "runtime_stagnation_guard",
        "run_finished",
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

    def test_all_v02_journals_validate(self) -> None:
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


class PlanWriteSequenceRuleTests(unittest.TestCase):
    """2026-08-16 审查收口: plan_write outcome↔attempt↔validation
    cross-checks (ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD_DESIGN §3-§5)."""

    def _event(
        self,
        outcome: str,
        attempt: int,
        valid: bool,
        degrade_reason: str | None = None,
    ) -> dict:
        return {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "plan_write",
            "payload": {
                "plan_id": "plan-1",
                "goal": "g",
                "step_count": 1,
                "outcome": outcome,
                "attempt": attempt,
                "validation": {
                    "valid": valid,
                    "errors": [] if valid else ["x"],
                    "ignored_fields": [],
                },
                "degrade_reason": degrade_reason,
            },
        }

    def test_refill_then_accept_sequence_passes(self) -> None:
        from assurance.run_event_journal_validation import _verify_v02_plan_write

        events = [self._event("refill_requested", 1, False), self._event("accepted", 2, True)]
        self.assertEqual(_verify_v02_plan_write(events), [])

    def test_refill_without_followup_fails(self) -> None:
        from assurance.run_event_journal_validation import _verify_v02_plan_write

        errors = _verify_v02_plan_write([self._event("refill_requested", 1, False)])
        self.assertTrue(any("without a following refill attempt" in e for e in errors))

    def test_degrade_after_refill_on_attempt_two_passes(self) -> None:
        from assurance.run_event_journal_validation import _verify_v02_plan_write

        events = [
            self._event("refill_requested", 1, False),
            self._event("degraded", 2, False, "validation_failed_after_refill"),
        ]
        self.assertEqual(_verify_v02_plan_write(events), [])

    def test_mechanical_degrade_keeps_valid_true(self) -> None:
        from assurance.run_event_journal_validation import _verify_v02_plan_write

        for reason in ("plan_rotate_failed", "plan_not_submitted"):
            with self.subTest(reason=reason):
                errors = _verify_v02_plan_write(
                    [self._event("degraded", 1, True, reason)]
                )
                self.assertEqual(errors, [], reason)

    def test_accepted_requires_valid(self) -> None:
        from assurance.run_event_journal_validation import _verify_v02_plan_write

        errors = _verify_v02_plan_write([self._event("accepted", 1, False)])
        self.assertTrue(any("accepted but" in e for e in errors))


class ConsoleModeTransitionRuleTests(unittest.TestCase):
    """PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console/direct
    dual-mode transition + console_order_written cross-checks."""

    def _transition(
        self,
        *,
        transition_id: str,
        from_mode: str,
        to_mode: str,
        trigger: str,
        streak: int | None,
        order_ids: list[str],
        decision: str,
        reason: str | None,
        run_id: str = "RUN-T",
        related: str | None = None,
    ) -> dict:
        return {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "console_mode_transition",
            "run_id": run_id,
            "payload": {
                "transition_id": transition_id,
                "from": from_mode,
                "to": to_mode,
                "trigger": trigger,
                "streak": streak,
                "order_ids": order_ids,
                "model_decision": decision,
                "model_reason": reason,
                "run_id": run_id,
                "round": 4,
                "plan_epoch": 1,
                "related_transition_id": related,
            },
        }

    def test_switch_then_return_passes(self) -> None:
        from assurance.run_event_journal_validation import (
            _verify_v02_console_mode_transition,
        )

        events = [
            self._transition(
                transition_id="T1",
                from_mode="console",
                to_mode="direct",
                trigger="assistant_failure_streak",
                streak=3,
                order_ids=["ORD-1", "ORD-2", "ORD-3"],
                decision="switch",
                reason="assistant failed",
            ),
            self._transition(
                transition_id="T2",
                from_mode="direct",
                to_mode="console",
                trigger="model_return",
                streak=None,
                order_ids=[],
                decision="return_to_console",
                reason="fixed",
                related="T1",
            ),
        ]
        self.assertEqual(_verify_v02_console_mode_transition(events), [])

    def test_switch_requires_streak_and_order_ids(self) -> None:
        from assurance.run_event_journal_validation import (
            _verify_v02_console_mode_transition,
        )

        events = [
            self._transition(
                transition_id="T1",
                from_mode="console",
                to_mode="direct",
                trigger="assistant_failure_streak",
                streak=0,
                order_ids=[],
                decision="switch",
                reason=None,
            )
        ]
        errors = _verify_v02_console_mode_transition(events)
        self.assertTrue(any("streak ≥ 1" in e for e in errors), errors)
        self.assertTrue(any("non-empty order_ids" in e for e in errors), errors)

    def test_return_without_prior_switch_fails(self) -> None:
        from assurance.run_event_journal_validation import (
            _verify_v02_console_mode_transition,
        )

        events = [
            self._transition(
                transition_id="T2",
                from_mode="direct",
                to_mode="console",
                trigger="model_return",
                streak=None,
                order_ids=[],
                decision="return_to_console",
                reason=None,
                related="T1",
            )
        ]
        errors = _verify_v02_console_mode_transition(events)
        self.assertTrue(
            any("does not match an earlier console→direct" in e for e in errors),
            errors,
        )

    def test_two_streak_decisions_per_run_fail(self) -> None:
        from assurance.run_event_journal_validation import (
            _verify_v02_console_mode_transition,
        )

        events = [
            self._transition(
                transition_id="T1",
                from_mode="console",
                to_mode="console",
                trigger="assistant_failure_streak",
                streak=3,
                order_ids=["ORD-1"],
                decision="stay",
                reason="keep console",
            ),
            self._transition(
                transition_id="T2",
                from_mode="console",
                to_mode="console",
                trigger="assistant_failure_streak",
                streak=3,
                order_ids=["ORD-2"],
                decision="stay",
                reason="again",
            ),
        ]
        errors = _verify_v02_console_mode_transition(events)
        self.assertTrue(any("at most one inquiry per run" in e for e in errors), errors)

    def test_direct_tool_event_requires_matching_transition(self) -> None:
        from assurance.run_event_journal_validation import (
            _verify_v02_console_mode_transition,
        )

        base = self._transition(
            transition_id="T1",
            from_mode="console",
            to_mode="direct",
            trigger="assistant_failure_streak",
            streak=3,
            order_ids=["ORD-1"],
            decision="switch",
            reason=None,
        )
        tool = {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "tool_completed",
            "run_id": "RUN-T",
            "payload": {
                "tool": "read_file",
                "call_id": "call-1",
                "exit_code": 0,
                "console_mode": "direct",
                "transition_id": "T1",
                "trace_id": "call-1",
            },
        }
        self.assertEqual(_verify_v02_console_mode_transition([base, tool]), [])
        bad = dict(tool)
        bad["payload"] = {**tool["payload"], "transition_id": "T9"}
        errors = _verify_v02_console_mode_transition([base, bad])
        self.assertTrue(any("transition_id" in e for e in errors), errors)

    def test_order_written_requires_prior_action_write(self) -> None:
        from assurance.run_event_journal_validation import (
            _verify_v02_console_order_written,
        )

        order = {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "console_order_written",
            "run_id": "RUN-T",
            "payload": {
                "order_id": "ORD-1",
                "write_call_id": "call-write-1",
                "action": "workspace.read_file",
                "step_id": "s1",
                "round": 2,
                "plan_epoch": 1,
                "run_id": "RUN-T",
            },
        }
        errors = _verify_v02_console_order_written([order])
        self.assertTrue(any("without a prior" in e for e in errors), errors)

        write = {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "tool_completed",
            "run_id": "RUN-T",
            "payload": {
                "tool": "blackboard_action_write",
                "call_id": "call-write-1",
                "exit_code": 0,
            },
        }
        self.assertEqual(_verify_v02_console_order_written([write, order]), [])
        dup = dict(order)
        dup["event_type"] = "console_order_written"
        errors = _verify_v02_console_order_written([write, order, dup])
        self.assertTrue(any("duplicate" in e for e in errors), errors)

    def test_order_written_write_call_id_matches_producer_shape(self) -> None:
        """F1 review closure (2026-08-16): the producer emits the action_write
        completion with the model's real call_id (never ORD-xxxxx) and the
        order record carries that same call_id as write_call_id; multiple
        orders in one run must all validate (no last-write overwrite)."""
        from assurance.run_event_journal_validation import (
            _verify_v02_console_order_written,
        )

        def write(call_id: str) -> dict:
            return {
                "payload_schema": "run-event-v0.2.schema.json",
                "event_type": "tool_completed",
                "run_id": "RUN-T",
                "payload": {
                    "tool": "blackboard_action_write",
                    "call_id": call_id,
                    "exit_code": 0,
                },
            }

        def order(order_id: str, write_call_id: str, step_id: str, round_: int) -> dict:
            return {
                "payload_schema": "run-event-v0.2.schema.json",
                "event_type": "console_order_written",
                "run_id": "RUN-T",
                "payload": {
                    "order_id": order_id,
                    "write_call_id": write_call_id,
                    "action": "workspace.read_file",
                    "step_id": step_id,
                    "round": round_,
                    "plan_epoch": 1,
                    "run_id": "RUN-T",
                },
            }

        events = [
            write("call-f1"),
            order("ORD-1", "call-f1", "s1", 2),
            write("call-f2"),
            order("ORD-2", "call-f2", "s2", 3),
        ]
        self.assertEqual(_verify_v02_console_order_written(events), [])

        # Mismatched write_call_id must be rejected (no cross-order matching).
        bad = [write("call-f1"), order("ORD-1", "call-other", "s1", 2)]
        errors = _verify_v02_console_order_written(bad)
        self.assertTrue(
            any("without a prior blackboard_action_write success" in e for e in errors),
            errors,
        )

        # One write cannot back two order records.
        reused = [
            write("call-f1"),
            order("ORD-1", "call-f1", "s1", 2),
            order("ORD-2", "call-f1", "s2", 3),
        ]
        errors = _verify_v02_console_order_written(reused)
        self.assertEqual(len(errors), 1, errors)

    def test_order_rejected_requires_prior_written_order_and_matching_stamps(
        self,
    ) -> None:
        """P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): a
        console_order_rejected must be preceded by a console_order_written of
        the same run carrying the same order_id and the same mechanical
        stamps; at most one rejection per order."""
        from assurance.run_event_journal_validation import (
            _verify_v02_console_order_rejected,
        )

        written = {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "console_order_written",
            "run_id": "RUN-T",
            "payload": {
                "order_id": "ORD-1",
                "write_call_id": "call-write-1",
                "action": "workspace.read_file",
                "step_id": "s1",
                "round": 2,
                "plan_epoch": 1,
                "run_id": "RUN-T",
            },
        }
        rejected = {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "console_order_rejected",
            "run_id": "RUN-T",
            "payload": {
                "order_id": "ORD-1",
                "step": "protocol",
                "phase": "pre_issue",
                "code": "step_not_done",
                "reason": "order refused",
                "round": 2,
                "plan_epoch": 1,
                "run_id": "RUN-T",
            },
        }
        # Without the written order the rejection cannot be attributed.
        errors = _verify_v02_console_order_rejected([rejected])
        self.assertTrue(
            any("without a prior console_order_written" in e for e in errors), errors
        )
        self.assertEqual(_verify_v02_console_order_rejected([written, rejected]), [])

        # Stamp mismatch against the written order must be caught.
        bad_stamp = {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "console_order_rejected",
            "run_id": "RUN-T",
            "payload": {
                **rejected["payload"],
                "round": 9,
            },
        }
        errors = _verify_v02_console_order_rejected([written, bad_stamp])
        self.assertTrue(any("round=9" in e for e in errors), errors)

        # A rejected order is consumed — one rejection per written order.
        dup = dict(rejected)
        dup["payload"] = dict(rejected["payload"])
        errors = _verify_v02_console_order_rejected([written, rejected, dup])
        self.assertTrue(any("duplicate console_order_rejected" in e for e in errors), errors)

    def test_order_rejected_phase_step_code_consistency(self) -> None:
        """P0-E 第 4 项: pre_issue rejections are protocol-step with the three
        pre-issuance codes; issue rejections are registry/contract/target/
        policy steps — execute/verify never appear (executed orders journal
        through tool_started/tool_completed)."""
        from assurance.run_event_journal_validation import (
            _verify_v02_console_order_rejected,
        )

        base_written = {
            "payload_schema": "run-event-v0.2.schema.json",
            "event_type": "console_order_written",
            "run_id": "RUN-T",
            "payload": {
                "order_id": "ORD-1",
                "write_call_id": "call-write-1",
                "action": "workspace.read_file",
                "step_id": "s1",
                "round": 2,
                "plan_epoch": 1,
                "run_id": "RUN-T",
            },
        }

        def rejected(order_id: str, phase: str, step: str, code: str) -> dict:
            return {
                "payload_schema": "run-event-v0.2.schema.json",
                "event_type": "console_order_rejected",
                "run_id": "RUN-T",
                "payload": {
                    "order_id": order_id,
                    "step": step,
                    "phase": phase,
                    "code": code,
                    "reason": "refused",
                    "round": 2,
                    "plan_epoch": 1,
                    "run_id": "RUN-T",
                },
            }

        # Valid: pre_issue + protocol + step_not_done; issue + policy +
        # policy_denied (ACAF/mode/permission normalization).
        events = [
            base_written,
            rejected("ORD-1", "pre_issue", "protocol", "step_not_done"),
            {
                **base_written,
                "payload": {
                    **base_written["payload"],
                    "order_id": "ORD-2",
                },
            },
            rejected("ORD-2", "issue", "policy", "policy_denied"),
        ]
        self.assertEqual(_verify_v02_console_order_rejected(events), [])

        # pre_issue must be protocol-step with a pre-issuance code.
        bad1 = rejected("ORD-1", "pre_issue", "contract", "step_not_done")
        errors = _verify_v02_console_order_rejected([base_written, bad1])
        self.assertTrue(any("requires step=protocol" in e for e in errors), errors)
        bad2 = rejected("ORD-1", "pre_issue", "protocol", "policy_denied")
        errors = _verify_v02_console_order_rejected([base_written, bad2])
        self.assertTrue(
            any("must be order_stale/step_not_done/budget_insufficient" in e for e in errors),
            errors,
        )

        # issue must be a gate step — execute/verify are excluded.
        bad3 = rejected("ORD-1", "issue", "execute", "execution_failed")
        errors = _verify_v02_console_order_rejected([base_written, bad3])
        self.assertTrue(
            any("must be registry/contract/target/policy" in e for e in errors), errors
        )

        # Unknown phase must be caught.
        bad4 = rejected("ORD-1", "executed", "protocol", "step_not_done")
        errors = _verify_v02_console_order_rejected([base_written, bad4])
        self.assertTrue(any("phase must be pre_issue or issue" in e for e in errors), errors)

    def test_ledger_fold_advance_window_invariants(self) -> None:
        """FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): every
        ledger_fold_advance carries a valid fold point (fold_start <
        fold_cut, rounds_folded ≥ 1, lane enum); inside a fold window
        fold_start is constant and fold_cut strictly increases; a
        context_compressed reset starts a fresh window."""
        from assurance.run_event_journal_validation import (
            _verify_v02_ledger_fold_advance,
        )

        def advance(
            run_id: str,
            fold_start: int,
            fold_cut: int,
            rounds_folded: int = 2,
            estimate: int = 128000,
            agent_role: str = "main",
        ) -> dict:
            return {
                "payload_schema": "run-event-v0.2.schema.json",
                "event_type": "ledger_fold_advance",
                "run_id": run_id,
                "payload": {
                    "fold_start": fold_start,
                    "fold_cut": fold_cut,
                    "rounds_folded": rounds_folded,
                    "view_estimate_tokens": estimate,
                    "agent_role": agent_role,
                },
            }

        def compact(run_id: str) -> dict:
            return {
                "payload_schema": "run-event-v0.2.schema.json",
                "event_type": "context_compressed",
                "run_id": run_id,
                "payload": {
                    "trigger_tokens": 192000,
                    "target_tokens": 9000,
                    "rounds_since_last_compaction": 20,
                    "rounds_dropped": 10,
                    "messages_dropped": 30,
                    "messages_kept": 8,
                    "estimated_tokens_after": 9000,
                    "mode": "template_summary",
                    "reason": "rhythm",
                    "summary_id": "compaction-RUN-T-0001",
                    "summary_digest": "a" * 64,
                    "summary_path": ".gsa/compaction/x.md",
                    "summary_incomplete": False,
                    "retained_rounds": 2,
                    "guard_failed": False,
                    "archive_write_failed": False,
                },
            }

        # Valid window: fold_start constant, fold_cut strictly increasing,
        # rounds_folded non-decreasing.
        events = [
            advance("RUN-T", 1, 5, rounds_folded=2),
            advance("RUN-T", 1, 9, rounds_folded=4),
        ]
        self.assertEqual(_verify_v02_ledger_fold_advance(events), [])

        # A compaction reset opens a fresh window — the next advance may
        # use any indices (re-accumulation from the marker).
        events = [
            advance("RUN-T", 1, 9, rounds_folded=4),
            compact("RUN-T"),
            advance("RUN-T", 0, 4, rounds_folded=1),
        ]
        self.assertEqual(_verify_v02_ledger_fold_advance(events), [])

        # Invariant violations.
        bad_invariant = advance("RUN-T", 5, 1)
        errors = _verify_v02_ledger_fold_advance([bad_invariant])
        self.assertTrue(any("fold_start < fold_cut" in e for e in errors), errors)

        shrinking_cut = [
            advance("RUN-T", 1, 9, rounds_folded=4),
            advance("RUN-T", 1, 5, rounds_folded=5),
        ]
        errors = _verify_v02_ledger_fold_advance(shrinking_cut)
        self.assertTrue(any("fold_cut must" in e for e in errors), errors)

        changed_start = [
            advance("RUN-T", 1, 9, rounds_folded=4),
            advance("RUN-T", 2, 11, rounds_folded=5),
        ]
        errors = _verify_v02_ledger_fold_advance(changed_start)
        self.assertTrue(any("fold_start changed" in e for e in errors), errors)

        shrinking_rounds = [
            advance("RUN-T", 1, 9, rounds_folded=4),
            advance("RUN-T", 1, 11, rounds_folded=3),
        ]
        errors = _verify_v02_ledger_fold_advance(shrinking_rounds)
        self.assertTrue(any("rounds_folded" in e for e in errors), errors)

        bad_role = advance("RUN-T", 1, 5, agent_role="orchestrator")
        errors = _verify_v02_ledger_fold_advance([bad_role])
        self.assertTrue(any("agent_role must be" in e for e in errors), errors)

    def test_ledger_fold_write_failed_burst_invariants(self) -> None:
        """ADR-0010 §14.28 审查修复: every ledger_fold_write_failed carries
        the full audit shape (non-empty ledger_path, attempt ≥ 1, disabled
        bool, rows ≥ 1, lane enum); consecutive failures increase attempt
        by 1; disabled == (attempt >= 3) per the producer budget; no
        further events after the budget is exhausted."""
        from assurance.run_event_journal_validation import (
            _verify_v02_ledger_fold_write_failed,
        )

        def failed(
            run_id: str,
            attempt: int,
            disabled: bool,
            agent_role: str = "main",
            rows: int = 3,
        ) -> dict:
            return {
                "payload_schema": "run-event-v0.2.schema.json",
                "event_type": "ledger_fold_write_failed",
                "run_id": run_id,
                "payload": {
                    "ledger_path": "/app/.gsa/ledger/current.md",
                    "attempt": attempt,
                    "disabled": disabled,
                    "rows": rows,
                    "view_estimate_tokens": 135000,
                    "agent_role": agent_role,
                },
            }

        def advance(run_id: str) -> dict:
            return {
                "payload_schema": "run-event-v0.2.schema.json",
                "event_type": "ledger_fold_advance",
                "run_id": run_id,
                "payload": {
                    "fold_start": 1,
                    "fold_cut": 5,
                    "rounds_folded": 2,
                    "view_estimate_tokens": 128000,
                    "agent_role": "main",
                },
            }

        # Valid burst: attempts 1,2,3 with disabled flipping at the budget.
        events = [
            failed("RUN-F", 1, False),
            failed("RUN-F", 2, False),
            failed("RUN-F", 3, True),
        ]
        self.assertEqual(_verify_v02_ledger_fold_write_failed(events), [])

        # A successful append (ledger_fold_advance) resets the counter — a
        # fresh burst starts at 1.
        events = [
            failed("RUN-F", 1, False),
            failed("RUN-F", 2, False),
            advance("RUN-F"),
            failed("RUN-F", 1, False),
        ]
        self.assertEqual(_verify_v02_ledger_fold_write_failed(events), [])

        # Attempt must increase by exactly 1 within a burst.
        gap = [
            failed("RUN-F", 1, False),
            failed("RUN-F", 3, True),
        ]
        errors = _verify_v02_ledger_fold_write_failed(gap)
        self.assertTrue(any("increase by 1" in e for e in errors), errors)

        # disabled must follow the producer budget (attempt >= 3).
        wrong_disabled = failed("RUN-F", 2, True)
        errors = _verify_v02_ledger_fold_write_failed([wrong_disabled])
        self.assertTrue(any("disabled must" in e for e in errors), errors)

        # No further events once the budget is exhausted.
        after_exhaustion = [
            failed("RUN-F", 3, True),
            failed("RUN-F", 4, True),
        ]
        errors = _verify_v02_ledger_fold_write_failed(after_exhaustion)
        self.assertTrue(any("budget was exhausted" in e for e in errors), errors)

        # Shape violations.
        bad_role = failed("RUN-F", 1, False, agent_role="orchestrator")
        errors = _verify_v02_ledger_fold_write_failed([bad_role])
        self.assertTrue(any("agent_role must be" in e for e in errors), errors)

        zero_rows = failed("RUN-F", 1, False, rows=0)
        errors = _verify_v02_ledger_fold_write_failed([zero_rows])
        self.assertTrue(any("rows must be" in e for e in errors), errors)

        missing_path = failed("RUN-F", 1, False)
        missing_path["payload"]["ledger_path"] = ""
        errors = _verify_v02_ledger_fold_write_failed([missing_path])
        self.assertTrue(any("ledger_path must" in e for e in errors), errors)


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
                event["payload"]["complete"] = "not-an-array"
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
    ticket_id: str,
    kind: str,
    seq: int,
    activation: str | None,
    template: str | None = None,
    resolved_target: str | None = None,
) -> dict:
    """A schema-valid `control_ticket_issued` payload (ACAF Slice 1 —
    ADR-0011 §4.2 binding fields; the HMAC never reaches the journal).
    Action kinds (Slice 2 first phase) carry `resolved_target_sha256` — the
    digest of the parsed real target object (§4.2 check 5 TOCTOU); control
    kinds leave it absent so historical journals stay valid."""
    payload = {
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
            "file_write_v1": "file_write",
            "credential_read_v1": "credential_read",
            "command_exec_v1": "command_exec",
            "network_v1": "network",
        }[kind],
        "template_sha256": template,
        "canonical_arguments_sha256": _ZERO,
        "issued_at": "2026-08-12T00:00:00Z",
        "expires_at": "2026-08-12T00:00:05Z",
        "signer_revision": 1,
        "signer_measurement": _ZERO,
    }
    if resolved_target is not None:
        payload["resolved_target_sha256"] = resolved_target
    return payload


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

    def test_action_kind_pairing_is_kind_agnostic(self) -> None:
        """Slice 2: a file_write_v1 / command_exec_v1 pair goes through the
        same issued → consumed rule (the pairing rule is kind-agnostic)."""
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_issued",
                    _issued_ticket("TKT-0001", "file_write_v1", 1, None, resolved_target=_ZERO),
                    0,
                    None,
                ),
                _mk_v02_event(
                    "control_ticket_consumed",
                    _consumed_ticket("TKT-0001", "file_write_v1"),
                    1,
                    _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_command_exec_pairing_validates(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "control_ticket_issued",
                    _issued_ticket("TKT-0002", "command_exec_v1", 1, None, resolved_target=_ZERO),
                    0,
                    None,
                ),
                _mk_v02_event(
                    "control_ticket_consumed",
                    _consumed_ticket("TKT-0002", "command_exec_v1"),
                    1,
                    _ZERO,
                ),
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])


class ActionTicketSchemaTests(unittest.TestCase):
    """ACAF Slice 2 (2026-08-12) — the file_write / credential_read /
    command_exec / network ticket kinds on the issued payload schema:
    resolved_target_sha256 binding (action kinds required non-null, control
    kinds null-or-absent), activation must be null for action kinds (main
    lane has no activation)."""

    def _journal_errors(self, issued: dict) -> list[str]:
        return validate_journal_text(
            _v02_journal(
                [
                    _mk_v02_event("control_ticket_issued", issued, 0, None),
                    _mk_v02_event(
                        "control_ticket_consumed",
                        _consumed_ticket(issued["ticket_id"], issued["ticket_kind"]),
                        1,
                        _ZERO,
                    ),
                ]
            )
        )

    def test_file_write_valid_with_target(self) -> None:
        errors = self._journal_errors(
            _issued_ticket("TKT-0001", "file_write_v1", 1, None, resolved_target=_ZERO)
        )
        self.assertEqual(errors, [])

    def test_credential_read_valid_with_target(self) -> None:
        errors = self._journal_errors(
            _issued_ticket("TKT-0001", "credential_read_v1", 1, None, resolved_target=_ZERO)
        )
        self.assertEqual(errors, [])

    def test_command_exec_valid_with_target(self) -> None:
        errors = self._journal_errors(
            _issued_ticket("TKT-0001", "command_exec_v1", 1, None, resolved_target=_ZERO)
        )
        self.assertEqual(errors, [])

    def test_network_valid_with_target(self) -> None:
        errors = self._journal_errors(
            _issued_ticket("TKT-0001", "network_v1", 1, None, resolved_target=_ZERO)
        )
        self.assertEqual(errors, [])

    def test_file_write_missing_target_rejected(self) -> None:
        errors = self._journal_errors(_issued_ticket("TKT-0001", "file_write_v1", 1, None))
        self.assertTrue(any("'resolved_target_sha256' is a required property" in e for e in errors))

    def test_file_write_with_activation_rejected(self) -> None:
        """Action kinds are main-lane only — activation must be null (D2)."""
        errors = self._journal_errors(
            _issued_ticket(
                "TKT-0001", "file_write_v1", 1, "ACT-1", resolved_target=_ZERO
            )
        )
        self.assertTrue(any("is not of type 'null'" in e for e in errors))

    def test_close_with_target_rejected(self) -> None:
        """Control kinds bind no target — a present non-null resolved target
        violates the control-kind null constraint (D11)."""
        errors = self._journal_errors(
            _issued_ticket("TKT-0001", "close_v1", 1, "ACT-1", resolved_target=_ZERO)
        )
        self.assertTrue(any("is not of type 'null'" in e for e in errors))

    def test_unknown_kind_rejected(self) -> None:
        """A kind outside the closed enum is rejected (mode_change_v1 stays
        a later-phase kind). Built by hand: the helper's capability map is
        keyed on known kinds only."""
        issued = _issued_ticket("TKT-0001", "file_write_v1", 1, None, resolved_target=_ZERO)
        issued["ticket_kind"] = "mode_change_v1"
        errors = self._journal_errors(issued)
        self.assertTrue(any("is not one of" in e for e in errors))


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
                "tier": "default",
                "mechanical_weight": 1.0,
                "weight_reason": "default",
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
                "tier": "default",
                "mechanical_weight": 1.0,
                "weight_reason": "default",
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


def _pool_entry(
    url: str,
    canonical_url: str | None = None,
    tier: str = "default",
    weight: float = 1.0,
    relevance: str = "partial",
    form_reasons: list[str] | None = None,
) -> dict:
    """A valid candidate_pool entry mirroring one retained candidate URL."""
    return {
        "url": url,
        "canonical_url": canonical_url or url,
        "tier": tier,
        "mechanical_weight": weight,
        "weight_reason": "default",
        "relevance": relevance,
        "form_reasons": form_reasons or [],
    }


class CitationValidationTests(unittest.TestCase):
    """FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): ADR-0010 §3.7.9 output-level
    citation verifier events — block must degrade with mechanical reason codes
    and marker details; pass must not degrade; counts and marker statuses stay
    consistent."""

    def _citation_block_payload(
        self,
        *,
        decision: str = "block",
        degraded: bool = True,
        reason_codes: list[str] | None = None,
        markers: list[dict] | None = None,
        marker_count: int | None = None,
    ) -> dict:
        reasons = reason_codes if reason_codes is not None else ["unknown_source_id"]
        marker_list = markers if markers is not None else [
            {
                "index": 0,
                "raw": "[来源: SRC-999]",
                "target": "SRC-999",
                "binding": "ledger_source_id",
                "status": "failed",
                "reason_codes": reasons,
            }
        ]
        return {
            "schema_version": "0.2.0-draft",
            "position": "final_answer",
            "decision": decision,
            "marker_count": len(marker_list) if marker_count is None else marker_count,
            "reason_codes": reasons,
            "degraded": degraded,
            "message_block": (
                "[CITATION_VALIDATION_FAILED v0.1]\n"
                "最终回答的引用标记未通过机械校验，已阻止交付。\n"
                f"reason_codes: {', '.join(reasons)}\n"
                "[/CITATION_VALIDATION_FAILED]"
            ),
            "markers": marker_list,
        }

    def test_valid_citation_block_journal(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "citation_validation", self._citation_block_payload(), 0, None
                )
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_citation_block_must_degrade(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "citation_validation",
                    self._citation_block_payload(degraded=False),
                    0,
                    None,
                )
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("payload schema violation" in e for e in errors),
            f"expected a schema violation, got {errors}",
        )

    def test_citation_block_needs_reason_codes(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "citation_validation",
                    self._citation_block_payload(reason_codes=[]),
                    0,
                    None,
                )
            ]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any("payload schema violation" in e for e in errors),
            f"expected a schema violation, got {errors}",
        )

    def test_citation_block_message_must_be_mechanical(self) -> None:
        payload = self._citation_block_payload()
        payload["message_block"] = "随便一段文本"
        journal = _v02_journal(
            [_mk_v02_event("citation_validation", payload, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("non-mechanical message block", " | ".join(errors))

    def test_citation_marker_count_mismatch_rejected(self) -> None:
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "citation_validation",
                    self._citation_block_payload(marker_count=2),
                    0,
                    None,
                )
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("marker_count 2 != 1", " | ".join(errors))

    def test_citation_passed_marker_with_reason_codes_rejected(self) -> None:
        markers = [
            {
                "index": 0,
                "raw": "[来源: SRC-001]",
                "target": "SRC-001",
                "binding": "ledger_source_id",
                "status": "passed",
                "reason_codes": ["unknown_source_id"],
            }
        ]
        journal = _v02_journal(
            [
                _mk_v02_event(
                    "citation_validation",
                    self._citation_block_payload(markers=markers),
                    0,
                    None,
                )
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("passed citation marker", " | ".join(errors))


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

    # ── GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 ──

    def test_web_page_requires_mechanical_tier(self) -> None:
        result = _committed_result()
        del result["source_ledger"][0]["tier"]
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("tier", " | ".join(errors))

    def test_tier_weight_pair_is_fixed(self) -> None:
        result = _committed_result()
        result["source_ledger"][0]["tier"] = "authoritative"
        result["source_ledger"][0]["mechanical_weight"] = 1.0
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertTrue(
            any(
                "1.0" in error or "mechanical_weight" in error or "expected" in error
                for error in errors
            ),
            errors,
        )

    def test_partial_model_annotation_fields_rejected(self) -> None:
        result = _committed_result()
        result["source_ledger"][0]["model_weight"] = 1.0
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("model_weight_reason", " | ".join(errors))

    def test_annotated_status_requires_low_weight(self) -> None:
        result = _committed_result()
        result["source_ledger"][0]["model_weight"] = 1.0
        result["source_ledger"][0]["model_weight_reason"] = "forum"
        result["source_ledger"][0]["annotation_status"] = "annotated"
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("annotated requires model_weight 0.7", " | ".join(errors))

    def test_low_quality_used_without_annotation_rejected(self) -> None:
        result = _committed_result()
        result["source_ledger"][1]["tier"] = "low_quality"
        result["source_ledger"][1]["mechanical_weight"] = 0.7
        result["source_ledger"][1]["weight_reason"] = "low_quality_platform:csdn.net"
        result["organized_response"]["sections"][0]["source_ids"] = ["SRC-2"]
        result["organized_response"]["sections"][0]["claim_strength"] = "derived"
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("without annotated status", " | ".join(errors))

    def test_low_quality_annotated_validates(self) -> None:
        result = _committed_result()
        result["source_ledger"][1]["tier"] = "low_quality"
        result["source_ledger"][1]["mechanical_weight"] = 0.7
        result["source_ledger"][1]["weight_reason"] = "low_quality_platform:csdn.net"
        result["source_ledger"][1]["model_weight"] = 0.7
        result["source_ledger"][1]["model_weight_reason"] = "platform blog"
        result["source_ledger"][1]["annotation_status"] = "annotated"
        result["organized_response"]["sections"][0]["source_ids"] = ["SRC-2"]
        result["organized_response"]["sections"][0]["claim_strength"] = "derived"
        result["organized_response"]["source_annotations"] = [
            {
                "source_id": "SRC-2",
                "weight": 0.7,
                "reason": "platform blog",
                "status": "annotated",
            }
        ]
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_annotation_must_match_merged_ledger(self) -> None:
        result = _committed_result()
        result["source_ledger"][0]["model_weight"] = 1.1
        result["source_ledger"][0]["model_weight_reason"] = "official"
        result["source_ledger"][0]["annotation_status"] = "adopted"
        result["organized_response"]["source_annotations"] = [
            {
                "source_id": "SRC-1",
                "weight": 1.0,
                "reason": "official",
                "status": "adopted",
            }
        ]
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("does not match merged ledger fields", " | ".join(errors))

    def test_merged_model_fields_require_annotation(self) -> None:
        result = _committed_result()
        result["source_ledger"][0]["model_weight"] = 1.0
        result["source_ledger"][0]["model_weight_reason"] = "official"
        result["source_ledger"][0]["annotation_status"] = "adopted"
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("without a source_annotation", " | ".join(errors))

    def test_duplicate_source_annotation_rejected(self) -> None:
        result = _committed_result()
        result["source_ledger"][0]["model_weight"] = 1.0
        result["source_ledger"][0]["model_weight_reason"] = "official docs"
        result["source_ledger"][0]["annotation_status"] = "adopted"
        annotation = {
            "source_id": "SRC-1",
            "weight": 1.0,
            "reason": "official docs",
            "status": "adopted",
        }
        duplicate = {
            "source_id": "SRC-1",
            "weight": 0.7,
            "reason": "second annotation for the same source",
            "status": "annotated",
        }
        result["organized_response"]["source_annotations"] = [
            dict(annotation),
            duplicate,
        ]
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("duplicate source_annotation", " | ".join(errors))

    # ── FUS-RETRIEVAL-MECH B-1 + step 3: web_search candidate pool ──

    def test_search_candidate_pool_validates_when_mirrored(self) -> None:
        result = _committed_result()
        candidates = ["https://a.example", "https://b.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        result["prefilter_log"] = []
        result["source_counts"] = {
            "total": 3,
            "full_text_observed": 1,
            "partial_text_observed": 2,
            "metadata_only": 0,
            "unavailable": 0,
        }
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_search_candidate_pool_requires_raw_ref_mirror(self) -> None:
        result = _committed_result()
        candidates = ["https://a.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        result["prefilter_log"] = []
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("not mirrored", " | ".join(errors))

    def test_candidate_pool_only_on_search_result_entries(self) -> None:
        result = _committed_result()
        result["source_ledger"][0]["candidate_urls"] = ["https://a.example"]
        result["raw_source_refs"][0]["candidate_urls"] = ["https://a.example"]
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("only web_search_result", " | ".join(errors))

    def test_candidate_pool_duplicates_rejected(self) -> None:
        result = _committed_result()
        candidates = ["https://a.example", "https://a.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        result["prefilter_log"] = [
            {
                "source_id": "SRC-3",
                "url": "https://a.example",
                "canonical_url": "https://a.example",
                "reason": "duplicate_canonical",
                "action": "removed",
                "filtered_at": "2026-08-09T00:00:00Z",
            }
        ]
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("contains duplicates", " | ".join(errors))

    def test_candidate_pool_ref_without_ledger_candidate_urls_rejected(self) -> None:
        result = _committed_result()
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
            }
        )
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": ["https://a.example"],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("does not match the ledger", " | ".join(errors))

    def test_candidate_pool_ref_without_ledger_entry_rejected(self) -> None:
        result = _committed_result()
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-GHOST",
                "source_title": "ghost ref",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": ["https://a.example"],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("does not match the ledger", " | ".join(errors))

    # ── FUS-RETRIEVAL-MECH step 3 (2026-08-14): prefiltered pool ──

    def test_candidate_urls_requires_candidate_pool(self) -> None:
        result = _committed_result()
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": ["https://a.example"],
            }
        )
        result["prefilter_log"] = []
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": ["https://a.example"],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("without candidate_pool", " | ".join(errors))

    def test_candidate_pool_requires_candidate_urls(self) -> None:
        result = _committed_result()
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_pool": [_pool_entry("https://a.example")],
            }
        )
        result["prefilter_log"] = []
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("without candidate_urls", " | ".join(errors))

    def test_candidate_pool_url_mismatch_rejected(self) -> None:
        result = _committed_result()
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": ["https://a.example"],
                "candidate_pool": [_pool_entry("https://b.example")],
            }
        )
        result["prefilter_log"] = []
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": ["https://a.example"],
                "candidate_pool": [_pool_entry("https://b.example")],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("does not match candidate_urls", " | ".join(errors))

    def test_candidate_pool_requires_prefilter_log(self) -> None:
        result = _committed_result()
        candidates = ["https://a.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("prefilter_log missing", " | ".join(errors))

    def test_prefilter_log_removed_url_still_retained_rejected(self) -> None:
        result = _committed_result()
        candidates = ["https://a.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        result["prefilter_log"] = [
            {
                "source_id": "SRC-3",
                "url": "https://a.example",
                "canonical_url": "https://a.example",
                "reason": "bad_url",
                "action": "removed",
                "filtered_at": "2026-08-09T00:00:00Z",
            }
        ]
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("still retained", " | ".join(errors))

    def test_prefilter_log_duplicate_removal_rejected(self) -> None:
        result = _committed_result()
        candidates = ["https://a.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        removal = {
            "source_id": "SRC-3",
            "url": "https://bad.example",
            "reason": "bad_url",
            "action": "removed",
            "filtered_at": "2026-08-09T00:00:00Z",
        }
        result["prefilter_log"] = [dict(removal), dict(removal)]
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("duplicate prefilter_log", " | ".join(errors))

    def test_prefilter_log_requires_search_entry_source(self) -> None:
        result = _committed_result()
        candidates = ["https://a.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        result["prefilter_log"] = [
            {
                "source_id": "SRC-1",
                "url": "https://bad.example",
                "reason": "bad_url",
                "action": "removed",
                "filtered_at": "2026-08-09T00:00:00Z",
            }
        ]
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("does not reference a web_search_result", " | ".join(errors))

    def test_prefilter_log_duplicate_reason_may_reference_retained_url(
        self,
    ) -> None:
        result = _committed_result()
        candidates = ["https://a.example"]
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        result["prefilter_log"] = [
            {
                "source_id": "SRC-3",
                "url": "https://a.example",
                "canonical_url": "https://a.example",
                "reason": "duplicate_canonical",
                "action": "removed",
                "filtered_at": "2026-08-09T00:00:00Z",
            }
        ]
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": candidates,
                "candidate_pool": [_pool_entry(url) for url in candidates],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_fully_purified_pool_may_be_empty_with_removal_log(self) -> None:
        result = _committed_result()
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": [],
                "candidate_pool": [],
            }
        )
        result["prefilter_log"] = [
            {
                "source_id": "SRC-3",
                "url": "javascript:alert(1)",
                "reason": "bad_url",
                "action": "removed",
                "filtered_at": "2026-08-09T00:00:00Z",
            }
        ]
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": [],
                "candidate_pool": [],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_empty_pool_without_removal_log_rejected(self) -> None:
        result = _committed_result()
        result["source_ledger"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "source_type": "web_search_result",
                "visibility": "partial_text_observed",
                "accessed_at": "2026-08-09T00:00:00Z",
                "observed_scope": "search snippet",
                "missing_scope": "full page",
                "relevance": "direct",
                "used_in_sections": [],
                "content_sha256": _ZERO,
                "highest_allowed_claim": "derived",
                "candidate_urls": [],
                "candidate_pool": [],
            }
        )
        result["prefilter_log"] = []
        result["source_counts"]["total"] = 3
        result["source_counts"]["partial_text_observed"] = 2
        result["raw_source_refs"].append(
            {
                "source_id": "SRC-3",
                "source_title": "search snippet",
                "source_url_or_ref": "q",
                "visibility": "partial_text_observed",
                "content_sha256": _ZERO,
                "candidate_urls": [],
                "candidate_pool": [],
            }
        )
        journal = _v02_journal(
            [_mk_v02_event("retrieval_result_committed", result, 0, None)]
        )
        errors = validate_journal_text(journal)
        self.assertIn("empty with no prefilter_log removal", " | ".join(errors))


class WebFetchCandidateCountTests(unittest.TestCase):
    """FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): candidate count
    fields on tool_completed events (web_fetch family + browser_read) —
    pairing, range, family and cap-boundary shape (cross-layer verifier)."""

    def _event(self, payload: dict) -> dict:
        return _mk_v02_event("tool_completed", payload, 0, None)

    def test_valid_success_carries_count_and_cap(self) -> None:
        event = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-1",
                "exit_code": 0,
                "candidate_count": 3,
                "candidate_cap": 8,
            }
        )
        self.assertEqual(validate_journal_text(_v02_journal([event])), [])

    def test_valid_cap_exceeded_refusal_at_boundary(self) -> None:
        event = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-9",
                "target": "external_retrieval",
                "status": "error",
                "error": "web_fetch_candidate_cap_exceeded",
                "candidate_count": 8,
                "candidate_cap": 8,
            }
        )
        self.assertEqual(validate_journal_text(_v02_journal([event])), [])

    def test_count_without_cap_rejected(self) -> None:
        event = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-1",
                "exit_code": 0,
                "candidate_count": 1,
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("travel together", " | ".join(errors))

    def test_fields_only_on_candidate_counted_family(self) -> None:
        event = self._event(
            {
                "tool": "web_search",
                "call_id": "call-1",
                "exit_code": 0,
                "candidate_count": 1,
                "candidate_cap": 8,
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("web_fetch/browser_read family only", " | ".join(errors))

    def test_count_over_cap_rejected(self) -> None:
        event = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-1",
                "exit_code": 0,
                "candidate_count": 9,
                "candidate_cap": 8,
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("out of range", " | ".join(errors))

    def test_non_error_web_fetch_must_carry_fields(self) -> None:
        event = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-1",
                "exit_code": 0,
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("must carry candidate_count/candidate_cap", " | ".join(errors))

    def test_dispatch_wrapper_without_count_fields_valid(self) -> None:
        # 2026-08-14 review P1: the subagent DISPATCH wrapper completion
        # (target=external_retrieval, exit_code, no count fields) is the
        # parent call being answered — not a fetch. It must NOT be flagged.
        wrapper = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-1",
                "target": "external_retrieval",
                "exit_code": 0,
            }
        )
        lane = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-2",
                "exit_code": 0,
                "candidate_count": 1,
                "candidate_cap": 8,
            }
        )
        self.assertEqual(validate_journal_text(_v02_journal([wrapper, lane])), [])

    def test_cap_exceeded_without_count_fields_rejected(self) -> None:
        event = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-9",
                "target": "external_retrieval",
                "status": "error",
                "error": "web_fetch_candidate_cap_exceeded",
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("cap-exceeded refusal must carry", " | ".join(errors))

    def test_cap_exceeded_refusal_count_must_equal_cap(self) -> None:
        event = self._event(
            {
                "tool": "web_fetch",
                "call_id": "call-9",
                "target": "external_retrieval",
                "status": "error",
                "error": "web_fetch_candidate_cap_exceeded",
                "candidate_count": 7,
                "candidate_cap": 8,
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("must equal candidate_cap", " | ".join(errors))

    def test_valid_browser_read_success_carries_count_and_cap(self) -> None:
        event = self._event(
            {
                "tool": "browser_read",
                "call_id": "call-b1",
                "exit_code": 0,
                "candidate_count": 1,
                "candidate_cap": 8,
            }
        )
        self.assertEqual(validate_journal_text(_v02_journal([event])), [])

    def test_browser_read_non_error_lane_must_carry_fields(self) -> None:
        event = self._event(
            {
                "tool": "browser_read",
                "call_id": "call-b1",
                "exit_code": 0,
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("must carry candidate_count/candidate_cap", " | ".join(errors))

    def test_valid_browser_read_cap_exceeded_refusal_at_boundary(self) -> None:
        event = self._event(
            {
                "tool": "browser_read",
                "call_id": "call-b9",
                "target": "external_retrieval",
                "status": "error",
                "error": "browser_read_candidate_cap_exceeded",
                "candidate_count": 8,
                "candidate_cap": 8,
            }
        )
        self.assertEqual(validate_journal_text(_v02_journal([event])), [])

    def test_browser_read_cap_exceeded_without_count_fields_rejected(self) -> None:
        event = self._event(
            {
                "tool": "browser_read",
                "call_id": "call-b9",
                "target": "external_retrieval",
                "status": "error",
                "error": "browser_read_candidate_cap_exceeded",
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("cap-exceeded refusal must carry", " | ".join(errors))

    def test_browser_read_cap_exceeded_count_must_equal_cap(self) -> None:
        event = self._event(
            {
                "tool": "browser_read",
                "call_id": "call-b9",
                "target": "external_retrieval",
                "status": "error",
                "error": "browser_read_candidate_cap_exceeded",
                "candidate_count": 7,
                "candidate_cap": 8,
            }
        )
        errors = validate_journal_text(_v02_journal([event]))
        self.assertIn("must equal candidate_cap", " | ".join(errors))


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


def _context_compressed(**overrides: object) -> dict:
    payload: dict[str, object] = {
        "trigger_tokens": 165000,
        "target_tokens": 12000,
        "rounds_since_last_compaction": 3,
        "rounds_dropped": 12,
        "messages_dropped": 40,
        "messages_kept": 9,
        "estimated_tokens_after": 11000,
        "mode": "template_summary",
        "reason": "rhythm",
        "summary_id": "compaction-RUN-CONF-0001-0001",
        "summary_digest": _ZERO,
        "summary_path": ".gsa/compaction/compaction-RUN-CONF-0001-0001.md",
        "summary_incomplete": False,
        "retained_rounds": 2,
        "guard_failed": False,
        "archive_write_failed": False,
    }
    payload.update(overrides)
    return _mk_v02_event("context_compressed", payload, 0, None)


class ContextCompressedV02RuleTests(unittest.TestCase):
    """P0-D review fix (2026-08-14, ADR-0010 v1.14): the five-section
    summary's v0.2 cross-rules — session_end reason, the guard-retry force
    report (guard_failed only on rhythm/fallback) and the explicit
    archive-write failure (only on a complete summary)."""

    def test_complete_rhythm_with_new_flags_validates(self) -> None:
        journal = _v02_journal([_context_compressed()])
        self.assertEqual(validate_journal_text(journal), [])

    def test_session_end_validates(self) -> None:
        journal = _v02_journal(
            [_context_compressed(reason="session_end", trigger_tokens=155000)]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_guard_failed_fallback_validates(self) -> None:
        journal = _v02_journal(
            [
                _context_compressed(
                    reason="fallback",
                    trigger_tokens=210000,
                    guard_failed=True,
                )
            ]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_guard_failed_never_rides_session_end(self) -> None:
        journal = _v02_journal(
            [
                _context_compressed(
                    reason="session_end",
                    guard_failed=True,
                )
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("guard_failed may only ride", " | ".join(errors))

    def test_archive_write_failed_on_complete_summary_validates(self) -> None:
        journal = _v02_journal(
            [_context_compressed(archive_write_failed=True)]
        )
        self.assertEqual(validate_journal_text(journal), [])

    def test_archive_write_failed_never_rides_incomplete_summary(self) -> None:
        journal = _v02_journal(
            [
                _context_compressed(
                    summary_incomplete=True,
                    summary_id=None,
                    summary_digest=None,
                    summary_path=None,
                    archive_write_failed=True,
                )
            ]
        )
        errors = validate_journal_text(journal)
        self.assertIn("archive_write_failed may only ride", " | ".join(errors))

    def test_unknown_reason_rejected(self) -> None:
        journal = _v02_journal([_context_compressed(reason="whole_round_drop")])
        errors = validate_journal_text(journal)
        self.assertIn("not one of", " | ".join(errors))


class CheckpointResponseCrossCheckTests(unittest.TestCase):
    """ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16): the
    `_verify_v02_checkpoint_responses` fire↔response cross-check."""

    def test_accepted_attempt_1_is_clean(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=1, outcome="accepted"),
        ]
        self.assertEqual(_verify_v02_checkpoint_responses(events), [])

    def test_response_without_preceding_fire_is_rejected(self) -> None:
        events = [_checkpoint_response(attempt=1, outcome="accepted")]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("no preceding" in e for e in errors), errors)

    def test_response_before_fire_is_rejected(self) -> None:
        events = [
            _checkpoint_response(attempt=1, outcome="accepted"),
            _CHECKPOINT_FIRE,
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("does not precede" in e for e in errors), errors)

    def test_family_and_role_must_match_fire(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(
                attempt=1,
                outcome="accepted",
                inquiry_kind="diagnostic_coverage_checkpoint",
            ),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("inquiry_kind" in e for e in errors), errors)
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(
                attempt=1,
                outcome="accepted",
                agent_role="internal_retrieval",
            ),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("agent_role" in e for e in errors), errors)

    def test_refill_requires_followup_and_cannot_be_last(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=1, outcome="refill_requested"),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("without a following attempt" in e for e in errors), errors)
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=1, outcome="refill_requested"),
            _checkpoint_response(attempt=2, outcome="accepted"),
        ]
        self.assertEqual(_verify_v02_checkpoint_responses(events), [])

    def test_degraded_requires_reason_and_attempt_2(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=1, outcome="refill_requested"),
            _checkpoint_response(attempt=2, outcome="degraded", degrade_reason=None),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("degrade_reason" in e for e in errors), errors)
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(
                attempt=1,
                outcome="degraded",
                degrade_reason="validation_failed_after_refill",
            ),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("degrade only after" in e for e in errors), errors)

    def test_accepted_cannot_have_later_attempt(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=1, outcome="accepted"),
            _checkpoint_response(attempt=2, outcome="accepted"),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("later" in e and "accepted" in e for e in errors), errors)

    def test_attempt_out_of_sequence_rejected(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=2, outcome="accepted"),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(any("out of sequence" in e for e in errors), errors)

    def test_dc_legacy_fire_without_agent_role_is_clean(self) -> None:
        # 2026-08-15 复核（P1）：DC fire 在 agent_role 字段引入前不携带该
        # 字段；响应恒为 main，验证器必须容忍旧 fire。
        events = [
            _CHECKPOINT_FIRE_DC,
            _v02_event(
                "checkpoint_response",
                {
                    "checkpoint_id": "DIAG-COV-RUN-1-0001",
                    "inquiry_family": "neutral",
                    "inquiry_kind": "diagnostic_coverage_checkpoint",
                    "agent_role": "main",
                    "attempt": 1,
                    "outcome": "accepted",
                    "response": {
                        "task_position": "修复缓存回归",
                        "progress_evidence": [],
                        "blockers": [],
                        "missing_evidence": [],
                        "next_action": "continue",
                        "changed_direction": False,
                    },
                    "validation": {
                        "valid": True,
                        "errors": [],
                        "ignored_fields": [],
                    },
                    "cross_check": {
                        "evidence_identity_found": [],
                        "evidence_identity_missing": [],
                        "gather_evidence_missing_surface_provided": True,
                    },
                    "degrade_reason": None,
                },
            ),
        ]
        self.assertEqual(_verify_v02_checkpoint_responses(events), [])

    def test_dc_fire_with_agent_role_matches_and_mismatch_detected(self) -> None:
        fire = dict(_CHECKPOINT_FIRE_DC)
        fire["payload"] = dict(fire["payload"], agent_role="main")
        response = _v02_event(
            "checkpoint_response",
            {
                "checkpoint_id": "DIAG-COV-RUN-1-0001",
                "inquiry_family": "neutral",
                "inquiry_kind": "diagnostic_coverage_checkpoint",
                "agent_role": "main",
                "attempt": 1,
                "outcome": "accepted",
                "response": {
                    "task_position": "t",
                    "progress_evidence": [],
                    "blockers": [],
                    "missing_evidence": [],
                    "next_action": "continue",
                    "changed_direction": False,
                },
                "validation": {"valid": True, "errors": [], "ignored_fields": []},
                "cross_check": {
                    "evidence_identity_found": [],
                    "evidence_identity_missing": [],
                    "gather_evidence_missing_surface_provided": True,
                },
                "degrade_reason": None,
            },
        )
        self.assertEqual(_verify_v02_checkpoint_responses([fire, response]), [])

        bad = dict(response)
        bad["payload"] = dict(response["payload"], agent_role="internal_retrieval")
        errors = _verify_v02_checkpoint_responses([fire, bad])
        self.assertTrue(any("agent_role" in e for e in errors), errors)

    def test_accepted_requires_valid_validation_and_non_null_response(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(
                attempt=1,
                outcome="accepted",
                validation={
                    "valid": False,
                    "errors": ["x"],
                    "ignored_fields": [],
                },
            ),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(
            any("accepted but validation.valid" in e for e in errors),
            errors,
        )

        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=1, outcome="accepted", response=None),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(
            any("accepted but response is null" in e for e in errors),
            errors,
        )

    def test_refill_and_degraded_require_invalid_validation(self) -> None:
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(attempt=1, outcome="refill_requested"),
            _checkpoint_response(
                attempt=2,
                outcome="degraded",
                degrade_reason="validation_failed_after_refill",
                validation={
                    "valid": True,
                    "errors": [],
                    "ignored_fields": [],
                },
            ),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(
            any("degraded but validation.valid" in e for e in errors),
            errors,
        )

        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(
                attempt=1,
                outcome="refill_requested",
                validation={
                    "valid": True,
                    "errors": [],
                    "ignored_fields": [],
                },
            ),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(
            any("refill_requested but validation.valid" in e for e in errors),
            errors,
        )

    def test_gather_evidence_requires_missing_surface_and_flag(self) -> None:
        base_response = {
            "task_position": "t",
            "progress_evidence": [],
            "blockers": [],
            "missing_evidence": ["测试日志"],
            "next_action": "gather_evidence",
            "changed_direction": False,
        }
        clean = _checkpoint_response(
            attempt=1,
            outcome="accepted",
            response=base_response,
        )
        self.assertEqual(
            _verify_v02_checkpoint_responses([_CHECKPOINT_FIRE, clean]),
            [],
        )

        missing = dict(base_response, missing_evidence=[])
        events = [
            _CHECKPOINT_FIRE,
            _checkpoint_response(
                attempt=1,
                outcome="accepted",
                response=missing,
            ),
        ]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(
            any(
                "requires non-empty response.missing_evidence" in e
                for e in errors
            ),
            errors,
        )

        flag_off = _checkpoint_response(
            attempt=1,
            outcome="accepted",
            response=base_response,
            cross_check={
                "evidence_identity_found": [],
                "evidence_identity_missing": [],
                "gather_evidence_missing_surface_provided": False,
            },
        )
        events = [_CHECKPOINT_FIRE, flag_off]
        errors = _verify_v02_checkpoint_responses(events)
        self.assertTrue(
            any(
                "gather_evidence_missing_surface_provided=true" in e
                for e in errors
            ),
            errors,
        )


_H1 = "1" * 64
_H2 = "2" * 64
_H3 = "3" * 64
_S1 = "a" * 64
_S2 = "d" * 64
_T1 = "b" * 64
_T2 = "e" * 64
_C1 = "c" * 64
_C2 = "f" * 64


def _header_event(
    *,
    role: str = "main",
    reason: str = "initial",
    header: str = _H1,
    previous: str | None = None,
    tools: list[str] | None = None,
    system: str | None = None,
    tools_sha: str | None = None,
    config: str | None = None,
    change_kind: str | None = None,
    omit_change_kind: bool = False,
) -> dict:
    tools = tools if tools is not None else ["read_file"]
    if reason == "change" and change_kind is None and not omit_change_kind:
        change_kind = "system"
    if reason == "change":
        if change_kind == "system" and system is None:
            system = _S2
        if change_kind == "tools" and tools_sha is None:
            tools_sha = _T2
        if change_kind == "config" and config is None:
            config = _C2
        if change_kind == "multiple":
            system = system or _S2
            tools_sha = tools_sha or _T2
            config = config or _C2
    payload = {
        "reason": reason,
        "header_sha256": header,
        "system_sha256": system or _S1,
        "tools_sha256": tools_sha or _T1,
        "config_sha256": config or _C1,
        "agent_role": role,
        "tools": tools,
        "tool_count": len(tools),
    }
    if previous is not None:
        payload["previous_header_sha256"] = previous
    if change_kind is not None:
        payload["change_kind"] = change_kind
    return _v02_event("request_header_change", payload)


class RequestHeaderChangeTests(unittest.TestCase):
    """ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the
    `_verify_v02_request_header` lane-partitioned initial/change chains.
    Each loop invocation restarts a lane's chain with `initial` (subagent
    activation / main run), so a lane may hold multiple chains."""

    def test_initial_then_change_is_clean(self) -> None:
        events = [
            _header_event(header=_H1),
            _header_event(reason="change", header=_H2, previous=_H1),
        ]
        self.assertEqual(_verify_v02_request_header(events), [])

    def test_lanes_keep_independent_initial_chains(self) -> None:
        events = [
            _header_event(role="main", header=_H1),
            _header_event(
                role="external_retrieval",
                header=_H2,
                tools=["web_search", "web_fetch"],
            ),
            _header_event(
                role="external_retrieval",
                reason="change",
                header=_H3,
                previous=_H2,
                tools=["web_search"],
            ),
        ]
        self.assertEqual(_verify_v02_request_header(events), [])

    def test_second_initial_starts_new_lane_chain(self) -> None:
        # Review fix (2026-08-15): a later `initial` for the same lane is
        # a NEW loop invocation (subagent multi-activation / multi-run),
        # not an error — it resets the lane's last-observed header.
        events = [
            _header_event(header=_H1),
            _header_event(header=_H2),
            _header_event(reason="change", header=_H3, previous=_H2),
        ]
        self.assertEqual(_verify_v02_request_header(events), [])

    def test_change_without_initial_rejected(self) -> None:
        events = [_header_event(reason="change", header=_H2, previous=_H1)]
        errors = _verify_v02_request_header(events)
        self.assertTrue(any("without a prior initial" in e for e in errors), errors)

    def test_initial_must_not_carry_previous_or_change_kind(self) -> None:
        events = [_header_event(header=_H1, previous=_H2)]
        errors = _verify_v02_request_header(events)
        self.assertTrue(
            any("must not carry previous_header_sha256" in e for e in errors),
            errors,
        )
        events = [_header_event(header=_H1, change_kind="system")]
        errors = _verify_v02_request_header(events)
        self.assertTrue(any("must not carry change_kind" in e for e in errors), errors)

    def test_change_previous_must_match_and_differ(self) -> None:
        events = [
            _header_event(header=_H1),
            _header_event(reason="change", header=_H2, previous=_H3),
        ]
        errors = _verify_v02_request_header(events)
        self.assertTrue(any("!= last header" in e for e in errors), errors)

        events = [
            _header_event(header=_H1),
            _header_event(reason="change", header=_H1, previous=_H1),
        ]
        errors = _verify_v02_request_header(events)
        self.assertTrue(any("must differ" in e for e in errors), errors)

    def test_change_kind_required_and_must_match_digests(self) -> None:
        # A change without change_kind is rejected.
        events = [
            _header_event(header=_H1),
            _header_event(
                reason="change",
                header=_H2,
                previous=_H1,
                omit_change_kind=True,
            ),
        ]
        errors = _verify_v02_request_header(events)
        self.assertTrue(any("must carry change_kind" in e for e in errors), errors)

        # change_kind="tools" with only the tools digest changed is clean.
        events = [
            _header_event(header=_H1),
            _header_event(
                reason="change",
                header=_H2,
                previous=_H1,
                tools_sha=_T2,
                change_kind="tools",
            ),
        ]
        self.assertEqual(_verify_v02_request_header(events), [])

        # change_kind contradicting the actual digest diff is rejected.
        events = [
            _header_event(header=_H1),
            _header_event(
                reason="change",
                header=_H2,
                previous=_H1,
                tools_sha=_T2,
                change_kind="system",
            ),
        ]
        errors = _verify_v02_request_header(events)
        self.assertTrue(
            any("!= actual changed components" in e for e in errors), errors
        )

        # Two changed components require "multiple".
        events = [
            _header_event(header=_H1),
            _header_event(
                reason="change",
                header=_H2,
                previous=_H1,
                system=_S2,
                tools_sha=_T2,
                change_kind="multiple",
            ),
        ]
        self.assertEqual(_verify_v02_request_header(events), [])

    def test_tool_count_mismatch_rejected(self) -> None:
        event = _header_event()
        event["payload"] = dict(event["payload"], tool_count=99)
        errors = _verify_v02_request_header([event])
        self.assertTrue(any("tool_count" in e for e in errors), errors)


class InjectBudgetCrossCheckTests(unittest.TestCase):
    """ORZ-CACHE-CONTEXT-COST (2026-08-15 review fix, ADR-0010 §3.6):
    `_verify_v02_inject_budget` — refusal error code ⇄ inject field
    pairing and ranges."""

    def _completed(self, **overrides: object) -> dict:
        payload: dict[str, object] = {
            "tool": "read_file",
            "call_id": "call-1",
            "exit_code": 1,
            "status": "error",
            "error": "round_inject_budget_exceeded",
            "inject_tokens_used": 50_000,
            "inject_tokens_budget": 50_000,
        }
        payload.update(overrides)
        return _v02_event("tool_completed", payload)

    def test_refusal_must_carry_both_fields(self) -> None:
        for key in ("inject_tokens_used", "inject_tokens_budget"):
            event = self._completed()
            event["payload"] = {
                k: v for k, v in event["payload"].items() if k != key
            }
            errors = _verify_v02_inject_budget([event])
            self.assertTrue(
                any("must carry inject_tokens_used and inject_tokens_budget" in e for e in errors),
                errors,
            )

    def test_fields_only_legal_with_refusal_code(self) -> None:
        events = [self._completed(error="permission_denied")]
        errors = _verify_v02_inject_budget(events)
        self.assertTrue(
            any("only legal with error=round_inject_budget_exceeded" in e for e in errors),
            errors,
        )

    def test_fields_must_travel_together(self) -> None:
        event = self._completed(error="permission_denied")
        event["payload"] = {
            k: v for k, v in event["payload"].items() if k != "inject_tokens_budget"
        }
        errors = _verify_v02_inject_budget([event])
        self.assertTrue(any("travel together" in e for e in errors), errors)

    def test_range_checks(self) -> None:
        events = [self._completed(inject_tokens_used=-1)]
        errors = _verify_v02_inject_budget(events)
        self.assertTrue(any("out of range" in e for e in errors), errors)
        events = [self._completed(inject_tokens_budget=0)]
        errors = _verify_v02_inject_budget(events)
        self.assertTrue(any("out of range" in e for e in errors), errors)

    def test_clean_refusal_passes(self) -> None:
        self.assertEqual(_verify_v02_inject_budget([self._completed()]), [])


class PolicyDenialCrossCheckTests(unittest.TestCase):
    """P0-C S3 前置 (2026-08-15, P1-2 定案): `_verify_v02_policy_denial` —
    structured policy denial shape (source/code/reason), non-zero exit_code,
    and tool-family routing per source."""

    def _completed(self, **overrides: object) -> dict:
        payload: dict[str, object] = {
            "tool": "project_doc_index",
            "call_id": "call-1",
            "exit_code": 1,
            "status": "error",
            "error": "retrieval_mode_off",
            "policy_denial": {
                "source": "retrieval_mode",
                "code": "retrieval_mode_off",
                "reason": "retrieval mode is off",
            },
        }
        payload.update(overrides)
        return _v02_event("tool_completed", payload)

    def test_clean_retrieval_mode_refusal_passes(self) -> None:
        self.assertEqual(_verify_v02_policy_denial([self._completed()]), [])

    def test_non_policy_completion_passes(self) -> None:
        event = self._completed()
        del event["payload"]["policy_denial"]
        self.assertEqual(_verify_v02_policy_denial([event]), [])

    def test_successful_exit_code_rejected(self) -> None:
        errors = _verify_v02_policy_denial([self._completed(exit_code=0)])
        self.assertTrue(any("non-zero exit_code" in e for e in errors), errors)
        errors = _verify_v02_policy_denial([self._completed(exit_code=None)])
        self.assertTrue(any("non-zero exit_code" in e for e in errors), errors)
        event = self._completed()
        del event["payload"]["exit_code"]
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("non-zero exit_code" in e for e in errors), errors)

    def test_unknown_source_rejected(self) -> None:
        event = self._completed()
        event["payload"]["policy_denial"] = dict(
            event["payload"]["policy_denial"], source="policy_engine"
        )
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("source" in e for e in errors), errors)

    def test_empty_code_rejected(self) -> None:
        event = self._completed()
        event["payload"]["policy_denial"] = dict(
            event["payload"]["policy_denial"], code=""
        )
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("non-empty string" in e for e in errors), errors)

    def test_acaf_requires_ticketed_tool(self) -> None:
        event = self._completed(
            tool="read_file",
            error="control_ticket_rejected:missing_goal_context",
        )
        event["payload"]["policy_denial"] = {
            "source": "acaf",
            "code": "control_ticket_rejected:missing_goal_context",
            "reason": "goal digest not pinned",
        }
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("non-ticketed tool" in e for e in errors), errors)
        for tool in (
            "search_replace",
            "run_tests",
            "run_terminal_cmd",
            "web_fetch",
            "browser_read",
        ):
            event["payload"]["tool"] = tool
            self.assertEqual(
                _verify_v02_policy_denial([event]),
                [],
                f"source=acaf on ticketed tool {tool} must pass",
            )

    def test_policy_denial_requires_error_status(self) -> None:
        event = self._completed()
        del event["payload"]["status"]
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("status=error" in e for e in errors), errors)
        event["payload"]["status"] = "error"
        self.assertEqual(_verify_v02_policy_denial([event]), [])

    def test_retrieval_mode_requires_retrieval_tool(self) -> None:
        event = self._completed(tool="read_file")
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("non-retrieval tool" in e for e in errors), errors)

    def test_permission_requires_work_tool(self) -> None:
        event = self._completed(tool="not_a_tool")
        event["payload"]["policy_denial"] = {
            "source": "permission",
            "code": "permission_deny",
            "reason": "denied",
        }
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("non-permission-gated tool" in e for e in errors), errors)
        event["payload"]["tool"] = "read_file"
        self.assertEqual(_verify_v02_policy_denial([event]), [])
        # P0-C S3 前置审查修复 (F7): host-routed retrieval tools are
        # permission-gated on the main lane and must validate.
        event["payload"]["tool"] = "project_doc_index"
        self.assertEqual(_verify_v02_policy_denial([event]), [])
        # Web-family tools are not permission-gated today (lane
        # self-execution skips the bridge) — a permission denial there is
        # outside the known refusal path.
        event["payload"]["tool"] = "web_search"
        errors = _verify_v02_policy_denial([event])
        self.assertTrue(any("non-permission-gated tool" in e for e in errors), errors)


class PolicyDenialProducerParityTests(unittest.TestCase):
    """P0-C S3 前置审查修复 (F5): parity lock — the exact ToolCompleted
    shapes the Rust producer emits for policy refusals must pass the Python
    cross-check (exit_code / status=error / error code / tool family)."""

    def _event(self, payload: dict[str, object]) -> dict:
        return _v02_event("tool_completed", payload)

    def test_retrieval_mode_off_producer_shape_passes(self) -> None:
        payload = {
            "tool": "project_doc_index",
            "call_id": "call-pd1",
            "exit_code": 1,
            "target": "internal_retrieval",
            "status": "error",
            "error": "retrieval_mode_off",
            "policy_denial": {
                "source": "retrieval_mode",
                "code": "retrieval_mode_off",
                "reason": "retrieval mode is 'off' for this session (ADR-0010 "
                "§3.7.1); no retrieval tools are available.",
            },
        }
        self.assertEqual(_verify_v02_policy_denial([self._event(payload)]), [])

    def test_acaf_browser_read_producer_shape_passes(self) -> None:
        payload = {
            "tool": "browser_read",
            "call_id": "call-s1",
            "exit_code": 1,
            "status": "error",
            "error": "control_ticket_rejected:signer_unreachable",
            "policy_denial": {
                "source": "acaf",
                "code": "control_ticket_rejected:signer_unreachable",
                "reason": "signer unreachable",
            },
        }
        self.assertEqual(_verify_v02_policy_denial([self._event(payload)]), [])

    def test_host_level_permission_denial_on_retrieval_tool_passes(self) -> None:
        payload = {
            "tool": "project_doc_index",
            "call_id": "call-pd2",
            "exit_code": 1,
            "status": "error",
            "error": "permission_deny",
            "policy_denial": {
                "source": "permission",
                "code": "permission_deny",
                "reason": "tool 'project_doc_index' — 本次调用未获权限门禁放行",
            },
        }
        self.assertEqual(_verify_v02_policy_denial([self._event(payload)]), [])


class ProbeAccuracyCrossCheckTests(unittest.TestCase):
    """ADR-0010 §3.5 条7 (ORZ-CACHE-CONTEXT-COST 2026-08-15): a probe flip
    must be followed by a main-lane request_header_change before the next
    model_output — the mechanical flip↔header「事后核对」."""

    def _probe(self, complete: list[str]) -> dict:
        return _v02_event(
            "tool_availability_check",
            {
                "probe_scope": "main_agent_work_tools",
                "probe_timestamp": "2026-08-15T00:00:00Z",
                "complete": complete,
                "incomplete": [
                    {"tool": "run_tests", "reason": "缺少测试运行器"},
                    {"tool": "ask_user_question", "reason": "无交互式用户会话"},
                ],
                "gate_decision": "pass",
            },
        )

    def test_flip_with_header_change_is_clean(self) -> None:
        events = [
            self._probe(["read_file"]),
            self._probe(["read_file", "grep"]),
            _header_event(reason="change", header=_H2, previous=_H1),
            _v02_event("model_output", {"text": "ok"}),
        ]
        self.assertEqual(_verify_v02_probe_accuracy(events), [])

    def test_flip_without_header_change_before_model_output_rejected(self) -> None:
        events = [
            _header_event(),
            self._probe(["read_file"]),
            self._probe(["read_file", "grep"]),
            _v02_event("model_output", {"text": "ok"}),
        ]
        errors = _verify_v02_probe_accuracy(events)
        self.assertTrue(
            any("was not followed by a request_header_change" in e for e in errors),
            errors,
        )

    def test_pre_feature_journal_without_header_events_is_clean(self) -> None:
        # Compatibility boundary (2026-08-15): journals captured before the
        # request-header feature carry no header events — the cross-check
        # does not retroactively fail them.
        events = [
            self._probe(["read_file"]),
            self._probe(["read_file", "grep"]),
            _v02_event("model_output", {"text": "ok"}),
        ]
        self.assertEqual(_verify_v02_probe_accuracy(events), [])

    def test_subagent_header_does_not_clear_main_flip(self) -> None:
        events = [
            self._probe(["read_file"]),
            self._probe(["read_file", "grep"]),
            _header_event(
                role="external_retrieval",
                reason="change",
                header=_H2,
                previous=_H1,
                tools=["web_search"],
            ),
            _v02_event("model_output", {"text": "ok"}),
        ]
        errors = _verify_v02_probe_accuracy(events)
        self.assertTrue(
            any("was not followed by a request_header_change" in e for e in errors),
            errors,
        )


if __name__ == "__main__":
    unittest.main()
