"""Tests for task checklist — GAK-PLAN-001 extension.

Covers:
1. ChecklistStatus enum and display symbols
2. ChecklistItem construction, to_dict/from_dict round-trip
3. ChecklistAnnotation fields
4. TaskChecklist current_index / current_item
5. TaskChecklist to_dict/from_dict round-trip
6. derive_checklist_from_plan — structured markdown, bullet lists, empty
7. step_id validation and synthesis
"""

from __future__ import annotations

import unittest

from assurance.plan_mode import (
    PlanArtifact,
    PlanSection,
    build_plan_artifact,
)
from assurance.task_checklist import (
    ChecklistAnnotation,
    ChecklistItem,
    ChecklistStatus,
    STATUS_DISPLAY,
    TaskChecklist,
    _synthesize_step_id,
    derive_checklist_from_plan,
    make_step_id,
    validate_step_id,
)


# ═══════════════════════════════════════════════════════════════════════════════
# Status vocabulary
# ═══════════════════════════════════════════════════════════════════════════════


class ChecklistStatusTests(unittest.TestCase):
    def test_all_six_values_present(self):
        values = {v.value for v in ChecklistStatus}
        self.assertEqual(
            values, {"todo", "doing", "done", "blocked", "deferred", "replanned"},
        )

    def test_display_symbols_for_every_status(self):
        for status in ChecklistStatus:
            self.assertIn(status, STATUS_DISPLAY)
            self.assertIsInstance(STATUS_DISPLAY[status], str)
            self.assertGreater(len(STATUS_DISPLAY[status]), 0)

    def test_display_symbol_values(self):
        self.assertEqual(STATUS_DISPLAY[ChecklistStatus.TODO], ".")
        self.assertEqual(STATUS_DISPLAY[ChecklistStatus.DOING], ">")
        self.assertEqual(STATUS_DISPLAY[ChecklistStatus.DONE], "done")
        self.assertEqual(STATUS_DISPLAY[ChecklistStatus.BLOCKED], "!")
        self.assertEqual(STATUS_DISPLAY[ChecklistStatus.DEFERRED], "~")
        self.assertEqual(STATUS_DISPLAY[ChecklistStatus.REPLANNED], "*")


# ═══════════════════════════════════════════════════════════════════════════════
# Step ID validation
# ═══════════════════════════════════════════════════════════════════════════════


class StepIdValidationTests(unittest.TestCase):
    def test_valid_step_ids(self):
        self.assertTrue(validate_step_id("GAK-01"))
        self.assertTrue(validate_step_id("UI-02"))
        self.assertTrue(validate_step_id("PLAN-03"))
        self.assertTrue(validate_step_id("GPS-04"))
        self.assertTrue(validate_step_id("LBR-05"))
        self.assertTrue(validate_step_id("GAK-INJ-01"))

    def test_invalid_step_ids(self):
        self.assertFalse(validate_step_id(""))               # empty
        self.assertFalse(validate_step_id("gak-01"))          # lower-case
        self.assertFalse(validate_step_id("01-GAK"))          # wrong order
        self.assertFalse(validate_step_id("GAK-1"))           # single digit
        self.assertFalse(validate_step_id("GAK-001"))         # three digits
        self.assertFalse(validate_step_id("GAK-XX"))          # non-numeric
        self.assertFalse(validate_step_id("G-01"))            # code too short
        self.assertFalse(validate_step_id("GAK_01"))          # underscore instead of hyphen

    def test_make_step_id(self):
        self.assertEqual(make_step_id("GAK", 1), "GAK-01")
        self.assertEqual(make_step_id("UI", 10), "UI-10")
        self.assertEqual(make_step_id("TASK", 99), "TASK-99")
        self.assertEqual(make_step_id("GAK-INJ", 5), "GAK-INJ-05")


# ═══════════════════════════════════════════════════════════════════════════════
# ChecklistItem
# ═══════════════════════════════════════════════════════════════════════════════


