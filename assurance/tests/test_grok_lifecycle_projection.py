from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from assurance.contracts import validate_contract
from assurance.grok_lifecycle_projection import (
    build_grok_lifecycle_orientation_checkpoint,
    project_grok_lifecycle_events_to_global_progress,
)


ROOT = Path(__file__).resolve().parents[2]
GPS_FIXTURE = ROOT / "runtime" / "fixtures" / "global-progress-sentinel-v0.1" / "input.json"
GPS_BUILDER = ROOT / "scripts" / "build_global_progress_review.py"


def _gps_input() -> dict[str, object]:
    return json.loads(GPS_FIXTURE.read_text(encoding="utf-8"))


def _events() -> list[dict[str, object]]:
    return [
        {
            "event_type": "workflow_started",
            "subject_id": "WF-GROK-001",
            "step_id": "STEP-ACP-02",
            "direction_id": "runtime_acp",
        },
        {
            "event_type": "subagent_started",
            "subject_id": "SUBAGENT-GROK-RETRIEVAL-001",
            "step_id": "STEP-ACP-02",
            "direction_id": "runtime_acp",
        },
        {
            "event_type": "subagent_completed",
            "subject_id": "SUBAGENT-GROK-RETRIEVAL-001",
            "step_id": "STEP-ACP-02",
            "direction_id": "runtime_acp",
            "write_effect": False,
        },
        {
            "event_type": "workflow_step_verified",
            "subject_id": "WF-GROK-STEP-001",
            "step_id": "STEP-ACP-02",
            "direction_id": "runtime_acp",
            "verification_state": "pass",
        },
    ]


class GrokLifecycleProjectionTests(unittest.TestCase):
    def test_projects_lifecycle_metadata_into_gps_journal_and_orientation_context(self) -> None:
        bundle = project_grok_lifecycle_events_to_global_progress(
            global_progress_input=_gps_input(),
            lifecycle_events=_events(),
            observed_at="2026-07-30T00:00:00Z",
        )
        receipt = bundle["receipt"]

        validate_contract(
            receipt,
            "grok-lifecycle-projection-receipt-v0.1.schema.json",
            label="grok lifecycle projection receipt",
        )
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["decision"], "allow")
        self.assertEqual(receipt["projected_gps_event_count"], 4)
        self.assertEqual(
            [event["event_type"] for event in receipt["gps_journal_events"]],
            [
                "artifact_registered",
                "artifact_registered",
                "action_terminal",
                "verification_result",
            ],
        )
        self.assertEqual(
            bundle["global_progress_input"]["journal"][-1]["verification_state"],
            "pass",
        )
        self.assertIn(
            "[GROK_LIFECYCLE_CONTEXT v0.1]",
            bundle["orientation_context_block"],
        )
        self.assertNotIn("claim_disposition", bundle["orientation_context_block"])

    def test_projected_input_is_consumed_by_existing_gps_builder(self) -> None:
        bundle = project_grok_lifecycle_events_to_global_progress(
            global_progress_input=_gps_input(),
            lifecycle_events=_events(),
            observed_at="2026-07-30T00:00:00Z",
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            input_path = root / "input.json"
            output_path = root / "review.json"
            input_path.write_text(
                json.dumps(bundle["global_progress_input"], ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(GPS_BUILDER), "--input", str(input_path), "--output", str(output_path)],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=20,
                check=False,
            )

        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_builds_neutral_orientation_checkpoint_from_projection(self) -> None:
        bundle = project_grok_lifecycle_events_to_global_progress(
            global_progress_input=_gps_input(),
            lifecycle_events=_events(),
            observed_at="2026-07-30T00:00:00Z",
        )
        checkpoint = build_grok_lifecycle_orientation_checkpoint(
            projection_receipt=bundle["receipt"],
            task_id="TASK-GPS-FIXTURE-001",
            trigger_step=9,
            task_contract_sha256="a" * 64,
        )

        self.assertTrue(
            checkpoint["message_block"].startswith("[GROK_LIFECYCLE_CONTEXT v0.1]")
        )
        self.assertFalse(
            checkpoint["claim_policy"]["may_generate_counterexample_candidate"]
        )
        self.assertIn("orientation_summary", checkpoint["allowed_response_fields"])

    def test_content_bearing_lifecycle_event_blocks_projection(self) -> None:
        events = _events()
        events.append(
            {
                "event_type": "subagent_completed",
                "subject_id": "SUBAGENT-GROK-RETRIEVAL-002",
                "step_id": "STEP-ACP-02",
                "direction_id": "runtime_acp",
                "raw_output": "PRIVATE CONTENT",
            }
        )

        receipt = project_grok_lifecycle_events_to_global_progress(
            global_progress_input=_gps_input(),
            lifecycle_events=events,
            observed_at="2026-07-30T00:00:00Z",
        )["receipt"]

        self.assertFalse(receipt["valid"])
        self.assertEqual(receipt["decision"], "block")
        self.assertFalse(receipt["checks"]["source_events_metadata_only"])

    def test_direction_binding_mismatch_blocks_projection(self) -> None:
        events = _events()
        events[0] = {
            **events[0],
            "direction_id": "evidence_gates",
        }

        receipt = project_grok_lifecycle_events_to_global_progress(
            global_progress_input=_gps_input(),
            lifecycle_events=events,
            observed_at="2026-07-30T00:00:00Z",
        )["receipt"]

        self.assertFalse(receipt["valid"])
        self.assertFalse(receipt["checks"]["gps_step_bindings_valid"])
        self.assertTrue(receipt["errors"])


if __name__ == "__main__":
    unittest.main()
