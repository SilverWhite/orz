from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
import tomllib
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker
import yaml


ROOT = Path(__file__).resolve().parents[1]
NON_REPOSITORY_PARTS = {
    ".git",
    ".observed-runs",
    ".pytest_cache",
    ".tools",
    "__pycache__",
}


class UniqueKeyLoader(yaml.SafeLoader):
    pass


def _unique_mapping(
    loader: UniqueKeyLoader, node: yaml.Node, deep: bool = False
) -> dict[Any, Any]:
    mapping: dict[Any, Any] = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in mapping:
            raise ValueError(
                f"duplicate YAML key {key!r} at line {key_node.start_mark.line + 1}"
            )
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


UniqueKeyLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _unique_mapping
)


def _load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def _load_yaml(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return yaml.load(handle, Loader=UniqueKeyLoader)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _validate_instance(instance: Any, schema_path: Path, label: str) -> list[str]:
    validator = Draft202012Validator(
        _load_json(schema_path), format_checker=FormatChecker()
    )
    errors: list[str] = []
    for error in sorted(validator.iter_errors(instance), key=lambda item: list(item.absolute_path)):
        pointer = "/".join(map(str, error.absolute_path))
        errors.append(f"{label}#/{pointer}: {error.message}")
    return errors


def _safe_fixture_path(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if not value or "\\" in value or path.is_absolute() or any(
        part in {"", ".", ".."} for part in path.parts
    ):
        raise ValueError(f"unsafe fixture path: {value!r}")
    return path


def _check_markdown_links() -> list[str]:
    errors: list[str] = []
    pattern = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    for path in sorted(ROOT.rglob("*.md")):
        relative_path = path.relative_to(ROOT)
        if any(part in NON_REPOSITORY_PARTS for part in relative_path.parts):
            continue
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in pattern.finditer(line):
                target = match.group(1).split("#", 1)[0]
                if not target or re.match(r"^(?:https?://|mailto:)", target):
                    continue
                resolved = (path.parent / target).resolve()
                if not resolved.exists():
                    errors.append(
                        f"{path.relative_to(ROOT)}:{line_number}: broken local link {target}"
                    )
    return errors


def check_repository() -> dict[str, Any]:
    errors: list[str] = []
    counts: dict[str, int] = {}

    schema_paths = sorted(ROOT.rglob("*.schema.json"))
    counts["schemas"] = len(schema_paths)
    for schema_path in schema_paths:
        try:
            Draft202012Validator.check_schema(_load_json(schema_path))
        except Exception as exc:
            errors.append(f"invalid schema {schema_path.relative_to(ROOT)}: {exc}")

    corpus_path = ROOT / "regression/cases-v0.1.yaml"
    coverage_path = ROOT / "regression/coverage-matrix-v0.1.yaml"
    reason_path = ROOT / "protocol/reason-codes-v0.1.yaml"
    gate_path = ROOT / "protocol/gate-matrix-v0.1.yaml"
    try:
        corpus = _load_yaml(corpus_path)
        coverage = _load_yaml(coverage_path)
        reasons = _load_yaml(reason_path)
        gates = _load_yaml(gate_path)
    except Exception as exc:
        errors.append(f"YAML load failed: {exc}")
        corpus = coverage = reasons = gates = {}

    if corpus:
        errors.extend(
            _validate_instance(
                corpus,
                ROOT / "regression/case-corpus-v0.1.schema.json",
                "regression/cases-v0.1.yaml",
            )
        )
    if coverage:
        errors.extend(
            _validate_instance(
                coverage,
                ROOT / "regression/coverage-matrix-v0.1.schema.json",
                "regression/coverage-matrix-v0.1.yaml",
            )
        )

    upstream_lock_path = ROOT / "upstream/grok-build.lock.json"
    upstream_lock = _load_json(upstream_lock_path)
    errors.extend(
        _validate_instance(
            upstream_lock,
            ROOT / "upstream/grok-build-lock-v0.1.schema.json",
            "upstream/grok-build.lock.json",
        )
    )
    ownership = upstream_lock.get("ownership", {})
    ownership_sets = {
        name: set(values) for name, values in ownership.items() if isinstance(values, list)
    }
    ownership_names = sorted(ownership_sets)
    for left_index, left_name in enumerate(ownership_names):
        for right_name in ownership_names[left_index + 1 :]:
            overlap = sorted(ownership_sets[left_name] & ownership_sets[right_name])
            if overlap:
                errors.append(
                    f"upstream ownership overlap {left_name}/{right_name}: {overlap}"
                )
    counts["upstream_locks"] = 1

    grok_config_path = ROOT / "integration/grok/deepseek-custom-model.example.toml"
    with grok_config_path.open("rb") as handle:
        grok_config = tomllib.load(handle)
    model_config = grok_config.get("model", {}).get("lif-deepseek-v4-pro", {})
    expected_model_config = {
        "model": "deepseek-v4-pro",
        "base_url": "https://api.deepseek.com",
        "env_key": "LIF_DEEPSEEK_API_KEY",
        "api_backend": "chat_completions",
    }
    for key, expected in expected_model_config.items():
        if model_config.get(key) != expected:
            errors.append(
                f"DeepSeek Grok config {key} must be {expected!r}, "
                f"got {model_config.get(key)!r}"
            )
    if "api_key" in model_config or "extra_headers" in model_config:
        errors.append("DeepSeek Grok config must not contain embedded credentials")
    if grok_config.get("models", {}).get("default"):
        errors.append("discovery-only DeepSeek Grok config must not set a default model")
    counts["grok_config_fixtures"] = 1

    grok_plan_example = ROOT / "integration/grok/examples/example-grok-observed-plan.json"
    errors.extend(
        _validate_instance(
            _load_json(grok_plan_example),
            ROOT / "integration/grok/grok-observed-plan-v0.1.schema.json",
            "integration/grok/examples/example-grok-observed-plan.json",
        )
    )
    counts["grok_observed_plan_examples"] = 1

    trust_example_path = (
        ROOT / "integration/grok/examples/example-grok-workspace-trust-receipt.json"
    )
    errors.extend(
        _validate_instance(
            _load_json(trust_example_path),
            ROOT / "integration/grok/grok-workspace-trust-receipt-v0.1.schema.json",
            "integration/grok/examples/example-grok-workspace-trust-receipt.json",
        )
    )
    trust_script_source = (
        ROOT / "scripts/new_grok_workspace_trust_receipt.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "ExpectedAggregateSha256",
        "expires_on_control_change",
        "launch_permitted",
        "reparse-policy=record-and-do-not-follow",
        "refusing to overwrite",
    ):
        if marker not in trust_script_source:
            errors.append(f"workspace trust preflight is missing safety marker: {marker}")
    counts["grok_workspace_trust_examples"] = 1

    discovery_example_path = (
        ROOT / "integration/grok/examples/example-grok-workspace-discovery-result.json"
    )
    errors.extend(
        _validate_instance(
            _load_json(discovery_example_path),
            ROOT / "integration/grok/grok-workspace-discovery-result-v0.1.schema.json",
            "integration/grok/examples/example-grok-workspace-discovery-result.json",
        )
    )
    discovery_script_source = (
        ROOT / "scripts/invoke_grok_workspace_discovery_probe.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "inspect_trust_flag_is_non_mutating",
        "clean_child_environment",
        "fail_closed_proxy_configured",
        "debug_capture_disabled",
        "refusing to overwrite",
    ):
        if marker not in discovery_script_source:
            errors.append(f"workspace discovery probe is missing safety marker: {marker}")
    if "--debug-file" in discovery_script_source:
        errors.append("workspace discovery probe must not enable Grok debug-file")
    counts["grok_workspace_discovery_examples"] = 1

    fake_provider_path = ROOT / "scripts/fake_deepseek_provider.py"
    fake_launcher_path = ROOT / "scripts/invoke_grok_fake_provider_conformance.ps1"
    fake_provider_source = fake_provider_path.read_text(encoding="utf-8")
    fake_launcher_source = fake_launcher_path.read_text(encoding="utf-8")
    provider_required = (
        'HOST = "127.0.0.1"',
        '"authorization_value_recorded": False',
        '"requests.private.jsonl"',
    )
    launcher_required = (
        "New-NetFirewallRule",
        "Remove-NetFirewallRule",
        "CleanEnvironment",
        "LifJobObject",
        "tool-continuity",
        "reasoning_marker_preserved",
        "credential_value_absent_from_artifacts",
        "debug_capture_disabled",
        "New-RestrictedWorkspaceTrustReceipt",
        "workspace_trust_receipt_preflight",
        "workspace_trust_receipt_launch",
        "workspace_control_aggregate_unchanged",
        "workspace_scan_policy_unchanged",
        "Workspace control surface changed between preflight and launch receipts",
    )
    for marker in provider_required:
        if marker not in fake_provider_source:
            errors.append(f"fake provider is missing safety marker: {marker}")
    for marker in launcher_required:
        if marker not in fake_launcher_source:
            errors.append(f"fake-provider launcher is missing safety marker: {marker}")
    ordered_launcher_markers = (
        "$workspaceTrustReceiptPreflight = New-RestrictedWorkspaceTrustReceipt",
        "$inspectionArgs = @(",
        "$providerHandle = Start-RedirectedProcess",
        "$workspaceTrustReceiptLaunch = New-RestrictedWorkspaceTrustReceipt",
        "$grokHandle = Start-RedirectedProcess",
    )
    marker_positions = [fake_launcher_source.find(marker) for marker in ordered_launcher_markers]
    if any(position < 0 for position in marker_positions) or marker_positions != sorted(marker_positions):
        errors.append(
            "fake-provider launcher must order preflight receipt before binary/provider "
            "processes and launch receipt immediately before the Grok agent process"
        )
    if "'--debug-file'" in fake_launcher_source or '"--debug-file"' in fake_launcher_source:
        errors.append("fake-provider launcher must not enable Grok debug-file on build 0.2.106")
    for schema_name in (
        "grok-fake-provider-result-v0.2.schema.json",
        "grok-tool-continuity-result-v0.3.schema.json",
    ):
        if not (ROOT / "integration/grok" / schema_name).is_file():
            errors.append(f"missing trust-integrated result schema: {schema_name}")
    counts["grok_fake_provider_fixtures"] = 1
    counts["grok_tool_continuity_fixtures"] = 1

    bridge_builder_source = (
        ROOT / "scripts/build_grok_event_bridge.py"
    ).read_text(encoding="utf-8")
    bridge_verifier_source = (
        ROOT / "scripts/verify_grok_event_bridge.py"
    ).read_text(encoding="utf-8")
    postrun_bridge_source = (
        ROOT / "scripts/invoke_grok_postrun_evidence_bridge.ps1"
    ).read_text(encoding="utf-8")
    for marker in (
        "cross_source_runtime_order",
        "not_established",
        "raw_content_omitted_from_bridge",
        "source_result_artifacts_match",
        "refusing to overwrite",
    ):
        if marker not in bridge_builder_source:
            errors.append(f"Grok event bridge is missing provenance marker: {marker}")
    for marker in (
        "event_hash_chain_valid",
        "source_artifacts_match",
        "event_count_and_breakdown_match",
        "completeness_not_promoted",
        "refusing to overwrite",
    ):
        if marker not in bridge_verifier_source:
            errors.append(f"Grok event bridge verifier is missing replay marker: {marker}")
    for marker in (
        "--local",
        "all_network_firewall_block",
        "LifPostrunJobObject",
        "workspace-trust-receipt.postrun.json",
        "debug_capture_disabled",
        "credential_present = $false",
    ):
        if marker not in postrun_bridge_source:
            errors.append(f"Grok post-run bridge is missing safety marker: {marker}")
    counts["grok_event_bridge_fixtures"] = 1

    runtime_examples = {
        "example-deepseek-adapter-profile.json": "deepseek-adapter-profile-v0.1.schema.json",
        "example-leak-scan-report.json": "leak-scan-report-v0.1.schema.json",
        "example-run-event.json": "run-event-v0.1.schema.json",
        "example-run-manifest.json": "run-manifest-v0.1.schema.json",
        "example-scenario-export-manifest.json": "scenario-export-manifest-v0.1.schema.json",
    }
    for example_name, schema_name in runtime_examples.items():
        errors.extend(
            _validate_instance(
                _load_json(ROOT / "runtime/examples" / example_name),
                ROOT / "runtime" / schema_name,
                f"runtime/examples/{example_name}",
            )
        )
    progress_fixture_root = ROOT / "runtime/fixtures/global-progress-sentinel-v0.1"
    progress_fixtures = {
        "input.json": "global-progress-input-v0.1.schema.json",
        "reasoned-continue.disposition.json": "global-progress-disposition-v0.1.schema.json",
        "replan.disposition.json": "global-progress-disposition-v0.1.schema.json",
    }
    for fixture_name, schema_name in progress_fixtures.items():
        errors.extend(
            _validate_instance(
                _load_json(progress_fixture_root / fixture_name),
                ROOT / "runtime" / schema_name,
                f"runtime/fixtures/global-progress-sentinel-v0.1/{fixture_name}",
            )
        )
    counts["global_progress_fixtures"] = len(progress_fixtures)
    errors.extend(
        _validate_instance(
            _load_json(ROOT / "evaluation/example-evaluation-result-v0.1.json"),
            ROOT / "evaluation/evaluation-result-v0.1.schema.json",
            "evaluation/example-evaluation-result-v0.1.json",
        )
    )

    reason_codes = {item["code"] for item in reasons.get("reason_codes", [])}
    corpus_reason_codes = set(corpus.get("reason_codes", []))
    unknown_corpus_reasons = sorted(corpus_reason_codes - reason_codes)
    if unknown_corpus_reasons:
        errors.append(f"corpus uses unknown reason codes: {unknown_corpus_reasons}")
    for stage in gates.get("stages", []):
        unknown = sorted(set(stage.get("gate_codes", [])) - reason_codes)
        if unknown:
            errors.append(f"gate stage {stage['stage_id']} uses unknown reason codes: {unknown}")

    cases = corpus.get("cases", [])
    case_by_id = {case["case_id"]: case for case in cases}
    counts["cases"] = len(cases)
    if len(case_by_id) != len(cases):
        errors.append("case IDs are not unique")
    for case in cases:
        case_id = case["case_id"]
        for decision in case["oracle"]["expected_gate_decisions"]:
            if decision["gate_id"] not in reason_codes:
                errors.append(f"{case_id} uses unknown oracle gate {decision['gate_id']}")
        for other_id in case.get("countercase_ids", []):
            other = case_by_id.get(other_id)
            if other is None:
                errors.append(f"{case_id} references missing countercase {other_id}")
            elif case_id not in other.get("countercase_ids", []):
                errors.append(f"countercase link is not reciprocal: {case_id} -> {other_id}")

    coverage_case_ids: set[str] = set()
    for cluster in coverage.get("clusters", []):
        unknown_reasons = sorted(set(cluster["primary_reason_codes"]) - reason_codes)
        if unknown_reasons:
            errors.append(
                f"{cluster['cluster_id']} uses unknown reason codes: {unknown_reasons}"
            )
        referenced = list(cluster["historical_case_ids"]) + [
            item["case_id"] for item in cluster["challenge_cases"]
        ]
        coverage_case_ids.update(referenced)
        for case_id in referenced:
            if case_id not in case_by_id:
                errors.append(f"{cluster['cluster_id']} references missing case {case_id}")
    uncovered = sorted(set(case_by_id) - coverage_case_ids)
    if uncovered:
        errors.append(f"cases missing from coverage matrix: {uncovered}")

    fixture_schema = ROOT / "regression/fixture-v0.1.schema.json"
    provenance_schema = ROOT / "regression/historical-excerpt-provenance-v0.1.schema.json"
    fixture_count = 0
    for case in cases:
        references = []
        if case["scenario"].get("fixture_manifest"):
            references.append((case["scenario"]["fixture_manifest"], "scenario_only"))
        if case.get("curation_fixture_manifest"):
            references.append((case["curation_fixture_manifest"], "reviewer_only"))
        for relative, expected_visibility in references:
            fixture_path = ROOT / "regression" / Path(*_safe_fixture_path(relative).parts)
            if not fixture_path.is_file():
                errors.append(f"{case['case_id']} fixture is missing: {relative}")
                continue
            fixture_count += 1
            fixture = _load_json(fixture_path)
            errors.extend(
                _validate_instance(
                    fixture,
                    fixture_schema,
                    str(fixture_path.relative_to(ROOT)),
                )
            )
            if fixture.get("case_id") != case["case_id"]:
                errors.append(f"fixture case mismatch at {fixture_path.relative_to(ROOT)}")
            if fixture.get("visibility") != expected_visibility:
                errors.append(
                    f"fixture visibility mismatch at {fixture_path.relative_to(ROOT)}; "
                    f"expected {expected_visibility}"
                )
            for file_record in fixture.get("files", []):
                file_path = fixture_path.parent / Path(
                    *_safe_fixture_path(file_record["path"]).parts
                )
                if not file_path.is_file():
                    errors.append(f"fixture file is missing: {file_path.relative_to(ROOT)}")
                    continue
                if _sha256(file_path) != file_record["sha256"]:
                    errors.append(f"fixture digest mismatch: {file_path.relative_to(ROOT)}")
                if file_path.name == "provenance.json":
                    errors.extend(
                        _validate_instance(
                            _load_json(file_path),
                            provenance_schema,
                            str(file_path.relative_to(ROOT)),
                        )
                    )
    counts["fixture_manifests"] = fixture_count

    errors.extend(_check_markdown_links())
    errors.sort()
    return {
        "valid": not errors,
        "counts": counts,
        "error_count": len(errors),
        "errors": errors,
        "limitations": [
            "This is a deterministic repository-integrity check, not scientific validation.",
            "It does not establish oracle-free semantics or evaluation/holdout readiness.",
        ],
    }


def main() -> int:
    try:
        report = check_repository()
    except Exception as exc:
        report = {
            "valid": False,
            "error_count": 1,
            "errors": [f"repository checker failed: {type(exc).__name__}: {exc}"],
        }
    print(
        json.dumps(
            report, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False
        )
    )
    return 0 if report["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