class ChecklistItemTests(unittest.TestCase):
    def test_defaults(self):
        item = ChecklistItem(step_id="GAK-01", title="Do the thing")
        self.assertEqual(item.step_id, "GAK-01")
        self.assertEqual(item.title, "Do the thing")
        self.assertEqual(item.status, ChecklistStatus.TODO)
        self.assertIsNotNone(item.annotations)

    def test_to_dict(self):
        item = ChecklistItem(
            step_id="GAK-01",
            title="Do the thing",
            status=ChecklistStatus.DOING,
            annotations=ChecklistAnnotation(
                plan_revision="v3",
                soft_constraints=["Keep it simple."],
                acceptance_refs=["UI-01"],
            ),
            created_at="2026-01-01T00:00:00Z",
            updated_at="2026-01-02T00:00:00Z",
        )
        d = item.to_dict()
        self.assertEqual(d["step_id"], "GAK-01")
        self.assertEqual(d["status"], "doing")
        self.assertEqual(d["annotations"]["plan_revision"], "v3")
        self.assertEqual(d["annotations"]["soft_constraints"], ["Keep it simple."])
        self.assertEqual(d["annotations"]["acceptance_refs"], ["UI-01"])

    def test_from_dict_round_trip(self):
        original = ChecklistItem(
            step_id="GAK-02",
            title="Second task",
            status=ChecklistStatus.BLOCKED,
            annotations=ChecklistAnnotation(
                plan_revision="v1",
                source_section="具体计划",
                deferred_decisions=["decide colour"],
                runtime_records=["Checked with team."],
            ),
            created_at="2026-01-01T00:00:00Z",
        )
        d = original.to_dict()
        restored = ChecklistItem.from_dict(d)
        self.assertEqual(restored.step_id, original.step_id)
        self.assertEqual(restored.title, original.title)
        self.assertEqual(restored.status, original.status)
        self.assertEqual(
            restored.annotations.plan_revision,
            original.annotations.plan_revision,
        )
        self.assertEqual(
            restored.annotations.deferred_decisions,
            original.annotations.deferred_decisions,
        )
        self.assertEqual(
            restored.annotations.runtime_records,
            original.annotations.runtime_records,
        )

    def test_from_dict_defaults_missing_annotations(self):
        d = {"step_id": "X-01", "title": "Naked item", "status": "todo"}
        item = ChecklistItem.from_dict(d)
        self.assertEqual(item.step_id, "X-01")
        self.assertIsNotNone(item.annotations)
        self.assertEqual(item.annotations.plan_revision, "")


# ═══════════════════════════════════════════════════════════════════════════════
# TaskChecklist
# ═══════════════════════════════════════════════════════════════════════════════


class TaskChecklistTests(unittest.TestCase):
    def _make_items(self, *statuses: ChecklistStatus) -> list[ChecklistItem]:
        items = []
        for i, s in enumerate(statuses, 1):
            items.append(ChecklistItem(
                step_id=f"TASK-{i:02d}",
                title=f"Step {i}",
                status=s,
            ))
        return items

    def test_current_index_doing(self):
        cl = TaskChecklist(
            plan_id="P-1", task_id="T-1", run_id="R-1",
            items=self._make_items(
                ChecklistStatus.DONE,
                ChecklistStatus.DOING,
                ChecklistStatus.TODO,
                ChecklistStatus.TODO,
            ),
        )
        self.assertEqual(cl.current_index, 1)

    def test_current_index_first_todo_after_done(self):
        cl = TaskChecklist(
            plan_id="P-1", task_id="T-1", run_id="R-1",
            items=self._make_items(
                ChecklistStatus.DONE,
                ChecklistStatus.DONE,
                ChecklistStatus.TODO,
            ),
        )
        self.assertEqual(cl.current_index, 2)

    def test_current_item_returns_none_for_empty(self):
        cl = TaskChecklist(plan_id="P-1", task_id="T-1", run_id="R-1")
        self.assertIsNone(cl.current_item)

    def test_current_item_returns_correct_item(self):
        cl = TaskChecklist(
            plan_id="P-1", task_id="T-1", run_id="R-1",
            items=self._make_items(
                ChecklistStatus.DONE,
                ChecklistStatus.DOING,
            ),
        )
        item = cl.current_item
        self.assertIsNotNone(item)
        self.assertEqual(item.step_id, "TASK-02")

    def test_to_dict_from_dict_round_trip(self):
        items = self._make_items(
            ChecklistStatus.DONE, ChecklistStatus.DOING, ChecklistStatus.TODO,
        )
        items[1].annotations = ChecklistAnnotation(
            plan_revision="v2",
            soft_constraints=["no hard gates"],
            acceptance_refs=["UI-01", "PLAN-02"],
        )
        original = TaskChecklist(
            plan_id="PLAN-ABC123456789",
            task_id="task-1",
            run_id="run-1",
            items=items,
            created_at="2026-07-29T00:00:00Z",
            updated_at="2026-07-29T01:00:00Z",
        )
        d = original.to_dict()
        restored = TaskChecklist.from_dict(d)
        self.assertEqual(restored.plan_id, original.plan_id)
        self.assertEqual(restored.task_id, original.task_id)
        self.assertEqual(len(restored.items), 3)
        self.assertEqual(restored.items[1].annotations.soft_constraints, ["no hard gates"])
        self.assertEqual(restored.items[1].annotations.acceptance_refs, ["UI-01", "PLAN-02"])
        self.assertEqual(restored.current_index, 1)


# ═══════════════════════════════════════════════════════════════════════════════
# derive_checklist_from_plan
# ═══════════════════════════════════════════════════════════════════════════════


def _make_plan(sections: list[PlanSection] | None = None) -> PlanArtifact:
    """Build a minimal PlanArtifact for testing."""
    if sections is None:
        sections = [
            PlanSection(title="前期调查", content_md="Investigated the codebase.", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="1. GAK-01: Survey existing guard\n2. GAK-02: Freeze checklist supplement", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="Design the thing.", evidence_status="inferred"),
            PlanSection(title="实施方案", content_md="Implement it.", evidence_status="observed"),
        ]
    return build_plan_artifact(
        task_id="task-1",
        run_id="run-1",
        workspace_root="/tmp/test",
        sections=sections,
        planning_policy="required",
    )


