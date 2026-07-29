"""Tests for UX safety validation framework — GAK-UX-001."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from assurance import MemoryInstallationKeyStore
from assurance.ux_safety import (
    UXAssertion,
    UXAssertionKind,
    UXPersona,
    UXSafetyEvaluator,
    UXSafetyScenario,
    UXSafetySuite,
    _build_experienced_scenarios,
    _build_novice_scenarios,
    _build_shared_scenarios,
)


class UXSafetyScenarioTests(unittest.TestCase):
    """Tests for scenario construction and structure."""

    def test_novice_scenarios_all_present(self) -> None:
        """All 4 novice scenarios are built."""
        scenarios = _build_novice_scenarios()
        self.assertEqual(len(scenarios), 4)
        ids = {s.scenario_id for s in scenarios}
        self.assertIn("UX-NOVICE-STANDARD-001", ids)
        self.assertIn("UX-NOVICE-READ-FILE-002", ids)
        self.assertIn("UX-NOVICE-BAD-COMMAND-003", ids)
        self.assertIn("UX-NOVICE-ARCHIVE-CONFIRM-004", ids)
        for s in scenarios:
            self.assertEqual(s.persona, UXPersona.NOVICE)
            self.assertGreater(len(s.assertions), 0)

    def test_experienced_scenarios_all_present(self) -> None:
        """All 3 experienced scenarios are built."""
        scenarios = _build_experienced_scenarios()
        self.assertEqual(len(scenarios), 3)
        ids = {s.scenario_id for s in scenarios}
        self.assertIn("UX-EXPERIENCED-STRICT-005", ids)
        self.assertIn("UX-EXPERIENCED-NETWORK-006", ids)
        self.assertIn("UX-EXPERIENCED-CREDENTIAL-007", ids)
        for s in scenarios:
            self.assertEqual(s.persona, UXPersona.EXPERIENCED)

    def test_shared_scenarios_all_present(self) -> None:
        """All 3 shared scenarios are built."""
        scenarios = _build_shared_scenarios()
        self.assertEqual(len(scenarios), 3)
        ids = {s.scenario_id for s in scenarios}
        self.assertIn("UX-BOTH-BOUNDARY-008", ids)
        self.assertIn("UX-BOTH-RECOVERY-HINT-009", ids)
        self.assertIn("UX-BOTH-DIALOG-REJECT-010", ids)

    def test_total_default_suite_is_10_scenarios(self) -> None:
        """Default suite contains exactly 10 scenarios."""
        evaluator = UXSafetyEvaluator()
        self.assertEqual(len(evaluator._scenarios.scenarios), 10)

    def test_every_scenario_has_expected_user_action(self) -> None:
        """Every scenario must guide the user to a concrete next action."""
        evaluator = UXSafetyEvaluator()
        for s in evaluator._scenarios.scenarios:
            self.assertTrue(
                len(s.expected_user_action) > 0,
                f"{s.scenario_id}: expected_user_action must not be empty",
            )

    def test_every_scenario_has_at_least_one_assertion(self) -> None:
        """Every scenario must verify at least one UX safety property."""
        evaluator = UXSafetyEvaluator()
        for s in evaluator._scenarios.scenarios:
            self.assertGreater(
                len(s.assertions), 0,
                f"{s.scenario_id}: must have at least one assertion",
            )


class UXSafetyEvaluatorTests(unittest.TestCase):
    """Tests for :class:`UXSafetyEvaluator`."""

    def setUp(self) -> None:
        self.evaluator = UXSafetyEvaluator()
        self.key_store = MemoryInstallationKeyStore()

    def test_evaluate_single_scenario_produces_pass_or_fail(self) -> None:
        """Evaluating a single scenario sets passed flag."""
        scenario = _build_novice_scenarios()[0]
        result = self.evaluator.evaluate_scenario(scenario)
        self.assertIsInstance(result.passed, bool)
        self.assertIsInstance(result.errors, list)

    def test_all_10_scenarios_evaluate_without_exception(self) -> None:
        """All default scenarios evaluate without throwing."""
        for scenario in self.evaluator._scenarios.scenarios:
            try:
                result = self.evaluator.evaluate_scenario(scenario)
                self.assertIsNotNone(result)
            except Exception as exc:
                self.fail(
                    f"{scenario.scenario_id} raised {type(exc).__name__}: {exc}"
                )

    def test_run_suite_produces_signed_receipt(self) -> None:
        """run_suite produces a signed receipt."""
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp)
            receipt = self.evaluator.run_suite(self.key_store, run_root)

            self.assertEqual(receipt["receipt_kind"], "ux_safety_evaluation")
            self.assertIn("integrity", receipt)
            self.assertIn("signature", receipt["integrity"])
            agg = receipt["aggregate"]
            self.assertEqual(agg["total_scenarios"], 10)
            self.assertGreaterEqual(agg["passed_scenarios"], 0)

    def test_receipt_verification_passes(self) -> None:
        """A freshly produced receipt passes verification."""
        with tempfile.TemporaryDirectory() as tmp:
            receipt = self.evaluator.run_suite(self.key_store, Path(tmp))
            result = self.evaluator.verify_receipt(receipt, self.key_store)
            self.assertTrue(result["valid"], result["errors"])
            self.assertEqual(result["scenarios_evaluated"], 10)

    def test_receipt_verification_fails_with_wrong_key(self) -> None:
        """Verification fails with a different key store."""
        with tempfile.TemporaryDirectory() as tmp:
            receipt = self.evaluator.run_suite(self.key_store, Path(tmp))
            other_key = MemoryInstallationKeyStore()
            result = self.evaluator.verify_receipt(receipt, other_key)
            self.assertFalse(result["valid"])

    def test_receipt_persisted_to_disk(self) -> None:
        """Receipt is written to the run_root directory."""
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp)
            receipt = self.evaluator.run_suite(self.key_store, run_root)
            receipt_files = list(run_root.glob("ux-safety-*.json"))
            self.assertEqual(len(receipt_files), 1)

    def test_tampered_receipt_detected(self) -> None:
        """Tampering with the receipt is detected."""
        with tempfile.TemporaryDirectory() as tmp:
            receipt = self.evaluator.run_suite(self.key_store, Path(tmp))
            receipt["aggregate"]["passed_scenarios"] = 999
            result = self.evaluator.verify_receipt(receipt, self.key_store)
            self.assertFalse(result["valid"])

    def test_empty_scenarios_in_receipt_detected(self) -> None:
        """Receipt with zero scenarios is flagged."""
        body = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "ux_safety_evaluation",
            "receipt_id": "UXR-TEST",
            "suite_id": "TEST",
            "created_at": "2026-01-01T00:00:00Z",
            "environment": {},
            "scenario_results": [],
            "aggregate": {"total_scenarios": 0, "passed_scenarios": 0, "failed_scenarios": 0},
            "claims": {},
            "limitations": [],
        }
        from assurance.ux_safety import canonical_bytes, sha256_bytes
        payload = canonical_bytes(body)
        receipt = {
            **body,
            "integrity": {
                "canonicalization": "RFC8785",
                "key_id": self.key_store.key_id,
                "signature_algorithm": "hmac-sha256",
                "signed_payload_sha256": sha256_bytes(payload),
                "signature": self.key_store.sign(payload),
            },
        }
        result = self.evaluator.verify_receipt(receipt, self.key_store)
        self.assertFalse(result["valid"])

    def test_wrong_receipt_kind_rejected(self) -> None:
        """Receipt with wrong receipt_kind is rejected."""
        with tempfile.TemporaryDirectory() as tmp:
            receipt = self.evaluator.run_suite(self.key_store, Path(tmp))
            receipt["receipt_kind"] = "wrong_kind"
            result = self.evaluator.verify_receipt(receipt, self.key_store)
            self.assertFalse(result["valid"])

    def test_passed_but_has_errors_detected(self) -> None:
        """Scenario marked passed but with errors is flagged."""
        with tempfile.TemporaryDirectory() as tmp:
            receipt = self.evaluator.run_suite(self.key_store, Path(tmp))
            # Inject inconsistency
            receipt["scenario_results"][0]["passed"] = True
            receipt["scenario_results"][0]["errors"] = ["should not happen"]
            result = self.evaluator.verify_receipt(receipt, self.key_store)
            self.assertFalse(result["valid"])


class UXAssertionKindTests(unittest.TestCase):
    """Tests for assertion kind coverage."""

    def test_all_8_assertion_kinds_used(self) -> None:
        """Every assertion kind is used by at least one scenario."""
        evaluator = UXSafetyEvaluator()
        used: set[UXAssertionKind] = set()
        for s in evaluator._scenarios.scenarios:
            for a in s.assertions:
                used.add(a.kind)
        all_kinds = set(UXAssertionKind)
        unused = all_kinds - used
        self.assertEqual(
            unused, set(),
            f"Unused assertion kinds: {unused}",
        )

    def test_scenario_categories_cover_key_areas(self) -> None:
        """Scenario categories cover all critical UX safety areas."""
        evaluator = UXSafetyEvaluator()
        categories = {s.category for s in evaluator._scenarios.scenarios}
        required = {
            UXAssertionKind.GATE_VISIBLE,
            UXAssertionKind.FAIL_CLOSED_EXPLAINED,
            UXAssertionKind.NO_SILENT_FALLBACK,
            UXAssertionKind.DIALOG_BOUND_TO_ACTION,
            UXAssertionKind.ERROR_DISTINGUISHABLE,
            UXAssertionKind.NEXT_ACTION_SUGGESTED,
            UXAssertionKind.PERMISSION_DENIED_CLEAR,
            UXAssertionKind.PERMISSION_COMPREHENSION,
        }
        missing = required - categories
        self.assertEqual(
            missing, set(),
            f"Scenario categories missing: {missing}",
        )


class EnvironmentSafeguardTests(unittest.TestCase):
    """Tests verifying the evaluation environment is safe."""

    def test_run_suite_declares_safeguards(self) -> None:
        """Receipt declares no real data, no network, no model usage."""
        evaluator = UXSafetyEvaluator()
        key_store = MemoryInstallationKeyStore()
        with tempfile.TemporaryDirectory() as tmp:
            receipt = evaluator.run_suite(key_store, Path(tmp))
            env = receipt["environment"]
            self.assertTrue(env["disposable_workspace_only"])
            self.assertTrue(env["fake_credentials_only"])
            self.assertFalse(env["real_project_data_used"])
            self.assertFalse(env["network_requested"])
            self.assertFalse(env["model_invoked"])
            self.assertEqual(env["external_participant_count"], 0)

    def test_claims_acknowledge_limitations(self) -> None:
        """Claims honestly state human testing is not done."""
        evaluator = UXSafetyEvaluator()
        key_store = MemoryInstallationKeyStore()
        with tempfile.TemporaryDirectory() as tmp:
            receipt = evaluator.run_suite(key_store, Path(tmp))
            claims = receipt["claims"]
            self.assertFalse(claims["human_comprehension_assessed"])
            self.assertFalse(claims["human_usability_assessed"])
            self.assertFalse(claims["ordinary_user_safety_established"])
            self.assertTrue(len(receipt["limitations"]) >= 3)


if __name__ == "__main__":
    unittest.main()
