"""Tests for plan mode and process usage monitor — GAK-PLAN-001.

Covers:
1. PlanArtifact construction, serialisation, and schema validation
2. PlanVerifier structural checks (7 check categories)
3. PlanStateMachine valid transitions and error paths
4. PlanApprovalRecord round-trip
5. ProcessUsageSampler single-sample and start/stop
6. UsageSample formatting (status line, anomaly line, VS Code title)
7. VSCodeTitleUpdater construction
8. PlanningPolicy resolution helper
9. Plan artifact write → load → verify round-trip
10. TUI event bridge integration
"""

from __future__ import annotations

import os
import tempfile
import threading
import time as _time
import unittest
from pathlib import Path

from assurance.plan_mode import (
    ApprovalPolicy,
    PlanApprovalDecision,
    PlanApprovalRecord,
    PlanArtifact,
    PlanSection,
    PlanState,
    PlanStateMachine,
    PlanningPolicy,
    PlanVerificationResult,
    ProcessUsageSampler,
    UsageSample,
    VSCodeTitleUpdater,
    _format_elapsed,
    _format_memory,
    _resolve_planning_policy,
    build_plan_artifact,
    format_vscode_title,
    load_plan_artifact,
    verify_plan_artifact,
    write_plan_artifact,
)
from assurance.errors import AssuranceError
from assurance.contracts import validate_contract


# ── helpers ──────────────────────────────────────────────────────────────────


def _make_section(title: str, content: str = "", evidence: str = "observed") -> PlanSection:
    return PlanSection(
        title=title,
        content_md=content or f"Content for {title}.",
        evidence_status=evidence,
    )


def _make_valid_sections() -> list[PlanSection]:
    return [
        _make_section("前期调查", "Investigation results: workspace is clean."),
        _make_section("具体计划", "Step 1: read files. Step 2: modify config."),
        _make_section("具体设计", "Interface: PlanArtifact dataclass with four sections."),
        _make_section("实施方案", "Implementation: modify assurance/plan_mode.py."),
    ]


def _make_valid_artifact(**kwargs: object) -> PlanArtifact:
    defaults: dict[str, object] = {
        "task_id": "TASK-TEST-001",
        "run_id": "RUN-TEST-001",
        "workspace_root": str(Path(__file__).parent),
        "sections": _make_valid_sections(),
    }
    defaults.update(kwargs)
    return build_plan_artifact(**defaults)  # type: ignore[arg-type]


# ══════════════════════════════════════════════════════════════════════════════
# PlanArtifact construction
# ══════════════════════════════════════════════════════════════════════════════


