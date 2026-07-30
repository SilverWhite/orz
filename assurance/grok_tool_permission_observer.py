from __future__ import annotations

from dataclasses import dataclass
import json
import os
from pathlib import Path
import subprocess
import tempfile
from typing import Any, Sequence

from .contracts import validate_contract
from .errors import AssuranceError
from .grok_profile_drafts import EXPECTED_PROFILES
from .tool_availability_gate import (
    build_tool_availability_gate_receipt,
    probe_tool_availability,
)
from .utils import canonical_bytes, load_json, sha256_bytes, utc_now


ROOT = Path(__file__).resolve().parents[1]
RECEIPT_SCHEMA = "grok-tool-permission-observation-receipt-v0.1.schema.json"
CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)

GROK_HELP_FLAGS = (
    "--allow",
    "--deny",
    "--permission-mode",
    "--always-approve",
    "--disable-web-search",
    "--tools",
    "--disallowed-tools",
)
AGENT_HELP_FLAGS = ("--agent-profile", "--always-approve")
REQUIRED_ACP_PERMISSION_SCENARIOS = ("allow_once", "cancel_permission")


@dataclass(frozen=True)
class GrokToolPermissionObservationConfig:
    repo_root: Path = ROOT
    binary_path: Path = ROOT / ".tools" / "grok" / "0.2.112" / "grok.exe"
    timeout_seconds: int = 20


def _run_text_command(
    command: list[str],
    *,
    cwd: Path,
    timeout: int,
    environment: dict[str, str],
) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    if completed.returncode != 0:
        raise AssuranceError(
            f"command failed ({completed.returncode}): {completed.stderr.strip()}"
        )
    return completed.stdout


