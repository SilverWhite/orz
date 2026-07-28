from __future__ import annotations

from copy import deepcopy
from pathlib import Path
from typing import Any

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .utils import load_json


# ── extension / capability → implementation traceability ──────────────
# Each entry maps a declared extension or capability to one or more
# assurance modules (relative to ASSURANCE_ROOT) that provide its
# implementation.  Entries marked with a comment ``# contract_only`` are
# frozen at the profile-contract level but do not yet have a dedicated
# implementation module.

_EXTENSION_MODULES: dict[str, list[str]] = {
    # general-science
    "ClaimBoundary": ["contracts.py", "adapter_output_validator.py"],
    "EvidenceKernel": ["general_science_review.py", "contracts.py"],
    "EvaluationRunner": ["runner_scoring_handoff.py", "runner.py"],
    "LeakScanner": [],  # contract_only — schema in runtime/ but no dedicated module yet
    "ResearchLifecycle": [],  # contract_only
    "ScenarioExporter": [],  # contract_only — schema in runtime/
    "SourceRouter": ["instruction_provenance_gate.py", "source_visibility.py"],
    "ValidatorBridge": ["validator_bridge.py", "artifact_registry.py"],
    # lif-research
    "LifCurrentSourceRouting": [],  # contract_only
    "LifValidatorProfile": [],  # contract_only
}

_CAPABILITY_MODULES: dict[str, list[str]] = {
    "action.precommitted": ["task_contract.py", "guarded_execution.py"],
    "audit.terminal_exactly_once": ["audit.py"],
    "claim.evidence_boundary": ["contracts.py"],
    "evaluation.oracle_isolation": ["runner_scoring_handoff.py"],
    "evidence.source_first": ["general_science_review.py"],
    "external_side_effect.deny_by_default": ["readonly_projection.py"],
    "network.digest_bound_permit": ["network_permit_gateway.py"],
    "network.disabled_by_default": ["network_permit_gateway.py", "sandbox.py"],
    "research.lif.claim_registry_prior_existence": [],  # contract_only
    "research.lif.current_source_routing": [],  # contract_only
    "research.multi_source_traceability": ["general_science_review.py"],
    "session.conversation_namespace": ["conversation.py"],
    "workspace.trust_before_discovery": ["workspace_trust.py"],
}

# Schemas that provide partial implementation evidence for contract_only
# extensions (schema exists but no dedicated Python module).
_EXTENSION_SCHEMAS: dict[str, list[str]] = {
    "LeakScanner": [
        "runtime/leak-scan-report-v0.1.schema.json",
    ],
    "ScenarioExporter": [
        "runtime/scenario-export-manifest-v0.1.schema.json",
    ],
}


PROFILE_REGISTRY_SCHEMA = "assurance-profile-registry-v0.1.schema.json"
PROFILE_REGISTRY_PATH = Path(__file__).resolve().parent / "profile-registry-v0.1.json"


def load_profile_registry(path: Path = PROFILE_REGISTRY_PATH) -> dict[str, Any]:
    registry = load_json(path)
    validate_profile_registry_semantics(registry)
    return registry


def _profile_index(registry: dict[str, Any]) -> dict[str, dict[str, Any]]:
    profiles = registry["profiles"]
    return {profile["profile_id"]: profile for profile in profiles}


def _inheritance_chain(
    profiles: dict[str, dict[str, Any]], profile_id: str
) -> list[dict[str, Any]]:
    chain: list[dict[str, Any]] = []
    seen: list[str] = []
    cursor: str | None = profile_id
    while cursor is not None:
        if cursor in seen:
            cycle = " -> ".join([*seen[seen.index(cursor) :], cursor])
            raise AssuranceError(f"profile inheritance cycle: {cycle}")
        seen.append(cursor)
        profile = profiles.get(cursor)
        if profile is None:
            raise AssuranceError(f"profile references unknown parent: {cursor}")
        chain.append(profile)
        cursor = profile["extends_profile_id"]
    chain.reverse()
    return chain


def validate_profile_registry_semantics(registry: Any) -> None:
    validate_contract(registry, PROFILE_REGISTRY_SCHEMA, label="profile_registry")
    assert isinstance(registry, dict)
    profiles = _profile_index(registry)
    if len(profiles) != len(registry["profiles"]):
        raise AssuranceError("profile IDs must be unique")

    for profile in registry["profiles"]:
        profile_id = profile["profile_id"]
        chain = _inheritance_chain(profiles, profile_id)
        inherited_extensions: set[str] = set()
        inherited_capabilities: set[str] = set()
        inherited_runtime_families: set[str] = set()
        for ancestor in chain[:-1]:
            inherited_extensions.update(ancestor["assurance_extensions"])
            inherited_capabilities.update(ancestor["required_capabilities"])
            inherited_runtime_families.update(
                item["runtime_family"] for item in ancestor["reference_runtimes"]
            )

        duplicate_extensions = sorted(
            inherited_extensions.intersection(profile["assurance_extensions"])
        )
        if duplicate_extensions:
            raise AssuranceError(
                f"profile {profile_id} redeclares inherited extensions: "
                f"{duplicate_extensions}"
            )
        duplicate_capabilities = sorted(
            inherited_capabilities.intersection(profile["required_capabilities"])
        )
        if duplicate_capabilities:
            raise AssuranceError(
                f"profile {profile_id} redeclares inherited capabilities: "
                f"{duplicate_capabilities}"
            )
        duplicate_runtime_families = sorted(
            inherited_runtime_families.intersection(
                item["runtime_family"] for item in profile["reference_runtimes"]
            )
        )
        if duplicate_runtime_families:
            raise AssuranceError(
                f"profile {profile_id} redeclares inherited reference runtimes: "
                f"{duplicate_runtime_families}"
            )