class PlanArtifactConstructionTests(unittest.TestCase):
    """Test PlanArtifact creation and basic properties."""

    def test_build_with_four_sections(self):
        artifact = _make_valid_artifact()
        self.assertTrue(artifact.plan_id.startswith("PLAN-"))
        self.assertEqual(len(artifact.plan_id), 17)  # PLAN- + 12 hex
        self.assertEqual(artifact.version, 1)
        self.assertEqual(artifact.previous_plan_sha256, "")
        self.assertEqual(len(artifact.sections), 4)

    def test_build_rejects_three_sections(self):
        sections = [
            _make_section("前期调查"),
            _make_section("具体计划"),
            _make_section("具体设计"),
        ]
        with self.assertRaises(AssuranceError) as ctx:
            build_plan_artifact(
                task_id="TASK-001",
                run_id="RUN-001",
                workspace_root="/tmp",
                sections=sections,
            )
        self.assertIn("missing", str(ctx.exception).lower())
        self.assertIn("实施方案", str(ctx.exception))

    def test_build_rejects_duplicate_titles(self):
        sections = [
            _make_section("前期调查"),
            _make_section("具体计划"),
            _make_section("具体设计"),
            _make_section("前期调查"),  # duplicate
        ]
        with self.assertRaises(AssuranceError):
            build_plan_artifact(
                task_id="TASK-001",
                run_id="RUN-001",
                workspace_root="/tmp",
                sections=sections,
            )

    def test_build_with_planning_policy_enum(self):
        artifact = build_plan_artifact(
            task_id="TASK-001",
            run_id="RUN-001",
            workspace_root="/tmp",
            sections=_make_valid_sections(),
            planning_policy=PlanningPolicy.REQUIRED,
        )
        self.assertEqual(artifact.planning_policy, "required")

    def test_build_with_deferred_decisions(self):
        artifact = build_plan_artifact(
            task_id="TASK-001",
            run_id="RUN-001",
            workspace_root="/tmp",
            sections=_make_valid_sections(),
            deferred_decisions=["UI layout details", "VS Code title mechanism"],
        )
        self.assertIn("UI layout details", artifact.deferred_decisions)

    def test_version_two_requires_previous_hash(self):
        artifact = build_plan_artifact(
            task_id="TASK-001",
            run_id="RUN-001",
            workspace_root="/tmp",
            sections=_make_valid_sections(),
            version=2,
            previous_plan_sha256="a" * 64,
        )
        self.assertEqual(artifact.version, 2)
        self.assertEqual(artifact.previous_plan_sha256, "a" * 64)

    def test_sha256_deterministic(self):
        sections = _make_valid_sections()
        a1 = build_plan_artifact(
            task_id="TASK-001", run_id="RUN-001",
            workspace_root="/tmp", sections=sections,
        )
        a2 = build_plan_artifact(
            task_id="TASK-001", run_id="RUN-001",
            workspace_root="/tmp", sections=sections,
        )
        # Different plan_ids but same content should produce matching sha256
        # (plan_id is part of the hash, so they differ — this tests determinism)
        h1 = a1.compute_sha256()
        h2 = a1.compute_sha256()
        self.assertEqual(h1, h2)


# ══════════════════════════════════════════════════════════════════════════════
# PlanArtifact serialisation
# ══════════════════════════════════════════════════════════════════════════════


class PlanArtifactSerialisationTests(unittest.TestCase):
    """Test to_dict, to_markdown, and schema validation."""

    def test_to_dict_has_required_fields(self):
        artifact = _make_valid_artifact()
        d = artifact.to_dict()
        self.assertEqual(d["schema_version"], "0.1.0-draft")
        self.assertEqual(len(d["sections"]), 4)
        self.assertEqual(len(d["plan_sha256"]), 64)
        self.assertTrue(d["plan_id"].startswith("PLAN-"))

    def test_to_dict_passes_schema_validation(self):
        artifact = _make_valid_artifact()
        d = artifact.to_dict()
        # Should not raise
        validate_contract(d, "plan-mode-result-v0.1.schema.json", label="test")

    def test_to_dict_with_approval_passes_schema(self):
        artifact = _make_valid_artifact()
        artifact.approval_decision = "approve"
        artifact.approval_timestamp = "2026-07-29T12:00:00Z"
        artifact.approval_authority = "user"
        artifact.selected_approval_policy = "manual"
        d = artifact.to_dict()
        validate_contract(d, "plan-mode-result-v0.1.schema.json", label="test-approval")

    def test_to_markdown_includes_all_sections(self):
        artifact = _make_valid_artifact()
        md = artifact.to_markdown()
        self.assertIn("# Plan:", md)
        self.assertIn("## 前期调查", md)
        self.assertIn("## 具体计划", md)
        self.assertIn("## 具体设计", md)
        self.assertIn("## 实施方案", md)

    def test_to_markdown_includes_approval_when_present(self):
        artifact = _make_valid_artifact()
        artifact.approval_decision = "approve"
        artifact.approval_authority = "user"
        artifact.selected_approval_policy = "mixed"
        md = artifact.to_markdown()
        self.assertIn("## Approval", md)
        self.assertIn("approve", md)

    def test_write_and_load_round_trip(self):
        artifact = _make_valid_artifact()
        artifact.approval_decision = "approve"
        artifact.approval_timestamp = "2026-07-29T12:00:00Z"
        artifact.approval_authority = "user"
        artifact.selected_approval_policy = "manual"
        with tempfile.TemporaryDirectory() as td:
            store = Path(td) / "plans"
            json_path = write_plan_artifact(artifact, store_root=store)
            self.assertTrue(json_path.is_file())
            md_path = store / artifact.plan_id / "plan-mode-result.md"
            self.assertTrue(md_path.is_file())
            # Load back
            loaded = load_plan_artifact(store / artifact.plan_id)
            self.assertEqual(loaded.plan_id, artifact.plan_id)
            self.assertEqual(loaded.task_id, artifact.task_id)
            self.assertEqual(loaded.approval_decision, "approve")
            self.assertEqual(len(loaded.sections), 4)
            self.assertEqual(loaded.sections[0].title, "前期调查")