class DeriveChecklistTests(unittest.TestCase):
    def test_derives_from_numbered_list_with_step_ids(self):
        plan = _make_plan([
            PlanSection(title="前期调查", content_md="Looked around.", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="1. GAK-01: Survey guard\n2. GAK-02: Freeze supplement\n3. GAK-03: Register index", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="Design.", evidence_status="observed"),
            PlanSection(title="实施方案", content_md="Implement.", evidence_status="observed"),
        ])
        cl = derive_checklist_from_plan(plan, task_code="GAK")
        self.assertEqual(len(cl.items), 3)
        self.assertEqual(cl.items[0].step_id, "GAK-01")
        self.assertEqual(cl.items[0].title, "Survey guard")
        self.assertEqual(cl.items[1].step_id, "GAK-02")
        self.assertEqual(cl.items[2].step_id, "GAK-03")

    def test_derives_from_markdown_headings(self):
        plan = _make_plan([
            PlanSection(title="前期调查", content_md="x", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="## GAK-01: First\n## GAK-02 Second\n### GAK-03: Third", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="x", evidence_status="observed"),
            PlanSection(title="实施方案", content_md="x", evidence_status="observed"),
        ])
        cl = derive_checklist_from_plan(plan, task_code="GAK")
        self.assertEqual(len(cl.items), 3)
        self.assertEqual(cl.items[0].title, "First")
        self.assertEqual(cl.items[1].title, "Second")
        self.assertEqual(cl.items[2].title, "Third")

    def test_derives_from_bullets(self):
        plan = _make_plan([
            PlanSection(title="前期调查", content_md="x", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="- GAK-01: A\n* GAK-02: B", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="x", evidence_status="observed"),
            PlanSection(title="实施方案", content_md="x", evidence_status="observed"),
        ])
        cl = derive_checklist_from_plan(plan, task_code="GAK")
        self.assertEqual(len(cl.items), 2)
        self.assertEqual(cl.items[0].title, "A")
        self.assertEqual(cl.items[1].title, "B")

    def test_synthesises_ids_from_bare_bullets(self):
        plan = _make_plan([
            PlanSection(title="前期调查", content_md="x", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="- Do something\n- Do another thing", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="x", evidence_status="observed"),
            PlanSection(title="实施方案", content_md="x", evidence_status="observed"),
        ])
        cl = derive_checklist_from_plan(plan, task_code="GAK")
        self.assertEqual(len(cl.items), 2)
        self.assertTrue(cl.items[0].step_id.startswith("GAK-"))
        self.assertTrue(cl.items[1].step_id.startswith("GAK-"))

    def test_annotations_populated(self):
        plan = _make_plan([
            PlanSection(title="前期调查", content_md="x", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="1. GAK-01: Do it", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="x", evidence_status="observed"),
            PlanSection(title="实施方案", content_md="x", evidence_status="observed"),
        ])
        plan.deferred_decisions = ["pick colour", "decide format"]
        cl = derive_checklist_from_plan(plan)
        self.assertEqual(len(cl.items), 1)
        ann = cl.items[0].annotations
        self.assertIn("v1", ann.plan_revision)
        self.assertEqual(ann.source_section, "具体计划")
        self.assertEqual(ann.deferred_decisions, ["pick colour", "decide format"])

    def test_empty_plan_section_yields_single_step(self):
        plan = _make_plan([
            PlanSection(title="前期调查", content_md="x", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="x", evidence_status="observed"),
            PlanSection(title="实施方案", content_md="x", evidence_status="observed"),
        ])
        cl = derive_checklist_from_plan(plan)
        self.assertEqual(len(cl.items), 1)
        self.assertEqual(cl.items[0].step_id, "TASK-01")

    def test_items_start_as_todo(self):
        plan = _make_plan()
        cl = derive_checklist_from_plan(plan)
        for item in cl.items:
            self.assertEqual(item.status, ChecklistStatus.TODO)

    def test_skips_duplicate_step_ids(self):
        plan = _make_plan([
            PlanSection(title="前期调查", content_md="x", evidence_status="observed"),
            PlanSection(title="具体计划", content_md="1. GAK-01: First\n2. GAK-01: Duplicate\n3. GAK-02: Third", evidence_status="observed"),
            PlanSection(title="具体设计", content_md="x", evidence_status="observed"),
            PlanSection(title="实施方案", content_md="x", evidence_status="observed"),
        ])
        cl = derive_checklist_from_plan(plan, task_code="GAK")
        self.assertEqual(len(cl.items), 2)
        ids = [i.step_id for i in cl.items]
        self.assertIn("GAK-01", ids)
        self.assertIn("GAK-02", ids)


class SynthesizeStepIdTests(unittest.TestCase):
    def test_extracts_embedded_id(self):
        sid, title = _synthesize_step_id("TASK", 1, "GAK-05: Some title")
        self.assertEqual(sid, "GAK-05")
        self.assertEqual(title, "GAK-05: Some title")

    def test_falls_back_to_prefix(self):
        sid, title = _synthesize_step_id("TASK", 5, "Just a description")
        self.assertEqual(sid, "TASK-05")
        self.assertEqual(title, "Just a description")