def resolve_effective_profile(registry: Any, profile_id: str) -> dict[str, Any]:
    validate_profile_registry_semantics(registry)
    assert isinstance(registry, dict)
    profiles = _profile_index(registry)
    if profile_id not in profiles:
        raise AssuranceError(f"unknown profile: {profile_id}")
    chain = _inheritance_chain(profiles, profile_id)

    extensions: list[str] = []
    capabilities: list[str] = []
    reference_runtimes: list[dict[str, Any]] = []
    for profile in chain:
        extensions.extend(profile["assurance_extensions"])
        capabilities.extend(profile["required_capabilities"])
        reference_runtimes.extend(deepcopy(profile["reference_runtimes"]))

    return {
        "profile_id": profile_id,
        "inheritance_chain": [profile["profile_id"] for profile in chain],
        "assurance_extensions": extensions,
        "required_capabilities": capabilities,
        "reference_runtimes": reference_runtimes,
        "acceptance_gate": deepcopy(chain[-1]["acceptance_gate"]),
    }


def verify_profile_registry_completeness(
    registry: dict[str, Any] | None = None,
    *,
    assurance_root: Path | None = None,
) -> dict[str, Any]:
    """Verify every declared extension and capability has a traceable implementation.

    Returns a dict with ``valid``, ``profile_reports``, and ``errors``.
    ``contract_only`` items (profile contract frozen but no dedicated module yet)
    are flagged under ``warnings`` rather than ``errors``.
    """
    if registry is None:
        registry = load_profile_registry()
    else:
        validate_profile_registry_semantics(registry)
        assert isinstance(registry, dict)

    root = assurance_root or ASSURANCE_ROOT
    profiles = registry["profiles"]
    profile_reports: list[dict[str, Any]] = []
    all_errors: list[str] = []
    all_warnings: list[str] = []

    for profile in profiles:
        profile_id = profile["profile_id"]
        effective = resolve_effective_profile(registry, profile_id)
        ext_errors: list[str] = []
        ext_warnings: list[str] = []
        cap_errors: list[str] = []
        cap_warnings: list[str] = []
        ref_errors: list[str] = []

        # ── extensions ──────────────────────────────────────────
        for ext in effective["assurance_extensions"]:
            modules = _EXTENSION_MODULES.get(ext)
            if modules is None:
                ext_errors.append(
                    f"extension '{ext}' is not registered in the extension→module map"
                )
                continue
            if not modules:
                schemas = _EXTENSION_SCHEMAS.get(ext, [])
                missing_schemas = [
                    s for s in schemas if not (root.parent / s).exists()
                ]
                if missing_schemas:
                    ext_errors.append(
                        f"extension '{ext}' is contract_only and its schemas "
                        f"are missing: {missing_schemas}"
                    )
                else:
                    ext_warnings.append(
                        f"extension '{ext}' is contract_only (no dedicated module yet)"
                    )
                continue
            missing_modules = [
                m for m in modules if not (root / m).exists()
            ]
            if missing_modules:
                ext_errors.append(
                    f"extension '{ext}' mapped modules are missing: {missing_modules}"
                )

        # ── capabilities ───────────────────────────────────────
        for cap in effective["required_capabilities"]:
            modules = _CAPABILITY_MODULES.get(cap)
            if modules is None:
                cap_errors.append(
                    f"capability '{cap}' is not registered in the capability→module map"
                )
                continue
            if not modules:
                cap_warnings.append(
                    f"capability '{cap}' is contract_only (no dedicated module yet)"
                )
                continue
            missing_modules = [
                m for m in modules if not (root / m).exists()
            ]
            if missing_modules:
                cap_errors.append(
                    f"capability '{cap}' mapped modules are missing: {missing_modules}"
                )

        # ── reference runtime evidence ─────────────────────────
        for runtime in profile.get("reference_runtimes", []):
            for ref in runtime.get("evidence_refs", []):
                ref_path = root.parent / ref
                if not ref_path.exists():
                    ref_errors.append(
                        f"evidence_ref '{ref}' for runtime "
                        f"'{runtime['runtime_family']}' does not exist"
                    )

        profile_valid = not (ext_errors or cap_errors or ref_errors)
        profile_report = {
            "profile_id": profile_id,
            "valid": profile_valid,
            "extension_errors": ext_errors,
            "extension_warnings": ext_warnings,
            "capability_errors": cap_errors,
            "capability_warnings": cap_warnings,
            "reference_evidence_errors": ref_errors,
        }
        profile_reports.append(profile_report)
        all_errors.extend(ext_errors)
        all_errors.extend(cap_errors)
        all_errors.extend(ref_errors)
        all_warnings.extend(ext_warnings)
        all_warnings.extend(cap_warnings)

    return {
        "valid": not all_errors,
        "profile_count": len(profiles),
        "error_count": len(all_errors),
        "warning_count": len(all_warnings),
        "profile_reports": profile_reports,
        "errors": all_errors,
        "warnings": all_warnings,
    }