# ══════════════════════════════════════════════════════════════════════════════
# PlanVerifier
# ══════════════════════════════════════════════════════════════════════════════


class PlanVerifierTests(unittest.TestCase):
    """Test structural verification of plan artifacts."""

    def test_valid_plan_passes_all_checks(self):
        artifact = _make_valid_artifact()
        result = verify_plan_artifact(artifact)
        self.assertTrue(result.valid, msg="; ".join(result.errors))
        self.assertEqual(len(result.errors), 0)

    def test_missing_section_detected(self):
        sections = [
            _make_section("前期调查"),
            _make_section("具体计划"),
            _make_section("具体设计"),
            PlanSection(title="Wrong Title", content_md="content"),
        ]
        # build_plan_artifact rejects invalid section titles at construction
        with self.assertRaises(AssuranceError) as ctx:
            build_plan_artifact(
                task_id="TASK-001", run_id="RUN-001",
                workspace_root="/tmp", sections=sections,
            )
        self.assertIn("实施方案", str(ctx.exception))

    def test_empty_content_detected(self):
        sections = [
            PlanSection(title="前期调查", content_md="   "),  # whitespace-only
            _make_section("具体计划", "ok"),
            _make_section("具体设计", "ok"),
            _make_section("实施方案", "ok"),
        ]
        artifact = build_plan_artifact(
            task_id="TASK-001", run_id="RUN-001",
            workspace_root="/tmp", sections=sections,
        )
        result = verify_plan_artifact(artifact)
        self.assertFalse(result.valid)
        self.assertTrue(any("empty" in e.lower() for e in result.errors))

    def test_sha256_integrity_fails_on_tamper(self):
        artifact = _make_valid_artifact()
        self.assertTrue(artifact._sha256)  # hash is set
        # Tamper with content without updating _sha256
        artifact.sections[0].content_md = "Tampered content!"
        # The stored _sha256 no longer matches the current content
        fresh = artifact.compute_sha256()
        self.assertNotEqual(fresh, artifact._sha256,
                           msg="Tampered content should produce different hash")
        result = verify_plan_artifact(artifact)
        self.assertFalse(result.valid)
        # Should contain SHA-256 mismatch error
        sha_errs = [e for e in result.errors if "sha" in e.lower()]
        self.assertTrue(sha_errs, msg=f"Errors: {result.errors}")

    def test_version_chain_v1_empty_previous(self):
        artifact = _make_valid_artifact()
        result = verify_plan_artifact(artifact)
        self.assertTrue(result.valid)
        # v1 with non-empty previous should fail
        artifact2 = build_plan_artifact(
            task_id="TASK-001", run_id="RUN-001",
            workspace_root="/tmp", sections=_make_valid_sections(),
            version=1, previous_plan_sha256="a" * 64,
        )
        result2 = verify_plan_artifact(artifact2)
        self.assertFalse(result2.valid)

    def test_version_chain_v2_requires_previous(self):
        artifact = build_plan_artifact(
            task_id="TASK-001", run_id="RUN-001",
            workspace_root="/tmp", sections=_make_valid_sections(),
            version=2, previous_plan_sha256="a" * 64,
        )
        result = verify_plan_artifact(artifact)
        self.assertTrue(result.valid)

        artifact2 = build_plan_artifact(
            task_id="TASK-001", run_id="RUN-001",
            workspace_root="/tmp", sections=_make_valid_sections(),
            version=2, previous_plan_sha256="",
        )
        result2 = verify_plan_artifact(artifact2)
        self.assertFalse(result2.valid)

    def test_require_approval_rejects_unapproved(self):
        artifact = _make_valid_artifact()
        result = verify_plan_artifact(artifact, require_approval=True)
        self.assertFalse(result.valid)
        self.assertTrue(any("approval" in e.lower() for e in result.errors))

    def test_require_approval_accepts_approved(self):
        artifact = _make_valid_artifact()
        artifact.approval_decision = "approve"
        artifact.approval_timestamp = "2026-07-29T12:00:00Z"
        artifact.approval_authority = "user"
        artifact.selected_approval_policy = "manual"
        # Recompute hash after setting approval fields
        artifact._sha256 = artifact.compute_sha256()
        result = verify_plan_artifact(artifact, require_approval=True)
        self.assertTrue(result.valid, msg="; ".join(result.errors))

    def test_result_checks_count(self):
        artifact = _make_valid_artifact()
        result = verify_plan_artifact(artifact)
        # 4 section presence + 1 count + 4 content + 1 version + 1 hash + 1 integrity
        self.assertEqual(len(result.checks), 12)

    def test_extra_section_count_fails(self):
        # Due to duplicate check in build, impossible to have >4 normally
        # but we can verify the count check via a section count mismatch
        sections = [
            _make_section("前期调查"),
            _make_section("具体计划"),
            _make_section("具体设计"),
            PlanSection(title="实施方案", content_md="ok"),
            PlanSection(title="Extra", content_md="bonus"),
        ]
        with self.assertRaises(AssuranceError):
            build_plan_artifact(
                task_id="TASK-001", run_id="RUN-001",
                workspace_root="/tmp", sections=sections,
            )


