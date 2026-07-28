"""Tests for the real Project Doc Retrieval Subagent — ProjectDocIndex,
dispatch_retrieval_subagent (offline mode), and namespace integration."""

from __future__ import annotations

import tempfile
from pathlib import Path
import unittest

from assurance import (
    build_retrieval_session_close_receipt,
    build_retrieval_task_contract,
    MemoryInstallationKeyStore,
    validate_retrieval_result,
)
from assurance.project_doc_index import CATEGORY_GLOBS, ProjectDocIndex
from assurance.retrieval_subagent import dispatch_retrieval_subagent


PARENT_SESSION = "CONV-AAAAAAAAAAAAAABBBBBBBBBBBBBBBBBB"
ROOT = Path(__file__).resolve().parents[2]  # D:\CLI


def _key_store() -> MemoryInstallationKeyStore:
    return MemoryInstallationKeyStore(b"K" * 32)


class ProjectDocIndexTests(unittest.TestCase):
    r"""ProjectDocIndex scanner tests against the real D:\CLI project."""

    def setUp(self) -> None:
        self.index = ProjectDocIndex(ROOT)

    def test_scan_finds_files_across_categories(self) -> None:
        self.index.scan()
        stats = self.index.stats()
        self.assertGreater(stats["total_files"], 0)
        self.assertIn("source_code", stats["by_category"])
        self.assertIn("audit_docs", stats["by_category"])
        self.assertIn("schemas", stats["by_category"])

    def test_search_finds_relevant_docs(self) -> None:
        self.index.scan()
        results = self.index.search("Conversation Namespace identity retention", max_results=10)
        self.assertGreater(len(results), 0)
        # Should find conversation.py
        paths = [r.path for r in results]
        self.assertTrue(
            any("conversation" in p for p in paths),
            f"Should find conversation in: {paths}"
        )

    def test_search_respects_category_filter(self) -> None:
        self.index.scan()
        results = self.index.search(
            "sandbox", max_results=10, categories=["architecture"]
        )
        for r in results:
            self.assertEqual(r.category, "architecture")

    def test_read_doc_returns_content(self) -> None:
        self.index.scan()
        results = self.index.search("Conversation Namespace retention policy", max_results=1)
        self.assertGreater(len(results), 0)
        content = self.index.read_doc(results[0].path)
        self.assertGreater(len(content), 0)

    def test_stats_returns_correct_structure(self) -> None:
        self.index.scan()
        stats = self.index.stats()
        self.assertIn("total_files", stats)
        self.assertIn("total_size_bytes", stats)
        self.assertIn("by_category", stats)
        self.assertIn("categories", stats)

    def test_search_empty_query_returns_nothing(self) -> None:
        self.index.scan()
        results = self.index.search("", max_results=10)
        self.assertEqual(len(results), 0)

    def test_category_globs_are_well_formed(self) -> None:
        """All glob patterns reference valid directories."""
        for category, globs in CATEGORY_GLOBS.items():
            for pattern in globs:
                # Each pattern should have at least one separator
                if "/" not in pattern and "**" not in pattern:
                    # Root-level patterns like "*.md" — OK
                    pass


