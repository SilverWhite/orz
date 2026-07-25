from __future__ import annotations

from copy import deepcopy
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import load_json


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
