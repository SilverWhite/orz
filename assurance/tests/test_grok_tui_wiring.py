from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from assurance.tui.bridge import build_grok_live_run_fn


ROOT = Path(__file__).resolve().parents[2]


class GrokTuiWiringTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name) / "run"
        self.run_root.mkdir()
        self.events_path = self.run_root / "events.jsonl"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    @patch("assurance.grok_runtime_adapter.run_grok_version_smoke")
    def test_grok_live_run_fn_streams_normalized_events(self, mock_run) -> None:
        events = [
            {
                "event_type": "run_preflight",
                "payload": {
                    "adapter_id": "grok-runtime-adapter",
                    "no_residue_required": True,
                },
            },
            {
                "event_type": "run_finished",
                "payload": {
                    "status": "completed",
                    "no_residue_observed": True,
                    "external_cleanup_required": False,
                },
            },
        ]
        self.events_path.write_text(
            "".join(json.dumps(event) + "\n" for event in events),
            encoding="utf-8",
        )
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-GROK-TUI-TEST-001",
            "artifacts": {"events_path": str(self.events_path)},
            "containment": {
                "no_residue_required": True,
                "no_residue_observed": True,
                "external_cleanup_required": False,
            },
        }
        emitted: list[dict[str, object]] = []
        run_fn = build_grok_live_run_fn(
            run_root=str(self.run_root),
            workspace=str(ROOT),
            run_id="RUN-GROK-TUI-TEST-001",
            retrieval_mode="framework_fallback",
            retrieval_mode_explicit=True,
        )

        receipt = run_fn(lambda event: emitted.append(event))

        self.assertTrue(receipt["valid"])
        self.assertEqual([event["event_type"] for event in emitted], [
            "run_preflight",
            "run_finished",
        ])
        self.assertTrue(emitted[1]["payload"]["no_residue_observed"])
        mock_run.assert_called_once_with(
            run_root=self.run_root,
            workspace_path=ROOT,
            run_id="RUN-GROK-TUI-TEST-001",
            retrieval_mode="framework_fallback",
            retrieval_mode_explicit=True,
        )