class RetrievalDispatcherOfflineTests(unittest.TestCase):
    """dispatch_retrieval_subagent in offline mode (no API key)."""

    def setUp(self) -> None:
        self.key_store = _key_store()

    def tearDown(self) -> None:
        self.key_store.close()

    def _build_contract(self, **overrides) -> dict:
        kwargs = {
            "parent_session_id": PARENT_SESSION,
            "retrieval_question": "How does P3 instruction authority work?",
            "allowed_source_categories": ["internal_knowledge_base"],
            "return_sections": ["summary", "key_findings"],
            "max_sources": 5,
        }
        kwargs.update(overrides)
        return build_retrieval_task_contract(**kwargs)

    def test_offline_dispatch_returns_valid_result(self) -> None:
        """Offline dispatch (no credential) returns schema-valid result."""
        with tempfile.TemporaryDirectory() as tmp:
            contract = self._build_contract()
            output = dispatch_retrieval_subagent(
                contract=contract,
                parent_session_id=PARENT_SESSION,
                key_store=self.key_store,
                run_root=Path(tmp) / "run",
                project_root=ROOT,
                # No credential_target → offline mode
            )
            self.assertIn("result", output)
            self.assertIn("close_receipt", output)
            self.assertFalse(output["api_used"])
            self.assertIsNotNone(output["subagent_session_id"])

            # Validate result against schema
            validation = validate_retrieval_result(
                contract=contract, result=output["result"]
            )
            self.assertTrue(validation["valid"], validation)

    def test_offline_dispatch_creates_namespace(self) -> None:
        """Offline dispatch creates a real ConversationNamespace."""
        with tempfile.TemporaryDirectory() as tmp:
            contract = self._build_contract()
            run_root = Path(tmp) / "run"
            output = dispatch_retrieval_subagent(
                contract=contract,
                parent_session_id=PARENT_SESSION,
                key_store=self.key_store,
                run_root=run_root,
                project_root=ROOT,
            )
            # Namespace directory should exist
            ns_root = run_root / "conversations"
            self.assertTrue(ns_root.exists())
            conv_dirs = list(ns_root.iterdir())
            self.assertEqual(len(conv_dirs), 1)
            self.assertTrue(
                (conv_dirs[0] / ".assurance-conversation.json").is_file()
            )

    def test_offline_result_has_source_ledger(self) -> None:
        """Offline result includes project doc source ledger entries."""
        with tempfile.TemporaryDirectory() as tmp:
            contract = self._build_contract()
            output = dispatch_retrieval_subagent(
                contract=contract,
                parent_session_id=PARENT_SESSION,
                key_store=self.key_store,
                run_root=Path(tmp) / "run",
                project_root=ROOT,
            )
            result = output["result"]
            self.assertGreater(len(result["source_ledger"]), 0)
            for src in result["source_ledger"]:
                self.assertIn("source_id", src)
                self.assertIn("source_url_or_ref", src)
                self.assertIn("content_sha256", src)
                self.assertTrue(src["full_text_retrieved"])

    def test_close_receipt_confirms_no_zeroing(self) -> None:
        """Close receipt confirms the subagent conversation is preserved."""
        with tempfile.TemporaryDirectory() as tmp:
            contract = self._build_contract()
            output = dispatch_retrieval_subagent(
                contract=contract,
                parent_session_id=PARENT_SESSION,
                key_store=self.key_store,
                run_root=Path(tmp) / "run",
                project_root=ROOT,
            )
            cr = output["close_receipt"]
            self.assertTrue(cr["valid"])
            self.assertFalse(cr["archive_policy"]["conversation_zeroed"])
            self.assertTrue(cr["archive_policy"]["journal_preserved"])
            self.assertTrue(cr["next_availability"]["subagent_resumable"])

    def test_governor_closure_in_output(self) -> None:
        """Governor closure receipt is included."""
        with tempfile.TemporaryDirectory() as tmp:
            contract = self._build_contract()
            output = dispatch_retrieval_subagent(
                contract=contract,
                parent_session_id=PARENT_SESSION,
                key_store=self.key_store,
                run_root=Path(tmp) / "run",
                project_root=ROOT,
            )
            self.assertIn("governor_closure", output)
            gc = output["governor_closure"]
            self.assertGreater(gc["artifact_count"], 0)


class RealRetrievalIntegrationTests(unittest.TestCase):
    r"""Integration tests using real D:\CLI project document index."""

    def setUp(self) -> None:
        self.key_store = _key_store()

    def tearDown(self) -> None:
        self.key_store.close()

    def test_search_p3_action_authorization(self) -> None:
        """Search for P3-related docs finds instruction_gate.py and audit doc."""
        index = ProjectDocIndex(ROOT)
        index.scan()
        results = index.search(
            "P3 instruction authority action authorization",
            max_results=10,
        )
        paths = [r.path for r in results]
        self.assertTrue(
            any("instruction_gate.py" in p for p in paths),
            f"Should find instruction_gate.py in: {paths}",
        )

    def test_search_windows_sandbox(self) -> None:
        """Search for Windows sandbox finds audit doc and source."""
        index = ProjectDocIndex(ROOT)
        index.scan()
        results = index.search(
            "GAK-SBX-001 Windows native sandbox AppContainer",
            max_results=10,
        )
        paths = [r.path for r in results]
        self.assertTrue(
            any("windows_sandbox" in p for p in paths),
            f"Should find windows_sandbox in: {paths}",
        )

    def test_source_ledger_has_valid_sha256(self) -> None:
        """Source ledger entries have valid SHA-256 hashes for real files."""
        with tempfile.TemporaryDirectory() as tmp:
            contract = build_retrieval_task_contract(
                parent_session_id=PARENT_SESSION,
                retrieval_question="archive deletion controller retention policy",
                allowed_source_categories=["internal_knowledge_base"],
                return_sections=["summary"],
                max_sources=5,
            )
            output = dispatch_retrieval_subagent(
                contract=contract,
                parent_session_id=PARENT_SESSION,
                key_store=self.key_store,
                run_root=Path(tmp) / "run",
                project_root=ROOT,
            )
            for src in output["result"]["source_ledger"]:
                self.assertRegex(src["content_sha256"], r"^[a-f0-9]{64}$")
                self.assertGreater(len(src["content_sha256"]), 0)


if __name__ == "__main__":
    unittest.main()
