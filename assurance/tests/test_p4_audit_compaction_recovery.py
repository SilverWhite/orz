from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import tempfile
import unittest

from assurance import (
    ArchiveController,
    AssuranceError,
    AuditLedger,
    ConversationNamespace,
    MemoryInstallationKeyStore,
    RecoveryExecutor,
    ShadowRecoveryStore,
    authorize_recovery_candidate,
    create_recovery_candidate,
    issue_sensitive_action_permit,
    recovery_permit_binding,
    verify_audit_seal,
    verify_recovery_candidate,
)
from assurance.utils import canonical_bytes, sha256_bytes


def _evidenced(value: str) -> dict[str, object]:
    return {
        "value": value,
        "evidence_status": "observed",
        "source_refs": ["fixture:p4"],
    }


def _context(label: str) -> dict[str, object]:
    return {
        "workspace_canonical_path_digest": _evidenced("a" * 64),
        "workspace_content_digest": _evidenced("b" * 64),
        "workspace_policy_digest": _evidenced("c" * 64),
        "runtime_family": _evidenced("runtime-neutral-test"),
        "runtime_adapter_id": _evidenced(label),
        "runtime_binary_digest": _evidenced("d" * 64),
        "runtime_capabilities_digest": _evidenced("e" * 64),
        "assurance_config_digest": _evidenced("f" * 64),
        "sandbox_backend": _evidenced("p4-test-sandbox"),
        "sandbox_backend_digest": _evidenced("1" * 64),
    }