# ══════════════════════════════════════════════════════════════════════════════
# PlanStateMachine
# ══════════════════════════════════════════════════════════════════════════════


class PlanStateMachineTests(unittest.TestCase):
    """Test planning lifecycle state transitions."""

    def setUp(self):
        self.sm = PlanStateMachine()

    def test_initial_state_is_idle(self):
        self.assertEqual(self.sm.state, PlanState.IDLE)
        self.assertFalse(self.sm.is_planning)

    def test_enter_planning_from_idle(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        self.assertEqual(self.sm.state, PlanState.PLANNING)
        self.assertTrue(self.sm.is_planning)
        self.assertEqual(self.sm.planning_policy, PlanningPolicy.REQUIRED)

    def test_cannot_submit_from_idle(self):
        artifact = _make_valid_artifact()
        with self.assertRaises(AssuranceError):
            self.sm.submit_plan(artifact)

    def test_submit_transitions_to_awaiting(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        artifact = _make_valid_artifact()
        self.sm.submit_plan(artifact)
        self.assertEqual(self.sm.state, PlanState.AWAITING_PLAN_APPROVAL)
        self.assertTrue(self.sm.is_awaiting_approval)

    def test_approve_transitions_to_executing(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        artifact = _make_valid_artifact()
        self.sm.submit_plan(artifact)
        record = self.sm.approve(
            authority="user",
            execution_policy=ApprovalPolicy.MIXED,
        )
        self.assertEqual(self.sm.state, PlanState.EXECUTING)
        self.assertTrue(self.sm.is_executing)
        self.assertEqual(record.decision, PlanApprovalDecision.APPROVE)
        self.assertEqual(record.execution_policy, ApprovalPolicy.MIXED)
        self.assertEqual(self.sm.current_plan.approval_decision, "approve")  # type: ignore[union-attr]

    def test_revise_returns_to_planning(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        artifact = _make_valid_artifact()
        self.sm.submit_plan(artifact)
        record = self.sm.revise(authority="user")
        self.assertEqual(self.sm.state, PlanState.PLANNING)
        self.assertEqual(record.decision, PlanApprovalDecision.REVISE)

    def test_reject_stops_task(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        artifact = _make_valid_artifact()
        self.sm.submit_plan(artifact)
        record = self.sm.reject(authority="user")
        self.assertEqual(self.sm.state, PlanState.REJECTED)
        self.assertEqual(record.decision, PlanApprovalDecision.REJECT)

    def test_complete_from_executing(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        self.sm.submit_plan(_make_valid_artifact())
        self.sm.approve()
        self.sm.complete()
        self.assertEqual(self.sm.state, PlanState.COMPLETED)

    def test_fail_from_any_state(self):
        self.sm.fail(reason="unexpected error")
        self.assertEqual(self.sm.state, PlanState.FAILED)

    def test_cannot_approve_without_submit(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        with self.assertRaises(AssuranceError):
            self.sm.approve()

    def test_cannot_submit_twice(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        self.sm.submit_plan(_make_valid_artifact())
        with self.assertRaises(AssuranceError):
            self.sm.submit_plan(_make_valid_artifact())

    def test_plan_versions_tracked(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        a1 = _make_valid_artifact()
        self.sm.submit_plan(a1)
        self.assertEqual(len(self.sm.plan_versions), 1)

    def test_approval_history_tracked(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        self.sm.submit_plan(_make_valid_artifact())
        self.sm.approve()
        self.assertEqual(len(self.sm.approval_history), 1)
        self.assertEqual(self.sm.approval_history[0]["decision"], "approve")

    def test_to_dict(self):
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        d = self.sm.to_dict()
        self.assertEqual(d["state"], "planning")
        self.assertEqual(d["planning_policy"], "required")

    def test_revise_then_resubmit(self):
        """Full revise → replan → submit → approve cycle."""
        self.sm.enter_planning(policy=PlanningPolicy.REQUIRED)
        self.sm.submit_plan(_make_valid_artifact())
        self.sm.revise()
        # Back in planning
        self.assertTrue(self.sm.is_planning)
        self.sm.submit_plan(_make_valid_artifact())
        self.sm.approve()
        self.assertEqual(self.sm.state, PlanState.EXECUTING)
        self.assertEqual(len(self.sm.approval_history), 2)  # revise + approve


# ══════════════════════════════════════════════════════════════════════════════
# PlanApprovalRecord
# ══════════════════════════════════════════════════════════════════════════════


class PlanApprovalRecordTests(unittest.TestCase):
    """Test approval record creation and serialisation."""

    def test_approve_record(self):
        record = PlanApprovalRecord(
            plan_id="PLAN-ABC123",
            decision=PlanApprovalDecision.APPROVE,
            authority="user",
            execution_policy=ApprovalPolicy.MANUAL,
        )
        d = record.to_dict()
        self.assertEqual(d["plan_id"], "PLAN-ABC123")
        self.assertEqual(d["decision"], "approve")
        self.assertEqual(d["authority"], "user")
        self.assertEqual(d["execution_policy"], "manual")

    def test_reject_record(self):
        record = PlanApprovalRecord(
            plan_id="PLAN-XYZ",
            decision=PlanApprovalDecision.REJECT,
            authority="policy:risk",
        )
        d = record.to_dict()
        self.assertEqual(d["decision"], "reject")


# ══════════════════════════════════════════════════════════════════════════════
# ProcessUsageSampler
# ══════════════════════════════════════════════════════════════════════════════


class ProcessUsageSamplerTests(unittest.TestCase):
    """Test the process usage sampler."""

    def test_sample_once_returns_valid_sample(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid())
        sample = sampler.sample_once()
        self.assertIsInstance(sample, UsageSample)
        self.assertEqual(sample.root_pid, os.getpid())
        self.assertIn(sample.completeness, {"complete", "partial", "unavailable"})
        self.assertGreater(sample.process_count, 0)

    def test_start_stop_collects_samples(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid(), interval=0.1, max_samples=100)
        sampler.start()
        self.assertTrue(sampler.running)
        _time.sleep(0.35)  # ~3 samples
        samples = sampler.stop()
        self.assertFalse(sampler.running)
        self.assertGreater(len(samples), 0)
        for s in samples:
            self.assertEqual(s.root_pid, os.getpid())

    def test_latest_returns_most_recent(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid(), interval=0.1)
        sampler.start()
        _time.sleep(0.25)
        latest = sampler.latest
        self.assertIsNotNone(latest)
        sampler.stop()
        # Post-stop, latest is still available
        self.assertIsNotNone(sampler.latest)

    def test_stop_returns_all_samples(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid(), interval=0.1)
        sampler.start()
        _time.sleep(0.3)
        samples = sampler.stop()
        self.assertGreaterEqual(len(samples), 1)

    def test_to_dict(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid())
        sample = sampler.sample_once()
        d = sample.to_dict()
        self.assertIn("cpu_percent", d)
        self.assertIn("memory_bytes", d)
        self.assertIn("elapsed_seconds", d)
        self.assertIn("process_count", d)
        self.assertIn("completeness", d)

    def test_format_status_line(self):
        sample = UsageSample(
            timestamp="2026-07-29T12:00:00Z",
            root_pid=1234,
            cpu_percent=185.0,
            memory_bytes=2_261_000_000,
            elapsed_seconds=840.0,
            process_count=12,
        )
        line = sample.format_status_line()
        self.assertIn("CPU", line)
        self.assertIn("MEM", line)
        self.assertIn("12p", line)

    def test_format_anomaly_high_cpu(self):
        sample = UsageSample(
            timestamp="2026-07-29T12:00:00Z",
            root_pid=1234,
            cpu_percent=420.0,
            memory_bytes=1_000_000_000,
            elapsed_seconds=120.0,
            process_count=5,
        )
        anomaly = sample.format_anomaly_line()
        self.assertTrue(anomaly.startswith("HIGH CPU"))

    def test_format_anomaly_idle(self):
        sample = UsageSample(
            timestamp="2026-07-29T12:00:00Z",
            root_pid=1234,
            cpu_percent=0.5,
            memory_bytes=1_000_000_000,
            elapsed_seconds=60.0,
            process_count=2,
        )
        anomaly = sample.format_anomaly_line()
        self.assertTrue(anomaly.startswith("IDLE?"))

    def test_format_anomaly_no_anomaly_returns_empty(self):
        sample = UsageSample(
            timestamp="2026-07-29T12:00:00Z",
            root_pid=1234,
            cpu_percent=50.0,
            memory_bytes=500_000_000,
            elapsed_seconds=10.0,
            process_count=3,
        )
        anomaly = sample.format_anomaly_line()
        self.assertEqual(anomaly, "")

    def test_double_start_is_idempotent(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid(), interval=0.1)
        sampler.start()
        thread_count = threading.active_count()
        sampler.start()  # second start should be no-op
        _time.sleep(0.2)
        sampler.stop()

    def test_double_stop_is_safe(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid(), interval=0.1)
        sampler.start()
        _time.sleep(0.2)
        sampler.stop()
        samples2 = sampler.stop()  # second stop safe
        self.assertIsInstance(samples2, list)


# ══════════════════════════════════════════════════════════════════════════════
# Formatting helpers
# ══════════════════════════════════════════════════════════════════════════════


class FormattingTests(unittest.TestCase):
    """Test memory, elapsed time, and VS Code title formatting."""

    def test_format_memory_gb(self):
        self.assertIn("G", _format_memory(3 * 1024**3))

    def test_format_memory_mb(self):
        self.assertIn("M", _format_memory(500 * 1024**2))

    def test_format_memory_kb(self):
        self.assertIn("K", _format_memory(50 * 1024))

    def test_format_memory_bytes(self):
        self.assertIn("B", _format_memory(500))

    def test_format_elapsed_seconds(self):
        self.assertIn("s", _format_elapsed(45))

    def test_format_elapsed_minutes(self):
        result = _format_elapsed(125)
        self.assertIn("m", result)
        self.assertIn("s", result)

    def test_format_elapsed_hours(self):
        result = _format_elapsed(3725)
        self.assertIn("h", result)
        self.assertIn("m", result)

    def test_format_vscode_title_normal(self):
        sample = UsageSample(
            timestamp="",
            root_pid=1,
            cpu_percent=50.0,
            memory_bytes=500_000_000,
            elapsed_seconds=120.0,
            process_count=3,
        )
        title = format_vscode_title(sample)
        self.assertIn("CPU", title)
        self.assertIn("MEM", title)
        self.assertLess(len(title), 60)

    def test_format_vscode_title_anomaly(self):
        sample = UsageSample(
            timestamp="",
            root_pid=1,
            cpu_percent=420.0,
            memory_bytes=1_000_000_000,
            elapsed_seconds=120.0,
            process_count=5,
        )
        title = format_vscode_title(sample)
        self.assertTrue(title.startswith("HIGH CPU"))


# ══════════════════════════════════════════════════════════════════════════════
# VSCodeTitleUpdater
# ══════════════════════════════════════════════════════════════════════════════


class VSCodeTitleUpdaterTests(unittest.TestCase):
    """Test VS Code title updater construction and lifecycle."""

    def test_construction(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid(), interval=0.5)
        updater = VSCodeTitleUpdater(sampler, interval=1.0)
        self.assertIs(updater.sampler, sampler)
        self.assertEqual(updater.interval, 1.0)
        self.assertFalse(updater.running)

    def test_start_stop(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid(), interval=0.5)
        updater = VSCodeTitleUpdater(sampler, interval=0.5)
        sampler.start()
        _time.sleep(0.3)
        updater.start()
        self.assertTrue(updater.running)
        _time.sleep(0.5)
        updater.stop()
        self.assertFalse(updater.running)
        sampler.stop()

    def test_stop_when_not_running_is_safe(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid())
        updater = VSCodeTitleUpdater(sampler)
        updater.stop()  # no-op, no error
        self.assertFalse(updater.running)


# ══════════════════════════════════════════════════════════════════════════════
# Planning policy resolution
# ══════════════════════════════════════════════════════════════════════════════


class PlanningPolicyResolutionTests(unittest.TestCase):
    """Test the planning policy resolution helper."""

    def test_explicit_overrides_heuristic(self):
        result = _resolve_planning_policy(
            explicit=PlanningPolicy.NONE,
            task_complexity="complex",
        )
        self.assertEqual(result, PlanningPolicy.NONE)

    def test_complex_task_suggests_planning(self):
        result = _resolve_planning_policy(
            explicit=None,
            task_complexity="complex",
        )
        self.assertEqual(result, PlanningPolicy.SUGGESTED)

    def test_normal_task_defaults_to_none(self):
        result = _resolve_planning_policy(explicit=None, task_complexity="normal")
        self.assertEqual(result, PlanningPolicy.NONE)

    def test_string_explicit_accepted(self):
        result = _resolve_planning_policy(explicit="required")
        self.assertEqual(result, PlanningPolicy.REQUIRED)


# ══════════════════════════════════════════════════════════════════════════════
# TUI event bridge integration (GAK-PLAN-001 events)
# ══════════════════════════════════════════════════════════════════════════════


class TuiEventIntegrationTests(unittest.TestCase):
    """Test that new plan/usage events map correctly via the bridge."""

    def test_bridge_plan_phase_entered(self):
        from assurance.tui.bridge import build_event_from_jsonl_line
        from assurance.tui.events import PlanPhaseEnteredEvent
        line = {
            "event_type": "plan_phase_entered",
            "timestamp": "2026-07-29T12:00:00Z",
            "payload": {
                "planning_policy": "required",
                "task_id": "TASK-001",
                "run_id": "RUN-001",
            },
        }
        event = build_event_from_jsonl_line(line)
        self.assertIsInstance(event, PlanPhaseEnteredEvent)
        self.assertEqual(event.planning_policy, "required")  # type: ignore[attr-defined]

    def test_bridge_plan_phase_submitted(self):
        from assurance.tui.bridge import build_event_from_jsonl_line
        from assurance.tui.events import PlanPhaseSubmittedEvent
        line = {
            "event_type": "plan_phase_submitted",
            "timestamp": "2026-07-29T12:00:00Z",
            "payload": {
                "plan_id": "PLAN-ABC123",
                "plan_sha256": "a" * 64,
                "section_count": 4,
                "version": 1,
            },
        }
        event = build_event_from_jsonl_line(line)
        self.assertIsInstance(event, PlanPhaseSubmittedEvent)
        self.assertEqual(event.plan_id, "PLAN-ABC123")  # type: ignore[attr-defined]

    def test_bridge_plan_approval_decision(self):
        from assurance.tui.bridge import build_event_from_jsonl_line
        from assurance.tui.events import PlanApprovalDecisionEvent
        line = {
            "event_type": "plan_approval_decision",
            "timestamp": "2026-07-29T12:00:00Z",
            "payload": {
                "plan_id": "PLAN-ABC123",
                "decision": "approve",
                "authority": "user",
                "execution_policy": "mixed",
            },
        }
        event = build_event_from_jsonl_line(line)
        self.assertIsInstance(event, PlanApprovalDecisionEvent)
        self.assertEqual(event.decision, "approve")  # type: ignore[attr-defined]

    def test_bridge_usage_sample(self):
        from assurance.tui.bridge import build_event_from_jsonl_line
        from assurance.tui.events import UsageSampleEvent
        line = {
            "event_type": "usage_sample",
            "timestamp": "2026-07-29T12:00:00Z",
            "payload": {
                "cpu_percent": 185.0,
                "memory_bytes": 2_261_000_000,
                "elapsed_seconds": 840.0,
                "process_count": 12,
                "completeness": "complete",
                "status_line": "CPU 185% MEM 2.1G 14m0s 12p",
                "anomaly_line": "",
            },
        }
        event = build_event_from_jsonl_line(line)
        self.assertIsInstance(event, UsageSampleEvent)
        self.assertEqual(event.cpu_percent, 185.0)  # type: ignore[attr-defined]
        self.assertEqual(event.process_count, 12)  # type: ignore[attr-defined]


# ══════════════════════════════════════════════════════════════════════════════
# Enums
# ══════════════════════════════════════════════════════════════════════════════


class EnumTests(unittest.TestCase):
    """Verify enum values match architecture spec."""

    def test_planning_policy_values(self):
        self.assertEqual(PlanningPolicy.NONE.value, "none")
        self.assertEqual(PlanningPolicy.SUGGESTED.value, "suggested")
        self.assertEqual(PlanningPolicy.REQUIRED.value, "required")

    def test_approval_policy_values(self):
        self.assertEqual(ApprovalPolicy.MANUAL.value, "manual")
        self.assertEqual(ApprovalPolicy.AUTO.value, "auto")
        self.assertEqual(ApprovalPolicy.MIXED.value, "mixed")

    def test_plan_state_values(self):
        self.assertEqual(PlanState.IDLE.value, "idle")
        self.assertEqual(PlanState.EXECUTING.value, "executing")
        self.assertEqual(PlanState.AWAITING_PLAN_APPROVAL.value, "awaiting_plan_approval")

    def test_approval_decision_values(self):
        self.assertEqual(PlanApprovalDecision.APPROVE.value, "approve")
        self.assertEqual(PlanApprovalDecision.REVISE.value, "revise")
        self.assertEqual(PlanApprovalDecision.REJECT.value, "reject")


# ══════════════════════════════════════════════════════════════════════════════
# Edge cases
# ══════════════════════════════════════════════════════════════════════════════


class EdgeCaseTests(unittest.TestCase):
    """Boundary and edge-case behaviour."""

    def test_sampler_with_invalid_pid(self):
        """Sampler should survive with a non-existent PID."""
        sampler = ProcessUsageSampler(root_pid=99999999)  # unlikely to exist
        sample = sampler.sample_once()
        self.assertIn(sample.completeness, {"partial", "unavailable"})

    def test_empty_source_refs(self):
        section = PlanSection(title="前期调查", content_md="test", source_refs=[])
        self.assertEqual(section.source_refs, [])

    def test_artifact_planning_policy_string_accepted(self):
        artifact = build_plan_artifact(
            task_id="TASK-001",
            run_id="RUN-001",
            workspace_root="/tmp",
            sections=_make_valid_sections(),
            planning_policy="suggested",
        )
        self.assertEqual(artifact.planning_policy, "suggested")

    def test_sampler_stop_before_start(self):
        sampler = ProcessUsageSampler(root_pid=os.getpid())
        samples = sampler.stop()
        self.assertEqual(samples, [])

    def test_set_vscode_title(self):
        from assurance.plan_mode import set_vscode_terminal_title
        result = set_vscode_terminal_title("GSA Test Title")
        # Should succeed or gracefully fail
        self.assertIsInstance(result, bool)