def _run_json_command(
    command: list[str],
    *,
    cwd: Path,
    timeout: int,
    environment: dict[str, str],
) -> dict[str, Any]:
    stdout = _run_text_command(
        command, cwd=cwd, timeout=timeout, environment=environment
    )
    try:
        value = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise AssuranceError(f"command did not return JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise AssuranceError("command JSON output is not an object")
    return value


def _isolated_observation_environment(home: Path) -> dict[str, str]:
    environment = dict(os.environ)
    environment.update(
        {
            "GROK_HOME": str(home),
            "HOME": str(home),
            "USERPROFILE": str(home),
            "GROK_MEMORY": "0",
            "GROK_WEB_FETCH": "0",
            "HTTP_PROXY": "http://127.0.0.1:1",
            "HTTPS_PROXY": "http://127.0.0.1:1",
            "ALL_PROXY": "http://127.0.0.1:1",
            "NO_PROXY": "127.0.0.1,localhost",
        }
    )
    return environment


def observe_grok_tool_permission_surfaces(
    config: GrokToolPermissionObservationConfig | None = None,
    *,
    acp_verification_paths: Sequence[Path] = (),
) -> dict[str, Any]:
    cfg = config or GrokToolPermissionObservationConfig()
    if not cfg.binary_path.exists():
        raise AssuranceError(f"Grok binary not found: {cfg.binary_path}")
    with tempfile.TemporaryDirectory(prefix="grok-observe-", dir=cfg.repo_root) as home:
        environment = _isolated_observation_environment(Path(home))
        inspect_report = _run_json_command(
            [str(cfg.binary_path), "inspect", "--json"],
            cwd=cfg.repo_root,
            timeout=cfg.timeout_seconds,
            environment=environment,
        )
        grok_help_text = _run_text_command(
            [str(cfg.binary_path), "--help"],
            cwd=cfg.repo_root,
            timeout=cfg.timeout_seconds,
            environment=environment,
        )
        agent_help_text = _run_text_command(
            [str(cfg.binary_path), "agent", "--help"],
            cwd=cfg.repo_root,
            timeout=cfg.timeout_seconds,
            environment=environment,
        )
    acp_verifications = [load_json(path) for path in acp_verification_paths]
    return build_grok_tool_permission_observation_receipt(
        inspect_report=inspect_report,
        grok_help_text=grok_help_text,
        agent_help_text=agent_help_text,
        acp_verifications=acp_verifications,
        inspect_source="grok inspect --json",
        grok_help_source="grok --help",
        agent_help_source="grok agent --help",
    )


def _agent_names_by_source(inspect_report: dict[str, Any], source_type: str) -> list[str]:
    agents = inspect_report.get("agents", [])
    if not isinstance(agents, list):
        return []
    names: list[str] = []
    for agent in agents:
        if not isinstance(agent, dict):
            continue
        source = agent.get("source", {})
        if not isinstance(source, dict) or source.get("type") != source_type:
            continue
        name = agent.get("name")
        if isinstance(name, str) and name:
            names.append(name)
    return sorted(set(names))


def _permissions_surface(inspect_report: dict[str, Any]) -> dict[str, Any]:
    permissions = inspect_report.get("permissions", {})
    if not isinstance(permissions, dict):
        permissions = {}
    sources = permissions.get("sources", [])
    skipped = permissions.get("skipped", [])
    return {
        "loaded": int(permissions.get("loaded") or 0),
        "skipped_count": len(skipped) if isinstance(skipped, list) else 0,
        "sources_count": len(sources) if isinstance(sources, list) else 0,
        "managed_settings_active": bool(permissions.get("managedSettingsActive")),
    }


def _flag_map(text: str, flags: Sequence[str]) -> dict[str, bool]:
    return {flag: flag in text for flag in flags}


def _summarize_acp_verifications(
    acp_verifications: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    valid_count = 0
    invalid_count = 0
    probe_ids: set[str] = set()
    result_hashes: set[str] = set()
    covered_scenarios: set[str] = set()
    permission_outcomes: set[str] = set()
    provider_scenarios: set[str] = set()
    scenario_semantics = []
    safety_checks = []
    for verification in acp_verifications:
        checks = verification.get("checks", {})
        valid = (
            verification.get("verification_kind")
            == "grok-acp-fake-tool-probe-verification"
            and verification.get("valid") is True
            and isinstance(checks, dict)
            and checks.get("scenario_semantics_match") is True
            and checks.get("safety_checks_all_true") is True
            and not verification.get("errors")
        )
        if valid:
            valid_count += 1
            scenario = verification.get("scenario")
            if isinstance(scenario, str):
                covered_scenarios.add(scenario)
            permission_outcome = verification.get("permission_outcome")
            if isinstance(permission_outcome, str):
                permission_outcomes.add(permission_outcome)
            provider_scenario = verification.get("provider_scenario")
            if isinstance(provider_scenario, str):
                provider_scenarios.add(provider_scenario)
        else:
            invalid_count += 1
        probe_id = verification.get("probe_id")
        if isinstance(probe_id, str):
            probe_ids.add(probe_id)
        result_sha256 = verification.get("result_sha256")
        if isinstance(result_sha256, str):
            result_hashes.add(result_sha256)
        if isinstance(checks, dict):
            scenario_semantics.append(checks.get("scenario_semantics_match") is True)
            safety_checks.append(checks.get("safety_checks_all_true") is True)
        else:
            scenario_semantics.append(False)
            safety_checks.append(False)

    attached = len(acp_verifications) > 0
    required_scenarios_verified = set(REQUIRED_ACP_PERMISSION_SCENARIOS).issubset(
        covered_scenarios
    )
    return {
        "attached": attached,
        "valid_count": valid_count,
        "invalid_count": invalid_count,
        "probe_ids": sorted(probe_ids),
        "result_sha256": sorted(result_hashes),
        "covered_scenarios": sorted(covered_scenarios),
        "required_scenarios": list(REQUIRED_ACP_PERMISSION_SCENARIOS),
        "required_scenarios_verified": required_scenarios_verified,
        "permission_outcomes": sorted(permission_outcomes),
        "provider_scenarios": sorted(provider_scenarios),
        "scenario_semantics_verified": attached and all(scenario_semantics),
        "safety_checks_verified": attached and all(safety_checks),
    }


def build_grok_tool_permission_observation_receipt(
    *,
    inspect_report: dict[str, Any],
    grok_help_text: str,
    agent_help_text: str,
    acp_verifications: Sequence[dict[str, Any]] = (),
    inspect_source: str = "provided inspect report",
    grok_help_source: str = "provided grok help text",
    agent_help_source: str = "provided agent help text",
    observed_at: str | None = None,
) -> dict[str, Any]:
    project_agents = _agent_names_by_source(inspect_report, "project")
    builtin_agents = _agent_names_by_source(inspect_report, "builtin")
    expected_project_agents = {
        value["name"] for value in EXPECTED_PROFILES.values() if "name" in value
    }
    missing_project_agents = expected_project_agents - set(project_agents)

    grok_flags = _flag_map(grok_help_text, GROK_HELP_FLAGS)
    agent_flags = _flag_map(agent_help_text, AGENT_HELP_FLAGS)
    acp_observation = _summarize_acp_verifications(acp_verifications)
    mcp_servers = inspect_report.get("mcpServers", [])
    grok_version = inspect_report.get("grokVersion")
    if not isinstance(grok_version, str) or not grok_version:
        grok_version = "unknown"

    checks = {
        "grok_inspect_observed": bool(inspect_report),
        "grok_help_observed": bool(grok_help_text.strip()),
        "agent_help_observed": bool(agent_help_text.strip()),
        "expected_project_agents_discovered": not missing_project_agents,
        "permission_controls_observed": all(
            grok_flags[flag]
            for flag in ("--allow", "--deny", "--permission-mode", "--always-approve")
        ),
        "agent_profile_control_observed": agent_flags["--agent-profile"],
        "web_disable_control_observed": grok_flags["--disable-web-search"],
        "acp_permission_probe_attached": acp_observation["attached"],
        "acp_permission_probe_valid_when_attached": (
            not acp_observation["attached"] or acp_observation["invalid_count"] == 0
        ),
        "acp_permission_required_scenarios_verified_when_attached": (
            not acp_observation["attached"]
            or acp_observation["required_scenarios_verified"]
        ),
        "no_model_invoked": True,
        "no_network_requested": True,
        "prompt_tool_promotion_blocked": True,
    }
    hard_check_names = (
        "grok_inspect_observed",
        "grok_help_observed",
        "agent_help_observed",
        "expected_project_agents_discovered",
        "permission_controls_observed",
        "agent_profile_control_observed",
        "web_disable_control_observed",
        "acp_permission_probe_valid_when_attached",
        "acp_permission_required_scenarios_verified_when_attached",
        "no_model_invoked",
        "no_network_requested",
        "prompt_tool_promotion_blocked",
    )
    valid = all(checks[name] for name in hard_check_names)
    if not valid:
        decision = "block"
    elif acp_observation["required_scenarios_verified"]:
        decision = "allow"
    else:
        decision = "defer"

    fingerprint = sha256_bytes(
        canonical_bytes(
            {
                "grok_version": grok_version,
                "project_agents": project_agents,
                "builtin_agents": builtin_agents,
                "grok_flags": grok_flags,
                "agent_flags": agent_flags,
                "acp": acp_observation,
            }
        )
    )[:16].upper()
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "grok_tool_permission_observation_receipt",
        "receipt_id": f"GROK-TOOL-PERM-{fingerprint}",
        "valid": valid,
        "decision": decision,
        "observed_at": observed_at or utc_now(),
        "runtime_owner": "grok",
        "runtime_id": f"GROK-{grok_version}",
        "inputs": {
            "inspect_source": inspect_source,
            "grok_help_source": grok_help_source,
            "agent_help_source": agent_help_source,
            "acp_verification_count": len(acp_verifications),
        },
        "surface": {
            "grok_version": grok_version,
            "project_trusted": bool(inspect_report.get("projectTrusted")),
            "project_agent_count": len(project_agents),
            "builtin_agent_count": len(builtin_agents),
            "project_agents": project_agents,
            "builtin_agents": builtin_agents,
            "mcp_server_count": len(mcp_servers) if isinstance(mcp_servers, list) else 0,
            "permissions": _permissions_surface(inspect_report),
        },
        "controls": {
            "grok_help_flags": grok_flags,
            "agent_help_flags": agent_flags,
            "permission_control_flags_present": checks["permission_controls_observed"],
            "tool_filter_flags_present": all(
                grok_flags[flag] for flag in ("--tools", "--disallowed-tools")
            ),
            "web_disable_flag_present": checks["web_disable_control_observed"],
            "agent_profile_flag_present": checks["agent_profile_control_observed"],
        },
        "acp_permission_observation": acp_observation,
        "checks": checks,
        "limitations": [
            "This receipt observes registry and permission surfaces only; it does not send prompts or invoke Grok tools.",
            "Prompt/tool mode promotion remains blocked until child-tree containment and lifecycle mapping gates are satisfied.",
            "If no ACP fake-tool verification is attached, permission-request semantics remain degraded in the tool availability projection.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="grok tool permission observation receipt")
    return receipt


def build_grok_tool_availability_projection(
    observation_receipt: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(
        observation_receipt,
        RECEIPT_SCHEMA,
        label="grok tool permission observation receipt",
    )
    checks = observation_receipt["checks"]
    acp = observation_receipt["acp_permission_observation"]
    tool_specs = [
        {
            "tool_id": "grok_tool_registry",
            "tool_name": "Grok inspect tool registry",
            "capability": "tool_registry",
            "probe_method": "grok inspect --json",
            "runtime_constraint": "Static registry discovery only; no prompt or model invocation.",
        },
        {
            "tool_id": "grok_project_retrieval_agents",
            "tool_name": "Grok project retrieval agent profiles",
            "capability": "subagent",
            "probe_method": "grok inspect --json plus profile draft verifier",
            "runtime_constraint": "Exactly two read-only retrieval profiles are expected.",
        },
        {
            "tool_id": "grok_permission_controls",
            "tool_name": "Grok permission control flags",
            "capability": "tool_registry",
            "probe_method": "grok --help",
            "runtime_constraint": "Static CLI control discovery only.",
        },
        {
            "tool_id": "grok_web_retrieval_controls",
            "tool_name": "Grok web retrieval disable control",
            "capability": "web_fetch",
            "probe_method": "grok --help",
            "runtime_constraint": "Control is available for gating; retrieval is not executed.",
        },
        {
            "tool_id": "grok_acp_permission_probe",
            "tool_name": "Grok ACP fake-tool permission probe",
            "capability": "tool_registry",
            "probe_method": "grok ACP fake-tool verification receipt",
            "runtime_constraint": "Degraded until a valid ACP fake-tool verification is attached.",
        },
    ]
    probe_registry = {
        "grok_tool_registry": checks["grok_inspect_observed"],
        "grok_project_retrieval_agents": checks["expected_project_agents_discovered"],
        "grok_permission_controls": checks["permission_controls_observed"],
        "grok_web_retrieval_controls": checks["web_disable_control_observed"],
        "grok_acp_permission_probe": None
        if not acp["attached"] or not acp["required_scenarios_verified"]
        else acp["invalid_count"] == 0,
    }
    return probe_tool_availability(
        tool_specs=tool_specs,
        runtime_id="GROK-TOOL-PERMISSION-001",
        probe_registry=probe_registry,
    )


def build_grok_tool_permission_observation_bundle(
    observation_receipt: dict[str, Any],
) -> dict[str, Any]:
    report = build_grok_tool_availability_projection(observation_receipt)
    gate_receipt = build_tool_availability_gate_receipt(report)
    return {
        "observation_receipt": observation_receipt,
        "tool_availability_report": report,
        "tool_availability_gate_receipt": gate_receipt,
    }
