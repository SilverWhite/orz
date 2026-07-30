from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker

from assurance.grok_event_normalizer import (
    normalize_grok_runtime_receipt,
    project_grok_events_for_tui,
    write_grok_events_jsonl,
)


ROOT = Path(__file__).resolve().parents[2]
RUNTIME_SCHEMA = ROOT / "runtime" / "run-event-v0.1.schema.json"


def _receipt(valid: bool = True) -> dict[str, object]:
    return {
        "schema_version": "0.1.0",
        "receipt_kind": "grok_runtime_adapter_receipt",
        "run_id": "RUN-GROK-RUNTIME-SMOKE-TEST",
        "created_at": "2026-07-30T00:00:00Z",
        "valid": valid,
        "adapter": {
            "adapter_id": "grok-runtime-adapter",
            "adapter_version": "0.1.0",
            "runtime_owner": "grok",
        },
        "request": {
            "mode": "version_smoke",
            "retrieval_mode": "local_browser",
            "retrieval_mode_explicit": True,
            "workspace_path": "D:\\CLI",
            "run_root": "D:\\CLI\\tmp-grok-smoke",
        },
        "retrieval": {
            "mode": "local_browser",
            "selected_explicitly": True,
            "applies_to": "prompt_tool_runs",
            "active_for_current_mode": False,
            "runtime_tool_retrieval_allowed": True,
            "assurance_receipts_required": True,
            "valid_modes": ["local_browser", "framework_fallback", "off"],
        },
        "binary": {
            "inspection_path": "D:\\CLI\\tmp-grok-smoke\\binary-inspection.json",
            "binary_path": "D:\\CLI\\.tools\\grok\\0.2.112\\grok.exe",
            "version_output": "grok 0.2.112 (9bbd559437) [stable]",
            "sha256": "a" * 64,
            "valid": True,
        },
        "workspace_trust": {
            "receipt_path": "D:\\CLI\\tmp-grok-smoke\\workspace-trust.json",
            "valid": True,
            "trust_granted": True,
            "aggregate_sha256": "b" * 64,
        },
        "execution": {
            "command_kind": "grok_version",
            "pid": 12345,
            "exit_code": 0 if valid else 1,
            "timeout_seconds": 30,
            "stdout_path": "D:\\CLI\\tmp-grok-smoke\\grok.stdout.log",
            "stderr_path": "D:\\CLI\\tmp-grok-smoke\\grok.stderr.log",
            "stdout_sha256": "c" * 64,
            "stderr_sha256": "d" * 64,
        },
        "containment": {
            "no_residue_required": True,
            "no_residue_observed": valid,
            "root_process_exited": valid,
            "external_cleanup_required": False,
            "residue_scan_scope": "root_process_only",
        },
        "artifacts": {
            "receipt_path": "D:\\CLI\\tmp-grok-smoke\\grok-runtime-receipt.json",
            "events_path": "D:\\CLI\\tmp-grok-smoke\\events.jsonl",
        },
        "checks": {
            "binary_inspection_valid": True,
            "workspace_trust_valid": True,
            "workspace_trust_granted": True,
            "root_process_exited": valid,
            "no_residue_observed": valid,
        },
        "limitations": ["fixture"],
    }


class GrokEventNormalizerTests(unittest.TestCase):
    def test_normalized_events_validate_and_hash_chain(self) -> None:
        events = normalize_grok_runtime_receipt(
            _receipt(), created_at="2026-07-30T00:00:00Z"
        )
        schema = json.loads(RUNTIME_SCHEMA.read_text(encoding="utf-8"))
        previous = None
        for index, event in enumerate(events):
            self.assertEqual(event["sequence"], index)
            self.assertEqual(event["previous_event_sha256"], previous)
            Draft202012Validator(
                schema, format_checker=FormatChecker()
            ).validate(event)
            previous = event["event_sha256"]
        self.assertEqual(
            [event["event_type"] for event in events],
            ["run_preflight", "run_started", "artifact_registered", "run_finished"],
        )
        self.assertEqual(events[0]["payload"]["retrieval_mode"], "local_browser")
        self.assertFalse(events[0]["payload"]["retrieval_active_for_current_mode"])
        self.assertTrue(events[0]["payload"]["retrieval_mode_explicit"])
        self.assertTrue(events[-1]["payload"]["no_residue_observed"])

    def test_invalid_receipt_projects_failed_terminal(self) -> None:
        events = normalize_grok_runtime_receipt(
            _receipt(False), created_at="2026-07-30T00:00:00Z"
        )
        self.assertEqual(events[-1]["event_type"], "run_failed")
        self.assertFalse(events[-1]["payload"]["no_residue_observed"])

    def test_write_jsonl_and_tui_projection_are_metadata_only(self) -> None:
        events = normalize_grok_runtime_receipt(
            _receipt(), created_at="2026-07-30T00:00:00Z"
        )
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            path = Path(temporary) / "events.jsonl"
            write_grok_events_jsonl(path, events)
            lines = path.read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(lines), len(events))
        projection = project_grok_events_for_tui(events)
        self.assertEqual(projection[0]["kind"], "run_preflight")
        self.assertEqual(
            {item["redaction"] for item in projection},
            {"metadata_only"},
        )


if __name__ == "__main__":
    unittest.main()
