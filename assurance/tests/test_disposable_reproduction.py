from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from assurance import (
    AssuranceError,
    build_disposable_reproduction_execution_lock,
    build_disposable_reproduction_run_proof,
    build_gsa_runtime_preflight_projection,
    inspect_gsa_runner_journal_recovery,
    replay_gsa_runner_journal,
    repair_gsa_runner_journal,
    resume_gsa_no_model_runner_skeleton,
    run_disposable_reproduction,
    run_gsa_no_model_runner_skeleton,
    verify_disposable_reproduction_receipt,
    verify_disposable_reproduction_execution_lock,
    verify_disposable_reproduction_run_proof,
    verify_gsa_no_model_runner_skeleton,
    verify_gsa_runtime_preflight_journal,
    verify_gsa_runtime_preflight_projection,
    write_gsa_runtime_preflight_journal,
)


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = (
    ROOT
    / "assurance"
    / "fixtures"
    / "general_science"
    / "computational_decay"
)


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class DisposableReproductionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name)
        self.bundle_root = self.run_root / "bundle"
        self.output_root = self.run_root / "reproduction-output"
        shutil.copytree(FIXTURE, self.bundle_root)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _source_hashes(self) -> dict[str, str]:
        return {
            path.relative_to(self.bundle_root).as_posix(): _sha256(path)
            for path in self.bundle_root.rglob("*")
            if path.is_file()
        }

    def _runtime_projection(self) -> dict[str, object]:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        return build_gsa_runtime_preflight_projection(
            proof,
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )

    def _runtime_journal_chain(
        self,
    ) -> tuple[
        dict[str, object],
        dict[str, object],
        dict[str, object],
        dict[str, object],
        Path,
    ]:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        projection = build_gsa_runtime_preflight_projection(
            proof,
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        journal_path = self.run_root / "runtime-journal/events.jsonl"
        journal_receipt = write_gsa_runtime_preflight_journal(
            projection,
            journal_path=journal_path,
        )
        return receipt, proof, projection, journal_receipt, journal_path

    def _execution_lock_chain(
        self,
    ) -> tuple[
        dict[str, object],
        dict[str, object],
        dict[str, object],
        dict[str, object],
        Path,
    ]:
        _, proof, projection, journal_receipt, journal_path = self._runtime_journal_chain()
        execution_lock = build_disposable_reproduction_execution_lock(
            proof,
            projection,
            journal_receipt,
            journal_path=journal_path,
        )
        return proof, projection, journal_receipt, execution_lock, journal_path

    def test_disposable_replay_writes_only_output_and_verifies(self) -> None:
        before = self._source_hashes()
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        after = self._source_hashes()

        self.assertEqual(before, after)
        self.assertTrue(receipt["valid"])
        self.assertTrue(receipt["source_snapshot_unchanged"])
        self.assertEqual(len(receipt["outputs"]), 2)
        self.assertEqual(
            receipt["evidence_boundary"]["independence_effect"],
            "no_new_independent_evidence",
        )
        self.assertEqual(
            receipt["evidence_boundary"]["claim_strength_effect"],
            "no_claim_promotion",
        )
        self.assertFalse(receipt["safety"]["model_invoked"])
        self.assertFalse(receipt["safety"]["network_requested"])
        self.assertFalse(receipt["safety"]["child_process_spawned"])

        fine = json.loads(
            (self.output_root / "replayed/result-dt-0.05.json").read_text(
                encoding="utf-8"
            )
        )
        coarse = json.loads(
            (self.output_root / "replayed/result-dt-0.1.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertLess(
            fine["run"]["final_absolute_error"],
            coarse["run"]["final_absolute_error"],
        )

        verification = verify_disposable_reproduction_receipt(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        self.assertTrue(verification["valid"])
        self.assertEqual(verification["verified_outputs"], 2)
        self.assertEqual(
            verification["evidence_boundary"]["independence_effect"],
            "no_new_independent_evidence",
        )

    def test_run_proof_binds_code_environment_inputs_outputs_and_journal(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )

        self.assertTrue(proof["valid"])
        self.assertEqual(proof["code"]["implementation"], "explicit_euler_decay_replay_v0.1")
        self.assertEqual(
            proof["code"]["files"][0]["path"],
            "assurance/disposable_reproduction.py",
        )
        self.assertEqual(len(proof["inputs"]), 4)
        self.assertEqual(len(proof["outputs"]), 2)
        events = proof["journal"]["events"]
        self.assertEqual(
            [item["event_type"] for item in events],
            ["run_preflight", "run_started", "run_finished"],
        )
        self.assertIsNone(events[0]["previous_event_sha256"])
        self.assertEqual(events[1]["previous_event_sha256"], events[0]["event_sha256"])
        self.assertEqual(events[2]["previous_event_sha256"], events[1]["event_sha256"])

        verification = verify_disposable_reproduction_run_proof(
            proof,
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        self.assertTrue(verification["valid"])
        self.assertEqual(verification["journal_event_count"], 3)
        self.assertEqual(
            verification["evidence_boundary"]["runner_status"],
            "development_fixture_only",
        )

    def test_run_proof_verifier_detects_hash_chain_tamper(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof["journal"]["events"][1]["previous_event_sha256"] = "0" * 64

        with self.assertRaisesRegex(AssuranceError, "run proof mismatch"):
            verify_disposable_reproduction_run_proof(
                proof,
                receipt,
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )

    def test_run_proof_verifier_detects_code_digest_tamper(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof["code"]["files"][0]["sha256"] = "1" * 64

        with self.assertRaisesRegex(AssuranceError, "run proof mismatch"):
            verify_disposable_reproduction_run_proof(
                proof,
                receipt,
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )

    def test_runtime_preflight_projection_builds_schema_valid_manifest_and_events(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        projection = build_gsa_runtime_preflight_projection(
            proof,
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )

        self.assertTrue(projection["valid"])
        manifest = projection["runtime_manifest"]
        self.assertEqual(manifest["run_id"], "RUN-GSA-DISPOSABLE-REPRODUCTION-PREFLIGHT")
        self.assertEqual(manifest["execution_mode"], "development")
        self.assertEqual(manifest["adapter"]["provider"], "none")
        self.assertEqual(manifest["adapter"]["model_id"], "no-model")
        self.assertEqual(manifest["tools"]["allowlist"], [])
        self.assertEqual(manifest["tools"]["network_profile"], "disabled")
        self.assertFalse(manifest["isolation"]["oracle_mounted"])
        events = projection["runtime_events"]
        self.assertEqual(
            [event["event_type"] for event in events],
            ["run_preflight", "run_started", "run_finished"],
        )
        self.assertTrue(all(event["redaction"] == "metadata_only" for event in events))
        self.assertIsNone(events[0]["previous_event_sha256"])
        self.assertEqual(events[1]["previous_event_sha256"], events[0]["event_sha256"])
        self.assertEqual(events[2]["previous_event_sha256"], events[1]["event_sha256"])
        self.assertNotIn("model_request", [event["event_type"] for event in events])
        self.assertNotIn("tool_started", [event["event_type"] for event in events])

        verification = verify_gsa_runtime_preflight_projection(
            projection,
            proof,
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        self.assertTrue(verification["valid"])
        self.assertEqual(verification["runtime_event_count"], 3)
        self.assertEqual(
            verification["evidence_boundary"]["runner_status"],
            "runtime_preflight_projection_only",
        )

    def test_runtime_preflight_projection_verifier_detects_event_tamper(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        projection = build_gsa_runtime_preflight_projection(
            proof,
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        projection["runtime_events"][1]["previous_event_sha256"] = "0" * 64

        with self.assertRaisesRegex(AssuranceError, "projection mismatch"):
            verify_gsa_runtime_preflight_projection(
                projection,
                proof,
                receipt,
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )

    def test_runtime_preflight_projection_verifier_detects_manifest_tamper(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        proof = build_disposable_reproduction_run_proof(
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        projection = build_gsa_runtime_preflight_projection(
            proof,
            receipt,
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        projection["runtime_manifest"]["adapter"]["model_id"] = "model"

        with self.assertRaisesRegex(AssuranceError, "projection mismatch"):
            verify_gsa_runtime_preflight_projection(
                projection,
                proof,
                receipt,
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )

    def test_runtime_preflight_journal_writes_jsonl_and_replays(self) -> None:
        projection = self._runtime_projection()
        journal_path = self.run_root / "runtime-journal/events.jsonl"
        receipt = write_gsa_runtime_preflight_journal(
            projection,
            journal_path=journal_path,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["event_count"], 3)
        self.assertEqual(receipt["terminal_event"], "run_finished")
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(lines), 3)
        self.assertEqual(
            [json.loads(line)["event_type"] for line in lines],
            ["run_preflight", "run_started", "run_finished"],
        )

        verification = verify_gsa_runtime_preflight_journal(
            receipt,
            projection,
            journal_path=journal_path,
        )
        self.assertTrue(verification["valid"])
        self.assertEqual(verification["verified_event_count"], 3)
        self.assertEqual(
            verification["evidence_boundary"]["runner_status"],
            "runtime_journal_disposable_write_replay_only",
        )

    def test_runtime_preflight_journal_verifier_detects_event_tamper(self) -> None:
        projection = self._runtime_projection()
        journal_path = self.run_root / "runtime-journal/events.jsonl"
        receipt = write_gsa_runtime_preflight_journal(
            projection,
            journal_path=journal_path,
        )
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        event = json.loads(lines[1])
        event["payload"]["model_invoked"] = True
        lines[1] = json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        journal_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "journal events do not match"):
            verify_gsa_runtime_preflight_journal(
                receipt,
                projection,
                journal_path=journal_path,
            )

    def test_runtime_preflight_journal_verifier_detects_receipt_tamper(self) -> None:
        projection = self._runtime_projection()
        journal_path = self.run_root / "runtime-journal/events.jsonl"
        receipt = write_gsa_runtime_preflight_journal(
            projection,
            journal_path=journal_path,
        )
        receipt["event_count"] = 2

        with self.assertRaisesRegex(AssuranceError, "was expected"):
            verify_gsa_runtime_preflight_journal(
                receipt,
                projection,
                journal_path=journal_path,
            )

    def test_runtime_preflight_journal_refuses_overwrite(self) -> None:
        projection = self._runtime_projection()
        journal_path = self.run_root / "runtime-journal/events.jsonl"
        write_gsa_runtime_preflight_journal(
            projection,
            journal_path=journal_path,
        )

        with self.assertRaisesRegex(AssuranceError, "already exists"):
            write_gsa_runtime_preflight_journal(
                projection,
                journal_path=journal_path,
            )

    def test_execution_lock_binds_code_environment_projection_and_journal(self) -> None:
        _, proof, projection, journal_receipt, journal_path = self._runtime_journal_chain()
        execution_lock = build_disposable_reproduction_execution_lock(
            proof,
            projection,
            journal_receipt,
            journal_path=journal_path,
        )

        self.assertTrue(execution_lock["valid"])
        self.assertEqual(
            execution_lock["evidence_boundary"]["runner_status"],
            "development_code_environment_lock_only",
        )
        self.assertGreaterEqual(len(execution_lock["selected_source_files"]), 10)
        self.assertEqual(
            execution_lock["environment"]["dependency_lock_status"],
            "python_distribution_snapshot_only",
        )
        self.assertEqual(
            [item["name"] for item in execution_lock["environment"]["python_distributions"]],
            ["jsonschema", "rfc8785"],
        )

        verification = verify_disposable_reproduction_execution_lock(
            execution_lock,
            proof,
            projection,
            journal_receipt,
            journal_path=journal_path,
        )
        self.assertTrue(verification["valid"])
        self.assertEqual(
            verification["locked_source_file_count"],
            len(execution_lock["selected_source_files"]),
        )

    def test_execution_lock_verifier_detects_source_digest_tamper(self) -> None:
        _, proof, projection, journal_receipt, journal_path = self._runtime_journal_chain()
        execution_lock = build_disposable_reproduction_execution_lock(
            proof,
            projection,
            journal_receipt,
            journal_path=journal_path,
        )
        execution_lock["selected_source_files"][0]["sha256"] = "0" * 64

        with self.assertRaisesRegex(AssuranceError, "execution lock mismatch"):
            verify_disposable_reproduction_execution_lock(
                execution_lock,
                proof,
                projection,
                journal_receipt,
                journal_path=journal_path,
            )

    def test_execution_lock_verifier_detects_journal_tamper(self) -> None:
        _, proof, projection, journal_receipt, journal_path = self._runtime_journal_chain()
        execution_lock = build_disposable_reproduction_execution_lock(
            proof,
            projection,
            journal_receipt,
            journal_path=journal_path,
        )
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        event = json.loads(lines[2])
        event["payload"]["valid"] = False
        lines[2] = json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        journal_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "journal events do not match"):
            verify_disposable_reproduction_execution_lock(
                execution_lock,
                proof,
                projection,
                journal_receipt,
                journal_path=journal_path,
            )

    def test_no_model_runner_skeleton_writes_manifest_journal_and_receipt(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        receipt = run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["event_count"], 3)
        self.assertEqual(receipt["terminal_event"], "run_finished")
        manifest_path = runner_root / "run-manifest.json"
        journal_path = runner_root / "run-journal.jsonl"
        self.assertEqual(_sha256(manifest_path), projection["run_manifest_sha256"])
        self.assertEqual(len(journal_path.read_text(encoding="utf-8").splitlines()), 3)
        self.assertTrue((runner_root / "run-journal.jsonl.lock").is_file())

        replay = replay_gsa_runner_journal(
            run_manifest_path=manifest_path,
            journal_path=journal_path,
        )
        self.assertTrue(replay["valid"])
        verification = verify_gsa_no_model_runner_skeleton(
            receipt,
            proof,
            projection,
            execution_lock,
            execution_lock_journal_path=lock_journal_path,
        )
        self.assertTrue(verification["valid"])
        self.assertEqual(
            verification["evidence_boundary"]["runner_status"],
            "no_model_runner_skeleton_only",
        )

    def test_runner_replay_allows_in_progress_journal_when_terminal_not_required(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        journal_path.write_text("\n".join(lines[:2]) + "\n", encoding="utf-8")

        strict = replay_gsa_runner_journal(
            run_manifest_path=runner_root / "run-manifest.json",
            journal_path=journal_path,
        )
        self.assertFalse(strict["valid"])
        relaxed = replay_gsa_runner_journal(
            run_manifest_path=runner_root / "run-manifest.json",
            journal_path=journal_path,
            require_terminal=False,
        )
        self.assertTrue(relaxed["valid"])
        self.assertEqual(relaxed["terminal_event"], None)

    def test_runner_recovery_inspection_classifies_torn_tail_read_only(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        original = journal_path.read_bytes()
        journal_path.write_bytes(original + b'{"partial":')

        inspection = inspect_gsa_runner_journal_recovery(
            run_manifest_path=runner_root / "run-manifest.json",
            journal_path=journal_path,
        )
        self.assertEqual(inspection["status"], "recoverable")
        self.assertEqual(inspection["classification"], "torn_tail_removed")
        self.assertEqual(inspection["prefix_event_count"], 3)
        self.assertGreater(inspection["discarded_size_bytes"], 0)

    def test_runner_journal_repair_normalizes_missing_newline(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        journal_path.write_bytes(journal_path.read_bytes().rstrip(b"\n"))
        receipt = repair_gsa_runner_journal(
            run_manifest_path=runner_root / "run-manifest.json",
            journal_path=journal_path,
            receipt_path=runner_root / "recovery-receipt.json",
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["classification"], "missing_newline_normalized")
        self.assertIsNone(receipt["quarantine_sha256"])
        self.assertTrue(journal_path.read_bytes().endswith(b"\n"))
        replay = replay_gsa_runner_journal(
            run_manifest_path=runner_root / "run-manifest.json",
            journal_path=journal_path,
        )
        self.assertTrue(replay["valid"])

    def test_runner_journal_repair_quarantines_torn_tail(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        journal_path.write_bytes(journal_path.read_bytes() + b'{"partial":')
        quarantine_path = runner_root / "run-journal.quarantine"
        receipt = repair_gsa_runner_journal(
            run_manifest_path=runner_root / "run-manifest.json",
            journal_path=journal_path,
            receipt_path=runner_root / "recovery-receipt.json",
            quarantine_path=quarantine_path,
        )

        self.assertEqual(receipt["classification"], "torn_tail_removed")
        self.assertEqual(_sha256(quarantine_path), receipt["quarantine_sha256"])
        self.assertEqual(receipt["discarded_size_bytes"], len(b'{"partial":'))
        replay = replay_gsa_runner_journal(
            run_manifest_path=runner_root / "run-manifest.json",
            journal_path=journal_path,
        )
        self.assertTrue(replay["valid"])

    def test_runner_journal_repair_refuses_history_corruption(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        event = json.loads(lines[0])
        event["run_id"] = "RUN-TAMPERED"
        lines[0] = json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        journal_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "repair refused"):
            repair_gsa_runner_journal(
                run_manifest_path=runner_root / "run-manifest.json",
                journal_path=journal_path,
                receipt_path=runner_root / "recovery-receipt.json",
            )

    def test_runner_lifecycle_resume_repairs_torn_tail_and_completes(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        journal_path.write_text("\n".join(lines[:2]) + "\n{\"partial\":", encoding="utf-8")
        policy = {
            "schema_version": "0.1.0-draft",
            "policy_kind": "gsa_runner_lifecycle_repair_policy",
            "mode": "repair_recoverable",
            "receipt_path": str(runner_root / "lifecycle-recovery-receipt.json"),
            "quarantine_path": str(runner_root / "lifecycle-recovery.quarantine"),
            "allow_missing_newline_normalization": True,
            "allow_torn_tail_quarantine": True,
            "valid": True,
        }

        receipt = resume_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            runner_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
            lifecycle_repair_policy=policy,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["event_count"], 3)
        self.assertEqual(receipt["terminal_event"], "run_finished")
        recovery_receipt = json.loads(
            (runner_root / "lifecycle-recovery-receipt.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(recovery_receipt["classification"], "torn_tail_removed")
        self.assertTrue((runner_root / "lifecycle-recovery.quarantine").is_file())

    def test_runner_lifecycle_resume_refuses_invalid_journal_without_policy(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        journal_path.write_text("\n".join(lines[:2]) + "\n{\"partial\":", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "invalid journal"):
            resume_gsa_no_model_runner_skeleton(
                proof,
                projection,
                execution_lock,
                runner_root=runner_root,
                execution_lock_journal_path=lock_journal_path,
            )

    def test_runner_lifecycle_resume_refuses_history_corruption_with_policy(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        event = json.loads(lines[0])
        event["run_id"] = "RUN-TAMPERED"
        lines[0] = json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        journal_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        policy = {
            "schema_version": "0.1.0-draft",
            "policy_kind": "gsa_runner_lifecycle_repair_policy",
            "mode": "repair_recoverable",
            "receipt_path": str(runner_root / "lifecycle-recovery-receipt.json"),
            "quarantine_path": str(runner_root / "lifecycle-recovery.quarantine"),
            "allow_missing_newline_normalization": True,
            "allow_torn_tail_quarantine": True,
            "valid": True,
        }

        with self.assertRaisesRegex(AssuranceError, "lifecycle repair refused"):
            resume_gsa_no_model_runner_skeleton(
                proof,
                projection,
                execution_lock,
                runner_root=runner_root,
                execution_lock_journal_path=lock_journal_path,
                lifecycle_repair_policy=policy,
            )

    def test_no_model_runner_verifier_detects_journal_tamper(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        receipt = run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )
        journal_path = runner_root / "run-journal.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        event = json.loads(lines[0])
        event["payload"]["code_file_count"] = 0
        lines[0] = json.dumps(
            event,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        journal_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "runner journal replay failed"):
            verify_gsa_no_model_runner_skeleton(
                receipt,
                proof,
                projection,
                execution_lock,
                execution_lock_journal_path=lock_journal_path,
            )

    def test_no_model_runner_verifier_detects_receipt_tamper(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        receipt = run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=self.run_root / "runner-output",
            execution_lock_journal_path=lock_journal_path,
        )
        receipt["event_count"] = 2

        with self.assertRaisesRegex(AssuranceError, "was expected"):
            verify_gsa_no_model_runner_skeleton(
                receipt,
                proof,
                projection,
                execution_lock,
                execution_lock_journal_path=lock_journal_path,
            )

    def test_no_model_runner_refuses_to_overwrite_runner_root(self) -> None:
        proof, projection, _, execution_lock, lock_journal_path = self._execution_lock_chain()
        runner_root = self.run_root / "runner-output"
        run_gsa_no_model_runner_skeleton(
            proof,
            projection,
            execution_lock,
            output_root=runner_root,
            execution_lock_journal_path=lock_journal_path,
        )

        with self.assertRaisesRegex(AssuranceError, "overwrite existing runner root"):
            run_gsa_no_model_runner_skeleton(
                proof,
                projection,
                execution_lock,
                output_root=runner_root,
                execution_lock_journal_path=lock_journal_path,
            )

    def test_output_root_inside_source_fails_closed(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "output_root"):
            run_disposable_reproduction(
                manifest_root=self.bundle_root,
                output_root=self.bundle_root / "generated",
            )

    def test_source_digest_mismatch_fails_before_output(self) -> None:
        design_path = self.bundle_root / "design.json"
        design = json.loads(design_path.read_text(encoding="utf-8"))
        design["question"] = "tampered"
        design_path.write_text(
            json.dumps(design, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        with self.assertRaisesRegex(AssuranceError, "digest mismatch"):
            run_disposable_reproduction(
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )
        self.assertFalse(self.output_root.exists())

    def test_verifier_detects_output_tamper(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        output_path = self.output_root / "replayed/result-dt-0.1.json"
        document = json.loads(output_path.read_text(encoding="utf-8"))
        document["run"]["final_absolute_error"] = 0.0
        output_path.write_text(
            json.dumps(document, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

        with self.assertRaisesRegex(AssuranceError, "output content differs"):
            verify_disposable_reproduction_receipt(
                receipt,
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )

    def test_verifier_detects_receipt_metadata_tamper(self) -> None:
        receipt = run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        receipt["outputs"][0]["path"] = "replayed/other.json"

        with self.assertRaisesRegex(AssuranceError, "output path mismatch"):
            verify_disposable_reproduction_receipt(
                receipt,
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )

    def test_runner_refuses_to_overwrite_existing_output(self) -> None:
        run_disposable_reproduction(
            manifest_root=self.bundle_root,
            output_root=self.output_root,
        )
        with self.assertRaisesRegex(AssuranceError, "overwrite output"):
            run_disposable_reproduction(
                manifest_root=self.bundle_root,
                output_root=self.output_root,
            )


if __name__ == "__main__":
    unittest.main()
