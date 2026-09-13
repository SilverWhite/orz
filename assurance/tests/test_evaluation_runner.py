"""Tests for EvaluationRunner — frozen evaluation with oracle isolation.

The emitted document must validate against the registered contract
``evaluation/evaluation-result-v0.1.schema.json``; that is asserted here rather
than assumed (GAP-EVAL-RESULT-SCHEMA-DRIFT, 2026-09-13).
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from assurance.errors import AssuranceError
from assurance.evaluation_runner import (
    EvaluationJournal,
    EvaluationRunner,
    FrozenSystemProfile,
    ScenarioResponse,
    build_evaluation_oracle_bundle,
    file_sha256,
    load_coverage_matrix,
    protocol_ref_block,
)
from assurance.utils import sha256_bytes

REPO_ROOT = Path(__file__).resolve().parents[2]
RESULT_SCHEMA = REPO_ROOT / "evaluation" / "evaluation-result-v0.1.schema.json"


def _validate_result(result: dict) -> list[str]:
    """Return contract violations of *result* (empty list = conformant)."""
    import jsonschema

    schema = json.loads(RESULT_SCHEMA.read_text(encoding="utf-8"))
    validator = jsonschema.Draft202012Validator(schema, format_checker=jsonschema.FormatChecker())
    return [
        f"{'/'.join(str(p) for p in e.absolute_path) or '<root>'}: {e.message}"
        for e in validator.iter_errors(result)
    ]


# ── helpers ──────────────────────────────────────────────────────────────────


def _make_profile(**overrides) -> FrozenSystemProfile:
    kwargs = {
        "model_id": "test-model",
        "model_parameters": {"temperature": 0.0},
        "prompt_digest": sha256_bytes(b"test system prompt"),
        "tool_allowlist": ["read", "search"],
        "network_policy": "none",
        "budget_seconds": 60,
        "mode": "guarded",
        "seed": 42,
    }
    kwargs.update(overrides)
    return FrozenSystemProfile(**kwargs)


def _make_scenario_bundle() -> list[dict]:
    # Case ids must follow the corpus format fixed by the result schema.
    return [
        {
            "case_id": "FEP-REG-001",
            "title": "Test case one",
            "task": "Evaluate claim X.",
            "visible_facts": ["Fact A", "Fact B"],
        },
        {
            "case_id": "FEP-REG-002",
            "title": "Test case two",
            "task": "Evaluate claim Y.",
            "visible_facts": ["Fact C"],
        },
    ]


def _make_oracle_bundle() -> list[dict]:
    return [
        {
            "case_id": "FEP-REG-001",
            "title": "Test case one",
            "expected_gate_decisions": [
                {"gate_id": "EVD-PROVENANCE-001", "decision": "defer"},
            ],
            "required_questions": ["Where is the source?"],
            "expected_state": {
                "action": "succeeded", "evidence": "source_grounded",
                "claim": "eligible", "claim_type": "correction",
            },
            "allowed_claims": ["Claim is supported by source."],
            "forbidden_claims": ["Claim is definitely true."],
            "correction_summary": "Check the source first.",
        },
        {
            "case_id": "FEP-REG-002",
            "title": "Test case two",
            "expected_gate_decisions": [
                {"gate_id": "CLM-STRENGTH-001", "decision": "defer"},
            ],
            "required_questions": ["What is the evidence strength?"],
            "expected_state": {
                "action": "succeeded", "evidence": "degraded",
                "claim": "deferred", "claim_type": "hypothesis",
            },
            "allowed_claims": ["X and Y are correlated."],
            "forbidden_claims": ["X causes Y."],
            "correction_summary": "No causal evidence.",
        },
    ]


def _make_response(case_id: str, *, precommitment: bool = True) -> ScenarioResponse:
    return ScenarioResponse(
        case_id=case_id,
        proposed_gates={"EVD-PROVENANCE-001": "defer"},
        action_state="succeeded",
        evidence_state="source_grounded",
        claim_state="eligible",
        claim_type="correction",
        candidate_claims=["Claim is supported by source."],
        claim_strength="full_text_grounded_but_unvalidated",
        blind_precommitment="Precommit: need source check" if precommitment else "",
        precommitment_timestamp="2026-07-30T00:00:00Z" if precommitment else "",
    )


# ══════════════════════════════════════════════════════════════════════════════
# 1. FrozenSystemProfile
# ══════════════════════════════════════════════════════════════════════════════


class FrozenProfileTests(unittest.TestCase):
    """Tests for FrozenSystemProfile."""

    def test_profile_is_immutable(self) -> None:
        profile = _make_profile()
        with self.assertRaises(Exception):
            profile.model_id = "changed"  # type: ignore[misc]

    def test_profile_digest_is_deterministic(self) -> None:
        p1 = _make_profile()
        p2 = _make_profile()
        self.assertEqual(p1.profile_digest, p2.profile_digest)

    def test_different_profiles_have_different_digests(self) -> None:
        p1 = _make_profile()
        p2 = _make_profile(model_id="different-model")
        self.assertNotEqual(p1.profile_digest, p2.profile_digest)


# ══════════════════════════════════════════════════════════════════════════════
# 2. Oracle isolation
# ══════════════════════════════════════════════════════════════════════════════


class OracleIsolationTests(unittest.TestCase):
    """Tests that oracle data never leaks into scenario context."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._output_dir = Path(self._tmp.name)
        self._profile = _make_profile()

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_scenario_bundle_has_no_oracle_fields(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        feed = runner.scenario_feed()
        for scenario in feed:
            self.assertNotIn("expected_gate_decisions", scenario)
            self.assertNotIn("allowed_claims", scenario)
            self.assertNotIn("forbidden_claims", scenario)
            self.assertNotIn("correction_summary", scenario)
            self.assertNotIn("required_questions", scenario)

    def test_oracle_leakage_in_scenario_blocked(self) -> None:
        bad_scenarios = [
            {
                "case_id": "FEP-TEST-001",
                "title": "Bad scenario",
                "task": "test",
                "visible_facts": ["fact"],
                "expected_gate_decisions": [{"gate_id": "X", "decision": "block"}],
            },
        ]
        with self.assertRaises(AssuranceError) as ctx:
            EvaluationRunner(
                profile=self._profile,
                scenario_bundle=bad_scenarios,
                oracle_bundle=_make_oracle_bundle(),
                output_dir=self._output_dir,
            )
        self.assertIn("ORACLE LEAKAGE", str(ctx.exception))

    def test_scenario_oracle_mismatch_blocked(self) -> None:
        bad_oracles = [_make_oracle_bundle()[0]]  # only one oracle, two scenarios
        with self.assertRaises(AssuranceError) as ctx:
            EvaluationRunner(
                profile=self._profile,
                scenario_bundle=_make_scenario_bundle(),
                oracle_bundle=bad_oracles,
                output_dir=self._output_dir,
            )
        self.assertIn("mismatch", str(ctx.exception))


# ══════════════════════════════════════════════════════════════════════════════
# 3. Journal
# ══════════════════════════════════════════════════════════════════════════════


class JournalTests(unittest.TestCase):
    """Tests for EvaluationJournal."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._journal_path = Path(self._tmp.name) / "test_journal.jsonl"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_journal_appends_entries(self) -> None:
        journal = EvaluationJournal(journal_path=self._journal_path)
        journal.append({"event": "test", "data": 1})
        journal.append({"event": "test", "data": 2})
        self.assertEqual(len(journal.entries), 2)
        self.assertEqual(journal.entries[0]["sequence"], 0)
        self.assertEqual(journal.entries[1]["sequence"], 1)

    def test_journal_persists_to_disk(self) -> None:
        journal = EvaluationJournal(journal_path=self._journal_path)
        journal.append({"event": "test"})
        self.assertTrue(self._journal_path.is_file())
        with open(self._journal_path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        self.assertEqual(len(lines), 1)
        data = json.loads(lines[0])
        self.assertEqual(data["event"], "test")

    def test_journal_head_sha256_changes_with_new_entries(self) -> None:
        journal = EvaluationJournal(journal_path=self._journal_path)
        h0 = journal.head_sha256()
        journal.append({"event": "test"})
        h1 = journal.head_sha256()
        self.assertNotEqual(h0, h1)


# ══════════════════════════════════════════════════════════════════════════════
# 4. Full evaluation flow
# ══════════════════════════════════════════════════════════════════════════════


class EvaluationFlowTests(unittest.TestCase):
    """Tests for the complete evaluation flow."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._output_dir = Path(self._tmp.name)
        self._profile = _make_profile()

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_full_flow_produces_result(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )

        # Feed scenarios
        feed = runner.scenario_feed()
        self.assertEqual(len(feed), 2)

        # Record responses
        for scenario in feed:
            response = _make_response(scenario["case_id"])
            runner.record_response(response)

        # Finalize
        result = runner.finalize()
        self.assertEqual(result["status"], "descriptive_only")
        self.assertEqual(result["counts"]["planned_cases"], 2)
        self.assertEqual(result["counts"]["completed_cases"], 2)
        self.assertEqual(result["counts"]["valid_cases"], 2)
        self.assertTrue(result["integrity"]["valid"])
        check_ids = {c["check_id"] for c in result["integrity"]["checks"]}
        self.assertIn("INT-ORACLE-ISOLATION", check_ids)
        self.assertEqual(result["acceptance"]["status"], "not_calibrated")
        self.assertIsNone(result["acceptance"]["threshold_set"])
        self.assertEqual(_validate_result(result), [])

    def test_finalize_without_all_responses_raises(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        response = _make_response("FEP-REG-001")
        runner.record_response(response)
        with self.assertRaises(AssuranceError):
            runner.finalize()

    def test_duplicate_response_raises(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        response = _make_response("FEP-REG-001")
        runner.record_response(response)
        with self.assertRaises(AssuranceError):
            runner.record_response(response)

    def test_unknown_case_id_raises(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        response = _make_response("UNKNOWN-CASE")
        with self.assertRaises(AssuranceError):
            runner.record_response(response)

    def test_result_written_to_disk(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        for scenario in runner.scenario_feed():
            runner.record_response(_make_response(scenario["case_id"]))
        runner.finalize()

        result_path = self._output_dir / "evaluation_result.json"
        self.assertTrue(result_path.is_file())
        with open(result_path, "r", encoding="utf-8") as fh:
            data = json.load(fh)
        self.assertEqual(data["evaluation_id"], runner.evaluation_id)

    def test_journal_written_to_disk(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        for scenario in runner.scenario_feed():
            runner.record_response(_make_response(scenario["case_id"]))
        runner.finalize()

        journal_path = self._output_dir / "evaluation_journal.jsonl"
        self.assertTrue(journal_path.is_file())
        with open(journal_path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        # init + 2 responses + completed = 4 entries
        self.assertGreaterEqual(len(lines), 4)

    def test_red_line_no_forbidden_claims_triggered(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        for scenario in runner.scenario_feed():
            response = _make_response(scenario["case_id"])
            response.candidate_claims = ["Claim is supported by source."]
            runner.record_response(response)
        result = runner.finalize()
        kinds = [r["kind"] for r in result["red_lines"]]
        self.assertNotIn("forbidden_claim", kinds)
        self.assertEqual(_validate_result(result), [])

    def test_red_line_forbidden_claims_triggered(self) -> None:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
        )
        for scenario in runner.scenario_feed():
            response = _make_response(scenario["case_id"])
            response.candidate_claims = ["Claim is definitely true."]
            runner.record_response(response)
        result = runner.finalize()
        forbidden = [r for r in result["red_lines"] if r["kind"] == "forbidden_claim"]
        # Only case one forbids "Claim is definitely true."; case two forbids a
        # different sentence, so exactly one case is flagged.
        self.assertEqual(len(forbidden), 1)
        self.assertEqual(forbidden[0]["case_id"], "FEP-REG-001")
        self.assertEqual(forbidden[0]["evidence_refs"][0], forbidden[0]["case_id"])
        self.assertEqual(_validate_result(result), [])


class ResultContractTests(unittest.TestCase):
    """The emitted document must satisfy the registered result contract."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self._output_dir = Path(self._tmp.name)
        self._profile = _make_profile()

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def _run(self, **runner_kwargs) -> dict:
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
            **runner_kwargs,
        )
        for scenario in runner.scenario_feed():
            runner.record_response(_make_response(scenario["case_id"]))
        return runner.finalize()

    def test_result_matches_registered_schema(self) -> None:
        self.assertEqual(_validate_result(self._run()), [])

    def test_unknown_case_id_format_is_rejected(self) -> None:
        """A bundle that cannot produce a schema-valid document fails closed."""
        bad = [{
            "case_id": "CASE-001",
            "title": "off-contract id",
            "task": "t",
            "visible_facts": [],
        }]
        bad_oracle = [{
            "case_id": "CASE-001",
            "expected_gate_decisions": [],
            "required_questions": [],
            "expected_state": {},
            "allowed_claims": [],
            "forbidden_claims": [],
            "correction_summary": "",
        }]
        with self.assertRaises(AssuranceError) as ctx:
            EvaluationRunner(
                profile=self._profile,
                scenario_bundle=bad,
                oracle_bundle=bad_oracle,
                output_dir=self._output_dir,
            )
        self.assertIn("case_id", str(ctx.exception))

    def test_corpus_revision_must_be_positive(self) -> None:
        with self.assertRaises(AssuranceError):
            EvaluationRunner(
                profile=self._profile,
                scenario_bundle=_make_scenario_bundle(),
                oracle_bundle=_make_oracle_bundle(),
                output_dir=self._output_dir,
                corpus_revision=0,
            )

    def test_no_result_ever_claims_comparability(self) -> None:
        """SCORING_PROTOCOL §9: no sealed split ⇒ never eligible_for_comparison."""
        result = self._run()
        self.assertNotEqual(result["status"], "eligible_for_comparison")
        self.assertEqual(result["acceptance"]["status"], "not_calibrated")
        self.assertNotIn(result["acceptance"]["status"], ("pass", "fail"))

    def test_protocol_ref_hashes_registered_artifacts(self) -> None:
        block = protocol_ref_block()
        self.assertEqual(
            block["scoring_protocol_sha256"],
            file_sha256(REPO_ROOT / "evaluation" / "SCORING_PROTOCOL_v0.1.md"),
        )
        self.assertEqual(
            block["coverage_matrix_sha256"],
            file_sha256(REPO_ROOT / "regression" / "coverage-matrix-v0.1.yaml"),
        )
        self.assertEqual(len(block["gate_matrix_sha256"]), 64)

    def test_coverage_matrix_pairs_carry_contrast_type(self) -> None:
        """A pair without a registered contrast type fails closed, not silently."""
        runner = EvaluationRunner(
            profile=self._profile,
            scenario_bundle=_make_scenario_bundle(),
            oracle_bundle=_make_oracle_bundle(),
            output_dir=self._output_dir,
            pair_map={"FEP-REG-001": "FEP-REG-002"},
        )
        for scenario in runner.scenario_feed():
            runner.record_response(_make_response(scenario["case_id"]))
        with self.assertRaises(AssuranceError):
            runner.finalize()

    def test_load_coverage_matrix_is_cluster_and_pair_authority(self) -> None:
        matrix = load_coverage_matrix()
        self.assertEqual(matrix["matrix_revision"], 3)
        distinct_clusters = {
            cluster for clusters in matrix["cluster_map"].values() for cluster in clusters
        }
        self.assertEqual(len(distinct_clusters), 10)
        # The seed corpus has overlapping clusters (SCORING_PROTOCOL §2).
        self.assertTrue(any(len(v) > 1 for v in matrix["cluster_map"].values()))
        self.assertTrue(matrix["pair_map"])
        for challenge_id, contrast in matrix["contrast_types"].items():
            self.assertIn(contrast, ("permission_reversal", "root_cause_contrast", "mode_boundary"))
            self.assertTrue(challenge_id.startswith("FEP-SYN-"))

    def test_clustered_run_reports_cluster_and_pair_blocks(self) -> None:
        matrix = load_coverage_matrix()
        scoped_cases = ["FEP-REG-001", "FEP-REG-002"]
        cluster_map = {c: matrix["cluster_map"][c] for c in scoped_cases}
        result = self._run(
            cluster_map=cluster_map,
            corpus_partitions=("development",),
        )
        self.assertEqual(_validate_result(result), [])
        expected_clusters = {c for clusters in cluster_map.values() for c in clusters}
        self.assertEqual(result["counts"]["clusters"], len(expected_clusters))
        for cluster in result["cluster_results"]:
            self.assertIn(cluster["cluster_id"], expected_clusters)


# ══════════════════════════════════════════════════════════════════════════════
# 5. Oracle bundle extraction
# ══════════════════════════════════════════════════════════════════════════════


class OracleBundleTests(unittest.TestCase):
    """Tests for build_evaluation_oracle_bundle."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_extracts_oracle_from_corpus(self) -> None:
        import yaml
        corpus_path = Path(self._tmp.name) / "test_corpus.yaml"
        corpus = {
            "schema_version": "0.1.0-draft",
            "corpus_id": "TEST",
            "created_at": "2026-07-30",
            "revised_at": "2026-07-30",
            "corpus_revision": 1,
            "status": "test",
            "revision_history": [],
            "purpose": "test",
            "corpus_policy": {
                "default_partition": "development",
                "oracle_default_mode": "guarded",
                "blind_first_required": True,
                "oracle_visible_to_subject": False,
                "retrieval_allowed_only_after_precommitment": True,
                "canonical_labels_require_review": True,
                "historical_cases_are_not_current_task_evidence": True,
                "evaluation_and_holdout_storage": "not_yet_created",
                "notes": [],
            },
            "taxonomy": {"primary": [], "bias_tags": [], "shortcut_tags": []},
            "reason_codes": [],
            "cases": [
                {
                    "case_id": "FEP-REG-001",
                    "title": "Test",
                    "case_kind": "historical",
                    "partition": "development",
                    "evidence_tier": "discussion_grounded",
                    "curation_status": "curated_initial",
                    "classification": {"primary": "test", "secondary": [], "stage": "test", "bias_tags": [], "shortcut_tags": ["test"], "trigger_features": ["test"]},
                    "sources": [],
                    "scenario": {"task": "test", "visible_facts": [], "hidden_from_subject": []},
                    "oracle": {
                        "expected_gate_decisions": [{"gate_id": "EVD-PROVENANCE-001", "decision": "defer"}],
                        "required_questions": ["Q1"],
                        "expected_state": {"action": "succeeded", "evidence": "source_grounded", "claim": "eligible", "claim_type": "correction"},
                        "allowed_claims": ["A"],
                        "forbidden_claims": ["B"],
                        "correction_summary": "check source",
                    },
                    "countercase_ids": [],
                },
            ],
        }
        with open(corpus_path, "w", encoding="utf-8") as fh:
            yaml.safe_dump(corpus, fh, allow_unicode=True, sort_keys=False)

        oracles = build_evaluation_oracle_bundle(corpus_path)
        self.assertEqual(len(oracles), 1)
        self.assertEqual(oracles[0]["case_id"], "FEP-REG-001")
        self.assertIn("expected_gate_decisions", oracles[0])
        self.assertIn("allowed_claims", oracles[0])
        # Sources must NOT be in oracle bundle
        self.assertNotIn("sources", oracles[0])


if __name__ == "__main__":
    unittest.main()
