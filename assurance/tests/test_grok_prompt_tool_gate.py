from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from assurance.errors import AssuranceError
from assurance.grok_prompt_tool_gate import (
    build_grok_prompt_tool_promotion_gate_receipt,
    write_grok_prompt_tool_promotion_gate_receipt,
)
from assurance.utils import atomic_write_json


ROOT = Path(__file__).resolve().parents[2]


def _tool_gate(decision: str = "allow") -> dict[str, object]:
    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "tool_availability_gate_receipt",
        "valid": True,
        "report_id": "TOOL-AVAIL-GROK-TEST-001",
        "report_sha256": "a" * 64,
        "available_count": 5 if decision == "allow" else 4,
        "unavailable_count": 0,
        "unprobed_count": 0,
        "degraded_count": 0 if decision == "allow" else 1,
        "context_block_sha256": "b" * 64,
        "decisions": {
            "gate_decision": decision,
            "context_injected": True,
            "model_must_not_guess": True,
        },
        "checks": {
            "all_declared_tools_probed": True,
            "unavailable_tools_listed": True,
            "no_capability_guessing_permitted": True,
            "mechanical_probe_only": True,
            "no_model_invoked": True,
            "no_network_requested": True,
        },
        "limitations": ["test fixture"],
    }


def _timeout_gate(
    *,
    owned_status: str = "passed",
    prompt_ready: bool = True,
) -> dict[str, object]:
    return {
        "schema_version": "0.1.0",
        "receipt_kind": "grok-timeout-gate-split-verification",
        "valid": True,
        "prompt_tool_promotion_ready": prompt_ready,
        "gates": {
            "windows_child_tree_baseline_regression": {
                "status": "passed",
                "checks": {"baseline_no_regression": True},
            },
            "windows_child_tree_owned_cleanup": {
                "status": owned_status,
                "checks": {"owned_cleanup_established": owned_status == "passed"},
            },
        },
    }


class GrokPromptToolGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_missing_evidence_blocks_without_prompt_execution(self) -> None:
        receipt = build_grok_prompt_tool_promotion_gate_receipt(
            run_root=self.root / "run",
            ask="hello",
        )

        self.assertEqual(receipt["decision"], "block")
        self.assertIn("tool_availability_gate_not_allow", receipt["blocking_reasons"])
        self.assertIn(
            "windows_child_tree_owned_cleanup_not_passed",
            receipt["blocking_reasons"],
        )
        self.assertFalse(receipt["checks"]["prompt_tool_execution_attempted"])
        self.assertTrue(receipt["checks"]["canonical_default_unchanged"])

    def test_tool_allow_still_blocks_when_owned_cleanup_is_carried(self) -> None:
        receipt = build_grok_prompt_tool_promotion_gate_receipt(
            run_root=self.root / "run",
            ask="hello",
            tool_availability_gate_receipt=_tool_gate("allow"),
            timeout_gate_receipt=_timeout_gate(
                owned_status="carried-limitation",
                prompt_ready=False,
            ),
        )

        self.assertEqual(receipt["decision"], "block")
        self.assertEqual(
            receipt["blocking_reasons"],
            ["windows_child_tree_owned_cleanup_not_passed"],
        )
        self.assertEqual(
            receipt["prerequisites"]["tool_availability"]["status"],
            "passed",
        )

    def test_all_prerequisites_allow_promotion_gate_only(self) -> None:
        receipt = build_grok_prompt_tool_promotion_gate_receipt(
            run_root=self.root / "run",
            ask="hello",
            retrieval_mode="local_browser",
            retrieval_mode_explicit=True,
            tool_availability_gate_receipt=_tool_gate("allow"),
            timeout_gate_receipt=_timeout_gate(),
        )

        self.assertEqual(receipt["decision"], "allow")
        self.assertEqual(receipt["blocking_reasons"], [])
        self.assertEqual(
            receipt["runtime"]["production_prompt_tool_path"],
            "not_promoted",
        )
        self.assertFalse(receipt["checks"]["prompt_tool_execution_attempted"])

    def test_write_receipt_refuses_nonempty_run_root(self) -> None:
        run_root = self.root / "occupied"
        run_root.mkdir()
        (run_root / "old.txt").write_text("old", encoding="utf-8")

        with self.assertRaises(AssuranceError):
            write_grok_prompt_tool_promotion_gate_receipt(
                run_root=run_root,
                ask="hello",
            )

    def test_write_receipt_loads_attached_evidence(self) -> None:
        run_root = self.root / "run"
        tool_path = self.root / "tool-gate.json"
        timeout_path = self.root / "timeout-gate.json"
        atomic_write_json(tool_path, _tool_gate("allow"))
        atomic_write_json(timeout_path, _timeout_gate())

        receipt = write_grok_prompt_tool_promotion_gate_receipt(
            run_root=run_root,
            ask="hello",
            tool_availability_gate_path=tool_path,
            timeout_gate_path=timeout_path,
        )

        self.assertTrue((run_root / "grok-prompt-tool-promotion-gate.json").is_file())
        self.assertEqual(receipt["decision"], "allow")


if __name__ == "__main__":
    unittest.main()
