from __future__ import annotations

import hashlib
import hmac
from pathlib import Path
from typing import Any

import yaml

from .errors import PrototypeError
from .io_utils import (
    atomic_write_bytes,
    atomic_write_json,
    collect_file_records,
    digest_file_records,
    ensure_regular_file_under,
    load_json,
    safe_relative_path,
    sha256_bytes,
    sha256_file,
    utc_now,
)
from .layout import REGRESSION_ROOT, RUNTIME_ROOT
from .scanner import scan_bundle
from .schema import validate_instance


class UniqueKeyLoader(yaml.SafeLoader):
    pass


def _unique_mapping(loader: UniqueKeyLoader, node: yaml.Node, deep: bool = False) -> dict[Any, Any]:
    mapping: dict[Any, Any] = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in mapping:
            raise PrototypeError(
                f"duplicate YAML key {key!r} at line {key_node.start_mark.line + 1}"
            )
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


UniqueKeyLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _unique_mapping)


def _load_corpus(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = yaml.load(handle, Loader=UniqueKeyLoader)
    except (OSError, yaml.YAMLError) as exc:
        raise PrototypeError(f"cannot read corpus {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise PrototypeError("corpus root must be an object")
    validate_instance(
        value,
        REGRESSION_ROOT / "case-corpus-v0.1.schema.json",
        label="source corpus",
    )
    return value


def _opaque_token(seed: str, corpus_sha256: str, case_id: str) -> str:
    material = f"{corpus_sha256}:{case_id}".encode("utf-8")
    digest = hmac.new(seed.encode("utf-8"), material, hashlib.sha256).hexdigest()
    return f"SCN-{digest[:16]}"


def _copy_fixture(
    *,
    case: dict[str, Any],
    token: str,
    bundle_root: Path,
    role_by_path: dict[str, str],
) -> list[dict[str, str]]:
    manifest_rel = case["scenario"].get("fixture_manifest")
    if not manifest_rel:
        return []
    safe_relative_path(manifest_rel)
    manifest_path = REGRESSION_ROOT / Path(*safe_relative_path(manifest_rel).parts)
    ensure_regular_file_under(manifest_path, REGRESSION_ROOT)
    manifest = load_json(manifest_path)
    validate_instance(
        manifest,
        REGRESSION_ROOT / "fixture-v0.1.schema.json",
        label=f"fixture manifest for {case['case_id']}",
    )
    if manifest["case_id"] != case["case_id"]:
        raise PrototypeError("fixture case ID does not match source case")
    if manifest["visibility"] != "scenario_only":
        raise PrototypeError(
            f"reviewer-only fixture cannot be exported: {manifest_path}"
        )

    exported: list[dict[str, str]] = []
    source_root = manifest_path.parent
    for item in manifest["files"]:
        rel = safe_relative_path(item["path"])
        source = source_root / Path(*rel.parts)
        ensure_regular_file_under(source, source_root)
        source_sha = sha256_file(source)
        if source_sha != item["sha256"]:
            raise PrototypeError(f"fixture hash mismatch: {source}")
        target_rel = f"fixtures/{token}/{rel.as_posix()}"
        target = bundle_root / Path(*safe_relative_path(target_rel).parts)
        atomic_write_bytes(target, source.read_bytes())
        copied_sha = sha256_file(target)
        if copied_sha != source_sha:
            raise PrototypeError(f"copied fixture hash mismatch: {target}")
        role_by_path[target_rel] = "allowed_fixture"
        exported.append(
            {
                "path": target_rel,
                "media_type": item["media_type"],
                "role": item["role"],
                "sha256": copied_sha,
            }
        )
    return exported


def export_scenarios(
    *,
    corpus_path: Path,
    output_dir: Path,
    case_ids: list[str],
    policy_mode: str,
    token_seed: str,
) -> dict[str, Any]:
    if policy_mode not in {"development", "challenge"}:
        raise PrototypeError("prototype exports only development or challenge policy modes")
    if not token_seed:
        raise PrototypeError("token seed must be non-empty")
    if not case_ids or len(case_ids) != len(set(case_ids)):
        raise PrototypeError("case IDs must be a non-empty unique list")
    if output_dir.exists():
        raise PrototypeError(f"refusing to overwrite existing output root: {output_dir}")
    output_dir.mkdir(parents=True)
    bundle_root = output_dir / "scenario_bundle"
    reviewer_root = output_dir / "reviewer"
    bundle_root.mkdir()
    reviewer_root.mkdir()

    try:
        corpus = _load_corpus(corpus_path)
        corpus_sha = sha256_file(corpus_path)
        case_map = {item["case_id"]: item for item in corpus["cases"]}
        missing = [case_id for case_id in case_ids if case_id not in case_map]
        if missing:
            raise PrototypeError(f"unknown case IDs: {missing}")

        role_by_path: dict[str, str] = {}
        case_tokens: list[dict[str, str]] = []
        identity_rows: list[dict[str, str]] = []
        tokens_seen: set[str] = set()
        for case_id in case_ids:
            case = case_map[case_id]
            if case["partition"] not in {"development", "challenge"}:
                raise PrototypeError(
                    f"case {case_id} belongs to forbidden source partition {case['partition']}"
                )
            token = _opaque_token(token_seed, corpus_sha, case_id)
            if token in tokens_seen:
                raise PrototypeError("opaque token collision")
            tokens_seen.add(token)
            fixtures = _copy_fixture(
                case=case, token=token, bundle_root=bundle_root, role_by_path=role_by_path
            )
            scenario = {
                "schema_version": "0.1.0-draft",
                "scenario_token": token,
                "mode": "guarded",
                "task": case["scenario"]["task"],
                "visible_facts": case["scenario"]["visible_facts"],
                "fixtures": fixtures,
            }
            validate_instance(
                scenario,
                RUNTIME_ROOT / "scenario-instance-v0.1.schema.json",
                label=f"exported scenario {token}",
            )
            scenario_rel = f"cases/{token}.json"
            scenario_path = bundle_root / Path(*safe_relative_path(scenario_rel).parts)
            atomic_write_json(scenario_path, scenario)
            role_by_path[scenario_rel] = "scenario_case"
            case_tokens.append(
                {"token": token, "scenario_path": scenario_rel, "sha256": sha256_file(scenario_path)}
            )
            identity_rows.append(
                {
                    "token": token,
                    "case_id": case_id,
                    "source_partition": case["partition"],
                }
            )

        pre_index_records = collect_file_records(bundle_root)
        index = {
            "schema_version": "0.1.0-draft",
            "digest_algorithm": "sha256-rfc8785-file-list-v0.1",
            "files": [
                {**item, "role": role_by_path[item["path"]]}
                for item in pre_index_records
            ],
        }
        atomic_write_json(bundle_root / "bundle-index.json", index)
        role_by_path["bundle-index.json"] = "bundle_index"

        all_records = collect_file_records(bundle_root)
        bundle_sha = digest_file_records(all_records)
        export_id = f"EXP-{bundle_sha[:12].upper()}"
        report = scan_bundle(
            bundle_root,
            export_id=export_id,
            policy_mode=policy_mode,
            expected_bundle_sha256=bundle_sha,
        )
        report_path = reviewer_root / "leak-scan-report.json"
        atomic_write_json(report_path, report)

        identity_map = {
            "schema_version": "0.1.0-prototype",
            "visibility": "reviewer_only",
            "export_id": export_id,
            "token_seed_sha256": sha256_bytes(token_seed.encode("utf-8")),
            "cases": identity_rows,
            "notes": ["Never mount this file into tested-Agent context."],
        }
        atomic_write_json(reviewer_root / "identity-map.json", identity_map)

        bundle_files = [
            {**item, "role": role_by_path[item["path"]]}
            for item in all_records
        ]
        status = "ready" if report["gate_action"] == "allow" else "blocked"
        manifest = {
            "schema_version": "0.1.0-draft",
            "export_id": export_id,
            "created_at": utc_now(),
            "partition": policy_mode,
            "source_corpus": {
                "corpus_id": corpus["corpus_id"],
                "revision": corpus["corpus_revision"],
                "sha256": corpus_sha,
            },
            "policy_version": "scenario-export-policy-v0.1-prototype",
            "case_count": len(case_tokens),
            "case_tokens": case_tokens,
            "bundle_files": bundle_files,
            "bundle_sha256": bundle_sha,
            "identity_map_location": "sealed_external",
            "policy_assertions": {
                "oracle_removed": True,
                "classification_removed": True,
                "source_paths_removed": True,
                "countercase_links_removed": True,
                "curation_fixtures_removed": True,
                "hidden_from_subject_removed": True,
                "case_tokens_opaque": True,
                "allowed_fixture_visibility": "scenario_only",
                "memory_disabled": True,
                "historical_retrieval_disabled": True,
            },
            "leak_scan": {
                "report_path": "reviewer/leak-scan-report.json",
                "report_sha256": sha256_file(report_path),
                "verdict": report["verdict"],
            },
            "status": status,
            "notes": [
                "Development-only prototype export; no model was invoked.",
                "A warn verdict is ready only under development policy and is not evaluation clearance.",
            ],
        }
        validate_instance(
            manifest,
            RUNTIME_ROOT / "scenario-export-manifest-v0.1.schema.json",
            label="scenario export manifest",
        )
        atomic_write_json(output_dir / "scenario-export-manifest.json", manifest)
        return manifest
    except Exception as exc:
        try:
            atomic_write_json(
                output_dir / "_INCOMPLETE.json",
                {
                    "status": "incomplete",
                    "error_type": type(exc).__name__,
                    "error": str(exc),
                    "recorded_at": utc_now(),
                },
            )
        except Exception:
            pass
        raise
