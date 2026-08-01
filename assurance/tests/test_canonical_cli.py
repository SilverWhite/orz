from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO

from assurance import (
    AssuranceError,
    run_canonical_guarded_cli,
    verify_canonical_guarded_cli_run,
)
from assurance.canonical_cli_main import main as canonical_cli_main
from assurance.task_contract import build_task_contract_from_ask
from assurance.utils import atomic_write_json, canonical_bytes, sha256_bytes


ROOT = Path(__file__).resolve().parents[2]
SOURCE_LEDGER = ROOT / "assurance/fixtures/source_visibility/mixed-visibility-ledger.json"
CREATED_AT = "2026-07-26T12:30:00Z"
ASK = "Check whether the cited source can support a mechanism claim."


class CanonicalCliRunTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name) / "canonical-run"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_canonical_cli_run_writes_guarded_artifacts_and_verifies(self) -> None:
        receipt = run_canonical_guarded_cli(
            run_root=self.run_root,
            source_ledger_path=SOURCE_LEDGER,
            ask=ASK,
            created_at=CREATED_AT,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(
            receipt["event_types"],
            [
                "run_preflight",
                "instruction_provenance_gate",
                "tool_availability_check",
                "orientation_checkpoint",
                "run_started",
                "gate_decision",
                "model_request",
                "model_output",
                "artifact_registered",
                "run_finished",
            ],
        )
        self.assertTrue(receipt["checks"]["gate_before_model_request"])
        self.assertTrue(receipt["checks"]["instruction_gate_before_model"])
        self.assertTrue(receipt["checks"]["fake_adapter_no_network"])
        self.assertTrue(receipt["checks"]["task_contract_valid"])
        self.assertTrue(receipt["checks"]["answer_binds_task_contract"])
        self.assertIsNotNone(receipt["instruction_provenance_gate_context_sha256"])
        self.assertIsNotNone(receipt["instruction_provenance_gate_receipt_sha256"])
        answer = json.loads(
            (self.run_root / "answer-packet.json").read_text(encoding="utf-8")
        )
        self.assertEqual(answer["source_visibility_gate"]["decision"], "warn")
        self.assertEqual(
            answer["claim_boundaries"]["scientific_claim_strength"],
            "observed_fragment_only",
        )
        self.assertTrue(answer["answer"]["deferred_claims"])
        task_contract = json.loads(
            (self.run_root / "task-contract.json").read_text(encoding="utf-8")
        )
        self.assertEqual(task_contract["user_request"]["raw_text"], ASK)
        self.assertEqual(answer["task_contract"]["entry_mode"], "ask")

        verification = verify_canonical_guarded_cli_run(run_root=self.run_root)
        self.assertEqual(verification, receipt)

    def test_cli_run_and_verify_emit_receipts(self) -> None:
        run_output = StringIO()
        with redirect_stdout(run_output):
            run_exit = canonical_cli_main(
                [
                    "run",
                    "--run-root",
                    str(self.run_root),
                    "--source-ledger",
                    str(SOURCE_LEDGER),
                    "--ask",
                    ASK,
                    "--created-at",
                    CREATED_AT,
                ]
            )
        run_receipt = json.loads(run_output.getvalue())

        verify_output = StringIO()
        with redirect_stdout(verify_output):
            verify_exit = canonical_cli_main(
                ["verify", "--run-root", str(self.run_root)]
            )
        verify_receipt = json.loads(verify_output.getvalue())

        self.assertEqual(run_exit, 0)
        self.assertEqual(verify_exit, 0)
        self.assertEqual(run_receipt, verify_receipt)

    def test_verifier_detects_gate_receipt_tamper(self) -> None:
        run_canonical_guarded_cli(
            run_root=self.run_root,
            source_ledger_path=SOURCE_LEDGER,
            ask=ASK,
            created_at=CREATED_AT,
        )
        gate_path = self.run_root / "source-visibility-gate-receipt.json"
        gate = json.loads(gate_path.read_text(encoding="utf-8"))
        gate["decision"] = "allow"
        gate_path.write_text(
            json.dumps(gate, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

        with self.assertRaisesRegex(AssuranceError, "does not recompute"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_detects_model_request_before_gate(self) -> None:
        run_canonical_guarded_cli(
            run_root=self.run_root,
            source_ledger_path=SOURCE_LEDGER,
            ask=ASK,
            created_at=CREATED_AT,
        )
        journal_path = self.run_root / "events.jsonl"
        events = [
            json.loads(line)
            for line in journal_path.read_text(encoding="utf-8").splitlines()
        ]
        events[5], events[6] = events[6], events[5]
        journal_path.write_text(
            "\n".join(
                json.dumps(event, sort_keys=True, separators=(",", ":"))
                for event in events
            )
            + "\n",
            encoding="utf-8",
        )

        with self.assertRaises(AssuranceError):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_run_refuses_nonempty_run_root(self) -> None:
        self.run_root.mkdir(parents=True)
        (self.run_root / "existing.txt").write_text("occupied\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "empty or absent"):
            run_canonical_guarded_cli(
                run_root=self.run_root,
                source_ledger_path=SOURCE_LEDGER,
                ask=ASK,
                created_at=CREATED_AT,
            )

    def test_run_can_reuse_task_contract_file(self) -> None:
        task_path = Path(self.temporary.name) / "task.json"
        task_contract = build_task_contract_from_ask(
            ask=ASK,
            source_ledger_path=SOURCE_LEDGER,
            created_at=CREATED_AT,
        )
        atomic_write_json(task_path, task_contract)

        receipt = run_canonical_guarded_cli(
            run_root=self.run_root,
            task_contract_path=task_path,
            created_at=CREATED_AT,
        )

        self.assertTrue(receipt["valid"])
        frozen_task = json.loads(
            (self.run_root / "task-contract.json").read_text(encoding="utf-8")
        )
        self.assertEqual(frozen_task, task_contract)

    def test_task_contract_records_absolute_source_ledger_path(self) -> None:
        task_contract = build_task_contract_from_ask(
            ask=ASK,
            source_ledger_path=Path(
                "assurance/fixtures/source_visibility/mixed-visibility-ledger.json"
            ),
            created_at=CREATED_AT,
        )

        self.assertTrue(Path(task_contract["source_ledger"]["path"]).is_absolute())

    def test_run_requires_exactly_one_task_entry_mode(self) -> None:
        task_path = Path(self.temporary.name) / "task.json"
        task_contract = build_task_contract_from_ask(
            ask=ASK,
            source_ledger_path=SOURCE_LEDGER,
            created_at=CREATED_AT,
        )
        atomic_write_json(task_path, task_contract)

        with self.assertRaisesRegex(AssuranceError, "exactly one"):
            run_canonical_guarded_cli(
                run_root=self.run_root,
                source_ledger_path=SOURCE_LEDGER,
                created_at=CREATED_AT,
            )
        self.assertFalse(self.run_root.exists())

        with self.assertRaisesRegex(AssuranceError, "exactly one"):
            run_canonical_guarded_cli(
                run_root=self.run_root,
                source_ledger_path=SOURCE_LEDGER,
                ask=ASK,
                task_contract_path=task_path,
                created_at=CREATED_AT,
            )
        self.assertFalse(self.run_root.exists())

    def test_ask_requires_source_ledger_before_run_root_write(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "requires a source ledger"):
            run_canonical_guarded_cli(
                run_root=self.run_root,
                ask=ASK,
                created_at=CREATED_AT,
            )

        self.assertFalse(self.run_root.exists())

    def test_verifier_detects_task_contract_tamper(self) -> None:
        run_canonical_guarded_cli(
            run_root=self.run_root,
            source_ledger_path=SOURCE_LEDGER,
            ask=ASK,
            created_at=CREATED_AT,
        )
        task_path = self.run_root / "task-contract.json"
        task = json.loads(task_path.read_text(encoding="utf-8"))
        task["user_request"]["raw_text"] = "tampered"
        task_path.write_text(
            json.dumps(task, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

        with self.assertRaisesRegex(AssuranceError, "answer packet does not bind task contract"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)


class CanonicalCliWithInstructionGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name) / "canonical-run"
        self.addCleanup(self.temporary.cleanup)

    def test_run_with_instruction_provenance_gate(self) -> None:
        from assurance.instruction_provenance_gate import build_instruction_provenance_gate_context

        ipg_dir = Path(self.temporary.name) / "ipg-input"
        ipg_dir.mkdir()
        ipg_context = build_instruction_provenance_gate_context(
            run_id="RUN-IPG-TEST-001",
            conversation_id="CONV-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            instructions=[
                {
                    "entry_id": "INS-USER-001",
                    "declared_source_type": "user",
                    "source_id": "user-prompt-main",
                    "content_sha256": "a" * 64,
                    "content_bytes": 256,
                    "instruction_kind": "user_prompt",
                },
            ],
        )
        ipg_context_path = ipg_dir / "instruction-provenance-gate-context.json"
        atomic_write_json(ipg_context_path, ipg_context)

        receipt = run_canonical_guarded_cli(
            run_root=self.run_root,
            ask=ASK,
            source_ledger_path=SOURCE_LEDGER,
            instruction_provenance_gate_context_path=ipg_context_path,
            run_id="RUN-IPG-TEST-001",
            created_at=CREATED_AT,
        )
        self.assertTrue(receipt["valid"])
        self.assertTrue(receipt["checks"]["instruction_gate_before_model"])
        self.assertTrue(receipt["checks"]["tool_availability_gate_before_model"])
        self.assertIsNotNone(receipt["instruction_provenance_gate_receipt_sha256"])
        self.assertIsNotNone(receipt["instruction_provenance_gate_context_sha256"])
        self.assertIsNotNone(receipt["tool_availability_report_sha256"])
        self.assertIsNotNone(receipt["tool_availability_gate_receipt_sha256"])

        event_types = receipt["event_types"]
        self.assertIn("instruction_provenance_gate", event_types)
        self.assertIn("tool_availability_check", event_types)
        pg_idx = event_types.index("instruction_provenance_gate")
        gd_idx = event_types.index("gate_decision")
        self.assertLess(pg_idx, gd_idx,
                        "instruction provenance gate must precede source visibility gate")

        verify_canonical_guarded_cli_run(run_root=self.run_root)

    # -- Journal integrity negative tests -----------------------------------

    def _run_and_get_journal(self) -> list[dict]:
        run_canonical_guarded_cli(
            run_root=self.run_root,
            source_ledger_path=SOURCE_LEDGER,
            ask=ASK,
            created_at=CREATED_AT,
        )
        journal_path = self.run_root / "events.jsonl"
        return [
            json.loads(line)
            for line in journal_path.read_text(encoding="utf-8").splitlines()
        ]

    def _write_journal(self, events: list[dict]) -> None:
        journal_path = self.run_root / "events.jsonl"
        journal_path.write_text(
            "\n".join(
                json.dumps(e, sort_keys=True, separators=(",", ":"))
                for e in events
            )
            + "\n",
            encoding="utf-8",
        )

    @staticmethod
    def _renumber(events: list[dict]) -> list[dict]:
        """Re-number event sequences AND recompute all event_sha256 values
        so the hash chain stays valid."""
        previous_hash: str | None = None
        for i, event in enumerate(events):
            event["sequence"] = i
            event["previous_event_sha256"] = previous_hash
            projection = dict(event)
            projection.pop("event_sha256", None)
            event["event_sha256"] = sha256_bytes(canonical_bytes(projection))
            previous_hash = event["event_sha256"]
        return events

    def test_verifier_detects_hash_chain_break(self) -> None:
        events = self._run_and_get_journal()
        # Break the chain by modifying previous_event_sha256
        events[3]["previous_event_sha256"] = "0" * 64
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError, "previous hash mismatch"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_detects_sequence_mismatch(self) -> None:
        events = self._run_and_get_journal()
        events[2]["sequence"] = 99
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError, "sequence mismatch"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_detects_manifest_digest_mismatch(self) -> None:
        events = self._run_and_get_journal()
        events[0]["run_manifest_sha256"] = "0" * 64
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError, "manifest digest mismatch"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_detects_run_id_mismatch(self) -> None:
        events = self._run_and_get_journal()
        events[1]["run_id"] = "RUN-WRONG-001"
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError, "run_id mismatch"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_detects_payload_digest_mismatch(self) -> None:
        events = self._run_and_get_journal()
        events[2]["payload_sha256"] = "0" * 64
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError, "payload digest mismatch"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_detects_tampered_event_digest(self) -> None:
        events = self._run_and_get_journal()
        # Change the event_sha256 directly to break the event digest check
        events[2]["event_sha256"] = "f" * 64
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError, "event digest mismatch"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_non_terminal_after_terminal(self) -> None:
        events = self._run_and_get_journal()
        # Move a non-terminal event after the terminal one
        extra = dict(events[2])
        extra["event_id"] = extra["event_id"].replace("-002-", "-099-")
        events.append(extra)
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError, "terminal event is not last"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_multiple_terminal_events(self) -> None:
        events = self._run_and_get_journal()
        # Duplicate the terminal event
        extra = dict(events[-1])
        extra["event_id"] = extra["event_id"].replace("-009-", "-099-")
        events.append(extra)
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "terminal event is not last"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_missing_terminal_event(self) -> None:
        events = self._run_and_get_journal()
        # Remove the terminal event (run_finished)
        events.pop()
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "exactly one terminal event"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_empty_journal(self) -> None:
        self._run_and_get_journal()
        journal_path = self.run_root / "events.jsonl"
        journal_path.write_text("", encoding="utf-8")
        with self.assertRaisesRegex(AssuranceError, "journal is empty"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_blank_journal_line(self) -> None:
        self._run_and_get_journal()
        journal_path = self.run_root / "events.jsonl"
        journal_path.write_text("\n\n", encoding="utf-8")
        with self.assertRaisesRegex(AssuranceError, "blank journal line"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    # -- Gate ordering negative tests ---------------------------------------

    def test_verifier_rejects_missing_ipg_event(self) -> None:
        events = self._run_and_get_journal()
        events = [e for e in events if e["event_type"] != "instruction_provenance_gate"]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "lacks instruction provenance gate event"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_missing_tool_check_event(self) -> None:
        events = self._run_and_get_journal()
        events = [e for e in events if e["event_type"] != "tool_availability_check"]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "lacks tool availability check event"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_missing_orientation_checkpoint(self) -> None:
        events = self._run_and_get_journal()
        events = [e for e in events if e["event_type"] != "orientation_checkpoint"]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "lacks orientation checkpoint event"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_ipg_after_run_started(self) -> None:
        events = self._run_and_get_journal()
        ipg = [e for e in events if e["event_type"] == "instruction_provenance_gate"][0]
        rs = [e for e in events if e["event_type"] == "run_started"][0]
        ipg["sequence"], rs["sequence"] = rs["sequence"], ipg["sequence"]
        # Re-order in the list so event_types reflect the swapped positions
        ipg_idx = events.index(ipg)
        rs_idx = events.index(rs)
        events[ipg_idx], events[rs_idx] = events[rs_idx], events[ipg_idx]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "must precede run_started"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_tool_check_before_ipg(self) -> None:
        events = self._run_and_get_journal()
        tac = [e for e in events if e["event_type"] == "tool_availability_check"][0]
        ipg = [e for e in events if e["event_type"] == "instruction_provenance_gate"][0]
        tac_idx = events.index(tac)
        ipg_idx = events.index(ipg)
        events[tac_idx], events[ipg_idx] = events[ipg_idx], events[tac_idx]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "must follow instruction provenance gate"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_orientation_before_tool_check(self) -> None:
        events = self._run_and_get_journal()
        oc = [e for e in events if e["event_type"] == "orientation_checkpoint"][0]
        tac = [e for e in events if e["event_type"] == "tool_availability_check"][0]
        oc_idx = events.index(oc)
        tac_idx = events.index(tac)
        events[oc_idx], events[tac_idx] = events[tac_idx], events[oc_idx]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "must follow tool availability check"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_missing_model_request(self) -> None:
        events = self._run_and_get_journal()
        events = [e for e in events if e["event_type"] != "model_request"]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "lacks model_request boundary event"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_verifier_rejects_model_request_before_gate_decision(self) -> None:
        events = self._run_and_get_journal()
        mr = [e for e in events if e["event_type"] == "model_request"][0]
        gd = [e for e in events if e["event_type"] == "gate_decision"][0]
        mr_idx = events.index(mr)
        gd_idx = events.index(gd)
        events[mr_idx], events[gd_idx] = events[gd_idx], events[mr_idx]
        self._renumber(events)
        self._write_journal(events)
        with self.assertRaisesRegex(AssuranceError,
                                    "before gate_decision"):
            verify_canonical_guarded_cli_run(run_root=self.run_root)

    def test_auto_built_ipg_context_passes_with_user_ask(self) -> None:
        receipt = run_canonical_guarded_cli(
            run_root=self.run_root,
            ask=ASK,
            source_ledger_path=SOURCE_LEDGER,
            run_id="RUN-AUTO-IPG-001",
            created_at=CREATED_AT,
        )
        self.assertTrue(receipt["valid"])
        self.assertIsNotNone(receipt["instruction_provenance_gate_context_sha256"])
        self.assertIsNotNone(receipt["instruction_provenance_gate_receipt_sha256"])
        self.assertTrue(receipt["checks"]["instruction_gate_before_model"])
        self.assertTrue(receipt["checks"]["tool_availability_gate_before_model"])

        event_types = receipt["event_types"]
        self.assertIn("instruction_provenance_gate", event_types)
        self.assertIn("tool_availability_check", event_types)
        ipg_idx = event_types.index("instruction_provenance_gate")
        gd_idx = event_types.index("gate_decision")
        self.assertLess(ipg_idx, gd_idx,
                        "instruction provenance gate must precede source visibility gate")

        verify_canonical_guarded_cli_run(run_root=self.run_root)
