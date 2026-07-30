"""Tests for ScenarioExporter — anonymous scenario export from regression corpus."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from assurance.errors import AssuranceError
from assurance.scenario_exporter import (
    DEFAULT_CORPUS_PATH,
    ExportedScenario,
    ExportResult,
    Partition,
    ScenarioExporter,
    export_scenarios,
)
from assurance.utils import sha256_bytes


# ── minimal test corpus ──────────────────────────────────────────────────────


def _write_test_corpus(path: Path, cases: list[dict] | None = None) -> str:
    """Write a minimal regression corpus YAML file and return its path as str."""
    import yaml

    if cases is None:
        cases = [
            {
                "case_id": "FEP-REG-001",
                "title": "Test case one",
                "case_kind": "historical",
                "partition": "development",
                "evidence_tier": "discussion_grounded",
                "curation_status": "curated_initial",
                "classification": {
                    "primary": "provenance",
                    "secondary": ["configuration"],
                    "stage": "source_reconstruction",
                    "bias_tags": ["confirmation_bias"],
                    "shortcut_tags": ["trust_cli_omission_as_absence"],
                    "trigger_features": ["missing_cli_flag"],
                },
                "sources": [
                    {
                        "path": "R8_DISCUSSION.md",
                        "line_start": 1,
                        "line_end": 10,
                        "role": "correction_source",
                    },
                ],
                "scenario": {
                    "task": "Determine whether the flag was set.",
                    "visible_facts": [
                        "The launch command did not contain --flag.",
                        "The model output is available.",
                    ],
                    "hidden_from_subject": [
                        "Historical final ruling.",
                    ],
                },
                "oracle": {
                    "expected_gate_decisions": [
                        {"gate_id": "EVD-PROVENANCE-001", "decision": "defer"},
                    ],
                    "required_questions": [
                        "Does the runtime define a default?",
                    ],
                    "expected_state": {
                        "action": "succeeded",
                        "evidence": "source_grounded",
                        "claim": "eligible",
                        "claim_type": "correction",
                    },
                    "allowed_claims": ["The flag had a default."],
                    "forbidden_claims": ["The flag was absent."],
                    "correction_summary": "Runtime default existed.",
                },
                "countercase_ids": [],
            },
            {
                "case_id": "FEP-SYN-001",
                "title": "Synthetic challenge case",
                "case_kind": "synthetic_countercase",
                "partition": "challenge",
                "evidence_tier": "synthetic",
                "curation_status": "curated_initial",
                "classification": {
                    "primary": "claim_strength",
                    "secondary": [],
                    "stage": "result_interpretation",
                    "bias_tags": ["surface_analogy"],
                    "shortcut_tags": ["promote_proxy_to_mechanism"],
                    "trigger_features": ["surface_similarity"],
                },
                "sources": [],
                "scenario": {
                    "task": "Evaluate whether correlation implies causation.",
                    "visible_facts": [
                        "X and Y are correlated (r=0.8).",
                        "No intervention was performed.",
                    ],
                    "hidden_from_subject": [
                        "Confounding variable Z drives both.",
                    ],
                },
                "oracle": {
                    "expected_gate_decisions": [
                        {"gate_id": "EVD-CONFOUND-001", "decision": "defer"},
                    ],
                    "required_questions": [
                        "Is there a plausible confounder?",
                    ],
                    "expected_state": {
                        "action": "succeeded",
                        "evidence": "degraded",
                        "claim": "deferred",
                        "claim_type": "causal",
                    },
                    "allowed_claims": ["X and Y are associated."],
                    "forbidden_claims": ["X causes Y."],
                    "correction_summary": "No causal evidence.",
                },
                "countercase_ids": ["FEP-REG-001"],
            },
        ]

    corpus = {
        "schema_version": "0.1.0-draft",
        "corpus_id": "FEP-AGENT-REGRESSION-TEST",
        "created_at": "2026-07-30",
        "revised_at": "2026-07-30",
        "corpus_revision": 1,
        "status": "test",
        "revision_history": [
            {"revision": 1, "date": "2026-07-30", "summary": "Test corpus."},
        ],
        "purpose": "Test corpus for ScenarioExporter tests.",
        "corpus_policy": {
            "default_partition": "development",
            "oracle_default_mode": "guarded",
            "blind_first_required": True,
            "oracle_visible_to_subject": False,
            "retrieval_allowed_only_after_precommitment": True,
            "canonical_labels_require_review": True,
            "historical_cases_are_not_current_task_evidence": True,
            "evaluation_and_holdout_storage": "not_yet_created",
            "notes": ["Test only."],
        },
        "taxonomy": {
            "primary": ["provenance"],
            "bias_tags": ["confirmation_bias"],
            "shortcut_tags": ["trust_cli_omission_as_absence"],
        },
        "reason_codes": ["EVD-PROVENANCE-001"],
        "cases": cases,
    }
    with open(path, "w", encoding="utf-8") as fh:
        yaml.safe_dump(corpus, fh, allow_unicode=True, sort_keys=False)
    return str(path)


# ══════════════════════════════════════════════════════════════════════════════
# 1. Basic export
# ══════════════════════════════════════════════════════════════════════════════


class BasicExportTests(unittest.TestCase):
    """Tests for basic export functionality."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._corpus_path = Path(self._tmp.name) / "test_corpus.yaml"
        _write_test_corpus(self._corpus_path)
        self._export_root = Path(self._tmp.name) / "exports"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_export_all_cases(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export()
        self.assertEqual(result.exported_count, 2)
        self.assertEqual(result.total_cases_in_corpus, 2)
        self.assertEqual(len(result.scenarios), 2)

    def test_export_filters_by_partition(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export(partitions={Partition.DEVELOPMENT})
        self.assertEqual(result.exported_count, 1)
        self.assertEqual(result.scenarios[0].case_id, "FEP-REG-001")

    def test_export_filters_by_case_ids(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export(case_ids=["FEP-SYN-001"])
        self.assertEqual(result.exported_count, 1)
        self.assertEqual(result.scenarios[0].case_id, "FEP-SYN-001")

    def test_export_writes_scenario_files(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export()
        export_dir = self._export_root / result.export_id
        self.assertTrue(export_dir.is_dir())
        self.assertTrue((export_dir / "FEP-REG-001.json").is_file())
        self.assertTrue((export_dir / "FEP-SYN-001.json").is_file())
        self.assertTrue((export_dir / "export_manifest.json").is_file())


# ══════════════════════════════════════════════════════════════════════════════
# 2. Redaction — oracle and metadata must be stripped
# ══════════════════════════════════════════════════════════════════════════════


class RedactionTests(unittest.TestCase):
    """Tests that oracle data and metadata are stripped from exports."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._corpus_path = Path(self._tmp.name) / "test_corpus.yaml"
        _write_test_corpus(self._corpus_path)
        self._export_root = Path(self._tmp.name) / "exports"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_scenario_file_has_no_oracle(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export()
        export_dir = self._export_root / result.export_id
        with open(export_dir / "FEP-REG-001.json", "r", encoding="utf-8") as fh:
            data = json.load(fh)
        self.assertNotIn("oracle", data)
        self.assertNotIn("expected_gate_decisions", data)
        self.assertNotIn("allowed_claims", data)
        self.assertNotIn("forbidden_claims", data)
        self.assertNotIn("correction_summary", data)

    def test_scenario_file_has_no_classification(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export()
        export_dir = self._export_root / result.export_id
        with open(export_dir / "FEP-REG-001.json", "r", encoding="utf-8") as fh:
            data = json.load(fh)
        self.assertNotIn("classification", data)
        self.assertNotIn("sources", data)
        self.assertNotIn("countercase_ids", data)
        self.assertNotIn("partition", data)

    def test_scenario_contains_only_visible_facts(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export()
        export_dir = self._export_root / result.export_id
        with open(export_dir / "FEP-REG-001.json", "r", encoding="utf-8") as fh:
            data = json.load(fh)
        # Keep-list fields
        self.assertIn("case_id", data)
        self.assertIn("title", data)
        self.assertIn("task", data)
        self.assertIn("visible_facts", data)
        self.assertIn("content_sha256", data)
        # Hidden from subject must not appear
        self.assertNotIn("hidden_from_subject", data)

    def test_exported_scenario_has_no_hidden_facts(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export(case_ids=["FEP-REG-001"])
        scenario = result.scenarios[0]
        self.assertNotIn("Historical final ruling", scenario.visible_facts)
        self.assertNotIn("hidden_from_subject", scenario.visible_facts)


# ══════════════════════════════════════════════════════════════════════════════
# 3. Fail-closed — unexpected fields
# ══════════════════════════════════════════════════════════════════════════════


class FailClosedTests(unittest.TestCase):
    """Tests that unexpected fields block export (fail-closed)."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_unexpected_top_level_field_blocks(self) -> None:
        corpus_path = Path(self._tmp.name) / "bad_corpus.yaml"
        _write_test_corpus(corpus_path)
        # Modify the YAML to add an unexpected field
        import yaml
        with open(corpus_path, "r", encoding="utf-8") as fh:
            corpus = yaml.safe_load(fh)
        corpus["cases"][0]["new_oracle_field"] = "leaked data"
        with open(corpus_path, "w", encoding="utf-8") as fh:
            yaml.safe_dump(corpus, fh, allow_unicode=True, sort_keys=False)

        exporter = ScenarioExporter(
            corpus_path=corpus_path,
            export_root=Path(self._tmp.name) / "exports",
            leak_scan=False,
        )
        result = exporter.export(case_ids=["FEP-REG-001"])
        self.assertEqual(result.exported_count, 0)
        self.assertGreater(result.skipped_count, 0)
        self.assertIn("FEP-REG-001", result.skip_reasons[0])

    def test_unexpected_scenario_field_blocks(self) -> None:
        corpus_path = Path(self._tmp.name) / "bad_corpus2.yaml"
        _write_test_corpus(corpus_path)
        import yaml
        with open(corpus_path, "r", encoding="utf-8") as fh:
            corpus = yaml.safe_load(fh)
        corpus["cases"][0]["scenario"]["internal_notes"] = "secret"
        with open(corpus_path, "w", encoding="utf-8") as fh:
            yaml.safe_dump(corpus, fh, allow_unicode=True, sort_keys=False)

        exporter = ScenarioExporter(
            corpus_path=corpus_path,
            export_root=Path(self._tmp.name) / "exports",
            leak_scan=False,
        )
        result = exporter.export(case_ids=["FEP-REG-001"])
        self.assertEqual(result.exported_count, 0)
        self.assertGreater(result.skipped_count, 0)


# ══════════════════════════════════════════════════════════════════════════════
# 4. Convenience function
# ══════════════════════════════════════════════════════════════════════════════


class ConvenienceFunctionTests(unittest.TestCase):
    """Tests for export_scenarios()."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._corpus_path = Path(self._tmp.name) / "test_corpus.yaml"
        _write_test_corpus(self._corpus_path)
        self._export_root = Path(self._tmp.name) / "exports"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_export_scenarios_returns_result(self) -> None:
        result = export_scenarios(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            verify_no_leaks=False,
        )
        self.assertIsInstance(result, ExportResult)
        self.assertEqual(result.exported_count, 2)

    def test_export_scenarios_with_leak_scan_enabled(self) -> None:
        # The test corpus has no credentials, so leak scan should pass.
        result = export_scenarios(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            verify_no_leaks=True,
        )
        self.assertTrue(result.leak_scan_clean)


# ══════════════════════════════════════════════════════════════════════════════
# 5. Content integrity
# ══════════════════════════════════════════════════════════════════════════════


class ContentIntegrityTests(unittest.TestCase):
    """Tests for content hashing and integrity."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._corpus_path = Path(self._tmp.name) / "test_corpus.yaml"
        _write_test_corpus(self._corpus_path)
        self._export_root = Path(self._tmp.name) / "exports"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_each_scenario_has_content_hash(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export()
        for scenario in result.scenarios:
            self.assertEqual(len(scenario.content_sha256), 64)
            self.assertTrue(
                all(c in "0123456789abcdef" for c in scenario.content_sha256)
            )

    def test_manifest_has_content_root_hash(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        result = exporter.export()
        export_dir = self._export_root / result.export_id
        with open(export_dir / "export_manifest.json", "r", encoding="utf-8") as fh:
            manifest = json.load(fh)
        self.assertIn("content_root_hash", manifest)
        self.assertEqual(len(manifest["content_root_hash"]), 64)

    def test_same_content_produces_same_hash(self) -> None:
        exporter = ScenarioExporter(
            corpus_path=self._corpus_path,
            export_root=self._export_root,
            leak_scan=False,
        )
        r1 = exporter.export(case_ids=["FEP-REG-001"])
        r2 = exporter.export(case_ids=["FEP-REG-001"])
        self.assertEqual(
            r1.scenarios[0].content_sha256,
            r2.scenarios[0].content_sha256,
        )


if __name__ == "__main__":
    unittest.main()
