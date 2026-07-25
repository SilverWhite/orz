from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from prototype.fep_agent_proto.errors import PrototypeError
from prototype.fep_agent_proto.io_utils import sha256_file
from prototype.fep_agent_proto.journal import append_event, replay_journal
from prototype.fep_agent_proto.journal_lock import exclusive_journal_lock
from prototype.fep_agent_proto.journal_recovery import (
    inspect_journal_recovery,
    recover_journal,
)


RUN_ID = "RUN-RECOVERY-001"
MANIFEST_SHA256 = "a" * 64
ROOT = Path(__file__).resolve().parents[2]
RECOVERY_CLI = ROOT / "scripts" / "recover_torn_journal.py"


class JournalRecoveryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.journal = self.root / "events.jsonl"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _append(self, event_type: str, payload: dict | None = None) -> dict:
        return append_event(
            self.journal,
            run_id=RUN_ID,
            run_manifest_sha256=MANIFEST_SHA256,
            event_type=event_type,
            payload_schema="fixture/recovery-v0.1",
            payload=payload or {"event_type": event_type},
        )

    def _inspect(self) -> dict:
        return inspect_journal_recovery(
            self.journal,
            expected_run_id=RUN_ID,
            expected_manifest_sha256=MANIFEST_SHA256,
        )

    def _recover(
        self,
        *,
        receipt_name: str = "recovery-receipt.json",
        quarantine: bool = True,
    ) -> dict:
        return recover_journal(
            self.journal,
            expected_run_id=RUN_ID,
            expected_manifest_sha256=MANIFEST_SHA256,
            receipt_path=self.root / receipt_name,
            quarantine_path=(self.root / "torn-tail.bin") if quarantine else None,
        )

    def test_torn_tail_is_quarantined_recovered_and_appendable(self) -> None:
        first = self._append("run_started")
        torn = b'{"schema_version":"0.1.0-draft","run_id":"RUN-REC'
        with self.journal.open("ab") as handle:
            handle.write(torn)
            handle.flush()

        inspection = self._inspect()
        self.assertEqual(inspection["status"], "recoverable")
        self.assertEqual(inspection["classification"], "torn_tail_removed")
        self.assertEqual(inspection["prefix_last_event_sha256"], first["event_sha256"])
        receipt = self._recover()
        self.assertEqual((self.root / "torn-tail.bin").read_bytes(), torn)
        self.assertEqual(receipt["discarded_sha256"], sha256_file(self.root / "torn-tail.bin"))
        self.assertIsNotNone(receipt["recovery_event_sha256"])

        replay = replay_journal(
            run_manifest_path=None,
            journal_path=self.journal,
            expected_manifest_sha256=MANIFEST_SHA256,
            expected_run_id=RUN_ID,
            require_terminal=False,
        )
        self.assertTrue(replay["valid"])
        self.assertEqual(replay["event_count"], 2)
        events = [
            json.loads(line)
            for line in self.journal.read_text(encoding="utf-8").splitlines()
        ]
        self.assertEqual(events[-1]["payload"]["artifact_kind"], "journal-recovery-event")

        appended = self._append("tool_completed")
        self.assertEqual(appended["sequence"], 2)
        unchanged = sha256_file(self.journal)
        with self.assertRaisesRegex(PrototypeError, "valid_journal"):
            self._recover(receipt_name="second-receipt.json", quarantine=False)
        self.assertEqual(sha256_file(self.journal), unchanged)

    def test_missing_newline_preserves_complete_event(self) -> None:
        first = self._append("run_started")
        raw = self.journal.read_bytes()
        self.journal.write_bytes(raw[:-1])

        inspection = self._inspect()
        self.assertEqual(inspection["classification"], "missing_newline_normalized")
        receipt = self._recover(quarantine=False)
        self.assertEqual(receipt["discarded_size_bytes"], 0)
        self.assertIsNone(receipt["quarantine_sha256"])
        self.assertIsNotNone(receipt["recovery_event_sha256"])
        events = [
            json.loads(line)
            for line in self.journal.read_text(encoding="utf-8").splitlines()
        ]
        self.assertEqual(events[0]["event_sha256"], first["event_sha256"])
        self.assertEqual(len(events), 2)

    def test_middle_corruption_is_never_truncated(self) -> None:
        self._append("run_started")
        self._append("tool_completed")
        lines = self.journal.read_bytes().splitlines(keepends=True)
        damaged = lines[0] + b'{"broken":\n' + lines[1]
        self.journal.write_bytes(damaged)
        before = sha256_file(self.journal)

        inspection = self._inspect()
        self.assertEqual(inspection["status"], "unrecoverable")
        self.assertEqual(inspection["classification"], "history_corruption")
        with self.assertRaisesRegex(PrototypeError, "history_corruption"):
            self._recover()
        self.assertEqual(sha256_file(self.journal), before)
        self.assertFalse((self.root / "torn-tail.bin").exists())
        self.assertFalse((self.root / "recovery-receipt.json").exists())

    def test_terminal_prefix_discards_tail_without_appending_after_terminal(self) -> None:
        self._append("run_started")
        terminal = self._append("run_finished")
        torn = b'{"post_terminal_partial":'
        with self.journal.open("ab") as handle:
            handle.write(torn)

        receipt = self._recover()
        self.assertIsNone(receipt["recovery_event_sha256"])
        self.assertEqual(receipt["final_last_event_sha256"], terminal["event_sha256"])
        replay = replay_journal(
            run_manifest_path=None,
            journal_path=self.journal,
            expected_manifest_sha256=MANIFEST_SHA256,
            expected_run_id=RUN_ID,
            require_terminal=True,
        )
        self.assertTrue(replay["valid"])
        self.assertEqual(replay["event_count"], 2)

    def test_partial_first_event_has_no_recoverable_prefix(self) -> None:
        self.journal.write_bytes(b'{"schema_version":"0.1')
        before = sha256_file(self.journal)
        inspection = self._inspect()
        self.assertEqual(inspection["classification"], "no_valid_prefix")
        with self.assertRaisesRegex(PrototypeError, "no_valid_prefix"):
            self._recover()
        self.assertEqual(sha256_file(self.journal), before)

    def test_inspection_waits_for_the_shared_journal_lock(self) -> None:
        self._append("run_started")
        with exclusive_journal_lock(self.journal):
            with self.assertRaisesRegex(PrototypeError, "timed out acquiring journal lock"):
                inspect_journal_recovery(
                    self.journal,
                    expected_run_id=RUN_ID,
                    expected_manifest_sha256=MANIFEST_SHA256,
                    lock_timeout_seconds=0.05,
                )

    def test_nonfinite_corruption_is_reported_without_hash_rebuild_escape(self) -> None:
        self._append("run_started")
        with self.journal.open("ab") as handle:
            handle.write(b'{"payload":NaN}\n')
        inspection = self._inspect()
        self.assertEqual(inspection["status"], "unrecoverable")
        self.assertEqual(inspection["classification"], "history_corruption")
        self.assertTrue(
            any("canonicalization failed" in error for error in inspection["errors"])
        )

    def test_recovery_cli_inspects_then_applies_explicitly(self) -> None:
        self._append("run_started")
        self.journal.write_bytes(self.journal.read_bytes()[:-1])
        common = [
            sys.executable,
            str(RECOVERY_CLI),
            "--journal",
            str(self.journal),
            "--expected-run-id",
            RUN_ID,
            "--expected-manifest-sha256",
            MANIFEST_SHA256,
        ]
        inspected = subprocess.run(
            common,
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(inspected.returncode, 0, inspected.stderr)
        self.assertEqual(json.loads(inspected.stdout)["status"], "recoverable")
        receipt = self.root / "cli-recovery-receipt.json"
        applied = subprocess.run(
            [*common, "--apply", "--receipt", str(receipt)],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(applied.returncode, 0, applied.stderr)
        self.assertTrue(receipt.is_file())
        self.assertTrue(json.loads(applied.stdout)["valid"])


if __name__ == "__main__":
    unittest.main()
