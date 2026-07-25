from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO

from assurance import AssuranceError, review_general_science_bundle
from assurance.artifact_registry import (
    load_general_science_artifact_registry,
    validate_artifact_registry_semantics,
)
from assurance.general_science_cli import main as review_cli_main
from assurance.validator_bridge import (
    load_general_science_validator_registry,
    validate_validator_registry_semantics,
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


class GeneralScienceReviewTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name) / "bundle"
        shutil.copytree(FIXTURE, self.root)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _load_bundle(self) -> dict[str, object]:
        return json.loads(
            (self.root / "review-bundle.json").read_text(encoding="utf-8")
        )

    def _write_bundle(self, bundle: dict[str, object]) -> None:
        (self.root / "review-bundle.json").write_text(
            json.dumps(
                bundle,
                ensure_ascii=False,
                indent=2,
                sort_keys=True,
                allow_nan=False,
            )
            + "\n",
            encoding="utf-8",
        )

    def _rewrite_json_source(
        self,
        filename: str,
        transform: object,
        *,
        record_id: str,
    ) -> None:
        path = self.root / filename
        document = json.loads(path.read_text(encoding="utf-8"))
        assert callable(transform)
        transform(document)
        path.write_text(
            json.dumps(document, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        bundle = self._load_bundle()
        records = [*bundle["sources"], *bundle["artifacts"]]
        record = next(item for item in records if item["record_id"] == record_id)
        record["sha256"] = _sha256(path)
        self._write_bundle(bundle)

    def test_checked_in_non_lif_fixture_allows_only_narrow_observation(
        self,
    ) -> None:
        before = {
            path.name: _sha256(path)
            for path in self.root.iterdir()
            if path.is_file()
        }
        first = review_general_science_bundle(bundle_root=self.root)
        second = review_general_science_bundle(bundle_root=self.root)
        after = {
            path.name: _sha256(path)
            for path in self.root.iterdir()
            if path.is_file()
        }

        self.assertEqual(first, second)
        self.assertEqual(before, after)
        self.assertEqual(first["decision"], "allow")
        self.assertEqual(len(first["files"]), 6)
        self.assertEqual(len(first["validator_results"]), 9)
        self.assertEqual(
            [
                item["status"]
                for item in first["validator_results"]
            ],
            [
                "pass",
                "pass",
                "pass",
                "pass",
                "pass",
                "pass",
                "not_applicable",
                "not_applicable",
                "pass",
            ],
        )
        claim = first["claim_decisions"][0]
        self.assertEqual(claim["decision"], "allow")
        self.assertEqual(
            claim["reason_codes"], ["DIRECT_OBSERVATION_SUPPORTED"]
        )
        comparison = claim["verified_comparisons"][0]
        self.assertEqual(
            comparison["left_artifact_id"], "ART_DECAY_DT_0_05"
        )
        self.assertEqual(
            comparison["right_artifact_id"], "ART_DECAY_DT_0_1"
        )
        self.assertLess(comparison["left_value"], comparison["right_value"])
        self.assertFalse(first["safety"]["source_write_attempted"])
        self.assertFalse(first["safety"]["analysis_code_executed"])
        self.assertFalse(first["safety"]["model_invoked"])
        self.assertFalse(first["safety"]["network_requested"])
        self.assertFalse(first["safety"]["child_process_spawned"])

    def test_cli_emits_the_structured_result_without_writing_files(self) -> None:
        before = {
            path.name: _sha256(path)
            for path in self.root.iterdir()
            if path.is_file()
        }
        output = StringIO()
        with redirect_stdout(output):
            exit_code = review_cli_main(
                ["--bundle-root", str(self.root)]
            )
        result = json.loads(output.getvalue())
        after = {
            path.name: _sha256(path)
            for path in self.root.iterdir()
            if path.is_file()
        }
        self.assertEqual(exit_code, 0)
        self.assertEqual(result["decision"], "allow")
        self.assertEqual(before, after)

    def test_tampered_artifact_digest_fails_closed(self) -> None:
        (self.root / "result-dt-0.05.json").write_text(
            '{"tampered": true}\n', encoding="utf-8"
        )
        with self.assertRaisesRegex(AssuranceError, "digest mismatch"):
            review_general_science_bundle(bundle_root=self.root)

    def test_path_escape_and_nonfinite_artifact_fail_closed(self) -> None:
        bundle = self._load_bundle()
        bundle["sources"][0]["path"] = "../outside.md"
        self._write_bundle(bundle)
        with self.assertRaises(AssuranceError):
            review_general_science_bundle(bundle_root=self.root)

        shutil.rmtree(self.root)
        shutil.copytree(FIXTURE, self.root)
        bundle = self._load_bundle()
        bundle["sources"][0]["artifact_kind"] = "numerical_result"
        self._write_bundle(bundle)
        with self.assertRaises(AssuranceError):
            review_general_science_bundle(bundle_root=self.root)

        shutil.rmtree(self.root)
        shutil.copytree(FIXTURE, self.root)
        result_path = self.root / "result-dt-0.05.json"
        result = json.loads(result_path.read_text(encoding="utf-8"))
        result["run"]["final_absolute_error"] = float("nan")
        result_path.write_text(
            json.dumps(result, allow_nan=True) + "\n", encoding="utf-8"
        )
        bundle = self._load_bundle()
        fine_artifact = next(
            item
            for item in bundle["artifacts"]
            if item["record_id"] == "ART_DECAY_DT_0_05"
        )
        fine_artifact["sha256"] = _sha256(result_path)
        self._write_bundle(bundle)
        with self.assertRaisesRegex(AssuranceError, "non-finite"):
            review_general_science_bundle(bundle_root=self.root)

    def test_unknown_bridge_support_defers_instead_of_promoting(self) -> None:
        bundle = self._load_bundle()
        evidence = next(
            item
            for item in bundle["evidence"]
            if item["evidence_id"] == "EV_DECAY_ERROR_LT"
        )
        evidence["evidence_class"] = "bridge_hypothesis"
        evidence["status"] = "unknown"
        del evidence["comparison"]
        self._write_bundle(bundle)

        result = review_general_science_bundle(bundle_root=self.root)
        claim = result["claim_decisions"][0]
        self.assertEqual(result["decision"], "defer")
        self.assertEqual(claim["decision"], "defer")
        self.assertIn("UNCERTAIN_EVIDENCE_CLASS", claim["reason_codes"])
        self.assertIn("SUPPORT_NOT_OBSERVED", claim["reason_codes"])

    def test_statistical_summary_requires_basic_reporting_fields(self) -> None:
        bundle = self._load_bundle()
        bundle["artifacts"][0]["artifact_kind"] = "statistical_summary"
        bundle["artifacts"][0][
            "artifact_schema_id"
        ] = "GSAS_STATISTICAL_SUMMARY_BASE_0_1"
        self._write_bundle(bundle)

        incomplete = review_general_science_bundle(bundle_root=self.root)
        statistical = next(
            item
            for item in incomplete["validator_results"]
            if item["validator_id"] == "GSV_STATISTICAL_REPORTING"
        )
        self.assertEqual(incomplete["decision"], "defer")
        self.assertEqual(statistical["status"], "defer")
        self.assertEqual(
            statistical["reason_codes"], ["EVD-COVERAGE-001"]
        )

        def add_reporting(document: dict[str, object]) -> None:
            document["statistical_reporting"] = {
                "analysis_role": "confirmatory",
                "sampling_unit": "synthetic independent run",
                "sample_size": 20,
                "effect_estimate": 0.25,
                "uncertainty": {
                    "kind": "confidence_interval",
                    "level": 0.95,
                    "lower": 0.1,
                    "upper": 0.4,
                },
                "missing_data": {
                    "count": 0,
                    "handling": "not applicable; no observations missing",
                },
                "multiplicity": {
                    "comparison_count": 1,
                    "adjustment": "not applicable for one comparison",
                },
                "stopping_rule": "fixed sample size of 20",
            }

        self._rewrite_json_source(
            "result-dt-0.1.json",
            add_reporting,
            record_id="ART_DECAY_DT_0_1",
        )
        bundle = self._load_bundle()
        bundle["artifacts"][0]["artifact_kind"] = "statistical_summary"
        bundle["artifacts"][0][
            "artifact_schema_id"
        ] = "GSAS_STATISTICAL_SUMMARY_BASE_0_1"
        self._write_bundle(bundle)
        complete = review_general_science_bundle(bundle_root=self.root)
        statistical = next(
            item
            for item in complete["validator_results"]
            if item["validator_id"] == "GSV_STATISTICAL_REPORTING"
        )
        self.assertEqual(complete["decision"], "allow")
        self.assertEqual(statistical["status"], "pass")

    def test_validator_registry_rejects_semantic_duplicate_ids(self) -> None:
        invalid = (
            ROOT
            / "assurance"
            / "fixtures"
            / "general_science"
            / "validator-registry.duplicate-id.semantic-invalid.json"
        )
        with self.assertRaisesRegex(AssuranceError, "IDs must be unique"):
            load_general_science_validator_registry(invalid)

        weakened = deepcopy(load_general_science_validator_registry())
        finite = next(
            item
            for item in weakened["validators"]
            if item["validator_id"] == "GSV_ARTIFACT_FINITE_JSON"
        )
        finite["failure_decision"] = "defer"
        with self.assertRaisesRegex(AssuranceError, "weakens or changes"):
            validate_validator_registry_semantics(weakened)

    def test_artifact_schema_registration_fails_closed(self) -> None:
        bundle = self._load_bundle()
        bundle["artifacts"][0]["artifact_schema_id"] = "GSAS_UNKNOWN"
        self._write_bundle(bundle)
        with self.assertRaisesRegex(AssuranceError, "validator bridge blocked"):
            review_general_science_bundle(bundle_root=self.root)

        shutil.rmtree(self.root)
        shutil.copytree(FIXTURE, self.root)

        def remove_required_run(document: dict[str, object]) -> None:
            del document["run"]

        self._rewrite_json_source(
            "result-dt-0.1.json",
            remove_required_run,
            record_id="ART_DECAY_DT_0_1",
        )
        with self.assertRaisesRegex(AssuranceError, "validator bridge blocked"):
            review_general_science_bundle(bundle_root=self.root)

        invalid = (
            ROOT
            / "assurance"
            / "fixtures"
            / "general_science"
            / "artifact-registry.duplicate-id.semantic-invalid.json"
        )
        with self.assertRaisesRegex(AssuranceError, "IDs must be unique"):
            load_general_science_artifact_registry(invalid)

        weakened = deepcopy(load_general_science_artifact_registry())
        numerical = next(
            item
            for item in weakened["artifact_schemas"]
            if item["artifact_schema_id"]
            == "GSAS_NUMERICAL_TIME_STEP_0_1"
        )
        numerical["artifact_kind"] = "other_json"
        with self.assertRaisesRegex(AssuranceError, "weakens or changes"):
            validate_artifact_registry_semantics(weakened)

    def test_cross_artifact_comparability_mismatch_defers(self) -> None:
        def change_unit(document: dict[str, object]) -> None:
            document["comparison_context"]["unit"] = "UNIT_SECONDS"

        self._rewrite_json_source(
            "result-dt-0.05.json",
            change_unit,
            record_id="ART_DECAY_DT_0_05",
        )
        result = review_general_science_bundle(bundle_root=self.root)
        comparability = next(
            item
            for item in result["validator_results"]
            if item["validator_id"]
            == "GSV_CROSS_ARTIFACT_COMPARABILITY"
        )
        self.assertEqual(result["decision"], "defer")
        self.assertEqual(comparability["status"], "defer")
        self.assertEqual(
            comparability["reason_codes"], ["EVD-COMPARABILITY-001"]
        )
        self.assertIn(
            "comparison_context.unit differs", comparability["details"]
        )

        shutil.rmtree(self.root)
        shutil.copytree(FIXTURE, self.root)

        def change_declared_condition(document: dict[str, object]) -> None:
            document["comparison_context"]["condition"]["value"] = 0.04

        self._rewrite_json_source(
            "result-dt-0.05.json",
            change_declared_condition,
            record_id="ART_DECAY_DT_0_05",
        )
        result = review_general_science_bundle(bundle_root=self.root)
        comparability = next(
            item
            for item in result["validator_results"]
            if item["validator_id"]
            == "GSV_CROSS_ARTIFACT_COMPARABILITY"
        )
        self.assertEqual(comparability["status"], "defer")
        self.assertIn(
            "left condition value disagrees with artifact",
            comparability["details"],
        )

        shutil.rmtree(self.root)
        shutil.copytree(FIXTURE, self.root)

        def drop_lineage_source(document: dict[str, object]) -> None:
            document["comparison_context"]["lineage"]["source_ids"] = [
                "SRC_DECAY_NOTES"
            ]

        self._rewrite_json_source(
            "result-dt-0.05.json",
            drop_lineage_source,
            record_id="ART_DECAY_DT_0_05",
        )
        result = review_general_science_bundle(bundle_root=self.root)
        comparability = next(
            item
            for item in result["validator_results"]
            if item["validator_id"]
            == "GSV_CROSS_ARTIFACT_COMPARABILITY"
        )
        self.assertEqual(comparability["status"], "defer")
        self.assertIn(
            "left lineage sources do not match action manifest",
            comparability["details"],
        )

    def test_mechanism_claim_requires_controlled_intervention(self) -> None:
        bundle = self._load_bundle()
        bundle["claims"][0]["claim_type"] = "mechanism"
        self._write_bundle(bundle)

        result = review_general_science_bundle(bundle_root=self.root)
        claim = result["claim_decisions"][0]
        self.assertEqual(result["decision"], "defer")
        self.assertIn(
            "CONTROLLED_INTERVENTION_REQUIRED", claim["reason_codes"]
        )
        self.assertIn(
            "STRONG_CLAIM_VALIDATOR_UNAVAILABLE", claim["reason_codes"]
        )

    def test_missing_support_blocks_and_terminal_multiplicity_is_invalid(
        self,
    ) -> None:
        bundle = self._load_bundle()
        bundle["claims"][0]["supporting_evidence_ids"] = []
        self._write_bundle(bundle)
        result = review_general_science_bundle(bundle_root=self.root)
        self.assertEqual(result["decision"], "block")
        self.assertEqual(
            result["claim_decisions"][0]["reason_codes"],
            ["MISSING_SUPPORT"],
        )

        shutil.rmtree(self.root)
        shutil.copytree(FIXTURE, self.root)

        def duplicate_terminal(document: dict[str, object]) -> None:
            document["terminal_event_count"] = 2

        self._rewrite_json_source(
            "run-manifest.json",
            duplicate_terminal,
            record_id="SRC_DECAY_RUN",
        )
        with self.assertRaisesRegex(AssuranceError, "validator bridge blocked"):
            review_general_science_bundle(bundle_root=self.root)


if __name__ == "__main__":
    unittest.main()
