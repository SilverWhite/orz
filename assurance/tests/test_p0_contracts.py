from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator, FormatChecker

from assurance.errors import AssuranceError
from assurance.profile_registry import (
    resolve_effective_profile,
    validate_profile_registry_semantics,
)


ASSURANCE = Path(__file__).resolve().parents[1]
FIXTURES = ASSURANCE / "fixtures" / "p0"
SEMANTIC_INVALID_PROFILE_REGISTRY = (
    FIXTURES / "profile-registry.inheritance-cycle.semantic-invalid.json"
)

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

    def test_profile_inheritance_cycle_is_structurally_valid_but_semantically_denied(
        self,
    ) -> None:
        self.assertEqual(
            errors_for(
                SEMANTIC_INVALID_PROFILE_REGISTRY,
                ASSURANCE / "assurance-profile-registry-v0.1.schema.json",
            ),
            [],
        )
        with self.assertRaisesRegex(AssuranceError, "inheritance cycle"):
            validate_profile_registry_semantics(
                load_json(SEMANTIC_INVALID_PROFILE_REGISTRY)
            )

    def test_general_science_is_domain_neutral_and_lif_adds_only_delta(self) -> None:
        registry = load_json(ASSURANCE / "profile-registry-v0.1.json")
        assert isinstance(registry, dict)
        validate_profile_registry_semantics(registry)
        profiles = registry["profiles"]
        profile_ids = [profile["profile_id"] for profile in profiles]
        self.assertEqual(len(profile_ids), len(set(profile_ids)))

        general_profile = next(
            profile
            for profile in profiles
            if profile["profile_id"] == "general-science"
        )
        lif_profile = next(
            profile for profile in profiles if profile["profile_id"] == "lif-research"
        )
        self.assertIsNone(general_profile["extends_profile_id"])
        self.assertEqual(lif_profile["extends_profile_id"], "general-science")
        serialized_general = json.dumps(general_profile, sort_keys=True).lower()
        for forbidden in (
            "lif-",
            "lif_",
            "lif ",
            " fep",
            "r211",
            "current_index",
            "map6",
            "index/map/r",
        ):
            self.assertNotIn(forbidden, serialized_general)
        self.assertEqual(
            set(lif_profile["assurance_extensions"]),
            {"LifCurrentSourceRouting", "LifValidatorProfile"},
        )
        self.assertTrue(
            set(general_profile["assurance_extensions"]).isdisjoint(
                lif_profile["assurance_extensions"]
            )
        )

        effective_lif = resolve_effective_profile(registry, "lif-research")
        self.assertEqual(
            effective_lif["inheritance_chain"],
            ["general-science", "lif-research"],
        )
        self.assertTrue(
            set(general_profile["assurance_extensions"]).issubset(
                effective_lif["assurance_extensions"]
            )
        )
        self.assertTrue(
            set(general_profile["required_capabilities"]).issubset(
                effective_lif["required_capabilities"]
            )
        )

        grok = next(
            runtime
            for runtime in effective_lif["reference_runtimes"]
            if runtime["runtime_family"] == "grok-build"
        )
        self.assertEqual(grok["role"], "reference_only")
        self.assertFalse(
            lif_profile["acceptance_gate"]["reference_status_grants_acceptance"]
        )
        self.assertNotIn("required_runtime_family", lif_profile)

    def test_profile_inheritance_rejects_dangling_and_redeclared_parent_state(
        self,
    ) -> None:
        registry = load_json(ASSURANCE / "profile-registry-v0.1.json")
        assert isinstance(registry, dict)

        dangling = deepcopy(registry)
        general = next(
            profile
            for profile in dangling["profiles"]
            if profile["profile_id"] == "general-science"
        )
        general["extends_profile_id"] = "missing-parent"
        with self.assertRaisesRegex(AssuranceError, "unknown parent"):
            validate_profile_registry_semantics(dangling)

        redeclared = deepcopy(registry)
        lif = next(
            profile
            for profile in redeclared["profiles"]
            if profile["profile_id"] == "lif-research"
        )
        lif["assurance_extensions"].append("EvidenceKernel")
        with self.assertRaisesRegex(AssuranceError, "redeclares inherited extensions"):
            validate_profile_registry_semantics(redeclared)


if __name__ == "__main__":
    unittest.main()