class P4AuditCompactionRecoveryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.key_store = MemoryInstallationKeyStore()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _namespace(self, name: str) -> ConversationNamespace:
        return ConversationNamespace.create(
            self.root / name,
            key_store=self.key_store,
            frozen_context=_context(name),
            allowed_capabilities=[
                "filesystem.workspace_read",
                "filesystem.workspace_write",
                "recovery.apply_candidate",
            ],
            denied_capabilities=[
                "credential.raw_read",
                "network.unrestricted",
                "secret.raw_read",
            ],
        )

    def _sealed_source(
        self,
    ) -> tuple[ConversationNamespace, dict[str, object]]:
        namespace = self._namespace("source")
        namespace.write_artifact(
            "user_pinned_snapshot",
            "checkpoint.bin",
            b"PINNED-RECOVERY-CHECKPOINT",
        )
        ledger = AuditLedger(
            namespace=namespace,
            key_store=self.key_store,
        )
        ledger.append(
            source_kind="acp",
            source_id="acp:session",
            source_sequence=0,
            source_completeness="complete",
            canonical_event_type="session.started",
            raw_payload=b'{"method":"session/new","prompt":"omitted"}',
            raw_retention_category="full_stdout_stderr",
            provenance_status="direct",
            source_refs=["fixture:p4/acp"],
            facts={"protocol": "acp_v1", "accepted": True},
        )
        ledger.append(
            source_kind="session",
            source_id="session:local",
            source_sequence=0,
            source_completeness="partial",
            canonical_event_type="context.compacted",
            raw_payload=b"PRIVATE-SOURCE-SPAN-AND-DERIVED-SUMMARY",
            raw_retention_category="private_reasoning",
            provenance_status="derived_unverified",
            source_refs=["fixture:p4/session"],
            facts={"trigger": "manual", "source_items": 4},
            compaction={
                "summary_status": "derived_unverified",
                "summary_sha256": sha256_bytes(b"DERIVED-SUMMARY"),
                "source_span_sha256": sha256_bytes(b"PRIVATE-SOURCE-SPAN"),
                "ranges": {
                    "retained": [
                        {
                            "first_index": 0,
                            "last_index": 1,
                            "reason_code": "PINNED_SOURCE_REFERENCE",
                        }
                    ],
                    "discarded": [
                        {
                            "first_index": 2,
                            "last_index": 2,
                            "reason_code": "RAW_PAYLOAD_ARCHIVE_DELETE",
                        }
                    ],
                    "unknown": [
                        {
                            "first_index": 3,
                            "last_index": 3,
                            "reason_code": "AUTOMATIC_THRESHOLD_OBSERVED",
                        }
                    ],
                },
            },
        )
        ledger.append(
            source_kind="provider",
            source_id="provider:fake",
            source_sequence=0,
            source_completeness="complete",
            canonical_event_type="provider.completed",
            raw_payload=b'{"private_model_output":"omitted"}',
            raw_retention_category="raw_provider_payload",
            provenance_status="direct",
            source_refs=["fixture:p4/provider"],
            facts={"status": "succeeded", "request_count": 1},
        )
        ledger.append(
            source_kind="supervisor",
            source_id="supervisor:local",
            source_sequence=0,
            source_completeness="unknown",
            canonical_event_type="run.terminal",
            raw_payload=b"RAW-PROCESS-STDOUT-AND-STDERR",
            raw_retention_category="full_stdout_stderr",
            provenance_status="unknown",
            source_refs=["fixture:p4/supervisor"],
            facts={"exit_code": 0, "residue_count": 0},
            terminal_outcome="succeeded",
        )
        seal = ledger.seal(
            source_completeness={
                "acp": "complete",
                "session": "partial",
                "provider": "complete",
                "supervisor": "unknown",
            }
        )
        return namespace, seal

    def test_metadata_ledger_survives_raw_archive_and_rebuilds_compaction(
        self,
    ) -> None:
        namespace, seal = self._sealed_source()
        active = verify_audit_seal(
            seal,
            namespace=namespace,
            key_store=self.key_store,
        )
        self.assertTrue(active["valid"], active["errors"])
        self.assertEqual(active["event_count"], 4)
        result = ArchiveController(key_store=self.key_store).archive(namespace)
        self.assertTrue(result["archive_complete"])
        archived = verify_audit_seal(
            seal,
            namespace=namespace,
            key_store=self.key_store,
            require_archive_complete=True,
        )
        self.assertTrue(archived["valid"], archived["errors"])
        self.assertTrue(archived["raw_payload_absence_proven"])
        self.assertEqual(archived["terminal_outcome"], "succeeded")
        self.assertFalse(
            (namespace.artifacts_root / "raw_provider_payload").exists()
        )
        self.assertFalse(
            (namespace.artifacts_root / "private_reasoning").exists()
        )
        self.assertTrue(
            (
                namespace.artifacts_root
                / "redacted_conversation"
                / "audit"
            ).is_dir()
        )
        self.assertTrue(
            (
                namespace.artifacts_root
                / "user_pinned_snapshot"
                / "checkpoint.bin"
            ).is_file()
        )

    def test_seal_requires_one_final_terminal_and_detects_journal_tampering(
        self,
    ) -> None:
        incomplete = self._namespace("incomplete")
        ledger = AuditLedger(
            namespace=incomplete,
            key_store=self.key_store,
        )
        ledger.append(
            source_kind="kernel",
            source_id="kernel:local",
            source_sequence=0,
            source_completeness="complete",
            canonical_event_type="run.started",
            raw_payload=b"START",
            raw_retention_category="full_stdout_stderr",
            provenance_status="direct",
            source_refs=["fixture:p4/kernel"],
        )
        with self.assertRaisesRegex(AssuranceError, "exactly one final"):
            ledger.seal(
                source_completeness={
                    "acp": "unknown",
                    "session": "unknown",
                    "provider": "unknown",
                    "supervisor": "unknown",
                    "kernel": "complete",
                }
            )

        namespace, seal = self._sealed_source()
        relative = seal["journal"]["relative_path"]
        path = namespace.root / Path(*relative.split("/"))
        path.write_bytes(path.read_bytes() + b"{}\n")
        verification = verify_audit_seal(
            seal,
            namespace=namespace,
            key_store=self.key_store,
        )
        self.assertFalse(verification["valid"])
        self.assertTrue(
            any(
                "journal artifact mismatch" in item
                for item in verification["errors"]
            )
        )

    def test_recovery_candidate_requires_separate_exact_permit_and_never_restores(
        self,
    ) -> None:
        source, seal = self._sealed_source()
        ArchiveController(key_store=self.key_store).archive(source)
        destination = self._namespace("destination")
        target_sha = sha256_bytes(b"D:\\DISPOSABLE\\RECOVERY-TARGET")
        candidate = create_recovery_candidate(
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
            snapshot_relative_path="checkpoint.bin",
            target_workspace_sha256=target_sha,
        )
        self.assertFalse(candidate["decision"]["action_authorized"])
        self.assertFalse(candidate["decision"]["restoration_performed"])
        candidate_verification = verify_recovery_candidate(
            candidate,
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
        )
        self.assertTrue(
            candidate_verification["valid"],
            candidate_verification["errors"],
        )

        denied, denied_consumed = authorize_recovery_candidate(
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
            candidate=candidate,
            attempt=1,
            issued_permit=None,
        )
        self.assertEqual(denied["decision"]["outcome"], "deny")
        self.assertIsNone(denied_consumed)

        binding = recovery_permit_binding(candidate, attempt=1)
        issued = issue_sensitive_action_permit(
            namespace=destination,
            key_store=self.key_store,
            confirmation_sha256=sha256_bytes(
                b"confirm exact recovery candidate"
            ),
            action_sha256=binding["action_sha256"],
            target_sha256=binding["target_workspace_sha256"],
            impact_scope_sha256=binding["snapshot_sha256"],
            attempt=binding["attempt"],
        )
        allowed, consumed = authorize_recovery_candidate(
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
            candidate=candidate,
            attempt=1,
            issued_permit=issued,
        )
        self.assertEqual(allowed["decision"]["outcome"], "allow")
        self.assertFalse(allowed["decision"]["restoration_performed"])
        self.assertIsNotNone(consumed)

        replay, replay_consumed = authorize_recovery_candidate(
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
            candidate=candidate,
            attempt=1,
            issued_permit=issued,
        )
        self.assertEqual(replay["decision"]["outcome"], "deny")
        self.assertIsNone(replay_consumed)

        tampered = deepcopy(candidate)
        tampered["source"]["snapshot_sha256"] = "0" * 64
        tampered_verification = verify_recovery_candidate(
            tampered,
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
        )
        self.assertFalse(tampered_verification["valid"])

    # ── GAK-REC-001: shadow recovery store + executor integration ──────

    def test_shadow_store_and_execute_cycle(self) -> None:
        """Store a recovery candidate in the shadow store, then execute."""
        source, seal = self._sealed_source()
        ArchiveController(key_store=self.key_store).archive(source)
        destination = self._namespace("dest-exec")

        target_sha = sha256_bytes(b"D:\\DISPOSABLE\\SHADOW-EXEC-TARGET")
        candidate = create_recovery_candidate(
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
            snapshot_relative_path="checkpoint.bin",
            target_workspace_sha256=target_sha,
        )

        binding = recovery_permit_binding(candidate, attempt=1)
        permit = issue_sensitive_action_permit(
            namespace=destination,
            key_store=self.key_store,
            confirmation_sha256=sha256_bytes(b"confirm shadow exec test"),
            action_sha256=binding["action_sha256"],
            target_sha256=binding["target_workspace_sha256"],
            impact_scope_sha256=binding["snapshot_sha256"],
            attempt=binding["attempt"],
        )
        auth, _consumed = authorize_recovery_candidate(
            source_namespace=source,
            destination_namespace=destination,
            key_store=self.key_store,
            audit_seal=seal,
            candidate=candidate,
            attempt=1,
            issued_permit=permit,
        )
        self.assertEqual(auth["decision"]["outcome"], "allow")

        # Store in shadow store
        store = ShadowRecoveryStore()
        snapshot_data = source.read_artifact(
            source.conversation_id,
            "user_pinned_snapshot",
            "checkpoint.bin",
        )
        self.assertIsNotNone(snapshot_data)
        entry = store.store(candidate, auth, snapshot_data)
        self.assertTrue(store.verify_entry(entry.entry_sha256))

        # Execute recovery to a target file
        import tempfile
        executor = RecoveryExecutor()
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / "restored_checkpoint.bin"
            receipt = executor.execute(
                candidate, auth, snapshot_data, str(target), self.key_store,
            )
            self.assertEqual(receipt.outcome, "restored")
            self.assertTrue(receipt.audit_events_preserved)
            self.assertEqual(receipt.bytes_written, len(snapshot_data))
            self.assertTrue(target.exists())
            self.assertEqual(target.read_bytes(), snapshot_data)


if __name__ == "__main__":
    unittest.main()
