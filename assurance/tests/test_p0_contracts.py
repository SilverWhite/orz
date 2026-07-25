from __future__ import annotations

import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator, FormatChecker


ASSURANCE = Path(__file__).resolve().parents[1]
FIXTURES = ASSURANCE / "fixtures" / "p0"

POSITIVE_CONTRACTS = {
    ASSURANCE / "profile-registry-v0.1.json": (
        ASSURANCE / "assurance-profile-registry-v0.1.schema.json"
    ),
    ASSURANCE / "retention-policy-v0.1.json": (
        ASSURANCE / "retention-policy-v0.1.schema.json"
    ),
    FIXTURES / "effective-security-envelope.valid.json": (
        ASSURANCE / "effective-security-envelope-v0.1.schema.json"
    ),
    FIXTURES / "sandbox-selection-receipt.valid.json": (
        ASSURANCE / "sandbox-selection-receipt-v0.1.schema.json"
    ),
    FIXTURES / "session-lifecycle-receipt.valid.json": (
        ASSURANCE / "session-lifecycle-receipt-v0.1.schema.json"
    ),
}

NEGATIVE_CONTRACTS = {
    FIXTURES / "effective-security-envelope.missing-evidence-status.invalid.json": (
        ASSURANCE / "effective-security-envelope-v0.1.schema.json"
    ),
    FIXTURES / "profile-registry.required-runtime.invalid.json": (
        ASSURANCE / "assurance-profile-registry-v0.1.schema.json"
    ),
    FIXTURES / "sandbox-selection-receipt.unverified-allow.invalid.json": (
        ASSURANCE / "sandbox-selection-receipt-v0.1.schema.json"
    ),
    FIXTURES / "session-lifecycle-receipt.archived-with-residue.invalid.json": (
        ASSURANCE / "session-lifecycle-receipt-v0.1.schema.json"
    ),
    FIXTURES / "retention-policy.private-reasoning-persisted.invalid.json": (
        ASSURANCE / "retention-policy-v0.1.schema.json"
    ),
}


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def errors_for(instance_path: Path, schema_path: Path) -> list[str]:
    validator = Draft202012Validator(
        load_json(schema_path), format_checker=FormatChecker()
    )
    return [error.message for error in validator.iter_errors(load_json(instance_path))]


class P0ContractTests(unittest.TestCase):
    def test_positive_contracts_validate(self) -> None:
        for instance_path, schema_path in POSITIVE_CONTRACTS.items():
            with self.subTest(instance=instance_path.name):
                self.assertEqual(errors_for(instance_path, schema_path), [])

    def test_negative_contracts_fail_closed(self) -> None:
        for instance_path, schema_path in NEGATIVE_CONTRACTS.items():
            with self.subTest(instance=instance_path.name):
                self.assertTrue(errors_for(instance_path, schema_path))

    def test_lif_profile_marks_grok_as_reference_only(self) -> None:
        registry = load_json(ASSURANCE / "profile-registry-v0.1.json")
        assert isinstance(registry, dict)
        profiles = registry["profiles"]
        profile_ids = [profile["profile_id"] for profile in profiles]
        self.assertEqual(len(profile_ids), len(set(profile_ids)))

        lif_profile = next(
            profile for profile in profiles if profile["profile_id"] == "lif-research"
        )
        grok = next(
            runtime
            for runtime in lif_profile["reference_runtimes"]
            if runtime["runtime_family"] == "grok-build"
        )
        self.assertEqual(grok["role"], "reference_only")
        self.assertFalse(
            lif_profile["acceptance_gate"]["reference_status_grants_acceptance"]
        )
        self.assertNotIn("required_runtime_family", lif_profile)


if __name__ == "__main__":
    unittest.main()
